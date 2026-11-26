//! `x/ansi` width.go / truncate.go: ANSI-aware width, stripping and
//! truncation. Everything works on bytes exactly like Go; results are
//! converted back to `String` (lossily only if Go itself would have produced
//! invalid UTF-8, which never happens for valid input).

use super::parser::{self, table};
use crate::unicode;

pub(crate) fn bytes_to_string(v: Vec<u8>) -> String {
    match String::from_utf8(v) {
        Ok(s) => s,
        Err(e) => String::from_utf8_lossy(e.as_bytes()).into_owned(),
    }
}

/// `utf8ByteLen`.
pub(crate) fn utf8_byte_len(b: u8) -> i32 {
    if b <= 0b0111_1111 {
        1
    } else if (0b1100_0000..=0b1101_1111).contains(&b) {
        2
    } else if (0b1110_0000..=0b1110_1111).contains(&b) {
        3
    } else if (0b1111_0000..=0b1111_0111).contains(&b) {
        4
    } else {
        -1
    }
}

/// `FirstGraphemeCluster(b, GraphemeWidth)`: byte length and display width
/// of the first grapheme cluster of `b`.
pub(crate) fn first_grapheme_cluster(b: &[u8]) -> (usize, i32) {
    let n = unicode::first_grapheme_len_bytes(b);
    let w = match std::str::from_utf8(&b[..n]) {
        Ok(s) => unicode::displaywidth_string(s),
        Err(_) => 0,
    };
    (n, w)
}

/// `ansi.Strip`: removes ANSI escape sequences.
pub fn strip(s: &str) -> String {
    let b = s.as_bytes();
    let mut buf = Vec::with_capacity(b.len());
    let mut ri = 0;
    let mut rw = 0;
    let mut pstate = parser::GROUND_STATE;
    let t = table();
    for &c in b {
        if pstate == parser::UTF8_STATE {
            buf.push(c);
            ri += 1;
            if ri < rw {
                continue;
            }
            pstate = parser::GROUND_STATE;
            ri = 0;
            rw = 0;
            continue;
        }
        let (state, action) = t.transition(pstate, c);
        match action {
            parser::COLLECT_ACTION => {
                if state == parser::UTF8_STATE {
                    rw = utf8_byte_len(c);
                    buf.push(c);
                    ri += 1;
                }
            }
            parser::PRINT_ACTION | parser::EXECUTE_ACTION => buf.push(c),
            _ => {}
        }
        if pstate != parser::UTF8_STATE {
            pstate = state;
        }
    }
    bytes_to_string(buf)
}

/// `ansi.StringWidth`: display width in cells, ignoring escape sequences.
pub fn string_width(s: &str) -> i32 {
    if s.is_empty() {
        return 0;
    }
    let b = s.as_bytes();
    let t = table();
    let mut pstate = parser::GROUND_STATE;
    let mut width = 0;
    let mut i = 0;
    while i < b.len() {
        let (state, action) = t.transition(pstate, b[i]);
        if action == parser::PRINT_ACTION || state == parser::UTF8_STATE {
            let (n, w) = first_grapheme_cluster(&b[i..]);
            width += w;
            i += n.max(1);
            pstate = parser::GROUND_STATE;
            continue;
        }
        if action == parser::PRINT_ACTION {
            width += 1;
        }
        pstate = state;
        i += 1;
    }
    width
}

/// `ansi.Truncate`.
pub fn truncate(s: &str, length: i32, tail: &str) -> String {
    if string_width(s) <= length {
        return s.to_string();
    }
    let tw = string_width(tail);
    let length = length - tw;
    if length < 0 {
        return String::new();
    }
    let b = s.as_bytes();
    let t = table();
    let mut buf: Vec<u8> = Vec::with_capacity(b.len());
    let mut cur_width = 0;
    let mut ignoring = false;
    let mut pstate = parser::GROUND_STATE;
    let mut i = 0;
    while i < b.len() {
        let (state, action) = t.transition(pstate, b[i]);
        if state == parser::UTF8_STATE {
            let (n, width) = first_grapheme_cluster(&b[i..]);
            let n = n.max(1);
            let cluster = &b[i..i + n];
            i += n;
            cur_width += width;
            if ignoring {
                continue;
            }
            if cur_width > length && !ignoring {
                ignoring = true;
                buf.extend_from_slice(tail.as_bytes());
            }
            if cur_width > length {
                continue;
            }
            buf.extend_from_slice(cluster);
            pstate = parser::GROUND_STATE;
            continue;
        }
        match action {
            parser::PRINT_ACTION => {
                if cur_width >= length && !ignoring {
                    ignoring = true;
                    buf.extend_from_slice(tail.as_bytes());
                }
                if ignoring {
                    i += 1;
                    continue;
                }
                cur_width += 1;
                buf.push(b[i]);
                i += 1;
            }
            parser::EXECUTE_ACTION => {
                if ignoring {
                    i += 1;
                    continue;
                }
                buf.push(b[i]);
                i += 1;
            }
            _ => {
                buf.push(b[i]);
                i += 1;
            }
        }
        pstate = state;
        if cur_width > length && !ignoring {
            ignoring = true;
            buf.extend_from_slice(tail.as_bytes());
        }
    }
    bytes_to_string(buf)
}

/// `ansi.TruncateLeft`: removes `n` cells from the left, adding `prefix`.
pub fn truncate_left(s: &str, n: i32, prefix: &str) -> String {
    if n <= 0 {
        return s.to_string();
    }
    let b = s.as_bytes();
    let t = table();
    let mut buf: Vec<u8> = Vec::with_capacity(b.len());
    let mut cur_width = 0;
    let mut ignoring = true;
    let mut pstate = parser::GROUND_STATE;
    let mut i = 0;
    while i < b.len() {
        if !ignoring {
            buf.extend_from_slice(&b[i..]);
            break;
        }
        let (state, action) = t.transition(pstate, b[i]);
        if state == parser::UTF8_STATE {
            let (len, width) = first_grapheme_cluster(&b[i..]);
            let len = len.max(1);
            let cluster = &b[i..i + len];
            i += len;
            cur_width += width;
            if cur_width > n && ignoring {
                ignoring = false;
                buf.extend_from_slice(prefix.as_bytes());
            }
            if cur_width > n {
                buf.extend_from_slice(cluster);
            }
            if ignoring {
                continue;
            }
            pstate = parser::GROUND_STATE;
            continue;
        }
        match action {
            parser::PRINT_ACTION => {
                cur_width += 1;
                if cur_width > n && ignoring {
                    ignoring = false;
                    buf.extend_from_slice(prefix.as_bytes());
                }
                if ignoring {
                    i += 1;
                    continue;
                }
                buf.push(b[i]);
                i += 1;
            }
            parser::EXECUTE_ACTION => {
                if ignoring {
                    i += 1;
                    continue;
                }
                buf.push(b[i]);
                i += 1;
            }
            _ => {
                buf.push(b[i]);
                i += 1;
            }
        }
        pstate = state;
        if cur_width > n && ignoring {
            ignoring = false;
            buf.extend_from_slice(prefix.as_bytes());
        }
    }
    bytes_to_string(buf)
}

/// `ansi.Cut`: the cells `[left, right)` of `s`.
pub fn cut(s: &str, left: i32, right: i32) -> String {
    if right <= left {
        return String::new();
    }
    if left == 0 {
        return truncate(s, right, "");
    }
    truncate_left(&truncate(s, right, ""), left, "")
}
