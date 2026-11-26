//! Port of the `github.com/charmbracelet/bubbles` v1.0.0 components used by
//! the game: [`textinput`] (with [`cursor`]), [`list`] and what it pulls in
//! ([`paginator`], [`help`], [`key`], [`spinner`], [`runeutil`]).
//!
//! Component `Update` methods take `&mut self` and `&Msg` (so a parent can
//! forward the same message to several children) and return the command:
//! Go `m.list, cmd = m.list.Update(msg)` -> `let cmd = m.list.update(&msg);`.

pub mod cursor;
pub mod help;
pub mod key;
pub mod list;
pub mod paginator;
pub mod runeutil;
pub mod spinner;
pub mod textinput;
