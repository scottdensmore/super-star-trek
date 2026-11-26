//! Port of `pkg/tui/components/damageschematic/schematic_test.go`.

use sst_tuikit::lipgloss;
use sst_tuikit::tea::{msg, KEY_ENTER, KEY_ESC, KEY_SPACE};
use sst_tuikit::termenv::Profile;

use super::*;
use crate::theme::detect;

fn setup() {
    detect::set_fixed_detection(true);
    lipgloss::set_color_profile(Profile::Ascii);
}

fn dt() -> Option<Theme> {
    Some(theme::default_theme())
}

fn dev(d: DeviceID) -> usize {
    d.as_i32() as usize
}

fn assert_dims(view: &str) {
    let lines: Vec<&str> = view.split('\n').collect();
    assert_eq!(lines.len(), 18);
    for (i, l) in lines.iter().enumerate() {
        assert_eq!(ansi::string_width(l), 66, "line {i}: {l:?}");
    }
}

#[test]
fn nominal_dimensions_and_content() {
    setup();
    let mut m = new(dt(), 66, 18);
    let ent = EnterpriseState {
        energy: 5000.0,
        shields: 2500.0,
        ..Default::default()
    };
    m.set_state(&ent, "GREEN", false, 1.0);
    let view = m.view();
    assert_dims(&view);
    assert!(view.contains("DAMAGE CONTROL SCHEMATIC"));
    assert!(view.contains("NCC-1701"));
    assert!(view.contains("SRS: OK"));
    assert!(view.contains("All other primary and tactical systems operational."));
}

#[test]
fn damaged_subsystems() {
    setup();
    let mut m = new(dt(), 66, 18);
    let mut ent = EnterpriseState::default();
    ent.devices[dev(DeviceID::Computer)] = 2.1;
    ent.devices[dev(DeviceID::PhotonTubes)] = 1.4;
    m.set_state(&ent, "YELLOW", false, 1.0);
    let view = m.view();
    assert_eq!(view.split('\n').count(), 18);
    for s in [
        "COMP: 2.1",
        "TUB: 1.4",
        "Library Computer",
        "No Chart/Nav",
        "Photon Tubes",
        "Tubes Locked",
    ] {
        assert!(view.contains(s), "missing {s}");
    }
}

#[test]
fn repair_multiplier_and_docked() {
    setup();
    let mut m = new(dt(), 66, 18);
    let mut ent = EnterpriseState::default();
    ent.devices[dev(DeviceID::Warp)] = 2.0;
    m.set_state(&ent, "RED", false, 1.5);
    let view = m.view();
    assert!(view.contains("3.0 SD"));
    assert!(view.contains("0.8 SD"));
}

#[test]
fn set_theme() {
    setup();
    let mut m = new(None, 0, 0);
    m.set_theme(Some(theme::get_theme("lcars")));
    assert_dims(&m.view());
}

#[test]
fn docked_and_overflow_devices() {
    setup();
    let mut m = new(dt(), 66, 18);
    let mut ent = EnterpriseState::default();
    for d in [
        DeviceID::Warp,
        DeviceID::SRSensors,
        DeviceID::LRSensors,
        DeviceID::Phasers,
        DeviceID::PhotonTubes,
    ] {
        ent.devices[dev(d)] = 1.0;
    }
    m.set_state(&ent, "GREEN", true, 1.0);
    let view = m.view();
    assert!(view.contains("ALERT STATUS: DOCKED"));
    assert!(view.contains("+2 more damaged subsystems"));
}

#[test]
fn update_dismiss_keys() {
    setup();
    let mut m = new(dt(), 66, 18);
    let keys = [
        KeyMsg::new(KEY_ESC),
        KeyMsg::new(KEY_ENTER),
        KeyMsg::runes("q"),
        KeyMsg::runes("Q"),
        KeyMsg::runes("d"),
        KeyMsg::runes("D"),
        KeyMsg::runes(" "),
        KeyMsg::new(KEY_SPACE),
    ];
    for k in keys {
        let label = k.string();
        let cmd = m
            .update(&msg(k))
            .unwrap_or_else(|| panic!("no cmd for {label:?}"));
        assert!(cmd
            .call()
            .unwrap()
            .downcast_ref::<CloseModalMsg>()
            .is_some());
    }
    assert!(m.update(&msg(KeyMsg::runes("x"))).is_none());
    assert!(m.update(&msg(())).is_none());
}

#[test]
fn damage_control_subsystem_impact() {
    setup();
    let mut m = new(dt(), 66, 18);
    let mut ent = EnterpriseState::default();
    ent.devices[dev(DeviceID::DamageControl)] = 1.0;
    m.set_state(&ent, "GREEN", false, 1.0);
    let view = m.view();
    assert!(view.contains("Repairs Slow"));
    for (i, l) in view.split('\n').enumerate() {
        assert_eq!(ansi::string_width(l), 66, "line {i}");
    }
}
