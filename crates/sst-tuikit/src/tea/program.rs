//! The bubbletea runtime (`tea.go`, `tty.go`, `tty_unix.go`,
//! `signals_unix.go`, `options.go`).
//!
//! Differences from Go, all invisible to models:
//! * Commands run on the event-loop thread instead of goroutines: immediate
//!   commands run right after the `Update` that returned them (in order,
//!   batches included), timers are kept in a timer list and fire on time.
//!   A command that blocks therefore blocks the UI.
//! * Input is read by a background thread from the TTY and decoded exactly
//!   like bubbletea (`readAnsiInputs`); resize (SIGWINCH) and SIGINT/SIGTERM
//!   are delivered through the same channel.

use std::collections::VecDeque;
use std::fmt;
use std::io::{self, Write};
use std::panic::{self, AssertUnwindSafe};
use std::sync::mpsc;
use std::time::{Duration, Instant, SystemTime};

use super::cmd::{
    BatchMsg, Cmd, InterruptMsg, Msg, QuitMsg, ScreenMsg, SequenceMsg, SuspendMsg, Tick,
    WindowSizeMsg,
};
use super::input::{InputDecoder, InputMsg};
use super::renderer::StandardRenderer;
use crate::term;

/// A bubbletea model (`tea.Model`).
///
/// Go's `Update(msg) (tea.Model, tea.Cmd)` returns a new model by value; in
/// Rust `update` mutates `self` in place and returns the command. When the Go
/// code returns a *different* model value (e.g. an unmodified copy), assign
/// it to `*self`. `init` takes `&self` because Go's `Init` has a value
/// receiver: mutations made there are lost (clone what you need to mutate).
pub trait Model {
    fn init(&self) -> Option<Cmd>;
    fn update(&mut self, msg: Msg) -> Option<Cmd>;
    fn view(&self) -> String;
}

/// Program errors (`tea.ErrProgramKilled`, `ErrProgramPanic`,
/// `ErrInterrupted`).
#[derive(Debug)]
pub enum ProgramError {
    /// `ErrProgramKilled` wrapping an inner error (Go `%w: %w`).
    Killed(Option<Box<ProgramError>>),
    Panic,
    Interrupted,
    Io(io::Error),
}

impl fmt::Display for ProgramError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProgramError::Killed(None) => f.write_str("program was killed"),
            ProgramError::Killed(Some(e)) => write!(f, "program was killed: {e}"),
            ProgramError::Panic => f.write_str("program experienced a panic"),
            ProgramError::Interrupted => f.write_str("program was interrupted"),
            ProgramError::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ProgramError {}

/// `tea.ProgramOption`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProgramOption {
    AltScreen,
    MouseCellMotion,
    MouseAllMotion,
    WithoutBracketedPaste,
    ReportFocus,
    WithoutSignalHandler,
    WithoutCatchPanics,
    Fps(i32),
    WindowTitle(String),
}

/// `tea.WithAltScreen()`.
pub fn with_alt_screen() -> ProgramOption {
    ProgramOption::AltScreen
}
/// `tea.WithMouseCellMotion()`.
pub fn with_mouse_cell_motion() -> ProgramOption {
    ProgramOption::MouseCellMotion
}
/// `tea.WithMouseAllMotion()`.
pub fn with_mouse_all_motion() -> ProgramOption {
    ProgramOption::MouseAllMotion
}
/// `tea.WithoutBracketedPaste()`.
pub fn without_bracketed_paste() -> ProgramOption {
    ProgramOption::WithoutBracketedPaste
}
/// `tea.WithReportFocus()`.
pub fn with_report_focus() -> ProgramOption {
    ProgramOption::ReportFocus
}
/// `tea.WithoutSignalHandler()`.
pub fn without_signal_handler() -> ProgramOption {
    ProgramOption::WithoutSignalHandler
}
/// `tea.WithoutCatchPanics()`.
pub fn without_catch_panics() -> ProgramOption {
    ProgramOption::WithoutCatchPanics
}
/// `tea.WithFPS(fps)`.
pub fn with_fps(fps: i32) -> ProgramOption {
    ProgramOption::Fps(fps)
}

/// Startup option bits (`startupOptions`), same values as Go.
pub const WITH_ALT_SCREEN: i16 = 1 << 0;
pub const WITH_MOUSE_CELL_MOTION: i16 = 1 << 1;
pub const WITH_MOUSE_ALL_MOTION: i16 = 1 << 2;
pub const WITH_ANSI_COMPRESSOR: i16 = 1 << 3;
pub const WITHOUT_SIGNAL_HANDLER: i16 = 1 << 4;
pub const WITHOUT_CATCH_PANICS: i16 = 1 << 5;
pub const WITHOUT_BRACKETED_PASTE: i16 = 1 << 6;
pub const WITH_REPORT_FOCUS: i16 = 1 << 7;

enum Event {
    Input(InputMsg),
    #[cfg(unix)]
    Resize,
    #[cfg(unix)]
    Signal(i32),
    InputClosed,
}

/// A bubbletea program (`tea.Program`).
pub struct Program<M: Model> {
    model: M,
    startup_options: i16,
    fps: i32,
    title: String,
}

struct Timer {
    deadline: Instant,
    tick: Tick,
    seq: Option<VecDeque<Cmd>>,
}

/// The command scheduler shared by [`Program`] and the test harness.
pub(crate) struct Scheduler {
    /// Real runtime: timers start at their creation instant (Go starts the
    /// timer when the command is created). Harness: at the virtual "now".
    use_created: bool,
    pub(crate) msgs: VecDeque<Msg>,
    timers: Vec<Timer>,
    ready: VecDeque<(Cmd, Option<VecDeque<Cmd>>)>,
}

impl Scheduler {
    pub(crate) fn new(use_created: bool) -> Self {
        Scheduler {
            use_created,
            msgs: VecDeque::new(),
            timers: Vec::new(),
            ready: VecDeque::new(),
        }
    }

    pub(crate) fn push(&mut self, c: Option<Cmd>) {
        if let Some(c) = c {
            self.ready.push_back((c, None));
        }
    }

    fn deliver(&mut self, m: Msg) {
        match m.downcast::<BatchMsg>() {
            Ok(b) => {
                for c in b.0 {
                    self.ready.push_back((c, None));
                }
            }
            Err(m) => match m.downcast::<SequenceMsg>() {
                Ok(s) => self.start_sequence(s.0.into()),
                Err(m) => self.msgs.push_back(m),
            },
        }
    }

    fn start_sequence(&mut self, mut rest: VecDeque<Cmd>) {
        if let Some(first) = rest.pop_front() {
            self.ready.push_back((first, Some(rest)));
        }
    }

    /// Runs all ready immediate commands; timers are registered against
    /// `now`.
    pub(crate) fn run_ready(&mut self, now: Instant, wall: SystemTime) {
        while let Some((c, seq)) = self.ready.pop_front() {
            match c {
                Cmd::Func(f) => {
                    if let Some(m) = f() {
                        self.deliver(m);
                    }
                    if let Some(rest) = seq {
                        self.start_sequence(rest);
                    }
                }
                Cmd::Tick(t) => {
                    let delay = t.delay_from(wall);
                    let base = if t.every || !self.use_created {
                        now
                    } else {
                        t.created.min(now)
                    };
                    self.timers.push(Timer {
                        deadline: base + delay,
                        tick: t,
                        seq,
                    });
                }
                Cmd::Batch(v) => {
                    for c in v {
                        self.ready.push_back((c, None));
                    }
                    if let Some(rest) = seq {
                        self.start_sequence(rest);
                    }
                }
                Cmd::Sequence(v) => {
                    let mut all: VecDeque<Cmd> = v.into();
                    if let Some(rest) = seq {
                        all.extend(rest);
                    }
                    self.start_sequence(all);
                }
            }
        }
    }

    /// Delivers cancellation messages of cancelled timers.
    pub(crate) fn reap_cancelled(&mut self) {
        let mut i = 0;
        while i < self.timers.len() {
            if self.timers[i].tick.is_cancelled() {
                let t = self.timers.remove(i);
                if let Some(m) = t.tick.cancelled_msg() {
                    self.deliver(m);
                }
                if let Some(rest) = t.seq {
                    self.start_sequence(rest);
                }
            } else {
                i += 1;
            }
        }
    }

    /// Fires the earliest timer due at or before `now`; returns false if none.
    pub(crate) fn fire_due(
        &mut self,
        now: Instant,
        wall_at: impl Fn(Instant) -> SystemTime,
    ) -> bool {
        let Some(idx) = self.next_timer_index() else {
            return false;
        };
        if self.timers[idx].deadline > now {
            return false;
        }
        let t = self.timers.remove(idx);
        let at = wall_at(t.deadline);
        if let Some(m) = t.tick.fire(at) {
            self.deliver(m);
        }
        if let Some(rest) = t.seq {
            self.start_sequence(rest);
        }
        true
    }

    fn next_timer_index(&self) -> Option<usize> {
        self.timers
            .iter()
            .enumerate()
            .min_by_key(|(i, t)| (t.deadline, *i))
            .map(|(i, _)| i)
    }

    pub(crate) fn next_deadline(&self) -> Option<Instant> {
        self.next_timer_index().map(|i| self.timers[i].deadline)
    }

    pub(crate) fn pending_timers(&self) -> Vec<(Instant, Duration)> {
        self.timers
            .iter()
            .map(|t| (t.deadline, t.tick.duration))
            .collect()
    }

    pub(crate) fn has_ready(&self) -> bool {
        !self.ready.is_empty()
    }
}

impl<M: Model> Program<M> {
    /// `tea.NewProgram(model, opts...)`.
    pub fn new(model: M, opts: &[ProgramOption]) -> Self {
        let mut p = Program {
            model,
            startup_options: 0,
            fps: 0,
            title: String::new(),
        };
        for o in opts {
            match o {
                ProgramOption::AltScreen => p.startup_options |= WITH_ALT_SCREEN,
                ProgramOption::MouseCellMotion => {
                    p.startup_options |= WITH_MOUSE_CELL_MOTION;
                    p.startup_options &= !WITH_MOUSE_ALL_MOTION;
                }
                ProgramOption::MouseAllMotion => {
                    p.startup_options |= WITH_MOUSE_ALL_MOTION;
                    p.startup_options &= !WITH_MOUSE_CELL_MOTION;
                }
                ProgramOption::WithoutBracketedPaste => {
                    p.startup_options |= WITHOUT_BRACKETED_PASTE
                }
                ProgramOption::ReportFocus => p.startup_options |= WITH_REPORT_FOCUS,
                ProgramOption::WithoutSignalHandler => p.startup_options |= WITHOUT_SIGNAL_HANDLER,
                ProgramOption::WithoutCatchPanics => p.startup_options |= WITHOUT_CATCH_PANICS,
                ProgramOption::Fps(f) => p.fps = *f,
                ProgramOption::WindowTitle(t) => p.title = t.clone(),
            }
        }
        p
    }

    /// The startup option bitmask (Go's unexported `startupOptions`).
    pub fn startup_options(&self) -> i16 {
        self.startup_options
    }

    /// `Program.Run`: runs until `tea.Quit`; returns the final model.
    pub fn run(self) -> Result<M, ProgramError> {
        let catch = self.startup_options & WITHOUT_CATCH_PANICS == 0;
        let mut rt = Runtime::new(self.startup_options, self.fps);
        let mut model = self.model;
        let title = self.title;
        if !catch {
            return rt.run(&mut model, &title).map(|_| model);
        }
        let res = panic::catch_unwind(AssertUnwindSafe(|| rt.run(&mut model, &title)));
        match res {
            Ok(Ok(())) => Ok(model),
            Ok(Err(e)) => Err(e),
            Err(p) => {
                rt.shutdown(true);
                let what = if let Some(s) = p.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = p.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "panic".to_string()
                };
                print!("Caught panic:\n\n{what}\n\nRestoring terminal...\n\n");
                let _ = io::stdout().flush();
                Err(ProgramError::Killed(Some(Box::new(ProgramError::Panic))))
            }
        }
    }
}

struct Runtime {
    opts: i16,
    framerate: Duration,
    renderer: StandardRenderer,
    input_fd: Option<term::Fd>,
    _tty: Option<term::Tty>,
    saved_input: Option<term::State>,
    output_is_tty: bool,
    events: Option<mpsc::Receiver<Event>>,
    shut_down: bool,
}

impl Runtime {
    fn new(opts: i16, fps: i32) -> Self {
        let fps = if fps < 1 { 60 } else { fps.min(120) };
        Runtime {
            opts,
            framerate: Duration::from_secs(1) / fps as u32,
            renderer: StandardRenderer::new(),
            input_fd: None,
            _tty: None,
            saved_input: None,
            output_is_tty: false,
            events: None,
            shut_down: false,
        }
    }

    fn write_out(&mut self) {
        let mut out = io::stdout();
        let _ = self.renderer.write_to(&mut out);
    }

    fn init_terminal(&mut self) -> Result<(), ProgramError> {
        // Input: stdin if it is a terminal, otherwise /dev/tty.
        if term::is_terminal(term::STDIN) {
            self.input_fd = Some(term::STDIN);
        } else if let Ok(t) = term::Tty::open() {
            self.input_fd = Some(t.fd());
            self._tty = Some(t);
        } else {
            return Err(ProgramError::Io(io::Error::other(
                "could not open a new TTY",
            )));
        }
        if let Some(fd) = self.input_fd {
            if term::is_terminal(fd) {
                self.saved_input = Some(term::make_raw(fd).map_err(|e| {
                    ProgramError::Io(io::Error::other(format!("error entering raw mode: {e}")))
                })?);
            }
        }
        self.output_is_tty = term::is_terminal(term::STDOUT);
        self.renderer.hide_cursor();
        Ok(())
    }

    fn restore_terminal_state(&mut self) {
        self.renderer.disable_bracketed_paste();
        self.renderer.show_cursor();
        self.disable_mouse();
        if self.renderer.report_focus() {
            self.renderer.disable_report_focus();
        }
        if self.renderer.alt_screen() {
            self.renderer.exit_alt_screen();
            self.write_out();
            std::thread::sleep(Duration::from_millis(10));
        }
        self.write_out();
        if let (Some(fd), Some(st)) = (self.input_fd, self.saved_input.take()) {
            let _ = term::restore(fd, &st);
        }
    }

    fn disable_mouse(&mut self) {
        self.renderer.disable_mouse_cell_motion();
        self.renderer.disable_mouse_all_motion();
        self.renderer.disable_mouse_sgr_mode();
    }

    fn shutdown(&mut self, kill: bool) {
        if self.shut_down {
            return;
        }
        self.shut_down = true;
        if kill {
            self.renderer.kill();
        } else {
            self.renderer.stop();
        }
        self.write_out();
        self.restore_terminal_state();
    }

    fn spawn_threads(&mut self) -> mpsc::Sender<Event> {
        let (tx, rx) = mpsc::channel();
        self.events = Some(rx);
        if let Some(fd) = self.input_fd {
            let tx2 = tx.clone();
            std::thread::spawn(move || {
                let mut dec = InputDecoder::default();
                let mut buf = [0u8; InputDecoder::BUF_SIZE];
                loop {
                    match term::read(fd, &mut buf) {
                        Ok(0) | Err(_) => {
                            let _ = tx2.send(Event::InputClosed);
                            return;
                        }
                        Ok(n) => {
                            for m in dec.feed(&buf[..n]) {
                                if tx2.send(Event::Input(m)).is_err() {
                                    return;
                                }
                            }
                        }
                    }
                }
            });
        }
        #[cfg(unix)]
        {
            use signal_hook::consts::{SIGINT, SIGTERM, SIGWINCH};
            let mut sigs = vec![SIGWINCH];
            if self.opts & WITHOUT_SIGNAL_HANDLER == 0 {
                sigs.push(SIGINT);
                sigs.push(SIGTERM);
            }
            if let Ok(mut signals) = signal_hook::iterator::Signals::new(&sigs) {
                let tx3 = tx.clone();
                std::thread::spawn(move || {
                    for s in signals.forever() {
                        let ev = if s == SIGWINCH {
                            Event::Resize
                        } else {
                            Event::Signal(s)
                        };
                        if tx3.send(ev).is_err() {
                            return;
                        }
                    }
                });
            }
        }
        tx
    }

    fn check_resize(&mut self, sched: &mut Scheduler) {
        if !self.output_is_tty {
            return;
        }
        if let Ok((w, h)) = term::get_size(term::STDOUT) {
            sched.msgs.push_back(Box::new(WindowSizeMsg {
                width: w,
                height: h,
            }));
        }
    }

    fn run<M: Model>(&mut self, model: &mut M, title: &str) -> Result<(), ProgramError> {
        self.init_terminal()?;
        if !title.is_empty() {
            self.renderer.set_window_title(title);
        }
        if self.opts & WITH_ALT_SCREEN != 0 {
            self.renderer.enter_alt_screen();
        }
        if self.opts & WITHOUT_BRACKETED_PASTE == 0 {
            self.renderer.enable_bracketed_paste();
        }
        if self.opts & WITH_MOUSE_CELL_MOTION != 0 {
            self.renderer.enable_mouse_cell_motion();
            self.renderer.enable_mouse_sgr_mode();
        } else if self.opts & WITH_MOUSE_ALL_MOTION != 0 {
            self.renderer.enable_mouse_all_motion();
            self.renderer.enable_mouse_sgr_mode();
        }
        if self.opts & WITH_REPORT_FOCUS != 0 {
            self.renderer.enable_report_focus();
        }
        self.write_out();

        let mut sched = Scheduler::new(true);
        sched.push(model.init());
        self.renderer.write(&model.view());
        let _tx = self.spawn_threads();
        self.check_resize(&mut sched);

        let start_wall = SystemTime::now();
        let start = Instant::now();
        let wall_at = move |i: Instant| start_wall + i.saturating_duration_since(start);
        let mut next_frame = Instant::now() + self.framerate;
        let result: Result<(), ProgramError>;

        'outer: loop {
            // Run commands produced so far.
            sched.run_ready(Instant::now(), SystemTime::now());
            sched.reap_cancelled();
            while sched.fire_due(Instant::now(), wall_at) {
                sched.run_ready(Instant::now(), SystemTime::now());
            }

            // Process one message at a time, like the Go event loop.
            if let Some(m) = sched.msgs.pop_front() {
                if m.downcast_ref::<QuitMsg>().is_some() {
                    result = Ok(());
                    break 'outer;
                }
                if m.downcast_ref::<InterruptMsg>().is_some() {
                    result = Err(ProgramError::Interrupted);
                    break 'outer;
                }
                if m.downcast_ref::<SuspendMsg>().is_some() {
                    // Suspension (SIGTSTP) is not supported; ignored.
                }
                if let Some(sm) = m.downcast_ref::<ScreenMsg>() {
                    match sm {
                        ScreenMsg::ClearScreen => self.renderer.clear_screen(),
                        ScreenMsg::EnterAltScreen => self.renderer.enter_alt_screen(),
                        ScreenMsg::ExitAltScreen => self.renderer.exit_alt_screen(),
                        ScreenMsg::EnableMouseCellMotion => {
                            self.renderer.enable_mouse_cell_motion();
                            self.renderer.enable_mouse_sgr_mode();
                        }
                        ScreenMsg::EnableMouseAllMotion => {
                            self.renderer.enable_mouse_all_motion();
                            self.renderer.enable_mouse_sgr_mode();
                        }
                        ScreenMsg::DisableMouse => self.disable_mouse(),
                        ScreenMsg::HideCursor => self.renderer.hide_cursor(),
                        ScreenMsg::ShowCursor => self.renderer.show_cursor(),
                        ScreenMsg::EnableBracketedPaste => self.renderer.enable_bracketed_paste(),
                        ScreenMsg::DisableBracketedPaste => self.renderer.disable_bracketed_paste(),
                        ScreenMsg::EnableReportFocus => self.renderer.enable_report_focus(),
                        ScreenMsg::DisableReportFocus => self.renderer.disable_report_focus(),
                        ScreenMsg::SetWindowTitle(t) => self.renderer.set_window_title(t),
                        ScreenMsg::WindowSize => self.check_resize(&mut sched),
                        ScreenMsg::Repaint => self.renderer.repaint(),
                    }
                }
                if let Some(ws) = m.downcast_ref::<WindowSizeMsg>() {
                    self.renderer.resize(ws.width, ws.height);
                }
                let c = model.update(m);
                sched.push(c);
                self.renderer.write(&model.view());
                continue;
            }

            // Flush a frame when due.
            let now = Instant::now();
            if now >= next_frame {
                self.renderer.flush();
                self.write_out();
                next_frame = now + self.framerate;
            }

            if sched.has_ready() {
                continue;
            }

            // Wait for input / signals / the next timer / the next frame.
            let mut wait = next_frame.saturating_duration_since(Instant::now());
            if let Some(d) = sched.next_deadline() {
                wait = wait.min(d.saturating_duration_since(Instant::now()));
            }
            let ev = match self.events.as_ref() {
                Some(rx) => match rx.recv_timeout(wait) {
                    Ok(ev) => Some(ev),
                    Err(mpsc::RecvTimeoutError::Timeout) => None,
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        std::thread::sleep(wait);
                        None
                    }
                },
                None => {
                    std::thread::sleep(wait);
                    None
                }
            };
            match ev {
                Some(Event::Input(m)) => sched.msgs.push_back(m.into_msg()),
                #[cfg(unix)]
                Some(Event::Resize) => self.check_resize(&mut sched),
                #[cfg(unix)]
                Some(Event::Signal(s)) => {
                    if s == signal_hook::consts::SIGINT {
                        sched.msgs.push_back(Box::new(InterruptMsg));
                    } else {
                        sched.msgs.push_back(Box::new(QuitMsg));
                    }
                }
                Some(Event::InputClosed) => {
                    // EOF on input: Go keeps running without input.
                }
                None => {}
            }
        }

        let killed = result.is_err();
        if !killed {
            self.renderer.write(&model.view());
        }
        self.shutdown(killed);
        result.map_err(|e| ProgramError::Killed(Some(Box::new(e))))
    }
}
