//! Port of pkg/engine/romulan_test.go.

use std::cell::RefCell;
use std::rc::Rc;

use sst_engine::json::GoVec;
use sst_engine::*;

fn eref(e: EnemyVessel) -> EnemyRef {
    Rc::new(RefCell::new(e))
}

fn pref(p: PlasmaTorpedo) -> Rc<RefCell<PlasmaTorpedo>> {
    Rc::new(RefCell::new(p))
}

#[test]
fn romulan_cloaking_and_repositioning() {
    let mut g = GameState::new_game_with_seed(1234);
    let r = eref(EnemyVessel {
        id: 1,
        faction: FactionType::Romulan,
        sector: [3, 3],
        energy: 1200.0,
        shields: 500.0,
        max_energy: 1200.0,
        is_cloaked: true,
        cloak_turns: 2,
        ..Default::default()
    });
    g.current_quad.enemies = GoVec::from_vec(vec![Some(r.clone())]);
    g.current_quad.grid[3][3] = EntityType::Romulan;

    let events = romulan_turn(&mut g, Some(&r));
    assert!(!events.is_empty());
    assert_eq!(r.borrow().cloak_turns, 1);

    let _ = romulan_turn(&mut g, Some(&r));
    assert!(!r.borrow().is_cloaked);
    assert_eq!(g.current_quad.plasma_torpedoes.len(), 1);
}

#[test]
fn romulan_plasma_torpedo_tracking_and_dissipation() {
    let mut g = GameState::new_game_with_seed(5678);
    g.enterprise.sector = [8, 8];
    let plasma = pref(PlasmaTorpedo {
        id: 10,
        source_id: 1,
        sector: [2, 2],
        energy: 1000.0,
        target_sector: g.enterprise.sector,
        turns_in_flight: 0,
    });
    g.current_quad.plasma_torpedoes = GoVec::from_vec(vec![Some(plasma.clone())]);
    g.current_quad.grid[2][2] = EntityType::PlasmaTorpedo;

    let events = advance_plasma_torpedoes(&mut g);
    assert!(!events.is_empty());
    assert!(plasma.borrow().energy < 1000.0);
    assert_ne!(plasma.borrow().sector, [2, 2]);

    let sec = plasma.borrow().sector;
    let (ok, ev) = intercept_plasma_torpedo(&mut g, sec, 900.0);
    assert!(ok && !ev.is_empty());
    assert!(g.current_quad.plasma_torpedoes.is_empty());
}

#[test]
fn romulan_decloaked_firing_and_recloaking() {
    let mut g = GameState::new_game_with_seed(4321);
    g.enterprise.sector = [5, 5];
    g.enterprise.shields = 400.0;
    g.enterprise.energy = 2500.0;
    let r = eref(EnemyVessel {
        id: 2,
        faction: FactionType::Romulan,
        sector: [4, 4],
        energy: 1000.0,
        shields: 400.0,
        max_energy: 1000.0,
        ..Default::default()
    });
    g.current_quad.enemies = GoVec::from_vec(vec![Some(r.clone())]);
    g.current_quad.grid[4][4] = EntityType::Romulan;

    let events = romulan_turn(&mut g, Some(&r));
    assert!(!events.is_empty());
    assert!(r.borrow().is_cloaked);
    assert_eq!(r.borrow().cloak_turns, 3);
    assert_eq!(g.current_quad.grid[4][4], EntityType::Empty);
    assert!(events
        .iter()
        .any(|e| e.event_type() == "KlingonCounterAttack"));
}

#[test]
fn romulan_plasma_torpedo_impact_50_percent_shield_penetration() {
    let mut g = GameState::new_game_with_seed(9999);
    g.enterprise.sector = [4, 4];
    g.enterprise.shields = 500.0;
    g.enterprise.energy = 2000.0;
    let pt = pref(PlasmaTorpedo {
        id: 20,
        source_id: 1,
        sector: [3, 3],
        energy: 600.0,
        target_sector: g.enterprise.sector,
        turns_in_flight: 0,
    });
    g.current_quad.plasma_torpedoes = GoVec::from_vec(vec![Some(pt)]);
    g.current_quad.grid[3][3] = EntityType::PlasmaTorpedo;

    let events = advance_plasma_torpedoes(&mut g);
    assert!(!events.is_empty());
    let impact = events
        .iter()
        .find_map(|e| match e {
            Event::PlasmaImpact(i) => Some(i.clone()),
            _ => None,
        })
        .expect("expected EventPlasmaImpact");
    assert_eq!(impact.shield_damage, 240.0);
    assert_eq!(impact.hull_damage, 240.0);
    assert_eq!(g.enterprise.shields, 260.0);
    assert_eq!(g.enterprise.energy, 1760.0);
    assert!(g.current_quad.plasma_torpedoes.is_empty());
    assert_eq!(g.current_quad.grid[3][3], EntityType::Empty);
}

#[test]
fn romulan_plasma_torpedo_dissipation_low_energy() {
    let mut g = GameState::new_game_with_seed(1111);
    g.enterprise.sector = [8, 8];
    let pt = pref(PlasmaTorpedo {
        id: 21,
        source_id: 1,
        sector: [2, 2],
        energy: 220.0,
        target_sector: g.enterprise.sector,
        turns_in_flight: 1,
    });
    g.current_quad.plasma_torpedoes = GoVec::from_vec(vec![Some(pt)]);
    g.current_quad.grid[2][2] = EntityType::PlasmaTorpedo;
    let events = advance_plasma_torpedoes(&mut g);
    assert!(events.len() == 1 && events[0].event_type() == "PlasmaDissipated");
    assert!(g.current_quad.plasma_torpedoes.is_empty());
    assert_eq!(g.current_quad.grid[2][2], EntityType::Empty);
}

#[test]
fn romulan_plasma_torpedo_dissipation_max_turns() {
    let mut g = GameState::new_game_with_seed(2222);
    g.enterprise.sector = [8, 8];
    let pt = pref(PlasmaTorpedo {
        id: 22,
        source_id: 1,
        sector: [2, 2],
        energy: 800.0,
        target_sector: g.enterprise.sector,
        turns_in_flight: 4,
    });
    g.current_quad.plasma_torpedoes = GoVec::from_vec(vec![Some(pt)]);
    g.current_quad.grid[2][2] = EntityType::PlasmaTorpedo;
    let events = advance_plasma_torpedoes(&mut g);
    assert!(events.len() == 1 && events[0].event_type() == "PlasmaDissipated");
    assert!(g.current_quad.plasma_torpedoes.is_empty());
}

#[test]
fn romulan_plasma_torpedo_obstacle_avoidance() {
    let mut g = GameState::new_game_with_seed(3333);
    g.enterprise.sector = [4, 4];
    let pt = pref(PlasmaTorpedo {
        id: 23,
        source_id: 1,
        sector: [2, 2],
        energy: 900.0,
        target_sector: g.enterprise.sector,
        turns_in_flight: 0,
    });
    g.current_quad.plasma_torpedoes = GoVec::from_vec(vec![Some(pt.clone())]);
    g.current_quad.grid[2][2] = EntityType::PlasmaTorpedo;
    g.current_quad.grid[3][3] = EntityType::Star;

    let events = advance_plasma_torpedoes(&mut g);
    assert!(!events.is_empty());
    let s = pt.borrow().sector;
    assert_ne!(s, [3, 3]);
    assert!(s == [3, 2] || s == [2, 3], "got {s:?}");
    assert_eq!(g.current_quad.grid[3][3], EntityType::Star);
    assert_eq!(
        g.current_quad.grid[s[0] as usize][s[1] as usize],
        EntityType::PlasmaTorpedo
    );
}

#[test]
fn romulan_intercept_plasma_torpedo_thresholds() {
    let mut g = GameState::new_game_with_seed(4444);
    let pt = pref(PlasmaTorpedo {
        id: 24,
        source_id: 1,
        sector: [3, 3],
        energy: 1000.0,
        target_sector: [5, 5],
        turns_in_flight: 0,
    });
    g.current_quad.plasma_torpedoes = GoVec::from_vec(vec![Some(pt)]);
    g.current_quad.grid[3][3] = EntityType::PlasmaTorpedo;

    let (ok, ev) = intercept_plasma_torpedo(&mut g, [3, 3], 700.0);
    assert!(!ok && ev.is_empty());
    assert_eq!(g.current_quad.plasma_torpedoes.len(), 1);

    let (ok, ev) = intercept_plasma_torpedo(&mut g, [3, 3], 750.0);
    assert!(ok && ev.len() == 1);
    assert_eq!(ev[0].event_type(), "PlasmaIntercepted");
    assert!(g.current_quad.plasma_torpedoes.is_empty());
    assert_eq!(g.current_quad.grid[3][3], EntityType::Empty);
}

#[test]
fn romulan_nil_and_invalid_handling() {
    let mut g = GameState::new_game_with_seed(5555);
    // RomulanTurn(nil, nil): a nil GameState is not representable; nil vessel is.
    assert!(romulan_turn(&mut g, None).is_empty());
    let r = eref(EnemyVessel {
        faction: FactionType::Klingon,
        ..Default::default()
    });
    assert!(romulan_turn(&mut g, Some(&r)).is_empty());
    // AdvancePlasmaTorpedoes(nil) / InterceptPlasmaTorpedo(nil, ...) not representable.
    let (ok, ev) = intercept_plasma_torpedo(&mut g, [1, 1], -50.0);
    assert!(!ok && ev.is_empty());
    let (ok, ev) = intercept_plasma_torpedo(&mut g, [1, 1], 0.0);
    assert!(!ok && ev.is_empty());
    let (ok, ev) = intercept_plasma_torpedo(&mut g, [7, 7], 500.0);
    assert!(!ok && ev.is_empty());
}
