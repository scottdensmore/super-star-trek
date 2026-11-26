//! Tactical grid animations (port of `pkg/tui/anim`: anim.go, line.go,
//! pulse.go, phaser.go, torpedo.go, cloak.go, plasma.go, web.go).
//!
//! Go's `Animation` interface becomes the [`Animation`] trait. Go
//! constructors return pointers, so an animation is shared mutable state:
//! store it as `Box<dyn Animation>` (or [`AnimationRef`] when the owning
//! model is cloned, matching Go's pointer aliasing).
//!
//! ```
//! use sst_tui::anim::{self, Animation};
//! let mut a: Box<dyn Animation> =
//!     Box::new(anim::new_torpedo_animation([1, 1], [1, 4], true, 2));
//! let f = a.step();
//! assert_eq!(f.overrides[&[1, 2]].glyph, " · ");
//! let _cmd = anim::tick_cmd(1, 1, f.duration); // fires anim::TickMsg { anim_id: 1, step: 1 }
//! ```

mod cloak;
mod line;
mod phaser;
mod plasma;
mod pulse;
mod torpedo;
mod web;

#[cfg(test)]
mod parity_tests;
#[cfg(test)]
mod tests;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::Duration;

use sst_engine::types::Coord;
use sst_tuikit::lipgloss::Style;
use sst_tuikit::tea;

pub use cloak::{new_cloak_animation, new_cloak_animation_with_speed, CloakAnimation};
pub use line::bresenham_line;
pub use phaser::{new_multi_phaser_animation, new_phaser_animation, PhaserAnimation};
pub use plasma::{new_plasma_animation, PlasmaAnimation};
pub use pulse::red_alert_badge_style;
pub use torpedo::{new_torpedo_animation, new_torpedo_animation_with_entity, TorpedoAnimation};
pub use web::{new_web_animation, new_web_animation_with_speed, WebAction, WebAnimation};

/// A temporary glyph and style replacing a grid cell (Go `CellOverride`).
#[derive(Clone, Default)]
pub struct CellOverride {
    /// Exact 3-rune cell glyph (e.g. " · ", " o ", "***", "---", " \\ ").
    pub glyph: String,
    pub style: Style,
}

/// Go `map[engine.Coord]CellOverride`. A Go nil map is an empty map.
pub type Overrides = BTreeMap<Coord, CellOverride>;

/// The overridden cells at one animation step (Go `Frame`). `Frame::default()`
/// is Go's `Frame{}` (returned once an animation is finished).
#[derive(Clone, Default)]
pub struct Frame {
    pub overrides: Overrides,
    pub duration: Duration,
}

/// An interactive visual sequence (Go `Animation` interface).
pub trait Animation {
    fn total_duration(&self) -> Duration;
    fn frames(&self) -> &[Frame];
    fn is_finished(&self) -> bool;
    fn step(&mut self) -> Frame;
    fn skip(&mut self) -> Frame;
}

/// A shared animation handle (Go's pointer semantics across model copies).
pub type AnimationRef = Rc<RefCell<dyn Animation>>;

/// Dispatched via `tea::tick` for frame advancement (Go `TickMsg`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TickMsg {
    pub anim_id: i32,
    pub step: i32,
}

/// `TickCmd`: schedules a [`TickMsg`] after `d`.
pub fn tick_cmd(anim_id: i32, step: i32, d: Duration) -> Option<tea::Cmd> {
    tea::tick(d, move |_| TickMsg { anim_id, step })
}

/// `FrameDuration`: 0 = Off (0ms), 1 = Fast (50ms), 2 = Normal (120ms),
/// 3 = Cinematic (240ms), anything else 120ms.
pub fn frame_duration(speed: i32) -> Duration {
    match speed {
        0 => Duration::ZERO,
        1 => Duration::from_millis(50),
        2 => Duration::from_millis(120),
        3 => Duration::from_millis(240),
        _ => Duration::from_millis(120),
    }
}

/// Shared frame-list playback state (the `frames/current/finished` fields
/// and identical method bodies of every Go animation type).
#[derive(Clone, Default)]
pub(crate) struct Playback {
    pub frames: Vec<Frame>,
    pub current: usize,
    pub finished: bool,
}

impl Playback {
    pub fn new(frames: Vec<Frame>) -> Self {
        Playback {
            frames,
            current: 0,
            finished: false,
        }
    }
    pub fn total_duration(&self) -> Duration {
        self.frames.iter().map(|f| f.duration).sum()
    }
    pub fn done(&self) -> bool {
        self.finished || self.current >= self.frames.len()
    }
    pub fn step(&mut self) -> Frame {
        if self.done() {
            self.finished = true;
            return Frame::default();
        }
        let f = self.frames[self.current].clone();
        self.current += 1;
        if self.current >= self.frames.len() {
            self.finished = true;
        }
        f
    }
    pub fn skip(&mut self) -> Frame {
        self.current = self.frames.len();
        self.finished = true;
        Frame::default()
    }
}

macro_rules! impl_animation {
    ($ty:ty) => {
        impl $crate::anim::Animation for $ty {
            fn total_duration(&self) -> ::std::time::Duration {
                self.pb.total_duration()
            }
            fn frames(&self) -> &[$crate::anim::Frame] {
                &self.pb.frames
            }
            fn is_finished(&self) -> bool {
                self.pb.done()
            }
            fn step(&mut self) -> $crate::anim::Frame {
                self.pb.step()
            }
            fn skip(&mut self) -> $crate::anim::Frame {
                self.pb.skip()
            }
        }
    };
}
pub(crate) use impl_animation;

/// One-cell frame helper.
pub(crate) fn single(coord: Coord, glyph: &str, style: Style, d: Duration) -> Frame {
    let mut overrides = Overrides::new();
    overrides.insert(
        coord,
        CellOverride {
            glyph: glyph.to_string(),
            style,
        },
    );
    Frame {
        overrides,
        duration: d,
    }
}
