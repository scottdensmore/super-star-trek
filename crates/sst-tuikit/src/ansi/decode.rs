//! `x/ansi` parser_decode.go / parser.go: `DecodeSequence` and the
//! parameter-collecting `Parser`.

use super::parser::{
    HAS_MORE_FLAG, INTERMED_SHIFT, MAX_PARAMS_SIZE, MISSING_COMMAND, MISSING_PARAM, PARAM_MASK,
    PREFIX_SHIFT,
};
use super::width::first_grapheme_cluster;

pub const NORMAL_STATE: u8 = 0;
pub const PREFIX_STATE: u8 = 1;
pub const PARAMS_STATE: u8 = 2;
pub const INTERMED_STATE: u8 = 3;
pub const ESCAPE_STATE: u8 = 4;
pub const STRING_STATE: u8 = 5;

const ESC: u8 = 0x1b;
const BEL: u8 = 0x07;
const CAN: u8 = 0x18;
const SUB: u8 = 0x1a;
const US: u8 = 0x1f;
const DEL: u8 = 0x7f;
const CSI: u8 = 0x9b;
const DCS: u8 = 0x90;
const OSC: u8 = 0x9d;
const APC: u8 = 0x9f;
const SOS: u8 = 0x98;
const PM: u8 = 0x9e;
const ST: u8 = 0x9c;

/// A packed sequence parameter (`ansi.Param`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Param(pub i64);

impl Param {
    /// The parameter value, or `def` if missing.
    pub fn param(self, def: i64) -> i64 {
        let p = self.0 & PARAM_MASK;
        if p == MISSING_PARAM {
            def
        } else {
            p
        }
    }
    /// Whether this is a sub-parameter followed by more (`:` separator).
    pub fn has_more(self) -> bool {
        self.0 & HAS_MORE_FLAG != 0
    }
}

/// `ansi.Params.Param`: (value, hasMore, ok).
pub fn params_param(p: &[Param], i: usize, def: i64) -> (i64, bool, bool) {
    if i >= p.len() {
        return (def, false, false);
    }
    (p[i].param(def), p[i].has_more(), true)
}

/// The sequence parser state used with [`decode_sequence`] (`ansi.Parser`,
/// as returned by `GetParser`: 32 params, 4 KiB data).
pub struct Parser {
    params: Vec<i64>,
    params_len: usize,
    data: Vec<u8>,
    data_len: usize,
    cmd: i64,
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

impl Parser {
    pub fn new() -> Self {
        Parser {
            params: vec![MISSING_PARAM; MAX_PARAMS_SIZE],
            params_len: 0,
            data: vec![0; 1024 * 4],
            data_len: 0,
            cmd: 0,
        }
    }
    /// The collected parameters.
    pub fn params(&self) -> Vec<Param> {
        self.params[..self.params_len.min(self.params.len())]
            .iter()
            .map(|&p| Param(p))
            .collect()
    }
    /// The packed command (final byte | prefix<<8 | intermediate<<16), or
    /// the OSC command number.
    pub fn command(&self) -> i64 {
        self.cmd
    }
    /// The collected string data (OSC, DCS, ...).
    pub fn data(&self) -> &[u8] {
        &self.data[..self.data_len]
    }
    fn param_mut(&mut self) -> Option<&mut i64> {
        let i = self.params_len;
        self.params.get_mut(i)
    }
}

pub fn has_csi_prefix(b: &[u8]) -> bool {
    (!b.is_empty() && b[0] == CSI) || (b.len() > 1 && b[0] == ESC && b[1] == b'[')
}
pub fn has_osc_prefix(b: &[u8]) -> bool {
    (!b.is_empty() && b[0] == OSC) || (b.len() > 1 && b[0] == ESC && b[1] == b']')
}
pub fn has_dcs_prefix(b: &[u8]) -> bool {
    (!b.is_empty() && b[0] == DCS) || (b.len() > 1 && b[0] == ESC && b[1] == b'P')
}
pub fn has_st_prefix(b: &[u8]) -> bool {
    (!b.is_empty() && b[0] == ST) || (b.len() > 1 && b[0] == ESC && b[1] == b'\\')
}

fn parse_osc_cmd(p: &mut Option<&mut Parser>) {
    let Some(p) = p.as_deref_mut() else { return };
    if p.cmd != MISSING_COMMAND {
        return;
    }
    for j in 0..p.data_len {
        let d = p.data[j];
        if !d.is_ascii_digit() {
            break;
        }
        if p.cmd == MISSING_COMMAND {
            p.cmd = 0;
        }
        p.cmd = p.cmd.wrapping_mul(10).wrapping_add((d - b'0') as i64);
    }
}

/// `ansi.DecodeSequence`: decodes the first escape sequence or grapheme of
/// `b`. Returns (sequence length, cell width, new state). The sequence is
/// `&b[..n]`.
pub fn decode_sequence(b: &[u8], mut state: u8, mut p: Option<&mut Parser>) -> (usize, i32, u8) {
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        // Emulates Go's switch with fallthrough between Prefix/Params/Intermed.
        let mut st = state;
        loop {
            match st {
                NORMAL_STATE => {
                    match c {
                        ESC => {
                            if let Some(p) = p.as_deref_mut() {
                                if !p.params.is_empty() {
                                    p.params[0] = MISSING_PARAM;
                                }
                                p.cmd = 0;
                                p.params_len = 0;
                                p.data_len = 0;
                            }
                            state = ESCAPE_STATE;
                            break;
                        }
                        CSI | DCS => {
                            if let Some(p) = p.as_deref_mut() {
                                if !p.params.is_empty() {
                                    p.params[0] = MISSING_PARAM;
                                }
                                p.cmd = 0;
                                p.params_len = 0;
                                p.data_len = 0;
                            }
                            state = PREFIX_STATE;
                            break;
                        }
                        OSC | APC | SOS | PM => {
                            if let Some(p) = p.as_deref_mut() {
                                p.cmd = MISSING_COMMAND;
                                p.data_len = 0;
                            }
                            state = STRING_STATE;
                            break;
                        }
                        _ => {}
                    }
                    if let Some(p) = p.as_deref_mut() {
                        p.data_len = 0;
                        p.params_len = 0;
                        p.cmd = 0;
                    }
                    if c > US && c < DEL {
                        return (i + 1, 1, NORMAL_STATE);
                    }
                    if c <= US || c == DEL || c < 0xC0 {
                        return (i + 1, 0, NORMAL_STATE);
                    }
                    // utf8.RuneStart(c) is true for every byte >= 0xC0.
                    let (n, width) = first_grapheme_cluster(&b[i..]);
                    let n = n.max(1);
                    return (i + n, width, NORMAL_STATE);
                }
                PREFIX_STATE => {
                    if (b'<'..=b'?').contains(&c) {
                        if let Some(p) = p.as_deref_mut() {
                            p.cmd &= !(0xff << PREFIX_SHIFT);
                            p.cmd |= (c as i64) << PREFIX_SHIFT;
                        }
                        break;
                    }
                    state = PARAMS_STATE;
                    st = PARAMS_STATE;
                    continue;
                }
                PARAMS_STATE => {
                    if c.is_ascii_digit() {
                        if let Some(p) = p.as_deref_mut() {
                            if let Some(v) = p.param_mut() {
                                if *v == MISSING_PARAM {
                                    *v = 0;
                                }
                                *v = v.wrapping_mul(10).wrapping_add((c - b'0') as i64);
                            }
                        }
                        break;
                    }
                    if c == b':' {
                        if let Some(p) = p.as_deref_mut() {
                            if let Some(v) = p.param_mut() {
                                *v |= HAS_MORE_FLAG;
                            }
                        }
                    }
                    if c == b';' || c == b':' {
                        if let Some(p) = p.as_deref_mut() {
                            p.params_len += 1;
                            if p.params_len < p.params.len() {
                                p.params[p.params_len] = MISSING_PARAM;
                            }
                        }
                        break;
                    }
                    state = INTERMED_STATE;
                    st = INTERMED_STATE;
                    continue;
                }
                INTERMED_STATE => {
                    if (b' '..=b'/').contains(&c) {
                        if let Some(p) = p.as_deref_mut() {
                            p.cmd &= !(0xff << INTERMED_SHIFT);
                            p.cmd |= (c as i64) << INTERMED_SHIFT;
                        }
                        break;
                    }
                    if let Some(p) = p.as_deref_mut() {
                        let plen = p.params.len();
                        if (p.params_len > 0 && p.params_len < plen - 1)
                            || (p.params_len == 0 && plen > 0 && p.params[0] != MISSING_PARAM)
                        {
                            p.params_len += 1;
                        }
                    }
                    if (b'@'..=b'~').contains(&c) {
                        if let Some(p) = p.as_deref_mut() {
                            p.cmd &= !0xff;
                            p.cmd |= c as i64;
                        }
                        if has_dcs_prefix(b) {
                            if let Some(p) = p.as_deref_mut() {
                                p.data_len = 0;
                            }
                            state = STRING_STATE;
                            break;
                        }
                        return (i + 1, 0, NORMAL_STATE);
                    }
                    return (i, 0, NORMAL_STATE);
                }
                ESCAPE_STATE => {
                    match c {
                        b'[' | b'P' => {
                            if let Some(p) = p.as_deref_mut() {
                                if !p.params.is_empty() {
                                    p.params[0] = MISSING_PARAM;
                                }
                                p.params_len = 0;
                                p.cmd = 0;
                            }
                            state = PREFIX_STATE;
                            break;
                        }
                        b']' | b'X' | b'^' | b'_' => {
                            if let Some(p) = p.as_deref_mut() {
                                p.cmd = MISSING_COMMAND;
                                p.data_len = 0;
                            }
                            state = STRING_STATE;
                            break;
                        }
                        _ => {}
                    }
                    if (b' '..=b'/').contains(&c) {
                        if let Some(p) = p.as_deref_mut() {
                            p.cmd &= !(0xff << INTERMED_SHIFT);
                            p.cmd |= (c as i64) << INTERMED_SHIFT;
                        }
                        break;
                    } else if (b'0'..=b'~').contains(&c) {
                        if let Some(p) = p.as_deref_mut() {
                            p.cmd &= !0xff;
                            p.cmd |= c as i64;
                        }
                        return (i + 1, 0, NORMAL_STATE);
                    }
                    return (i, 0, NORMAL_STATE);
                }
                STRING_STATE => {
                    match c {
                        BEL => {
                            if has_osc_prefix(b) {
                                parse_osc_cmd(&mut p);
                                return (i + 1, 0, NORMAL_STATE);
                            }
                        }
                        CAN | SUB => {
                            if has_osc_prefix(b) {
                                parse_osc_cmd(&mut p);
                            }
                            return (i, 0, NORMAL_STATE);
                        }
                        ST => {
                            if has_osc_prefix(b) {
                                parse_osc_cmd(&mut p);
                            }
                            return (i + 1, 0, NORMAL_STATE);
                        }
                        ESC => {
                            if has_st_prefix(&b[i..]) {
                                if has_osc_prefix(b) {
                                    parse_osc_cmd(&mut p);
                                }
                                return (i + 2, 0, NORMAL_STATE);
                            }
                            return (i, 0, NORMAL_STATE);
                        }
                        _ => {}
                    }
                    if let Some(pp) = p.as_deref_mut() {
                        if pp.data_len < pp.data.len() {
                            pp.data[pp.data_len] = c;
                            pp.data_len += 1;
                            if c == b';' && has_osc_prefix(b) {
                                parse_osc_cmd(&mut p);
                            }
                        }
                    }
                    break;
                }
                _ => break,
            }
        }
        i += 1;
    }
    (b.len(), 0, state)
}

/// Unpacked parts of a packed command.
pub fn cmd_final(cmd: i64) -> u8 {
    (cmd & 0xff) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_sgr() {
        let mut p = Parser::new();
        let s = b"\x1b[1;38;5;62mX";
        let (n, w, st) = decode_sequence(s, 0, Some(&mut p));
        assert_eq!((n, w, st), (s.len() - 1, 0, NORMAL_STATE));
        assert_eq!(cmd_final(p.command()), b'm');
        let params: Vec<i64> = p.params().iter().map(|x| x.param(0)).collect();
        assert_eq!(params, vec![1, 38, 5, 62]);
    }
}
