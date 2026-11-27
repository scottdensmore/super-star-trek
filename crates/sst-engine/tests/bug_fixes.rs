//! Regression tests for Go behaviour that the port first kept for parity and
//! then fixed (see testdata/README.md, "Intentional changes").

use sst_engine::json::GoVec;
use sst_engine::*;

fn enemy(id: i32, faction: FactionType, sector: Coord, energy: f64, shields: f64) -> EnemyVessel {
    EnemyVessel {
        id,
        faction,
        sector,
        energy,
        shields,
        max_energy: energy,
        ..Default::default()
    }
}

/// An empty quadrant with the Enterprise at [4,4] and the given hostiles.
fn quadrant_game(enemies: Vec<EnemyVessel>) -> GameState {
    let mut g = GameState::new_game_with_seed(77);
    g.current_quad = QuadrantState::default();
    g.enterprise.quad = [1, 1];
    g.enterprise.sector = [4, 4];
    g.enterprise.energy = 5000.0;
    g.enterprise.torpedoes = 10;
    g.current_quad.grid[4][4] = EntityType::Enterprise;
    for e in &enemies {
        let cell = match e.faction {
            FactionType::Klingon if e.is_commander => EntityType::Commander,
            FactionType::Klingon => EntityType::Klingon,
            FactionType::Tholian => EntityType::Tholian,
            _ if e.is_cloaked => EntityType::Empty,
            _ => EntityType::Romulan,
        };
        g.current_quad.grid[e.sector[0] as usize][e.sector[1] as usize] = cell;
    }
    g.current_quad.enemies = GoVec::from_vec(enemies.into_iter().map(ptr).collect());
    sync_quadrant_enemies(&mut g.current_quad);
    g.remaining_klingons = g.current_quad.enemies.len() as i32;
    g.klingons_remaining = g.remaining_klingons;
    g.rules.adversaries = true;
    g
}

fn torpedo_at(g: &mut GameState, target: Coord) -> Vec<Event> {
    Action::FireTorpedo(ActionFireTorpedo {
        target,
        ..Default::default()
    })
    .execute(g)
    .expect("torpedo")
}

fn game_over(events: &[Event]) -> Option<&EventGameOver> {
    events.iter().find_map(|e| match e {
        Event::GameOver(g) => Some(g),
        _ => None,
    })
}

fn enemy_by_id(g: &GameState, id: i32) -> Option<EnemyVessel> {
    g.current_quad
        .enemies
        .iter()
        .flatten()
        .find(|e| e.borrow().id == id)
        .map(|e| e.borrow().clone())
}

fn klingon_by_id(g: &GameState, id: i32) -> Option<Klingon> {
    g.current_quad
        .klingons
        .iter()
        .flatten()
        .find(|k| k.borrow().id == id)
        .map(|k| k.borrow().clone())
}

// 1. A won sector game clears the tour sector.
#[test]
fn tour_sector_is_cleared_by_a_won_game() {
    let mut t = TourState::new_at(42, 0);
    let g = t.start_current_sector(None).unwrap();
    {
        let mut g = g.borrow_mut();
        // Hostiles still on the counters (e.g. a scripted or scenario win).
        assert!(g.klingons_remaining > 0 && g.remaining_klingons > 0);
        g.game_over = true;
        g.game_over_reason = GameOverReason::Won;
        g.game_won = true;
    }
    let (cleared, failed, bounty) = t.evaluate_sector();
    assert!(cleared, "a won game must clear the sector");
    assert!(!failed);
    assert!(bounty > 0);
    assert!(t.active && !t.failed);
}

#[test]
fn tour_sector_still_fails_on_a_lost_game() {
    let mut t = TourState::new_at(42, 0);
    let g = t.start_current_sector(None).unwrap();
    {
        let mut g = g.borrow_mut();
        g.game_over = true;
        g.game_over_reason = GameOverReason::Destroyed;
    }
    assert_eq!(t.evaluate_sector(), (false, true, 0));
}

// 2. Torpedoes hit (and can destroy) a cloaked Romulan.
#[test]
fn torpedo_hits_a_cloaked_romulan_and_decloaks_it() {
    let mut t = TourState::new_at(42, 0);
    let gr = t.start_current_sector(None).unwrap();
    let mut g = gr.borrow_mut();
    let before = enemy_by_id(&g, 1).unwrap();
    assert!(before.is_cloaked);
    assert_eq!(g.current_quad.grid[2][3], EntityType::Empty);

    let events = torpedo_at(&mut g, before.sector);
    assert!(
        events.iter().any(|e| matches!(e,
            Event::RomulanDecloak(d) if d.romulan_id == 1 && d.sector == before.sector)),
        "expected a decloak naming Romulan 1: {events:?}"
    );
    assert!(events.iter().any(|e| matches!(e,
        Event::TorpedoHit(h) if h.entity == EntityType::Romulan && !h.destroyed)));
    let after = enemy_by_id(&g, 1).unwrap();
    assert!(!after.is_cloaked);
    assert_eq!(after.shields, 0.0);
    assert_eq!(after.energy, 900.0, "400 shields absorb 400 of 500");
    assert_eq!(g.current_quad.grid[2][3], EntityType::Romulan);

    // Two more hits (also while it is cloaked again) destroy it.
    g.current_quad.enemies[0]
        .as_ref()
        .unwrap()
        .borrow_mut()
        .is_cloaked = true;
    g.current_quad.grid[2][3] = EntityType::Empty;
    let _ = torpedo_at(&mut g, before.sector);
    assert_eq!(enemy_by_id(&g, 1).unwrap().energy, 400.0);
    let events = torpedo_at(&mut g, before.sector);
    assert!(events.iter().any(|e| matches!(e,
        Event::TorpedoHit(h) if h.entity == EntityType::Romulan && h.destroyed)));
    assert!(enemy_by_id(&g, 1).is_none());
    assert_eq!(g.metrics.romulans_killed, 1);
    assert_eq!(g.current_quad.grid[2][3], EntityType::Empty);
}

#[test]
fn torpedo_misses_open_space_next_to_a_cloaked_romulan() {
    let mut g = quadrant_game(vec![EnemyVessel {
        is_cloaked: true,
        cloak_turns: 2,
        ..enemy(5, FactionType::Romulan, [4, 7], 1000.0, 400.0)
    }]);
    let events = torpedo_at(&mut g, [2, 4]);
    assert!(!events.iter().any(|e| matches!(e, Event::TorpedoHit(_))));
    assert!(enemy_by_id(&g, 5).unwrap().is_cloaked);
}

// 3. Damage to Klingons survives the adversary turn, and kills stay dead.
#[test]
fn klingon_damage_survives_the_adversary_turn() {
    let mut g = quadrant_game(vec![
        enemy(1, FactionType::Klingon, [4, 6], 500.0, 0.0),
        enemy(2, FactionType::Tholian, [8, 8], 800.0, 300.0),
    ]);
    let events = Action::FirePhasers(ActionFirePhasers {
        energy: 400.0,
        ..Default::default()
    })
    .execute(&mut g)
    .unwrap();
    assert!(events.iter().any(|e| matches!(e, Event::PhaserHit(_))));
    let damaged = klingon_by_id(&g, 1).unwrap().energy;
    assert!(damaged < 500.0);

    let _ = unified_adversary_turn(&mut g);
    assert_eq!(enemy_by_id(&g, 1).unwrap().energy, damaged);
    assert_eq!(klingon_by_id(&g, 1).unwrap().energy, damaged);
}

#[test]
fn destroyed_klingon_does_not_come_back() {
    let mut g = quadrant_game(vec![
        enemy(1, FactionType::Klingon, [4, 6], 200.0, 0.0),
        enemy(2, FactionType::Klingon, [1, 1], 400.0, 0.0),
        enemy(3, FactionType::Tholian, [8, 8], 800.0, 300.0),
    ]);
    let events = torpedo_at(&mut g, [4, 6]);
    assert!(events.iter().any(|e| matches!(e,
        Event::TorpedoHit(h) if h.destroyed && h.entity == EntityType::Klingon)));
    assert!(enemy_by_id(&g, 1).is_none());

    let events = unified_adversary_turn(&mut g);
    assert!(
        klingon_by_id(&g, 1).is_none(),
        "destroyed Klingon resurrected"
    );
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, Event::KlingonCounterAttack(a) if a.enemy_id == 1)),
        "destroyed Klingon fired: {events:?}"
    );
    assert_eq!(g.current_quad.grid[4][6], EntityType::Empty);
}

#[test]
fn focused_phasers_on_a_klingon_reach_the_legacy_list() {
    let mut g = quadrant_game(vec![
        enemy(1, FactionType::Klingon, [4, 6], 500.0, 0.0),
        enemy(2, FactionType::Klingon, [1, 1], 300.0, 0.0),
        enemy(3, FactionType::Tholian, [8, 8], 800.0, 300.0),
    ]);
    let events = Action::PhaserDirect(ActionPhaserDirect {
        target_sector: [4, 6],
        energy: 200.0,
    })
    .execute(&mut g)
    .unwrap();
    assert!(events.iter().any(|e| matches!(e,
        Event::PhaserHit(h) if h.klingon_id == 1 && !h.destroyed)));
    let hit = enemy_by_id(&g, 1).unwrap().energy;
    assert!(hit < 500.0);
    assert_eq!(klingon_by_id(&g, 1).unwrap().energy, hit);

    let _ = Action::PhaserDirect(ActionPhaserDirect {
        target_sector: [4, 6],
        energy: 2000.0,
    })
    .execute(&mut g)
    .unwrap();
    assert!(enemy_by_id(&g, 1).is_none());
    assert!(klingon_by_id(&g, 1).is_none());
    assert_eq!(g.metrics.klingons_killed, 1);
}

#[test]
fn adversary_fire_that_drains_the_ship_ends_the_game() {
    let mut g = quadrant_game(vec![enemy(1, FactionType::Klingon, [4, 5], 900.0, 0.0)]);
    g.enterprise.energy = 50.0;
    let events = unified_adversary_turn(&mut g);
    let go = game_over(&events).expect("game over");
    assert_eq!(go.reason, GameOverReason::Lost);
    assert_eq!(g.enterprise.energy, 0.0);
}

// 4. Destroying the last Klingon wins a normal game.
#[test]
fn destroying_the_last_klingon_wins_a_normal_game() {
    let mut g = quadrant_game(vec![enemy(1, FactionType::Klingon, [4, 6], 100.0, 0.0)]);
    g.rules.adversaries = false;
    assert_eq!(g.scenario, ScenarioID::none());
    assert_eq!(g.remaining_klingons, 1);
    let events = torpedo_at(&mut g, [4, 6]);
    let go = game_over(&events).expect("victory");
    assert_eq!(go.reason, GameOverReason::Won);
    assert!(g.game_won);
    assert_eq!(g.remaining_klingons, 0);
    let classic = compute_score(Some(&g), true);
    assert!(classic.win_bonus > 0);
    assert_eq!(go.score, classic.total_score as f64);
}

#[test]
fn victory_by_phasers_and_not_before_the_last_kill() {
    let mut g = quadrant_game(vec![
        enemy(1, FactionType::Klingon, [4, 6], 100.0, 0.0),
        enemy(2, FactionType::Klingon, [4, 2], 100.0, 0.0),
    ]);
    g.rules.adversaries = false;
    g.remaining_klingons = 3; // one more elsewhere in the galaxy
    g.klingons_remaining = 3;
    let events = Action::FirePhasers(ActionFirePhasers {
        energy: 2000.0,
        ..Default::default()
    })
    .execute(&mut g)
    .unwrap();
    assert!(game_over(&events).is_none());
    assert_eq!(g.remaining_klingons, 1);
    assert!(!g.game_won);
}

#[test]
fn victory_check_leaves_scenarios_alone() {
    let mut g = quadrant_game(vec![]);
    g.remaining_klingons = 0;
    g.scenario = ScenarioID::kobayashi_maru();
    let mut events = Vec::new();
    evaluate_victory(&mut g, true, &mut events);
    assert!(events.is_empty() && !g.game_won);

    g.scenario = ScenarioID::none();
    evaluate_victory(&mut g, false, &mut events);
    assert!(events.is_empty(), "no kill this turn, no victory");
    evaluate_victory(&mut g, true, &mut events);
    assert_eq!(game_over(&events).unwrap().reason, GameOverReason::Won);
}

// 5. A loaded tour save gets a working, deterministic generator.
#[test]
fn loaded_tour_save_has_a_working_rng() {
    let t = tour_ref(TourState::new_at(4242, 0));
    let g = t.borrow_mut().start_current_sector(Some(2)).unwrap();
    let data = encode_tour_save(Some(&g), Some(&t)).unwrap();

    let (lg, lt) = decode_tour_game(&data, "TOUR.TRK").unwrap();
    let lg = lg.unwrap();
    assert!(lt.is_some());
    let mut lg = lg.borrow_mut();
    let rng = lg.rng.as_ref().expect("rng");
    assert!(!rng.is_nil_source(), "tour load left a nil-source PRNG");
    assert_eq!(*rng, PRNG::new(lg.stardate as i64));

    // Deterministic, and identical to the plain LoadGame path.
    let (again, _) = decode_tour_game(&data, "TOUR.TRK").unwrap();
    assert_eq!(again.unwrap().borrow().rng, lg.rng);
    let plain = decode_load_game(&data).unwrap();
    assert_eq!(plain.borrow().rng, lg.rng);

    // Drawing from it (entering a quadrant) no longer panics.
    let _ = lg.rng().float64();
    lg.populate_quadrant([2, 2], [4, 4]);
}

// 6. Saves keep the Klingons in the current quadrant.
const SAVES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../testdata/saves/");

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!("{SAVES}{name}")).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// A normal game (no adversaries) in a quadrant with three Klingons, one of
/// them damaged.
fn normal_game_with_klingons() -> GameState {
    let mut g = GameState::new_game_with_options(
        2024,
        SkillLevel::Good,
        GameLength::Medium,
        default_rules_for_profile(&DifficultyProfile::normal()),
    );
    g.populate_quadrant([2, 4], [4, 4]);
    assert_eq!(g.current_quad.klingons.len(), 3);
    assert!(g.current_quad.enemies.is_empty());
    g.current_quad.klingons[1]
        .as_ref()
        .unwrap()
        .borrow_mut()
        .energy = 123.0;
    g
}

fn klingon_list(g: &GameState) -> Vec<Klingon> {
    g.current_quad
        .klingons
        .iter()
        .flatten()
        .map(|k| k.borrow().clone())
        .collect()
}

fn grid_klingon_cells(g: &GameState) -> Vec<Coord> {
    let mut v = Vec::new();
    for r in 1..=8i32 {
        for c in 1..=8i32 {
            if matches!(
                g.current_quad.grid[r as usize][c as usize],
                EntityType::Klingon | EntityType::Commander | EntityType::SuperCommander
            ) {
                v.push([r, c]);
            }
        }
    }
    v
}

/// Every Klingon on the grid has exactly one live entry on the legacy list,
/// and spread phasers find (and hit) them.
fn assert_klingons_fight(g: &mut GameState, what: &str) {
    let mut cells = grid_klingon_cells(g);
    let mut listed: Vec<Coord> = klingon_list(g).iter().map(|k| k.sector).collect();
    cells.sort();
    listed.sort();
    assert!(
        !cells.is_empty(),
        "{what}: fixture has Klingons on the grid"
    );
    assert_eq!(listed, cells, "{what}: legacy list matches the grid");
    let mut ids: Vec<i32> = klingon_list(g).iter().map(|k| k.id).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), cells.len(), "{what}: unique Klingon IDs");
    assert!(klingon_list(g).iter().all(|k| k.energy > 0.0), "{what}");
    g.enterprise.condition = ConditionType::Red;
    g.enterprise.energy = 5000.0;
    let events = Action::FirePhasers(ActionFirePhasers {
        energy: 300.0,
        ..Default::default()
    })
    .execute(g)
    .unwrap_or_else(|e| panic!("{what}: phasers: {e}"));
    assert!(
        events.iter().any(|e| matches!(e, Event::PhaserHit(_))),
        "{what}: {events:?}"
    );
}

#[test]
fn normal_save_keeps_the_quadrants_klingons() {
    let g = normal_game_with_klingons();
    let want = klingon_list(&g);
    let gr = game_ref(g.clone());

    let env = encode_save_game(&gr).unwrap();
    let v: json::Value = json::parse(&env).unwrap();
    assert_eq!(
        v.get("version"),
        Some(&json::Value::Number("3".into())),
        "new saves are format version 3"
    );
    let legacy = encode_legacy_save(&g).unwrap();
    for (what, data) in [("envelope", &env), ("legacy", &legacy)] {
        let l = decode_load_game(data).unwrap();
        let mut l = l.borrow_mut();
        assert_eq!(klingon_list(&l), want, "{what}");
        assert_klingons_fight(&mut l, what);
    }

    // A Klingon's damage survives the round trip: a torpedo no longer
    // destroys a full-strength Klingon outright.
    let l = decode_load_game(&env).unwrap();
    let mut l = l.borrow_mut();
    let strong = want
        .iter()
        .find(|k| k.energy > 500.0)
        .expect("strong Klingon");
    let events = Action::FireTorpedo(ActionFireTorpedo {
        target: strong.sector,
        ..Default::default()
    })
    .execute(&mut l)
    .unwrap();
    let hit_strong = events
        .iter()
        .any(|e| matches!(e, Event::TorpedoHit(h) if h.target == strong.sector && !h.destroyed));
    let blocked = !events
        .iter()
        .any(|e| matches!(e, Event::TorpedoHit(h) if h.target == strong.sector));
    assert!(hit_strong || blocked, "{events:?}");
}

#[test]
fn tour_save_keeps_the_quadrants_klingons() {
    let t = tour_ref(TourState::new_at(4242, 0));
    let g = t.borrow_mut().start_current_sector(Some(3)).unwrap();
    g.borrow_mut().current_quad.klingons[0]
        .as_ref()
        .unwrap()
        .borrow_mut()
        .energy = 77.0;
    let want = klingon_list(&g.borrow());
    let data = encode_tour_save(Some(&g), Some(&t)).unwrap();
    let (lg, _) = decode_tour_game(&data, "TOUR.TRK").unwrap();
    assert_eq!(klingon_list(&lg.unwrap().borrow()), want);
}

#[test]
fn older_saves_rebuild_the_quadrants_klingons() {
    for name in [
        "go-legacy-kobayashi.TRK",
        "go-v2-kobayashi.TRK",
        "go-legacy-tour-sector.TRK",
        "prev-legacy-normal.TRK",
        "prev-v2-normal.TRK",
        "prev-v2-adversaries.TRK",
        "prev-v2-tour.TRK",
    ] {
        let data = fixture(name);
        let l = decode_load_game(&data).unwrap_or_else(|e| panic!("{name}: {e}"));
        let mut l = l.borrow_mut();
        assert!(!l.current_quad.klingons.is_empty(), "{name}");
        // Reconstruction is deterministic.
        let again = decode_load_game(&data).unwrap();
        assert_eq!(klingon_list(&again.borrow()), klingon_list(&l), "{name}");
        assert_klingons_fight(&mut l, name);
    }

    // Adversary saves already stored the Klingons in `enemies`: those
    // values are kept rather than rebuilt.
    let l = decode_load_game(&fixture("prev-v2-adversaries.TRK")).unwrap();
    let l = l.borrow();
    for k in klingon_list(&l) {
        let e = enemy_by_id(&l, k.id).expect("enemy entry");
        assert_eq!((e.sector, e.energy), (k.sector, k.energy));
    }

    // A previous-version tour save still loads as a tour.
    let (lg, lt) = decode_tour_game(&fixture("prev-v2-tour.TRK"), "TOUR.TRK").unwrap();
    assert!(lt.is_some());
    assert!(!lg.unwrap().borrow().current_quad.klingons.is_empty());
}

// 10. Focused phasers hit Klingons in games without adversaries.
/// A game without adversaries: the Klingons live only on the legacy list.
fn classic_quadrant(klingons: &[(i32, Coord, f64)]) -> GameState {
    let mut g = quadrant_game(vec![]);
    g.rules.adversaries = false;
    g.current_quad.klingons = GoVec::empty();
    for &(id, sector, energy) in klingons {
        g.current_quad.grid[sector[0] as usize][sector[1] as usize] = EntityType::Klingon;
        g.current_quad.klingons.push(ptr(Klingon {
            id,
            sector,
            energy,
            ..Default::default()
        }));
    }
    g.remaining_klingons = klingons.len() as i32 + 1;
    g.klingons_remaining = g.remaining_klingons;
    g
}

fn phaser_at(g: &mut GameState, target_sector: Coord, energy: f64) -> ActionResult {
    Action::PhaserDirect(ActionPhaserDirect {
        target_sector,
        energy,
    })
    .execute(g)
}

#[test]
fn focused_phasers_hit_a_klingon_without_adversaries() {
    let mut g = classic_quadrant(&[(1, [4, 6], 500.0), (2, [1, 1], 300.0)]);
    let events = phaser_at(&mut g, [4, 6], 200.0).unwrap();
    // 200 energy at distance 2.
    assert!(
        events.iter().any(|e| matches!(e,
            Event::PhaserHit(h) if h.klingon_id == 1 && !h.destroyed && h.damage == 100.0)),
        "{events:?}"
    );
    assert_eq!(klingon_by_id(&g, 1).unwrap().energy, 400.0);
    assert_eq!(g.enterprise.energy, 4800.0);

    let events = phaser_at(&mut g, [4, 6], 2000.0).unwrap();
    assert!(events.iter().any(|e| matches!(e,
        Event::PhaserHit(h) if h.klingon_id == 1 && h.destroyed && h.damage == 400.0)));
    assert!(klingon_by_id(&g, 1).is_none());
    assert_eq!(g.current_quad.grid[4][6], EntityType::Empty);
    assert_eq!(g.metrics.klingons_killed, 1);
    assert_eq!(g.remaining_klingons, 2);
    assert!(klingon_by_id(&g, 2).is_some());
}

// 11. Spread phasers hit every visible hostile ship.
#[test]
fn spread_phasers_hit_romulans_and_tholians() {
    let mut g = quadrant_game(vec![
        enemy(1, FactionType::Klingon, [4, 6], 500.0, 0.0),
        enemy(2, FactionType::Romulan, [4, 8], 1000.0, 400.0),
        enemy(3, FactionType::Tholian, [8, 4], 800.0, 300.0),
        EnemyVessel {
            is_cloaked: true,
            ..enemy(4, FactionType::Romulan, [1, 1], 1000.0, 400.0)
        },
    ]);
    let events = Action::FirePhasers(ActionFirePhasers {
        energy: 2400.0,
        ..Default::default()
    })
    .execute(&mut g)
    .unwrap();
    let hits: Vec<&EventPhaserHit> = events
        .iter()
        .filter_map(|e| match e {
            Event::PhaserHit(h) => Some(h),
            _ => None,
        })
        .collect();
    // 2400 split over the three visible ships (800 each), divided by distance.
    assert_eq!(hits.len(), 3, "{events:?}");
    assert_eq!(
        (hits[0].klingon_id, hits[0].faction),
        (1, FactionType::Klingon)
    );
    assert_eq!(hits[0].damage, 400.0);
    assert_eq!(
        (hits[1].klingon_id, hits[1].faction),
        (2, FactionType::Romulan)
    );
    assert_eq!(hits[1].damage, 200.0);
    assert_eq!(
        (hits[2].klingon_id, hits[2].faction),
        (3, FactionType::Tholian)
    );
    assert_eq!(hits[2].damage, 200.0);
    assert_eq!(klingon_by_id(&g, 1).unwrap().energy, 100.0);
    let r = enemy_by_id(&g, 2).unwrap();
    assert_eq!(
        (r.shields, r.energy),
        (200.0, 1000.0),
        "shields absorb first"
    );
    let t = enemy_by_id(&g, 3).unwrap();
    assert_eq!((t.shields, t.energy), (100.0, 800.0));
    let cloaked = enemy_by_id(&g, 4).unwrap();
    assert_eq!(
        (cloaked.shields, cloaked.energy),
        (400.0, 1000.0),
        "cloaked ships are not targeted"
    );
}

#[test]
fn spread_phasers_destroy_the_last_tholian_and_win() {
    let mut g = quadrant_game(vec![enemy(3, FactionType::Tholian, [4, 5], 800.0, 300.0)]);
    let events = Action::FirePhasers(ActionFirePhasers {
        energy: 1500.0,
        ..Default::default()
    })
    .execute(&mut g)
    .expect("phasers fire with only a Tholian in the quadrant");
    assert!(events.iter().any(|e| matches!(e,
        Event::PhaserHit(h) if h.destroyed && h.faction == FactionType::Tholian && h.damage == 800.0)));
    assert!(enemy_by_id(&g, 3).is_none());
    assert_eq!(g.current_quad.grid[4][5], EntityType::Empty);
    assert_eq!(g.remaining_klingons, 0);
    assert_eq!(game_over(&events).unwrap().reason, GameOverReason::Won);
}

#[test]
fn spread_phasers_skip_cloaked_ships_and_refuse_when_none_is_visible() {
    let mut g = quadrant_game(vec![EnemyVessel {
        is_cloaked: true,
        ..enemy(4, FactionType::Romulan, [1, 1], 1000.0, 400.0)
    }]);
    let err = Action::FirePhasers(ActionFirePhasers {
        energy: 500.0,
        ..Default::default()
    })
    .execute(&mut g)
    .unwrap_err();
    assert_eq!(err.0, "no enemy ships in quadrant");
    assert_eq!(g.enterprise.energy, 5000.0);

    // A cloaked Klingon commander is skipped as well.
    let mut g = quadrant_game(vec![
        enemy(1, FactionType::Klingon, [4, 6], 500.0, 0.0),
        EnemyVessel {
            is_commander: true,
            is_cloaked: true,
            ..enemy(2, FactionType::Klingon, [6, 4], 1200.0, 0.0)
        },
    ]);
    let events = Action::FirePhasers(ActionFirePhasers {
        energy: 400.0,
        ..Default::default()
    })
    .execute(&mut g)
    .unwrap();
    let hit_ids: Vec<i32> = events
        .iter()
        .filter_map(|e| match e {
            Event::PhaserHit(h) => Some(h.klingon_id),
            _ => None,
        })
        .collect();
    assert_eq!(hit_ids, vec![1]);
    assert_eq!(klingon_by_id(&g, 2).unwrap().energy, 1200.0);

    // With only the cloaked commander (an unknown contact on the scanners)
    // left, the refusal says why.
    let mut g = quadrant_game(vec![EnemyVessel {
        is_commander: true,
        is_cloaked: true,
        ..enemy(2, FactionType::Klingon, [6, 4], 1200.0, 0.0)
    }]);
    let err = Action::FirePhasers(ActionFirePhasers {
        energy: 400.0,
        ..Default::default()
    })
    .execute(&mut g)
    .unwrap_err();
    assert_eq!(err.0, "phasers cannot lock on to a cloaked vessel");
}

// 12. Focused phasers and target lock refuse cloaked ships.
#[test]
fn focused_phasers_refuse_a_cloaked_ship() {
    let mut g = quadrant_game(vec![
        EnemyVessel {
            is_cloaked: true,
            ..enemy(4, FactionType::Romulan, [4, 7], 1000.0, 400.0)
        },
        EnemyVessel {
            is_commander: true,
            is_cloaked: true,
            ..enemy(2, FactionType::Klingon, [6, 4], 1200.0, 0.0)
        },
    ]);
    for target in [[4, 7], [6, 4]] {
        let err = phaser_at(&mut g, target, 500.0).unwrap_err();
        assert_eq!(err.0, "TARGET LOCK FAILED: CLOAKED VESSEL");
        let err = Action::TorpedoDirect(ActionTorpedoDirect {
            target_sector: target,
        })
        .execute(&mut g)
        .unwrap_err();
        assert_eq!(err.0, "TARGET LOCK FAILED: CLOAKED VESSEL");
    }
    assert_eq!(g.enterprise.energy, 5000.0);
    assert_eq!(g.enterprise.torpedoes, 10);
    assert_eq!(enemy_by_id(&g, 4).unwrap().energy, 1000.0);
    assert_eq!(klingon_by_id(&g, 2).unwrap().energy, 1200.0);

    // Once it decloaks, focused phasers hit it.
    g.current_quad.enemies[0]
        .as_ref()
        .unwrap()
        .borrow_mut()
        .is_cloaked = false;
    g.current_quad.grid[4][7] = EntityType::Romulan;
    let events = phaser_at(&mut g, [4, 7], 1500.0).unwrap();
    assert!(events.iter().any(|e| matches!(e,
        Event::PhaserHit(h) if h.klingon_id == 4 && h.faction == FactionType::Romulan)));
    assert_eq!(enemy_by_id(&g, 4).unwrap().shields, 0.0);
}

// 13. Torpedo hits report the damage actually dealt.
fn torpedo_hit(events: &[Event]) -> &EventTorpedoHit {
    events
        .iter()
        .find_map(|e| match e {
            Event::TorpedoHit(h) => Some(h),
            _ => None,
        })
        .expect("torpedo hit")
}

#[test]
fn torpedo_hit_reports_damage_after_shields() {
    // A Romulan with 400 shields: 400 of the 500 are absorbed.
    let mut g = quadrant_game(vec![enemy(2, FactionType::Romulan, [4, 7], 1000.0, 400.0)]);
    let events = torpedo_at(&mut g, [4, 7]);
    let h = torpedo_hit(&events);
    assert!(!h.destroyed);
    assert_eq!((h.damage, h.shield_damage), (100.0, 400.0));
    assert_eq!(enemy_by_id(&g, 2).unwrap().energy, 900.0);

    // Shields that absorb the whole torpedo: no hull damage.
    let mut g = quadrant_game(vec![enemy(3, FactionType::Tholian, [4, 7], 800.0, 600.0)]);
    let h = torpedo_hit(&torpedo_at(&mut g, [4, 7])).clone();
    assert_eq!((h.damage, h.shield_damage), (0.0, 500.0));

    // An unshielded Klingon takes the full 500.
    let mut g = classic_quadrant(&[(1, [4, 7], 900.0)]);
    let h = torpedo_hit(&torpedo_at(&mut g, [4, 7])).clone();
    assert_eq!(
        (h.damage, h.shield_damage, h.destroyed),
        (500.0, 0.0, false)
    );

    // A kill reports the energy the ship had left and what its shields took.
    let mut g = quadrant_game(vec![enemy(1, FactionType::Klingon, [4, 7], 150.0, 100.0)]);
    let h = torpedo_hit(&torpedo_at(&mut g, [4, 7])).clone();
    assert_eq!(
        (h.damage, h.shield_damage, h.destroyed),
        (150.0, 100.0, true)
    );
}
