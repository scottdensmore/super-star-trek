//! Port of pkg/engine/tactics_test.go.

use sst_engine::json::GoVec;
use sst_engine::*;

fn ev(id: i32, faction: FactionType, sector: Coord, energy: f64) -> EnemyVessel {
    EnemyVessel {
        id,
        faction,
        sector,
        energy,
        ..Default::default()
    }
}

#[test]
fn tactics_unified_turn_deterministic_order() {
    let mut g = GameState::new_game_with_seed(3333);
    let r = EnemyVessel {
        is_cloaked: true,
        ..ev(1, FactionType::Romulan, [1, 1], 1000.0)
    };
    let t1 = ev(2, FactionType::Tholian, [8, 8], 800.0);
    let k = ev(3, FactionType::Klingon, [4, 4], 600.0);
    g.current_quad.enemies = GoVec::from_vec(vec![ptr(r), ptr(t1), ptr(k)]);
    sync_quadrant_enemies(&mut g.current_quad);
    assert!(!unified_adversary_turn(&mut g).is_empty());
}

#[test]
fn tactics_point_defense_phaser_interception() {
    let mut g = GameState::new_game_with_seed(4444);
    let target = g.enterprise.sector;
    g.current_quad.plasma_torpedoes = GoVec::from_vec(vec![ptr(PlasmaTorpedo {
        id: 50,
        sector: [3, 3],
        energy: 500.0,
        target_sector: target,
        ..Default::default()
    })]);
    g.current_quad.grid[3][3] = EntityType::PlasmaTorpedo;
    let events = ActionPhaserDirect {
        target_sector: [3, 3],
        energy: 600.0,
    }
    .execute(&mut g)
    .unwrap();
    assert!(g.current_quad.plasma_torpedoes.is_empty());
    assert!(!events.is_empty());
    assert_eq!(g.metrics.romulans_surrendered, 1);
}

#[test]
fn tactics_point_defense_phaser_web() {
    let mut g = GameState::new_game_with_seed(5555);
    g.enterprise.energy = 1000.0;
    g.current_quad.web_segments = GoVec::from_vec(vec![ptr(TholianWebSegment {
        coord: [2, 2],
        strength: 250.0,
    })]);
    g.current_quad.grid[2][2] = EntityType::TholianWeb;

    let events = ActionPhaserDirect {
        target_sector: [2, 2],
        energy: 100.0,
    }
    .execute(&mut g)
    .unwrap();
    assert!(events.is_empty());
    assert_eq!(g.current_quad.web_segments.len(), 1);
    assert_eq!(
        g.current_quad.web_segments[0]
            .as_ref()
            .unwrap()
            .borrow()
            .strength,
        150.0
    );
    assert_eq!(g.current_quad.grid[2][2], EntityType::TholianWeb);

    let events = ActionPhaserDirect {
        target_sector: [2, 2],
        energy: 200.0,
    }
    .execute(&mut g)
    .unwrap();
    assert!(!events.is_empty());
    assert!(g.current_quad.web_segments.is_empty());
    assert_eq!(g.current_quad.grid[2][2], EntityType::Empty);
}

#[test]
fn tactics_point_defense_phaser_vessel() {
    let mut g = GameState::new_game_with_seed(6666);
    g.enterprise.sector = [4, 4];
    g.enterprise.energy = 2000.0;
    let r = ptr(EnemyVessel {
        shields: 50.0,
        max_energy: 500.0,
        ..ev(10, FactionType::Romulan, [4, 6], 100.0)
    });
    g.current_quad.enemies = GoVec::from_vec(vec![r.clone()]);
    g.current_quad.grid[4][6] = EntityType::Romulan;
    ActionPhaserDirect {
        target_sector: [4, 6],
        energy: 500.0,
    }
    .execute(&mut g)
    .unwrap();
    assert!(r.unwrap().borrow().energy <= 0.0);
    assert_eq!(g.current_quad.grid[4][6], EntityType::Empty);
    assert_eq!(g.metrics.romulans_killed, 1);
}

#[test]
fn tactics_action_phaser_direct_validation() {
    // Execute(nil) is not representable.
    let mut g = GameState::new_game_with_seed(7777);
    g.enterprise.energy = 100.0;
    let cases: [(Coord, f64); 7] = [
        ([1, 1], 0.0),
        ([1, 1], -50.0),
        ([1, 1], 500.0),
        ([0, 1], 50.0),
        ([9, 1], 50.0),
        ([1, 0], 50.0),
        ([1, 9], 50.0),
    ];
    for (t, e) in cases {
        let r = ActionPhaserDirect {
            target_sector: t,
            energy: e,
        }
        .execute(&mut g);
        assert!(r.is_err(), "expected error for {t:?} {e}");
    }
}

#[test]
fn tactics_unified_turn_defensive_and_edge_cases() {
    // UnifiedAdversaryTurn(nil) is not representable.
    let mut g = GameState::new_game_with_seed(8888);
    g.current_quad.enemies = GoVec::from_vec(vec![
        None,
        ptr(ev(1, FactionType::Klingon, [2, 2], 0.0)),
        ptr(ev(2, FactionType::Romulan, [3, 3], 0.0)),
        None,
    ]);
    assert!(unified_adversary_turn(&mut g).is_empty());
}

#[test]
fn tactics_compute_distance() {
    assert_eq!(compute_distance([1, 1], [4, 5]), distance([1, 1], [4, 5]));
}

#[test]
fn tactics_unified_turn_all_phases_sequence() {
    let mut g = GameState::new_game_with_seed(1234);
    g.enterprise.sector = [4, 1];
    g.enterprise.energy = 5000.0;
    g.enterprise.shields = 2000.0;
    let target = g.enterprise.sector;
    g.current_quad.plasma_torpedoes = GoVec::from_vec(vec![ptr(PlasmaTorpedo {
        id: 99,
        sector: [7, 7],
        energy: 600.0,
        target_sector: target,
        turns_in_flight: 0,
        ..Default::default()
    })]);
    g.current_quad.grid[7][7] = EntityType::PlasmaTorpedo;
    let cmd = EnemyVessel {
        is_commander: true,
        ..ev(10, FactionType::Klingon, [4, 6], 500.0)
    };
    let raider = ev(11, FactionType::Klingon, [3, 4], 400.0);
    g.current_quad.grid[4][6] = EntityType::Commander;
    g.current_quad.grid[3][4] = EntityType::Klingon;
    let rom = EnemyVessel {
        is_cloaked: true,
        cloak_turns: 0,
        ..ev(20, FactionType::Romulan, [1, 1], 800.0)
    };
    let th = EnemyVessel {
        special_state: 0,
        ..ev(30, FactionType::Tholian, [8, 8], 600.0)
    };
    g.current_quad.grid[8][8] = EntityType::Tholian;
    g.current_quad.enemies = GoVec::from_vec(vec![ptr(cmd), ptr(raider), ptr(rom), ptr(th)]);
    sync_quadrant_enemies(&mut g.current_quad);

    let events = unified_adversary_turn(&mut g);
    let first = |t: &str| events.iter().position(|e| e.event_type() == t);
    let plasma = first("PlasmaMoved").expect("expected PlasmaMoved event in phase 1");
    let screen = first("KlingonScreening").expect("expected KlingonScreening event in phase 2");
    let attack =
        first("KlingonCounterAttack").expect("expected KlingonCounterAttack event in phase 2");
    let romulan = first("RomulanDecloak").expect("expected RomulanDecloak event in phase 3");
    let tholian = first("WebSegmentLaid").expect("expected WebSegmentLaid event in phase 4");
    assert!(
        plasma < screen && screen < attack && attack < romulan && romulan < tholian,
        "plasma={plasma}, screen={screen}, attack={attack}, romulan={romulan}, tholian={tholian}"
    );
}

#[test]
fn unified_adversary_turn_decloak() {
    let mut g = GameState::new_game_with_seed(123);
    g.enterprise.sector = [1, 1];
    let k = EnemyVessel {
        is_cloaked: true,
        ..ev(1, FactionType::Klingon, [4, 4], 500.0)
    };
    g.current_quad.enemies = GoVec::from_vec(vec![ptr(k)]);
    let events = unified_adversary_turn(&mut g);
    assert!(
        !g.current_quad.enemies[0]
            .as_ref()
            .unwrap()
            .borrow()
            .is_cloaked,
        "Expected Klingon to be decloaked after firing"
    );
    // Go only logs when no firing event is found.
    let _fired = events
        .iter()
        .any(|e| e.event_type() == "KlingonCounterAttack" || e.event_type() == "KlingonDecloak");
}
