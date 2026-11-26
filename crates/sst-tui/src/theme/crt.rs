//! CRT theme (port of `pkg/tui/theme/crt.go`).

use sst_tuikit::lipgloss;

use super::{
    bfg, boxed, fg, plain_header, theme_common, title_style, ColorMode, ModernTheme, Styles,
};

/// The 1970s monochrome phosphor CRT green aesthetic (Go `CrtTheme`).
/// `CrtTheme::default()` is Go's `CrtTheme{}`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct CrtTheme {
    mode: ColorMode,
}

theme_common!(CrtTheme, "crt", ModernTheme);

impl CrtTheme {
    /// `FormatHeader`: phosphor text banner.
    pub fn format_header(&self, title: &str, theme_info: &str, width: i32) -> String {
        plain_header(&self.styles(), title, theme_info, width)
    }

    /// `FormatAudioBadge`: HUD audio volume indicator.
    pub fn format_audio_badge(&self, volume: i32, muted: bool) -> String {
        let styles = self.styles();
        if muted || volume <= 0 {
            return styles.empty.render("[SND: OFF]");
        }
        styles.prompt.render(&format!("[SND: {volume}%]"))
    }

    /// `PaletteStyles(isDark)`.
    pub fn palette_styles(&self, is_dark: bool) -> Styles {
        let border = lipgloss::normal_border();
        let c = lipgloss::color;
        if is_dark {
            return Styles {
                title: title_style("#050B05", "#33FF33"),
                panel: boxed(&border, "#33FF33", "#050B05").padding(&[0, 1]),
                panel_title: bfg("#33FF33"),
                border: border.clone(),
                grid: boxed(&border, "#116611", "#050B05"),
                grid_header: bfg("#33FF33"),
                enterprise: bfg("#33FF33").reverse(true),
                klingon: bfg("#33FF33"),
                romulan: bfg("#33FF33"),
                tholian: bfg("#FFB000"),
                plasma_torpedo: bfg("#33FF33"),
                tholian_web: bfg("#66FF66"),
                starbase: bfg("#33FF33"),
                star: fg("#22CC22"),
                planet: fg("#22AA22"),
                black_hole: fg("#116611"),
                wormhole: bfg("#33FF33"),
                empty: fg("#0D3B0D"),
                condition_green: bfg("#33FF33"),
                condition_yellow: bfg("#33FF33").underline(true),
                condition_red: bfg("#050B05").background(c("#33FF33")),
                condition_docked: bfg("#33FF33").italic(true),
                alert_red: bfg("#050B05").background(c("#33FF33")),
                alert_yellow: bfg("#33FF33").underline(true),
                progress_bar_filled: fg("#33FF33"),
                progress_bar_empty: fg("#114411"),
                gauge_label: fg("#22CC22"),
                gauge_value: bfg("#33FF33"),
                subsystem_normal: fg("#33FF33"),
                subsystem_damaged: bfg("#050B05").background(c("#33FF33")),
                prompt: bfg("#33FF33"),
                command_text: fg("#33FF33"),
                log_text: fg("#22AA22"),
            };
        }
        Styles {
            title: title_style("#F0FDF4", "#14532D"),
            panel: boxed(&border, "#166534", "#F0FDF4").padding(&[0, 1]),
            panel_title: bfg("#14532D"),
            border: border.clone(),
            grid: boxed(&border, "#86EFAC", "#F0FDF4"),
            grid_header: bfg("#14532D"),
            enterprise: bfg("#14532D").reverse(true),
            klingon: bfg("#14532D"),
            romulan: bfg("#14532D"),
            tholian: bfg("#854D0E"),
            plasma_torpedo: bfg("#14532D"),
            tholian_web: bfg("#15803D"),
            starbase: bfg("#14532D"),
            star: fg("#15803D"),
            planet: fg("#15803D"),
            black_hole: fg("#86EFAC"),
            wormhole: bfg("#166534"),
            empty: fg("#86EFAC"),
            condition_green: bfg("#14532D"),
            condition_yellow: bfg("#14532D").underline(true),
            condition_red: bfg("#F0FDF4").background(c("#14532D")),
            condition_docked: bfg("#14532D").italic(true),
            alert_red: bfg("#F0FDF4").background(c("#14532D")),
            alert_yellow: bfg("#14532D").underline(true),
            progress_bar_filled: fg("#14532D"),
            progress_bar_empty: fg("#DCFCE7"),
            gauge_label: fg("#15803D"),
            gauge_value: bfg("#14532D"),
            subsystem_normal: fg("#14532D"),
            subsystem_damaged: bfg("#F0FDF4").background(c("#14532D")),
            prompt: bfg("#14532D"),
            command_text: fg("#14532D"),
            log_text: fg("#15803D"),
        }
    }
}
