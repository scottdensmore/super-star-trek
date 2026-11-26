//! Romulan plasma torpedo tracking animation (port of `pkg/tui/anim/plasma.go`).

use std::time::Duration;

use sst_engine::types::Coord;
use sst_tuikit::lipgloss;

use super::{bresenham_line, impl_animation, single, Frame, Playback};

/// Step-wise projectile along a Bresenham trajectory (Go `PlasmaAnimation`).
#[derive(Clone)]
pub struct PlasmaAnimation {
    from: Coord,
    to: Coord,
    path: Vec<Coord>,
    pb: Playback,
}
impl_animation!(PlasmaAnimation);

/// `NewPlasmaAnimation`. Go clamps a negative `stepDuration` to 100ms; a
/// Rust `Duration` cannot be negative, so callers converting from signed
/// values should map negatives to `Duration::from_millis(100)`.
pub fn new_plasma_animation(from: Coord, to: Coord, step_duration: Duration) -> PlasmaAnimation {
    let mut pts = bresenham_line(from, to);
    if pts.is_empty() {
        pts = vec![from];
    }

    let plasma_style = lipgloss::new_style()
        .foreground(lipgloss::color("208"))
        .bold(true);
    let frames: Vec<Frame> = pts
        .iter()
        .map(|&pt| single(pt, "*P*", plasma_style.clone(), step_duration))
        .collect();

    let mut anim = PlasmaAnimation {
        from,
        to,
        path: pts,
        pb: Playback::new(frames),
    };
    if step_duration.is_zero() {
        anim.pb.finished = true;
        anim.pb.current = anim.pb.frames.len();
    }
    anim
}

impl PlasmaAnimation {
    /// `From`: launch coordinate.
    pub fn from(&self) -> Coord {
        self.from
    }
    /// `To`: destination coordinate.
    pub fn to(&self) -> Coord {
        self.to
    }
    /// `Path`: the trajectory.
    pub fn path(&self) -> &[Coord] {
        &self.path
    }
    /// `CurrentCoord`: the projectile's current sector.
    pub fn current_coord(&self) -> Coord {
        if self.path.is_empty() {
            return self.to;
        }
        if self.pb.current < self.path.len() {
            return self.path[self.pb.current];
        }
        self.path[self.path.len() - 1]
    }
    /// `CurrentGlyph`.
    pub fn current_glyph(&self) -> &'static str {
        "*P*"
    }
    /// `Done`: all steps completed.
    pub fn done(&self) -> bool {
        self.pb.done()
    }
    /// `Tick`: advances one frame.
    pub fn tick(&mut self) -> Frame {
        self.pb.step()
    }
}
