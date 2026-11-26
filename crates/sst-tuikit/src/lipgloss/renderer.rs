//! `lipgloss` renderer.go: the default renderer's colour profile and
//! dark-background setting.
//!
//! Go has one process-wide default renderer. Rust tests run in parallel
//! threads, so the explicit settings are kept per thread: a test calling
//! `set_color_profile(Profile::Ascii)` only affects its own thread. When a
//! value was never set explicitly it is detected from the environment exactly
//! like termenv (once per process): not a TTY -> `Ascii`, `NO_COLOR`,
//! `CLICOLOR(_FORCE)`, `COLORTERM`, `TERM`; the background colour comes from
//! an OSC 11 query / `COLORFGBG`.

use std::cell::Cell;

use crate::termenv::{self, Profile};

thread_local! {
    static PROFILE: Cell<Option<Profile>> = const { Cell::new(None) };
    static DARK: Cell<Option<bool>> = const { Cell::new(None) };
}

/// `lipgloss.ColorProfile()`.
pub fn color_profile() -> Profile {
    PROFILE
        .with(|p| p.get())
        .unwrap_or_else(termenv::env_color_profile)
}

/// `lipgloss.SetColorProfile(p)`.
pub fn set_color_profile(p: Profile) {
    PROFILE.with(|c| c.set(Some(p)));
}

/// `lipgloss.HasDarkBackground()`.
pub fn has_dark_background() -> bool {
    DARK.with(|d| d.get())
        .unwrap_or_else(termenv::has_dark_background)
}

/// `lipgloss.SetHasDarkBackground(b)`.
pub fn set_has_dark_background(b: bool) {
    DARK.with(|c| c.set(Some(b)));
}
