//! Ports of `modal_test.go` (scenario leaderboards come from the built-in
//! defaults through the injected loader instead of the user's config dir).

use sst_engine::leaderboard::default_scenario_leaderboard;
use sst_tuikit::tea::{self, KeyMsg};

use super::*;
use crate::theme::get_theme;

fn loader() -> LeaderboardLoader {
    Rc::new(|id: &ScenarioID| Ok(default_scenario_leaderboard(id)))
}

fn model(name: &str) -> Model {
    crate::theme::detect::set_fixed_detection(true);
    new_with_loader(get_theme(name), 72, 18, loader())
}

fn key(t: tea::KeyType) -> Msg {
    tea::msg(KeyMsg::new(t))
}

fn runes(s: &str) -> Msg {
    tea::msg(KeyMsg::runes(s))
}

fn sel(m: &Model) -> ScenarioID {
    m.selected_scenario().unwrap().id.clone()
}

#[test]
fn navigation_and_selection() {
    let mut m = model("modern");
    m.set_dimensions(80, 24);
    assert_eq!(sel(&m), ScenarioID::kobayashi_maru());
    let _ = m.update(&key(tea::KEY_DOWN));
    assert_eq!(sel(&m), ScenarioID::mutara_nebula());
    let msg = m.update(&key(tea::KEY_ENTER)).unwrap().call().unwrap();
    let launch = msg.downcast_ref::<MsgLaunchScenario>().unwrap();
    assert_eq!(launch.scenario_id, ScenarioID::mutara_nebula());
}

#[test]
fn navigation_keys() {
    let mut m = model("modern");
    let steps: Vec<(Msg, ScenarioID)> = vec![
        (key(tea::KEY_UP), ScenarioID::starbase_siege()),
        (key(tea::KEY_DOWN), ScenarioID::kobayashi_maru()),
        (runes("j"), ScenarioID::mutara_nebula()),
        (runes("k"), ScenarioID::kobayashi_maru()),
    ];
    for (msg, want) in steps {
        let _ = m.update(&msg);
        assert_eq!(sel(&m), want);
    }
}

#[test]
fn dismiss_keys() {
    let mut m = model("modern");
    for msg in [key(tea::KEY_ESC), runes("q"), runes("Q")] {
        let r = m.update(&msg).unwrap().call().unwrap();
        assert!(r.downcast_ref::<MsgCloseScenarioModal>().is_some());
    }
}

#[test]
fn view_rendering() {
    let mut m = model("modern");
    let v = m.view();
    for s in [
        "Kobayashi Maru",
        "Mutara Nebula",
        "[EXTREME]",
        "[HARD]",
        "[CHALLENGE]",
        "BRIEFING",
        "CONSTRAINTS",
        "James T. Kirk",
    ] {
        assert!(v.contains(s), "missing {s}:\n{v}");
    }
    let _ = m.update(&key(tea::KEY_DOWN));
    let v2 = m.view();
    assert!(v2.contains("MUTARA NEBULA"));
    assert!(v2.contains("Admiral James T. Kirk"));
    // The cloaked commander cannot be locked on to; the constraints say how
    // to fight it.
    assert!(v2.contains("No sensor lock: torpedo its last bearing"));
    let _ = m.update(&key(tea::KEY_DOWN));
    let v3 = m.view();
    assert!(v3.contains("STARBASE"));
    assert!(v3.contains("Hikaru Sulu"));
}

#[test]
fn theme_and_dimensions() {
    let mut m = model("lcars");
    m.set_theme(get_theme("crt"));
    m.set_dimensions(80, 24);
    m.set_size(80, 24);
    let v = m.view();
    assert_eq!(v.trim_end_matches('\n').split('\n').count(), 18);
}

#[test]
fn empty_scenarios() {
    crate::theme::detect::set_fixed_detection(true);
    let mut m = Model::default();
    assert!(m.selected_scenario().is_none());
    assert!(m.update(&key(tea::KEY_ENTER)).is_none());
    assert!(!m.view().is_empty());
}
