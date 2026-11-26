//! `lipgloss` join.go.

use super::position::{go_round, Position, BOTTOM, LEFT, RIGHT, TOP};
use super::size::get_lines;
use crate::ansi;

/// `lipgloss.JoinHorizontal`.
pub fn join_horizontal(pos: Position, strs: &[&str]) -> String {
    if strs.is_empty() {
        return String::new();
    }
    if strs.len() == 1 {
        return strs[0].to_string();
    }
    let mut blocks: Vec<Vec<&str>> = Vec::with_capacity(strs.len());
    let mut max_widths = Vec::with_capacity(strs.len());
    let mut max_height = 0;
    for s in strs {
        let (lines, w) = get_lines(s);
        if lines.len() > max_height {
            max_height = lines.len();
        }
        blocks.push(lines);
        max_widths.push(w);
    }
    for block in blocks.iter_mut() {
        if block.len() >= max_height {
            continue;
        }
        let n = max_height - block.len();
        if pos == TOP {
            block.extend(std::iter::repeat_n("", n));
        } else if pos == BOTTOM {
            let mut v: Vec<&str> = vec![""; n];
            v.append(block);
            *block = v;
        } else {
            let split = go_round(n as f64 * pos.value()) as usize;
            let top = n - split;
            let bottom = n - top;
            // Go: append(extraLines[top:], ...) then append(extraLines[bottom:])
            let mut v: Vec<&str> = vec![""; n - top];
            v.append(block);
            v.extend(std::iter::repeat_n("", n - bottom));
            *block = v;
        }
    }
    let mut b = String::new();
    let rows = blocks[0].len();
    for i in 0..rows {
        for (j, block) in blocks.iter().enumerate() {
            b.push_str(block[i]);
            let pad = max_widths[j] - ansi::string_width(block[i]);
            b.push_str(&" ".repeat(pad.max(0) as usize));
        }
        if i < rows - 1 {
            b.push('\n');
        }
    }
    b
}

/// `lipgloss.JoinVertical`.
pub fn join_vertical(pos: Position, strs: &[&str]) -> String {
    if strs.is_empty() {
        return String::new();
    }
    if strs.len() == 1 {
        return strs[0].to_string();
    }
    let mut blocks = Vec::with_capacity(strs.len());
    let mut max_width = 0;
    for s in strs {
        let (lines, w) = get_lines(s);
        if w > max_width {
            max_width = w;
        }
        blocks.push(lines);
    }
    let mut b = String::new();
    let nb = blocks.len();
    for (i, block) in blocks.iter().enumerate() {
        let nl = block.len();
        for (j, line) in block.iter().enumerate() {
            let w = max_width - ansi::string_width(line);
            if pos == LEFT {
                b.push_str(line);
                b.push_str(&" ".repeat(w.max(0) as usize));
            } else if pos == RIGHT {
                b.push_str(&" ".repeat(w.max(0) as usize));
                b.push_str(line);
            } else if w < 1 {
                b.push_str(line);
            } else {
                let split = go_round(w as f64 * pos.value());
                let right = w - split;
                let left = w - right;
                b.push_str(&" ".repeat(left.max(0) as usize));
                b.push_str(line);
                b.push_str(&" ".repeat(right.max(0) as usize));
            }
            if !(i == nb - 1 && j == nl - 1) {
                b.push('\n');
            }
        }
    }
    b
}
