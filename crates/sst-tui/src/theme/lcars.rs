//! LCARS theme (port of `pkg/tui/theme/lcars.go`).

use sst_tuikit::lipgloss;

use super::{bfg, boxed, fg, theme_common, title_style, ColorMode, CrtTheme, Styles};

/// The 24th-century LCARS aesthetic palette (Go `LcarsTheme`).
/// `LcarsTheme::default()` is Go's `LcarsTheme{}`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct LcarsTheme {
    mode: ColorMode,
}

theme_common!(LcarsTheme, "lcars", CrtTheme);

impl LcarsTheme {
    /// `PillCaps`: the LCARS curved pill caps.
    pub fn pill_caps(&self) -> (&'static str, &'static str) {
        ("◖", "◗")
    }

    /// `FormatHeader`: banner with curved pill caps and colour blocking.
    pub fn format_header(&self, title: &str, theme_info: &str, width: i32) -> String {
        let is_dark = self.is_dark();

        let (primary, secondary, text, panel_bg) = if is_dark {
            ("#FF9900", "#3399CC", "#000000", "#000000")
        } else {
            ("#C2410C", "#0E7490", "#FEF9EF", "#FEF9EF")
        };
        let c = lipgloss::color;

        let cap_left = lipgloss::new_style()
            .foreground(c(primary))
            .background(c(panel_bg))
            .render("◖");
        let title_block = lipgloss::new_style()
            .bold(true)
            .foreground(c(text))
            .background(c(primary))
            .render(&format!(" {title} "));
        let theme_block = lipgloss::new_style()
            .bold(true)
            .foreground(c(text))
            .background(c(secondary))
            .render(&format!(" {theme_info} "));
        let cap_right = lipgloss::new_style()
            .foreground(c(secondary))
            .background(c(panel_bg))
            .render("◗");

        let fixed_width = lipgloss::width(&cap_left)
            + lipgloss::width(&title_block)
            + lipgloss::width(&theme_block)
            + lipgloss::width(&cap_right);
        let mut gap = width - fixed_width;
        if gap < 1 {
            gap = 1;
        }
        let gap_str = lipgloss::new_style()
            .background(c(panel_bg))
            .render(&" ".repeat(gap as usize));

        format!("{cap_left}{title_block}{gap_str}{theme_block}{cap_right}")
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
                title: title_style("#000000", "#FF9900"),
                panel: boxed(&border, "#FF9900", "#000000").padding(&[0, 1]),
                panel_title: bfg("#FFCC66"),
                border: border.clone(),
                grid: boxed(&border, "#3399CC", "#000000"),
                grid_header: bfg("#FFCC66"),
                enterprise: bfg("#FFCC66"),
                klingon: bfg("#FF3300"),
                romulan: bfg("#CC6699"),
                tholian: bfg("#FFD700"),
                plasma_torpedo: bfg("#FF6600"),
                tholian_web: bfg("#FFCC66"),
                starbase: bfg("#3399CC"),
                star: fg("#FFFFFF"),
                planet: fg("#CC6699"),
                black_hole: fg("#9966CC"),
                wormhole: bfg("#00CCCC"),
                empty: fg("#443322"),
                condition_green: bfg("#3399CC"),
                condition_yellow: bfg("#FF9900"),
                condition_red: bfg("#FF3300"),
                condition_docked: bfg("#CC6699"),
                alert_red: bfg("#FF3300"),
                alert_yellow: bfg("#FF9900"),
                progress_bar_filled: fg("#FF9900"),
                progress_bar_empty: fg("#332211"),
                gauge_label: fg("#FFCC66"),
                gauge_value: bfg("#FF9900"),
                subsystem_normal: fg("#3399CC"),
                subsystem_damaged: bfg("#FF3300"),
                prompt: bfg("#FF9900"),
                command_text: fg("#FFCC66"),
                log_text: fg("#CC9966"),
            };
        }
        Styles {
            title: title_style("#FEF9EF", "#C2410C"),
            panel: boxed(&border, "#C2410C", "#FEF9EF").padding(&[0, 1]),
            panel_title: bfg("#9A3412"),
            border: border.clone(),
            grid: boxed(&border, "#0E7490", "#FEF9EF"),
            grid_header: bfg("#9A3412"),
            enterprise: bfg("#C2410C"),
            klingon: bfg("#B91C1C"),
            romulan: bfg("#86198F"),
            tholian: bfg("#A16207"),
            plasma_torpedo: bfg("#C2410C"),
            tholian_web: bfg("#D97706"),
            starbase: bfg("#0E7490"),
            star: fg("#57534E"),
            planet: fg("#9D174D"),
            black_hole: fg("#6B21A8"),
            wormhole: bfg("#008888"),
            empty: fg("#A8A29E"),
            condition_green: bfg("#0E7490"),
            condition_yellow: bfg("#C2410C"),
            condition_red: bfg("#B91C1C"),
            condition_docked: bfg("#9D174D"),
            alert_red: bfg("#B91C1C"),
            alert_yellow: bfg("#C2410C"),
            progress_bar_filled: fg("#C2410C"),
            progress_bar_empty: fg("#E7E5E4"),
            gauge_label: fg("#1C1917"),
            gauge_value: bfg("#0E7490"),
            subsystem_normal: fg("#0E7490"),
            subsystem_damaged: bfg("#B91C1C"),
            prompt: bfg("#C2410C"),
            command_text: fg("#1C1917"),
            log_text: fg("#78350F"),
        }
    }
}
