//! Port of pkg/engine/combat_test.go.

use sst_engine::json::GoVec;
use sst_engine::*;

fn kref(k: Klingon) -> KlingonRef {
    temp_klingon(k)
}

fn set_klingons(g: &mut GameState, ks: &[&KlingonRef]) {
    g.current_quad.klingons = GoVec::from_vec(ks.iter().map(|k| Some((*k).clone())).collect());
}

#[test]
fn phaser_attenuation() {
    let d1 = compute_phaser_damage(1000.0, 1.0);
    let d2 = compute_phaser_damage(1000.0, 2.0);
    assert!(
        d2 < d1,
        "expected phaser damage to fall off with distance: d1={d1}, d2={d2}"
    );
    let d3 = compute_phaser_damage(2000.0, 2.0);
    assert!((d3 - d1).abs() <= 0.0001);
    let d_zero = compute_phaser_damage(100.0, 0.0);
    let d_neg = compute_phaser_damage(100.0, -1.0);
    let expected = 100.0 * (1.0 / 0.1);
    assert!((d_zero - expected).abs() <= 0.0001);
    assert!((d_neg - expected).abs() <= 0.0001);
    assert_eq!(compute_phaser_damage(0.0, 5.0), 0.0);
    assert_eq!(compute_phaser_damage(-50.0, 5.0), 0.0);
}

#[test]
fn torpedo_path_tracing() {
    // hit directly right east
    let mut quad = QuadrantState::default();
    quad.grid[4][7] = EntityType::Klingon;
    assert_eq!(
        trace_torpedo_path([4, 1], 0.0, Some(&quad)),
        ([4, 7], EntityType::Klingon, true)
    );

    // hit directly north up
    let mut quad = QuadrantState::default();
    quad.grid[2][3] = EntityType::Star;
    assert_eq!(
        trace_torpedo_path([6, 3], gomath::PI / 2.0, Some(&quad)),
        ([2, 3], EntityType::Star, true)
    );

    // hit directly west left
    let mut quad = QuadrantState::default();
    quad.grid[3][2] = EntityType::Starbase;
    assert_eq!(
        trace_torpedo_path([3, 6], gomath::PI, Some(&quad)),
        ([3, 2], EntityType::Starbase, true)
    );

    // hit directly south down (Go constant 3*math.Pi/2)
    let mut quad = QuadrantState::default();
    quad.grid[7][5] = EntityType::Planet;
    let three_pi_2 = 3.0 * gomath::PI / 2.0;
    assert_eq!(
        trace_torpedo_path([2, 5], three_pi_2, Some(&quad)),
        ([7, 5], EntityType::Planet, true)
    );

    // hit northeast diagonal
    let mut quad = QuadrantState::default();
    quad.grid[2][5] = EntityType::Commander;
    let angle = bearing([5, 2], [2, 5]);
    assert_eq!(
        trace_torpedo_path([5, 2], angle, Some(&quad)),
        ([2, 5], EntityType::Commander, true)
    );

    // obstacle in path hit first
    let mut quad = QuadrantState::default();
    quad.grid[4][4] = EntityType::Star;
    quad.grid[4][7] = EntityType::Klingon;
    assert_eq!(
        trace_torpedo_path([4, 1], 0.0, Some(&quad)),
        ([4, 4], EntityType::Star, true)
    );

    // miss exits quadrant
    let quad = QuadrantState::default();
    assert_eq!(
        trace_torpedo_path([4, 1], 0.0, Some(&quad)),
        ([0, 0], EntityType::Empty, false)
    );

    // nil quadrant or out of bounds
    assert_eq!(
        trace_torpedo_path([4, 1], 0.0, None),
        ([0, 0], EntityType::Empty, false)
    );
    for o in [[0, 4], [9, 4], [4, 0], [4, 9]] {
        assert_eq!(
            trace_torpedo_path(o, 0.0, Some(&quad)),
            ([0, 0], EntityType::Empty, false),
            "origin {o:?}"
        );
    }
}

fn ent(energy: f64, shields: f64) -> Enterprise {
    Enterprise {
        energy,
        shields,
        ..Default::default()
    }
}

#[test]
fn resolve_shield_hit_cases() {
    // shields absorb all damage
    let mut e = ent(3000.0, 1000.0);
    assert_eq!(resolve_shield_hit(&mut e, 400.0), (400.0, 0.0));
    assert_eq!((e.shields, e.energy), (600.0, 3000.0));

    // shields absorb exact damage
    let mut e = ent(2500.0, 500.0);
    assert_eq!(resolve_shield_hit(&mut e, 500.0), (500.0, 0.0));
    assert_eq!((e.shields, e.energy), (0.0, 2500.0));

    // partial absorption
    let mut e = ent(2000.0, 300.0);
    assert_eq!(resolve_shield_hit(&mut e, 500.0), (300.0, 200.0));
    assert_eq!((e.shields, e.energy), (0.0, 1800.0));

    // zero shields
    let mut e = ent(1000.0, 0.0);
    assert_eq!(resolve_shield_hit(&mut e, 250.0), (0.0, 250.0));
    assert_eq!((e.shields, e.energy), (0.0, 750.0));

    // negative shields clamped
    let mut e = ent(1000.0, -50.0);
    assert_eq!(resolve_shield_hit(&mut e, 100.0), (0.0, 100.0));
    assert_eq!((e.shields, e.energy), (0.0, 900.0));

    // zero or negative damage
    let mut e = ent(1000.0, 500.0);
    assert_eq!(resolve_shield_hit(&mut e, 0.0), (0.0, 0.0));
    assert_eq!(resolve_shield_hit(&mut e, -50.0), (0.0, 0.0));
    assert_eq!((e.shields, e.energy), (500.0, 1000.0));
    // "nil enterprise" sub-test: not representable (takes &mut Enterprise).
}

#[test]
fn klingon_cloaking_lock_lockout_and_decloak() {
    let rules = default_rules_for_profile(&DifficultyProfile::hardcore());
    let mut g = GameState::new_game_with_options(100, SkillLevel::Good, GameLength::Medium, rules);
    let k = kref(Klingon {
        id: 1,
        sector: [4, 4],
        energy: 400.0,
        is_commander: true,
        is_cloaked: true,
        ..Default::default()
    });
    set_klingons(&mut g, &[&k]);
    g.current_quad.grid[4][4] = EntityType::Commander;

    let action = ActionTorpedoDirect {
        target_sector: [4, 4],
    };
    let err = action.execute(&mut g).unwrap_err();
    assert!(err.0.contains("CLOAKED"), "got {err}");

    let events = decloak_klingon(Some(&k));
    assert!(!k.borrow().is_cloaked);
    assert!(!events.is_empty());
    match &events[0] {
        Event::KlingonCloakState(ev) => {
            assert!(!ev.cloaked);
            assert_eq!(ev.klingon_id, 1);
        }
        other => panic!("unexpected {other:?}"),
    }
    assert_eq!(events[0].event_type(), "KlingonCloakState");

    assert!(decloak_klingon(Some(&k)).is_empty());

    if let Err(e) = action.execute(&mut g) {
        assert!(!e.0.contains("CLOAKED"), "unexpected cloaked error {e}");
    }
}

#[test]
fn repair_multiplier() {
    let rules = GameRules {
        repair_multiplier: 2.0,
        ..Default::default()
    };
    let mut g = GameState::new_game_with_options(100, SkillLevel::Good, GameLength::Medium, rules);
    let w = DeviceID::Warp.index();
    g.enterprise.devices[w] = 4.0;
    advance_repairs(&mut g, 1.0);
    assert_eq!(g.enterprise.devices[w], 3.5);
    advance_repairs(&mut g, 10.0);
    assert_eq!(g.enterprise.devices[w], 0.0);
    g.enterprise.devices[w] = 2.0;
    advance_repairs(&mut g, 0.0);
    advance_repairs(&mut g, -1.0);
    assert_eq!(g.enterprise.devices[w], 2.0);
    g.rules.repair_multiplier = 0.0;
    advance_repairs(&mut g, 0.5);
    assert_eq!(g.enterprise.devices[w], 1.5);
    // AdvanceRepairs(nil, ...) not representable.
}

#[test]
fn cloak_klingon_success_and_guards() {
    let rules = GameRules {
        klingon_cloak: true,
        ..Default::default()
    };
    let g = GameState::new_game_with_options(100, SkillLevel::Good, GameLength::Medium, rules);
    let cmd = kref(Klingon {
        id: 1,
        is_commander: true,
        ..Default::default()
    });
    let events = cloak_klingon(Some(&g), Some(&cmd));
    assert!(cmd.borrow().is_cloaked);
    assert_eq!(events.len(), 1);
    match &events[0] {
        Event::KlingonCloakState(ev) => assert!(ev.cloaked && ev.klingon_id == 1),
        other => panic!("unexpected {other:?}"),
    }
    assert!(cloak_klingon(Some(&g), Some(&cmd)).is_empty());

    let regular = kref(Klingon {
        id: 2,
        ..Default::default()
    });
    assert!(cloak_klingon(Some(&g), Some(&regular)).is_empty());
    assert!(!regular.borrow().is_cloaked);

    let g2 = GameState::new_game_with_options(
        100,
        SkillLevel::Good,
        GameLength::Medium,
        GameRules::default(),
    );
    let cmd2 = kref(Klingon {
        id: 3,
        is_commander: true,
        ..Default::default()
    });
    assert!(cloak_klingon(Some(&g2), Some(&cmd2)).is_empty());
    assert!(!cmd2.borrow().is_cloaked);

    assert!(cloak_klingon(Some(&g), None).is_empty());
}

#[test]
fn klingon_counter_attack_decloaks_attacker() {
    let rules = GameRules {
        klingon_cloak: true,
        ..Default::default()
    };
    let mut g = GameState::new_game_with_options(100, SkillLevel::Good, GameLength::Medium, rules);
    g.enterprise.shields = 500.0;
    g.enterprise.energy = 3000.0;
    let cmd = kref(Klingon {
        id: 1,
        is_commander: true,
        is_cloaked: true,
        energy: 400.0,
        ..Default::default()
    });
    let events = klingon_counter_attack(&mut g, &cmd, 150.0);
    assert!(!cmd.borrow().is_cloaked);
    assert_eq!(events.len(), 2, "{events:?}");
    match &events[0] {
        Event::KlingonCloakState(ev) => assert!(!ev.cloaked && ev.klingon_id == 1),
        other => panic!("unexpected {other:?}"),
    }
    match &events[1] {
        Event::KlingonCounterAttack(ev) => assert!(ev.enemy_id == 1 && ev.damage == 150.0),
        other => panic!("unexpected {other:?}"),
    }
    assert_eq!(g.enterprise.shields, 350.0);

    let events2 = klingon_counter_attack(&mut g, &cmd, 100.0);
    assert_eq!(events2.len(), 1);
    assert!(matches!(events2[0], Event::KlingonCounterAttack(_)));
}

#[test]
fn klingon_combat_zero_shields_in_nebula() {
    let mut g = GameState::new_game(10, SkillLevel::Good, GameLength::Medium);
    let q = g.enterprise.quad;
    g.quadrant_env[q[0] as usize][q[1] as usize] = EnvironmentType::Nebula;
    let k = kref(Klingon {
        id: 1,
        sector: [4, 5],
        energy: 400.0,
        shields: 100.0,
        ..Default::default()
    });
    set_klingons(&mut g, &[&k]);
    g.current_quad.grid[4][5] = EntityType::Klingon;
    ActionFirePhasers {
        energy: 200.0,
        ..Default::default()
    }
    .execute(&mut g)
    .unwrap();
    assert!(k.borrow().energy < 400.0);
    assert_eq!(k.borrow().shields, 0.0);
}

#[test]
fn klingon_combat_absorbed_by_black_hole() {
    let mut g = GameState::new_game(10, SkillLevel::Good, GameLength::Medium);
    g.current_quad.grid[4][4] = EntityType::BlackHole;
    let k = kref(Klingon {
        id: 1,
        sector: [4, 3],
        energy: 400.0,
        ..Default::default()
    });
    set_klingons(&mut g, &[&k]);
    g.current_quad.grid[4][3] = EntityType::Klingon;
    let events = move_klingon(&mut g, &k, [4, 4]);
    let mut found = false;
    for e in &events {
        if let Event::SingularityAbsorption(sa) = e {
            found = true;
            assert_eq!(sa.target, EntityType::Klingon);
            assert_eq!(sa.sector, [4, 4]);
        }
    }
    assert!(found);
    assert!(g.current_quad.klingons.is_empty());
    assert_eq!(g.current_quad.grid[4][3], EntityType::Empty);
}

#[test]
fn klingon_turn_symmetry_and_black_hole() {
    let mut g = GameState::new_game(10, SkillLevel::Good, GameLength::Medium);
    let q = g.enterprise.quad;
    g.quadrant_env[q[0] as usize][q[1] as usize] = EnvironmentType::Nebula;
    let k = kref(Klingon {
        id: 1,
        sector: [4, 3],
        energy: 400.0,
        shields: 200.0,
        ..Default::default()
    });
    set_klingons(&mut g, &[&k]);
    g.current_quad.grid[4][3] = EntityType::Klingon;
    klingon_turn(&mut g);
    assert_eq!(k.borrow().shields, 0.0);
}

#[test]
fn klingon_combat_commander_absorbed_by_black_hole() {
    let mut g = GameState::new_game(10, SkillLevel::Good, GameLength::Medium);
    g.current_quad.grid[5][5] = EntityType::BlackHole;
    let cmd = kref(Klingon {
        id: 1,
        sector: [5, 4],
        energy: 600.0,
        is_commander: true,
        ..Default::default()
    });
    set_klingons(&mut g, &[&cmd]);
    g.current_quad.grid[5][4] = EntityType::Commander;
    let events = move_klingon(&mut g, &cmd, [5, 5]);
    assert_eq!(events.len(), 1);
    let Event::SingularityAbsorption(sa) = &events[0] else {
        panic!("expected EventSingularityAbsorption, got {:?}", events[0]);
    };
    assert_eq!(sa.target, EntityType::Commander);
    assert_eq!(g.metrics.commanders_killed, 1);
    assert!(g.current_quad.klingons.is_empty());
}

#[test]
fn klingon_turn_black_hole_absorption() {
    let mut g = GameState::new_game(10, SkillLevel::Good, GameLength::Medium);
    g.current_quad.grid[4][4] = EntityType::BlackHole;
    let k = kref(Klingon {
        id: 1,
        sector: [4, 4],
        energy: 400.0,
        ..Default::default()
    });
    set_klingons(&mut g, &[&k]);
    let events = klingon_turn(&mut g);
    assert_eq!(events.len(), 1);
    match &events[0] {
        Event::SingularityAbsorption(sa) => assert_eq!(sa.sector, [4, 4]),
        other => panic!("unexpected {other:?}"),
    }
    assert!(g.current_quad.klingons.is_empty());
}

#[test]
fn move_klingon_rejects_occupied_sector() {
    let mut g = GameState::new_game(10, SkillLevel::Good, GameLength::Medium);
    let k = kref(Klingon {
        id: 1,
        sector: [4, 3],
        energy: 400.0,
        ..Default::default()
    });
    set_klingons(&mut g, &[&k]);
    g.current_quad.grid[4][3] = EntityType::Klingon;
    g.current_quad.grid[4][4] = EntityType::Star;
    let events = move_klingon(&mut g, &k, [4, 4]);
    assert!(events.is_empty());
    assert_eq!(k.borrow().sector, [4, 3]);
    assert_eq!(g.current_quad.grid[4][4], EntityType::Star);
}

#[test]
fn move_klingon_preserves_super_commander() {
    let mut g = GameState::new_game(10, SkillLevel::Good, GameLength::Medium);
    let sc = kref(Klingon {
        id: 1,
        sector: [3, 3],
        energy: 1500.0,
        is_commander: true,
        ..Default::default()
    });
    set_klingons(&mut g, &[&sc]);
    g.current_quad.grid[3][3] = EntityType::SuperCommander;
    assert!(move_klingon(&mut g, &sc, [3, 4]).is_empty());
    assert_eq!(g.current_quad.grid[3][4], EntityType::SuperCommander);
    assert_eq!(g.current_quad.grid[3][3], EntityType::Empty);

    g.current_quad.grid[3][5] = EntityType::BlackHole;
    let events2 = move_klingon(&mut g, &sc, [3, 5]);
    assert_eq!(events2.len(), 1);
    match &events2[0] {
        Event::SingularityAbsorption(sa) => assert_eq!(sa.target, EntityType::SuperCommander),
        other => panic!("unexpected {other:?}"),
    }
    assert_eq!(g.metrics.super_commanders_killed, 1);
}
