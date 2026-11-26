//! `lipgloss` color.go.

use super::renderer;
use crate::termenv::{self, Profile};

/// A colour usable in styles (`lipgloss.TerminalColor` implementations).
///
/// Go `lipgloss.Color("#FF0000")` -> [`color`]`("#FF0000")` (or
/// `TerminalColor::Color("#FF0000".into())`), `lipgloss.NoColor{}` ->
/// `TerminalColor::NoColor`. Go's `nil` colour behaves exactly like
/// `NoColor` in every lipgloss code path, so it has no separate variant.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum TerminalColor {
    /// `lipgloss.NoColor{}`.
    #[default]
    NoColor,
    /// `lipgloss.Color` (hex "#rrggbb"/"#rgb" or an ANSI index string).
    Color(String),
    /// `lipgloss.ANSIColor`.
    ANSIColor(u32),
    /// `lipgloss.AdaptiveColor`.
    AdaptiveColor { light: String, dark: String },
    /// `lipgloss.CompleteColor`.
    CompleteColor(CompleteColor),
    /// `lipgloss.CompleteAdaptiveColor`.
    CompleteAdaptiveColor {
        light: CompleteColor,
        dark: CompleteColor,
    },
}

/// `lipgloss.CompleteColor`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct CompleteColor {
    pub true_color: String,
    pub ansi256: String,
    pub ansi: String,
}

/// `lipgloss.Color(s)`.
pub fn color(s: &str) -> TerminalColor {
    TerminalColor::Color(s.to_string())
}

/// `lipgloss.AdaptiveColor{Light: light, Dark: dark}`.
pub fn adaptive_color(light: &str, dark: &str) -> TerminalColor {
    TerminalColor::AdaptiveColor {
        light: light.to_string(),
        dark: dark.to_string(),
    }
}

impl From<&str> for TerminalColor {
    fn from(s: &str) -> Self {
        color(s)
    }
}

impl From<String> for TerminalColor {
    fn from(s: String) -> Self {
        TerminalColor::Color(s)
    }
}

impl From<&TerminalColor> for TerminalColor {
    fn from(c: &TerminalColor) -> Self {
        c.clone()
    }
}

fn complete(c: &CompleteColor, p: Profile) -> Option<termenv::Color> {
    match p {
        Profile::TrueColor => p.color(&c.true_color),
        Profile::ANSI256 => p.color(&c.ansi256),
        Profile::ANSI => p.color(&c.ansi),
        Profile::Ascii => Some(termenv::Color::NoColor),
    }
}

impl TerminalColor {
    /// `TerminalColor.color(renderer)`: the termenv colour for the current
    /// renderer (profile and dark-background setting). `None` is Go's nil.
    pub fn color(&self) -> Option<termenv::Color> {
        let p = renderer::color_profile();
        match self {
            TerminalColor::NoColor => Some(termenv::Color::NoColor),
            TerminalColor::Color(s) => p.color(s),
            TerminalColor::ANSIColor(u) => p.color(&u.to_string()),
            TerminalColor::AdaptiveColor { light, dark } => {
                if renderer::has_dark_background() {
                    p.color(dark)
                } else {
                    p.color(light)
                }
            }
            TerminalColor::CompleteColor(c) => complete(c, p),
            TerminalColor::CompleteAdaptiveColor { light, dark } => {
                if renderer::has_dark_background() {
                    complete(dark, p)
                } else {
                    complete(light, p)
                }
            }
        }
    }

    /// `TerminalColor.RGBA()` (16-bit components).
    pub fn rgba(&self) -> (u32, u32, u32, u32) {
        match self {
            TerminalColor::NoColor => (0, 0, 0, 0xFFFF),
            _ => termenv::convert_to_rgb(self.color().as_ref()).rgba(),
        }
    }
}
