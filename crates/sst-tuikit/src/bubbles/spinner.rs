//! `bubbles/spinner`.

use std::sync::atomic::{AtomicI64, Ordering};
use std::time::{Duration, SystemTime};

use crate::lipgloss::{self, Style};
use crate::tea::{self, Cmd, Msg};

static LAST_ID: AtomicI64 = AtomicI64::new(0);

fn next_id() -> i32 {
    (LAST_ID.fetch_add(1, Ordering::SeqCst) + 1) as i32
}

/// `spinner.Spinner`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Spinner {
    pub frames: Vec<String>,
    pub fps: Duration,
}

fn sp(frames: &[&str], fps: Duration) -> Spinner {
    Spinner {
        frames: frames.iter().map(|s| s.to_string()).collect(),
        fps,
    }
}

pub fn line() -> Spinner {
    sp(&["|", "/", "-", "\\"], Duration::from_secs(1) / 10)
}
pub fn dot() -> Spinner {
    sp(
        &["⣾ ", "⣽ ", "⣻ ", "⢿ ", "⡿ ", "⣟ ", "⣯ ", "⣷ "],
        Duration::from_secs(1) / 10,
    )
}
pub fn mini_dot() -> Spinner {
    sp(
        &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
        Duration::from_secs(1) / 12,
    )
}
pub fn pulse() -> Spinner {
    sp(&["█", "▓", "▒", "░"], Duration::from_secs(1) / 8)
}
pub fn points() -> Spinner {
    sp(&["∙∙∙", "●∙∙", "∙●∙", "∙∙●"], Duration::from_secs(1) / 7)
}
pub fn ellipsis() -> Spinner {
    sp(&["", ".", "..", "..."], Duration::from_secs(1) / 3)
}

/// `spinner.TickMsg`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TickMsg {
    pub time: SystemTime,
    tag: i32,
    pub id: i32,
}

/// `spinner.Model`.
#[derive(Clone, Debug)]
pub struct Model {
    pub spinner: Spinner,
    pub style: Style,
    frame: usize,
    id: i32,
    tag: i32,
}

/// `spinner.New()`.
pub fn new() -> Model {
    Model {
        spinner: line(),
        style: lipgloss::new_style(),
        frame: 0,
        id: next_id(),
        tag: 0,
    }
}

impl Model {
    pub fn id(&self) -> i32 {
        self.id
    }

    /// `Update`.
    pub fn update(&mut self, msg: &Msg) -> Option<Cmd> {
        if let Some(t) = msg.downcast_ref::<TickMsg>() {
            if t.id > 0 && t.id != self.id {
                return None;
            }
            if t.tag > 0 && t.tag != self.tag {
                return None;
            }
            self.frame += 1;
            if self.frame >= self.spinner.frames.len() {
                self.frame = 0;
            }
            self.tag += 1;
            return self.tick_cmd(self.id, self.tag);
        }
        None
    }

    /// `View`.
    pub fn view(&self) -> String {
        if self.frame >= self.spinner.frames.len() {
            return "(error)".into();
        }
        self.style.render(&self.spinner.frames[self.frame])
    }

    /// `Model.Tick` used as a command.
    pub fn tick(&self) -> Option<Cmd> {
        let (id, tag) = (self.id, self.tag);
        tea::cmd(move || TickMsg {
            time: SystemTime::now(),
            id,
            tag,
        })
    }

    fn tick_cmd(&self, id: i32, tag: i32) -> Option<Cmd> {
        tea::tick(self.spinner.fps, move |t| TickMsg { time: t, id, tag })
    }
}
