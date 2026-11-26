//! A deterministic, terminal-free driver for models (no Go equivalent; Go
//! tests call `Update` and `cmd()` by hand, which still works in Rust via
//! [`super::Cmd::call`]).
//!
//! The harness keeps a virtual clock. Immediate commands run synchronously
//! after each update (batches flattened, sequences in order); timers
//! (`tea::tick`, cursor blinks, spinners) stay pending until the test
//! advances the clock with [`Harness::advance`] or
//! [`Harness::fire_next_tick`]. Cancelled blink timers deliver their
//! cancellation message right away, like Go's context cancellation.
//!
//! ```
//! use sst_tuikit::tea::{self, harness::Harness, Cmd, Model, Msg};
//! struct Counter(i32);
//! struct Inc;
//! impl Model for Counter {
//!     fn init(&self) -> Option<Cmd> { tea::tick(std::time::Duration::from_secs(1), |_| Inc) }
//!     fn update(&mut self, msg: Msg) -> Option<Cmd> {
//!         if msg.downcast_ref::<Inc>().is_some() { self.0 += 1; }
//!         None
//!     }
//!     fn view(&self) -> String { self.0.to_string() }
//! }
//! let mut h = Harness::new(Counter(0));
//! h.init();
//! assert_eq!(h.view(), "0");
//! assert_eq!(h.pending_ticks().len(), 1);
//! h.advance(std::time::Duration::from_secs(1));
//! assert_eq!(h.view(), "1");
//! ```

use std::any::Any;
use std::time::{Duration, Instant, SystemTime};

use super::cmd::{InterruptMsg, Msg, QuitMsg};
use super::program::{Model, Scheduler};

/// Drives a [`Model`] deterministically.
pub struct Harness<M: Model> {
    pub model: M,
    sched: Scheduler,
    base: Instant,
    elapsed: Duration,
    start_wall: SystemTime,
    quit: bool,
    interrupted: bool,
    updates: usize,
}

impl<M: Model> Harness<M> {
    /// A harness whose virtual wall clock starts at a fixed instant
    /// (2025-01-01T00:00:00Z).
    pub fn new(model: M) -> Self {
        Self::with_start_time(
            model,
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_735_689_600),
        )
    }

    /// A harness whose virtual wall clock starts at `start`.
    pub fn with_start_time(model: M, start: SystemTime) -> Self {
        Harness {
            model,
            sched: Scheduler::new(false),
            base: Instant::now(),
            elapsed: Duration::ZERO,
            start_wall: start,
            quit: false,
            interrupted: false,
            updates: 0,
        }
    }

    fn now(&self) -> Instant {
        self.base + self.elapsed
    }

    /// The current virtual wall-clock time.
    pub fn now_wall(&self) -> SystemTime {
        self.start_wall + self.elapsed
    }

    /// Virtual time elapsed since the harness was created.
    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    /// Runs `model.init()` and processes its immediate commands.
    pub fn init(&mut self) {
        let c = self.model.init();
        self.sched.push(c);
        self.run_until_idle();
    }

    /// Sends a message and processes everything that follows immediately.
    pub fn send<T: Any>(&mut self, m: T) {
        self.send_msg(Box::new(m));
    }

    /// Sends an already boxed message.
    pub fn send_msg(&mut self, m: Msg) {
        self.sched.msgs.push_back(m);
        self.run_until_idle();
    }

    /// Feeds raw terminal input bytes, decoded like bubbletea's reader.
    pub fn send_input(&mut self, bytes: &[u8]) {
        for m in super::input::parse_input(bytes) {
            self.sched.msgs.push_back(m.into_msg());
        }
        self.run_until_idle();
    }

    /// Processes pending commands and messages until only timers remain or
    /// the model quit.
    pub fn run_until_idle(&mut self) {
        let start_wall = self.start_wall;
        let base = self.base;
        loop {
            if self.quit || self.interrupted {
                return;
            }
            let now = self.now();
            let wall = self.now_wall();
            self.sched.run_ready(now, wall);
            self.sched.reap_cancelled();
            // Timers that are already due (zero-length ticks).
            let wall_at = move |i: Instant| start_wall + i.saturating_duration_since(base);
            while self.sched.fire_due(now, wall_at) {
                self.sched.run_ready(now, wall);
            }
            let Some(m) = self.sched.msgs.pop_front() else {
                if self.sched.has_ready() {
                    continue;
                }
                return;
            };
            if m.downcast_ref::<QuitMsg>().is_some() {
                self.quit = true;
                return;
            }
            if m.downcast_ref::<InterruptMsg>().is_some() {
                self.interrupted = true;
                return;
            }
            let c = self.model.update(m);
            self.updates += 1;
            self.sched.push(c);
        }
    }

    /// Remaining time of each pending timer, in creation order.
    pub fn pending_ticks(&self) -> Vec<Duration> {
        let now = self.now();
        self.sched
            .pending_timers()
            .into_iter()
            .map(|(d, _)| d.saturating_duration_since(now))
            .collect()
    }

    /// Advances the virtual clock by `d`, firing due timers in deadline
    /// order and processing their consequences.
    pub fn advance(&mut self, d: Duration) {
        let target = self.base + self.elapsed + d;
        loop {
            if self.quit || self.interrupted {
                break;
            }
            match self.sched.next_deadline() {
                Some(dl) if dl <= target => {
                    if dl > self.now() {
                        self.elapsed = dl - self.base;
                    }
                    self.run_until_idle();
                }
                _ => break,
            }
        }
        if target > self.now() {
            self.elapsed = target - self.base;
        }
        self.run_until_idle();
    }

    /// Jumps to the next timer deadline and fires it. Returns false when no
    /// timer is pending.
    pub fn fire_next_tick(&mut self) -> bool {
        let Some(dl) = self.sched.next_deadline() else {
            return false;
        };
        if dl > self.now() {
            self.elapsed = dl - self.base;
        }
        self.run_until_idle();
        true
    }

    /// The model's current view.
    pub fn view(&self) -> String {
        self.model.view()
    }

    /// True once a `QuitMsg` was processed.
    pub fn quit_requested(&self) -> bool {
        self.quit
    }

    /// True once an `InterruptMsg` was processed.
    pub fn interrupted(&self) -> bool {
        self.interrupted
    }

    /// Number of `update` calls so far.
    pub fn updates(&self) -> usize {
        self.updates
    }
}
