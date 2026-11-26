//! Sound identifiers, the [`Player`] trait and [`NullPlayer`] (port of `pkg/audio/sound.go`).

use std::borrow::Cow;
use std::fmt;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

/// Identifies a distinct procedural retro sound effect (Go `type SoundID string`).
/// Any string is a valid id; unknown ids synthesize a fallback beep.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug, Default)]
pub struct SoundID(pub Cow<'static, str>);

impl SoundID {
    /// Builds an id from any string (Go `SoundID(s)`).
    pub fn new(s: impl Into<String>) -> Self {
        SoundID(Cow::Owned(s.into()))
    }

    /// Const constructor for static ids.
    pub const fn from_static(s: &'static str) -> Self {
        SoundID(Cow::Borrowed(s))
    }

    /// The underlying string (Go `string(id)`).
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SoundID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for SoundID {
    fn from(s: &str) -> Self {
        SoundID::new(s)
    }
}

impl From<String> for SoundID {
    fn from(s: String) -> Self {
        SoundID(Cow::Owned(s))
    }
}

pub const SOUND_PHASER: SoundID = SoundID::from_static("phaser");
pub const SOUND_TORPEDO_LAUNCH: SoundID = SoundID::from_static("torpedo_launch");
pub const SOUND_EXPLOSION: SoundID = SoundID::from_static("explosion");
pub const SOUND_RED_ALERT: SoundID = SoundID::from_static("red_alert");
pub const SOUND_DOCK: SoundID = SoundID::from_static("dock");
pub const SOUND_WARP: SoundID = SoundID::from_static("warp");
pub const SOUND_DAMAGE: SoundID = SoundID::from_static("damage");
pub const SOUND_SHIELDS: SoundID = SoundID::from_static("shields");
pub const SOUND_VICTORY: SoundID = SoundID::from_static("victory");
pub const SOUND_DEFEAT: SoundID = SoundID::from_static("defeat");
pub const SOUND_CLOAK: SoundID = SoundID::from_static("cloak");
pub const SOUND_DECLOAK: SoundID = SoundID::from_static("decloak");
pub const SOUND_PLASMA_LAUNCH: SoundID = SoundID::from_static("plasma_launch");
pub const SOUND_PLASMA_IMPACT: SoundID = SoundID::from_static("plasma_impact");
pub const SOUND_THOLIAN_WEB: SoundID = SoundID::from_static("tholian_web");
pub const SOUND_WEB_BREACHED: SoundID = SoundID::from_static("web_breached");
pub const SOUND_POINT_DEFENSE: SoundID = SoundID::from_static("point_defense");
pub const SOUND_COMM_CHIME: SoundID = SoundID::from_static("comm_chime");
pub const SOUND_COMPUTER_BEEP: SoundID = SoundID::from_static("computer_beep");

/// Playback interface for sound effects (Go `Player` interface).
///
/// Methods take `&self`; implementations use interior mutability so a player
/// can be shared as a [`PlayerRef`].
pub trait Player {
    fn play(&self, sound: SoundID);
    fn set_muted(&self, muted: bool);
    fn is_muted(&self) -> bool;
    /// Sets the volume; implementations clamp to 0..=100.
    fn set_volume(&self, vol: i32);
    fn volume(&self) -> i32;
}

/// Shared player handle (the Go code passes `Player` interface values around freely).
pub type PlayerRef = Rc<dyn Player>;

#[inline]
pub(crate) fn clamp_volume(vol: i32) -> i32 {
    vol.clamp(0, 100)
}

/// No-op [`Player`] (Go `NullPlayer`).
#[derive(Debug)]
pub struct NullPlayer {
    muted: AtomicBool,
    volume: AtomicI32,
}

impl NullPlayer {
    /// Go `NewNullPlayer`: unmuted, volume 100.
    pub fn new() -> Self {
        NullPlayer {
            muted: AtomicBool::new(false),
            volume: AtomicI32::new(100),
        }
    }
}

impl Default for NullPlayer {
    fn default() -> Self {
        Self::new()
    }
}

impl Player for NullPlayer {
    fn play(&self, _sound: SoundID) {}

    fn set_muted(&self, muted: bool) {
        self.muted.store(muted, Ordering::SeqCst);
    }

    fn is_muted(&self) -> bool {
        self.muted.load(Ordering::SeqCst)
    }

    fn set_volume(&self, vol: i32) {
        self.volume.store(clamp_volume(vol), Ordering::SeqCst);
    }

    fn volume(&self) -> i32 {
        self.volume.load(Ordering::SeqCst)
    }
}
