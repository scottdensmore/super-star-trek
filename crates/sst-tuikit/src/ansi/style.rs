//! `x/ansi` style.go / color.go subset: SGR builder, colours, and
//! `ReadStyleColor`.

use super::decode::Param;

/// `ansi.ResetStyle`.
pub const RESET_STYLE: &str = "\x1b[m";

/// An `x/ansi` colour as produced by SGR parsing (`ansi.Color`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnsiColor {
    /// `ansi.BasicColor` (0-15).
    Basic(u8),
    /// `ansi.IndexedColor` / `ExtendedColor` (0-255).
    Indexed(u8),
    /// Any `color.Color` given by its 16-bit RGBA values.
    Rgba(u32, u32, u32, u32),
}

impl AnsiColor {
    /// `color.RGBA{R,G,B,A}` (8-bit components).
    pub fn from_rgba8(r: u8, g: u8, b: u8, a: u8) -> Self {
        let x = |v: u8| (v as u32) | ((v as u32) << 8);
        AnsiColor::Rgba(x(r), x(g), x(b), x(a))
    }
    /// `color.CMYK{C,M,Y,K}`.
    pub fn from_cmyk(c: u8, m: u8, y: u8, k: u8) -> Self {
        let w = 0xffff - (k as u32) * 0x101;
        let r = (0xffff - (c as u32) * 0x101) * w / 0xffff;
        let g = (0xffff - (m as u32) * 0x101) * w / 0xffff;
        let b = (0xffff - (y as u32) * 0x101) * w / 0xffff;
        AnsiColor::Rgba(r, g, b, 0xffff)
    }
}

fn shift(x: u32) -> u32 {
    if x > 0xff {
        x >> 8
    } else {
        x
    }
}

fn color_string(
    c: Option<AnsiColor>,
    base: u32,
    bright_base: u32,
    ext: &str,
    default: &str,
) -> String {
    match c {
        None => default.to_string(),
        Some(AnsiColor::Basic(b)) => {
            if b < 8 {
                (base + b as u32).to_string()
            } else if b < 16 {
                (bright_base + (b as u32 - 8)).to_string()
            } else {
                // Unknown BasicColor values fall through the Go switch.
                String::new()
            }
        }
        Some(AnsiColor::Indexed(i)) => format!("{ext};5;{i}"),
        Some(AnsiColor::Rgba(r, g, b, _)) => {
            format!("{ext};2;{};{};{}", shift(r), shift(g), shift(b))
        }
    }
}

/// An SGR attribute list (`ansi.Style`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SgrStyle(pub Vec<String>);

impl SgrStyle {
    fn push(mut self, s: &str) -> Self {
        self.0.push(s.to_string());
        self
    }
    /// `Style.String`.
    pub fn string(&self) -> String {
        if self.0.is_empty() {
            return RESET_STYLE.to_string();
        }
        format!("\x1b[{}m", self.0.join(";"))
    }
    pub fn bold(self) -> Self {
        self.push("1")
    }
    pub fn faint(self) -> Self {
        self.push("2")
    }
    pub fn italic(self, v: bool) -> Self {
        self.push(if v { "3" } else { "23" })
    }
    pub fn underline(self, v: bool) -> Self {
        self.push(if v { "4" } else { "24" })
    }
    pub fn underline_style(self, u: u8) -> Self {
        match u {
            0 => self.underline(false),
            1 => self.underline(true),
            2 => self.push("4:2"),
            3 => self.push("4:3"),
            4 => self.push("4:4"),
            5 => self.push("4:5"),
            _ => self,
        }
    }
    pub fn blink(self, v: bool) -> Self {
        self.push(if v { "5" } else { "25" })
    }
    pub fn rapid_blink(self, v: bool) -> Self {
        self.push(if v { "6" } else { "25" })
    }
    pub fn reverse(self, v: bool) -> Self {
        self.push(if v { "7" } else { "27" })
    }
    pub fn conceal(self, v: bool) -> Self {
        self.push(if v { "8" } else { "28" })
    }
    pub fn strikethrough(self, v: bool) -> Self {
        self.push(if v { "9" } else { "29" })
    }
    pub fn foreground_color(self, c: Option<AnsiColor>) -> Self {
        let s = color_string(c, 30, 90, "38", "39");
        self.push(&s)
    }
    pub fn background_color(self, c: Option<AnsiColor>) -> Self {
        let s = color_string(c, 40, 100, "48", "49");
        self.push(&s)
    }
    pub fn underline_color(self, c: Option<AnsiColor>) -> Self {
        let s = match c {
            None => "59".to_string(),
            Some(AnsiColor::Basic(b)) => format!("58;5;{b}"),
            Some(AnsiColor::Indexed(i)) => format!("58;5;{i}"),
            Some(AnsiColor::Rgba(r, g, b, _)) => {
                format!("58;2;{};{};{}", shift(r), shift(g), shift(b))
            }
        };
        self.push(&s)
    }
}

/// `ansi.ReadStyleColor`: decodes an extended colour starting at
/// `params[0]` (38/48/58). Returns the number of params consumed (0 on
/// failure) and the colour (`None` with n>0 means "implementation defined",
/// i.e. no change).
pub fn read_style_color(params: &[Param]) -> (usize, Option<Option<AnsiColor>>) {
    if params.len() < 2 {
        return (0, None);
    }
    let s = params[0];
    let p = params[1];
    let color_type = p.param(0);
    let mut n = 2usize;
    let get = |i: usize| params.get(i).copied().unwrap_or(Param(0));
    let paramsfn = |n: &mut usize| -> (i64, i64, i64, i64) {
        let hm = |i: usize| get(i).has_more();
        if s.has_more()
            && p.has_more()
            && params.len() > 8
            && hm(2)
            && hm(3)
            && hm(4)
            && hm(5)
            && hm(6)
            && hm(7)
        {
            *n += 7;
            return (
                get(3).param(0),
                get(4).param(0),
                get(5).param(0),
                get(6).param(0),
            );
        }
        if s.has_more()
            && p.has_more()
            && params.len() > 7
            && hm(2)
            && hm(3)
            && hm(4)
            && hm(5)
            && hm(6)
        {
            *n += 6;
            return (
                get(3).param(0),
                get(4).param(0),
                get(5).param(0),
                get(6).param(0),
            );
        }
        if s.has_more() && p.has_more() && params.len() > 6 && hm(2) && hm(3) && hm(4) && hm(5) {
            *n += 5;
            return (
                get(3).param(0),
                get(4).param(0),
                get(5).param(0),
                get(6).param(0),
            );
        }
        if s.has_more() && p.has_more() && params.len() > 5 && hm(2) && hm(3) && hm(4) && !hm(5) {
            *n += 4;
            return (get(3).param(0), get(4).param(0), get(5).param(0), -1);
        }
        let case_a = s.has_more() && p.has_more() && p.param(0) == 2 && hm(2) && hm(3) && !hm(4);
        let case_b =
            !s.has_more() && !p.has_more() && p.param(0) == 2 && !hm(2) && !hm(3) && !hm(4);
        if case_a || case_b {
            *n += 3;
            return (get(2).param(0), get(3).param(0), get(4).param(0), -1);
        }
        (-1, -1, -1, -1)
    };
    match color_type {
        0 => (2, None),
        1 => (2, Some(Some(AnsiColor::Rgba(0, 0, 0, 0)))),
        2 => {
            if params.len() < 5 {
                return (0, None);
            }
            let (r, g, b, _) = paramsfn(&mut n);
            if r == -1 || g == -1 || b == -1 {
                return (0, None);
            }
            (
                n,
                Some(Some(AnsiColor::from_rgba8(r as u8, g as u8, b as u8, 0xff))),
            )
        }
        3 => {
            if params.len() < 5 {
                return (0, None);
            }
            let (c, m, y, _) = paramsfn(&mut n);
            if c == -1 || m == -1 || y == -1 {
                return (0, None);
            }
            (
                n,
                Some(Some(AnsiColor::from_cmyk(c as u8, m as u8, y as u8, 0))),
            )
        }
        4 => {
            if params.len() < 6 {
                return (0, None);
            }
            let (c, m, y, k) = paramsfn(&mut n);
            if c == -1 || m == -1 || y == -1 || k == -1 {
                return (0, None);
            }
            (
                n,
                Some(Some(AnsiColor::from_cmyk(
                    c as u8, m as u8, y as u8, k as u8,
                ))),
            )
        }
        5 => {
            if params.len() < 3 {
                return (0, None);
            }
            let ok = (s.has_more() && p.has_more() && !params[2].has_more())
                || (!s.has_more() && !p.has_more() && !params[2].has_more());
            if !ok {
                return (0, None);
            }
            (3, Some(Some(AnsiColor::Indexed(params[2].param(0) as u8))))
        }
        6 => {
            if params.len() < 6 {
                return (0, None);
            }
            let (r, g, b, a) = paramsfn(&mut n);
            if r == -1 || g == -1 || b == -1 || a == -1 {
                return (0, None);
            }
            (
                n,
                Some(Some(AnsiColor::from_rgba8(
                    r as u8, g as u8, b as u8, a as u8,
                ))),
            )
        }
        _ => (0, None),
    }
}
