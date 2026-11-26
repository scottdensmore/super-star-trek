//! The JS audio player of `cmd/wasm/bridge_js.go`, written against a small
//! [`AudioHost`] trait so its semantics can be tested natively. The wasm32
//! build plugs in the real `window.sst*` functions (see `crate::web`).

use std::cell::Cell;

use sst_audio::{Player, SoundID};

/// The optional JS audio hooks (`window.sstPlaySound`, `sstSetMuted`, ...).
/// Each method mirrors one `getJSFunction(name)` lookup plus `Invoke`.
pub trait AudioHost {
    /// Calls `sstPlaySound(id)` if it exists.
    fn play_sound(&self, id: &str);
    /// Calls `sstSetMuted(muted)` if it exists.
    fn set_muted(&self, muted: bool);
    /// `sstIsMuted()` when it exists and returns a boolean.
    fn is_muted(&self) -> Option<bool>;
    /// Calls `sstSetVolume(vol)` if it exists.
    fn set_volume(&self, vol: i32);
    /// `sstGetVolume()` when it exists and returns a number (Go `Value.Int`).
    fn get_volume(&self) -> Option<i32>;
}

/// A host with none of the JS functions defined.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoHost;

impl AudioHost for NoHost {
    fn play_sound(&self, _id: &str) {}
    fn set_muted(&self, _muted: bool) {}
    fn is_muted(&self) -> Option<bool> {
        None
    }
    fn set_volume(&self, _vol: i32) {}
    fn get_volume(&self) -> Option<i32> {
        None
    }
}

/// Go `jsAudioPlayer`: delegates to the JS hooks and falls back to local state.
pub struct JsAudioPlayer<H: AudioHost> {
    host: H,
    muted: Cell<bool>,
    volume: Cell<i32>,
}

impl<H: AudioHost> JsAudioPlayer<H> {
    /// Go `&jsAudioPlayer{volume: volume}`.
    pub fn new(host: H, volume: i32) -> Self {
        JsAudioPlayer {
            host,
            muted: Cell::new(false),
            volume: Cell::new(volume),
        }
    }

    /// The host the player calls into.
    pub fn host(&self) -> &H {
        &self.host
    }
}

impl<H: AudioHost> Player for JsAudioPlayer<H> {
    fn play(&self, sound: SoundID) {
        if self.is_muted() {
            return;
        }
        self.host.play_sound(sound.as_str());
    }

    fn set_muted(&self, muted: bool) {
        self.muted.set(muted);
        self.host.set_muted(muted);
    }

    fn is_muted(&self) -> bool {
        self.host.is_muted().unwrap_or_else(|| self.muted.get())
    }

    fn set_volume(&self, vol: i32) {
        let vol = vol.clamp(0, 100);
        self.volume.set(vol);
        self.host.set_volume(vol);
    }

    fn volume(&self) -> i32 {
        self.host.get_volume().unwrap_or_else(|| self.volume.get())
    }
}

impl<H: AudioHost + ?Sized> AudioHost for std::rc::Rc<H> {
    fn play_sound(&self, id: &str) {
        (**self).play_sound(id)
    }
    fn set_muted(&self, muted: bool) {
        (**self).set_muted(muted)
    }
    fn is_muted(&self) -> Option<bool> {
        (**self).is_muted()
    }
    fn set_volume(&self, vol: i32) {
        (**self).set_volume(vol)
    }
    fn get_volume(&self) -> Option<i32> {
        (**self).get_volume()
    }
}
