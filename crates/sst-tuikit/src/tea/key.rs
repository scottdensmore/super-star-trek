//! Keyboard messages (`key.go`, `key_sequences.go`).
//!
//! Go `tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'a'}}` ->
//! `KeyMsg { typ: KEY_RUNES, runes: vec!['a'], ..Default::default() }` or
//! [`KeyMsg::runes`]`("a")`; `msg.Type == tea.KeyEnter` ->
//! `msg.typ == KEY_ENTER`; `msg.String()` -> [`KeyMsg::string`].

use std::collections::HashMap;
use std::fmt;
use std::sync::OnceLock;

/// `tea.KeyType`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct KeyType(pub i32);

macro_rules! keys {
    ($($name:ident = $v:expr),* $(,)?) => { $(pub const $name: KeyType = KeyType($v);)* };
}

// Control keys (`keyNUL` .. `keyDEL`).
keys! {
    KEY_NUL = 0, KEY_SOH = 1, KEY_STX = 2, KEY_ETX = 3, KEY_EOT = 4, KEY_ENQ = 5, KEY_ACK = 6,
    KEY_BEL = 7, KEY_BS = 8, KEY_HT = 9, KEY_LF = 10, KEY_VT = 11, KEY_FF = 12, KEY_CR = 13,
    KEY_SO = 14, KEY_SI = 15, KEY_DLE = 16, KEY_DC1 = 17, KEY_DC2 = 18, KEY_DC3 = 19, KEY_DC4 = 20,
    KEY_NAK = 21, KEY_SYN = 22, KEY_ETB = 23, KEY_CAN = 24, KEY_EM = 25, KEY_SUB = 26, KEY_ESC_BYTE = 27,
    KEY_FS = 28, KEY_GS = 29, KEY_RS = 30, KEY_US = 31, KEY_DEL = 127,
}

keys! {
    KEY_NULL = 0, KEY_BREAK = 3, KEY_ENTER = 13, KEY_BACKSPACE = 127, KEY_TAB = 9, KEY_ESC = 27, KEY_ESCAPE = 27,
    KEY_CTRL_AT = 0, KEY_CTRL_A = 1, KEY_CTRL_B = 2, KEY_CTRL_C = 3, KEY_CTRL_D = 4, KEY_CTRL_E = 5,
    KEY_CTRL_F = 6, KEY_CTRL_G = 7, KEY_CTRL_H = 8, KEY_CTRL_I = 9, KEY_CTRL_J = 10, KEY_CTRL_K = 11,
    KEY_CTRL_L = 12, KEY_CTRL_M = 13, KEY_CTRL_N = 14, KEY_CTRL_O = 15, KEY_CTRL_P = 16, KEY_CTRL_Q = 17,
    KEY_CTRL_R = 18, KEY_CTRL_S = 19, KEY_CTRL_T = 20, KEY_CTRL_U = 21, KEY_CTRL_V = 22, KEY_CTRL_W = 23,
    KEY_CTRL_X = 24, KEY_CTRL_Y = 25, KEY_CTRL_Z = 26, KEY_CTRL_OPEN_BRACKET = 27, KEY_CTRL_BACKSLASH = 28,
    KEY_CTRL_CLOSE_BRACKET = 29, KEY_CTRL_CARET = 30, KEY_CTRL_UNDERSCORE = 31, KEY_CTRL_QUESTION_MARK = 127,
}

// Other keys (negative values, Go `-(iota + 1)`).
keys! {
    KEY_RUNES = -1, KEY_UP = -2, KEY_DOWN = -3, KEY_RIGHT = -4, KEY_LEFT = -5, KEY_SHIFT_TAB = -6,
    KEY_HOME = -7, KEY_END = -8, KEY_PG_UP = -9, KEY_PG_DOWN = -10, KEY_CTRL_PG_UP = -11,
    KEY_CTRL_PG_DOWN = -12, KEY_DELETE = -13, KEY_INSERT = -14, KEY_SPACE = -15, KEY_CTRL_UP = -16,
    KEY_CTRL_DOWN = -17, KEY_CTRL_RIGHT = -18, KEY_CTRL_LEFT = -19, KEY_CTRL_HOME = -20,
    KEY_CTRL_END = -21, KEY_SHIFT_UP = -22, KEY_SHIFT_DOWN = -23, KEY_SHIFT_RIGHT = -24,
    KEY_SHIFT_LEFT = -25, KEY_SHIFT_HOME = -26, KEY_SHIFT_END = -27, KEY_CTRL_SHIFT_UP = -28,
    KEY_CTRL_SHIFT_DOWN = -29, KEY_CTRL_SHIFT_LEFT = -30, KEY_CTRL_SHIFT_RIGHT = -31,
    KEY_CTRL_SHIFT_HOME = -32, KEY_CTRL_SHIFT_END = -33, KEY_F1 = -34, KEY_F2 = -35, KEY_F3 = -36,
    KEY_F4 = -37, KEY_F5 = -38, KEY_F6 = -39, KEY_F7 = -40, KEY_F8 = -41, KEY_F9 = -42, KEY_F10 = -43,
    KEY_F11 = -44, KEY_F12 = -45, KEY_F13 = -46, KEY_F14 = -47, KEY_F15 = -48, KEY_F16 = -49,
    KEY_F17 = -50, KEY_F18 = -51, KEY_F19 = -52, KEY_F20 = -53,
}

/// `keyNames`.
pub(crate) fn key_name(k: KeyType) -> Option<&'static str> {
    Some(match k.0 {
        0 => "ctrl+@",
        1 => "ctrl+a",
        2 => "ctrl+b",
        3 => "ctrl+c",
        4 => "ctrl+d",
        5 => "ctrl+e",
        6 => "ctrl+f",
        7 => "ctrl+g",
        8 => "ctrl+h",
        9 => "tab",
        10 => "ctrl+j",
        11 => "ctrl+k",
        12 => "ctrl+l",
        13 => "enter",
        14 => "ctrl+n",
        15 => "ctrl+o",
        16 => "ctrl+p",
        17 => "ctrl+q",
        18 => "ctrl+r",
        19 => "ctrl+s",
        20 => "ctrl+t",
        21 => "ctrl+u",
        22 => "ctrl+v",
        23 => "ctrl+w",
        24 => "ctrl+x",
        25 => "ctrl+y",
        26 => "ctrl+z",
        27 => "esc",
        28 => "ctrl+\\",
        29 => "ctrl+]",
        30 => "ctrl+^",
        31 => "ctrl+_",
        127 => "backspace",
        -1 => "runes",
        -2 => "up",
        -3 => "down",
        -4 => "right",
        -15 => " ",
        -5 => "left",
        -6 => "shift+tab",
        -7 => "home",
        -8 => "end",
        -20 => "ctrl+home",
        -21 => "ctrl+end",
        -26 => "shift+home",
        -27 => "shift+end",
        -32 => "ctrl+shift+home",
        -33 => "ctrl+shift+end",
        -9 => "pgup",
        -10 => "pgdown",
        -11 => "ctrl+pgup",
        -12 => "ctrl+pgdown",
        -13 => "delete",
        -14 => "insert",
        -16 => "ctrl+up",
        -17 => "ctrl+down",
        -18 => "ctrl+right",
        -19 => "ctrl+left",
        -22 => "shift+up",
        -23 => "shift+down",
        -24 => "shift+right",
        -25 => "shift+left",
        -28 => "ctrl+shift+up",
        -29 => "ctrl+shift+down",
        -30 => "ctrl+shift+left",
        -31 => "ctrl+shift+right",
        -34 => "f1",
        -35 => "f2",
        -36 => "f3",
        -37 => "f4",
        -38 => "f5",
        -39 => "f6",
        -40 => "f7",
        -41 => "f8",
        -42 => "f9",
        -43 => "f10",
        -44 => "f11",
        -45 => "f12",
        -46 => "f13",
        -47 => "f14",
        -48 => "f15",
        -49 => "f16",
        -50 => "f17",
        -51 => "f18",
        -52 => "f19",
        -53 => "f20",
        _ => return None,
    })
}

impl KeyType {
    /// `KeyType.String()`.
    pub fn string(self) -> String {
        key_name(self).unwrap_or("").to_string()
    }
}

impl fmt::Display for KeyType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.string())
    }
}

/// A key press (`tea.KeyMsg` / `tea.Key`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeyMsg {
    /// Go `Type`.
    pub typ: KeyType,
    pub runes: Vec<char>,
    pub alt: bool,
    pub paste: bool,
}

/// `tea.Key` (same thing as [`KeyMsg`] in Rust).
pub type Key = KeyMsg;

impl KeyMsg {
    /// A key of the given type.
    pub fn new(typ: KeyType) -> KeyMsg {
        KeyMsg {
            typ,
            ..Default::default()
        }
    }
    /// A `KeyRunes` key with the runes of `s`.
    pub fn runes(s: &str) -> KeyMsg {
        KeyMsg {
            typ: KEY_RUNES,
            runes: s.chars().collect(),
            ..Default::default()
        }
    }
    /// `Key.String()`: e.g. "ctrl+c", "alt+enter", "a", "[pasted]".
    pub fn string(&self) -> String {
        let mut buf = String::new();
        if self.alt {
            buf.push_str("alt+");
        }
        if self.typ == KEY_RUNES {
            if self.paste {
                buf.push('[');
            }
            buf.extend(self.runes.iter());
            if self.paste {
                buf.push(']');
            }
            return buf;
        } else if let Some(s) = key_name(self.typ) {
            buf.push_str(s);
            return buf;
        }
        String::new()
    }
}

impl fmt::Display for KeyMsg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.string())
    }
}

fn k(typ: KeyType, alt: bool) -> KeyMsg {
    KeyMsg {
        typ,
        alt,
        ..Default::default()
    }
}

/// `sequences`.
fn sequences() -> Vec<(&'static str, KeyMsg)> {
    vec![
        ("\x1b[A", k(KEY_UP, false)),
        ("\x1b[B", k(KEY_DOWN, false)),
        ("\x1b[C", k(KEY_RIGHT, false)),
        ("\x1b[D", k(KEY_LEFT, false)),
        ("\x1b[1;2A", k(KEY_SHIFT_UP, false)),
        ("\x1b[1;2B", k(KEY_SHIFT_DOWN, false)),
        ("\x1b[1;2C", k(KEY_SHIFT_RIGHT, false)),
        ("\x1b[1;2D", k(KEY_SHIFT_LEFT, false)),
        ("\x1b[OA", k(KEY_SHIFT_UP, false)),
        ("\x1b[OB", k(KEY_SHIFT_DOWN, false)),
        ("\x1b[OC", k(KEY_SHIFT_RIGHT, false)),
        ("\x1b[OD", k(KEY_SHIFT_LEFT, false)),
        ("\x1b[a", k(KEY_SHIFT_UP, false)),
        ("\x1b[b", k(KEY_SHIFT_DOWN, false)),
        ("\x1b[c", k(KEY_SHIFT_RIGHT, false)),
        ("\x1b[d", k(KEY_SHIFT_LEFT, false)),
        ("\x1b[1;3A", k(KEY_UP, true)),
        ("\x1b[1;3B", k(KEY_DOWN, true)),
        ("\x1b[1;3C", k(KEY_RIGHT, true)),
        ("\x1b[1;3D", k(KEY_LEFT, true)),
        ("\x1b[1;4A", k(KEY_SHIFT_UP, true)),
        ("\x1b[1;4B", k(KEY_SHIFT_DOWN, true)),
        ("\x1b[1;4C", k(KEY_SHIFT_RIGHT, true)),
        ("\x1b[1;4D", k(KEY_SHIFT_LEFT, true)),
        ("\x1b[1;5A", k(KEY_CTRL_UP, false)),
        ("\x1b[1;5B", k(KEY_CTRL_DOWN, false)),
        ("\x1b[1;5C", k(KEY_CTRL_RIGHT, false)),
        ("\x1b[1;5D", k(KEY_CTRL_LEFT, false)),
        ("\x1b[Oa", k(KEY_CTRL_UP, true)),
        ("\x1b[Ob", k(KEY_CTRL_DOWN, true)),
        ("\x1b[Oc", k(KEY_CTRL_RIGHT, true)),
        ("\x1b[Od", k(KEY_CTRL_LEFT, true)),
        ("\x1b[1;6A", k(KEY_CTRL_SHIFT_UP, false)),
        ("\x1b[1;6B", k(KEY_CTRL_SHIFT_DOWN, false)),
        ("\x1b[1;6C", k(KEY_CTRL_SHIFT_RIGHT, false)),
        ("\x1b[1;6D", k(KEY_CTRL_SHIFT_LEFT, false)),
        ("\x1b[1;7A", k(KEY_CTRL_UP, true)),
        ("\x1b[1;7B", k(KEY_CTRL_DOWN, true)),
        ("\x1b[1;7C", k(KEY_CTRL_RIGHT, true)),
        ("\x1b[1;7D", k(KEY_CTRL_LEFT, true)),
        ("\x1b[1;8A", k(KEY_CTRL_SHIFT_UP, true)),
        ("\x1b[1;8B", k(KEY_CTRL_SHIFT_DOWN, true)),
        ("\x1b[1;8C", k(KEY_CTRL_SHIFT_RIGHT, true)),
        ("\x1b[1;8D", k(KEY_CTRL_SHIFT_LEFT, true)),
        ("\x1b[Z", k(KEY_SHIFT_TAB, false)),
        ("\x1b[2~", k(KEY_INSERT, false)),
        ("\x1b[3;2~", k(KEY_INSERT, true)),
        ("\x1b[3~", k(KEY_DELETE, false)),
        ("\x1b[3;3~", k(KEY_DELETE, true)),
        ("\x1b[5~", k(KEY_PG_UP, false)),
        ("\x1b[5;3~", k(KEY_PG_UP, true)),
        ("\x1b[5;5~", k(KEY_CTRL_PG_UP, false)),
        ("\x1b[5^", k(KEY_CTRL_PG_UP, false)),
        ("\x1b[5;7~", k(KEY_CTRL_PG_UP, true)),
        ("\x1b[6~", k(KEY_PG_DOWN, false)),
        ("\x1b[6;3~", k(KEY_PG_DOWN, true)),
        ("\x1b[6;5~", k(KEY_CTRL_PG_DOWN, false)),
        ("\x1b[6^", k(KEY_CTRL_PG_DOWN, false)),
        ("\x1b[6;7~", k(KEY_CTRL_PG_DOWN, true)),
        ("\x1b[1~", k(KEY_HOME, false)),
        ("\x1b[H", k(KEY_HOME, false)),
        ("\x1b[1;3H", k(KEY_HOME, true)),
        ("\x1b[1;5H", k(KEY_CTRL_HOME, false)),
        ("\x1b[1;7H", k(KEY_CTRL_HOME, true)),
        ("\x1b[1;2H", k(KEY_SHIFT_HOME, false)),
        ("\x1b[1;4H", k(KEY_SHIFT_HOME, true)),
        ("\x1b[1;6H", k(KEY_CTRL_SHIFT_HOME, false)),
        ("\x1b[1;8H", k(KEY_CTRL_SHIFT_HOME, true)),
        ("\x1b[4~", k(KEY_END, false)),
        ("\x1b[F", k(KEY_END, false)),
        ("\x1b[1;3F", k(KEY_END, true)),
        ("\x1b[1;5F", k(KEY_CTRL_END, false)),
        ("\x1b[1;7F", k(KEY_CTRL_END, true)),
        ("\x1b[1;2F", k(KEY_SHIFT_END, false)),
        ("\x1b[1;4F", k(KEY_SHIFT_END, true)),
        ("\x1b[1;6F", k(KEY_CTRL_SHIFT_END, false)),
        ("\x1b[1;8F", k(KEY_CTRL_SHIFT_END, true)),
        ("\x1b[7~", k(KEY_HOME, false)),
        ("\x1b[7^", k(KEY_CTRL_HOME, false)),
        ("\x1b[7$", k(KEY_SHIFT_HOME, false)),
        ("\x1b[7@", k(KEY_CTRL_SHIFT_HOME, false)),
        ("\x1b[8~", k(KEY_END, false)),
        ("\x1b[8^", k(KEY_CTRL_END, false)),
        ("\x1b[8$", k(KEY_SHIFT_END, false)),
        ("\x1b[8@", k(KEY_CTRL_SHIFT_END, false)),
        ("\x1b[[A", k(KEY_F1, false)),
        ("\x1b[[B", k(KEY_F2, false)),
        ("\x1b[[C", k(KEY_F3, false)),
        ("\x1b[[D", k(KEY_F4, false)),
        ("\x1b[[E", k(KEY_F5, false)),
        ("\x1bOP", k(KEY_F1, false)),
        ("\x1bOQ", k(KEY_F2, false)),
        ("\x1bOR", k(KEY_F3, false)),
        ("\x1bOS", k(KEY_F4, false)),
        ("\x1b[1;3P", k(KEY_F1, true)),
        ("\x1b[1;3Q", k(KEY_F2, true)),
        ("\x1b[1;3R", k(KEY_F3, true)),
        ("\x1b[1;3S", k(KEY_F4, true)),
        ("\x1b[11~", k(KEY_F1, false)),
        ("\x1b[12~", k(KEY_F2, false)),
        ("\x1b[13~", k(KEY_F3, false)),
        ("\x1b[14~", k(KEY_F4, false)),
        ("\x1b[15~", k(KEY_F5, false)),
        ("\x1b[15;3~", k(KEY_F5, true)),
        ("\x1b[17~", k(KEY_F6, false)),
        ("\x1b[18~", k(KEY_F7, false)),
        ("\x1b[19~", k(KEY_F8, false)),
        ("\x1b[20~", k(KEY_F9, false)),
        ("\x1b[21~", k(KEY_F10, false)),
        ("\x1b[17;3~", k(KEY_F6, true)),
        ("\x1b[18;3~", k(KEY_F7, true)),
        ("\x1b[19;3~", k(KEY_F8, true)),
        ("\x1b[20;3~", k(KEY_F9, true)),
        ("\x1b[21;3~", k(KEY_F10, true)),
        ("\x1b[23~", k(KEY_F11, false)),
        ("\x1b[24~", k(KEY_F12, false)),
        ("\x1b[23;3~", k(KEY_F11, true)),
        ("\x1b[24;3~", k(KEY_F12, true)),
        ("\x1b[1;2P", k(KEY_F13, false)),
        ("\x1b[1;2Q", k(KEY_F14, false)),
        ("\x1b[25~", k(KEY_F13, false)),
        ("\x1b[26~", k(KEY_F14, false)),
        ("\x1b[25;3~", k(KEY_F13, true)),
        ("\x1b[26;3~", k(KEY_F14, true)),
        ("\x1b[1;2R", k(KEY_F15, false)),
        ("\x1b[1;2S", k(KEY_F16, false)),
        ("\x1b[28~", k(KEY_F15, false)),
        ("\x1b[29~", k(KEY_F16, false)),
        ("\x1b[28;3~", k(KEY_F15, true)),
        ("\x1b[29;3~", k(KEY_F16, true)),
        ("\x1b[15;2~", k(KEY_F17, false)),
        ("\x1b[17;2~", k(KEY_F18, false)),
        ("\x1b[18;2~", k(KEY_F19, false)),
        ("\x1b[19;2~", k(KEY_F20, false)),
        ("\x1b[31~", k(KEY_F17, false)),
        ("\x1b[32~", k(KEY_F18, false)),
        ("\x1b[33~", k(KEY_F19, false)),
        ("\x1b[34~", k(KEY_F20, false)),
        ("\x1bOA", k(KEY_UP, false)),
        ("\x1bOB", k(KEY_DOWN, false)),
        ("\x1bOC", k(KEY_RIGHT, false)),
        ("\x1bOD", k(KEY_LEFT, false)),
    ]
}

type ExtSequences = (HashMap<Vec<u8>, KeyMsg>, Vec<usize>);

/// `extSequences` and `seqLengths` (descending).
pub(crate) fn ext_sequences() -> &'static ExtSequences {
    static EXT: OnceLock<ExtSequences> = OnceLock::new();
    EXT.get_or_init(|| {
        let mut s: HashMap<Vec<u8>, KeyMsg> = HashMap::new();
        for (seq, key) in sequences() {
            s.insert(seq.as_bytes().to_vec(), key.clone());
            if !key.alt {
                let mut k2 = key.clone();
                k2.alt = true;
                let mut v = vec![0x1b];
                v.extend_from_slice(seq.as_bytes());
                s.insert(v, k2);
            }
        }
        let mut i = KEY_NUL.0 + 1;
        while i <= KEY_DEL.0 {
            if i == KEY_ESC.0 {
                i += 1;
                continue;
            }
            s.insert(vec![i as u8], k(KeyType(i), false));
            s.insert(vec![0x1b, i as u8], k(KeyType(i), true));
            if i == KEY_US.0 {
                i = KEY_DEL.0 - 1;
            }
            i += 1;
        }
        s.insert(
            b" ".to_vec(),
            KeyMsg {
                typ: KEY_SPACE,
                runes: vec![' '],
                ..Default::default()
            },
        );
        s.insert(
            b"\x1b ".to_vec(),
            KeyMsg {
                typ: KEY_SPACE,
                alt: true,
                runes: vec![' '],
                ..Default::default()
            },
        );
        s.insert(b"\x1b\x1b".to_vec(), k(KEY_ESCAPE, true));
        let mut lens: Vec<usize> = s.keys().map(|k| k.len()).collect();
        lens.sort_unstable_by(|a, b| b.cmp(a));
        lens.dedup();
        (s, lens)
    })
}
