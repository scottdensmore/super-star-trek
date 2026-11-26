//! Super Star Trek game engine: pure game rules, state and deterministic
//! simulation (port of the Go `pkg/engine` package, plus `pkg/tui/parser`).
//! No terminal I/O; filesystem / environment / wall-clock access is isolated
//! in [`persist`].
//!
//! Shared Go pointers: `*GameState` -> [`GameRef`] (`Rc<RefCell<GameState>>`),
//! `*TourState` -> [`TourRef`], `[]*Klingon` etc. -> `GoVec<Ptr<T>>`.

pub mod json;

pub mod actions;
pub mod anomalies;
pub mod campaign;
pub mod combat;
pub mod engine;
pub mod events;
pub mod geometry;
pub mod gofmt;
pub mod gomath;
pub mod gopath;
pub mod gotime;
pub mod klingon_pack;
pub mod leaderboard;
pub mod parser;
pub mod persist;
pub mod prng;
pub mod refits;
pub mod romulan;
pub mod rules;
pub mod save;
pub mod scenario_kobayashi;
pub mod scenario_mutara;
pub mod scenario_siege;
pub mod scenarios;
pub mod score;
pub mod state;
pub mod tactics;
pub mod tholian;
pub mod types;

pub use actions::*;
pub use anomalies::*;
pub use campaign::*;
pub use combat::*;
pub use engine::*;
pub use events::*;
pub use geometry::*;
pub use klingon_pack::*;
pub use leaderboard::*;
pub use prng::PRNG;
pub use refits::*;
pub use romulan::*;
pub use rules::*;
pub use save::*;
pub use scenario_kobayashi::*;
pub use scenario_mutara::*;
pub use scenario_siege::*;
pub use scenarios::*;
pub use score::*;
pub use state::*;
pub use tactics::*;
pub use tholian::*;
pub use types::*;
