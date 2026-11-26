//! Port of the `github.com/muesli/termenv` v0.16.0 subset used by lipgloss:
//! colour profiles, colour parsing and degradation (`color.go`,
//! `profile.go`), SGR styles (`style.go`), environment-based profile
//! detection (`termenv.go`, `termenv_unix.go`) and the background colour
//! query behind `HasDarkBackground`.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Duration;

use crate::colorful;
use crate::term;

/// A terminal colour profile (`termenv.Profile`). The numeric order matches
/// Go (`TrueColor` = 0 ... `Ascii` = 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Profile {
    TrueColor = 0,
    ANSI256 = 1,
    ANSI = 2,
    Ascii = 3,
}

pub use Profile::{Ascii, TrueColor, ANSI, ANSI256};

/// A termenv colour (`termenv.Color` implementations). Go's `nil` colour is
/// `Option::None` wherever it can occur.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Color {
    /// `termenv.NoColor{}`.
    NoColor,
    /// `termenv.ANSIColor` (0-15; Go accepts any int).
    ANSI(i64),
    /// `termenv.ANSI256Color`.
    ANSI256(i64),
    /// `termenv.RGBColor` (a "#rrggbb" string, unvalidated).
    RGB(String),
}

/// `termenv.ansiHex`.
pub(crate) const ANSI_HEX: [&str; 256] = [
    "#000000", "#800000", "#008000", "#808000", "#000080", "#800080", "#008080", "#c0c0c0",
    "#808080", "#ff0000", "#00ff00", "#ffff00", "#0000ff", "#ff00ff", "#00ffff", "#ffffff",
    "#000000", "#00005f", "#000087", "#0000af", "#0000d7", "#0000ff", "#005f00", "#005f5f",
    "#005f87", "#005faf", "#005fd7", "#005fff", "#008700", "#00875f", "#008787", "#0087af",
    "#0087d7", "#0087ff", "#00af00", "#00af5f", "#00af87", "#00afaf", "#00afd7", "#00afff",
    "#00d700", "#00d75f", "#00d787", "#00d7af", "#00d7d7", "#00d7ff", "#00ff00", "#00ff5f",
    "#00ff87", "#00ffaf", "#00ffd7", "#00ffff", "#5f0000", "#5f005f", "#5f0087", "#5f00af",
    "#5f00d7", "#5f00ff", "#5f5f00", "#5f5f5f", "#5f5f87", "#5f5faf", "#5f5fd7", "#5f5fff",
    "#5f8700", "#5f875f", "#5f8787", "#5f87af", "#5f87d7", "#5f87ff", "#5faf00", "#5faf5f",
    "#5faf87", "#5fafaf", "#5fafd7", "#5fafff", "#5fd700", "#5fd75f", "#5fd787", "#5fd7af",
    "#5fd7d7", "#5fd7ff", "#5fff00", "#5fff5f", "#5fff87", "#5fffaf", "#5fffd7", "#5fffff",
    "#870000", "#87005f", "#870087", "#8700af", "#8700d7", "#8700ff", "#875f00", "#875f5f",
    "#875f87", "#875faf", "#875fd7", "#875fff", "#878700", "#87875f", "#878787", "#8787af",
    "#8787d7", "#8787ff", "#87af00", "#87af5f", "#87af87", "#87afaf", "#87afd7", "#87afff",
    "#87d700", "#87d75f", "#87d787", "#87d7af", "#87d7d7", "#87d7ff", "#87ff00", "#87ff5f",
    "#87ff87", "#87ffaf", "#87ffd7", "#87ffff", "#af0000", "#af005f", "#af0087", "#af00af",
    "#af00d7", "#af00ff", "#af5f00", "#af5f5f", "#af5f87", "#af5faf", "#af5fd7", "#af5fff",
    "#af8700", "#af875f", "#af8787", "#af87af", "#af87d7", "#af87ff", "#afaf00", "#afaf5f",
    "#afaf87", "#afafaf", "#afafd7", "#afafff", "#afd700", "#afd75f", "#afd787", "#afd7af",
    "#afd7d7", "#afd7ff", "#afff00", "#afff5f", "#afff87", "#afffaf", "#afffd7", "#afffff",
    "#d70000", "#d7005f", "#d70087", "#d700af", "#d700d7", "#d700ff", "#d75f00", "#d75f5f",
    "#d75f87", "#d75faf", "#d75fd7", "#d75fff", "#d78700", "#d7875f", "#d78787", "#d787af",
    "#d787d7", "#d787ff", "#d7af00", "#d7af5f", "#d7af87", "#d7afaf", "#d7afd7", "#d7afff",
    "#d7d700", "#d7d75f", "#d7d787", "#d7d7af", "#d7d7d7", "#d7d7ff", "#d7ff00", "#d7ff5f",
    "#d7ff87", "#d7ffaf", "#d7ffd7", "#d7ffff", "#ff0000", "#ff005f", "#ff0087", "#ff00af",
    "#ff00d7", "#ff00ff", "#ff5f00", "#ff5f5f", "#ff5f87", "#ff5faf", "#ff5fd7", "#ff5fff",
    "#ff8700", "#ff875f", "#ff8787", "#ff87af", "#ff87d7", "#ff87ff", "#ffaf00", "#ffaf5f",
    "#ffaf87", "#ffafaf", "#ffafd7", "#ffafff", "#ffd700", "#ffd75f", "#ffd787", "#ffd7af",
    "#ffd7d7", "#ffd7ff", "#ffff00", "#ffff5f", "#ffff87", "#ffffaf", "#ffffd7", "#ffffff",
    "#080808", "#121212", "#1c1c1c", "#262626", "#303030", "#3a3a3a", "#444444", "#4e4e4e",
    "#585858", "#626262", "#6c6c6c", "#767676", "#808080", "#8a8a8a", "#949494", "#9e9e9e",
    "#a8a8a8", "#b2b2b2", "#bcbcbc", "#c6c6c6", "#d0d0d0", "#dadada", "#e4e4e4", "#eeeeee",
];

fn ansi_hex(i: i64) -> &'static str {
    if (0..256).contains(&i) {
        ANSI_HEX[i as usize]
    } else {
        // Go would panic with an index out of range.
        ""
    }
}

impl Color {
    /// `Color.Sequence(bg)`.
    pub fn sequence(&self, bg: bool) -> String {
        match self {
            Color::NoColor => String::new(),
            Color::ANSI(col) => {
                let col = *col;
                let bg_mod = |c: i64| if bg { c + 10 } else { c };
                if col < 8 {
                    format!("{}", bg_mod(col) + 30)
                } else {
                    format!("{}", bg_mod(col - 8) + 90)
                }
            }
            Color::ANSI256(c) => {
                let prefix = if bg { "48" } else { "38" };
                format!("{prefix};5;{c}")
            }
            Color::RGB(s) => match colorful::hex(s) {
                None => String::new(),
                Some(f) => {
                    let prefix = if bg { "48" } else { "38" };
                    format!(
                        "{prefix};2;{};{};{}",
                        (f.r * 255.0) as u8,
                        (f.g * 255.0) as u8,
                        (f.b * 255.0) as u8
                    )
                }
            },
        }
    }

    /// `ANSIColor.String()` / `ANSI256Color.String()` (hex value).
    pub fn string(&self) -> String {
        match self {
            Color::NoColor => String::new(),
            Color::ANSI(i) | Color::ANSI256(i) => ansi_hex(*i).to_string(),
            Color::RGB(s) => s.clone(),
        }
    }
}

/// `termenv.ConvertToRGB`.
pub fn convert_to_rgb(c: Option<&Color>) -> colorful::Color {
    let hex = match c {
        Some(Color::RGB(s)) => s.as_str(),
        Some(Color::ANSI(i)) | Some(Color::ANSI256(i)) => ansi_hex(*i),
        _ => "",
    };
    colorful::hex(hex).unwrap_or_default()
}

/// `ansi256ToANSIColor` (memoised; the mapping is a pure function).
fn ansi256_to_ansi_color(c: i64) -> i64 {
    static TABLE: OnceLock<[u8; 256]> = OnceLock::new();
    if !(0..256).contains(&c) {
        return c;
    }
    let t = TABLE.get_or_init(|| {
        let mut t = [0u8; 256];
        for (ci, slot) in t.iter_mut().enumerate() {
            let mut r = 0usize;
            let mut md = f64::MAX;
            let h = colorful::hex(ANSI_HEX[ci]).unwrap_or_default();
            for (i, hx) in ANSI_HEX.iter().enumerate().take(16) {
                let hb = colorful::hex(hx).unwrap_or_default();
                let d = h.distance_hsluv(&hb);
                if d < md {
                    md = d;
                    r = i;
                }
            }
            *slot = r as u8;
        }
        t
    });
    t[c as usize] as i64
}

/// `hexToANSI256Color`.
pub(crate) fn hex_to_ansi256_color(c: &colorful::Color) -> i64 {
    let v2ci = |v: f64| -> i64 {
        if v < 48.0 {
            return 0;
        }
        if v < 115.0 {
            return 1;
        }
        ((v - 35.0) / 40.0) as i64
    };
    let r = v2ci(c.r * 255.0);
    let g = v2ci(c.g * 255.0);
    let b = v2ci(c.b * 255.0);
    let ci = 36 * r + 6 * g + b;
    const I2CV: [i64; 6] = [0, 0x5f, 0x87, 0xaf, 0xd7, 0xff];
    let idx = |v: i64| I2CV[v.clamp(0, 5) as usize];
    let cr = idx(r);
    let cg = idx(g);
    let cb = idx(b);
    let average = (r + g + b) / 3;
    let gray_idx = if average > 238 {
        23
    } else {
        (average - 3) / 10
    };
    let gv = 8 + 10 * gray_idx;
    let c2 = colorful::Color {
        r: cr as f64 / 255.0,
        g: cg as f64 / 255.0,
        b: cb as f64 / 255.0,
    };
    let g2 = colorful::Color {
        r: gv as f64 / 255.0,
        g: gv as f64 / 255.0,
        b: gv as f64 / 255.0,
    };
    let color_dist = c.distance_hsluv(&c2);
    let gray_dist = c.distance_hsluv(&g2);
    if color_dist <= gray_dist {
        16 + ci
    } else {
        232 + gray_idx
    }
}

thread_local! {
    static HEX256_CACHE: RefCell<HashMap<String, Option<i64>>> = RefCell::new(HashMap::new());
}

fn rgb_to_ansi256(s: &str) -> Option<i64> {
    HEX256_CACHE.with(|c| {
        if let Some(v) = c.borrow().get(s) {
            return *v;
        }
        let v = colorful::hex(s).map(|h| hex_to_ansi256_color(&h));
        c.borrow_mut().insert(s.to_string(), v);
        v
    })
}

/// Go `strconv.Atoi`.
pub(crate) fn go_atoi(s: &str) -> Option<i64> {
    let b = s.as_bytes();
    if b.is_empty() {
        return None;
    }
    let (neg, digits) = match b[0] {
        b'+' => (false, &s[1..]),
        b'-' => (true, &s[1..]),
        _ => (false, s),
    };
    if digits.is_empty() || !digits.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let v: i64 = digits.parse().ok()?;
    Some(if neg { -v } else { v })
}

impl Profile {
    /// `Profile.Name`.
    pub fn name(self) -> &'static str {
        match self {
            Ascii => "Ascii",
            ANSI => "ANSI",
            ANSI256 => "ANSI256",
            TrueColor => "TrueColor",
        }
    }

    /// `Profile.Convert`.
    pub fn convert(self, c: Color) -> Option<Color> {
        if self == Ascii {
            return Some(Color::NoColor);
        }
        match c {
            Color::ANSI(v) => Some(Color::ANSI(v)),
            Color::ANSI256(v) => {
                if self == ANSI {
                    Some(Color::ANSI(ansi256_to_ansi_color(v)))
                } else {
                    Some(Color::ANSI256(v))
                }
            }
            Color::RGB(v) => {
                colorful::hex(&v)?;
                if self != TrueColor {
                    let ac = rgb_to_ansi256(&v)?;
                    if self == ANSI {
                        return Some(Color::ANSI(ansi256_to_ansi_color(ac)));
                    }
                    return Some(Color::ANSI256(ac));
                }
                Some(Color::RGB(v))
            }
            Color::NoColor => Some(Color::NoColor),
        }
    }

    /// `Profile.Color`: parses "#rrggbb" or an ANSI index and converts it.
    pub fn color(self, s: &str) -> Option<Color> {
        if s.is_empty() {
            return None;
        }
        let c = if s.starts_with('#') {
            Color::RGB(s.to_string())
        } else {
            let i = go_atoi(s)?;
            if i < 16 {
                Color::ANSI(i)
            } else {
                Color::ANSI256(i)
            }
        };
        self.convert(c)
    }

    /// `Profile.String(s...)`: a style bound to this profile.
    pub fn string(self, s: &[&str]) -> Style {
        Style {
            profile: self,
            string: s.join(" "),
            styles: Vec::new(),
        }
    }
}

/// An SGR style (`termenv.Style`). The zero value (`Style::default()`) has
/// the `TrueColor` profile, exactly like Go's `termenv.Style{}`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Style {
    profile: Profile,
    string: String,
    styles: Vec<String>,
}

impl Default for Style {
    fn default() -> Self {
        Style {
            profile: TrueColor,
            string: String::new(),
            styles: Vec::new(),
        }
    }
}

pub const CSI: &str = "\x1b[";
pub const RESET_SEQ: &str = "0";

impl Style {
    /// `Style.Styled`.
    pub fn styled(&self, s: &str) -> String {
        if self.profile == Ascii {
            return s.to_string();
        }
        if self.styles.is_empty() {
            return s.to_string();
        }
        let seq = self.styles.join(";");
        if seq.is_empty() {
            return s.to_string();
        }
        format!("{CSI}{seq}m{s}{CSI}{RESET_SEQ}m")
    }
    /// `Style.String`.
    pub fn string(&self) -> String {
        self.styled(&self.string)
    }
    fn push(mut self, s: &str) -> Self {
        self.styles.push(s.to_string());
        self
    }
    /// `Style.Foreground` (a `None` colour is ignored, like Go's nil).
    pub fn foreground(self, c: Option<Color>) -> Self {
        match c {
            Some(c) => {
                let s = c.sequence(false);
                self.push(&s)
            }
            None => self,
        }
    }
    /// `Style.Background`.
    pub fn background(self, c: Option<Color>) -> Self {
        match c {
            Some(c) => {
                let s = c.sequence(true);
                self.push(&s)
            }
            None => self,
        }
    }
    pub fn bold(self) -> Self {
        self.push("1")
    }
    pub fn faint(self) -> Self {
        self.push("2")
    }
    pub fn italic(self) -> Self {
        self.push("3")
    }
    pub fn underline(self) -> Self {
        self.push("4")
    }
    pub fn overline(self) -> Self {
        self.push("53")
    }
    pub fn blink(self) -> Self {
        self.push("5")
    }
    pub fn reverse(self) -> Self {
        self.push("7")
    }
    pub fn cross_out(self) -> Self {
        self.push("9")
    }
    /// `Style.Width` (uniseg width of the bound string).
    pub fn width(&self) -> i32 {
        crate::unicode::uniseg_string_width(&self.string)
    }
}

/// Environment lookup used by detection (`termenv.Environ`).
pub trait Environ {
    fn getenv(&self, key: &str) -> String;
}

/// The process environment.
pub struct OsEnviron;

impl Environ for OsEnviron {
    fn getenv(&self, key: &str) -> String {
        std::env::var(key).unwrap_or_default()
    }
}

impl<F: Fn(&str) -> String> Environ for F {
    fn getenv(&self, key: &str) -> String {
        self(key)
    }
}

/// A terminal output (`termenv.Output`), reduced to what detection needs.
pub struct Output<'a> {
    environ: &'a dyn Environ,
    /// Output file descriptor (None: not a file, e.g. a buffer).
    fd: Option<term::Fd>,
    assume_tty: bool,
    unsafe_mode: bool,
}

impl<'a> Output<'a> {
    /// An output writing to `fd` with the given environment.
    pub fn new(environ: &'a dyn Environ, fd: Option<term::Fd>) -> Self {
        Output {
            environ,
            fd,
            assume_tty: false,
            unsafe_mode: false,
        }
    }
    /// `WithTTY`.
    pub fn with_tty(mut self, v: bool) -> Self {
        self.assume_tty = v;
        self
    }
    /// `WithUnsafe`.
    pub fn with_unsafe(mut self) -> Self {
        self.unsafe_mode = true;
        self
    }

    fn is_tty(&self) -> bool {
        if self.assume_tty || self.unsafe_mode {
            return true;
        }
        if !self.environ.getenv("CI").is_empty() {
            return false;
        }
        match self.fd {
            Some(fd) => term::is_terminal(fd),
            None => false,
        }
    }

    /// `Output.ColorProfile` (unix).
    pub fn color_profile(&self) -> Profile {
        if !self.is_tty() {
            return Ascii;
        }
        let env = |k: &str| self.environ.getenv(k);
        if env("GOOGLE_CLOUD_SHELL") == "true" {
            return TrueColor;
        }
        let term = env("TERM");
        let color_term = env("COLORTERM");
        match color_term.to_lowercase().as_str() {
            "24bit" | "truecolor" => {
                if term.starts_with("screen") && env("TERM_PROGRAM") != "tmux" {
                    return ANSI256;
                }
                return TrueColor;
            }
            "yes" | "true" => return ANSI256,
            _ => {}
        }
        match term.as_str() {
            "alacritty" | "contour" | "rio" | "wezterm" | "xterm-ghostty" | "xterm-kitty" => {
                return TrueColor
            }
            "linux" | "xterm" => return ANSI,
            _ => {}
        }
        if term.contains("256color") {
            return ANSI256;
        }
        if term.contains("color") {
            return ANSI;
        }
        if term.contains("ansi") {
            return ANSI;
        }
        Ascii
    }

    fn cli_color_forced(&self) -> bool {
        let forced = self.environ.getenv("CLICOLOR_FORCE");
        if !forced.is_empty() {
            return forced != "0";
        }
        false
    }

    /// `Output.EnvNoColor`.
    pub fn env_no_color(&self) -> bool {
        !self.environ.getenv("NO_COLOR").is_empty()
            || (self.environ.getenv("CLICOLOR") == "0" && !self.cli_color_forced())
    }

    /// `Output.EnvColorProfile`.
    pub fn env_color_profile(&self) -> Profile {
        if self.env_no_color() {
            return Ascii;
        }
        let p = self.color_profile();
        if self.cli_color_forced() && p == Ascii {
            return ANSI;
        }
        p
    }

    /// `Output.BackgroundColor` (uncached).
    pub fn background_color(&self) -> Color {
        if !self.is_tty() {
            return Color::NoColor;
        }
        if let Ok(s) = self.term_status_report(11) {
            if let Some(c) = xterm_color(&s) {
                return c;
            }
        }
        let colorfgbg = self.environ.getenv("COLORFGBG");
        if colorfgbg.contains(';') {
            let c: Vec<&str> = colorfgbg.split(';').collect();
            if let Some(i) = go_atoi(c[c.len() - 1]) {
                return Color::ANSI(i);
            }
        }
        Color::ANSI(0)
    }

    /// `Output.HasDarkBackground`.
    pub fn has_dark_background(&self) -> bool {
        let c = convert_to_rgb(Some(&self.background_color()));
        let (_, _, l) = c.hsl();
        l < 0.5
    }

    /// `termStatusReport`: queries OSC `sequence` and returns the response.
    fn term_status_report(&self, sequence: i32) -> Result<String, ()> {
        let term_env = self.environ.getenv("TERM");
        if term_env.starts_with("screen")
            || term_env.starts_with("tmux")
            || term_env.starts_with("dumb")
        {
            return Err(());
        }
        let fd = self.fd.ok_or(())?;
        let mut saved = None;
        if !self.unsafe_mode {
            if !term::is_foreground(fd) {
                return Err(());
            }
            saved = Some(term::set_noecho_noncanon(fd).map_err(|_| ())?);
        }
        let result = (|| {
            let q = format!("\x1b]{sequence};?\x1b\\\x1b[6n");
            term::write_all(fd, q.as_bytes()).map_err(|_| ())?;
            let (res, is_osc) = self.read_next_response(fd)?;
            if !is_osc {
                return Err(());
            }
            self.read_next_response(fd)?;
            Ok(res)
        })();
        if let Some(st) = saved {
            let _ = term::restore(fd, &st);
        }
        result
    }

    fn read_next_byte(&self, fd: term::Fd) -> Result<u8, ()> {
        if !self.unsafe_mode && !term::wait_readable(fd, OSC_TIMEOUT).map_err(|_| ())? {
            return Err(());
        }
        let mut b = [0u8; 1];
        let n = term::read(fd, &mut b).map_err(|_| ())?;
        if n == 0 {
            return Err(());
        }
        Ok(b[0])
    }

    fn read_next_response(&self, fd: term::Fd) -> Result<(String, bool), ()> {
        let mut start = self.read_next_byte(fd)?;
        while start != 0x1b {
            start = self.read_next_byte(fd)?;
        }
        let mut response: Vec<u8> = vec![start];
        let tpe = self.read_next_byte(fd)?;
        response.push(tpe);
        let osc = match tpe {
            b'[' => false,
            b']' => true,
            _ => return Err(()),
        };
        loop {
            let b = self.read_next_byte(fd)?;
            response.push(b);
            if osc {
                if b == 0x07 || response.ends_with(b"\x1b") {
                    return Ok((String::from_utf8_lossy(&response).into_owned(), true));
                }
            } else if b == b'R' {
                return Ok((String::from_utf8_lossy(&response).into_owned(), false));
            }
            if response.len() > 25 {
                break;
            }
        }
        Err(())
    }
}

/// `termenv.OSCTimeout`.
pub const OSC_TIMEOUT: Duration = Duration::from_secs(5);

/// `xTermColor`: parses an OSC colour response.
pub fn xterm_color(s: &str) -> Option<Color> {
    if s.len() < 24 || s.len() > 25 {
        return None;
    }
    let s = if let Some(x) = s.strip_suffix('\x07') {
        x
    } else if let Some(x) = s.strip_suffix("\x1b\\") {
        // Go checks the ESC suffix before ST; an ST-terminated response ends
        // in '\\' so the ESC case cannot match it.
        x
    } else {
        s.strip_suffix('\x1b')?
    };
    let s = s.get(4..)?;
    let s = s.strip_prefix(";rgb:")?;
    let h: Vec<&str> = s.split('/').collect();
    if h.len() < 3 {
        return None;
    }
    Some(Color::RGB(format!(
        "#{}{}{}",
        h[0].get(..2)?,
        h[1].get(..2)?,
        h[2].get(..2)?
    )))
}

/// The environment-detected profile of stdout (computed once per process,
/// like the default lipgloss renderer's `sync.Once`).
pub fn env_color_profile() -> Profile {
    static P: OnceLock<Profile> = OnceLock::new();
    *P.get_or_init(|| Output::new(&OsEnviron, Some(term::STDOUT)).env_color_profile())
}

/// `termenv.HasDarkBackground()` of stdout (queried once per process).
pub fn has_dark_background() -> bool {
    static D: OnceLock<bool> = OnceLock::new();
    *D.get_or_init(|| Output::new(&OsEnviron, Some(term::STDOUT)).has_dark_background())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequences() {
        assert_eq!(
            TrueColor.color("#ff0000").unwrap().sequence(false),
            "38;2;255;0;0"
        );
        assert_eq!(
            ANSI256.color("#ff0000").unwrap().sequence(false),
            "38;5;196"
        );
        assert_eq!(ANSI.color("#ff0000").unwrap().sequence(true), "101");
        assert_eq!(Ascii.color("#ff0000"), Some(Color::NoColor));
        assert_eq!(TrueColor.color(""), None);
        assert_eq!(TrueColor.color("x"), None);
    }

    #[test]
    fn xterm_color_parsing() {
        assert_eq!(
            xterm_color("\x1b]11;rgb:1c1c/1c1c/1c1c\x1b\\"),
            Some(Color::RGB("#1c1c1c".into()))
        );
        assert_eq!(
            xterm_color("\x1b]11;rgb:ffff/ffff/ffff\x07"),
            Some(Color::RGB("#ffffff".into()))
        );
    }
}
