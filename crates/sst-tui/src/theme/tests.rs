//! Ports of `theme_test.go` and `detect_test.go`.

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use sst_tuikit::lipgloss::{self, TerminalColor};

use super::detect::{self, *};
use super::*;

/// Pins Auto-mode detection for tests that use zero-value themes (Go runs
/// the real detector there).
fn pin_dark() {
    detect::set_fixed_detection(true);
}

fn c(s: &str) -> TerminalColor {
    lipgloss::color(s)
}

fn all_zero_themes() -> Vec<Theme> {
    vec![
        ModernTheme::default().into(),
        LcarsTheme::default().into(),
        CrtTheme::default().into(),
    ]
}

#[test]
fn theme_cycling_and_lookup() {
    let def = default_theme();
    assert_eq!(def.name(), "modern");
    let next = def.next();
    assert_eq!(next.name(), "lcars");
    let third = next.next();
    assert_eq!(third.name(), "crt");
    assert_eq!(third.next().name(), "modern");
    assert_eq!(get_theme("lcars").name(), "lcars");
    assert_eq!(get_theme("unknown").name(), "modern");
}

#[test]
fn theme_case_insensitive_lookup() {
    for (input, expected) in [
        ("modern", "modern"),
        ("MODERN", "modern"),
        ("Lcars", "lcars"),
        ("LCARS", "lcars"),
        ("  crt  ", "crt"),
        ("CRT", "crt"),
        ("nonexistent", "modern"),
        ("", "modern"),
    ] {
        assert_eq!(get_theme(input).name(), expected, "GetTheme({input:?})");
    }
}

#[test]
fn styles_non_null() {
    pin_dark();
    for th in all_zero_themes() {
        let s = th.styles();
        assert!(!s.title.render("test").is_empty());
        assert!(!s.panel.render("panel").is_empty());
        assert!(!s.grid.render("grid").is_empty());
        assert!(!s.condition_green.render("GREEN").is_empty());
        assert!(!s.condition_yellow.render("YELLOW").is_empty());
        assert!(!s.condition_red.render("RED").is_empty());
        assert!(!s.condition_docked.render("DOCKED").is_empty());
        assert!(!s.prompt.render("COMMAND> ").is_empty());
    }
}

#[test]
fn color_mode_resolution() {
    for (mode, term_dark, expected) in [
        (ColorMode::Auto, true, true),
        (ColorMode::Auto, false, false),
        (ColorMode::Dark, true, true),
        (ColorMode::Dark, false, true),
        (ColorMode::Light, true, false),
        (ColorMode::Light, false, false),
    ] {
        assert_eq!(
            mode.resolve(term_dark),
            expected,
            "{mode}.Resolve({term_dark})"
        );
    }
}

#[test]
fn color_mode_next() {
    assert_eq!(ColorMode::Auto.next(), ColorMode::Dark);
    assert_eq!(ColorMode::Dark.next(), ColorMode::Light);
    assert_eq!(ColorMode::Light.next(), ColorMode::Auto);
    assert_eq!(ColorMode::Unset.next(), ColorMode::Auto);
}

#[test]
fn parse_color_mode_cases() {
    for (input, expected, err) in [
        ("auto", ColorMode::Auto, false),
        ("AUTO", ColorMode::Auto, false),
        ("dark", ColorMode::Dark, false),
        ("Dark", ColorMode::Dark, false),
        ("light", ColorMode::Light, false),
        ("LIGHT", ColorMode::Light, false),
        ("invalid", ColorMode::Auto, true),
    ] {
        let got = parse_color_mode(input);
        assert_eq!(got.is_err(), err, "{input}");
        assert_eq!(got.clone().unwrap_or(ColorMode::Auto), expected, "{input}");
    }
    assert_eq!(
        parse_color_mode("invalid").unwrap_err().to_string(),
        "invalid color mode: \"invalid\""
    );
}

#[test]
fn default_theme_color_mode() {
    assert_eq!(default_theme().color_mode(), ColorMode::Auto);
    assert_eq!(ModernTheme::default().color_mode(), ColorMode::Auto);
}

#[test]
fn theme_palette_styles() {
    for th in all_zero_themes() {
        let dark = th.palette_styles(true);
        let light = th.palette_styles(false);
        assert_ne!(dark.title.get_foreground(), TerminalColor::NoColor);
        assert_ne!(light.title.get_foreground(), TerminalColor::NoColor);
        assert_ne!(
            dark.panel.get_border_top_foreground(),
            TerminalColor::NoColor
        );
        assert_ne!(
            light.panel.get_border_top_foreground(),
            TerminalColor::NoColor
        );

        let dark_bg = dark.panel.get_background();
        let light_bg = light.panel.get_background();
        assert_ne!(dark_bg, light_bg, "theme {}", th.name());

        let th_dark = th.with_color_mode(ColorMode::Dark);
        assert_eq!(th_dark.color_mode(), ColorMode::Dark);
        assert_eq!(th_dark.styles().panel.get_background(), dark_bg);

        let th_light = th.with_color_mode(ColorMode::Light);
        assert_eq!(th_light.color_mode(), ColorMode::Light);
        assert_eq!(th_light.styles().panel.get_background(), light_bg);
    }
}

#[test]
fn modern_theme_palette_colors() {
    let th = ModernTheme::default();
    let dark = th.palette_styles(true);
    let light = th.palette_styles(false);

    assert_eq!(dark.panel.get_background(), c("#0B0F19"));
    assert_eq!(dark.panel.get_border_top_foreground(), c("#1E293B"));
    assert_eq!(dark.enterprise.get_foreground(), c("#00E5FF"));
    assert_eq!(dark.klingon.get_foreground(), c("#FF3344"));
    assert_eq!(dark.starbase.get_foreground(), c("#FFD700"));
    assert_eq!(dark.star.get_foreground(), c("#FFFFFF"));
    assert_eq!(dark.planet.get_foreground(), c("#00E676"));
    assert_eq!(dark.black_hole.get_foreground(), c("#A855F7"));
    assert_eq!(dark.wormhole.get_foreground(), c("#04D9FF"));
    assert_eq!(dark.empty.get_foreground(), c("#334155"));

    assert_eq!(light.panel.get_background(), c("#F8FAFC"));
    assert_eq!(light.panel.get_border_top_foreground(), c("#CBD5E1"));
    assert_eq!(light.title.get_foreground(), c("#0284C7"));
    assert_eq!(light.enterprise.get_foreground(), c("#0284C7"));
    assert_eq!(light.klingon.get_foreground(), c("#DC2626"));
    assert_eq!(light.starbase.get_foreground(), c("#D97706"));
    assert_eq!(light.star.get_foreground(), c("#475569"));
    assert_eq!(light.planet.get_foreground(), c("#16A34A"));
    assert_eq!(light.black_hole.get_foreground(), c("#7C3AED"));
    assert_eq!(light.wormhole.get_foreground(), c("#0284C7"));
    assert_eq!(light.empty.get_foreground(), c("#94A3B8"));
    assert_eq!(light.condition_green.get_foreground(), c("#16A34A"));
    assert_eq!(light.condition_yellow.get_foreground(), c("#D97706"));
    assert_eq!(light.condition_red.get_foreground(), c("#DC2626"));
    assert_eq!(light.progress_bar_filled.get_foreground(), c("#0284C7"));
    assert_eq!(light.progress_bar_empty.get_foreground(), c("#E2E8F0"));
    assert_eq!(light.gauge_label.get_foreground(), c("#334155"));
    assert_eq!(light.gauge_value.get_foreground(), c("#0F172A"));
    assert_eq!(light.prompt.get_foreground(), c("#0284C7"));
    assert_eq!(light.log_text.get_foreground(), c("#0F172A"));
}

#[test]
fn lcars_theme_palette_colors() {
    let th = LcarsTheme::default();
    let dark = th.palette_styles(true);
    let light = th.palette_styles(false);

    assert_eq!(dark.panel.get_background(), c("#000000"));
    assert_eq!(dark.title.get_background(), c("#FF9900"));
    assert_eq!(dark.title.get_foreground(), c("#000000"));
    assert_eq!(dark.panel.get_border_top_foreground(), c("#FF9900"));
    assert_eq!(dark.panel_title.get_foreground(), c("#FFCC66"));
    assert_eq!(dark.enterprise.get_foreground(), c("#FFCC66"));
    assert_eq!(dark.klingon.get_foreground(), c("#FF3300"));
    assert_eq!(dark.starbase.get_foreground(), c("#3399CC"));
    assert_eq!(dark.star.get_foreground(), c("#FFFFFF"));

    assert_eq!(light.panel.get_background(), c("#FEF9EF"));
    assert_eq!(light.title.get_background(), c("#C2410C"));
    assert_eq!(light.title.get_foreground(), c("#FEF9EF"));
    assert_eq!(light.panel.get_border_top_foreground(), c("#C2410C"));
    assert_eq!(light.panel_title.get_foreground(), c("#9A3412"));
    assert_eq!(light.enterprise.get_foreground(), c("#C2410C"));
    assert_eq!(light.klingon.get_foreground(), c("#B91C1C"));
    assert_eq!(light.starbase.get_foreground(), c("#0E7490"));
    assert_eq!(light.star.get_foreground(), c("#57534E"));
    assert_eq!(light.progress_bar_filled.get_foreground(), c("#C2410C"));
    assert_eq!(light.progress_bar_empty.get_foreground(), c("#E7E5E4"));
    assert_eq!(light.gauge_label.get_foreground(), c("#1C1917"));
    assert_eq!(light.gauge_value.get_foreground(), c("#0E7490"));
}

#[test]
fn crt_theme_palette_colors() {
    let th = CrtTheme::default();
    let dark = th.palette_styles(true);
    let light = th.palette_styles(false);

    assert_eq!(dark.panel.get_background(), c("#050B05"));
    assert_eq!(dark.title.get_background(), c("#33FF33"));
    assert_eq!(dark.title.get_foreground(), c("#050B05"));
    assert_eq!(dark.panel.get_border_top_foreground(), c("#33FF33"));
    assert_eq!(dark.enterprise.get_foreground(), c("#33FF33"));
    assert_eq!(dark.klingon.get_foreground(), c("#33FF33"));
    assert_eq!(dark.starbase.get_foreground(), c("#33FF33"));
    assert_eq!(dark.star.get_foreground(), c("#22CC22"));

    assert_eq!(light.panel.get_background(), c("#F0FDF4"));
    assert_eq!(light.title.get_background(), c("#14532D"));
    assert_eq!(light.title.get_foreground(), c("#F0FDF4"));
    assert_eq!(light.panel.get_border_top_foreground(), c("#166534"));
    assert_eq!(light.enterprise.get_foreground(), c("#14532D"));
    assert_eq!(light.klingon.get_foreground(), c("#14532D"));
    assert_eq!(light.starbase.get_foreground(), c("#14532D"));
    assert_eq!(light.star.get_foreground(), c("#15803D"));
    assert_eq!(light.gauge_label.get_foreground(), c("#15803D"));
    assert_eq!(light.gauge_value.get_foreground(), c("#14532D"));
    assert_eq!(light.empty.get_foreground(), c("#86EFAC"));
}

#[test]
fn theme_grid_backgrounds() {
    for th in all_zero_themes() {
        for is_dark in [true, false] {
            let styles = th.palette_styles(is_dark);
            let panel_bg = styles.panel.get_background();
            let grid_bg = styles.grid.get_background();
            assert_ne!(grid_bg, TerminalColor::NoColor);
            assert_eq!(grid_bg, panel_bg, "theme {} dark={is_dark}", th.name());
        }
    }
}

const TITLE: &str = "★ SUPER STAR TREK ★  USS ENTERPRISE NCC-1701";

#[test]
fn lcars_header_curves_and_pill_caps() {
    pin_dark();
    let th = LcarsTheme::default();
    let tag = "[Theme: LCARS (F2)]";
    let hdr = th.format_header(TITLE, tag, 80);
    assert!(hdr.contains('◖') && hdr.contains('◗'));
    assert!(hdr.contains(TITLE));
    assert!(hdr.contains(tag));
    assert!(!hdr.contains('\n'));
    assert_eq!(lipgloss::width(&hdr), 80);
    assert_eq!(th.pill_caps(), ("◖", "◗"));
}

#[test]
fn crt_phosphor_styling_and_scanline() {
    pin_dark();
    let th = CrtTheme::default();
    let tag = "[Theme: CRT (F2)]";
    let hdr = th.format_header(TITLE, tag, 80);
    assert!(hdr.contains(TITLE) && hdr.contains(tag));
    assert!(!hdr.contains('\n'));
    assert_eq!(lipgloss::width(&hdr), 80);
    let styles = th.styles();
    assert!(!styles.alert_red.render("WARNING").is_empty());
    assert!(styles.condition_red.get_bold());
}

#[test]
fn modern_header_clean_borders() {
    pin_dark();
    let th = ModernTheme::default();
    let tag = "[Theme: MODERN (F2)]";
    let hdr = th.format_header(TITLE, tag, 80);
    assert!(hdr.contains(TITLE) && hdr.contains(tag));
    assert!(!hdr.contains('\n'));
    assert_eq!(lipgloss::width(&hdr), 80);
}

#[test]
fn theme_audio_badges() {
    pin_dark();
    let modern = ModernTheme::default();
    let lcars = LcarsTheme::default();
    let crt = CrtTheme::default();

    let b = modern.format_audio_badge(80, false);
    assert!(b.contains("80%") && b.contains("🔊"), "{b:?}");
    let b = lcars.format_audio_badge(80, false);
    assert!(b.contains("80%") && b.contains("🔊"), "{b:?}");
    assert!(crt.format_audio_badge(80, false).contains("[SND: 80%]"));

    let b = modern.format_audio_badge(0, true);
    assert!(b.contains("MUTED") && b.contains("🔇"));
    let b = lcars.format_audio_badge(0, true);
    assert!(b.contains("MUTED") && b.contains("🔇"));
    assert!(crt.format_audio_badge(0, true).contains("[SND: OFF]"));

    assert!(modern.format_audio_badge(0, false).contains("MUTED"));
    assert!(crt.format_audio_badge(0, false).contains("[SND: OFF]"));
}

#[test]
fn themes_compare_by_value() {
    assert_eq!(
        Theme::from(ModernTheme::default()).with_color_mode(ColorMode::Auto),
        default_theme()
    );
    assert_ne!(
        default_theme(),
        default_theme().with_color_mode(ColorMode::Dark)
    );
    // Go: ModernTheme{} (mode "") != DefaultTheme() (mode "auto").
    assert_ne!(Theme::from(ModernTheme::default()), default_theme());
}

// ---- detect_test.go ----

#[test]
fn parse_apple_script_rgb_cases() {
    for (input, dark, ok) in [
        ("65535, 65535, 65535", false, true),
        ("0, 0, 0", true, true),
        ("50000, 50000, 50000", false, true),
        ("10000, 10000, 10000", true, true),
        ("65535, 65535", false, false),
        ("a, b, c", false, false),
        ("", false, false),
    ] {
        let (d, o) = parse_apple_script_rgb(input);
        assert_eq!(o, ok, "{input:?}");
        if o {
            assert_eq!(d, dark, "{input:?}");
        }
    }
}

fn env_fn(pairs: &'static [(&'static str, &'static str)]) -> GetenvFn {
    Rc::new(move |k| {
        pairs
            .iter()
            .find(|(n, _)| *n == k)
            .map(|(_, v)| v.to_string())
            .unwrap_or_default()
    })
}

/// Restores the default detection state when dropped (Go's deferred restores).
struct Restore;
impl Drop for Restore {
    fn drop(&mut self) {
        detect::reset_state();
    }
}

#[test]
fn detect_colorfgbg() {
    let _r = Restore;
    with_state(|s| {
        s.runtime_goos = "linux".into();
        s.detect_cache_ttl = Duration::ZERO;
        // Never query the real terminal.
        s.osc_detector = Rc::new(|_| (false, false));
    });
    reset_detection_cache();
    for (v, expected) in [
        ("15;0", true),
        ("0;15", false),
        ("15;7", false),
        ("15;8", true),
        ("15;1", true),
        ("15;11", false),
        ("15; +12 ", false),
        ("0;default", true),
    ] {
        let val = v.to_string();
        with_state(|s| {
            s.os_getenv = Rc::new(move |k| {
                if k == "COLORFGBG" {
                    val.clone()
                } else {
                    String::new()
                }
            })
        });
        assert_eq!(detect_dark_background(), expected, "COLORFGBG={v}");
        assert_eq!(lipgloss::has_dark_background(), expected);
    }
}

#[test]
fn detect_darwin_apple_terminal() {
    let _r = Restore;
    with_state(|s| {
        s.runtime_goos = "darwin".into();
        s.darwin_cache_ttl = Duration::ZERO;
        s.detect_cache_ttl = Duration::ZERO;
        s.os_getenv = env_fn(&[("TERM_PROGRAM", "Apple_Terminal")]);
        s.darwin_detector = Rc::new(|| false);
    });
    assert!(!detect_dark_background());
    with_state(|s| s.darwin_detector = Rc::new(|| true));
    assert!(detect_dark_background());
}

#[test]
fn detect_darwin_cache() {
    let _r = Restore;
    let count = Rc::new(Cell::new(0));
    let cc = count.clone();
    with_state(|s| {
        s.runtime_goos = "darwin".into();
        s.darwin_cache_ttl = Duration::from_secs(10);
        s.detect_cache_ttl = Duration::ZERO;
        s.os_getenv = env_fn(&[("TERM_PROGRAM", "Apple_Terminal")]);
        s.darwin_detector = Rc::new(move || {
            cc.set(cc.get() + 1);
            false
        });
    });
    assert!(!detect_dark_background());
    let _ = detect_dark_background();
    let _ = detect_dark_background();
    assert_eq!(count.get(), 1);
}

#[test]
fn lazy_color_mode_resolution() {
    let _r = Restore;
    set_detect_impl(Some(Rc::new(|| {
        panic!(
            "DetectDarkBackground should NOT be called when ColorMode is explicitly dark or light"
        )
    })));
    for th in [
        Theme::from(ModernTheme::default()),
        LcarsTheme::default().into(),
        CrtTheme::default().into(),
    ] {
        for m in [ColorMode::Light, ColorMode::Dark] {
            let t = th.with_color_mode(m);
            let _ = t.styles();
            let _ = t.format_header("a", "b", 20);
            let _ = t.format_audio_badge(5, false);
        }
    }
}

#[test]
fn parse_osc11_response_cases() {
    for (input, dark, ok) in [
        ("\x1b]11;rgb:ffff/ffff/ffff\x1b\\", false, true),
        ("\x1b]11;rgb:0000/0000/0000\x07", true, true),
        ("\x1b]11;rgb:1234/1234/1234\x1b\\", true, true),
        ("\x1b]11;rgb:eeee/eeee/eeee\x07", false, true),
        ("\x1b]11;rgb:ff/ff/ff\x1b\\", false, true),
        ("\x1b]11;rgb:00/00/00\x1b\\", true, true),
        ("invalid response", false, false),
        ("\x1b]11;rgb:zz/yy/xx\x1b\\", false, false),
        // Go strconv.ParseUint rejects signs.
        ("\x1b]11;rgb:+f/ff/ff\x1b\\", false, false),
    ] {
        let (d, o) = parse_osc11_response(input);
        assert_eq!(o, ok, "{input:?}");
        if o {
            assert_eq!(d, dark, "{input:?}");
        }
    }
    // 16-digit components: Go's (1 << 64) - 1 wraps to MaxUint64.
    assert_eq!(parse_hex_component("ffffffffffffffff"), Some(1.0));
    assert_eq!(parse_hex_component(""), None);
}

#[test]
fn detect_global_cache() {
    let _r = Restore;
    let count = Rc::new(Cell::new(0));
    let cc = count.clone();
    with_state(|s| {
        s.runtime_goos = "linux".into();
        s.detect_cache_ttl = Duration::from_secs(5);
        s.os_getenv = Rc::new(|_| String::new());
        s.osc_detector = Rc::new(move |_| {
            cc.set(cc.get() + 1);
            (false, true)
        });
    });
    reset_detection_cache();
    assert!(!detect_dark_background());
    lipgloss::set_has_dark_background(true);
    assert!(!detect_dark_background());
    // The cached path still applies SetHasDarkBackground.
    assert!(!lipgloss::has_dark_background());
    assert_eq!(count.get(), 1);
}

#[test]
fn detect_osc_fallback() {
    let _r = Restore;
    with_state(|s| {
        s.runtime_goos = "linux".into();
        s.detect_cache_ttl = Duration::ZERO;
        s.os_getenv = env_fn(&[("TERM_PROGRAM", "Apple_Terminal")]);
        s.osc_detector = Rc::new(|_| (false, false));
    });
    assert!(!detect_dark_background());
    with_state(|s| s.os_getenv = Rc::new(|_| String::new()));
    assert!(detect_dark_background());
    // Darwin without Apple_Terminal falls back to the system appearance.
    with_state(|s| {
        s.runtime_goos = "darwin".into();
        s.darwin_system_dark_detector = Rc::new(|| false);
    });
    assert!(!detect_dark_background());
}

#[test]
fn is_apple_script_default_white_cases() {
    for (input, expected) in [
        ("65535, 65535, 65535", true),
        ("65530, 65530, 65530", true),
        ("65535, 65535, 65535\n", true),
        (" 65535 , 65535 , 65535 ", true),
        ("0, 0, 0", false),
        ("55372, 54249, 46908", false),
        ("65520, 65535, 65535", false),
        ("65535, 65535", false),
        ("invalid, 1, 2", false),
        ("", false),
    ] {
        assert_eq!(is_apple_script_default_white(input), expected, "{input:?}");
    }
}

#[test]
fn detect_darwin_terminal_permutations() {
    let _r = Restore;
    #[rustfmt::skip]
    let cases: &[(&str, &'static str, bool, bool, bool)] = &[
        ("Default white profile in Dark Mode", "65535, 65535, 65535", false, true, true),
        ("Default white profile in Light Mode", "65535, 65535, 65535", false, false, false),
        ("Explicit dark profile (Pro) in Light Mode", "0, 0, 0", false, false, true),
        ("Explicit dark profile (Pro) in Dark Mode", "0, 0, 0", false, true, true),
        ("Explicit light profile (Novel) in Dark Mode", "55372, 54249, 46908", false, true, false),
        ("Explicit light profile (Novel) in Light Mode", "55372, 54249, 46908", false, false, false),
        ("AppleScript error fallback to Dark Mode", "", true, true, true),
        ("AppleScript error fallback to Light Mode", "", true, false, false),
        ("Invalid response fallback to Dark Mode", "unrecognized output", false, true, true),
        ("Invalid response fallback to Light Mode", "unrecognized output", false, false, false),
    ];
    for &(name, out, err, system_dark, expected) in cases {
        with_state(|s| {
            s.apple_script_detector = Rc::new(move |_| {
                if err {
                    Err("osascript execution failed".into())
                } else {
                    Ok(out.as_bytes().to_vec())
                }
            });
            s.darwin_system_dark_detector = Rc::new(move || system_dark);
        });
        assert_eq!(detect_darwin_terminal_dark_background(), expected, "{name}");
    }
}

#[test]
fn fixed_detection_sets_lipgloss_flag() {
    let _r = Restore;
    detect::set_fixed_detection(false);
    lipgloss::set_has_dark_background(true);
    let s = ModernTheme::default().styles();
    assert_eq!(
        s.panel.get_background(),
        ModernTheme::default()
            .palette_styles(false)
            .panel
            .get_background()
    );
    assert!(!lipgloss::has_dark_background());
}
