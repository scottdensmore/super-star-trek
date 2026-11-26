//! Port of pkg/engine/scenarios_test.go.
#![allow(clippy::needless_range_loop)]

use sst_engine::json::GoVec;
use sst_engine::*;

fn sc(id: ScenarioID) -> std::sync::Arc<Scenario> {
    get_scenario(&id).unwrap_or_else(|| panic!("failed to find scenario {id}"))
}

fn eval(s: &Scenario, g: &mut GameState) -> (bool, bool, GameOverReason) {
    (s.evaluate.unwrap())(g)
}

fn score(s: &Scenario, g: &GameState, won: bool) -> ScoreBreakdown {
    (s.compute_score.unwrap())(g, won)
}

#[test]
fn scenario_registry_lookup_and_aliases() {
    let km = ScenarioID::kobayashi_maru();
    let mn = ScenarioID::mutara_nebula();
    let ss = ScenarioID::starbase_siege();
    let cases = [
        ("kobayashi-maru", km.clone(), true),
        ("kobayashi", km.clone(), true),
        ("km", km.clone(), true),
        ("Kobayashi-Maru", km.clone(), true),
        ("KM", km.clone(), true),
        ("mutara-nebula", mn.clone(), true),
        ("mutara", mn.clone(), true),
        ("nebula", mn.clone(), true),
        ("MUTARA", mn.clone(), true),
        ("starbase-siege", ss.clone(), true),
        ("siege", ss.clone(), true),
        ("starbase", ss.clone(), true),
        ("Starbase-Siege", ss.clone(), true),
        ("unknown-scenario", ScenarioID::none(), false),
        ("", ScenarioID::none(), false),
    ];
    for (input, expected, found) in cases {
        let s = get_scenario(&ScenarioID::new(input));
        assert_eq!(s.is_some(), found, "GetScenario({input:?})");
        if let Some(s) = s {
            assert_eq!(s.id, expected, "GetScenario({input:?})");
        }
    }
}

#[test]
fn list_scenarios_ordering() {
    let list = list_scenarios();
    assert!(
        list.len() >= 3,
        "expected at least 3 scenarios, got {}",
        list.len()
    );
    let expected = [
        ScenarioID::kobayashi_maru(),
        ScenarioID::mutara_nebula(),
        ScenarioID::starbase_siege(),
    ];
    for (i, id) in expected.iter().enumerate() {
        assert_eq!(&list[i].id, id, "scenario index {i}");
    }
}

#[test]
fn register_scenario_custom() {
    struct Restore(Option<RegistrySnapshot>);
    impl Drop for Restore {
        fn drop(&mut self) {
            if let Some(s) = self.0.take() {
                restore_registry(s);
            }
        }
    }
    let _guard = Restore(Some(snapshot_registry()));

    let custom_id = ScenarioID::new("custom-challenge");
    register_scenario(Scenario {
        id: custom_id.clone(),
        name: "Custom Challenge".into(),
        subtitle: "Testing Registry".into(),
        difficulty: "Test".into(),
        description: "A test scenario".into(),
        aliases: GoVec::from_vec(vec!["custom".into(), "cc".into()]),
        ..Default::default()
    });
    let s = get_scenario(&custom_id).expect("expected custom scenario to be found");
    assert_eq!(s.name, "Custom Challenge");
    let a = get_scenario(&ScenarioID::new("cc")).expect("alias cc");
    assert_eq!(a.id, custom_id);
}

#[test]
fn scenario_kobayashi_maru_mechanics() {
    let s = sc(ScenarioID::kobayashi_maru());
    let mut g = s.build(42);
    assert_eq!(g.scenario, ScenarioID::kobayashi_maru());
    assert_eq!(g.enterprise.quad, [4, 4]);
    assert_eq!(g.current_quad.klingons.len(), 3);
    g.metrics.klingons_killed = 4;
    g.stardate = g.initial_stardate + 3.0;
    assert_eq!(
        score(&s, &g, false).rank_title,
        "[COMM-2] Commendation for Tactical Excellence"
    );
}

#[test]
fn scenario_kobayashi_maru_wave_reinforcement() {
    let s = sc(ScenarioID::kobayashi_maru());
    let mut g = s.build(42);
    assert_eq!(g.current_quad.klingons.len(), 3);
    let k1 = g.current_quad.klingons[0].clone();
    let k1_sector = k1.as_ref().unwrap().borrow().sector;
    g.current_quad.klingons = GoVec::from_vec(vec![k1]);
    for r in 1..=8 {
        for c in 1..=8 {
            if g.current_quad.grid[r][c] == EntityType::Klingon && [r as i32, c as i32] != k1_sector
            {
                g.current_quad.grid[r][c] = EntityType::Empty;
            }
        }
    }
    let (done, won, _) = eval(&s, &mut g);
    assert!(
        !done && !won,
        "expected game to continue, got done={done} won={won}"
    );
    assert_eq!(g.current_quad.klingons.len(), 3);
}

#[test]
fn scenario_kobayashi_maru_commendation_tiers() {
    let s = sc(ScenarioID::kobayashi_maru());
    let cases = [
        (1, 1.0, "[COMM-1] Cadet Commendation for Valor", "[COMM-1]"),
        (
            4,
            3.0,
            "[COMM-2] Commendation for Tactical Excellence",
            "[COMM-2]",
        ),
        (7, 5.0, "[COMM-3] Starfleet Cross of Honor", "[COMM-3]"),
        (
            11,
            2.0,
            "[COMM-4] Admiral's Citation for Gallantry",
            "[COMM-4]",
        ),
        (
            2,
            9.0,
            "[COMM-4] Admiral's Citation for Gallantry",
            "[COMM-4]",
        ),
    ];
    for (kills, elapsed, title, badge) in cases {
        let mut g = s.build(42);
        g.metrics.klingons_killed = kills;
        g.stardate = g.initial_stardate + elapsed;
        let sb = score(&s, &g, false);
        assert_eq!(sb.rank_title, title, "kills={kills} elapsed={elapsed}");
        assert_eq!(sb.rank_badge, badge, "kills={kills} elapsed={elapsed}");
    }
}

#[test]
fn scenario_mutara_nebula_mechanics() {
    let s = sc(ScenarioID::mutara_nebula());
    let mut g = s.build(42);
    assert_eq!(g.enterprise.quad, [5, 5]);
    assert_eq!(g.quadrant_env[5][5], EnvironmentType::Nebula);
    assert_eq!(g.enterprise.shields, 0.0);
    g.metrics.super_commanders_killed = 1;
    let (done, won, _) = eval(&s, &mut g);
    assert!(done && won);
}

#[test]
fn scenario_starbase_siege_mechanics() {
    let s = sc(ScenarioID::starbase_siege());
    let mut g = s.build(42);
    assert_eq!(g.enterprise.quad, [2, 2]);
    assert_eq!(g.time_remaining, 6.0);
    g.remaining_starbases = 0;
    let (done, won, reason) = eval(&s, &mut g);
    assert!(done && !won && reason == GameOverReason::Lost);
}

#[test]
fn scenario_starbase_siege_victory() {
    let s = sc(ScenarioID::starbase_siege());
    let mut g = s.build(42);
    g.metrics.klingons_killed = 3;
    g.remaining_klingons = 0;
    let (done, won, reason) = eval(&s, &mut g);
    assert!(done && won && reason == GameOverReason::Won);
}

#[test]
fn scenario_action_move_turn_hook() {
    let s = sc(ScenarioID::mutara_nebula());
    let mut g = s.build(42);
    g.metrics.super_commanders_killed = 1;
    let mut dest = [g.enterprise.sector[0], g.enterprise.sector[1] + 1];
    if dest[1] > 8 {
        dest[1] = g.enterprise.sector[1] - 1;
    }
    g.current_quad.grid[dest[0] as usize][dest[1] as usize] = EntityType::Empty;
    let act = ActionMove {
        dest_sector: dest,
        warp: 0.1,
        ..Default::default()
    };
    let events = act.execute(&mut g).expect("ActionMove");
    let mut found = false;
    for ev in &events {
        if let Event::GameOver(go) = ev {
            found = true;
            assert_eq!(go.reason, GameOverReason::Won);
        }
    }
    assert!(found, "expected EventGameOver");
    assert!(g.game_won);
}

#[test]
fn scenario_mutara_nebula_fleeing_does_not_win() {
    let s = sc(ScenarioID::mutara_nebula());
    let mut g = s.build(42);
    g.enterprise.quad = [5, 6];
    g.current_quad = QuadrantState::default();
    let (done, won, reason) = eval(&s, &mut g);
    assert!(!won);
    assert_ne!(reason, GameOverReason::Won);
    assert!(done && reason == GameOverReason::Lost);
}

#[test]
fn scenario_mutara_nebula_galaxy_chart_only_one_klingon() {
    let s = sc(ScenarioID::mutara_nebula());
    let g = s.build(42);
    let mut total = 0;
    for r in 1..=8 {
        for c in 1..=8 {
            total += g.galaxy_chart[r][c] / 100;
        }
    }
    assert_eq!(total, 1);
}
