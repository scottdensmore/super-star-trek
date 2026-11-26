//! `lipgloss` borders.go.

use super::color::TerminalColor;
use super::size::get_lines;
use super::style::Style;
use crate::ansi;
use crate::termenv;
use crate::unicode;

/// Border runes (`lipgloss.Border`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Border {
    pub top: String,
    pub bottom: String,
    pub left: String,
    pub right: String,
    pub top_left: String,
    pub top_right: String,
    pub bottom_left: String,
    pub bottom_right: String,
    pub middle_left: String,
    pub middle_right: String,
    pub middle: String,
    pub middle_top: String,
    pub middle_bottom: String,
}

impl Border {
    pub fn get_top_size(&self) -> i32 {
        get_border_edge_width(&[&self.top_left, &self.top, &self.top_right])
    }
    pub fn get_right_size(&self) -> i32 {
        get_border_edge_width(&[&self.top_right, &self.right, &self.bottom_right])
    }
    pub fn get_bottom_size(&self) -> i32 {
        get_border_edge_width(&[&self.bottom_left, &self.bottom, &self.bottom_right])
    }
    pub fn get_left_size(&self) -> i32 {
        get_border_edge_width(&[&self.top_left, &self.left, &self.bottom_left])
    }
}

fn get_border_edge_width(parts: &[&str]) -> i32 {
    parts
        .iter()
        .map(|p| max_rune_width(p))
        .max()
        .unwrap_or(0)
        .max(0)
}

#[allow(clippy::too_many_arguments)]
fn b(
    top: &str,
    bottom: &str,
    left: &str,
    right: &str,
    top_left: &str,
    top_right: &str,
    bottom_left: &str,
    bottom_right: &str,
    middle_left: &str,
    middle_right: &str,
    middle: &str,
    middle_top: &str,
    middle_bottom: &str,
) -> Border {
    Border {
        top: top.into(),
        bottom: bottom.into(),
        left: left.into(),
        right: right.into(),
        top_left: top_left.into(),
        top_right: top_right.into(),
        bottom_left: bottom_left.into(),
        bottom_right: bottom_right.into(),
        middle_left: middle_left.into(),
        middle_right: middle_right.into(),
        middle: middle.into(),
        middle_top: middle_top.into(),
        middle_bottom: middle_bottom.into(),
    }
}

pub(crate) fn no_border() -> Border {
    Border::default()
}

/// `lipgloss.NormalBorder()`.
pub fn normal_border() -> Border {
    b(
        "─", "─", "│", "│", "┌", "┐", "└", "┘", "├", "┤", "┼", "┬", "┴",
    )
}
/// `lipgloss.RoundedBorder()`.
pub fn rounded_border() -> Border {
    b(
        "─", "─", "│", "│", "╭", "╮", "╰", "╯", "├", "┤", "┼", "┬", "┴",
    )
}
/// `lipgloss.BlockBorder()`.
pub fn block_border() -> Border {
    b(
        "█", "█", "█", "█", "█", "█", "█", "█", "█", "█", "█", "█", "█",
    )
}
/// `lipgloss.OuterHalfBlockBorder()`.
pub fn outer_half_block_border() -> Border {
    b("▀", "▄", "▌", "▐", "▛", "▜", "▙", "▟", "", "", "", "", "")
}
/// `lipgloss.InnerHalfBlockBorder()`.
pub fn inner_half_block_border() -> Border {
    b("▄", "▀", "▐", "▌", "▗", "▖", "▝", "▘", "", "", "", "", "")
}
/// `lipgloss.ThickBorder()`.
pub fn thick_border() -> Border {
    b(
        "━", "━", "┃", "┃", "┏", "┓", "┗", "┛", "┣", "┫", "╋", "┳", "┻",
    )
}
/// `lipgloss.DoubleBorder()`.
pub fn double_border() -> Border {
    b(
        "═", "═", "║", "║", "╔", "╗", "╚", "╝", "╠", "╣", "╬", "╦", "╩",
    )
}
/// `lipgloss.HiddenBorder()`.
pub fn hidden_border() -> Border {
    b(
        " ", " ", " ", " ", " ", " ", " ", " ", " ", " ", " ", " ", " ",
    )
}
/// `lipgloss.MarkdownBorder()`.
pub fn markdown_border() -> Border {
    b(
        "-", "-", "|", "|", "|", "|", "|", "|", "|", "|", "|", "|", "|",
    )
}
/// `lipgloss.ASCIIBorder()`.
pub fn ascii_border() -> Border {
    b(
        "-", "-", "|", "|", "+", "+", "+", "+", "+", "+", "+", "+", "+",
    )
}

impl Style {
    pub(crate) fn apply_border(&self, s: &str) -> String {
        let mut border = self.get_border_style();
        let (mut has_top, mut has_right, mut has_bottom, mut has_left, implicit) =
            self.border_flags();
        let [top_fg, right_fg, bottom_fg, left_fg, top_bg, right_bg, bottom_bg, left_bg] =
            self.border_colors();

        if implicit {
            has_top = true;
            has_right = true;
            has_bottom = true;
            has_left = true;
        }

        if border == no_border() || (!has_top && !has_right && !has_bottom && !has_left) {
            return s.to_string();
        }

        let (lines, mut width) = get_lines(s);

        if has_left {
            if border.left.is_empty() {
                border.left = " ".into();
            }
            width += max_rune_width(&border.left);
        }
        if has_right && border.right.is_empty() {
            border.right = " ".into();
        }
        if has_top && has_left && border.top_left.is_empty() {
            border.top_left = " ".into();
        }
        if has_top && has_right && border.top_right.is_empty() {
            border.top_right = " ".into();
        }
        if has_bottom && has_left && border.bottom_left.is_empty() {
            border.bottom_left = " ".into();
        }
        if has_bottom && has_right && border.bottom_right.is_empty() {
            border.bottom_right = " ".into();
        }

        if has_top {
            if !has_left && !has_right {
                border.top_left.clear();
                border.top_right.clear();
            } else if !has_left {
                border.top_left.clear();
            } else if !has_right {
                border.top_right.clear();
            }
        }
        if has_bottom {
            if !has_left && !has_right {
                border.bottom_left.clear();
                border.bottom_right.clear();
            } else if !has_left {
                border.bottom_left.clear();
            } else if !has_right {
                border.bottom_right.clear();
            }
        }

        border.top_left = first_rune_as_string(&border.top_left);
        border.top_right = first_rune_as_string(&border.top_right);
        border.bottom_right = first_rune_as_string(&border.bottom_right);
        border.bottom_left = first_rune_as_string(&border.bottom_left);

        let mut out = String::with_capacity(s.len() * 2);

        if has_top {
            let top =
                render_horizontal_edge(&border.top_left, &border.top, &border.top_right, width);
            out.push_str(&style_border(&top, &top_fg, &top_bg));
            out.push('\n');
        }

        let left_runes: Vec<char> = border.left.chars().collect();
        let mut left_index = 0;
        let right_runes: Vec<char> = border.right.chars().collect();
        let mut right_index = 0;

        let n = lines.len();
        for (i, l) in lines.iter().enumerate() {
            if has_left {
                let r = left_runes[left_index].to_string();
                left_index += 1;
                if left_index >= left_runes.len() {
                    left_index = 0;
                }
                out.push_str(&style_border(&r, &left_fg, &left_bg));
            }
            out.push_str(l);
            if has_right {
                let r = right_runes[right_index].to_string();
                right_index += 1;
                if right_index >= right_runes.len() {
                    right_index = 0;
                }
                out.push_str(&style_border(&r, &right_fg, &right_bg));
            }
            if i < n - 1 {
                out.push('\n');
            }
        }

        if has_bottom {
            let bottom = render_horizontal_edge(
                &border.bottom_left,
                &border.bottom,
                &border.bottom_right,
                width,
            );
            out.push('\n');
            out.push_str(&style_border(&bottom, &bottom_fg, &bottom_bg));
        }

        out
    }
}

fn render_horizontal_edge(left: &str, middle: &str, right: &str, width: i32) -> String {
    let middle = if middle.is_empty() { " " } else { middle };
    let left_width = ansi::string_width(left);
    let right_width = ansi::string_width(right);
    let runes: Vec<char> = middle.chars().collect();
    let mut j = 0;
    let mut out = String::new();
    out.push_str(left);
    let mut i = left_width + right_width;
    while i < width + right_width {
        out.push(runes[j]);
        j += 1;
        if j >= runes.len() {
            j = 0;
        }
        i += ansi::string_width(&runes[j].to_string());
    }
    out.push_str(right);
    out
}

/// `Style.styleBorder`: a zero `termenv.Style{}` (TrueColor profile!) with
/// the colours converted through the renderer's profile.
fn style_border(border: &str, fg: &TerminalColor, bg: &TerminalColor) -> String {
    if *fg == TerminalColor::NoColor && *bg == TerminalColor::NoColor {
        return border.to_string();
    }
    let mut style = termenv::Style::default();
    if *fg != TerminalColor::NoColor {
        style = style.foreground(fg.color());
    }
    if *bg != TerminalColor::NoColor {
        style = style.background(bg.color());
    }
    style.styled(border)
}

/// `maxRuneWidth`: widest uniseg grapheme cluster of `s`.
pub(crate) fn max_rune_width(s: &str) -> i32 {
    let mut width = 0;
    let mut rest = s;
    while !rest.is_empty() {
        let (_, r, w) = unicode::uniseg_first_grapheme_cluster(rest);
        if w > width {
            width = w;
        }
        rest = r;
    }
    width
}

fn first_rune_as_string(s: &str) -> String {
    s.chars().next().map(|c| c.to_string()).unwrap_or_default()
}
