//! `github.com/charmbracelet/x/cellbuf` v0.0.15 subset: [`wrap`] (used by
//! lipgloss `Style.Render` when a width is set) and the pen `Style`/`Link`
//! it tracks across wrapped lines (`wrap.go`, `cell.go`, `sequence.go`).

use crate::ansi::decode::{self, params_param, Param, Parser};
use crate::ansi::style::{read_style_color, AnsiColor, SgrStyle, RESET_STYLE};
use crate::ansi::{bytes_to_string, reset_hyperlink, set_hyperlink};

const BOLD_ATTR: u8 = 1 << 0;
const FAINT_ATTR: u8 = 1 << 1;
const ITALIC_ATTR: u8 = 1 << 2;
const SLOW_BLINK_ATTR: u8 = 1 << 3;
const RAPID_BLINK_ATTR: u8 = 1 << 4;
const REVERSE_ATTR: u8 = 1 << 5;
const CONCEAL_ATTR: u8 = 1 << 6;
const STRIKETHROUGH_ATTR: u8 = 1 << 7;

const NO_UNDERLINE: u8 = 0;
const SINGLE_UNDERLINE: u8 = 1;

/// The pen style of a cell (`cellbuf.Style`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CellStyle {
    pub fg: Option<AnsiColor>,
    pub bg: Option<AnsiColor>,
    pub ul: Option<AnsiColor>,
    pub attrs: u8,
    pub ul_style: u8,
}

impl CellStyle {
    fn set_attr(&mut self, a: u8, v: bool) {
        if v {
            self.attrs |= a;
        } else {
            self.attrs &= !a;
        }
    }
    fn reset(&mut self) {
        *self = CellStyle::default();
    }
    /// `Style.Empty`.
    pub fn empty(&self) -> bool {
        self.fg.is_none()
            && self.bg.is_none()
            && self.ul.is_none()
            && self.attrs == 0
            && self.ul_style == NO_UNDERLINE
    }
    /// `Style.Sequence`.
    pub fn sequence(&self) -> String {
        if self.empty() {
            return RESET_STYLE.to_string();
        }
        let mut b = SgrStyle::default();
        if self.attrs != 0 {
            if self.attrs & BOLD_ATTR != 0 {
                b = b.bold();
            }
            if self.attrs & FAINT_ATTR != 0 {
                b = b.faint();
            }
            if self.attrs & ITALIC_ATTR != 0 {
                b = b.italic(true);
            }
            if self.attrs & SLOW_BLINK_ATTR != 0 {
                b = b.blink(true);
            }
            if self.attrs & RAPID_BLINK_ATTR != 0 {
                b = b.rapid_blink(true);
            }
            if self.attrs & REVERSE_ATTR != 0 {
                b = b.reverse(true);
            }
            if self.attrs & CONCEAL_ATTR != 0 {
                b = b.conceal(true);
            }
            if self.attrs & STRIKETHROUGH_ATTR != 0 {
                b = b.strikethrough(true);
            }
        }
        if self.ul_style != NO_UNDERLINE {
            b = b.underline(true);
            b = b.underline_style(self.ul_style);
        }
        if self.fg.is_some() {
            b = b.foreground_color(self.fg);
        }
        if self.bg.is_some() {
            b = b.background_color(self.bg);
        }
        if self.ul.is_some() {
            b = b.underline_color(self.ul);
        }
        b.string()
    }
}

/// A hyperlink (`cellbuf.Link`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Link {
    pub url: String,
    pub params: String,
}

impl Link {
    pub fn empty(&self) -> bool {
        self.url.is_empty() && self.params.is_empty()
    }
}

/// `cellbuf.ReadStyle`: applies SGR parameters to `pen`.
pub fn read_style(params: &[Param], pen: &mut CellStyle) {
    if params.is_empty() {
        pen.reset();
        return;
    }
    let mut i = 0usize;
    while i < params.len() {
        let (param, has_more, _) = params_param(params, i, 0);
        match param {
            0 => pen.reset(),
            1 => pen.set_attr(BOLD_ATTR, true),
            2 => pen.set_attr(FAINT_ATTR, true),
            3 => pen.set_attr(ITALIC_ATTR, true),
            4 => {
                let (next, _, ok) = params_param(params, i + 1, 0);
                if has_more && ok {
                    if (0..=5).contains(&next) {
                        i += 1;
                        pen.ul_style = next as u8;
                    }
                } else {
                    pen.ul_style = SINGLE_UNDERLINE;
                }
            }
            5 => pen.set_attr(SLOW_BLINK_ATTR, true),
            6 => pen.set_attr(RAPID_BLINK_ATTR, true),
            7 => pen.set_attr(REVERSE_ATTR, true),
            8 => pen.set_attr(CONCEAL_ATTR, true),
            9 => pen.set_attr(STRIKETHROUGH_ATTR, true),
            22 => {
                pen.set_attr(BOLD_ATTR, false);
                pen.set_attr(FAINT_ATTR, false);
            }
            23 => pen.set_attr(ITALIC_ATTR, false),
            24 => pen.ul_style = NO_UNDERLINE,
            25 => {
                pen.set_attr(SLOW_BLINK_ATTR, false);
                pen.set_attr(RAPID_BLINK_ATTR, false);
            }
            27 => pen.set_attr(REVERSE_ATTR, false),
            28 => pen.set_attr(CONCEAL_ATTR, false),
            29 => pen.set_attr(STRIKETHROUGH_ATTR, false),
            30..=37 => pen.fg = Some(AnsiColor::Basic((param - 30) as u8)),
            38 | 48 | 58 => {
                let (n, c) = read_style_color(&params[i..]);
                if n > 0 {
                    // A nil colour (implementation defined) still "sets" nil.
                    let c = c.flatten();
                    match param {
                        38 => pen.fg = c,
                        48 => pen.bg = c,
                        _ => pen.ul = c,
                    }
                    i += n - 1;
                }
            }
            39 => pen.fg = None,
            40..=47 => pen.bg = Some(AnsiColor::Basic((param - 40) as u8)),
            49 => pen.bg = None,
            59 => pen.ul = None,
            90..=97 => pen.fg = Some(AnsiColor::Basic((param - 90 + 8) as u8)),
            100..=107 => pen.bg = Some(AnsiColor::Basic((param - 100 + 8) as u8)),
            _ => {}
        }
        i += 1;
    }
}

/// `cellbuf.ReadLink`.
pub fn read_link(p: &[u8], link: &mut Link) {
    let params: Vec<&[u8]> = p.split(|&b| b == b';').collect();
    if params.len() != 3 {
        return;
    }
    link.params = String::from_utf8_lossy(params[1]).into_owned();
    link.url = String::from_utf8_lossy(params[2]).into_owned();
}

fn is_space_byte(r: u8) -> bool {
    // unicode.IsSpace for a single-byte rune.
    matches!(r, b'\t' | b'\n' | 0x0b | 0x0c | b'\r' | b' ')
}

/// `cellbuf.Wrap`: wraps `s` to `limit` cells, preferring word boundaries
/// (spaces, `-`, and `breakpoints`), preserving ANSI styles and hyperlinks
/// across line breaks.
#[allow(unused_assignments)]
pub fn wrap(s: &str, limit: i32, breakpoints: &str) -> String {
    if s.is_empty() {
        return String::new();
    }
    if limit < 1 {
        return s.to_string();
    }
    let mut p = Parser::new();
    let mut buf: Vec<u8> = Vec::new();
    let mut word: Vec<u8> = Vec::new();
    let mut space: Vec<u8> = Vec::new();
    let mut style = CellStyle::default();
    let mut cur_style = CellStyle::default();
    let mut link = Link::default();
    let mut cur_link = Link::default();
    let mut cur_width: i32 = 0;
    let mut word_len: i32 = 0;
    let limit_i = limit;

    let bp: Vec<char> = breakpoints.chars().collect();

    macro_rules! add_space {
        () => {{
            cur_width += space.len() as i32;
            buf.extend_from_slice(&space);
            space.clear();
        }};
    }
    macro_rules! add_word {
        () => {{
            if !word.is_empty() {
                cur_link = link.clone();
                cur_style = style;
                add_space!();
                cur_width += word_len;
                buf.extend_from_slice(&word);
                word.clear();
                word_len = 0;
            }
        }};
    }
    macro_rules! add_newline {
        () => {{
            if !cur_style.empty() {
                buf.extend_from_slice(RESET_STYLE.as_bytes());
            }
            if !cur_link.empty() {
                buf.extend_from_slice(reset_hyperlink().as_bytes());
            }
            buf.push(b'\n');
            if !cur_link.empty() {
                buf.extend_from_slice(set_hyperlink(&cur_link.url, &cur_link.params).as_bytes());
            }
            if !cur_style.empty() {
                buf.extend_from_slice(cur_style.sequence().as_bytes());
            }
            cur_width = 0;
            space.clear();
        }};
    }

    let has_blank_style = |style: &CellStyle| {
        style.attrs & REVERSE_ATTR == 0 && style.bg.is_none() && style.ul_style == NO_UNDERLINE
    };

    let mut rest = s.as_bytes();
    let mut state = 0u8;
    while !rest.is_empty() {
        let (n, width, new_state) = decode::decode_sequence(rest, state, Some(&mut p));
        let seq = &rest[..n];
        'sw: {
            if width == 0 {
                if seq == b"\t" {
                    add_word!();
                    space.extend_from_slice(seq);
                    break 'sw;
                } else if seq == b"\n" {
                    if word_len == 0 {
                        if cur_width + space.len() as i32 > limit_i {
                            cur_width = 0;
                        } else {
                            buf.extend_from_slice(&space);
                        }
                        space.clear();
                    }
                    add_word!();
                    add_newline!();
                    break 'sw;
                } else if decode::has_csi_prefix(seq) && p.command() == b'm' as i64 {
                    read_style(&p.params(), &mut style);
                } else if decode::has_osc_prefix(seq) && p.command() == 8 {
                    read_link(p.data(), &mut link);
                }
                word.extend_from_slice(seq);
            } else {
                if seq.len() == 1 {
                    let r = seq[0];
                    if is_space_byte(r) && has_blank_style(&style) {
                        add_word!();
                        space.push(r);
                        break 'sw;
                    } else if r == b'-' || bp.contains(&(r as char)) {
                        add_space!();
                        if cur_width + word_len + width <= limit_i {
                            add_word!();
                            buf.extend_from_slice(seq);
                            cur_width += width;
                            break 'sw;
                        }
                    }
                }
                if word_len + width > limit_i {
                    add_word!();
                }
                word.extend_from_slice(seq);
                word_len += width;
                if cur_width + word_len + space.len() as i32 > limit_i {
                    add_newline!();
                }
            }
        }
        rest = &rest[n.max(1).min(rest.len())..];
        state = new_state;
    }

    if word_len == 0 {
        // Go resets curWidth here when the trailing space overflows; the
        // value is not used afterwards.
        if cur_width + space.len() as i32 <= limit_i {
            buf.extend_from_slice(&space);
        }
        space.clear();
    }
    add_word!();
    if !cur_link.empty() {
        buf.extend_from_slice(reset_hyperlink().as_bytes());
    }
    if !cur_style.empty() {
        buf.extend_from_slice(RESET_STYLE.as_bytes());
    }
    bytes_to_string(buf)
}
