//! Port of `pkg/tui/components/statuspanel/statuspanel_test.go`.

use sst_engine::{GameLength, SkillLevel, ANIM_SPEED_NORMAL, ANIM_SPEED_OFF};
use sst_tuikit::lipgloss;
use sst_tuikit::termenv::Profile;

use super::*;
use crate::theme::{detect, CrtTheme, LcarsTheme, ModernTheme};

fn setup() {
    detect::set_fixed_detection(true);
    lipgloss::set_color_profile(Profile::Ascii);
}

fn game() -> GameState {
    GameState::new_game(12345, SkillLevel::Good, GameLength::Medium)
}

fn dt() -> Option<Theme> {
    Some(theme::default_theme())
}

fn modern() -> Option<Theme> {
    Some(ModernTheme::default().into())
}

fn lrs() -> usize {
    DeviceID::LRSensors.as_i32() as usize
}

#[test]
fn status_panel_rendering() {
    setup();
    let m = new(dt(), &[]);
    let mut g = game();
    g.enterprise.energy = 4500.0;
    g.enterprise.shields = 1500.0;
    g.enterprise.condition = ConditionType::Yellow;
    let view = m.view(Some(&g));
    for s in ["YELLOW", "4500", "1500"] {
        assert!(view.contains(s), "missing {s}:\n{view}");
    }
}

#[test]
fn condition_alerts() {
    setup();
    let m = new(dt(), &[]);
    for (cond, want) in [
        (ConditionType::Green, "GREEN"),
        (ConditionType::Yellow, "YELLOW"),
        (ConditionType::Red, "RED"),
        (ConditionType::Docked, "DOCKED"),
    ] {
        let mut g = game();
        g.enterprise.condition = cond;
        assert!(m.view(Some(&g)).contains(want), "{want}");
    }
}

#[test]
fn telemetry_progress_bars() {
    setup();
    let m = new(dt(), &[]);
    let mut g = game();
    g.enterprise.energy = 4500.0;
    g.enterprise.shields = 1500.0;
    let view = m.view(Some(&g));
    assert!(view.contains('█') && view.contains('░'), "{view}");
    assert!(view.contains("4500"));
    assert!(view.contains("1500"));
}

#[test]
fn torpedo_inventory() {
    setup();
    let m = new(dt(), &[]);
    for (t, want) in [
        (10, "[TORP: 10/10]"),
        (5, "[TORP: 5/10]"),
        (0, "[TORP: 0/10]"),
    ] {
        let mut g = game();
        g.enterprise.torpedoes = t;
        assert!(m.view(Some(&g)).contains(want), "{want}");
    }
}

#[test]
fn status_panel_sound_telemetry_badge() {
    setup();
    let mut m = new(dt(), &[]);
    let g = game();
    let on = m.view(Some(&g));
    assert!(on.contains("80%"), "{on}");
    assert!(!on.contains("MUTED"));
    m.set_sound_enabled(false);
    assert!(m.view(Some(&g)).contains("MUTED"));
    m.set_sound_enabled(true);
    assert!(m.view(Some(&g)).contains("80%"));
    m.set_theme(Some(CrtTheme::default().into()));
    assert!(m.view(Some(&g)).contains("[SND: 80%]"));
    m.set_sound_enabled(false);
    assert!(m.view(Some(&g)).contains("[SND: OFF]"));
}

#[test]
fn stardate_and_time_remaining() {
    setup();
    let m = new(dt(), &[]);
    let mut g = game();
    g.stardate = 2851.4;
    g.time_remaining = 24.5;
    let view = m.view(Some(&g));
    assert!(view.contains("2851.4") && view.contains("24.5"), "{view}");
}

#[test]
fn subsystem_device_repair_countdowns() {
    setup();
    let m = new(dt(), &[]);
    let mut g = game();
    let view = m.view(Some(&g));
    for d in [
        "Warp",
        "SRS",
        "LRS",
        "Phasers",
        "Tubes",
        "Damage Control",
        "Shields",
        "Computer",
    ] {
        assert!(view.contains(d), "missing {d}");
    }
    assert!(view.contains("OK"));
    g.enterprise.devices[DeviceID::Warp.as_i32() as usize] = 4.2;
    g.enterprise.devices[DeviceID::DamageControl.as_i32() as usize] = 7.5;
    let v = m.view(Some(&g));
    assert!(v.contains("4.2") && v.contains("7.5"), "{v}");
}

#[test]
fn surrounding_quadrant_radar() {
    setup();
    let m = new(dt(), &[]);
    let mut g = game();
    g.enterprise.quad = [4, 4];
    g.galaxy_chart[3][3] = 105;
    g.galaxy_chart[3][4] = 203;
    g.galaxy_chart[4][4] = 312;
    g.galaxy_chart[5][5] = 407;
    let view = m.view(Some(&g));
    for s in ["105", "203", "312", "407"] {
        assert!(view.contains(s), "missing {s}:\n{view}");
    }
    g.enterprise.quad = [1, 1];
    assert!(m.view(Some(&g)).contains("***"));
}

#[test]
fn status_panel_location_readout_and_radar_headers() {
    setup();
    let mut g = game();
    g.enterprise.quad = [3, 5];
    g.enterprise.sector = [2, 6];
    g.galaxy_chart[3][5] = 3;
    g.galaxy_chart[2][5] = 105;
    g.galaxy_chart[3][6] = 12;
    let view = new(dt(), &[]).view(Some(&g));
    assert!(
        view.contains("LOC: ") && view.contains("Q[3,5] S[2,6]"),
        "{view}"
    );
    assert!(view.contains("RADAR (±1) [K-B-S]:"));
    assert!(view.contains("4    5    6"), "{view}");
    for h in ["  2  ", "  3  ", "  4  "] {
        assert!(view.contains(h), "{h:?}");
    }
    assert!(view.contains("<003>"));
}

#[test]
fn set_theme() {
    setup();
    lipgloss::set_color_profile(Profile::TrueColor);
    let mut m = new(modern(), &[]);
    assert_eq!(m.theme().name(), "modern");
    let g = game();
    let vm = m.view(Some(&g));
    m.set_theme(Some(LcarsTheme::default().into()));
    assert_eq!(m.theme().name(), "lcars");
    let vl = m.view(Some(&g));
    assert!(!vm.is_empty() && !vl.is_empty());
    assert_ne!(vm, vl);
    m.set_theme(Some(CrtTheme::default().into()));
    assert_eq!(m.theme().name(), "crt");
    let vc = m.view(Some(&g));
    assert!(!vc.is_empty() && vc != vm);
    m.set_theme(None);
    assert_eq!(m.theme().name(), "modern");
    lipgloss::set_color_profile(Profile::Ascii);
}

#[test]
fn nil_game_and_defaults() {
    setup();
    let m = new(None, &[]);
    assert_eq!(m.theme().name(), "modern");
    assert!(!m.view(None).is_empty());
}

#[test]
fn status_panel_radar_lrs_damaged() {
    setup();
    let mut m = new(modern(), &[30, 16]);
    let mut ent = Enterprise {
        quad: [4, 4],
        sector: [2, 3],
        energy: 4500.0,
        shields: 1000.0,
        torpedoes: 8,
        condition: ConditionType::Green,
        ..Default::default()
    };
    ent.devices[lrs()] = 2.5;
    let mut chart = [[0; 9]; 9];
    chart[4][4] = 105;
    chart[3][4] = 203;
    m.set_state(ent, 25.0, 10, 3, false, chart, None);
    let view = m.view(None);
    assert!(view.contains("RADAR (±1) [LRS OFFLINE]:"), "{view}");
    assert!(view.contains("???"));
    assert!(view.contains("105"));
}

#[test]
fn status_panel_radar_lrs_damaged_docked() {
    setup();
    let mut m = new(modern(), &[30, 16]);
    let mut ent = Enterprise {
        quad: [4, 4],
        sector: [2, 3],
        energy: 4500.0,
        shields: 1000.0,
        torpedoes: 8,
        condition: ConditionType::Docked,
        ..Default::default()
    };
    ent.devices[lrs()] = 2.5;
    let mut chart = [[0; 9]; 9];
    chart[4][4] = 105;
    chart[3][4] = 203;
    m.set_state(ent, 25.0, 10, 3, true, chart, None);
    let view = m.view(None);
    assert!(!view.contains("[LRS OFFLINE]"));
    assert!(view.contains("203"));
}

#[test]
fn status_panel_height_and_line_budget() {
    setup();
    let m = new(modern(), &[]);
    let mut g = game();
    assert_eq!(m.view(Some(&g)).split('\n').count(), 16);
    g.enterprise.devices[lrs()] = 3.0;
    assert_eq!(m.view(Some(&g)).split('\n').count(), 16);
}

#[test]
fn radar_degradation_two_tier() {
    setup();
    let panel = new(modern(), &[]);
    let mut d = sample_panel_data();
    d.rules = default_rules_for_profile(&DifficultyProfile::normal());
    d.devices[lrs()] = 1.0;
    assert!(panel.render(&d).contains("[LRS DEGRADED]"));
    d.devices[lrs()] = 2.5;
    assert!(panel.render(&d).contains("[LRS OFFLINE]"));
}

#[test]
fn red_alert_badge_pulse() {
    setup();
    lipgloss::set_color_profile(Profile::TrueColor);
    let mut m = new(modern(), &[]);
    let mut g = game();
    g.enterprise.condition = ConditionType::Red;
    g.rules.anim_speed = ANIM_SPEED_NORMAL;
    m.set_red_alert_cycle(0);
    let v0 = m.view(Some(&g));
    m.set_red_alert_cycle(1);
    let v1 = m.view(Some(&g));
    assert!(v0.contains("CONDITION RED") && v1.contains("CONDITION RED"));
    assert_ne!(v0, v1);
    m.set_red_alert_cycle(2);
    assert_eq!(m.view(Some(&g)), v0);
    lipgloss::set_color_profile(Profile::Ascii);
}

#[test]
fn red_alert_badge_anim_speed_off() {
    setup();
    lipgloss::set_color_profile(Profile::TrueColor);
    let mut m = new(modern(), &[]);
    let mut g = game();
    g.enterprise.condition = ConditionType::Red;
    g.rules.anim_speed = ANIM_SPEED_OFF;
    m.set_red_alert_cycle(0);
    let v0 = m.view(Some(&g));
    m.set_red_alert_cycle(1);
    assert_eq!(m.view(Some(&g)), v0);
    lipgloss::set_color_profile(Profile::Ascii);
}

#[test]
fn audio_telemetry_badge() {
    setup();
    let mut sp = new(dt(), &[]);
    sp.set_audio_telemetry(80, false);
    assert!(sp.view(None).contains("80%"));
    sp.set_audio_telemetry(0, true);
    let v = sp.view(None);
    assert!(v.contains("MUTED") || v.contains("OFF"), "{v}");
}
