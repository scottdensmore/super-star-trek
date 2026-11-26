//! `lipgloss` style.go, set.go, get.go, unset.go.

use std::fmt;
use std::rc::Rc;

use super::align::{align_text_horizontal, align_text_vertical};
use super::borders::{no_border, Border};
use super::color::TerminalColor;
use super::position::Position;
use super::renderer;
use super::size::get_lines;
use crate::termenv;

const TAB_WIDTH_DEFAULT: i32 = 4;

/// `lipgloss.NoTabConversion`.
pub const NO_TAB_CONVERSION: i32 = -1;

type PropKey = u64;

const BOLD_KEY: PropKey = 1 << 0;
const ITALIC_KEY: PropKey = 1 << 1;
const UNDERLINE_KEY: PropKey = 1 << 2;
const STRIKETHROUGH_KEY: PropKey = 1 << 3;
const REVERSE_KEY: PropKey = 1 << 4;
const BLINK_KEY: PropKey = 1 << 5;
const FAINT_KEY: PropKey = 1 << 6;
const UNDERLINE_SPACES_KEY: PropKey = 1 << 7;
const STRIKETHROUGH_SPACES_KEY: PropKey = 1 << 8;
const COLOR_WHITESPACE_KEY: PropKey = 1 << 9;
const FOREGROUND_KEY: PropKey = 1 << 10;
const BACKGROUND_KEY: PropKey = 1 << 11;
const WIDTH_KEY: PropKey = 1 << 12;
const HEIGHT_KEY: PropKey = 1 << 13;
const ALIGN_HORIZONTAL_KEY: PropKey = 1 << 14;
const ALIGN_VERTICAL_KEY: PropKey = 1 << 15;
const PADDING_TOP_KEY: PropKey = 1 << 16;
const PADDING_RIGHT_KEY: PropKey = 1 << 17;
const PADDING_BOTTOM_KEY: PropKey = 1 << 18;
const PADDING_LEFT_KEY: PropKey = 1 << 19;
const MARGIN_TOP_KEY: PropKey = 1 << 20;
const MARGIN_RIGHT_KEY: PropKey = 1 << 21;
const MARGIN_BOTTOM_KEY: PropKey = 1 << 22;
const MARGIN_LEFT_KEY: PropKey = 1 << 23;
const MARGIN_BACKGROUND_KEY: PropKey = 1 << 24;
const BORDER_STYLE_KEY: PropKey = 1 << 25;
const BORDER_TOP_KEY: PropKey = 1 << 26;
const BORDER_RIGHT_KEY: PropKey = 1 << 27;
const BORDER_BOTTOM_KEY: PropKey = 1 << 28;
const BORDER_LEFT_KEY: PropKey = 1 << 29;
const BORDER_TOP_FOREGROUND_KEY: PropKey = 1 << 30;
const BORDER_RIGHT_FOREGROUND_KEY: PropKey = 1 << 31;
const BORDER_BOTTOM_FOREGROUND_KEY: PropKey = 1 << 32;
const BORDER_LEFT_FOREGROUND_KEY: PropKey = 1 << 33;
const BORDER_TOP_BACKGROUND_KEY: PropKey = 1 << 34;
const BORDER_RIGHT_BACKGROUND_KEY: PropKey = 1 << 35;
const BORDER_BOTTOM_BACKGROUND_KEY: PropKey = 1 << 36;
const BORDER_LEFT_BACKGROUND_KEY: PropKey = 1 << 37;
const INLINE_KEY: PropKey = 1 << 38;
const MAX_WIDTH_KEY: PropKey = 1 << 39;
const MAX_HEIGHT_KEY: PropKey = 1 << 40;
const TAB_WIDTH_KEY: PropKey = 1 << 41;
const TRANSFORM_KEY: PropKey = 1 << 42;

/// A string transform (`Style.Transform`).
pub type TransformFn = Rc<dyn Fn(&str) -> String>;

/// A set of style rules (`lipgloss.Style`). Like Go, every setter returns a
/// modified copy: `NewStyle().Bold(true).Padding(0, 1)` translates to
/// `new_style().bold(true).padding(&[0, 1])`.
#[derive(Clone, Default)]
pub struct Style {
    props: u64,
    value: String,
    attrs: u64,
    fg_color: TerminalColor,
    bg_color: TerminalColor,
    width: i32,
    height: i32,
    align_horizontal: Position,
    align_vertical: Position,
    padding_top: i32,
    padding_right: i32,
    padding_bottom: i32,
    padding_left: i32,
    margin_top: i32,
    margin_right: i32,
    margin_bottom: i32,
    margin_left: i32,
    margin_bg_color: TerminalColor,
    border_style: Border,
    border_top_fg_color: TerminalColor,
    border_right_fg_color: TerminalColor,
    border_bottom_fg_color: TerminalColor,
    border_left_fg_color: TerminalColor,
    border_top_bg_color: TerminalColor,
    border_right_bg_color: TerminalColor,
    border_bottom_bg_color: TerminalColor,
    border_left_bg_color: TerminalColor,
    max_width: i32,
    max_height: i32,
    tab_width: i32,
    transform: Option<TransformFn>,
}

impl fmt::Debug for Style {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Style")
            .field("props", &format_args!("{:#x}", self.props))
            .field("value", &self.value)
            .field("fg", &self.fg_color)
            .field("bg", &self.bg_color)
            .finish_non_exhaustive()
    }
}

/// `lipgloss.NewStyle()`.
pub fn new_style() -> Style {
    Style::default()
}

#[derive(Clone)]
enum Value {
    Bool(bool),
    Int(i32),
    Attrs(u64),
    Color(TerminalColor),
    Pos(Position),
    Border(Border),
    Transform(Option<TransformFn>),
}

impl Style {
    // ---- internal property machinery (set.go / get.go) ----

    fn is_set(&self, k: PropKey) -> bool {
        self.props & k != 0
    }

    fn set(&mut self, key: PropKey, value: Value) {
        match (key, value) {
            (FOREGROUND_KEY, Value::Color(c)) => self.fg_color = c,
            (BACKGROUND_KEY, Value::Color(c)) => self.bg_color = c,
            (WIDTH_KEY, Value::Int(v)) => self.width = v.max(0),
            (HEIGHT_KEY, Value::Int(v)) => self.height = v.max(0),
            (ALIGN_HORIZONTAL_KEY, Value::Pos(p)) => self.align_horizontal = p,
            (ALIGN_VERTICAL_KEY, Value::Pos(p)) => self.align_vertical = p,
            (PADDING_TOP_KEY, Value::Int(v)) => self.padding_top = v.max(0),
            (PADDING_RIGHT_KEY, Value::Int(v)) => self.padding_right = v.max(0),
            (PADDING_BOTTOM_KEY, Value::Int(v)) => self.padding_bottom = v.max(0),
            (PADDING_LEFT_KEY, Value::Int(v)) => self.padding_left = v.max(0),
            (MARGIN_TOP_KEY, Value::Int(v)) => self.margin_top = v.max(0),
            (MARGIN_RIGHT_KEY, Value::Int(v)) => self.margin_right = v.max(0),
            (MARGIN_BOTTOM_KEY, Value::Int(v)) => self.margin_bottom = v.max(0),
            (MARGIN_LEFT_KEY, Value::Int(v)) => self.margin_left = v.max(0),
            (MARGIN_BACKGROUND_KEY, Value::Color(c)) => self.margin_bg_color = c,
            (BORDER_STYLE_KEY, Value::Border(b)) => self.border_style = b,
            (BORDER_TOP_FOREGROUND_KEY, Value::Color(c)) => self.border_top_fg_color = c,
            (BORDER_RIGHT_FOREGROUND_KEY, Value::Color(c)) => self.border_right_fg_color = c,
            (BORDER_BOTTOM_FOREGROUND_KEY, Value::Color(c)) => self.border_bottom_fg_color = c,
            (BORDER_LEFT_FOREGROUND_KEY, Value::Color(c)) => self.border_left_fg_color = c,
            (BORDER_TOP_BACKGROUND_KEY, Value::Color(c)) => self.border_top_bg_color = c,
            (BORDER_RIGHT_BACKGROUND_KEY, Value::Color(c)) => self.border_right_bg_color = c,
            (BORDER_BOTTOM_BACKGROUND_KEY, Value::Color(c)) => self.border_bottom_bg_color = c,
            (BORDER_LEFT_BACKGROUND_KEY, Value::Color(c)) => self.border_left_bg_color = c,
            (MAX_WIDTH_KEY, Value::Int(v)) => self.max_width = v.max(0),
            (MAX_HEIGHT_KEY, Value::Int(v)) => self.max_height = v.max(0),
            (TAB_WIDTH_KEY, Value::Int(v)) => self.tab_width = v,
            (TRANSFORM_KEY, Value::Transform(t)) => self.transform = t,
            (k, Value::Bool(v)) => {
                if v {
                    self.attrs |= k;
                } else {
                    self.attrs &= !k;
                }
            }
            (k, Value::Attrs(a)) => {
                if a & k != 0 {
                    self.attrs |= k;
                } else {
                    self.attrs &= !k;
                }
            }
            _ => {}
        }
        self.props |= key;
    }

    fn set_from(&mut self, key: PropKey, i: &Style) {
        let v = match key {
            FOREGROUND_KEY => Value::Color(i.fg_color.clone()),
            BACKGROUND_KEY => Value::Color(i.bg_color.clone()),
            WIDTH_KEY => Value::Int(i.width),
            HEIGHT_KEY => Value::Int(i.height),
            ALIGN_HORIZONTAL_KEY => Value::Pos(i.align_horizontal),
            ALIGN_VERTICAL_KEY => Value::Pos(i.align_vertical),
            PADDING_TOP_KEY => Value::Int(i.padding_top),
            PADDING_RIGHT_KEY => Value::Int(i.padding_right),
            PADDING_BOTTOM_KEY => Value::Int(i.padding_bottom),
            PADDING_LEFT_KEY => Value::Int(i.padding_left),
            MARGIN_TOP_KEY => Value::Int(i.margin_top),
            MARGIN_RIGHT_KEY => Value::Int(i.margin_right),
            MARGIN_BOTTOM_KEY => Value::Int(i.margin_bottom),
            MARGIN_LEFT_KEY => Value::Int(i.margin_left),
            MARGIN_BACKGROUND_KEY => Value::Color(i.margin_bg_color.clone()),
            BORDER_STYLE_KEY => Value::Border(i.border_style.clone()),
            BORDER_TOP_FOREGROUND_KEY => Value::Color(i.border_top_fg_color.clone()),
            BORDER_RIGHT_FOREGROUND_KEY => Value::Color(i.border_right_fg_color.clone()),
            BORDER_BOTTOM_FOREGROUND_KEY => Value::Color(i.border_bottom_fg_color.clone()),
            BORDER_LEFT_FOREGROUND_KEY => Value::Color(i.border_left_fg_color.clone()),
            BORDER_TOP_BACKGROUND_KEY => Value::Color(i.border_top_bg_color.clone()),
            BORDER_RIGHT_BACKGROUND_KEY => Value::Color(i.border_right_bg_color.clone()),
            BORDER_BOTTOM_BACKGROUND_KEY => Value::Color(i.border_bottom_bg_color.clone()),
            BORDER_LEFT_BACKGROUND_KEY => Value::Color(i.border_left_bg_color.clone()),
            MAX_WIDTH_KEY => Value::Int(i.max_width),
            MAX_HEIGHT_KEY => Value::Int(i.max_height),
            TAB_WIDTH_KEY => Value::Int(i.tab_width),
            TRANSFORM_KEY => Value::Transform(i.transform.clone()),
            _ => Value::Attrs(i.attrs),
        };
        self.set(key, v);
    }

    fn unset(&mut self, key: PropKey) {
        self.props &= !key;
    }

    fn get_as_bool(&self, k: PropKey, default: bool) -> bool {
        if !self.is_set(k) {
            return default;
        }
        self.attrs & k != 0
    }

    fn get_as_color(&self, k: PropKey) -> TerminalColor {
        if !self.is_set(k) {
            return TerminalColor::NoColor;
        }
        let c = match k {
            FOREGROUND_KEY => &self.fg_color,
            BACKGROUND_KEY => &self.bg_color,
            MARGIN_BACKGROUND_KEY => &self.margin_bg_color,
            BORDER_TOP_FOREGROUND_KEY => &self.border_top_fg_color,
            BORDER_RIGHT_FOREGROUND_KEY => &self.border_right_fg_color,
            BORDER_BOTTOM_FOREGROUND_KEY => &self.border_bottom_fg_color,
            BORDER_LEFT_FOREGROUND_KEY => &self.border_left_fg_color,
            BORDER_TOP_BACKGROUND_KEY => &self.border_top_bg_color,
            BORDER_RIGHT_BACKGROUND_KEY => &self.border_right_bg_color,
            BORDER_BOTTOM_BACKGROUND_KEY => &self.border_bottom_bg_color,
            BORDER_LEFT_BACKGROUND_KEY => &self.border_left_bg_color,
            _ => return TerminalColor::NoColor,
        };
        c.clone()
    }

    fn get_as_int(&self, k: PropKey) -> i32 {
        if !self.is_set(k) {
            return 0;
        }
        match k {
            WIDTH_KEY => self.width,
            HEIGHT_KEY => self.height,
            PADDING_TOP_KEY => self.padding_top,
            PADDING_RIGHT_KEY => self.padding_right,
            PADDING_BOTTOM_KEY => self.padding_bottom,
            PADDING_LEFT_KEY => self.padding_left,
            MARGIN_TOP_KEY => self.margin_top,
            MARGIN_RIGHT_KEY => self.margin_right,
            MARGIN_BOTTOM_KEY => self.margin_bottom,
            MARGIN_LEFT_KEY => self.margin_left,
            MAX_WIDTH_KEY => self.max_width,
            MAX_HEIGHT_KEY => self.max_height,
            TAB_WIDTH_KEY => self.tab_width,
            _ => 0,
        }
    }

    fn get_as_position(&self, k: PropKey) -> Position {
        if !self.is_set(k) {
            return Position(0.0);
        }
        match k {
            ALIGN_HORIZONTAL_KEY => self.align_horizontal,
            ALIGN_VERTICAL_KEY => self.align_vertical,
            _ => Position(0.0),
        }
    }

    pub(crate) fn get_border_style(&self) -> Border {
        if !self.is_set(BORDER_STYLE_KEY) {
            return no_border();
        }
        self.border_style.clone()
    }

    fn implicit_borders(&self) -> bool {
        let border_style = self.get_border_style();
        let any = self.is_set(BORDER_TOP_KEY)
            || self.is_set(BORDER_RIGHT_KEY)
            || self.is_set(BORDER_BOTTOM_KEY)
            || self.is_set(BORDER_LEFT_KEY);
        border_style != no_border() && !any
    }

    fn get_as_transform(&self) -> Option<TransformFn> {
        if !self.is_set(TRANSFORM_KEY) {
            return None;
        }
        self.transform.clone()
    }

    // ---- style.go ----

    /// `Style.SetString`: sets the underlying string value (joined by spaces).
    pub fn set_string(mut self, strs: &[&str]) -> Style {
        self.value = strs.join(" ");
        self
    }

    /// `Style.Value`.
    pub fn value(&self) -> &str {
        &self.value
    }

    /// `Style.String()`: renders the underlying string value.
    pub fn string(&self) -> String {
        self.render_strs(&[])
    }

    /// `Style.Copy`.
    pub fn copy(&self) -> Style {
        self.clone()
    }

    /// `Style.Inherit`: copies explicitly set values from `i` that are not set
    /// here (margins, padding and the string value are not inherited).
    pub fn inherit(mut self, i: &Style) -> Style {
        let mut k: PropKey = BOLD_KEY;
        while k <= TRANSFORM_KEY {
            if i.is_set(k) {
                let mut skip = false;
                match k {
                    MARGIN_TOP_KEY | MARGIN_RIGHT_KEY | MARGIN_BOTTOM_KEY | MARGIN_LEFT_KEY => {
                        skip = true
                    }
                    PADDING_TOP_KEY | PADDING_RIGHT_KEY | PADDING_BOTTOM_KEY | PADDING_LEFT_KEY => {
                        skip = true
                    }
                    BACKGROUND_KEY
                        if !self.is_set(MARGIN_BACKGROUND_KEY)
                            && !i.is_set(MARGIN_BACKGROUND_KEY) =>
                    {
                        // The margins also inherit the background colour.
                        self.set(MARGIN_BACKGROUND_KEY, Value::Color(i.bg_color.clone()));
                    }
                    _ => {}
                }
                if !skip && !self.is_set(k) {
                    self.set_from(k, i);
                }
            }
            k <<= 1;
        }
        self
    }

    /// `Style.Render(s)` for a single string.
    pub fn render(&self, s: &str) -> String {
        self.render_strs(&[s])
    }

    /// `Style.Render(strs...)`: the strings are joined with spaces (after the
    /// underlying value, if any).
    pub fn render_strs(&self, strs: &[&str]) -> String {
        let mut parts: Vec<&str> = Vec::with_capacity(strs.len() + 1);
        if !self.value.is_empty() {
            parts.push(&self.value);
        }
        parts.extend_from_slice(strs);
        let mut s = parts.join(" ");

        let p = renderer::color_profile();
        let mut te = p.string(&[]);
        let mut te_space = p.string(&[]);
        let mut te_whitespace = p.string(&[]);

        let bold = self.get_as_bool(BOLD_KEY, false);
        let italic = self.get_as_bool(ITALIC_KEY, false);
        let underline = self.get_as_bool(UNDERLINE_KEY, false);
        let strikethrough = self.get_as_bool(STRIKETHROUGH_KEY, false);
        let reverse = self.get_as_bool(REVERSE_KEY, false);
        let blink = self.get_as_bool(BLINK_KEY, false);
        let faint = self.get_as_bool(FAINT_KEY, false);

        let fg = self.get_as_color(FOREGROUND_KEY);
        let bg = self.get_as_color(BACKGROUND_KEY);

        let width = self.get_as_int(WIDTH_KEY);
        let height = self.get_as_int(HEIGHT_KEY);
        let horizontal_align = self.get_as_position(ALIGN_HORIZONTAL_KEY);
        let vertical_align = self.get_as_position(ALIGN_VERTICAL_KEY);

        let top_padding = self.get_as_int(PADDING_TOP_KEY);
        let right_padding = self.get_as_int(PADDING_RIGHT_KEY);
        let bottom_padding = self.get_as_int(PADDING_BOTTOM_KEY);
        let left_padding = self.get_as_int(PADDING_LEFT_KEY);

        let color_whitespace = self.get_as_bool(COLOR_WHITESPACE_KEY, true);
        let inline = self.get_as_bool(INLINE_KEY, false);
        let max_width = self.get_as_int(MAX_WIDTH_KEY);
        let max_height = self.get_as_int(MAX_HEIGHT_KEY);

        let underline_spaces = self.get_as_bool(UNDERLINE_SPACES_KEY, false)
            || (underline && self.get_as_bool(UNDERLINE_SPACES_KEY, true));
        let strikethrough_spaces = self.get_as_bool(STRIKETHROUGH_SPACES_KEY, false)
            || (strikethrough && self.get_as_bool(STRIKETHROUGH_SPACES_KEY, true));

        let style_whitespace = reverse;
        let use_space_styler = (underline && !underline_spaces)
            || (strikethrough && !strikethrough_spaces)
            || underline_spaces
            || strikethrough_spaces;

        if let Some(t) = self.get_as_transform() {
            s = t(&s);
        }

        if self.props == 0 {
            return self.maybe_convert_tabs(&s);
        }

        if bold {
            te = te.bold();
        }
        if italic {
            te = te.italic();
        }
        if underline {
            te = te.underline();
        }
        if reverse {
            te_whitespace = te_whitespace.reverse();
            te = te.reverse();
        }
        if blink {
            te = te.blink();
        }
        if faint {
            te = te.faint();
        }

        if fg != TerminalColor::NoColor {
            te = te.foreground(fg.color());
            if style_whitespace {
                te_whitespace = te_whitespace.foreground(fg.color());
            }
            if use_space_styler {
                te_space = te_space.foreground(fg.color());
            }
        }

        if bg != TerminalColor::NoColor {
            te = te.background(bg.color());
            if color_whitespace {
                te_whitespace = te_whitespace.background(bg.color());
            }
            if use_space_styler {
                te_space = te_space.background(bg.color());
            }
        }

        if underline {
            te = te.underline();
        }
        if strikethrough {
            te = te.cross_out();
        }

        if underline_spaces {
            te_space = te_space.underline();
        }
        if strikethrough_spaces {
            te_space = te_space.cross_out();
        }

        s = self.maybe_convert_tabs(&s);
        s = s.replace("\r\n", "\n");

        if inline {
            s = s.replace('\n', "");
        }

        if !inline && width > 0 {
            let wrap_at = width - left_padding - right_padding;
            s = crate::cellbuf::wrap(&s, wrap_at, "");
        }

        // Render core text
        {
            let mut b = String::with_capacity(s.len() + 16);
            let lines: Vec<&str> = s.split('\n').collect();
            let n = lines.len();
            for (i, l) in lines.iter().enumerate() {
                if use_space_styler {
                    for r in l.chars() {
                        let mut buf = [0u8; 4];
                        let rs = r.encode_utf8(&mut buf);
                        if is_go_space(r) {
                            b.push_str(&te_space.styled(rs));
                            continue;
                        }
                        b.push_str(&te.styled(rs));
                    }
                } else {
                    b.push_str(&te.styled(l));
                }
                if i != n - 1 {
                    b.push('\n');
                }
            }
            s = b;
        }

        let ws_style = if color_whitespace || style_whitespace {
            Some(&te_whitespace)
        } else {
            None
        };

        // Padding
        if !inline {
            if left_padding > 0 {
                s = pad_left(&s, left_padding, ws_style);
            }
            if right_padding > 0 {
                s = pad_right(&s, right_padding, ws_style);
            }
            if top_padding > 0 {
                s = "\n".repeat(top_padding as usize) + &s;
            }
            if bottom_padding > 0 {
                s += &"\n".repeat(bottom_padding as usize);
            }
        }

        // Height
        if height > 0 {
            s = align_text_vertical(&s, vertical_align, height);
        }

        // Horizontal alignment / padding of short lines
        {
            let num_lines = s.matches('\n').count();
            if num_lines != 0 || width != 0 {
                s = align_text_horizontal(&s, horizontal_align, width, ws_style);
            }
        }

        if !inline {
            s = self.apply_border(&s);
            s = self.apply_margins(&s, inline);
        }

        if max_width > 0 {
            let lines: Vec<String> = s
                .split('\n')
                .map(|l| crate::ansi::truncate(l, max_width, ""))
                .collect();
            s = lines.join("\n");
        }

        if max_height > 0 {
            let lines: Vec<&str> = s.split('\n').collect();
            let h = (max_height as usize).min(lines.len());
            if !lines.is_empty() {
                s = lines[..h].join("\n");
            }
        }

        s
    }

    fn maybe_convert_tabs(&self, s: &str) -> String {
        let mut tw = TAB_WIDTH_DEFAULT;
        if self.is_set(TAB_WIDTH_KEY) {
            tw = self.get_as_int(TAB_WIDTH_KEY);
        }
        match tw {
            -1 => s.to_string(),
            0 => s.replace('\t', ""),
            _ => s.replace('\t', &" ".repeat(tw.max(0) as usize)),
        }
    }

    fn apply_margins(&self, s: &str, inline: bool) -> String {
        let top_margin = self.get_as_int(MARGIN_TOP_KEY);
        let right_margin = self.get_as_int(MARGIN_RIGHT_KEY);
        let bottom_margin = self.get_as_int(MARGIN_BOTTOM_KEY);
        let left_margin = self.get_as_int(MARGIN_LEFT_KEY);

        let mut styler = termenv::Style::default();
        let bgc = self.get_as_color(MARGIN_BACKGROUND_KEY);
        if bgc != TerminalColor::NoColor {
            styler = styler.background(bgc.color());
        }

        let mut s = pad_left(s, left_margin, Some(&styler));
        s = pad_right(&s, right_margin, Some(&styler));

        if !inline {
            let (_, width) = get_lines(&s);
            let spaces = " ".repeat(width.max(0) as usize);
            if top_margin > 0 {
                s = styler.styled(&format!("{spaces}\n").repeat(top_margin as usize)) + &s;
            }
            if bottom_margin > 0 {
                s += &styler.styled(&format!("\n{spaces}").repeat(bottom_margin as usize));
            }
        }
        s
    }

    // ---- set.go ----

    pub fn bold(mut self, v: bool) -> Style {
        self.set(BOLD_KEY, Value::Bool(v));
        self
    }
    pub fn italic(mut self, v: bool) -> Style {
        self.set(ITALIC_KEY, Value::Bool(v));
        self
    }
    pub fn underline(mut self, v: bool) -> Style {
        self.set(UNDERLINE_KEY, Value::Bool(v));
        self
    }
    pub fn strikethrough(mut self, v: bool) -> Style {
        self.set(STRIKETHROUGH_KEY, Value::Bool(v));
        self
    }
    pub fn reverse(mut self, v: bool) -> Style {
        self.set(REVERSE_KEY, Value::Bool(v));
        self
    }
    pub fn blink(mut self, v: bool) -> Style {
        self.set(BLINK_KEY, Value::Bool(v));
        self
    }
    pub fn faint(mut self, v: bool) -> Style {
        self.set(FAINT_KEY, Value::Bool(v));
        self
    }
    pub fn foreground(mut self, c: impl Into<TerminalColor>) -> Style {
        self.set(FOREGROUND_KEY, Value::Color(c.into()));
        self
    }
    pub fn background(mut self, c: impl Into<TerminalColor>) -> Style {
        self.set(BACKGROUND_KEY, Value::Color(c.into()));
        self
    }
    pub fn width(mut self, i: i32) -> Style {
        self.set(WIDTH_KEY, Value::Int(i));
        self
    }
    pub fn height(mut self, i: i32) -> Style {
        self.set(HEIGHT_KEY, Value::Int(i));
        self
    }
    /// `Style.Align(p...)`: horizontal, then vertical.
    pub fn align(mut self, p: &[Position]) -> Style {
        if !p.is_empty() {
            self.set(ALIGN_HORIZONTAL_KEY, Value::Pos(p[0]));
        }
        if p.len() > 1 {
            self.set(ALIGN_VERTICAL_KEY, Value::Pos(p[1]));
        }
        self
    }
    pub fn align_horizontal(mut self, p: Position) -> Style {
        self.set(ALIGN_HORIZONTAL_KEY, Value::Pos(p));
        self
    }
    pub fn align_vertical(mut self, p: Position) -> Style {
        self.set(ALIGN_VERTICAL_KEY, Value::Pos(p));
        self
    }
    /// `Style.Padding(i...)` with CSS-like 1-4 values.
    pub fn padding(mut self, i: &[i32]) -> Style {
        let Some((top, right, bottom, left)) = which_sides(i) else {
            return self;
        };
        self.set(PADDING_TOP_KEY, Value::Int(top));
        self.set(PADDING_RIGHT_KEY, Value::Int(right));
        self.set(PADDING_BOTTOM_KEY, Value::Int(bottom));
        self.set(PADDING_LEFT_KEY, Value::Int(left));
        self
    }
    pub fn padding_left(mut self, i: i32) -> Style {
        self.set(PADDING_LEFT_KEY, Value::Int(i));
        self
    }
    pub fn padding_right(mut self, i: i32) -> Style {
        self.set(PADDING_RIGHT_KEY, Value::Int(i));
        self
    }
    pub fn padding_top(mut self, i: i32) -> Style {
        self.set(PADDING_TOP_KEY, Value::Int(i));
        self
    }
    pub fn padding_bottom(mut self, i: i32) -> Style {
        self.set(PADDING_BOTTOM_KEY, Value::Int(i));
        self
    }
    pub fn color_whitespace(mut self, v: bool) -> Style {
        self.set(COLOR_WHITESPACE_KEY, Value::Bool(v));
        self
    }
    /// `Style.Margin(i...)` with CSS-like 1-4 values.
    pub fn margin(mut self, i: &[i32]) -> Style {
        let Some((top, right, bottom, left)) = which_sides(i) else {
            return self;
        };
        self.set(MARGIN_TOP_KEY, Value::Int(top));
        self.set(MARGIN_RIGHT_KEY, Value::Int(right));
        self.set(MARGIN_BOTTOM_KEY, Value::Int(bottom));
        self.set(MARGIN_LEFT_KEY, Value::Int(left));
        self
    }
    pub fn margin_left(mut self, i: i32) -> Style {
        self.set(MARGIN_LEFT_KEY, Value::Int(i));
        self
    }
    pub fn margin_right(mut self, i: i32) -> Style {
        self.set(MARGIN_RIGHT_KEY, Value::Int(i));
        self
    }
    pub fn margin_top(mut self, i: i32) -> Style {
        self.set(MARGIN_TOP_KEY, Value::Int(i));
        self
    }
    pub fn margin_bottom(mut self, i: i32) -> Style {
        self.set(MARGIN_BOTTOM_KEY, Value::Int(i));
        self
    }
    pub fn margin_background(mut self, c: impl Into<TerminalColor>) -> Style {
        self.set(MARGIN_BACKGROUND_KEY, Value::Color(c.into()));
        self
    }
    /// `Style.Border(b, sides...)`: no sides means all four.
    pub fn border(mut self, b: Border, sides: &[bool]) -> Style {
        self.set(BORDER_STYLE_KEY, Value::Border(b));
        let (top, right, bottom, left) = which_sides(sides).unwrap_or((true, true, true, true));
        self.set(BORDER_TOP_KEY, Value::Bool(top));
        self.set(BORDER_RIGHT_KEY, Value::Bool(right));
        self.set(BORDER_BOTTOM_KEY, Value::Bool(bottom));
        self.set(BORDER_LEFT_KEY, Value::Bool(left));
        self
    }
    pub fn border_style(mut self, b: Border) -> Style {
        self.set(BORDER_STYLE_KEY, Value::Border(b));
        self
    }
    pub fn border_top(mut self, v: bool) -> Style {
        self.set(BORDER_TOP_KEY, Value::Bool(v));
        self
    }
    pub fn border_right(mut self, v: bool) -> Style {
        self.set(BORDER_RIGHT_KEY, Value::Bool(v));
        self
    }
    pub fn border_bottom(mut self, v: bool) -> Style {
        self.set(BORDER_BOTTOM_KEY, Value::Bool(v));
        self
    }
    pub fn border_left(mut self, v: bool) -> Style {
        self.set(BORDER_LEFT_KEY, Value::Bool(v));
        self
    }
    /// `Style.BorderForeground(c...)` with CSS-like 1-4 colours.
    pub fn border_foreground(mut self, c: &[TerminalColor]) -> Style {
        let Some((top, right, bottom, left)) = which_sides(c) else {
            return self;
        };
        self.set(BORDER_TOP_FOREGROUND_KEY, Value::Color(top));
        self.set(BORDER_RIGHT_FOREGROUND_KEY, Value::Color(right));
        self.set(BORDER_BOTTOM_FOREGROUND_KEY, Value::Color(bottom));
        self.set(BORDER_LEFT_FOREGROUND_KEY, Value::Color(left));
        self
    }
    pub fn border_top_foreground(mut self, c: impl Into<TerminalColor>) -> Style {
        self.set(BORDER_TOP_FOREGROUND_KEY, Value::Color(c.into()));
        self
    }
    pub fn border_right_foreground(mut self, c: impl Into<TerminalColor>) -> Style {
        self.set(BORDER_RIGHT_FOREGROUND_KEY, Value::Color(c.into()));
        self
    }
    pub fn border_bottom_foreground(mut self, c: impl Into<TerminalColor>) -> Style {
        self.set(BORDER_BOTTOM_FOREGROUND_KEY, Value::Color(c.into()));
        self
    }
    pub fn border_left_foreground(mut self, c: impl Into<TerminalColor>) -> Style {
        self.set(BORDER_LEFT_FOREGROUND_KEY, Value::Color(c.into()));
        self
    }
    /// `Style.BorderBackground(c...)` with CSS-like 1-4 colours.
    pub fn border_background(mut self, c: &[TerminalColor]) -> Style {
        let Some((top, right, bottom, left)) = which_sides(c) else {
            return self;
        };
        self.set(BORDER_TOP_BACKGROUND_KEY, Value::Color(top));
        self.set(BORDER_RIGHT_BACKGROUND_KEY, Value::Color(right));
        self.set(BORDER_BOTTOM_BACKGROUND_KEY, Value::Color(bottom));
        self.set(BORDER_LEFT_BACKGROUND_KEY, Value::Color(left));
        self
    }
    pub fn border_top_background(mut self, c: impl Into<TerminalColor>) -> Style {
        self.set(BORDER_TOP_BACKGROUND_KEY, Value::Color(c.into()));
        self
    }
    pub fn border_right_background(mut self, c: impl Into<TerminalColor>) -> Style {
        self.set(BORDER_RIGHT_BACKGROUND_KEY, Value::Color(c.into()));
        self
    }
    pub fn border_bottom_background(mut self, c: impl Into<TerminalColor>) -> Style {
        self.set(BORDER_BOTTOM_BACKGROUND_KEY, Value::Color(c.into()));
        self
    }
    pub fn border_left_background(mut self, c: impl Into<TerminalColor>) -> Style {
        self.set(BORDER_LEFT_BACKGROUND_KEY, Value::Color(c.into()));
        self
    }
    pub fn inline(mut self, v: bool) -> Style {
        self.set(INLINE_KEY, Value::Bool(v));
        self
    }
    pub fn max_width(mut self, n: i32) -> Style {
        self.set(MAX_WIDTH_KEY, Value::Int(n));
        self
    }
    pub fn max_height(mut self, n: i32) -> Style {
        self.set(MAX_HEIGHT_KEY, Value::Int(n));
        self
    }
    pub fn tab_width(mut self, n: i32) -> Style {
        let n = if n <= -1 { -1 } else { n };
        self.set(TAB_WIDTH_KEY, Value::Int(n));
        self
    }
    pub fn underline_spaces(mut self, v: bool) -> Style {
        self.set(UNDERLINE_SPACES_KEY, Value::Bool(v));
        self
    }
    pub fn strikethrough_spaces(mut self, v: bool) -> Style {
        self.set(STRIKETHROUGH_SPACES_KEY, Value::Bool(v));
        self
    }
    pub fn transform(mut self, f: impl Fn(&str) -> String + 'static) -> Style {
        self.set(TRANSFORM_KEY, Value::Transform(Some(Rc::new(f))));
        self
    }

    // ---- get.go ----

    pub fn get_bold(&self) -> bool {
        self.get_as_bool(BOLD_KEY, false)
    }
    pub fn get_italic(&self) -> bool {
        self.get_as_bool(ITALIC_KEY, false)
    }
    pub fn get_underline(&self) -> bool {
        self.get_as_bool(UNDERLINE_KEY, false)
    }
    pub fn get_strikethrough(&self) -> bool {
        self.get_as_bool(STRIKETHROUGH_KEY, false)
    }
    pub fn get_reverse(&self) -> bool {
        self.get_as_bool(REVERSE_KEY, false)
    }
    pub fn get_blink(&self) -> bool {
        self.get_as_bool(BLINK_KEY, false)
    }
    pub fn get_faint(&self) -> bool {
        self.get_as_bool(FAINT_KEY, false)
    }
    pub fn get_foreground(&self) -> TerminalColor {
        self.get_as_color(FOREGROUND_KEY)
    }
    pub fn get_background(&self) -> TerminalColor {
        self.get_as_color(BACKGROUND_KEY)
    }
    pub fn get_width(&self) -> i32 {
        self.get_as_int(WIDTH_KEY)
    }
    pub fn get_height(&self) -> i32 {
        self.get_as_int(HEIGHT_KEY)
    }
    pub fn get_align(&self) -> Position {
        self.get_align_horizontal()
    }
    pub fn get_align_horizontal(&self) -> Position {
        let v = self.get_as_position(ALIGN_HORIZONTAL_KEY);
        if v == Position(0.0) {
            return super::position::LEFT;
        }
        v
    }
    pub fn get_align_vertical(&self) -> Position {
        let v = self.get_as_position(ALIGN_VERTICAL_KEY);
        if v == Position(0.0) {
            return super::position::TOP;
        }
        v
    }
    pub fn get_padding(&self) -> (i32, i32, i32, i32) {
        (
            self.get_as_int(PADDING_TOP_KEY),
            self.get_as_int(PADDING_RIGHT_KEY),
            self.get_as_int(PADDING_BOTTOM_KEY),
            self.get_as_int(PADDING_LEFT_KEY),
        )
    }
    pub fn get_padding_top(&self) -> i32 {
        self.get_as_int(PADDING_TOP_KEY)
    }
    pub fn get_padding_right(&self) -> i32 {
        self.get_as_int(PADDING_RIGHT_KEY)
    }
    pub fn get_padding_bottom(&self) -> i32 {
        self.get_as_int(PADDING_BOTTOM_KEY)
    }
    pub fn get_padding_left(&self) -> i32 {
        self.get_as_int(PADDING_LEFT_KEY)
    }
    pub fn get_horizontal_padding(&self) -> i32 {
        self.get_as_int(PADDING_LEFT_KEY) + self.get_as_int(PADDING_RIGHT_KEY)
    }
    pub fn get_vertical_padding(&self) -> i32 {
        self.get_as_int(PADDING_TOP_KEY) + self.get_as_int(PADDING_BOTTOM_KEY)
    }
    pub fn get_color_whitespace(&self) -> bool {
        self.get_as_bool(COLOR_WHITESPACE_KEY, false)
    }
    pub fn get_margin(&self) -> (i32, i32, i32, i32) {
        (
            self.get_as_int(MARGIN_TOP_KEY),
            self.get_as_int(MARGIN_RIGHT_KEY),
            self.get_as_int(MARGIN_BOTTOM_KEY),
            self.get_as_int(MARGIN_LEFT_KEY),
        )
    }
    pub fn get_margin_top(&self) -> i32 {
        self.get_as_int(MARGIN_TOP_KEY)
    }
    pub fn get_margin_right(&self) -> i32 {
        self.get_as_int(MARGIN_RIGHT_KEY)
    }
    pub fn get_margin_bottom(&self) -> i32 {
        self.get_as_int(MARGIN_BOTTOM_KEY)
    }
    pub fn get_margin_left(&self) -> i32 {
        self.get_as_int(MARGIN_LEFT_KEY)
    }
    pub fn get_horizontal_margins(&self) -> i32 {
        self.get_as_int(MARGIN_LEFT_KEY) + self.get_as_int(MARGIN_RIGHT_KEY)
    }
    pub fn get_vertical_margins(&self) -> i32 {
        self.get_as_int(MARGIN_TOP_KEY) + self.get_as_int(MARGIN_BOTTOM_KEY)
    }
    pub fn get_border(&self) -> (Border, bool, bool, bool, bool) {
        (
            self.get_border_style(),
            self.get_as_bool(BORDER_TOP_KEY, false),
            self.get_as_bool(BORDER_RIGHT_KEY, false),
            self.get_as_bool(BORDER_BOTTOM_KEY, false),
            self.get_as_bool(BORDER_LEFT_KEY, false),
        )
    }
    /// `Style.GetBorderStyle`.
    pub fn get_border_style_value(&self) -> Border {
        self.get_border_style()
    }
    pub fn get_border_top(&self) -> bool {
        self.get_as_bool(BORDER_TOP_KEY, false)
    }
    pub fn get_border_right(&self) -> bool {
        self.get_as_bool(BORDER_RIGHT_KEY, false)
    }
    pub fn get_border_bottom(&self) -> bool {
        self.get_as_bool(BORDER_BOTTOM_KEY, false)
    }
    pub fn get_border_left(&self) -> bool {
        self.get_as_bool(BORDER_LEFT_KEY, false)
    }
    pub fn get_border_top_foreground(&self) -> TerminalColor {
        self.get_as_color(BORDER_TOP_FOREGROUND_KEY)
    }
    pub fn get_border_right_foreground(&self) -> TerminalColor {
        self.get_as_color(BORDER_RIGHT_FOREGROUND_KEY)
    }
    pub fn get_border_bottom_foreground(&self) -> TerminalColor {
        self.get_as_color(BORDER_BOTTOM_FOREGROUND_KEY)
    }
    pub fn get_border_left_foreground(&self) -> TerminalColor {
        self.get_as_color(BORDER_LEFT_FOREGROUND_KEY)
    }
    pub fn get_border_top_background(&self) -> TerminalColor {
        self.get_as_color(BORDER_TOP_BACKGROUND_KEY)
    }
    pub fn get_border_right_background(&self) -> TerminalColor {
        self.get_as_color(BORDER_RIGHT_BACKGROUND_KEY)
    }
    pub fn get_border_bottom_background(&self) -> TerminalColor {
        self.get_as_color(BORDER_BOTTOM_BACKGROUND_KEY)
    }
    pub fn get_border_left_background(&self) -> TerminalColor {
        self.get_as_color(BORDER_LEFT_BACKGROUND_KEY)
    }
    pub fn get_border_top_width(&self) -> i32 {
        self.get_border_top_size()
    }
    pub fn get_border_top_size(&self) -> i32 {
        if !self.get_as_bool(BORDER_TOP_KEY, false) && !self.implicit_borders() {
            return 0;
        }
        self.get_border_style().get_top_size()
    }
    pub fn get_border_left_size(&self) -> i32 {
        if !self.get_as_bool(BORDER_LEFT_KEY, false) && !self.implicit_borders() {
            return 0;
        }
        self.get_border_style().get_left_size()
    }
    pub fn get_border_bottom_size(&self) -> i32 {
        if !self.get_as_bool(BORDER_BOTTOM_KEY, false) && !self.implicit_borders() {
            return 0;
        }
        self.get_border_style().get_bottom_size()
    }
    pub fn get_border_right_size(&self) -> i32 {
        if !self.get_as_bool(BORDER_RIGHT_KEY, false) && !self.implicit_borders() {
            return 0;
        }
        self.get_border_style().get_right_size()
    }
    pub fn get_horizontal_border_size(&self) -> i32 {
        self.get_border_left_size() + self.get_border_right_size()
    }
    pub fn get_vertical_border_size(&self) -> i32 {
        self.get_border_top_size() + self.get_border_bottom_size()
    }
    pub fn get_inline(&self) -> bool {
        self.get_as_bool(INLINE_KEY, false)
    }
    pub fn get_max_width(&self) -> i32 {
        self.get_as_int(MAX_WIDTH_KEY)
    }
    pub fn get_max_height(&self) -> i32 {
        self.get_as_int(MAX_HEIGHT_KEY)
    }
    pub fn get_tab_width(&self) -> i32 {
        self.get_as_int(TAB_WIDTH_KEY)
    }
    pub fn get_underline_spaces(&self) -> bool {
        self.get_as_bool(UNDERLINE_SPACES_KEY, false)
    }
    pub fn get_strikethrough_spaces(&self) -> bool {
        self.get_as_bool(STRIKETHROUGH_SPACES_KEY, false)
    }
    pub fn get_horizontal_frame_size(&self) -> i32 {
        self.get_horizontal_margins()
            + self.get_horizontal_padding()
            + self.get_horizontal_border_size()
    }
    pub fn get_vertical_frame_size(&self) -> i32 {
        self.get_vertical_margins() + self.get_vertical_padding() + self.get_vertical_border_size()
    }
    pub fn get_frame_size(&self) -> (i32, i32) {
        (
            self.get_horizontal_frame_size(),
            self.get_vertical_frame_size(),
        )
    }
    pub fn get_transform(&self) -> Option<TransformFn> {
        self.get_as_transform()
    }

    // ---- unset.go ----

    pub fn unset_bold(mut self) -> Style {
        self.unset(BOLD_KEY);
        self
    }
    pub fn unset_italic(mut self) -> Style {
        self.unset(ITALIC_KEY);
        self
    }
    pub fn unset_underline(mut self) -> Style {
        self.unset(UNDERLINE_KEY);
        self
    }
    pub fn unset_strikethrough(mut self) -> Style {
        self.unset(STRIKETHROUGH_KEY);
        self
    }
    pub fn unset_reverse(mut self) -> Style {
        self.unset(REVERSE_KEY);
        self
    }
    pub fn unset_blink(mut self) -> Style {
        self.unset(BLINK_KEY);
        self
    }
    pub fn unset_faint(mut self) -> Style {
        self.unset(FAINT_KEY);
        self
    }
    pub fn unset_foreground(mut self) -> Style {
        self.unset(FOREGROUND_KEY);
        self
    }
    pub fn unset_background(mut self) -> Style {
        self.unset(BACKGROUND_KEY);
        self
    }
    pub fn unset_width(mut self) -> Style {
        self.unset(WIDTH_KEY);
        self
    }
    pub fn unset_height(mut self) -> Style {
        self.unset(HEIGHT_KEY);
        self
    }
    pub fn unset_align(mut self) -> Style {
        self.unset(ALIGN_HORIZONTAL_KEY);
        self.unset(ALIGN_VERTICAL_KEY);
        self
    }
    pub fn unset_align_horizontal(mut self) -> Style {
        self.unset(ALIGN_HORIZONTAL_KEY);
        self
    }
    pub fn unset_align_vertical(mut self) -> Style {
        self.unset(ALIGN_VERTICAL_KEY);
        self
    }
    pub fn unset_padding(mut self) -> Style {
        self.unset(PADDING_LEFT_KEY);
        self.unset(PADDING_RIGHT_KEY);
        self.unset(PADDING_TOP_KEY);
        self.unset(PADDING_BOTTOM_KEY);
        self
    }
    pub fn unset_padding_left(mut self) -> Style {
        self.unset(PADDING_LEFT_KEY);
        self
    }
    pub fn unset_padding_right(mut self) -> Style {
        self.unset(PADDING_RIGHT_KEY);
        self
    }
    pub fn unset_padding_top(mut self) -> Style {
        self.unset(PADDING_TOP_KEY);
        self
    }
    pub fn unset_padding_bottom(mut self) -> Style {
        self.unset(PADDING_BOTTOM_KEY);
        self
    }
    pub fn unset_color_whitespace(mut self) -> Style {
        self.unset(COLOR_WHITESPACE_KEY);
        self
    }
    pub fn unset_margins(mut self) -> Style {
        self.unset(MARGIN_LEFT_KEY);
        self.unset(MARGIN_RIGHT_KEY);
        self.unset(MARGIN_TOP_KEY);
        self.unset(MARGIN_BOTTOM_KEY);
        self
    }
    pub fn unset_margin_left(mut self) -> Style {
        self.unset(MARGIN_LEFT_KEY);
        self
    }
    pub fn unset_margin_right(mut self) -> Style {
        self.unset(MARGIN_RIGHT_KEY);
        self
    }
    pub fn unset_margin_top(mut self) -> Style {
        self.unset(MARGIN_TOP_KEY);
        self
    }
    pub fn unset_margin_bottom(mut self) -> Style {
        self.unset(MARGIN_BOTTOM_KEY);
        self
    }
    pub fn unset_margin_background(mut self) -> Style {
        self.unset(MARGIN_BACKGROUND_KEY);
        self
    }
    pub fn unset_border_style(mut self) -> Style {
        self.unset(BORDER_STYLE_KEY);
        self
    }
    pub fn unset_border_top(mut self) -> Style {
        self.unset(BORDER_TOP_KEY);
        self
    }
    pub fn unset_border_right(mut self) -> Style {
        self.unset(BORDER_RIGHT_KEY);
        self
    }
    pub fn unset_border_bottom(mut self) -> Style {
        self.unset(BORDER_BOTTOM_KEY);
        self
    }
    pub fn unset_border_left(mut self) -> Style {
        self.unset(BORDER_LEFT_KEY);
        self
    }
    pub fn unset_border_foreground(mut self) -> Style {
        self.unset(BORDER_TOP_FOREGROUND_KEY);
        self.unset(BORDER_RIGHT_FOREGROUND_KEY);
        self.unset(BORDER_BOTTOM_FOREGROUND_KEY);
        self.unset(BORDER_LEFT_FOREGROUND_KEY);
        self
    }
    pub fn unset_border_background(mut self) -> Style {
        self.unset(BORDER_TOP_BACKGROUND_KEY);
        self.unset(BORDER_RIGHT_BACKGROUND_KEY);
        self.unset(BORDER_BOTTOM_BACKGROUND_KEY);
        self.unset(BORDER_LEFT_BACKGROUND_KEY);
        self
    }
    pub fn unset_inline(mut self) -> Style {
        self.unset(INLINE_KEY);
        self
    }
    pub fn unset_max_width(mut self) -> Style {
        self.unset(MAX_WIDTH_KEY);
        self
    }
    pub fn unset_max_height(mut self) -> Style {
        self.unset(MAX_HEIGHT_KEY);
        self
    }
    pub fn unset_tab_width(mut self) -> Style {
        self.unset(TAB_WIDTH_KEY);
        self
    }
    pub fn unset_underline_spaces(mut self) -> Style {
        self.unset(UNDERLINE_SPACES_KEY);
        self
    }
    pub fn unset_strikethrough_spaces(mut self) -> Style {
        self.unset(STRIKETHROUGH_SPACES_KEY);
        self
    }
    pub fn unset_transform(mut self) -> Style {
        self.unset(TRANSFORM_KEY);
        self
    }
    pub fn unset_string(mut self) -> Style {
        self.value.clear();
        self
    }

    // Border accessors used by borders.rs.
    pub(crate) fn border_flags(&self) -> (bool, bool, bool, bool, bool) {
        (
            self.get_as_bool(BORDER_TOP_KEY, false),
            self.get_as_bool(BORDER_RIGHT_KEY, false),
            self.get_as_bool(BORDER_BOTTOM_KEY, false),
            self.get_as_bool(BORDER_LEFT_KEY, false),
            self.implicit_borders(),
        )
    }
    pub(crate) fn border_colors(&self) -> [TerminalColor; 8] {
        [
            self.get_as_color(BORDER_TOP_FOREGROUND_KEY),
            self.get_as_color(BORDER_RIGHT_FOREGROUND_KEY),
            self.get_as_color(BORDER_BOTTOM_FOREGROUND_KEY),
            self.get_as_color(BORDER_LEFT_FOREGROUND_KEY),
            self.get_as_color(BORDER_TOP_BACKGROUND_KEY),
            self.get_as_color(BORDER_RIGHT_BACKGROUND_KEY),
            self.get_as_color(BORDER_BOTTOM_BACKGROUND_KEY),
            self.get_as_color(BORDER_LEFT_BACKGROUND_KEY),
        ]
    }
}

impl fmt::Display for Style {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.string())
    }
}

pub(crate) use crate::unicode::go_is_space as is_go_space;

/// `whichSidesInt` / `whichSidesBool` / `whichSidesColor`.
fn which_sides<T: Clone>(i: &[T]) -> Option<(T, T, T, T)> {
    match i.len() {
        1 => Some((i[0].clone(), i[0].clone(), i[0].clone(), i[0].clone())),
        2 => Some((i[0].clone(), i[1].clone(), i[0].clone(), i[1].clone())),
        3 => Some((i[0].clone(), i[1].clone(), i[2].clone(), i[1].clone())),
        4 => Some((i[0].clone(), i[1].clone(), i[2].clone(), i[3].clone())),
        _ => None,
    }
}

/// `padLeft`.
pub(crate) fn pad_left(s: &str, n: i32, style: Option<&termenv::Style>) -> String {
    pad(s, -n, style)
}

/// `padRight`.
pub(crate) fn pad_right(s: &str, n: i32, style: Option<&termenv::Style>) -> String {
    pad(s, n, style)
}

fn pad(s: &str, n: i32, style: Option<&termenv::Style>) -> String {
    if n == 0 {
        return s.to_string();
    }
    let mut sp = " ".repeat(n.unsigned_abs() as usize);
    if let Some(st) = style {
        sp = st.styled(&sp);
    }
    let mut b = String::with_capacity(s.len() + sp.len() * 4);
    let lines: Vec<&str> = s.split('\n').collect();
    let len = lines.len();
    for (i, l) in lines.iter().enumerate() {
        if n > 0 {
            b.push_str(l);
            b.push_str(&sp);
        } else {
            b.push_str(&sp);
            b.push_str(l);
        }
        if i != len - 1 {
            b.push('\n');
        }
    }
    b
}
