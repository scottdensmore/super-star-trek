//! Port of pkg/engine/adversaries_test.go.

use sst_engine::json::GoVec;
use sst_engine::*;

#[test]
fn adversaries_entity_constants_and_types() {
    assert!(EntityType::Romulan > EntityType::Wormhole);
    assert!(EntityType::Tholian > EntityType::Romulan);
    assert!(EntityType::PlasmaTorpedo > EntityType::Tholian);
    assert!(EntityType::TholianWeb > EntityType::PlasmaTorpedo);

    let enemy = EnemyVessel {
        id: 1,
        faction: FactionType::Romulan,
        sector: [3, 4],
        energy: 1200.0,
        shields: 400.0,
        max_energy: 1200.0,
        is_commander: false,
        is_cloaked: true,
        ..Default::default()
    };
    assert!(enemy.faction == FactionType::Romulan && enemy.is_cloaked);

    let plasma = PlasmaTorpedo {
        id: 101,
        source_id: 1,
        sector: [4, 4],
        energy: 1000.0,
        target_sector: [6, 6],
        turns_in_flight: 1,
    };
    assert!(plasma.energy == 1000.0 && plasma.turns_in_flight == 1);

    let web = TholianWebSegment {
        coord: [5, 5],
        strength: 250.0,
    };
    assert_eq!(web.strength, 250.0);
}

#[test]
fn adversaries_quadrant_state_sync_with_legacy_klingons() {
    let mut g = GameState::new_game_with_seed(42);
    g.current_quad.enemies = GoVec::from_vec(vec![ptr(EnemyVessel {
        id: 1,
        faction: FactionType::Klingon,
        sector: [2, 3],
        energy: 500.0,
        shields: 200.0,
        is_commander: true,
        ..Default::default()
    })]);
    sync_quadrant_enemies(&mut g.current_quad);
    assert_eq!(g.current_quad.klingons.len(), 1);
    let k = g.current_quad.klingons[0].as_ref().unwrap().borrow();
    assert!(k.id == 1 && k.is_commander, "{k:?}");
}

#[test]
fn adversaries_quadrant_state_sync_edge_cases() {
    // SyncQuadrantEnemies(nil) is not representable.
    let e = |id, faction, sector, energy, shields, is_commander, is_cloaked| {
        ptr(EnemyVessel {
            id,
            faction,
            sector,
            energy,
            shields,
            is_commander,
            is_cloaked,
            ..Default::default()
        })
    };
    let mut quad = QuadrantState {
        enemies: GoVec::from_vec(vec![
            e(1, FactionType::Romulan, [1, 1], 1000.0, 300.0, false, false),
            e(2, FactionType::Klingon, [2, 2], 600.0, 250.0, true, true),
            e(3, FactionType::Tholian, [3, 3], 500.0, 500.0, false, false),
            e(4, FactionType::Klingon, [4, 4], 400.0, 150.0, false, false),
        ]),
        ..Default::default()
    };
    sync_quadrant_enemies(&mut quad);
    assert_eq!(quad.klingons.len(), 2);
    {
        let k0 = quad.klingons[0].as_ref().unwrap().borrow();
        assert!(k0.id == 2 && k0.is_commander && k0.is_cloaked);
        let k1 = quad.klingons[1].as_ref().unwrap().borrow();
        assert!(k1.id == 4 && !k1.is_commander);
    }
    quad.enemies = GoVec::nil();
    sync_quadrant_enemies(&mut quad);
    assert!(quad.klingons.is_empty());
}

#[test]
fn adversaries_sandbox_spawning_in_populate_quadrant() {
    let mut gc = GameState::new_game_with_seed(42);
    gc.rules.adversaries = false;
    gc.galaxy_chart[4][4] = 203;
    gc.populate_quadrant([4, 4], [1, 1]);
    assert_eq!(gc.current_quad.klingons.len(), 2);
    assert!(gc.current_quad.enemies.is_empty());

    let mut gr = GameState::new_game_with_seed(42);
    gr.rules.adversaries = true;
    gr.galaxy_chart[4][4] = 203;
    gr.populate_quadrant([4, 4], [1, 1]);
    assert_eq!(gr.current_quad.enemies.len(), 2);
    let (mut rom, mut kl) = (false, false);
    for e in gr.current_quad.enemies.iter() {
        let e = e.as_ref().unwrap().borrow();
        if e.faction == FactionType::Romulan {
            rom = true;
            assert!(e.is_cloaked);
            assert_eq!(
                gr.current_quad.grid[e.sector[0] as usize][e.sector[1] as usize],
                EntityType::Empty
            );
        }
        if e.faction == FactionType::Klingon {
            kl = true;
        }
    }
    assert!(rom && kl);
    assert_eq!(gr.current_quad.klingons.len(), 1);

    let mut gb = GameState::new_game_with_seed(42);
    gb.rules.adversaries = true;
    gb.galaxy_chart[1][4] = 203;
    gb.populate_quadrant([1, 4], [1, 1]);
    let mut found = false;
    for e in gb.current_quad.enemies.iter() {
        let e = e.as_ref().unwrap().borrow();
        if e.faction == FactionType::Tholian {
            found = true;
            assert_eq!(
                gb.current_quad.grid[e.sector[0] as usize][e.sector[1] as usize],
                EntityType::Tholian
            );
        }
    }
    assert!(found);

    let mut gs = GameState::new_game_with_seed(42);
    gs.rules.adversaries = true;
    gs.galaxy_chart[4][4] = 213;
    gs.populate_quadrant([4, 4], [1, 1]);
    assert!(gs
        .current_quad
        .enemies
        .iter()
        .any(|e| e.as_ref().unwrap().borrow().faction == FactionType::Tholian));
}
