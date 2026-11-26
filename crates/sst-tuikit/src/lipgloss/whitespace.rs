//! `lipgloss` whitespace.go.

use super::color::TerminalColor;
use super::renderer;
use crate::ansi;
use crate::termenv;

/// Whitespace rendering options for `Place*` (`lipgloss.WhitespaceOption`).
#[derive(Clone, Debug)]
pub enum WhitespaceOption {
    /// `WithWhitespaceForeground`.
    Foreground(TerminalColor),
    /// `WithWhitespaceBackground`.
    Background(TerminalColor),
    /// `WithWhitespaceChars`.
    Chars(String),
}

/// `lipgloss.WithWhitespaceForeground`.
pub fn with_whitespace_foreground(c: impl Into<TerminalColor>) -> WhitespaceOption {
    WhitespaceOption::Foreground(c.into())
}
/// `lipgloss.WithWhitespaceBackground`.
pub fn with_whitespace_background(c: impl Into<TerminalColor>) -> WhitespaceOption {
    WhitespaceOption::Background(c.into())
}
/// `lipgloss.WithWhitespaceChars`.
pub fn with_whitespace_chars(s: &str) -> WhitespaceOption {
    WhitespaceOption::Chars(s.to_string())
}

pub(crate) struct Whitespace {
    style: termenv::Style,
    chars: String,
}

pub(crate) fn new_whitespace(opts: &[WhitespaceOption]) -> Whitespace {
    let mut w = Whitespace {
        style: renderer::color_profile().string(&[]),
        chars: String::new(),
    };
    for o in opts {
        match o {
            WhitespaceOption::Foreground(c) => w.style = w.style.clone().foreground(c.color()),
            WhitespaceOption::Background(c) => w.style = w.style.clone().background(c.color()),
            WhitespaceOption::Chars(s) => w.chars = s.clone(),
        }
    }
    w
}

impl Whitespace {
    pub(crate) fn render(&self, width: i32) -> String {
        let chars = if self.chars.is_empty() {
            " "
        } else {
            self.chars.as_str()
        };
        let r: Vec<char> = chars.chars().collect();
        let mut j = 0;
        let mut b = String::new();
        let mut i = 0;
        while i < width {
            b.push(r[j]);
            j += 1;
            if j >= r.len() {
                j = 0;
            }
            i += ansi::string_width(&r[j].to_string());
        }
        let short = width - ansi::string_width(&b);
        if short > 0 {
            b.push_str(&" ".repeat(short as usize));
        }
        self.style.styled(&b)
    }
}
