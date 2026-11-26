//! sst-wasm: the WebAssembly edition of Super Star Trek (port of `cmd/wasm`).
//!
//! * [`session::Session`] (session.go): pure, natively testable command loop.
//! * [`formatter`] (formatter.go): ANSI/`\r\n` teletype output for xterm.js.
//! * [`bridge`] (bridge_js.go): the JS audio player behind an [`bridge::AudioHost`].
//! * `web` (bridge_js.go + main.go, wasm32 only): wasm-bindgen glue that
//!   registers `sstInit`, `sstCommand`, `sstSave`, `sstLoad` and `sstReset`.

pub mod bridge;
pub mod formatter;
pub mod session;

#[cfg(target_arch = "wasm32")]
pub mod web;

pub use formatter::*;
pub use session::Session;

#[cfg(target_arch = "wasm32")]
pub use web::register_bridge;

/// Go `registerBridge` on non-js targets (bridge_other.go): a no-op.
#[cfg(not(target_arch = "wasm32"))]
pub fn register_bridge() {}
