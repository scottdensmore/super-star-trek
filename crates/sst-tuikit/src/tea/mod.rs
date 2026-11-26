//! Port of `github.com/charmbracelet/bubbletea` v1.3.10: the Elm-style
//! runtime, messages/commands, key and mouse decoding, the standard
//! renderer, and a deterministic [`harness`] for tests.
//!
//! Translation guide:
//!
//! | Go | Rust |
//! |---|---|
//! | `tea.Msg` | [`Msg`] (`Box<dyn Any>`) |
//! | `tea.Cmd` (nil-able) | `Option<`[`Cmd`]`>` |
//! | `func() tea.Msg { return X{} }` | `tea::cmd(move \|\| X {})` |
//! | `tea.Batch(a, b)` / `tea.Sequence(a, b)` | `tea::batch(vec![a, b])` / `tea::sequence(vec![a, b])` |
//! | `tea.Tick(d, func(t time.Time) tea.Msg { ... })` | `tea::tick(d, \|t\| ...)` |
//! | `tea.Quit` | `tea::quit()` |
//! | `switch msg := msg.(type) { case tea.KeyMsg: ... }` | `if let Some(k) = msg.downcast_ref::<KeyMsg>() { ... }` |
//! | `tea.KeyMsg{Type: tea.KeyEnter}` | `KeyMsg::new(KEY_ENTER)` |
//! | `msg.String() == "ctrl+c"` | `msg.string() == "ctrl+c"` |
//! | `tea.NewProgram(m, tea.WithAltScreen(), tea.WithMouseCellMotion()).Run()` | `Program::new(m, &[with_alt_screen(), with_mouse_cell_motion()]).run()` |

mod cmd;
pub mod harness;
pub mod input;
mod key;
mod mouse;
mod program;
pub mod renderer;

pub use cmd::*;
pub use key::*;
pub use mouse::*;
pub use program::{
    with_alt_screen, with_fps, with_mouse_all_motion, with_mouse_cell_motion, with_report_focus,
    without_bracketed_paste, without_catch_panics, without_signal_handler, Model, Program,
    ProgramError, ProgramOption, WITHOUT_BRACKETED_PASTE, WITHOUT_CATCH_PANICS,
    WITHOUT_SIGNAL_HANDLER, WITH_ALT_SCREEN, WITH_ANSI_COMPRESSOR, WITH_MOUSE_ALL_MOTION,
    WITH_MOUSE_CELL_MOTION, WITH_REPORT_FOCUS,
};
