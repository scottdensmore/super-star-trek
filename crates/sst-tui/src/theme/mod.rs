//! TUI styling palettes (port of `pkg/tui/theme`: theme.go, modern.go,
//! lcars.go, crt.go, detect*.go).
//!
//! Go's `Theme` interface (implemented by the value types `ModernTheme`,
//! `LcarsTheme`, `CrtTheme`) becomes the `Copy` enum [`Theme`]; the concrete
//! structs keep their Go methods too. Themes compare with `==` like Go
//! interface values. A Go `nil` theme is `Option<Theme>::None`.
//!
//! ```
//! use sst_tui::theme::{self, ColorMode, Theme};
//! let th: Theme = theme::get_theme("lcars").with_color_mode(ColorMode::Dark);
//! let s = th.styles();
//! let k = s.klingon.render("+K+");
//! assert!(k.contains("+K+"));
//! ```

mod crt;
pub mod detect;
mod lcars;
mod modern;

#[cfg(test)]
mod parity_tests;
#[cfg(test)]
mod tests;

use std::fmt;

use sst_engine::gofmt;
use sst_tuikit::lipgloss::{self, Border, Style};

pub use crt::CrtTheme;
pub use detect::{detect_dark_background, reset_detection_cache};
pub use lcars::LcarsTheme;
pub use modern::ModernTheme;

/// Luminance mode for TUI themes (Go `type ColorMode string`).
///
/// [`ColorMode::Unset`] is Go's zero value `""`: it resolves like `Auto`,
/// but `"".Next()` is `Auto` (not `Dark`) and its string is empty.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ColorMode {
    /// Go zero value `ColorMode("")`.
    #[default]
    Unset,
    /// `ColorModeAuto`: dynamically detect terminal background darkness.
    Auto,
    /// `ColorModeDark`: force dark palettes.
    Dark,
    /// `ColorModeLight`: force light palettes.
    Light,
}

impl ColorMode {
    /// The Go string value ("", "auto", "dark", "light").
    pub fn as_str(self) -> &'static str {
        match self {
            ColorMode::Unset => "",
            ColorMode::Auto => "auto",
            ColorMode::Dark => "dark",
            ColorMode::Light => "light",
        }
    }

    /// `Resolve`: true if the mode resolves to dark.
    pub fn resolve(self, has_dark_bg: bool) -> bool {
        match self {
            ColorMode::Dark => true,
            ColorMode::Light => false,
            _ => has_dark_bg,
        }
    }

    /// `Next`: Auto -> Dark -> Light -> Auto (anything else -> Auto).
    pub fn next(self) -> ColorMode {
        match self {
            ColorMode::Auto => ColorMode::Dark,
            ColorMode::Dark => ColorMode::Light,
            _ => ColorMode::Auto,
        }
    }
}

impl fmt::Display for ColorMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Error of [`parse_color_mode`] (`invalid color mode: %q`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseColorModeError(pub String);

impl fmt::Display for ParseColorModeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid color mode: {}", gofmt::quote(&self.0))
    }
}

impl std::error::Error for ParseColorModeError {}

/// `ParseColorMode`. On error Go also returns `ColorModeAuto`.
pub fn parse_color_mode(val: &str) -> Result<ColorMode, ParseColorModeError> {
    match val.trim().to_lowercase().as_str() {
        "auto" => Ok(ColorMode::Auto),
        "dark" => Ok(ColorMode::Dark),
        "light" => Ok(ColorMode::Light),
        _ => Err(ParseColorModeError(val.to_string())),
    }
}

/// Lip Gloss styles for dashboard panels, grid cells, progress bars, alert
/// conditions and the command prompt (Go `Styles`).
#[derive(Clone, Default)]
pub struct Styles {
    // Title banner
    pub title: Style,

    // Panels and layout
    pub panel: Style,
    pub panel_title: Style,
    pub border: Border,

    // Sector grid
    pub grid: Style,
    pub grid_header: Style,

    // Entity glyphs
    pub enterprise: Style,
    pub klingon: Style,
    pub romulan: Style,
    pub tholian: Style,
    pub plasma_torpedo: Style,
    pub tholian_web: Style,
    pub starbase: Style,
    pub star: Style,
    pub planet: Style,
    pub black_hole: Style,
    pub wormhole: Style,
    pub empty: Style,

    // Condition alert badges
    pub condition_green: Style,
    pub condition_yellow: Style,
    pub condition_red: Style,
    pub condition_docked: Style,
    pub alert_red: Style,
    pub alert_yellow: Style,

    // Telemetry & progress bars
    pub progress_bar_filled: Style,
    pub progress_bar_empty: Style,
    pub gauge_label: Style,
    pub gauge_value: Style,

    // Subsystems / devices
    pub subsystem_normal: Style,
    pub subsystem_damaged: Style,

    // Command bar & log
    pub prompt: Style,
    pub command_text: Style,
    pub log_text: Style,
}

/// Identifier of the Starfleet Modern theme.
pub const THEME_MODERN: &str = "modern";
/// Identifier of the LCARS theme.
pub const THEME_LCARS: &str = "lcars";
/// Identifier of the CRT theme.
pub const THEME_CRT: &str = "crt";

/// A TUI theme (Go `Theme` interface).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Theme {
    Modern(ModernTheme),
    Lcars(LcarsTheme),
    Crt(CrtTheme),
}

/// `DefaultTheme()`: Starfleet Modern with `ColorModeAuto`.
pub fn default_theme() -> Theme {
    Theme::Modern(ModernTheme::with_mode(ColorMode::Auto))
}

/// `GetTheme(name)`: case-insensitive lookup, falling back to the default.
pub fn get_theme(name: &str) -> Theme {
    match name.trim().to_lowercase().as_str() {
        "modern" => Theme::Modern(ModernTheme::with_mode(ColorMode::Auto)),
        "lcars" => Theme::Lcars(LcarsTheme::with_mode(ColorMode::Auto)),
        "crt" => Theme::Crt(CrtTheme::with_mode(ColorMode::Auto)),
        _ => default_theme(),
    }
}

impl Default for Theme {
    /// Same as [`default_theme`].
    fn default() -> Self {
        default_theme()
    }
}

impl From<ModernTheme> for Theme {
    fn from(t: ModernTheme) -> Self {
        Theme::Modern(t)
    }
}
impl From<LcarsTheme> for Theme {
    fn from(t: LcarsTheme) -> Self {
        Theme::Lcars(t)
    }
}
impl From<CrtTheme> for Theme {
    fn from(t: CrtTheme) -> Self {
        Theme::Crt(t)
    }
}

macro_rules! dispatch {
    ($self:ident, $t:ident => $e:expr) => {
        match $self {
            Theme::Modern($t) => $e,
            Theme::Lcars($t) => $e,
            Theme::Crt($t) => $e,
        }
    };
}

impl Theme {
    pub fn name(&self) -> &'static str {
        dispatch!(self, t => t.name())
    }
    pub fn styles(&self) -> Styles {
        dispatch!(self, t => t.styles())
    }
    pub fn palette_styles(&self, is_dark: bool) -> Styles {
        dispatch!(self, t => t.palette_styles(is_dark))
    }
    pub fn color_mode(&self) -> ColorMode {
        dispatch!(self, t => t.color_mode())
    }
    pub fn with_color_mode(&self, mode: ColorMode) -> Theme {
        dispatch!(self, t => t.with_color_mode(mode))
    }
    pub fn next(&self) -> Theme {
        dispatch!(self, t => t.next())
    }
    pub fn romulan(&self) -> Style {
        dispatch!(self, t => t.romulan())
    }
    pub fn tholian(&self) -> Style {
        dispatch!(self, t => t.tholian())
    }
    pub fn plasma_torpedo(&self) -> Style {
        dispatch!(self, t => t.plasma_torpedo())
    }
    pub fn tholian_web(&self) -> Style {
        dispatch!(self, t => t.tholian_web())
    }
    pub fn alert_red(&self) -> Style {
        dispatch!(self, t => t.alert_red())
    }
    pub fn alert_yellow(&self) -> Style {
        dispatch!(self, t => t.alert_yellow())
    }
    pub fn format_header(&self, title: &str, theme_info: &str, width: i32) -> String {
        dispatch!(self, t => t.format_header(title, theme_info, width))
    }
    pub fn format_audio_badge(&self, volume: i32, muted: bool) -> String {
        dispatch!(self, t => t.format_audio_badge(volume, muted))
    }
}

/// Methods shared verbatim by the three Go theme types.
macro_rules! theme_common {
    ($ty:ident, $name:expr, $next:ident) => {
        impl $ty {
            pub(crate) const fn with_mode(mode: $crate::theme::ColorMode) -> Self {
                $ty { mode }
            }
            pub fn name(&self) -> &'static str {
                $name
            }
            pub fn color_mode(&self) -> $crate::theme::ColorMode {
                if self.mode == $crate::theme::ColorMode::Unset {
                    return $crate::theme::ColorMode::Auto;
                }
                self.mode
            }
            pub fn with_color_mode(&self, mode: $crate::theme::ColorMode) -> $crate::theme::Theme {
                let mut t = *self;
                t.mode = mode;
                t.into()
            }
            pub fn next(&self) -> $crate::theme::Theme {
                $next::with_mode(self.color_mode()).into()
            }
            pub fn romulan(&self) -> ::sst_tuikit::lipgloss::Style {
                self.styles().romulan
            }
            pub fn tholian(&self) -> ::sst_tuikit::lipgloss::Style {
                self.styles().tholian
            }
            pub fn plasma_torpedo(&self) -> ::sst_tuikit::lipgloss::Style {
                self.styles().plasma_torpedo
            }
            pub fn tholian_web(&self) -> ::sst_tuikit::lipgloss::Style {
                self.styles().tholian_web
            }
            pub fn alert_red(&self) -> ::sst_tuikit::lipgloss::Style {
                self.styles().alert_red
            }
            pub fn alert_yellow(&self) -> ::sst_tuikit::lipgloss::Style {
                self.styles().alert_yellow
            }
            /// Resolves the colour mode (detecting only in auto mode).
            fn is_dark(&self) -> bool {
                match self.color_mode() {
                    $crate::theme::ColorMode::Dark => true,
                    $crate::theme::ColorMode::Light => false,
                    _ => $crate::theme::detect_dark_background(),
                }
            }
            pub fn styles(&self) -> $crate::theme::Styles {
                self.palette_styles(self.is_dark())
            }
        }
    };
}
pub(crate) use theme_common;

/// `NewStyle().Foreground(c)`.
pub(crate) fn fg(c: &str) -> Style {
    lipgloss::new_style().foreground(lipgloss::color(c))
}

/// `NewStyle().Bold(true).Foreground(c)`.
pub(crate) fn bfg(c: &str) -> Style {
    lipgloss::new_style()
        .bold(true)
        .foreground(lipgloss::color(c))
}

/// `NewStyle().Bold(true).Foreground(f).Background(b).Padding(0, 1)`.
pub(crate) fn title_style(f: &str, b: &str) -> Style {
    bfg(f).background(lipgloss::color(b)).padding(&[0, 1])
}

/// `NewStyle().BorderStyle(border).BorderForeground(bf).Background(bg)`.
pub(crate) fn boxed(border: &Border, bf: &str, bg: &str) -> Style {
    lipgloss::new_style()
        .border_style(border.clone())
        .border_foreground(&[lipgloss::color(bf)])
        .background(lipgloss::color(bg))
}

/// Modern/CRT `FormatHeader`: title and info separated by a gap, rendered
/// with the title style at `width`.
pub(crate) fn plain_header(styles: &Styles, title: &str, theme_info: &str, width: i32) -> String {
    let frame = styles.title.get_horizontal_frame_size();
    let content_width = width - frame;
    let mut gap = content_width - lipgloss::width(title) - lipgloss::width(theme_info);
    if gap < 2 {
        gap = 2;
    }
    let header_line = format!("{title}{}{theme_info}", " ".repeat(gap as usize));
    styles.title.clone().width(width).render(&header_line)
}
