//! Port of pkg/engine/klingon_pack_test.go.

use sst_engine::json::GoVec;
use sst_engine::*;

fn kv(id: i32, sector: Coord, energy: f64) -> Ptr<EnemyVessel> {
    ptr(EnemyVessel {
        id,
        faction: FactionType::Klingon,
        sector,
        energy,
        ..Default::default()
    })
}

fn cmdv(id: i32, sector: Coord, energy: f64) -> Ptr<EnemyVessel> {
    ptr(EnemyVessel {
        id,
        faction: FactionType::Klingon,
        sector,
        energy,
        is_commander: true,
        ..Default::default()
    })
}

fn sector(p: &Ptr<EnemyVessel>) -> Coord {
    p.as_ref().unwrap().borrow().sector
}

#[test]
fn klingon_pack_crossfire_bracket_detection() {
    let mut g = GameState::new_game_with_seed(777);
    g.enterprise.sector = [4, 4];
    g.current_quad.enemies = GoVec::from_vec(vec![kv(1, [4, 1], 200.0), kv(2, [4, 7], 200.0)]);
    assert!(detect_crossfire_bracket(Some(&g)));
    assert_eq!(calculate_pack_damage_multiplier(Some(&g)), 1.35);
}

#[test]
fn klingon_pack_commander_screening_interposition() {
    let mut g = GameState::new_game_with_seed(888);
    g.enterprise.sector = [4, 1];
    g.current_quad.grid[4][1] = EntityType::Enterprise;
    let commander = cmdv(10, [4, 6], 300.0);
    let raider = kv(11, [3, 4], 200.0);
    g.current_quad.enemies = GoVec::from_vec(vec![commander, raider.clone()]);
    g.current_quad.grid[4][6] = EntityType::Commander;
    g.current_quad.grid[3][4] = EntityType::Klingon;

    let events = execute_commander_screening(&mut g);
    assert!(!events.is_empty());
    assert_eq!(sector(&raider), [4, 4]);
    assert_eq!(g.current_quad.grid[4][4], EntityType::Klingon);
    assert_eq!(g.current_quad.grid[3][4], EntityType::Empty);
    let Event::KlingonScreening(s) = &events[0] else {
        panic!("expected EventKlingonScreening, got {:?}", events[0]);
    };
    assert_eq!(events[0].event_type(), "KlingonScreening");
    assert!(
        s.raider_id == 11 && s.commander_id == 10 && s.interposed == [4, 4],
        "{s:?}"
    );
}

#[test]
fn klingon_pack_angle_thresholds() {
    let mut g = GameState::new_game_with_seed(101);
    g.enterprise.sector = [4, 4];
    let k1 = kv(1, [4, 8], 200.0);
    g.current_quad.enemies = GoVec::from_vec(vec![k1.clone(), kv(2, [5, 8], 200.0)]);
    assert!(!detect_crossfire_bracket(Some(&g)));
    assert_eq!(calculate_pack_damage_multiplier(Some(&g)), 1.0);

    g.current_quad.enemies = GoVec::from_vec(vec![k1.clone(), kv(3, [8, 4], 200.0)]);
    assert!(detect_crossfire_bracket(Some(&g)));
    assert_eq!(calculate_pack_damage_multiplier(Some(&g)), 1.35);

    g.current_quad.enemies = GoVec::from_vec(vec![k1, kv(4, [4, 4], 200.0)]);
    assert!(!detect_crossfire_bracket(Some(&g)));
}

#[test]
fn klingon_pack_inactive_klingons_filtered() {
    let mut g = GameState::new_game_with_seed(102);
    g.enterprise.sector = [4, 4];
    let dead = kv(2, [4, 7], 0.0);
    g.current_quad.enemies = GoVec::from_vec(vec![kv(1, [4, 1], 200.0), dead.clone()]);
    assert!(!detect_crossfire_bracket(Some(&g)));
    dead.as_ref().unwrap().borrow_mut().energy = -50.0;
    assert!(!detect_crossfire_bracket(Some(&g)));
}

#[test]
fn klingon_pack_faction_filtering() {
    let mut g = GameState::new_game_with_seed(103);
    g.enterprise.sector = [4, 4];
    let r = ptr(EnemyVessel {
        id: 2,
        faction: FactionType::Romulan,
        sector: [4, 7],
        energy: 400.0,
        ..Default::default()
    });
    g.current_quad.enemies = GoVec::from_vec(vec![kv(1, [4, 1], 200.0), r]);
    assert!(!detect_crossfire_bracket(Some(&g)));
}

#[test]
fn klingon_pack_commander_screening_vertical() {
    let mut g = GameState::new_game_with_seed(104);
    g.enterprise.sector = [1, 4];
    let raider = kv(21, [4, 3], 200.0);
    g.current_quad.enemies = GoVec::from_vec(vec![cmdv(20, [6, 4], 300.0), raider.clone()]);
    g.current_quad.grid[6][4] = EntityType::Commander;
    g.current_quad.grid[4][3] = EntityType::Klingon;
    let events = execute_commander_screening(&mut g);
    assert_eq!(events.len(), 1);
    assert_eq!(sector(&raider), [4, 4]);
    assert_eq!(g.current_quad.grid[4][4], EntityType::Klingon);
    assert_eq!(g.current_quad.grid[4][3], EntityType::Empty);
}

#[test]
fn klingon_pack_commander_screening_target_blocked() {
    let mut g = GameState::new_game_with_seed(105);
    g.enterprise.sector = [4, 1];
    let raider = kv(31, [3, 4], 200.0);
    g.current_quad.enemies = GoVec::from_vec(vec![cmdv(30, [4, 6], 300.0), raider.clone()]);
    g.current_quad.grid[4][6] = EntityType::Commander;
    g.current_quad.grid[3][4] = EntityType::Klingon;
    g.current_quad.grid[4][4] = EntityType::Star;
    assert!(execute_commander_screening(&mut g).is_empty());
    assert_eq!(sector(&raider), [3, 4]);
    assert_eq!(g.current_quad.grid[4][4], EntityType::Star);
}

#[test]
fn klingon_pack_commander_screening_destroyed_units() {
    let mut g = GameState::new_game_with_seed(106);
    g.enterprise.sector = [4, 1];
    let dead_cmd = ptr(EnemyVessel {
        id: 40,
        faction: FactionType::Klingon,
        sector: [4, 6],
        is_commander: true,
        max_energy: 300.0,
        energy: 0.0,
        ..Default::default()
    });
    g.current_quad.enemies = GoVec::from_vec(vec![dead_cmd, kv(41, [3, 4], 200.0)]);
    g.current_quad.grid[3][4] = EntityType::Klingon;
    assert!(execute_commander_screening(&mut g).is_empty());

    let dead_raider = ptr(EnemyVessel {
        id: 43,
        faction: FactionType::Klingon,
        sector: [3, 4],
        max_energy: 200.0,
        energy: -10.0,
        ..Default::default()
    });
    g.current_quad.enemies = GoVec::from_vec(vec![cmdv(42, [4, 6], 300.0), dead_raider]);
    g.current_quad.grid[4][6] = EntityType::Commander;
    assert!(execute_commander_screening(&mut g).is_empty());
}

#[test]
fn klingon_pack_defensive_nil_and_bounds() {
    assert!(!detect_crossfire_bracket(None));
    assert_eq!(calculate_pack_damage_multiplier(None), 1.0);
    // ExecuteCommanderScreening(nil) is not representable.
    let mut g = GameState::new_game_with_seed(107);
    g.current_quad.enemies = GoVec::from_vec(vec![None, None]);
    assert!(!detect_crossfire_bracket(Some(&g)));
    assert!(execute_commander_screening(&mut g).is_empty());
}
