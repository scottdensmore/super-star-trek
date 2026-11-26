//! Terminal input decoding (`key.go` `readAnsiInputs`/`detectOneMsg`,
//! `key_sequences.go`, `mouse.go` parsers, focus and bracketed paste).

use std::fmt;

use super::cmd::{BlurMsg, FocusMsg};
use super::key::{ext_sequences, KeyMsg, KEY_ESCAPE, KEY_NUL, KEY_RUNES, KEY_SPACE};
use super::mouse::{
    parse_mouse_button, MouseMsg, MOUSE_ACTION_MOTION, MOUSE_ACTION_RELEASE, MOUSE_RELEASE,
};

/// A decoded input message. All variants are `Send` so the reader thread can
/// pass them to the event loop; [`InputMsg::into_msg`] boxes them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InputMsg {
    Key(KeyMsg),
    Mouse(MouseMsg),
    Focus,
    Blur,
    /// bubbletea's `unknownCSISequenceMsg`.
    UnknownCsi(Vec<u8>),
    /// bubbletea's `unknownInputByteMsg`.
    UnknownByte(u8),
}

/// bubbletea's internal `unknownCSISequenceMsg`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnknownCsiSequenceMsg(pub Vec<u8>);

impl fmt::Display for UnknownCsiSequenceMsg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // fmt.Sprintf("?CSI%+v?", []byte(u)[2:])
        let tail: Vec<String> = self.0.iter().skip(2).map(|b| b.to_string()).collect();
        write!(f, "?CSI[{}]?", tail.join(" "))
    }
}

/// bubbletea's internal `unknownInputByteMsg`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnknownInputByteMsg(pub u8);

impl fmt::Display for UnknownInputByteMsg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // fmt.Sprintf("?%#02x?", int(u))
        write!(f, "?{:#x}?", self.0)
    }
}

impl InputMsg {
    /// Converts to a boxed [`super::Msg`].
    pub fn into_msg(self) -> super::Msg {
        match self {
            InputMsg::Key(k) => Box::new(k),
            InputMsg::Mouse(m) => Box::new(m),
            InputMsg::Focus => Box::new(FocusMsg),
            InputMsg::Blur => Box::new(BlurMsg),
            InputMsg::UnknownCsi(v) => Box::new(UnknownCsiSequenceMsg(v)),
            InputMsg::UnknownByte(b) => Box::new(UnknownInputByteMsg(b)),
        }
    }
}

/// Go `utf8.DecodeRune` on bytes: (rune or None for RuneError, width).
fn decode_rune(b: &[u8]) -> (Option<char>, usize) {
    if b.is_empty() {
        return (None, 0);
    }
    let n = match b[0] {
        0x00..=0x7F => 1,
        0xC2..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xF4 => 4,
        _ => return (None, 1),
    };
    if b.len() < n {
        return (None, 1);
    }
    match std::str::from_utf8(&b[..n]) {
        Ok(s) => {
            let c = s.chars().next().unwrap_or('\u{FFFD}');
            if c == '\u{FFFD}' {
                (None, n)
            } else {
                (Some(c), n)
            }
        }
        Err(_) => (None, 1),
    }
}

/// `mouseSGRRegex` `(\d+);(\d+);(\d+)([Mm])` (unanchored, leftmost match):
/// returns (match end, [b, x, y], release).
fn find_sgr_mouse(b: &[u8]) -> Option<(usize, [i64; 3], bool)> {
    let digits = |mut i: usize| -> Option<(usize, i64)> {
        let start = i;
        let mut v: i64 = 0;
        while i < b.len() && b[i].is_ascii_digit() {
            v = v.saturating_mul(10).saturating_add((b[i] - b'0') as i64);
            i += 1;
        }
        if i == start {
            None
        } else {
            Some((i, v))
        }
    };
    for start in 0..b.len() {
        let Some((i, n1)) = digits(start) else {
            continue;
        };
        if i >= b.len() || b[i] != b';' {
            continue;
        }
        let Some((i, n2)) = digits(i + 1) else {
            continue;
        };
        if i >= b.len() || b[i] != b';' {
            continue;
        }
        let Some((i, n3)) = digits(i + 1) else {
            continue;
        };
        if i >= b.len() || (b[i] != b'M' && b[i] != b'm') {
            continue;
        }
        return Some((i + 1, [n1, n2, n3], b[i] == b'm'));
    }
    None
}

/// `unknownCSIRe` `^\x1b\[[\x30-\x3f]*[\x20-\x2f]*[\x40-\x7e]`: match length.
fn match_unknown_csi(b: &[u8]) -> Option<usize> {
    if b.len() < 3 || b[0] != 0x1b || b[1] != b'[' {
        return None;
    }
    let mut i = 2;
    while i < b.len() && (0x30..=0x3f).contains(&b[i]) {
        i += 1;
    }
    while i < b.len() && (0x20..=0x2f).contains(&b[i]) {
        i += 1;
    }
    if i < b.len() && (0x40..=0x7e).contains(&b[i]) {
        Some(i + 1)
    } else {
        None
    }
}

fn detect_sequence(input: &[u8]) -> Option<(usize, InputMsg)> {
    let (seqs, lens) = ext_sequences();
    for &sz in lens {
        if sz > input.len() {
            continue;
        }
        if let Some(k) = seqs.get(&input[..sz]) {
            return Some((sz, InputMsg::Key(k.clone())));
        }
    }
    match_unknown_csi(input).map(|n| (n, InputMsg::UnknownCsi(input[..n].to_vec())))
}

/// Returns Some(width, msg) for a complete paste, Some(0, None) for an
/// incomplete one.
fn detect_bracketed_paste(input: &[u8]) -> Option<(usize, Option<InputMsg>)> {
    const BP_START: &[u8] = b"\x1b[200~";
    const BP_END: &[u8] = b"\x1b[201~";
    if input.len() < BP_START.len() || &input[..BP_START.len()] != BP_START {
        return None;
    }
    let rest = &input[BP_START.len()..];
    let idx = rest.windows(BP_END.len()).position(|w| w == BP_END);
    let Some(idx) = idx else {
        return Some((0, None));
    };
    let input_len = BP_START.len() + idx + BP_END.len();
    let mut paste = &rest[..idx];
    let mut k = KeyMsg {
        typ: KEY_RUNES,
        paste: true,
        ..Default::default()
    };
    while !paste.is_empty() {
        let (r, w) = decode_rune(paste);
        if let Some(r) = r {
            k.runes.push(r);
        }
        paste = &paste[w.max(1)..];
    }
    Some((input_len, Some(InputMsg::Key(k))))
}

fn parse_sgr_mouse_event(buf: &[u8]) -> MouseMsg {
    let (_, [b, px, py], release) = find_sgr_mouse(&buf[3..]).expect("invalid mouse event");
    let mut m = parse_mouse_button(b as i32, true);
    if m.action != MOUSE_ACTION_MOTION && !m.is_wheel() && release {
        m.action = MOUSE_ACTION_RELEASE;
        m.typ = MOUSE_RELEASE;
    }
    m.x = px as i32 - 1;
    m.y = py as i32 - 1;
    m
}

fn parse_x10_mouse_event(buf: &[u8]) -> MouseMsg {
    let v = &buf[3..6];
    let mut m = parse_mouse_button(v[0] as i32, false);
    m.x = v[1] as i32 - 32 - 1;
    m.y = v[2] as i32 - 32 - 1;
    m
}

/// `detectOneMsg`: decodes one message from `b`. A width of 0 means "need
/// more data".
pub fn detect_one_msg(b: &[u8], can_have_more_data: bool) -> (usize, Option<InputMsg>) {
    const MOUSE_EVENT_X10_LEN: usize = 6;
    if b.len() >= MOUSE_EVENT_X10_LEN && b[0] == 0x1b && b[1] == b'[' {
        match b[2] {
            b'M' => {
                return (
                    MOUSE_EVENT_X10_LEN,
                    Some(InputMsg::Mouse(parse_x10_mouse_event(b))),
                )
            }
            b'<' => {
                if let Some((end, _, _)) = find_sgr_mouse(&b[3..]) {
                    return (end + 3, Some(InputMsg::Mouse(parse_sgr_mouse_event(b))));
                }
            }
            _ => {}
        }
    }

    if b == b"\x1b[I" {
        return (3, Some(InputMsg::Focus));
    }
    if b == b"\x1b[O" {
        return (3, Some(InputMsg::Blur));
    }

    if let Some((w, m)) = detect_bracketed_paste(b) {
        return (w, m);
    }

    if let Some((w, m)) = detect_sequence(b) {
        return (w, Some(m));
    }

    let mut alt = false;
    let mut i = 0;
    if b[0] == 0x1b {
        alt = true;
        i += 1;
    }
    if i < b.len() && b[i] == 0 {
        return (
            i + 1,
            Some(InputMsg::Key(KeyMsg {
                typ: KEY_NUL,
                alt,
                ..Default::default()
            })),
        );
    }

    let mut runes = Vec::new();
    while i < b.len() {
        let (r, rw) = decode_rune(&b[i..]);
        let Some(r) = r else { break };
        if (r as u32) <= 31 || r as u32 == 127 || r == ' ' {
            break;
        }
        runes.push(r);
        if alt {
            i += rw;
            break;
        }
        i += rw;
    }
    if i >= b.len() && can_have_more_data {
        return (0, None);
    }

    if !runes.is_empty() {
        let mut k = KeyMsg {
            typ: KEY_RUNES,
            runes,
            alt,
            paste: false,
        };
        if k.runes.len() == 1 && k.runes[0] == ' ' {
            k.typ = KEY_SPACE;
        }
        return (i, Some(InputMsg::Key(k)));
    }

    if alt && b.len() == 1 {
        return (1, Some(InputMsg::Key(KeyMsg::new(KEY_ESCAPE))));
    }

    (1, Some(InputMsg::UnknownByte(b[0])))
}

/// The input loop state of `readAnsiInputs` (256-byte reads with leftovers).
#[derive(Default)]
pub struct InputDecoder {
    leftover: Vec<u8>,
}

impl InputDecoder {
    pub const BUF_SIZE: usize = 256;

    /// Feeds one `Read` result (`num_bytes` = bytes read into a 256-byte
    /// buffer) and returns the decoded messages.
    pub fn feed(&mut self, chunk: &[u8]) -> Vec<InputMsg> {
        let can_have_more_data = chunk.len() == Self::BUF_SIZE;
        let mut b = std::mem::take(&mut self.leftover);
        b.extend_from_slice(chunk);
        let mut out = Vec::new();
        let mut i = 0;
        while i < b.len() {
            let (w, m) = detect_one_msg(&b[i..], can_have_more_data);
            if w == 0 {
                self.leftover = b[i..].to_vec();
                return out;
            }
            if let Some(m) = m {
                out.push(m);
            }
            i += w;
        }
        out
    }
}

/// Decodes a complete input buffer as a single read (test helper).
pub fn parse_input(b: &[u8]) -> Vec<InputMsg> {
    let mut d = InputDecoder::default();
    let mut out = Vec::new();
    for chunk in b.chunks(InputDecoder::BUF_SIZE) {
        out.extend(d.feed(chunk));
    }
    out
}
