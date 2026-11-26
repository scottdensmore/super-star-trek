//! Byte-for-byte parity with `testdata/parity/theme/theme.txt`, recorded by
//! `pkg/tui/theme/zz_paritygen_test.go`. The Rust side regenerates every
//! line in the same order and compares.

use std::rc::Rc;

use sst_engine::gofmt::quote;
use sst_tuikit::lipgloss::{self, Border, Style, TerminalColor};
use sst_tuikit::termenv::Profile;

use super::*;

const DATA: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testdata/parity/theme/theme.txt"
));

fn pg_color(c: &TerminalColor) -> String {
    match c {
        TerminalColor::NoColor => "NoColor".into(),
        TerminalColor::Color(s) => format!("Color({s})"),
        other => format!("{other:?}"),
    }
}

fn pg_border(b: &Border) -> String {
    [
        &b.top,
        &b.bottom,
        &b.left,
        &b.right,
        &b.top_left,
        &b.top_right,
        &b.bottom_left,
        &b.bottom_right,
        &b.middle_left,
        &b.middle_right,
        &b.middle,
        &b.middle_top,
        &b.middle_bottom,
    ]
    .iter()
    .map(|s| s.as_str())
    .collect::<Vec<_>>()
    .join("|")
}

const SAMPLES: [&str; 3] = ["USS Enterprise ★ 1701", "line one\nline two", ""];

struct Out(Vec<String>);

impl Out {
    fn w(&mut self, k: &str, v: &str) {
        self.0.push(format!("{k}\t{}", quote(v)));
    }

    fn style(&mut self, key: &str, s: &Style) {
        for (i, sample) in SAMPLES.iter().enumerate() {
            self.w(&format!("{key}.render{i}"), &s.render(sample));
        }
        self.w(&format!("{key}.fg"), &pg_color(&s.get_foreground()));
        self.w(&format!("{key}.bg"), &pg_color(&s.get_background()));
        self.w(
            &format!("{key}.border_top_fg"),
            &pg_color(&s.get_border_top_foreground()),
        );
        self.w(
            &format!("{key}.flags"),
            &format!(
                "bold={} italic={} underline={} reverse={} hframe={} vframe={}",
                s.get_bold(),
                s.get_italic(),
                s.get_underline(),
                s.get_reverse(),
                s.get_horizontal_frame_size(),
                s.get_vertical_frame_size()
            ),
        );
    }

    fn styles(&mut self, prefix: &str, st: &Styles) {
        let fields: [(&str, &Style); 32] = [
            ("Title", &st.title),
            ("Panel", &st.panel),
            ("PanelTitle", &st.panel_title),
            ("Grid", &st.grid),
            ("GridHeader", &st.grid_header),
            ("Enterprise", &st.enterprise),
            ("Klingon", &st.klingon),
            ("Romulan", &st.romulan),
            ("Tholian", &st.tholian),
            ("PlasmaTorpedo", &st.plasma_torpedo),
            ("TholianWeb", &st.tholian_web),
            ("Starbase", &st.starbase),
            ("Star", &st.star),
            ("Planet", &st.planet),
            ("BlackHole", &st.black_hole),
            ("Wormhole", &st.wormhole),
            ("Empty", &st.empty),
            ("ConditionGreen", &st.condition_green),
            ("ConditionYellow", &st.condition_yellow),
            ("ConditionRed", &st.condition_red),
            ("ConditionDocked", &st.condition_docked),
            ("AlertRed", &st.alert_red),
            ("AlertYellow", &st.alert_yellow),
            ("ProgressBarFilled", &st.progress_bar_filled),
            ("ProgressBarEmpty", &st.progress_bar_empty),
            ("GaugeLabel", &st.gauge_label),
            ("GaugeValue", &st.gauge_value),
            ("SubsystemNormal", &st.subsystem_normal),
            ("SubsystemDamaged", &st.subsystem_damaged),
            ("Prompt", &st.prompt),
            ("CommandText", &st.command_text),
            ("LogText", &st.log_text),
        ];
        for (i, (name, s)) in fields.iter().enumerate() {
            self.style(&format!("{prefix}{name}"), s);
            if i == 2 {
                // Go declaration order: Border follows PanelTitle.
                self.w(&format!("{prefix}Border"), &pg_border(&st.border));
            }
        }
    }
}

fn generate() -> Vec<String> {
    let mut o = Out(Vec::new());
    detect::set_detect_impl(Some(Rc::new(|| panic!("detection called"))));

    for m in [
        ColorMode::Unset,
        ColorMode::Auto,
        ColorMode::Dark,
        ColorMode::Light,
    ] {
        o.w(&format!("colormode/{m}/next"), m.next().as_str());
        o.w(
            &format!("colormode/{m}/resolve"),
            &format!("{} {}", m.resolve(true), m.resolve(false)),
        );
    }
    for input in [
        "auto",
        "AUTO",
        " dark ",
        "Light",
        "",
        "invalid",
        "darkk",
        "\tlight\n",
    ] {
        let (m, es) = match parse_color_mode(input) {
            Ok(m) => (m, "nil".to_string()),
            Err(e) => (ColorMode::Auto, e.to_string()),
        };
        o.w(&format!("parse/{}", quote(input)), &format!("{m} {es}"));
    }
    for input in ["modern", "MODERN", "Lcars", "  crt  ", "", "unknown"] {
        let th = get_theme(input);
        o.w(
            &format!("get/{}", quote(input)),
            &format!("{} {}", th.name(), th.color_mode()),
        );
    }
    o.w(
        "default",
        &format!(
            "{} {}",
            default_theme().name(),
            default_theme().color_mode()
        ),
    );

    let profiles = [
        ("Ascii", Profile::Ascii),
        ("ANSI", Profile::ANSI),
        ("ANSI256", Profile::ANSI256),
        ("TrueColor", Profile::TrueColor),
    ];
    let titles = ["★ SUPER STAR TREK ★  USS ENTERPRISE NCC-1701", "SST", ""];
    let tags = ["[Theme: MODERN (F2)]", ""];
    let widths = [0, 1, 10, 20, 40, 60, 80, 100, 132];
    let badges = [
        (80, false),
        (0, false),
        (0, true),
        (100, true),
        (-5, false),
        (1, false),
        (100, false),
        (150, false),
    ];

    lipgloss::set_has_dark_background(true);
    for name in [THEME_MODERN, THEME_LCARS, THEME_CRT] {
        for mode in [ColorMode::Dark, ColorMode::Light] {
            for (pname, p) in profiles {
                lipgloss::set_color_profile(p);
                let th = get_theme(name).with_color_mode(mode);
                let pre = format!("{name}/{mode}/{pname}/");
                o.w(&format!("{pre}name"), th.name());
                o.w(&format!("{pre}color_mode"), th.color_mode().as_str());
                o.w(
                    &format!("{pre}next"),
                    &format!("{} {}", th.next().name(), th.next().color_mode()),
                );
                o.w(
                    &format!("{pre}with_auto"),
                    th.with_color_mode(ColorMode::Auto).color_mode().as_str(),
                );
                o.styles(&format!("{pre}styles."), &th.styles());
                o.style(&format!("{pre}get.Romulan"), &th.romulan());
                o.style(&format!("{pre}get.Tholian"), &th.tholian());
                o.style(&format!("{pre}get.PlasmaTorpedo"), &th.plasma_torpedo());
                o.style(&format!("{pre}get.TholianWeb"), &th.tholian_web());
                o.style(&format!("{pre}get.AlertRed"), &th.alert_red());
                o.style(&format!("{pre}get.AlertYellow"), &th.alert_yellow());
                for (ti, title) in titles.iter().enumerate() {
                    for (gi, tag) in tags.iter().enumerate() {
                        for width in widths {
                            o.w(
                                &format!("{pre}header.{ti}.{gi}.{width}"),
                                &th.format_header(title, tag, width),
                            );
                        }
                    }
                }
                for (vol, muted) in badges {
                    o.w(
                        &format!("{pre}badge.{vol}.{muted}"),
                        &th.format_audio_badge(vol, muted),
                    );
                }
                if let Theme::Lcars(lc) = th {
                    let (l, r) = lc.pill_caps();
                    o.w(&format!("{pre}pillcaps"), &format!("{l}{r}"));
                }
            }
        }
    }
    lipgloss::set_color_profile(Profile::TrueColor);
    for th in [
        Theme::from(ModernTheme::default()),
        LcarsTheme::default().into(),
        CrtTheme::default().into(),
    ] {
        for dark in [true, false] {
            o.styles(
                &format!("palette/{}/{dark}/", th.name()),
                &th.palette_styles(dark),
            );
        }
    }
    detect::reset_state();
    o.0
}

#[test]
fn theme_parity() {
    let got = generate();
    let want: Vec<&str> = DATA.lines().collect();
    let mut mismatches = 0;
    for (i, (g, w)) in got.iter().zip(want.iter()).enumerate() {
        if g != w {
            mismatches += 1;
            if mismatches <= 5 {
                eprintln!("line {}:\n  go:   {w}\n  rust: {g}", i + 1);
            }
        }
    }
    assert_eq!(got.len(), want.len(), "line count");
    assert_eq!(mismatches, 0, "{mismatches} mismatching lines");
    eprintln!(
        "theme parity: {} lines, {} bytes compared",
        want.len(),
        DATA.len()
    );
}
