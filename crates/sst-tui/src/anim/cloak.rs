//! Romulan cloak shimmer (port of `pkg/tui/anim/cloak.go`).

use std::time::Duration;

use sst_engine::rules::{ANIM_SPEED_NORMAL, ANIM_SPEED_OFF};
use sst_engine::types::Coord;
use sst_tuikit::lipgloss;

use super::{frame_duration, impl_animation, single, Frame, Playback};

/// A 4-stage Romulan shimmer between visible and cloaked (Go `CloakAnimation`).
#[derive(Clone)]
pub struct CloakAnimation {
    coord: Coord,
    cloaking: bool,
    stages: Vec<&'static str>,
    pb: Playback,
}
impl_animation!(CloakAnimation);

/// `NewCloakAnimation`: normal speed.
pub fn new_cloak_animation(coord: Coord, cloaking: bool) -> CloakAnimation {
    new_cloak_animation_with_speed(coord, cloaking, ANIM_SPEED_NORMAL)
}

/// `NewCloakAnimationWithSpeed`.
pub fn new_cloak_animation_with_speed(coord: Coord, cloaking: bool, speed: i32) -> CloakAnimation {
    let mut d = frame_duration(speed);
    if !(0..=3).contains(&speed) {
        d = Duration::from_millis(120);
    }

    let style_romulan = lipgloss::new_style()
        .foreground(lipgloss::color("10"))
        .bold(true);
    let style_shimmer = lipgloss::new_style()
        .foreground(lipgloss::color("11"))
        .italic(true);
    let style_phased = lipgloss::new_style().foreground(lipgloss::color("8"));
    let style_empty = lipgloss::new_style().foreground(lipgloss::color("240"));

    let (stages, styles) = if cloaking {
        (
            vec!["+R+", "~R~", "·?·", " . "],
            [style_romulan, style_shimmer, style_phased, style_empty],
        )
    } else {
        (
            vec![" . ", "·?·", "~R~", "+R+"],
            [style_empty, style_phased, style_shimmer, style_romulan],
        )
    };

    let frames: Vec<Frame> = stages
        .iter()
        .zip(styles)
        .map(|(g, s)| single(coord, g, s, d))
        .collect();

    let mut anim = CloakAnimation {
        coord,
        cloaking,
        stages,
        pb: Playback::new(frames),
    };
    if speed == ANIM_SPEED_OFF || d.is_zero() {
        anim.pb.finished = true;
        anim.pb.current = anim.pb.frames.len();
    }
    anim
}

impl CloakAnimation {
    /// `Coord`: where the shimmer is rendered.
    pub fn coord(&self) -> Coord {
        self.coord
    }
    /// `IsCloaking`: true when transitioning to cloaked.
    pub fn is_cloaking(&self) -> bool {
        self.cloaking
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
