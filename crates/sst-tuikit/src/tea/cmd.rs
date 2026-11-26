//! Messages and commands (`tea.go`, `commands.go`).
//!
//! * Go `tea.Msg` (any value) -> [`Msg`] = `Box<dyn Any>`; build one with
//!   [`msg`]`(value)` and inspect it with `m.downcast_ref::<KeyMsg>()` (Go
//!   type switch) or [`is`]`::<T>(&m)`.
//! * Go `tea.Cmd` (a `func() Msg` that may be `nil`) -> `Option<`[`Cmd`]`>`.
//!   `return m, nil` becomes returning `None`.
//! * Unlike Go's opaque functions, a [`Cmd`] tells you what it is: an
//!   immediate function ([`Cmd::Func`]), a timer ([`Cmd::Tick`], produced by
//!   [`tick`], [`every`] and cursor blinking), or a [`Cmd::Batch`] /
//!   [`Cmd::Sequence`]. A deterministic harness can therefore run immediate
//!   commands synchronously and fire timers on demand (see
//!   [`super::harness::Harness`]). [`Cmd::call`] reproduces Go's `cmd()`
//!   (blocking for timers).

use std::any::Any;
use std::cell::Cell;
use std::fmt;
use std::rc::Rc;
use std::time::{Duration, Instant, SystemTime};

/// A message (`tea.Msg`).
pub type Msg = Box<dyn Any>;

/// Boxes a value as a [`Msg`].
pub fn msg<T: Any>(t: T) -> Msg {
    Box::new(t)
}

/// Go `_, ok := m.(T)`.
pub fn is<T: Any>(m: &Msg) -> bool {
    m.downcast_ref::<T>().is_some()
}

/// A cancellation flag shared by a timer command and its owner (used by the
/// cursor blink, which Go implements with `context.WithTimeout`).
pub type CancelToken = Rc<Cell<bool>>;

/// A timer command (`tea.Tick`, `tea.Every`, `cursor.BlinkCmd`).
pub struct Tick {
    /// How long after `created` the timer fires.
    pub duration: Duration,
    /// When the command was created (Go starts the timer at creation).
    pub created: Instant,
    /// For `tea.Every`: align the deadline to a multiple of `duration`.
    pub(crate) every: bool,
    fire: Box<dyn FnOnce(SystemTime) -> Option<Msg>>,
    cancel: Option<CancelToken>,
    on_cancel: Option<Box<dyn FnOnce() -> Option<Msg>>>,
}

impl Tick {
    /// Produces the message for a timer firing at `at` (Go passes the timer
    /// time to the tick function).
    pub fn fire(self, at: SystemTime) -> Option<Msg> {
        (self.fire)(at)
    }
    /// True once the owner cancelled the timer (blink restarted).
    pub fn is_cancelled(&self) -> bool {
        self.cancel.as_ref().is_some_and(|c| c.get())
    }
    /// The message delivered immediately when a cancelled timer is dropped
    /// (Go's `blinkCanceled{}`), if any.
    pub fn cancelled_msg(self) -> Option<Msg> {
        self.on_cancel.and_then(|f| f())
    }
    /// The deadline relative to `created`, honouring `Every` alignment.
    pub fn delay_from(&self, wall: SystemTime) -> Duration {
        if !self.every {
            return self.duration;
        }
        // Go: n.Truncate(d).Add(d).Sub(n)
        let d = self.duration.as_nanos().max(1);
        let n = wall
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|x| x.as_nanos())
            .unwrap_or(0);
        Duration::from_nanos((d - n % d) as u64)
    }
}

/// A command (`tea.Cmd`, non-nil).
pub enum Cmd {
    /// An immediate function (`func() tea.Msg`). `None` is a nil message.
    Func(Box<dyn FnOnce() -> Option<Msg>>),
    /// A timer.
    Tick(Tick),
    /// `tea.Batch`: run concurrently (in order, here).
    Batch(Vec<Cmd>),
    /// `tea.Sequence`: run one after another.
    Sequence(Vec<Cmd>),
}

impl fmt::Debug for Cmd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Cmd::Func(_) => f.write_str("Cmd::Func"),
            Cmd::Tick(t) => write!(f, "Cmd::Tick({:?})", t.duration),
            Cmd::Batch(v) => f.debug_tuple("Cmd::Batch").field(v).finish(),
            Cmd::Sequence(v) => f.debug_tuple("Cmd::Sequence").field(v).finish(),
        }
    }
}

/// `tea.BatchMsg`: what calling a batch command returns.
pub struct BatchMsg(pub Vec<Cmd>);

/// bubbletea's internal `sequenceMsg`.
pub struct SequenceMsg(pub Vec<Cmd>);

impl Cmd {
    /// An immediate command from a closure returning a message.
    pub fn new<T: Any>(f: impl FnOnce() -> T + 'static) -> Cmd {
        Cmd::Func(Box::new(move || Some(msg(f()))))
    }

    /// Go `cmd()`: runs the command and returns its message. Timers block
    /// until they fire (or return their cancellation message), batches and
    /// sequences return [`BatchMsg`] / [`SequenceMsg`] like Go.
    pub fn call(self) -> Option<Msg> {
        match self {
            Cmd::Func(f) => f(),
            Cmd::Tick(t) => {
                let delay = t.delay_from(SystemTime::now());
                let deadline = t.created + delay;
                loop {
                    if t.is_cancelled() {
                        return t.cancelled_msg();
                    }
                    let now = Instant::now();
                    if now >= deadline {
                        break;
                    }
                    std::thread::sleep((deadline - now).min(Duration::from_millis(5)));
                }
                t.fire(SystemTime::now())
            }
            Cmd::Batch(v) => Some(msg(BatchMsg(v))),
            Cmd::Sequence(v) => Some(msg(SequenceMsg(v))),
        }
    }

    /// True for timer commands.
    pub fn is_tick(&self) -> bool {
        matches!(self, Cmd::Tick(_))
    }

    /// True if this command or any nested command is a timer.
    pub fn contains_tick(&self) -> bool {
        match self {
            Cmd::Tick(_) => true,
            Cmd::Func(_) => false,
            Cmd::Batch(v) | Cmd::Sequence(v) => v.iter().any(Cmd::contains_tick),
        }
    }

    /// Runs every immediate part synchronously (flattening batches and
    /// sequences, and messages that are themselves `BatchMsg`/`SequenceMsg`)
    /// and returns the produced messages plus the timers still pending, in
    /// order.
    pub fn run_immediate(self) -> (Vec<Msg>, Vec<Tick>) {
        let mut msgs = Vec::new();
        let mut ticks = Vec::new();
        fn go(c: Cmd, msgs: &mut Vec<Msg>, ticks: &mut Vec<Tick>) {
            match c {
                Cmd::Func(f) => {
                    if let Some(m) = f() {
                        expand_msg(m, msgs, ticks);
                    }
                }
                Cmd::Tick(t) => ticks.push(t),
                Cmd::Batch(v) | Cmd::Sequence(v) => {
                    for c in v {
                        go(c, msgs, ticks);
                    }
                }
            }
        }
        fn expand_msg(m: Msg, msgs: &mut Vec<Msg>, ticks: &mut Vec<Tick>) {
            match m.downcast::<BatchMsg>() {
                Ok(b) => {
                    for c in b.0 {
                        go(c, msgs, ticks);
                    }
                }
                Err(m) => match m.downcast::<SequenceMsg>() {
                    Ok(s) => {
                        for c in s.0 {
                            go(c, msgs, ticks);
                        }
                    }
                    Err(m) => msgs.push(m),
                },
            }
        }
        go(self, &mut msgs, &mut ticks);
        (msgs, ticks)
    }
}

/// An immediate command producing `f()` (Go `func() tea.Msg { return X{} }`).
pub fn cmd<T: Any>(f: impl FnOnce() -> T + 'static) -> Option<Cmd> {
    Some(Cmd::new(f))
}

/// An immediate command whose function may return a nil message.
pub fn cmd_opt(f: impl FnOnce() -> Option<Msg> + 'static) -> Option<Cmd> {
    Some(Cmd::Func(Box::new(f)))
}

/// A command that just returns `value`.
pub fn msg_cmd<T: Any>(value: T) -> Option<Cmd> {
    cmd(move || value)
}

fn compact(cmds: Vec<Option<Cmd>>, seq: bool) -> Option<Cmd> {
    let mut valid: Vec<Cmd> = cmds.into_iter().flatten().collect();
    match valid.len() {
        0 => None,
        1 => valid.pop(),
        _ => Some(if seq {
            Cmd::Sequence(valid)
        } else {
            Cmd::Batch(valid)
        }),
    }
}

/// `tea.Batch`: nil commands are dropped; zero -> nil, one -> that command.
pub fn batch(cmds: Vec<Option<Cmd>>) -> Option<Cmd> {
    compact(cmds, false)
}

/// `tea.Sequence`: like [`batch`] but run in order.
pub fn sequence(cmds: Vec<Option<Cmd>>) -> Option<Cmd> {
    compact(cmds, true)
}

/// `tea.Tick(d, fn)`: a timer started now.
pub fn tick<T: Any>(d: Duration, f: impl FnOnce(SystemTime) -> T + 'static) -> Option<Cmd> {
    Some(Cmd::Tick(Tick {
        duration: d,
        created: Instant::now(),
        every: false,
        fire: Box::new(move |t| Some(msg(f(t)))),
        cancel: None,
        on_cancel: None,
    }))
}

/// `tea.Every(d, fn)`: a timer firing at the next multiple of `d` of the
/// wall clock.
pub fn every<T: Any>(d: Duration, f: impl FnOnce(SystemTime) -> T + 'static) -> Option<Cmd> {
    Some(Cmd::Tick(Tick {
        duration: d,
        created: Instant::now(),
        every: true,
        fire: Box::new(move |t| Some(msg(f(t)))),
        cancel: None,
        on_cancel: None,
    }))
}

/// A cancellable timer: fires `fire()` after `d` unless `cancel` is set
/// first, in which case `on_cancel()` is delivered right away.
pub fn cancellable_tick(
    d: Duration,
    cancel: CancelToken,
    fire: impl FnOnce() -> Option<Msg> + 'static,
    on_cancel: impl FnOnce() -> Option<Msg> + 'static,
) -> Option<Cmd> {
    Some(Cmd::Tick(Tick {
        duration: d,
        created: Instant::now(),
        every: false,
        fire: Box::new(move |_| fire()),
        cancel: Some(cancel),
        on_cancel: Some(Box::new(on_cancel)),
    }))
}

/// `tea.Sequentially` (deprecated in Go): runs commands in order, returning
/// the first non-nil message.
pub fn sequentially(cmds: Vec<Option<Cmd>>) -> Option<Cmd> {
    cmd_opt(move || {
        for c in cmds.into_iter().flatten() {
            if let Some(m) = c.call() {
                return Some(m);
            }
        }
        None
    })
}

/// `tea.QuitMsg`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct QuitMsg;

/// `tea.InterruptMsg`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InterruptMsg;

/// `tea.SuspendMsg`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SuspendMsg;

/// `tea.ResumeMsg`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ResumeMsg;

/// `tea.WindowSizeMsg`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WindowSizeMsg {
    pub width: i32,
    pub height: i32,
}

/// `tea.FocusMsg`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FocusMsg;

/// `tea.BlurMsg`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BlurMsg;

/// `tea.Quit` (used as a command: `return m, tea.Quit`).
pub fn quit() -> Option<Cmd> {
    msg_cmd(QuitMsg)
}

/// `tea.Interrupt`.
pub fn interrupt() -> Option<Cmd> {
    msg_cmd(InterruptMsg)
}

/// `tea.Suspend`.
pub fn suspend() -> Option<Cmd> {
    msg_cmd(SuspendMsg)
}

/// Program-internal messages produced by the screen commands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScreenMsg {
    ClearScreen,
    EnterAltScreen,
    ExitAltScreen,
    EnableMouseCellMotion,
    EnableMouseAllMotion,
    DisableMouse,
    HideCursor,
    ShowCursor,
    EnableBracketedPaste,
    DisableBracketedPaste,
    EnableReportFocus,
    DisableReportFocus,
    SetWindowTitle(String),
    WindowSize,
    Repaint,
}

pub fn clear_screen() -> Option<Cmd> {
    msg_cmd(ScreenMsg::ClearScreen)
}
pub fn enter_alt_screen() -> Option<Cmd> {
    msg_cmd(ScreenMsg::EnterAltScreen)
}
pub fn exit_alt_screen() -> Option<Cmd> {
    msg_cmd(ScreenMsg::ExitAltScreen)
}
pub fn enable_mouse_cell_motion() -> Option<Cmd> {
    msg_cmd(ScreenMsg::EnableMouseCellMotion)
}
pub fn enable_mouse_all_motion() -> Option<Cmd> {
    msg_cmd(ScreenMsg::EnableMouseAllMotion)
}
pub fn disable_mouse() -> Option<Cmd> {
    msg_cmd(ScreenMsg::DisableMouse)
}
pub fn hide_cursor() -> Option<Cmd> {
    msg_cmd(ScreenMsg::HideCursor)
}
pub fn show_cursor() -> Option<Cmd> {
    msg_cmd(ScreenMsg::ShowCursor)
}
pub fn enable_bracketed_paste() -> Option<Cmd> {
    msg_cmd(ScreenMsg::EnableBracketedPaste)
}
pub fn disable_bracketed_paste() -> Option<Cmd> {
    msg_cmd(ScreenMsg::DisableBracketedPaste)
}
pub fn enable_report_focus() -> Option<Cmd> {
    msg_cmd(ScreenMsg::EnableReportFocus)
}
pub fn disable_report_focus() -> Option<Cmd> {
    msg_cmd(ScreenMsg::DisableReportFocus)
}
pub fn set_window_title(title: &str) -> Option<Cmd> {
    msg_cmd(ScreenMsg::SetWindowTitle(title.to_string()))
}
/// `tea.WindowSize()`: asks the program to send a [`WindowSizeMsg`].
pub fn window_size() -> Option<Cmd> {
    msg_cmd(ScreenMsg::WindowSize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_compacts_like_go() {
        assert!(batch(vec![]).is_none());
        assert!(batch(vec![None, None]).is_none());
        let one = batch(vec![None, quit(), None]).unwrap();
        assert!(matches!(one, Cmd::Func(_)));
        assert!(is::<QuitMsg>(&one.call().unwrap()));
        let two = batch(vec![quit(), None, interrupt()]).unwrap();
        let m = two.call().unwrap();
        let b = m.downcast::<BatchMsg>().ok().unwrap();
        assert_eq!(b.0.len(), 2);
        let seq = sequence(vec![quit(), interrupt()]).unwrap();
        assert!(is::<SequenceMsg>(&seq.call().unwrap()));
    }

    #[test]
    fn run_immediate_separates_ticks() {
        let c = batch(vec![
            quit(),
            tick(Duration::from_millis(600), |_| 1i32),
            cmd_opt(|| {
                batch(vec![interrupt(), tick(Duration::from_millis(5), |_| 2i32)])
                    .unwrap()
                    .call()
            }),
        ])
        .unwrap();
        assert!(c.contains_tick());
        let (msgs, ticks) = c.run_immediate();
        // The third command returns a BatchMsg; it is expanded.
        assert_eq!(ticks.len(), 2);
        assert!(is::<QuitMsg>(&msgs[0]));
        assert!(is::<InterruptMsg>(&msgs[1]));
        assert_eq!(msgs.len(), 2);
    }

    #[test]
    fn tick_call_blocks_and_fires() {
        let c = tick(Duration::from_millis(1), |_| 42i32).unwrap();
        assert!(c.is_tick());
        let m = c.call().unwrap();
        assert_eq!(*m.downcast_ref::<i32>().unwrap(), 42);
        let token: CancelToken = Rc::new(Cell::new(false));
        let c = cancellable_tick(
            Duration::from_secs(60),
            token.clone(),
            || Some(msg(1i32)),
            || Some(msg("cancelled")),
        )
        .unwrap();
        token.set(true);
        let m = c.call().unwrap();
        assert_eq!(*m.downcast_ref::<&str>().unwrap(), "cancelled");
    }
}
