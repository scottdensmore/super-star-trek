//! `lipgloss` position.go: positions and `Place*`.

use super::size::get_lines;
use super::whitespace::{new_whitespace, WhitespaceOption};
use crate::ansi;

/// A relative position (`lipgloss.Position`, 0.0 = top/left, 1.0 =
/// bottom/right).
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct Position(pub f64);

impl Position {
    pub(crate) fn value(self) -> f64 {
        crate::gomath::min(1.0, crate::gomath::max(0.0, self.0))
    }
}

pub const TOP: Position = Position(0.0);
pub const BOTTOM: Position = Position(1.0);
pub const CENTER: Position = Position(0.5);
pub const LEFT: Position = Position(0.0);
pub const RIGHT: Position = Position(1.0);

/// Go `int(math.Round(x))`.
pub(crate) fn go_round(x: f64) -> i32 {
    x.round() as i32
}

/// `lipgloss.Place`.
pub fn place(
    width: i32,
    height: i32,
    h_pos: Position,
    v_pos: Position,
    s: &str,
    opts: &[WhitespaceOption],
) -> String {
    place_vertical(
        height,
        v_pos,
        &place_horizontal(width, h_pos, s, opts),
        opts,
    )
}

/// `lipgloss.PlaceHorizontal`.
pub fn place_horizontal(width: i32, pos: Position, s: &str, opts: &[WhitespaceOption]) -> String {
    let (lines, content_width) = get_lines(s);
    let gap = width - content_width;
    if gap <= 0 {
        return s.to_string();
    }
    let ws = new_whitespace(opts);
    let mut b = String::new();
    let n = lines.len();
    for (i, l) in lines.iter().enumerate() {
        let short = 0.max(content_width - ansi::string_width(l));
        if pos == LEFT {
            b.push_str(l);
            b.push_str(&ws.render(gap + short));
        } else if pos == RIGHT {
            b.push_str(&ws.render(gap + short));
            b.push_str(l);
        } else {
            let total_gap = gap + short;
            let split = go_round(total_gap as f64 * pos.value());
            let left = total_gap - split;
            let right = total_gap - left;
            b.push_str(&ws.render(left));
            b.push_str(l);
            b.push_str(&ws.render(right));
        }
        if i < n - 1 {
            b.push('\n');
        }
    }
    b
}

/// `lipgloss.PlaceVertical`.
pub fn place_vertical(height: i32, pos: Position, s: &str, opts: &[WhitespaceOption]) -> String {
    let content_height = s.matches('\n').count() as i32 + 1;
    let gap = height - content_height;
    if gap <= 0 {
        return s.to_string();
    }
    let ws = new_whitespace(opts);
    let (_, width) = get_lines(s);
    let empty_line = ws.render(width);
    let mut b = String::new();
    if pos == TOP {
        b.push_str(s);
        b.push('\n');
        for i in 0..gap {
            b.push_str(&empty_line);
            if i < gap - 1 {
                b.push('\n');
            }
        }
    } else if pos == BOTTOM {
        b.push_str(&format!("{empty_line}\n").repeat(gap as usize));
        b.push_str(s);
    } else {
        let split = go_round(gap as f64 * pos.value());
        let top = gap - split;
        let bottom = gap - top;
        b.push_str(&format!("{empty_line}\n").repeat(top.max(0) as usize));
        b.push_str(s);
        for _ in 0..bottom {
            b.push('\n');
            b.push_str(&empty_line);
        }
    }
    b
}
