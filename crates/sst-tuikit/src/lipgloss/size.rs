//! `lipgloss` size.go and `getLines`.

use crate::ansi;

/// `lipgloss.Width`: the cell width of the widest line.
pub fn width(s: &str) -> i32 {
    s.split('\n')
        .map(ansi::string_width)
        .max()
        .unwrap_or(0)
        .max(0)
}

/// `lipgloss.Height`: the number of lines.
pub fn height(s: &str) -> i32 {
    s.matches('\n').count() as i32 + 1
}

/// `lipgloss.Size`.
pub fn size(s: &str) -> (i32, i32) {
    (width(s), height(s))
}

/// `getLines`: split lines and the widest line width.
pub(crate) fn get_lines(s: &str) -> (Vec<&str>, i32) {
    let lines: Vec<&str> = s.split('\n').collect();
    let mut widest = 0;
    for l in &lines {
        let w = ansi::string_width(l);
        if widest < w {
            widest = w;
        }
    }
    (lines, widest)
}
