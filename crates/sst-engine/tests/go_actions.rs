//! Port of pkg/engine/actions_test.go.
//!
//! Skipped: `TestActionMove_NilGameState` (Go `Execute(nil)`; the Rust API
//! takes `&mut GameState`, so a nil state is unrepresentable).
#![allow(clippy::needless_range_loop)] // loops mirror the Go indexing

use std::collections::BTreeMap;

use sst_engine::json::GoVec;
use sst_engine::*;

fn kref(k: Klingon) -> (Ptr<Klingon>, KlingonRef) {
    let p = ptr(k);
    let r = p.clone().unwrap();
    (p, r)
}

fn rules_surv(s: SurveillanceMode) -> GameRules {
    GameRules {
        surveillance: s,
        ..Default::default()
    }
}

fn surveillance(e: &Event) -> EventStarbaseSurveillance {
    match e {
        Event::StarbaseSurveillance(s) => s.clone(),
        other => panic!("expected StarbaseSurveillance, got {other:?}"),
    }
}

fn clear_discovered(g: &mut GameState) {
    for r in 1..=8 {
        for c in 1..=8 {
            g.chart_discovered[r][c] = false;
        }
    }
}

#[test]
fn test_action_dock_surveillance_modes() {
    let mut gf = GameState::new_game_with_options(
        100,
        SkillLevel::Good,
        GameLength::Medium,
        rules_surv(SurveillanceMode::full()),
    );
    gf.current_quad.starbase = Some([4, 4]);
    gf.enterprise.sector = [4, 5];
    let ev = ActionDock.execute(&mut gf).unwrap();
    let n = (1..=8)
        .flat_map(|r| (1..=8).map(move |c| (r, c)))
        .filter(|&(r, c)| gf.chart_discovered[r][c])
        .count();
    assert_eq!(n, 64);
    assert_eq!(surveillance(&ev[1]).mode, SurveillanceMode::full());

    let mut gl = GameState::new_game_with_options(
        200,
        SkillLevel::Good,
        GameLength::Medium,
        rules_surv(SurveillanceMode::local()),
    );
    clear_discovered(&mut gl);
    gl.enterprise.quad = [2, 2];
    gl.current_quad.starbase = Some([5, 5]);
    gl.enterprise.sector = [5, 6];
    let ev = ActionDock.execute(&mut gl).unwrap();
    for r in 1..=8 {
        for c in 1..=8 {
            let inside = (1..=3).contains(&r) && (1..=3).contains(&c);
            assert_eq!(gl.chart_discovered[r][c], inside, "[{r},{c}]");
        }
    }
    assert_eq!(surveillance(&ev[1]).mode, SurveillanceMode::local());

    let mut gb = GameState::new_game_with_options(
        300,
        SkillLevel::Good,
        GameLength::Medium,
        rules_surv(SurveillanceMode::blackout()),
    );
    clear_discovered(&mut gb);
    gb.enterprise.quad = [3, 3];
    gb.current_quad.starbase = Some([1, 1]);
    gb.enterprise.sector = [1, 2];
    let ev = ActionDock.execute(&mut gb).unwrap();
    let s = surveillance(&ev[1]);
    assert_eq!(s.updated_quads, 0);
    assert_eq!(s.mode, SurveillanceMode::blackout());
}

#[test]
fn test_action_lr_scan_degraded_warning() {
    let mut g = GameState::new_game_with_options(
        100,
        SkillLevel::Good,
        GameLength::Medium,
        default_rules_for_profile(&DifficultyProfile::normal()),
    );
    g.enterprise.devices[DeviceID::LRSensors.index()] = 1.2;
    let ev = ActionLRScan.execute(&mut g).unwrap();
    assert!(ev
        .iter()
        .any(|e| matches!(e, Event::LRScanCompleted(l) if l.degraded)));
}

fn cloak_events(ev: &[Event]) -> (bool, bool) {
    let mut d1 = false;
    let mut c2 = false;
    for e in ev {
        if let Event::KlingonCloakState(c) = e {
            if c.klingon_id == 1 && !c.cloaked {
                d1 = true;
            }
            if c.klingon_id == 2 && c.cloaked {
                c2 = true;
            }
        }
    }
    (d1, c2)
}

#[test]
fn test_action_fire_phasers_decloak_on_hit_and_cloak_on_evade() {
    let rules = GameRules {
        klingon_cloak: true,
        ..Default::default()
    };
    let mut g = GameState::new_game_with_options(100, SkillLevel::Good, GameLength::Medium, rules);
    g.enterprise.energy = 3000.0;
    g.enterprise.sector = [1, 1];
    let (p1, c1) = kref(Klingon {
        id: 1,
        sector: [2, 2],
        energy: 500.0,
        is_commander: true,
        is_cloaked: true,
        ..Default::default()
    });
    let (p2, c2) = kref(Klingon {
        id: 2,
        sector: [3, 3],
        energy: 500.0,
        is_commander: true,
        is_cloaked: false,
        ..Default::default()
    });
    g.current_quad.klingons = GoVec::from_vec(vec![p1, p2]);
    g.current_quad.grid[2][2] = EntityType::Commander;
    g.current_quad.grid[3][3] = EntityType::Commander;
    let a = ActionFirePhasers {
        energy: 0.0,
        manual_allocation: BTreeMap::from([(1, 200.0), (2, 0.0)]),
    };
    let ev = a.execute(&mut g).unwrap();
    assert!(!c1.borrow().is_cloaked, "cmd1 should decloak");
    assert!(c2.borrow().is_cloaked, "cmd2 should cloak");
    let (d1, cl2) = cloak_events(&ev);
    assert!(d1, "decloak event for cmd1");
    assert!(cl2, "cloak event for cmd2");
}

#[test]
fn test_action_fire_torpedo_decloak_on_hit_and_cloak_on_evade() {
    let rules = GameRules {
        klingon_cloak: true,
        ..Default::default()
    };
    let mut g = GameState::new_game_with_options(100, SkillLevel::Good, GameLength::Medium, rules);
    g.enterprise.torpedoes = 10;
    g.enterprise.sector = [4, 1];
    let (p1, c1) = kref(Klingon {
        id: 1,
        sector: [4, 5],
        energy: 800.0,
        is_commander: true,
        is_cloaked: true,
        ..Default::default()
    });
    let (p2, c2) = kref(Klingon {
        id: 2,
        sector: [7, 7],
        energy: 800.0,
        is_commander: true,
        is_cloaked: false,
        ..Default::default()
    });
    g.current_quad.klingons = GoVec::from_vec(vec![p1, p2]);
    g.current_quad.grid[4][5] = EntityType::Commander;
    g.current_quad.grid[7][7] = EntityType::Commander;
    let ev = ActionFireTorpedo {
        angle: 0.0,
        ..Default::default()
    }
    .execute(&mut g)
    .unwrap();
    assert!(!c1.borrow().is_cloaked);
    assert!(c2.borrow().is_cloaked);
    let (d1, cl2) = cloak_events(&ev);
    assert!(d1 && cl2, "{ev:?}");
}

#[test]
fn test_action_klingon_counter_attack_execute() {
    let rules = GameRules {
        klingon_cloak: true,
        ..Default::default()
    };
    let mut g = GameState::new_game_with_options(100, SkillLevel::Good, GameLength::Medium, rules);
    g.enterprise.shields = 400.0;
    g.enterprise.energy = 2500.0;
    let (p, cmd) = kref(Klingon {
        id: 5,
        sector: [3, 3],
        energy: 500.0,
        is_commander: true,
        is_cloaked: true,
        ..Default::default()
    });
    g.current_quad.klingons = GoVec::from_vec(vec![p]);
    let ev = ActionKlingonCounterAttack {
        enemy_id: 5,
        damage: 120.0,
    }
    .execute(&mut g)
    .unwrap();
    assert!(!cmd.borrow().is_cloaked);
    assert_eq!(ev.len(), 2);
    assert!(
        matches!(&ev[0], Event::KlingonCloakState(c) if !c.cloaked && c.klingon_id == 5),
        "{:?}",
        ev[0]
    );
    assert!(
        matches!(&ev[1], Event::KlingonCounterAttack(a) if a.enemy_id == 5 && a.damage == 120.0),
        "{:?}",
        ev[1]
    );
    assert!(ActionKlingonCounterAttack {
        enemy_id: 999,
        damage: 50.0
    }
    .execute(&mut g)
    .is_err());
    assert!(ActionKlingonCounterAttack {
        enemy_id: 5,
        damage: 0.0
    }
    .execute(&mut g)
    .is_err());
}

fn new10() -> GameState {
    GameState::new_game(10, SkillLevel::Good, GameLength::Medium)
}

fn game_over_reasons(ev: &[Event]) -> Vec<GameOverReason> {
    ev.iter()
        .filter_map(|e| match e {
            Event::GameOver(g) => Some(g.reason),
            _ => None,
        })
        .collect()
}

#[test]
fn test_action_move_black_hole_fatal_collision() {
    let mut g = new10();
    g.enterprise.sector = [4, 4];
    g.current_quad.grid[4][4] = EntityType::Enterprise;
    g.current_quad.grid[4][5] = EntityType::BlackHole;
    g.current_quad.grid[3][5] = EntityType::BlackHole;
    let ev = ActionMove {
        course: 1.0,
        warp: 0.125,
        ..Default::default()
    }
    .execute(&mut g)
    .unwrap();
    let r = game_over_reasons(&ev);
    assert!(!r.is_empty(), "expected EventGameOver");
    assert!(r.iter().all(|x| *x == GameOverReason::Lost), "{r:?}");
}

#[test]
fn test_action_move_black_hole_gravity_well_energy_cost() {
    let mut g = new10();
    g.enterprise.sector = [4, 4];
    g.current_quad.grid[4][4] = EntityType::Enterprise;
    g.current_quad.grid[5][5] = EntityType::BlackHole;
    let e0 = g.enterprise.energy;
    ActionMove {
        dest_sector: [3, 4],
        warp: 0.1,
        ..Default::default()
    }
    .execute(&mut g)
    .unwrap();
    let used = e0 - g.enterprise.energy;
    assert!((used - 0.2).abs() <= 0.001, "used {used}");
}

#[test]
fn test_action_move_wormhole_jump() {
    let mut g = new10();
    g.enterprise.quad = [2, 2];
    g.enterprise.sector = [4, 4];
    g.current_quad.grid[4][4] = EntityType::Enterprise;
    g.current_quad.grid[4][5] = EntityType::Wormhole;
    let ev = ActionMove {
        dest_sector: [4, 5],
        warp: 0.1,
        ..Default::default()
    }
    .execute(&mut g)
    .unwrap();
    let mut found = false;
    for e in &ev {
        if let Event::WormholeJump(j) = e {
            found = true;
            assert!(j.from_quad == [2, 2] && j.from_sector == [4, 4], "{j:?}");
        }
    }
    assert!(found);
    assert!(!(g.enterprise.quad == [2, 2] && g.enterprise.sector == [4, 5]));
}

fn ion_storm_game(rng_nil: bool) -> GameState {
    let rules = default_rules_for_profile(&DifficultyProfile::hardcore());
    let mut g =
        GameState::new_game_with_options(12345, SkillLevel::Good, GameLength::Medium, rules);
    g.enterprise.quad = [1, 1];
    g.enterprise.sector = [4, 1];
    g.current_quad.grid[4][1] = EntityType::Enterprise;
    g.quadrant_env[1][1] = EnvironmentType::IonStorm;
    if rng_nil {
        g.rng = None;
    }
    for c in 2..=8 {
        for r in 1..=8 {
            g.current_quad.grid[r][c] = EntityType::Empty;
        }
    }
    g
}

fn has_drift(ev: &[Event]) -> bool {
    ev.iter()
        .any(|e| matches!(e, Event::HazardTriggered(h) if h.hazard_type == "ion_storm_drift"))
}

#[test]
fn test_action_move_ion_storm_drift() {
    let mut g = ion_storm_game(false);
    let ev = ActionMove {
        course: 0.0,
        warp: 0.625,
        ..Default::default()
    }
    .execute(&mut g)
    .unwrap();
    assert!(has_drift(&ev), "{ev:?}");
}

#[test]
fn test_action_move_black_hole_fatal_collision_dest_sector() {
    let mut g = new10();
    g.enterprise.sector = [4, 4];
    g.current_quad.grid[4][4] = EntityType::Enterprise;
    g.current_quad.grid[4][5] = EntityType::BlackHole;
    let ev = ActionMove {
        dest_sector: [4, 5],
        warp: 0.1,
        ..Default::default()
    }
    .execute(&mut g)
    .unwrap();
    let mut absorbed = false;
    for e in &ev {
        if let Event::SingularityAbsorption(s) = e {
            absorbed = true;
            assert!(
                s.sector == [4, 5] && s.target == EntityType::Enterprise && s.weapon == "ship",
                "{s:?}"
            );
        }
    }
    assert!(absorbed);
    assert_eq!(game_over_reasons(&ev), vec![GameOverReason::Lost]);
}

#[test]
fn test_action_move_black_hole_gravity_well_energy_cost_moving_into() {
    let mut g = new10();
    g.enterprise.sector = [1, 1];
    g.current_quad.grid[1][1] = EntityType::Enterprise;
    g.current_quad.grid[3][3] = EntityType::BlackHole;
    let e0 = g.enterprise.energy;
    ActionMove {
        dest_sector: [2, 2],
        warp: 0.1,
        ..Default::default()
    }
    .execute(&mut g)
    .unwrap();
    let used = e0 - g.enterprise.energy;
    assert!((used - 0.2).abs() <= 0.001, "used {used}");
}

#[test]
fn test_action_move_wormhole_jump_vector() {
    let mut g = new10();
    g.enterprise.quad = [2, 2];
    g.enterprise.sector = [4, 4];
    g.current_quad.grid[4][4] = EntityType::Enterprise;
    g.current_quad.grid[4][5] = EntityType::Wormhole;
    let ev = ActionMove {
        course: 0.0,
        warp: 0.125,
        ..Default::default()
    }
    .execute(&mut g)
    .unwrap();
    let mut found = false;
    for e in &ev {
        if let Event::WormholeJump(j) = e {
            found = true;
            assert!(j.from_quad == [2, 2] && j.from_sector == [4, 4], "{j:?}");
        }
    }
    assert!(found);
}

#[test]
fn test_action_move_mid_flight_gravity_well_depletion() {
    let mut g = new10();
    g.enterprise.sector = [4, 1];
    g.current_quad.grid[4][1] = EntityType::Enterprise;
    g.current_quad.grid[4][4] = EntityType::BlackHole;
    g.enterprise.energy = 0.3;
    let ev = ActionMove {
        course: 0.0,
        warp: 0.25,
        ..Default::default()
    }
    .execute(&mut g)
    .unwrap();
    assert!(g.enterprise.energy >= 0.0);
    assert_eq!(g.enterprise.energy, 0.0);
    let r = game_over_reasons(&ev);
    assert!(!r.is_empty());
    assert!(r.iter().all(|x| *x == GameOverReason::Energy), "{r:?}");
}

#[test]
fn test_action_move_mid_flight_gravity_well_sufficient_energy() {
    let mut g = new10();
    g.enterprise.sector = [4, 1];
    g.current_quad.grid[4][1] = EntityType::Enterprise;
    g.current_quad.grid[4][4] = EntityType::BlackHole;
    g.enterprise.energy = 100.0;
    ActionMove {
        course: 0.0,
        warp: 0.25,
        ..Default::default()
    }
    .execute(&mut g)
    .unwrap();
    assert!(
        (g.enterprise.energy - 99.5).abs() <= 0.001,
        "{}",
        g.enterprise.energy
    );
}

#[test]
fn test_action_shields_blocked_in_nebula() {
    let mut g = new10();
    let q = g.enterprise.quad;
    g.quadrant_env[q[0] as usize][q[1] as usize] = EnvironmentType::Nebula;
    let err = ActionShields { amount: 500.0 }.execute(&mut g).unwrap_err();
    assert_eq!(
        err.to_string(),
        "sensors indicate extreme particle ionization: shields cannot hold cohesive geometry in nebula"
    );
    assert_eq!(g.enterprise.shields, 0.0);
}

#[test]
fn test_action_fire_torpedo_absorbed_by_black_hole() {
    let mut g = new10();
    g.enterprise.sector = [4, 1];
    g.current_quad.grid[4][1] = EntityType::Enterprise;
    g.current_quad.grid[4][4] = EntityType::BlackHole;
    let (p, k) = kref(Klingon {
        id: 1,
        sector: [4, 7],
        energy: 500.0,
        ..Default::default()
    });
    g.current_quad.klingons = GoVec::from_vec(vec![p]);
    g.current_quad.grid[4][7] = EntityType::Klingon;
    let ev = ActionFireTorpedo {
        direction: 1.0,
        ..Default::default()
    }
    .execute(&mut g)
    .unwrap();
    let mut found = false;
    for e in &ev {
        if let Event::SingularityAbsorption(s) = e {
            found = true;
            assert_eq!(s.weapon, "torpedo");
            assert_eq!(s.sector, [4, 4]);
            assert_eq!(s.target, EntityType::BlackHole);
        }
    }
    assert!(found);
    assert!(k.borrow().energy >= 500.0);
}

fn nebula_target(g: &GameState) -> Coord {
    let from = g.enterprise.quad;
    let mut to = [from[0] + 1, from[1]];
    if to[0] > 8 {
        to[0] = from[0] - 1;
    }
    to
}

#[test]
fn test_nebula_shield_discharge() {
    let mut g = new10();
    let to = nebula_target(&g);
    g.quadrant_env[to[0] as usize][to[1] as usize] = EnvironmentType::Nebula;
    g.enterprise.energy = 4000.0;
    g.enterprise.shields = 500.0;
    let ev = ActionMove {
        dest_quad: to,
        dest_sector: [4, 4],
        warp: 1.0,
        ..Default::default()
    }
    .execute(&mut g)
    .unwrap();
    assert_eq!(g.enterprise.shields, 0.0);
    assert!(g.enterprise.energy >= 4400.0, "{}", g.enterprise.energy);
    let mut found = false;
    for e in &ev {
        if let Event::AnomalyDiscovered(a) = e {
            found = true;
            assert!(a.quad == to && a.env == EnvironmentType::Nebula);
        }
    }
    assert!(found);
}

#[test]
fn test_nebula_lrs_masking() {
    let mut g = new10();
    g.enterprise.quad = [3, 3];
    g.quadrant_env[3][4] = EnvironmentType::Nebula;
    let ev = ActionLRScan.execute(&mut g).unwrap();
    let mut found = false;
    for e in &ev {
        if let Event::LRScanCompleted(s) = e {
            found = true;
            assert_eq!(s.readings[1][2], -1);
        }
    }
    assert!(found);
}

#[test]
fn test_nebula_shield_discharge_clamped_max_energy() {
    let mut g = new10();
    let to = nebula_target(&g);
    g.quadrant_env[to[0] as usize][to[1] as usize] = EnvironmentType::Nebula;
    g.enterprise.energy = 4900.0;
    g.enterprise.shields = 500.0;
    ActionMove {
        dest_quad: to,
        dest_sector: [4, 4],
        warp: 1.0,
        ..Default::default()
    }
    .execute(&mut g)
    .unwrap();
    assert_eq!(g.enterprise.shields, 0.0);
    assert_eq!(g.enterprise.energy, 5000.0);
}

#[test]
fn test_scan_quadrant_helper() {
    let mut g = new10();
    g.quadrant_env[2][3] = EnvironmentType::Nebula;
    g.galaxy_chart[2][4] = 205;
    assert_eq!(scan_quadrant(Some(&g), 2, 3), -1);
    assert_eq!(scan_quadrant(Some(&g), 2, 4), 205);
    assert_eq!(scan_quadrant(Some(&g), 0, 4), -1);
    assert_eq!(scan_quadrant(Some(&g), 2, 9), -1);
    assert_eq!(ActionLRScan.read_quadrant(Some(&g), 2, 3), -1);
    assert_eq!(scan_quadrant(None, 2, 4), -1);
    assert_eq!(ActionLRScan.read_quadrant(None, 2, 4), -1);
}

#[test]
fn test_action_move_wormhole_jump_nil_rng() {
    let mut g = new10();
    g.enterprise.sector = [4, 4];
    g.current_quad.grid[4][4] = EntityType::Enterprise;
    g.current_quad.grid[4][5] = EntityType::Wormhole;
    g.rng = None;
    assert!(ActionMove {
        dest_sector: [4, 5],
        warp: 0.1,
        ..Default::default()
    }
    .execute(&mut g)
    .is_err());
}

#[test]
fn test_action_move_ion_storm_nil_rng() {
    let mut g = ion_storm_game(true);
    let ev = ActionMove {
        course: 0.0,
        warp: 0.625,
        ..Default::default()
    }
    .execute(&mut g)
    .unwrap();
    assert!(!has_drift(&ev));
}
