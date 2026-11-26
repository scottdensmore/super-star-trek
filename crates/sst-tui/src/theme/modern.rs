//! Starfleet Modern theme (port of `pkg/tui/theme/modern.go`).

use sst_tuikit::lipgloss;

use super::{
    bfg, boxed, fg, plain_header, theme_common, title_style, ColorMode, LcarsTheme, Styles,
};

/// The Starfleet Modern high-contrast 24-bit TrueColor theme (Go `ModernTheme`).
/// `ModernTheme::default()` is Go's `ModernTheme{}`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ModernTheme {
    mode: ColorMode,
}

theme_common!(ModernTheme, "modern", LcarsTheme);

impl ModernTheme {
    /// `FormatHeader`: top status banner with clean UTF-8 borders.
    pub fn format_header(&self, title: &str, theme_info: &str, width: i32) -> String {
        plain_header(&self.styles(), title, theme_info, width)
    }

    /// `FormatAudioBadge`: HUD audio volume indicator.
    pub fn format_audio_badge(&self, volume: i32, muted: bool) -> String {
        let styles = self.styles();
        if muted || volume <= 0 {
            return styles.empty.render("🔇 MUTED");
        }
        styles.prompt.render(&format!("🔊 {volume}%"))
    }

    /// `PaletteStyles(isDark)`.
    pub fn palette_styles(&self, is_dark: bool) -> Styles {
        let border = lipgloss::rounded_border();
        if is_dark {
            return Styles {
                title: title_style("#00E5FF", "#0B0F19"),
                panel: boxed(&border, "#1E293B", "#0B0F19").padding(&[0, 1]),
                panel_title: bfg("#00E5FF"),
                border: border.clone(),
                grid: boxed(&lipgloss::normal_border(), "#1E293B", "#0B0F19"),
                grid_header: bfg("#64748B"),
                enterprise: bfg("#00E5FF"),
                klingon: bfg("#FF3344"),
                romulan: bfg("#10B981"),
                tholian: bfg("#00E5FF"),
                plasma_torpedo: bfg("#FF5722"),
                tholian_web: bfg("#22D3EE"),
                starbase: bfg("#FFD700"),
                star: fg("#FFFFFF"),
                planet: fg("#00E676"),
                black_hole: fg("#A855F7"),
                wormhole: bfg("#04D9FF"),
                empty: fg("#334155"),
                condition_green: bfg("#00E676"),
                condition_yellow: bfg("#FFD700"),
                condition_red: bfg("#FF3344"),
                condition_docked: bfg("#00E5FF"),
                alert_red: bfg("#FF3344"),
                alert_yellow: bfg("#FFD700"),
                progress_bar_filled: fg("#00E5FF"),
                progress_bar_empty: fg("#1E293B"),
                gauge_label: fg("#E2E8F0"),
                gauge_value: bfg("#00E5FF"),
                subsystem_normal: fg("#00E676"),
                subsystem_damaged: bfg("#FF3344"),
                prompt: bfg("#00E5FF"),
                command_text: fg("#E2E8F0"),
                log_text: fg("#94A3B8"),
            };
        }
        Styles {
            title: title_style("#0284C7", "#F8FAFC"),
            panel: boxed(&border, "#CBD5E1", "#F8FAFC").padding(&[0, 1]),
            panel_title: bfg("#0284C7"),
            border: border.clone(),
            grid: boxed(&lipgloss::normal_border(), "#CBD5E1", "#F8FAFC"),
            grid_header: bfg("#475569"),
            enterprise: bfg("#0284C7"),
            klingon: bfg("#DC2626"),
            romulan: bfg("#059669"),
            tholian: bfg("#0891B2"),
            plasma_torpedo: bfg("#EA580C"),
            tholian_web: bfg("#06B6D4"),
            starbase: bfg("#D97706"),
            star: fg("#475569"),
            planet: fg("#16A34A"),
            black_hole: fg("#7C3AED"),
            wormhole: bfg("#0284C7"),
            empty: fg("#94A3B8"),
            condition_green: bfg("#16A34A"),
            condition_yellow: bfg("#D97706"),
            condition_red: bfg("#DC2626"),
            condition_docked: bfg("#0284C7"),
            alert_red: bfg("#DC2626"),
            alert_yellow: bfg("#D97706"),
            progress_bar_filled: fg("#0284C7"),
            progress_bar_empty: fg("#E2E8F0"),
            gauge_label: fg("#334155"),
            gauge_value: bfg("#0F172A"),
            subsystem_normal: fg("#16A34A"),
            subsystem_damaged: bfg("#DC2626"),
            prompt: bfg("#0284C7"),
            command_text: fg("#0F172A"),
            log_text: fg("#0F172A"),
        }
    }
}
