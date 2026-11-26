//! Tholian web weave/breach animation (port of `pkg/tui/anim/web.go`).

use std::fmt;
use std::time::Duration;

use sst_engine::rules::{ANIM_SPEED_NORMAL, ANIM_SPEED_OFF};
use sst_engine::types::Coord;
use sst_tuikit::lipgloss::{self, Style};

use super::{frame_duration, impl_animation, single, Frame, Playback};

/// Type of web animation sequence (Go `WebAction int`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct WebAction(pub i32);

#[allow(non_upper_case_globals)]
impl WebAction {
    /// `WebActionWeave`: pulsating lattice placement.
    pub const Weave: WebAction = WebAction(0);
    /// `WebActionBreach`: particle fracture and collapse.
    pub const Breach: WebAction = WebAction(1);
}

impl fmt::Display for WebAction {
    /// `String`: "Weave", "Breach" or "Unknown".
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match *self {
            WebAction::Weave => "Weave",
            WebAction::Breach => "Breach",
            _ => "Unknown",
        })
    }
}

/// Tholian web filament weave pulse and breach fracture (Go `WebAnimation`).
#[derive(Clone)]
pub struct WebAnimation {
    coord: Coord,
    action: WebAction,
    stages: Vec<&'static str>,
    pb: Playback,
}
impl_animation!(WebAnimation);

/// `NewWebAnimation`: normal speed.
pub fn new_web_animation(coord: Coord, action: WebAction) -> WebAnimation {
    new_web_animation_with_speed(coord, action, ANIM_SPEED_NORMAL)
}

fn fg(c: &str) -> Style {
    lipgloss::new_style().foreground(lipgloss::color(c))
}
fn bfg(c: &str) -> Style {
    lipgloss::new_style()
        .bold(true)
        .foreground(lipgloss::color(c))
}

/// `NewWebAnimationWithSpeed`.
pub fn new_web_animation_with_speed(coord: Coord, action: WebAction, speed: i32) -> WebAnimation {
    let mut d = frame_duration(speed);
    if !(0..=3).contains(&speed) {
        d = Duration::from_millis(120);
    }

    let (stages, styles): (Vec<&'static str>, Vec<Style>) = match action {
        WebAction::Weave => (
            vec![" : ", "·:·", ":::", ":::"],
            vec![
                fg("#06B6D4"),
                bfg("#22D3EE"),
                bfg("#67E8F9"),
                bfg("#22D3EE"),
            ],
        ),
        WebAction::Breach => (
            vec![":::", "*#*", "·*·", " · ", " . "],
            vec![
                bfg("#F43F5E"),
                bfg("#FFD700"),
                bfg("#FB923C"),
                fg("#94A3B8"),
                fg("#64748B"),
            ],
        ),
        _ => (vec![":::"], vec![bfg("#22D3EE")]),
    };

    let frames: Vec<Frame> = stages
        .iter()
        .zip(styles)
        .map(|(g, s)| single(coord, g, s, d))
        .collect();

    let mut anim = WebAnimation {
        coord,
        action,
        stages,
        pb: Playback::new(frames),
    };
    if speed == ANIM_SPEED_OFF || d.is_zero() {
        anim.pb.finished = true;
        anim.pb.current = anim.pb.frames.len();
    }
    anim
}

impl WebAnimation {
    /// `Coord`: where the web animation occurs.
    pub fn coord(&self) -> Coord {
        self.coord
    }
    /// `Action`: the performed [`WebAction`].
    pub fn action(&self) -> WebAction {
        self.action
    }
    /// `CurrentGlyph`: glyph of the current stage.
    pub fn current_glyph(&self) -> &'static str {
        if self.pb.current < self.stages.len() {
            return self.stages[self.pb.current];
        }
        self.stages.last().copied().unwrap_or(" . ")
    }
    /// `Done`: all stages completed.
    pub fn done(&self) -> bool {
        self.pb.done()
    }
    /// `Tick`: advances one stage.
    pub fn tick(&mut self) -> Frame {
        self.pb.step()
    }
}
