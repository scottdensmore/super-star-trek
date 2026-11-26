//! Port of `pkg/tui/components/galacticchart/chart_test.go`.

use sst_tuikit::ansi;
use sst_tuikit::lipgloss;
use sst_tuikit::tea::{msg, KeyType};
use sst_tuikit::termenv::Profile;

use super::*;
use crate::theme::{detect, ModernTheme};

fn setup() {
    detect::set_fixed_detection(true);
    lipgloss::set_color_profile(Profile::Ascii);
}

fn dt() -> Option<Theme> {
    Some(theme::default_theme())
}

fn modern() -> Option<Theme> {
    Some(ModernTheme::default().into())
}

const Z: [[i32; 9]; 9] = [[0; 9]; 9];
const F: [[bool; 9]; 9] = [[false; 9]; 9];

fn key(t: KeyType) -> Msg {
    msg(KeyMsg::new(t))
}

fn runes(s: &str) -> Msg {
    msg(KeyMsg::runes(s))
}

fn assert_dims(view: &str) {
    let lines: Vec<&str> = view.trim_end_matches('\n').split('\n').collect();
    assert_eq!(lines.len(), 18);
    for (i, l) in lines.iter().enumerate() {
        assert_eq!(lipgloss::width(l), 64, "line {i}: {l:?}");
    }
}

#[test]
fn cursor_navigation_and_bounds() {
    setup();
    let mut m = new(dt(), 64, 18);
    m.set_state([3, 3], Z, F, F, false);
    assert_eq!(m.cursor(), [3, 3]);
    let _ = m.update(&runes("k"));
    assert_eq!(m.cursor(), [2, 3]);
    let _ = m.update(&runes("h"));
    assert_eq!(m.cursor(), [2, 2]);
    for _ in 0..3 {
        let _ = m.update(&runes("k"));
    }
    assert_eq!(m.cursor()[0], 1);
    for _ in 0..3 {
        let _ = m.update(&runes("h"));
    }
    assert_eq!(m.cursor()[1], 1);
    let _ = m.update(&key(KEY_DOWN));
    assert_eq!(m.cursor(), [2, 1]);
    let _ = m.update(&key(KEY_RIGHT));
    assert_eq!(m.cursor(), [2, 2]);
    for _ in 0..10 {
        let _ = m.update(&runes("j"));
    }
    assert_eq!(m.cursor()[0], 8);
    for _ in 0..10 {
        let _ = m.update(&runes("l"));
    }
    assert_eq!(m.cursor()[1], 8);
}

#[test]
fn enter_and_esc_messages() {
    setup();
    let mut m = new(dt(), 64, 18);
    m.set_state([3, 3], Z, F, F, false);
    let _ = m.update(&key(KEY_UP));
    let r = m.update(&key(KEY_ENTER)).expect("cmd").call().unwrap();
    let warp = r
        .downcast_ref::<WarpToQuadrantMsg>()
        .expect("WarpToQuadrantMsg");
    assert_eq!(warp.dest_quad, [2, 3]);
    assert_eq!(warp.warp, 1.0);
    let r = m.update(&key(KEY_ESC)).expect("cmd").call().unwrap();
    assert!(r.downcast_ref::<CloseChartMsg>().is_some());
}

#[test]
fn view_dimensions_and_layout() {
    setup();
    let mut m = new(dt(), 64, 18);
    let mut chart = Z;
    let mut disc = F;
    chart[3][3] = 3;
    disc[3][3] = true;
    chart[2][5] = 105;
    disc[2][5] = true;
    m.set_state([3, 3], chart, disc, F, false);
    let view = m.view();
    assert!(view.contains("GALACTIC STAR CHART"));
    assert!(view.contains("105"));
    assert_dims(&view);
}

#[test]
fn telemetry_formatting() {
    setup();
    let mut m = new(dt(), 64, 18);
    m.set_state([3, 3], Z, F, F, false);
    assert!(m.view().contains("Current Position"));
    let _ = m.update(&key(KEY_UP));
    let v = m.view();
    assert!(v.contains("Target: Quad [2, 3]"), "{v}");
    assert!(v.contains("Dist: 1.0 quads (ΔR: -1, ΔC: 0)"), "{v}");
    assert!(v.contains("Course:") && v.contains("North"));
    assert!(v.contains("Warp: 1.0"));
    for k in [
        KEY_DOWN, KEY_DOWN, KEY_DOWN, KEY_RIGHT, KEY_RIGHT, KEY_RIGHT,
    ] {
        let _ = m.update(&key(k));
    }
    let v = m.view();
    assert!(v.contains("Target: Quad [5, 6]"));
    assert!(v.contains("ΔR: +2, ΔC: +3"));
    assert!(v.contains("South-East"));
}

#[test]
fn theme_and_lifecycle() {
    setup();
    let mut m = new(None, 0, 0);
    assert!(m.theme().is_some());
    assert!(m.init().is_none());
    m.set_theme(Some(theme::get_theme("lcars")));
    assert_eq!(m.theme().unwrap().name(), "lcars");
    m.set_theme(Some(theme::get_theme("crt")));
    assert!(m.view().contains("GALACTIC STAR CHART"));
    m.set_theme(None);
    assert!(m.theme().is_some());
    m.set_size(80, 24);
    assert!(m
        .update(&msg(WindowSizeMsg {
            width: 100,
            height: 50
        }))
        .is_none());
    struct Custom;
    assert!(m.update(&msg(Custom)).is_none());
}

#[test]
fn column_header_alignment() {
    setup();
    let mut m = new(dt(), 64, 18);
    let mut chart = Z;
    let mut disc = F;
    for c in 1..=8 {
        chart[1][c] = c as i32 * 10;
        disc[1][c] = true;
    }
    m.set_state([1, 1], chart, disc, F, false);
    let view = m.view();
    let lines: Vec<String> = view
        .trim_end_matches('\n')
        .split('\n')
        .map(ansi::strip)
        .collect();
    let idx = lines
        .iter()
        .position(|l| l.contains('1') && l.contains('8') && l.contains('2') && l.contains('3'))
        .expect("header line");
    let header: Vec<char> = lines[idx].chars().collect();
    let row: Vec<char> = lines[idx + 1].chars().collect();
    for col in 1..=8u32 {
        let d = char::from_digit(col, 10).unwrap();
        let hi = header.iter().position(|&r| r == d).expect("digit");
        assert!(hi < row.len());
        assert_eq!(row[hi], d, "column {col}");
    }
}

#[test]
fn set_state_clamps_enterprise_quad() {
    setup();
    let mut m = new(None, 64, 18);
    m.set_state([0, 10], Z, F, F, false);
    assert_eq!(m.enterprise_quad, [1, 8]);
    assert_eq!(m.cursor(), [1, 8]);
}

#[test]
fn known_base_rendering() {
    setup();
    let mut m = new(modern(), 64, 18);
    let mut chart = Z;
    let mut bases = F;
    chart[2][3] = 15;
    bases[2][3] = true;
    m.set_state([4, 4], chart, F, bases, false);
    assert!(m.view().contains(".1."));
}

#[test]
fn computer_damaged_telemetry_and_lockout() {
    setup();
    let mut m = new(modern(), 64, 18);
    m.set_state([4, 4], Z, F, F, true);
    let v = m.view();
    for s in [
        "[CALC OFFLINE]",
        "Target: ",
        "Quad [",
        "Disabled (Comp Offline)",
    ] {
        assert!(v.contains(s), "missing {s}");
    }
    m.cursor = [6, 6];
    let r = m.update(&key(KEY_ENTER)).expect("cmd").call().unwrap();
    let b = r.downcast_ref::<WarpBlockedMsg>().expect("WarpBlockedMsg");
    assert!(b.reason.contains("COMPUTER DAMAGED"));
}

#[test]
fn computer_damaged_dimensions() {
    setup();
    let mut m = new(modern(), 64, 18);
    m.set_state([4, 4], Z, F, F, true);
    assert_dims(&m.view());
}

#[test]
fn legend_rendering() {
    setup();
    let mut m = new(modern(), 64, 18);
    m.set_state([3, 3], Z, F, F, false);
    let v = m.view();
    assert!(v.contains("Legend:") && v.contains("··· Unexplored"));
    assert!(v.contains(".1. Base"));
    assert!(v.contains("KBS (Klingon/Base/Star)"));
    assert!(v.contains("[?] Help"));
    assert_dims(&v);
}

#[test]
fn help_toggle() {
    setup();
    let mut m = new(modern(), 64, 18);
    m.set_state([3, 3], Z, F, F, false);
    assert!(!m.showing_help());
    let _ = m.update(&runes("?"));
    assert!(m.showing_help());
    let hv = m.view();
    for s in [
        "GALACTIC STAR CHART GUIDE",
        "Unexplored quadrant",
        "K (hundreds) = Klingon battlecruisers",
        "B (tens)     = Federation starbases",
        "S (units)    = Stars",
    ] {
        assert!(hv.contains(s), "missing {s}");
    }
    assert_dims(&hv);
    let _ = m.update(&runes("?"));
    assert!(!m.showing_help());
    let _ = m.update(&runes("?"));
    assert!(m.showing_help());
    let _ = m.update(&key(KEY_ESC));
    assert!(!m.showing_help());
    m.set_showing_help(true);
    assert!(m.showing_help());
}

#[test]
fn hit_test() {
    setup();
    let mut m = new(dt(), 64, 18);
    for r in 1..=8 {
        for c in 1..=8 {
            let sx = 7 + 7 * (c - 1);
            for dx in 0..5 {
                assert_eq!(m.hit_test(sx + dx, r + 3), Some([r, c]));
            }
        }
    }
    for (x, y) in [
        (10, 0),
        (10, 1),
        (10, 2),
        (10, 3),
        (10, 12),
        (0, 5),
        (3, 5),
        (12, 5),
        (13, 5),
        (63, 5),
        (64, 5),
    ] {
        assert_eq!(m.hit_test(x, y), None, "({x}, {y})");
    }
    m.set_showing_help(true);
    assert_eq!(m.hit_test(8, 4), None);
}
