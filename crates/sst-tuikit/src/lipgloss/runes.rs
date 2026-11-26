//! `lipgloss` runes.go and ranges.go.

use super::style::Style;
use crate::ansi;
use std::collections::HashSet;

/// `lipgloss.StyleRunes`: styles the runes at `indices` with `matched`, the
/// rest with `unmatched`.
pub fn style_runes(s: &str, indices: &[i32], matched: &Style, unmatched: &Style) -> String {
    let m: HashSet<i32> = indices.iter().copied().collect();
    let runes: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut group = String::new();
    for (i, r) in runes.iter().enumerate() {
        group.push(*r);
        let matches = m.contains(&(i as i32));
        let next_matches = m.contains(&(i as i32 + 1));
        if matches != next_matches || i == runes.len() - 1 {
            let style = if matches { matched } else { unmatched };
            out.push_str(&style.render(&group));
            group.clear();
        }
    }
    out
}

/// A styled range of cells (`lipgloss.Range`).
#[derive(Clone, Debug)]
pub struct Range {
    pub start: i32,
    pub end: i32,
    pub style: Style,
}

/// `lipgloss.NewRange`.
pub fn new_range(start: i32, end: i32, style: Style) -> Range {
    Range { start, end, style }
}

/// `lipgloss.StyleRanges`.
pub fn style_ranges(s: &str, ranges: &[Range]) -> String {
    if ranges.is_empty() {
        return s.to_string();
    }
    let mut buf = String::new();
    let mut last_idx = 0;
    let stripped = ansi::strip(s);
    for rng in ranges {
        if rng.start > last_idx {
            buf.push_str(&ansi::cut(s, last_idx, rng.start));
        }
        buf.push_str(&rng.style.render(&ansi::cut(&stripped, rng.start, rng.end)));
        last_idx = rng.end;
    }
    buf.push_str(&ansi::truncate_left(s, last_idx, ""));
    buf
}
