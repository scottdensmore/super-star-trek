//! Port of `pkg/tui/components/targetlock/targetlock_test.go`.

use sst_engine::ptr;
use sst_tuikit::lipgloss;
use sst_tuikit::tea::{msg, KeyType};
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

fn kl(id: i32, sector: Coord, energy: f64) -> Ptr<Klingon> {
    ptr(Klingon {
        id,
        sector,
        energy,
        ..Default::default()
    })
}

fn key(t: KeyType) -> Msg {
    msg(KeyMsg::new(t))
}

fn runes(s: &str) -> Msg {
    msg(KeyMsg::runes(s))
}

fn call(c: Option<Cmd>) -> Msg {
    c.expect("cmd").call().expect("msg")
}

#[test]
fn ballistics_math() {
    setup();
    let mut m = new(dt());
    let ent = [4, 4];
    m.set_state(ent, 5000.0, 10, &[kl(1, [4, 7], 400.0)], [0, 0]);
    let t = m.current_target().expect("target");
    assert!((t.distance - 3.0).abs() <= 1e-4);
    assert!((t.bearing - 1.0).abs() <= 1e-4);
    assert!((t.hit_probability - 0.80).abs() <= 1e-4);

    let h = 18f64.sqrt();
    for (name, pos, dist, bearing) in [
        ("Due North", [1, 4], 3.0, 3.0),
        ("Due West", [4, 1], 3.0, 5.0),
        ("Due South", [7, 4], 3.0, 7.0),
        ("North-East", [1, 7], h, 2.0),
        ("North-West", [1, 1], h, 4.0),
        ("South-West", [7, 1], h, 6.0),
        ("South-East", [7, 7], h, 8.0),
    ] {
        m.set_state(ent, 5000.0, 10, &[kl(2, pos, 400.0)], [0, 0]);
        let c = m.current_target().expect(name);
        assert!((c.distance - dist).abs() <= 1e-4, "{name}");
        assert!((c.bearing - bearing).abs() <= 1e-4, "{name}: {}", c.bearing);
    }

    m.set_state(ent, 5000.0, 10, &[kl(3, ent, 400.0)], [0, 0]);
    assert_eq!(m.current_target().unwrap().hit_probability, 0.95);
    m.set_state([1, 1], 5000.0, 10, &[kl(4, [16, 16], 400.0)], [0, 0]);
    assert_eq!(m.current_target().unwrap().hit_probability, 0.10);
}

#[test]
fn target_cycling() {
    setup();
    let mut m = new(dt());
    let ent = [4, 4];
    let (k1, k2, k3) = (
        kl(10, [4, 6], 300.0),
        kl(20, [4, 7], 400.0),
        kl(30, [1, 1], 500.0),
    );
    m.set_state(
        ent,
        5000.0,
        10,
        &[k3.clone(), k1.clone(), k2.clone()],
        [0, 0],
    );
    let ids: Vec<i32> = m.targets().iter().map(|t| t.klingon_id).collect();
    assert_eq!(ids, [10, 20, 30]);
    let cur = |m: &Model| m.current_target().map(|t| t.klingon_id);
    assert_eq!(cur(&m), Some(10));
    assert!(m.update(&key(KEY_TAB)).is_none());
    assert_eq!(cur(&m), Some(20));
    let _ = m.update(&key(KEY_RIGHT));
    assert_eq!(cur(&m), Some(30));
    let _ = m.update(&key(KEY_TAB));
    assert_eq!(cur(&m), Some(10));
    let _ = m.update(&key(KEY_LEFT));
    assert_eq!(cur(&m), Some(30));
    let _ = m.update(&key(KEY_SHIFT_TAB));
    assert_eq!(cur(&m), Some(20));
    m.set_state(ent, 5000.0, 10, &[k1, k2, k3], [4, 7]);
    assert_eq!(cur(&m), Some(20));
}

#[test]
fn fire_torpedo() {
    setup();
    let mut m = new(dt());
    m.set_state([4, 4], 5000.0, 8, &[kl(1, [4, 7], 400.0)], [0, 0]);
    let r = call(m.update(&key(KEY_ENTER)));
    let f = r.downcast_ref::<FireTorpedoMsg>().expect("FireTorpedoMsg");
    assert_eq!(f.target, [4, 7]);
    assert!((f.bearing - 1.0).abs() <= 1e-4);
    assert_eq!(m.warning_message(), "");
}

#[test]
fn zero_torpedoes_warning() {
    setup();
    let mut m = new(dt());
    m.set_state([4, 4], 5000.0, 0, &[kl(1, [4, 7], 400.0)], [0, 0]);
    assert!(m.update(&key(KEY_ENTER)).is_none());
    assert!(m.warning_message().contains("NO TORPEDOES"));
    assert!(m.view().contains("NO TORPEDOES"));
}

#[test]
fn fire_phasers_prompt() {
    setup();
    let mut m = new(dt());
    m.set_state([4, 4], 5000.0, 10, &[kl(1, [4, 7], 400.0)], [0, 0]);
    assert!(!m.inputting_phaser());
    assert!(m.update(&runes("p")).is_none());
    assert!(m.inputting_phaser());
    assert!(m.view().contains("PHASER"));
    for d in ["3", "5", "0"] {
        let _ = m.update(&runes(d));
    }
    let r = call(m.update(&key(KEY_ENTER)));
    assert_eq!(r.downcast_ref::<FirePhasersMsg>().unwrap().energy, 350.0);
    assert!(!m.inputting_phaser());

    for k in ["P", "5", "0"] {
        let _ = m.update(&runes(k));
    }
    let _ = m.update(&key(KEY_BACKSPACE));
    let r = call(m.update(&key(KEY_ENTER)));
    assert_eq!(r.downcast_ref::<FirePhasersMsg>().unwrap().energy, 5.0);

    let _ = m.update(&runes("p"));
    assert!(m.inputting_phaser());
    let _ = m.update(&runes("p"));
    assert!(!m.inputting_phaser());
}

#[test]
fn phaser_warning_cleared_on_typing() {
    setup();
    let mut m = new(dt());
    m.set_state([4, 4], 5000.0, 10, &[kl(1, [4, 7], 400.0)], [0, 0]);
    let _ = m.update(&runes("p"));
    assert!(m.update(&key(KEY_ENTER)).is_none());
    assert!(m.warning_message().contains("INVALID ENERGY"));
    assert!(m.view().contains("INVALID ENERGY"));
    let _ = m.update(&runes("2"));
    assert_eq!(m.warning_message(), "");
    let v = m.view();
    assert!(!v.contains("INVALID ENERGY"));
    assert!(v.contains("PHASER ENERGY> 2"), "{v}");
    let _ = m.update(&key(KEY_BACKSPACE));
    let _ = m.update(&key(KEY_ENTER));
    assert!(!m.warning_message().is_empty());
    let _ = m.update(&key(KEY_BACKSPACE));
    assert_eq!(m.warning_message(), "");
}

#[test]
fn close() {
    setup();
    let mut m = new(dt());
    m.set_state([4, 4], 5000.0, 10, &[], [0, 0]);
    assert!(call(m.update(&key(KEY_ESC)))
        .downcast_ref::<CloseHUDMsg>()
        .is_some());
}

#[test]
fn view_dimensions_and_layout() {
    setup();
    let mut m = new(dt());
    m.set_state([4, 4], 3450.0, 8, &[kl(1, [3, 6], 280.0)], [0, 0]);
    let view = m.view();
    for e in [
        "TACTICAL TARGET LOCK",
        "KLINGON",
        "Sector [3, 6]",
        "Range:",
        "Bearing:",
        "Hit Prob",
        "[TORP: 8/10]",
        "[ENERGY: 3450]",
        "[Enter]",
        "[P]",
        "[Tab]",
        "[Esc]",
    ] {
        assert!(view.contains(e), "missing {e}:\n{view}");
    }
    let lines: Vec<&str> = view.trim_end_matches('\n').split('\n').collect();
    assert_eq!(lines.len(), 12);
    for l in lines {
        assert_eq!(lipgloss::width(l), 52, "{l:?}");
    }
}

#[test]
fn no_targets() {
    setup();
    let mut m = new(dt());
    m.set_state([4, 4], 5000.0, 10, &[], [0, 0]);
    assert!(m.current_target().is_none());
    assert!(m.view().contains("NO HOSTILE"));
    assert!(m.update(&key(KEY_ENTER)).is_none());
    assert!(call(m.update(&key(KEY_ESC)))
        .downcast_ref::<CloseHUDMsg>()
        .is_some());
}

#[test]
fn set_theme() {
    setup();
    let mut m = new(dt());
    m.set_theme(Some(theme::get_theme("lcars")));
    let v1 = m.view();
    m.set_theme(Some(theme::get_theme("crt")));
    let v2 = m.view();
    assert!(!v1.is_empty() && !v2.is_empty());
}
