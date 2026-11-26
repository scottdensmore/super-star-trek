//! Port of the `github.com/charmbracelet/x/ansi` v0.11.9 subset used by
//! lipgloss, bubbles and bubbletea: the escape-sequence state machine,
//! ANSI-aware width/strip/truncate/cut, `DecodeSequence`, SGR building and
//! the terminal control sequences the renderer emits.
//!
//! Go: `ansi.StringWidth(s)` -> [`string_width`], `ansi.Truncate(s, n, tail)`
//! -> [`truncate`], `ansi.TruncateLeft(s, n, prefix)` -> [`truncate_left`].

pub mod decode;
pub mod parser;
pub mod style;
mod width;

pub use decode::{decode_sequence, Param, Parser};
pub use style::{AnsiColor, SgrStyle, RESET_STYLE};
pub(crate) use width::bytes_to_string;
pub use width::{cut, string_width, strip, truncate, truncate_left};

/// `ansi.SetHyperlink(uri, params...)`.
pub fn set_hyperlink(uri: &str, params: &str) -> String {
    format!("\x1b]8;{params};{uri}\x07")
}

/// `ansi.ResetHyperlink()`.
pub fn reset_hyperlink() -> String {
    set_hyperlink("", "")
}

/// `ansi.CursorUp(n)`.
pub fn cursor_up(n: i32) -> String {
    if n > 1 {
        format!("\x1b[{n}A")
    } else {
        "\x1b[A".to_string()
    }
}

/// `ansi.CursorPosition(col, row)`.
pub fn cursor_position(col: i32, row: i32) -> String {
    if row <= 1 && col <= 1 {
        return CURSOR_HOME_POSITION.to_string();
    }
    let r = if row > 0 {
        row.to_string()
    } else {
        String::new()
    };
    let c = if col > 0 {
        col.to_string()
    } else {
        String::new()
    };
    format!("\x1b[{r};{c}H")
}

/// `ansi.SetWindowTitle`.
pub fn set_window_title(s: &str) -> String {
    format!("\x1b]2;{s}\x07")
}

pub const CURSOR_HOME_POSITION: &str = "\x1b[H";
pub const ERASE_SCREEN_BELOW: &str = "\x1b[J";
pub const ERASE_ENTIRE_SCREEN: &str = "\x1b[2J";
pub const ERASE_LINE_RIGHT: &str = "\x1b[K";
pub const ERASE_ENTIRE_LINE: &str = "\x1b[2K";
pub const SHOW_CURSOR: &str = "\x1b[?25h";
pub const HIDE_CURSOR: &str = "\x1b[?25l";
pub const SET_ALT_SCREEN_SAVE_CURSOR_MODE: &str = "\x1b[?1049h";
pub const RESET_ALT_SCREEN_SAVE_CURSOR_MODE: &str = "\x1b[?1049l";
pub const SET_BUTTON_EVENT_MOUSE_MODE: &str = "\x1b[?1002h";
pub const RESET_BUTTON_EVENT_MOUSE_MODE: &str = "\x1b[?1002l";
pub const SET_ANY_EVENT_MOUSE_MODE: &str = "\x1b[?1003h";
pub const RESET_ANY_EVENT_MOUSE_MODE: &str = "\x1b[?1003l";
pub const SET_FOCUS_EVENT_MODE: &str = "\x1b[?1004h";
pub const RESET_FOCUS_EVENT_MODE: &str = "\x1b[?1004l";
pub const SET_SGR_EXT_MOUSE_MODE: &str = "\x1b[?1006h";
pub const RESET_SGR_EXT_MOUSE_MODE: &str = "\x1b[?1006l";
pub const SET_BRACKETED_PASTE_MODE: &str = "\x1b[?2004h";
pub const RESET_BRACKETED_PASTE_MODE: &str = "\x1b[?2004l";
