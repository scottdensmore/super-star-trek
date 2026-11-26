//! Port of `github.com/charmbracelet/lipgloss` v1.1.0.
//!
//! Translation guide (Go -> Rust):
//!
//! | Go | Rust |
//! |---|---|
//! | `lipgloss.NewStyle()` | `lipgloss::new_style()` |
//! | `.Foreground(lipgloss.Color("#F00"))` | `.foreground(lipgloss::color("#F00"))` (or `.foreground("#F00")`) |
//! | `.Padding(0, 1)` / `.Margin(1)` | `.padding(&[0, 1])` / `.margin(&[1])` |
//! | `.Border(lipgloss.NormalBorder(), false, false, false, true)` | `.border(lipgloss::normal_border(), &[false, false, false, true])` |
//! | `.Border(b)` | `.border(b, &[])` |
//! | `.BorderForeground(c)` | `.border_foreground(&[c])` |
//! | `.Align(lipgloss.Center)` | `.align(&[lipgloss::CENTER])` |
//! | `s.Render(str)` / `s.Render(a, b)` | `s.render(str)` / `s.render_strs(&[a, b])` |
//! | `lipgloss.JoinHorizontal(lipgloss.Top, a, b)` | `lipgloss::join_horizontal(lipgloss::TOP, &[a, b])` |
//! | `lipgloss.Place(w, h, lipgloss.Center, lipgloss.Center, s)` | `lipgloss::place(w, h, CENTER, CENTER, s, &[])` |
//! | `lipgloss.Width(s)` | `lipgloss::width(s)` |
//! | `lipgloss.SetColorProfile(termenv.Ascii)` | `lipgloss::set_color_profile(termenv::Ascii)` |
//!
//! Styles are values: every setter takes `self` and returns the modified
//! copy, so Go's value semantics (including "lost" mutations on copies) carry
//! over directly.

mod align;
mod borders;
mod color;
mod join;
mod position;
mod renderer;
mod runes;
mod size;
mod style;
mod whitespace;

pub use borders::{
    ascii_border, block_border, double_border, hidden_border, inner_half_block_border,
    markdown_border, normal_border, outer_half_block_border, rounded_border, thick_border, Border,
};
pub use color::{adaptive_color, color, CompleteColor, TerminalColor};
pub use join::{join_horizontal, join_vertical};
pub use position::{
    place, place_horizontal, place_vertical, Position, BOTTOM, CENTER, LEFT, RIGHT, TOP,
};
pub use renderer::{
    color_profile, has_dark_background, set_color_profile, set_has_dark_background,
};
pub use runes::{new_range, style_ranges, style_runes, Range};
pub use size::{height, size, width};
pub use style::{new_style, Style, TransformFn, NO_TAB_CONVERSION};
pub use whitespace::{
    with_whitespace_background, with_whitespace_chars, with_whitespace_foreground, WhitespaceOption,
};
