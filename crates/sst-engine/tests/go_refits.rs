//! Port of pkg/engine/refits_test.go.

use std::collections::BTreeMap;

use sst_engine::*;

fn refits(items: &[(RefitID, i32)]) -> BTreeMap<RefitID, i32> {
    items.iter().cloned().collect()
}

#[test]
fn refits_apply_energy_and_torpedo_scaling() {
    let mut g = GameState::new_game_with_seed(100);
    g.enterprise.energy = 1500.0;
    g.enterprise.torpedoes = 5;
    let r = refits(&[(RefitID::dilithium_core(), 2), (RefitID::torpedo_bays(), 1)]);
    apply_refits(&mut g, Some(&r));
    assert_eq!(g.enterprise.max_energy, 4000.0);
    assert_eq!(g.enterprise.energy, 1500.0);
    assert_eq!(g.enterprise.max_torpedoes, 14);
    assert_eq!(g.enterprise.torpedoes, 5);
}

#[test]
fn refits_apply_refits_caps_initial_energy() {
    let mut g = GameState::new_game(1234, SkillLevel::Good, GameLength::Medium);
    assert_eq!(g.enterprise.energy, 5000.0, "expected initial energy 5000");
    apply_refits(&mut g, Some(&BTreeMap::new()));
    assert_eq!(g.enterprise.max_energy, 3000.0);
    assert_eq!(g.enterprise.energy, 3000.0);

    g.enterprise.energy = 5000.0;
    apply_refits(&mut g, Some(&refits(&[(RefitID::dilithium_core(), 1)])));
    assert_eq!(g.enterprise.max_energy, 3500.0);
    assert_eq!(g.enterprise.energy, 3500.0);
}

fn tier(t: &TourState, id: &RefitID) -> i32 {
    t.installed_refits
        .as_ref()
        .and_then(|m| m.get(id).copied())
        .unwrap_or(0)
}

#[test]
fn refits_purchase_workflow_and_constraints() {
    let mut tour = TourState::new_at(555, 0);
    tour.requisition_points = 1000;
    let dc = RefitID::dilithium_core();

    purchase_refit(Some(&mut tour), &dc).expect("unexpected error purchasing refit");
    assert_eq!(tier(&tour, &dc), 1);
    assert_eq!(tour.requisition_points, 500);

    assert!(
        purchase_refit(Some(&mut tour), &dc).is_err(),
        "expected error for insufficient funds"
    );

    tour.requisition_points = 5000;
    purchase_refit(Some(&mut tour), &dc).expect("tier 2");
    purchase_refit(Some(&mut tour), &dc).expect("tier 3");
    assert!(
        purchase_refit(Some(&mut tour), &dc).is_err(),
        "expected error beyond tier 3"
    );

    assert!(purchase_refit(Some(&mut tour), &RefitID::new("unknown_module")).is_err());
    assert!(
        purchase_refit(None, &dc).is_err(),
        "expected error for nil tour"
    );
}

#[test]
fn refits_torpedo_damage_scaling() {
    let mut g = GameState::new_game_with_seed(200);
    let base = calculate_torpedo_damage(Some(&g), 100.0);
    g.active_refits = Some(refits(&[(RefitID::torpedo_casings(), 2)]));
    let boosted = calculate_torpedo_damage(Some(&g), 100.0);
    assert_eq!(boosted, base * 1.5);
}

#[test]
fn refits_shield_absorption_reduction() {
    let mut g = GameState::new_game_with_seed(300);
    let incoming = 200.0;
    let unshielded = calculate_shield_damage_absorption(Some(&g), incoming);
    g.active_refits = Some(refits(&[(RefitID::deflector_grid(), 2)]));
    let reinforced = calculate_shield_damage_absorption(Some(&g), incoming);
    assert!(
        reinforced < unshielded,
        "expected reinforced loss {reinforced} < {unshielded}"
    );
    let expected = incoming * 0.70;
    assert!(
        (reinforced - expected).abs() <= 0.001,
        "expected {expected}, got {reinforced}"
    );
}

#[test]
fn refits_repair_nanites_warp_movement() {
    let mut g = GameState::new_game_with_seed(400);
    g.active_refits = Some(refits(&[(RefitID::damage_nanites(), 2)]));
    g.enterprise.damage[DeviceID::Warp.index()] = 2.0;
    g.enterprise.damage[DeviceID::Phasers.index()] = 0.8;
    let act = ActionMove {
        warp: 1.0,
        course: 0.0,
        ..Default::default()
    };
    act.execute(&mut g).expect("warp move failed");
    assert_eq!(g.enterprise.damage[DeviceID::Warp.index()], 1.0);
    assert_eq!(g.enterprise.damage[DeviceID::Phasers.index()], 0.0);
}

#[test]
fn refits_get_refit_definition() {
    let def = get_refit_definition(&RefitID::dilithium_core()).expect("definition");
    assert_eq!(def.name, "Dilithium Core Tuning");
    assert!(get_refit_definition(&RefitID::new("nonexistent")).is_none());
}
