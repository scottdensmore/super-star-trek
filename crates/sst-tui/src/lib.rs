//! sst-tui: the Bubble Tea TUI (port of `pkg/tui/**` and `pkg/config`).
//!
//! The root model lives in [`model`] (model.go), [`update`] (update.go and
//! parser.go) and [`view`] (view.go); everything is re-exported at the crate root.

pub mod anim;
pub mod components;
pub mod config;
pub mod model;
pub mod theme;
pub mod update;
pub mod view;

pub use model::*;
pub use update::*;
pub use view::*;
