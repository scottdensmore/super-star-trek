//! `lipgloss` align.go.

use super::position::{Position, BOTTOM, CENTER, RIGHT, TOP};
use super::size::get_lines;
use crate::ansi;
use crate::termenv;

/// `alignTextHorizontal`: pads every line to the widest line (or `width`).
pub(crate) fn align_text_horizontal(
    s: &str,
    pos: Position,
    width: i32,
    style: Option<&termenv::Style>,
) -> String {
    let (lines, widest_line) = get_lines(s);
    let mut b = String::with_capacity(s.len() + 16);
    let n = lines.len();
    for (i, l) in lines.iter().enumerate() {
        let line_width = ansi::string_width(l);
        let mut short_amount = widest_line - line_width;
        short_amount += 0.max(width - (short_amount + line_width));
        let mut l = l.to_string();
        if short_amount > 0 {
            if pos == RIGHT {
                let mut sp = " ".repeat(short_amount as usize);
                if let Some(st) = style {
                    sp = st.styled(&sp);
                }
                l = sp + &l;
            } else if pos == CENTER {
                let left = short_amount / 2;
                let right = left + short_amount % 2;
                let mut left_spaces = " ".repeat(left as usize);
                let mut right_spaces = " ".repeat(right as usize);
                if let Some(st) = style {
                    left_spaces = st.styled(&left_spaces);
                    right_spaces = st.styled(&right_spaces);
                }
                l = left_spaces + &l + &right_spaces;
            } else {
                let mut sp = " ".repeat(short_amount as usize);
                if let Some(st) = style {
                    sp = st.styled(&sp);
                }
                l += &sp;
            }
        }
        b.push_str(&l);
        if i < n - 1 {
            b.push('\n');
        }
    }
    b
}

/// `alignTextVertical`.
pub(crate) fn align_text_vertical(s: &str, pos: Position, height: i32) -> String {
    let str_height = s.matches('\n').count() as i32 + 1;
    if height < str_height {
        return s.to_string();
    }
    if pos == TOP {
        return s.to_string() + &"\n".repeat((height - str_height) as usize);
    }
    if pos == CENTER {
        let (mut top_padding, mut bottom_padding) =
            ((height - str_height) / 2, (height - str_height) / 2);
        if str_height + top_padding + bottom_padding > height {
            top_padding -= 1;
        } else if str_height + top_padding + bottom_padding < height {
            bottom_padding += 1;
        }
        return "\n".repeat(top_padding.max(0) as usize)
            + s
            + &"\n".repeat(bottom_padding.max(0) as usize);
    }
    if pos == BOTTOM {
        return "\n".repeat((height - str_height) as usize) + s;
    }
    s.to_string()
}
