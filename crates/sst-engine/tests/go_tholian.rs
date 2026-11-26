//! Port of pkg/engine/tholian_test.go.

use sst_engine::json::GoVec;
use sst_engine::*;

fn web(coord: Coord, strength: f64) -> Ptr<TholianWebSegment> {
    ptr(TholianWebSegment { coord, strength })
}

fn tholian(id: i32, sector: Coord, energy: f64, special_state: i32) -> Ptr<EnemyVessel> {
    ptr(EnemyVessel {
        id,
        faction: FactionType::Tholian,
        sector,
        energy,
        special_state,
        ..Default::default()
    })
}

#[test]
fn tholian_binary_spinning_and_web_placement() {
    let mut g = GameState::new_game_with_seed(9999);
    g.enterprise.sector = [4, 4];
    g.current_quad.enemies = GoVec::from_vec(vec![
        tholian(201, [2, 2], 800.0, 0),
        tholian(202, [6, 6], 800.0, 1),
    ]);
    g.current_quad.grid[2][2] = EntityType::Tholian;
    g.current_quad.grid[6][6] = EntityType::Tholian;

    let events = tholian_turn(&mut g);
    assert!(!events.is_empty());
    assert!(g.current_quad.web_segments.len() >= 2);
    let c = calculate_web_containment(Some(&g));
    assert!(c > 0.0 && c <= 100.0, "{c}");
    let first = g.current_quad.web_segments[0]
        .as_ref()
        .unwrap()
        .borrow()
        .coord;
    let (destroyed, ev) = damage_web_segment(&mut g, first, 300.0);
    assert!(destroyed && !ev.is_empty());
}

#[test]
fn tholian_spinner_destruction_cascade() {
    let mut g = GameState::new_game_with_seed(9999);
    g.current_quad.web_segments = GoVec::from_vec(vec![web([1, 1], 250.0), web([1, 2], 250.0)]);
    g.current_quad.grid[1][1] = EntityType::TholianWeb;
    g.current_quad.grid[1][2] = EntityType::TholianWeb;
    g.current_quad.enemies = GoVec::nil();
    let events = tholian_turn(&mut g);
    assert!(g.current_quad.web_segments.is_empty());
    assert_eq!(g.current_quad.grid[1][1], EntityType::Empty);
    assert!(!events.is_empty());
}

#[test]
fn tholian_destroyed_spinners_cascade() {
    let mut g = GameState::new_game_with_seed(9999);
    g.current_quad.web_segments = GoVec::from_vec(vec![web([1, 1], 250.0), web([1, 2], 250.0)]);
    g.current_quad.grid[1][1] = EntityType::TholianWeb;
    g.current_quad.grid[1][2] = EntityType::TholianWeb;
    g.current_quad.enemies = GoVec::from_vec(vec![
        tholian(201, [2, 2], 0.0, 0),
        tholian(202, [6, 6], -50.0, 1),
        None,
    ]);
    let events = tholian_turn(&mut g);
    assert!(g.current_quad.web_segments.is_empty());
    assert_eq!(g.current_quad.grid[1][1], EntityType::Empty);
    assert_eq!(g.current_quad.grid[1][2], EntityType::Empty);
    assert!(events.len() == 1 && events[0].event_type() == "WebCollapsed");
}

#[test]
fn tholian_perimeter_navigation() {
    assert_eq!(get_perimeter_step([1, 2], 0), (0, 1));
    assert_eq!(get_perimeter_step([2, 8], 0), (1, 0));
    assert_eq!(get_perimeter_step([8, 7], 0), (0, -1));
    assert_eq!(get_perimeter_step([7, 1], 0), (-1, 0));
    assert_eq!(get_perimeter_step([1, 7], 1), (0, -1));
    assert_eq!(get_perimeter_step([2, 1], 1), (1, 0));
    assert_eq!(get_perimeter_step([8, 2], 1), (0, 1));
    assert_eq!(get_perimeter_step([7, 8], 1), (-1, 0));
}

#[test]
fn tholian_web_damage_and_breach() {
    let mut g = GameState::new_game_with_seed(9999);
    g.current_quad.web_segments = GoVec::from_vec(vec![web([3, 3], 250.0)]);
    g.current_quad.grid[3][3] = EntityType::TholianWeb;
    // DamageWebSegment(nil, ...) is not representable.

    let (d, ev) = damage_web_segment(&mut g, [3, 3], 0.0);
    assert!(!d && ev.is_empty());
    let (d, ev) = damage_web_segment(&mut g, [3, 3], -50.0);
    assert!(!d && ev.is_empty());

    let (d, ev) = damage_web_segment(&mut g, [3, 3], 100.0);
    assert!(!d && ev.is_empty());
    assert_eq!(
        g.current_quad.web_segments[0]
            .as_ref()
            .unwrap()
            .borrow()
            .strength,
        150.0
    );
    assert_eq!(g.current_quad.grid[3][3], EntityType::TholianWeb);

    let (d, ev) = damage_web_segment(&mut g, [1, 1], 100.0);
    assert!(!d && ev.is_empty());

    let (d, ev) = damage_web_segment(&mut g, [3, 3], 150.0);
    assert!(d && ev.len() == 1);
    assert_eq!(g.current_quad.grid[3][3], EntityType::Empty);
    assert!(g.current_quad.web_segments.is_empty());
    assert_eq!(ev[0].event_type(), "WebBreached");

    g.current_quad.web_segments = GoVec::from_vec(vec![None, web([5, 5], 250.0)]);
    g.current_quad.grid[5][5] = EntityType::TholianWeb;
    let (d, ev) = damage_web_segment(&mut g, [5, 5], 300.0);
    assert!(d && ev.len() == 1);
    assert!(g.current_quad.web_segments.len() == 1 && g.current_quad.web_segments[0].is_none());
}

#[test]
fn tholian_calculate_web_containment() {
    assert_eq!(calculate_web_containment(None), 0.0);
    let mut g = GameState::new_game_with_seed(9999);
    g.current_quad.web_segments = GoVec::nil();
    assert_eq!(calculate_web_containment(Some(&g)), 0.0);
    for i in 1..=10 {
        g.current_quad.web_segments.push(web([1, i % 8 + 1], 250.0));
    }
    assert_eq!(calculate_web_containment(Some(&g)), 50.0);
    for i in 11..=20 {
        g.current_quad.web_segments.push(web([2, i % 8 + 1], 250.0));
    }
    assert_eq!(calculate_web_containment(Some(&g)), 100.0);
    for i in 21..=25 {
        g.current_quad.web_segments.push(web([3, i % 8 + 1], 250.0));
    }
    assert_eq!(calculate_web_containment(Some(&g)), 100.0);
}

#[test]
fn tholian_movement_blocked_by_obstacle() {
    let mut g = GameState::new_game_with_seed(9999);
    let spinner = tholian(201, [1, 2], 800.0, 0);
    g.current_quad.enemies = GoVec::from_vec(vec![spinner.clone()]);
    g.current_quad.grid[1][2] = EntityType::Tholian;
    g.current_quad.grid[1][3] = EntityType::Star;
    let events = tholian_turn(&mut g);
    assert!(events.is_empty());
    assert_eq!(spinner.unwrap().borrow().sector, [1, 2]);
    assert!(g.current_quad.web_segments.is_empty());
}

// TestTholian_NilGameState: TholianTurn(nil) is not representable (takes &mut GameState).

#[test]
fn tholian_event_types() {
    let e1 = Event::WebSegmentLaid(EventWebSegmentLaid {
        spinner_id: 1,
        coord: [2, 3],
    });
    assert_eq!(e1.event_type(), "WebSegmentLaid");
    let e2 = Event::WebBreached(EventWebBreached { coord: [2, 3] });
    assert_eq!(e2.event_type(), "WebBreached");
    let e3 = Event::WebCollapsed(EventWebCollapsed { segments_count: 5 });
    assert_eq!(e3.event_type(), "WebCollapsed");
}
