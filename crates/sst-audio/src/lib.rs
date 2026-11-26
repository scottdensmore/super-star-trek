//! sst-audio: procedural retro sound effects (port of Go `pkg/audio`).
//!
//! * [`SoundID`], the [`Player`] trait (shared as [`PlayerRef`]) and [`NullPlayer`] (sound.go)
//! * [`TerminalBellPlayer`] and, on non-wasm targets, `NativeOSPlayer` (player.go)
//! * [`synthesize_wav`] / [`synthesize_wav_with_volume`] (wav.go)
//! * [`Dispatcher`] mapping `sst_engine::Event`s to sounds (dispatcher.go)
//!
//! [`gomath`] holds bit-exact ports of the Go `math` functions the synthesizer needs.

pub mod dispatcher;
pub mod gomath;
pub mod player;
pub mod sound;
pub mod wav;

#[cfg(test)]
mod go_tests;

pub use dispatcher::*;
pub use player::*;
pub use sound::*;
pub use wav::{synthesize_wav, synthesize_wav_with_volume};
