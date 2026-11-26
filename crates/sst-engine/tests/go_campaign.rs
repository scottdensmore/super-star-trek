//! Port of pkg/engine/campaign_test.go.

use sst_engine::*;

#[test]
fn new_tour_initializes_four_sectors() {
    let tour = TourState::new_at(42, 0);
    assert!(tour.active);
    assert_eq!(tour.sectors.len(), 4);
    assert_eq!(tour.current_sector_index, 0);
    assert_eq!(tour.requisition_points, 0);
    assert_eq!(
        tour.sectors[0].objective,
        SectorObjectiveType::border_patrol()
    );
}

#[test]
fn tour_state_sector_progression_and_bounty() {
    let mut tour = TourState::new_at(12345, 0);
    let state = tour.start_current_sector(None).expect("sector 1");
    assert!(!tour.in_drydock);
    {
        let mut g = state.borrow_mut();
        for row in g.galaxy.iter_mut() {
            for q in row.iter_mut().flatten() {
                q.klingons = 0;
                q.commanders = 0;
                q.super_commanders = 0;
            }
        }
        g.klingons_remaining = 0;
    }
    let (cleared, failed, bounty) = tour.evaluate_sector();
    assert!(cleared, "expected sector to be cleared");
    assert!(!failed);
    assert!(bounty > 0, "expected positive bounty, got {bounty}");

    tour.advance_to_drydock(bounty);
    assert!(tour.in_drydock);
    assert_eq!(tour.requisition_points, bounty);
    assert_eq!(tour.sectors_completed, 1);

    let next = tour.disembark_to_next_sector().expect("disembark");
    assert!(next.is_some(), "expected non-nil nextState");
    assert!(!tour.in_drydock);
    assert_eq!(tour.current_sector_index, 1);
}

#[test]
fn tour_state_permadeath_on_loss() {
    let mut tour = TourState::new_at(999, 0);
    let state = tour.start_current_sector(None).unwrap();
    {
        let mut g = state.borrow_mut();
        g.game_over = true;
        g.game_over_reason = GameOverReason::Destroyed;
    }
    let (cleared, failed, _) = tour.evaluate_sector();
    assert!(!cleared);
    assert!(failed);
    assert!(!tour.active);
    assert!(tour.failed);
}

#[test]
fn new_tour_zero_seed_uses_time() {
    let tour = persist::new_tour(0);
    assert_ne!(tour.seed, 0, "expected non-zero seed when passing 0");
    assert!(!tour.id.0.is_empty());
}

#[test]
fn tour_state_current_sector_bounds() {
    let mut tour = TourState::new_at(42, 0);
    tour.current_sector_index = -1;
    assert!(tour.current_sector().is_none());
    assert!(tour.start_current_sector(None).is_err());
    tour.current_sector_index = 10;
    assert!(tour.current_sector().is_none());
    assert!(tour.start_current_sector(None).is_err());
}

#[test]
fn tour_state_evaluate_sector_nil_state() {
    let mut tour = TourState::new_at(42, 0);
    assert_eq!(tour.evaluate_sector(), (false, false, 0));
}

#[test]
fn tour_state_evaluate_sector_flawless_bonus() {
    let mut tour = TourState::new_at(100, 0);
    let state = tour.start_current_sector(None).unwrap();
    state.borrow_mut().klingons_remaining = 0;
    let (cleared, failed, no_damage) = tour.evaluate_sector();
    assert!(cleared && !failed);
    state.borrow_mut().enterprise.damage[0] = 2.0;
    let (_, _, with_damage) = tour.evaluate_sector();
    assert_eq!(no_damage - with_damage, 250, "{no_damage} vs {with_damage}");
}

#[test]
fn tour_state_disembark_to_next_sector_not_docked() {
    let mut tour = TourState::new_at(100, 0);
    let _ = tour.start_current_sector(None);
    assert!(tour.disembark_to_next_sector().is_err());
}

#[test]
fn tour_state_complete_tour_sequence() {
    let mut tour = TourState::new_at(100, 0);
    let n = tour.sectors.len();
    for i in 0..n {
        let state = tour
            .start_current_sector(None)
            .unwrap_or_else(|e| panic!("sector {}: {e}", i + 1));
        state.borrow_mut().klingons_remaining = 0;
        let (cleared, failed, bounty) = tour.evaluate_sector();
        assert!(cleared && !failed, "sector {} evaluation failed", i + 1);
        tour.advance_to_drydock(bounty);
        let next = tour.disembark_to_next_sector().expect("disembark");
        if i == n - 1 {
            assert!(next.is_none());
            assert!(tour.completed);
            assert!(!tour.active);
            assert!(!tour.in_drydock);
        } else {
            assert!(next.is_some(), "expected next state for sector {}", i + 2);
        }
    }
    assert_eq!(tour.sectors_completed, 4);
}

fn factions(g: &GameRef) -> Vec<(FactionType, bool)> {
    g.borrow()
        .current_quad
        .enemies
        .iter()
        .flatten()
        .map(|e| {
            let e = e.borrow();
            (e.faction, e.is_commander)
        })
        .collect()
}

#[test]
fn campaign_sector_faction_populations() {
    let mut tour = TourState::new_at(112233, 0);

    let g1 = tour.start_current_sector(Some(0)).unwrap();
    assert!(
        factions(&g1).iter().any(|f| f.0 == FactionType::Romulan),
        "expected Romulans in sector 1"
    );

    let g2 = tour.start_current_sector(Some(1)).unwrap();
    assert!(
        factions(&g2).iter().any(|f| f.0 == FactionType::Tholian),
        "expected Tholians in sector 2"
    );

    let g3 = tour.start_current_sector(Some(2)).unwrap();
    let f3 = factions(&g3);
    let kc = f3.iter().filter(|f| f.0 == FactionType::Klingon).count();
    let cmd = f3.iter().any(|f| f.0 == FactionType::Klingon && f.1);
    assert!(cmd && kc == 3, "count={kc} hasCommander={cmd}");
    assert_eq!(g3.borrow().current_quad.klingons.len(), 3);

    let g4 = tour.start_current_sector(Some(3)).unwrap();
    let f4 = factions(&g4);
    let kcmd = f4.iter().any(|f| f.0 == FactionType::Klingon && f.1);
    let rom = f4.iter().any(|f| f.0 == FactionType::Romulan);
    let th = f4.iter().filter(|f| f.0 == FactionType::Tholian).count();
    assert!(
        kcmd && rom && th == 2,
        "KlingonCommander={kcmd} Romulan={rom} Tholians={th}"
    );
    assert_eq!(g4.borrow().current_quad.klingons.len(), 1);
}
