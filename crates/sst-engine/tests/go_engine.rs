//! Port of pkg/engine/engine_test.go.
#![allow(clippy::needless_range_loop)] // loops mirror the Go indexing

use std::collections::BTreeMap;
use std::f64::consts::PI;

use sst_engine::json::GoVec;
use sst_engine::*;

fn new_game() -> GameState {
    GameState::new_game(12345, SkillLevel::Good, GameLength::Medium)
}

fn kl(id: i32, sector: Coord, energy: f64) -> (Ptr<Klingon>, KlingonRef) {
    let p = ptr(Klingon {
        id,
        sector,
        energy,
        ..Default::default()
    });
    let r = p.clone().unwrap();
    (p, r)
}

fn types(ev: &[Event]) -> Vec<&'static str> {
    ev.iter().map(|e| e.event_type()).collect()
}

fn torpedo_hit(e: &Event) -> EventTorpedoHit {
    match e {
        Event::TorpedoHit(h) => h.clone(),
        other => panic!("expected TorpedoHit, got {other:?}"),
    }
}

fn phaser_hit(e: &Event) -> EventPhaserHit {
    match e {
        Event::PhaserHit(h) => h.clone(),
        other => panic!("expected PhaserHit, got {other:?}"),
    }
}

fn ship_moved(e: &Event) -> EventShipMoved {
    match e {
        Event::ShipMoved(m) => m.clone(),
        other => panic!("expected ShipMoved, got {other:?}"),
    }
}

fn lr_scan(e: &Event) -> EventLRScanCompleted {
    match e {
        Event::LRScanCompleted(m) => m.clone(),
        other => panic!("expected LRScanCompleted, got {other:?}"),
    }
}

fn mv(course: f64, warp: f64) -> Action {
    ActionMove {
        course,
        warp,
        ..Default::default()
    }
    .into()
}

#[test]
fn test_dispatch_shield_transfer() {
    let mut g = new_game();
    let (e0, s0) = (g.enterprise.energy, g.enterprise.shields);
    let ev = g.dispatch(&ActionShields { amount: 500.0 }.into()).unwrap();
    assert!(g.enterprise.shields == s0 + 500.0 && g.enterprise.energy == e0 - 500.0);
    assert!(!ev.is_empty() && ev[0].event_type() == "ShieldTransfer");
    match &ev[0] {
        Event::ShieldTransfer(t) => {
            assert_eq!(t.new_shields, g.enterprise.shields);
            assert_eq!(t.new_energy, g.enterprise.energy);
        }
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn test_dispatch_shield_transfer_errors() {
    let mut g = new_game();
    g.enterprise.energy = 100.0;
    g.enterprise.shields = 50.0;
    assert!(g.dispatch(&ActionShields { amount: 200.0 }.into()).is_err());
    assert!(g
        .dispatch(&ActionShields { amount: -100.0 }.into())
        .is_err());
    let ev = g.dispatch(&ActionShields { amount: -50.0 }.into()).unwrap();
    assert_eq!(g.enterprise.shields, 0.0);
    assert_eq!(g.enterprise.energy, 150.0);
    assert!(ev.len() == 1 && ev[0].event_type() == "ShieldTransfer");
}

#[test]
fn test_dispatch_dock() {
    let mut g = new_game();
    g.enterprise.sector = [4, 4];
    g.current_quad.starbase = Some([4, 5]);
    g.current_quad.grid[4][5] = EntityType::Starbase;
    g.enterprise.energy = 2000.0;
    g.enterprise.torpedoes = 3;
    g.enterprise.condition = ConditionType::Yellow;
    g.enterprise.devices[DeviceID::Warp.index()] = 5.0;
    let ev = g.dispatch(&ActionDock.into()).unwrap();
    assert_eq!(g.enterprise.condition, ConditionType::Docked);
    assert_eq!(g.enterprise.energy, 5000.0);
    assert_eq!(g.enterprise.torpedoes, 10);
    assert_eq!(g.enterprise.devices[DeviceID::Warp.index()], 0.0);
    assert!(!ev.is_empty() && ev[0].event_type() == "Docked");
    match &ev[0] {
        Event::Docked(d) => assert_eq!(d.starbase, [4, 5]),
        other => panic!("{other:?}"),
    }
    assert!(g.dispatch(&ActionDock.into()).is_err());
}

#[test]
fn test_dispatch_dock_errors() {
    let mut g = new_game();
    g.enterprise.sector = [1, 1];
    g.current_quad.starbase = None;
    assert!(g.dispatch(&ActionDock.into()).is_err());
    g.current_quad.starbase = Some([8, 8]);
    g.current_quad.grid[8][8] = EntityType::Starbase;
    assert!(g.dispatch(&ActionDock.into()).is_err());
    g.current_quad.starbase = Some([1, 1]);
    assert!(g.dispatch(&ActionDock.into()).is_err());
}

#[test]
fn test_dispatch_fire_torpedo_hit_and_destroy_klingon() {
    let mut g = new_game();
    g.enterprise.sector = [4, 1];
    g.enterprise.torpedoes = 5;
    g.remaining_klingons = 10;
    let (p, _) = kl(1, [4, 7], 300.0);
    g.current_quad.klingons = GoVec::from_vec(vec![p]);
    g.current_quad.grid[4][7] = EntityType::Klingon;
    let ev = g
        .dispatch(
            &ActionFireTorpedo {
                angle: 0.0,
                ..Default::default()
            }
            .into(),
        )
        .unwrap();
    assert_eq!(g.enterprise.torpedoes, 4);
    assert_eq!(g.remaining_klingons, 9);
    assert_eq!(g.current_quad.grid[4][7], EntityType::Empty);
    assert!(g.current_quad.klingons.is_empty());
    assert_eq!(types(&ev), vec!["TorpedoFired", "TorpedoHit"]);
    let h = torpedo_hit(&ev[1]);
    assert!(
        h.destroyed && h.target == [4, 7] && h.entity == EntityType::Klingon,
        "{h:?}"
    );
}

#[test]
fn test_dispatch_fire_torpedo_hit_klingon_partial_damage() {
    let mut g = new_game();
    g.enterprise.sector = [4, 1];
    g.enterprise.torpedoes = 5;
    g.remaining_klingons = 10;
    let (p, k) = kl(1, [4, 7], 1000.0);
    g.current_quad.klingons = GoVec::from_vec(vec![p]);
    g.current_quad.grid[4][7] = EntityType::Klingon;
    let ev = g
        .dispatch(
            &ActionFireTorpedo {
                angle: 0.0,
                ..Default::default()
            }
            .into(),
        )
        .unwrap();
    assert_eq!(k.borrow().energy, 500.0);
    assert_eq!(g.remaining_klingons, 10);
    assert!(ev.len() == 2 && !torpedo_hit(&ev[1]).destroyed);
}

#[test]
fn test_dispatch_fire_torpedo_target_coord_bearing() {
    let mut g = new_game();
    g.enterprise.sector = [4, 1];
    g.enterprise.torpedoes = 5;
    let (p, _) = kl(1, [4, 7], 300.0);
    g.current_quad.klingons = GoVec::from_vec(vec![p]);
    g.current_quad.grid[4][7] = EntityType::Klingon;
    let ev = g
        .dispatch(
            &ActionFireTorpedo {
                target: [4, 7],
                ..Default::default()
            }
            .into(),
        )
        .unwrap();
    assert!(ev.len() == 2 && ev[1].event_type() == "TorpedoHit");
}

#[test]
fn test_dispatch_fire_torpedo_obstacles_and_starbase() {
    let mut g = new_game();
    g.enterprise.sector = [4, 1];
    g.enterprise.torpedoes = 5;
    g.current_quad.grid[4][5] = EntityType::Star;
    let east: Action = ActionFireTorpedo {
        angle: 0.0,
        ..Default::default()
    }
    .into();

    let ev = g.dispatch(&east).unwrap();
    assert_eq!(ev.len(), 2);
    let h = torpedo_hit(&ev[1]);
    assert!(h.entity == EntityType::Star && !h.destroyed);

    g.current_quad.grid[4][5] = EntityType::Planet;
    let ev = g.dispatch(&east).unwrap();
    let h = torpedo_hit(&ev[1]);
    assert!(h.entity == EntityType::Planet && !h.destroyed);

    g.current_quad.starbase = Some([4, 5]);
    g.current_quad.grid[4][5] = EntityType::Starbase;
    g.remaining_starbases = 3;
    let ev = g.dispatch(&east).unwrap();
    let h = torpedo_hit(&ev[1]);
    assert!(h.entity == EntityType::Starbase && h.destroyed);
    assert!(g.current_quad.starbase.is_none());
    assert_eq!(g.remaining_starbases, 2);

    g.enterprise.condition = ConditionType::Docked;
    g.enterprise.torpedoes = 10;
    g.current_quad.grid[4][5] = EntityType::Star;
    g.dispatch(&east).unwrap();
    assert_eq!(g.enterprise.torpedoes, 10);

    g.enterprise.condition = ConditionType::Green;
    let ev = g
        .dispatch(
            &ActionFireTorpedo {
                angle: PI / 2.0,
                ..Default::default()
            }
            .into(),
        )
        .unwrap();
    assert_eq!(types(&ev), vec!["TorpedoFired"]);

    g.enterprise.torpedoes = 0;
    assert!(g.dispatch(&east).is_err());
}

#[test]
fn test_dispatch_fire_phasers_automatic() {
    let mut g = new_game();
    g.enterprise.sector = [4, 1];
    g.enterprise.energy = 4000.0;
    g.remaining_klingons = 10;
    let (p1, _) = kl(1, [4, 3], 100.0);
    let (p2, _) = kl(2, [4, 5], 50.0);
    g.current_quad.klingons = GoVec::from_vec(vec![p1, p2]);
    g.current_quad.grid[4][3] = EntityType::Klingon;
    g.current_quad.grid[4][5] = EntityType::Klingon;
    let ev = g
        .dispatch(
            &ActionFirePhasers {
                energy: 1000.0,
                ..Default::default()
            }
            .into(),
        )
        .unwrap();
    assert_eq!(g.enterprise.energy, 3000.0);
    assert!(g.current_quad.klingons.is_empty());
    assert_eq!(g.remaining_klingons, 8);
    assert_eq!(ev.len(), 3);
    assert_eq!(ev[0].event_type(), "PhaserFired");
    let h1 = phaser_hit(&ev[1]);
    assert!(h1.destroyed && h1.klingon_id == 1);
    let h2 = phaser_hit(&ev[2]);
    assert!(h2.destroyed && h2.klingon_id == 2);

    let (p3, k3) = kl(3, [4, 3], 500.0);
    g.current_quad.klingons = GoVec::from_vec(vec![p3]);
    g.current_quad.grid[4][3] = EntityType::Klingon;
    let ev = g
        .dispatch(
            &ActionFirePhasers {
                energy: 200.0,
                ..Default::default()
            }
            .into(),
        )
        .unwrap();
    assert!(ev.len() == 2 && !phaser_hit(&ev[1]).destroyed);
    assert_eq!(k3.borrow().energy, 400.0);
}

fn alloc(pairs: &[(i32, f64)]) -> BTreeMap<i32, f64> {
    pairs.iter().copied().collect()
}

#[test]
fn test_dispatch_fire_phasers_manual_and_errors() {
    let mut g = new_game();
    g.enterprise.sector = [4, 1];
    g.enterprise.energy = 4000.0;
    let (p1, k1) = kl(1, [4, 3], 500.0);
    g.current_quad.klingons = GoVec::from_vec(vec![p1]);
    g.current_quad.grid[4][3] = EntityType::Klingon;
    let ev = g
        .dispatch(
            &ActionFirePhasers {
                energy: 200.0,
                manual_allocation: alloc(&[(1, 200.0)]),
            }
            .into(),
        )
        .unwrap();
    assert_eq!(k1.borrow().energy, 400.0);
    assert!(ev.len() == 2 && ev[1].event_type() == "PhaserHit");
    let h = phaser_hit(&ev[1]);
    assert!(!h.destroyed && h.damage == 100.0, "{h:?}");

    g.dispatch(
        &ActionFirePhasers {
            energy: 0.0,
            manual_allocation: alloc(&[(1, 200.0), (99, 100.0), (2, -50.0)]),
        }
        .into(),
    )
    .unwrap();
    assert_eq!(k1.borrow().energy, 300.0);

    assert!(g
        .dispatch(
            &ActionFirePhasers {
                energy: 0.0,
                manual_allocation: alloc(&[(1, -10.0)]),
            }
            .into()
        )
        .is_err());
    assert!(g
        .dispatch(
            &ActionFirePhasers {
                energy: 0.0,
                ..Default::default()
            }
            .into()
        )
        .is_err());
    g.enterprise.energy = 50.0;
    let hundred: Action = ActionFirePhasers {
        energy: 100.0,
        ..Default::default()
    }
    .into();
    assert!(g.dispatch(&hundred).is_err());
    g.enterprise.energy = 5000.0;
    g.enterprise.condition = ConditionType::Docked;
    assert!(g.dispatch(&hundred).is_err());
    g.enterprise.condition = ConditionType::Green;
    g.current_quad.klingons = GoVec::nil();
    assert!(g.dispatch(&hundred).is_err());
}

#[test]
fn test_dispatch_fire_phasers_manual_allocation_exploit() {
    let mut g = new_game();
    g.enterprise.sector = [4, 1];
    g.enterprise.energy = 500.0;
    let (p1, _) = kl(1, [4, 3], 500.0);
    let (p2, _) = kl(2, [4, 5], 500.0);
    g.current_quad.klingons = GoVec::from_vec(vec![p1, p2]);
    g.current_quad.grid[4][3] = EntityType::Klingon;
    g.current_quad.grid[4][5] = EntityType::Klingon;
    let a: Action = ActionFirePhasers {
        energy: 100.0,
        manual_allocation: alloc(&[(1, 300.0), (2, 300.0)]),
    }
    .into();
    assert!(g.dispatch(&a).is_err());
    g.enterprise.energy = 1000.0;
    let ev = g.dispatch(&a).unwrap();
    assert_eq!(g.enterprise.energy, 400.0);
    assert!(!ev.is_empty() && ev[0].event_type() == "PhaserFired");
    match &ev[0] {
        Event::PhaserFired(f) => assert_eq!(f.energy, 600.0),
        other => panic!("{other:?}"),
    }
}

#[test]
fn test_dispatch_move() {
    let mut g = new_game();
    g.enterprise.quad = [1, 1];
    g.enterprise.sector = [4, 1];
    g.current_quad.grid[4][1] = EntityType::Enterprise;
    g.enterprise.energy = 5000.0;
    let sd0 = g.stardate;
    let ev = g.dispatch(&mv(0.0, 0.25)).unwrap();
    assert_eq!(g.enterprise.sector, [4, 3]);
    assert_eq!(g.current_quad.grid[4][1], EntityType::Empty);
    assert_eq!(g.current_quad.grid[4][3], EntityType::Enterprise);
    assert!(g.enterprise.energy < 5000.0);
    assert!(g.stardate > sd0);
    assert!(!ev.is_empty() && ev[0].event_type() == "ShipMoved");
    let m = ship_moved(&ev[0]);
    assert!(m.from_sector == [4, 1] && m.to_sector == [4, 3]);
}

#[test]
fn test_dispatch_move_dest_sector() {
    let mut g = new_game();
    g.enterprise.sector = [2, 2];
    g.current_quad.grid[2][2] = EntityType::Enterprise;
    g.enterprise.condition = ConditionType::Docked;
    let ds = |c: Coord| -> Action {
        ActionMove {
            warp: 1.0,
            dest_sector: c,
            ..Default::default()
        }
        .into()
    };
    let ev = g.dispatch(&ds([5, 5])).unwrap();
    assert_eq!(g.enterprise.sector, [5, 5]);
    assert_eq!(g.enterprise.condition, ConditionType::Green);
    assert!(!ev.is_empty() && ev[0].event_type() == "ShipMoved");
    assert!(g.dispatch(&ds([9, 9])).is_err());
    g.current_quad.grid[6][6] = EntityType::Star;
    assert!(g.dispatch(&ds([6, 6])).is_err());
}

#[test]
fn test_dispatch_move_obstacle_collision() {
    let mut g = new_game();
    g.enterprise.sector = [4, 1];
    g.current_quad.grid[4][1] = EntityType::Enterprise;
    g.current_quad.grid[4][3] = EntityType::Star;
    let ev = g.dispatch(&mv(0.0, 0.5)).unwrap();
    assert_eq!(g.enterprise.sector, [4, 2]);
    assert_eq!(g.current_quad.grid[4][2], EntityType::Enterprise);
    assert!(types(&ev).contains(&"ObstacleEncountered"), "{ev:?}");
}

#[test]
fn test_dispatch_move_inter_quadrant() {
    let mut g = new_game();
    g.enterprise.quad = [2, 2];
    g.enterprise.sector = [4, 4];
    g.current_quad.grid[4][4] = EntityType::Enterprise;
    g.enterprise.energy = 5000.0;
    let ev = g.dispatch(&mv(0.0, 2.0)).unwrap();
    assert_eq!(g.enterprise.quad, [2, 4]);
    assert_eq!(g.enterprise.sector, [4, 4]);
    assert_eq!(g.current_quad.grid[4][4], EntityType::Enterprise);
    assert!(g.chart_discovered[2][4]);
    assert!(!ev.is_empty() && ev[0].event_type() == "ShipMoved");
    let m = ship_moved(&ev[0]);
    assert!(m.from_quad == [2, 2] && m.to_quad == [2, 4]);

    g.enterprise.quad = [1, 1];
    g.enterprise.sector = [2, 2];
    g.current_quad.grid[2][2] = EntityType::Enterprise;
    g.enterprise.energy = 5000.0;
    g.dispatch(&mv(PI / 2.0, 4.0)).unwrap();
    assert_eq!(g.enterprise.quad, [1, 1]);
    assert_eq!(g.enterprise.sector, [1, 2]);

    g.enterprise.quad = [1, 1];
    g.enterprise.sector = [4, 1];
    g.current_quad.grid[4][1] = EntityType::Enterprise;
    g.current_quad.grid[4][3] = EntityType::Star;
    g.enterprise.energy = 5000.0;
    g.dispatch(&mv(0.0, 2.0)).unwrap();
    assert_eq!(g.enterprise.quad, [1, 1]);
    assert_eq!(g.enterprise.sector, [4, 2]);
    assert_eq!(g.current_quad.grid[4][2], EntityType::Enterprise);
}

#[test]
fn test_dispatch_move_explicit_dest_quad() {
    let mut g = new_game();
    g.enterprise.quad = [1, 1];
    g.enterprise.sector = [3, 3];
    g.current_quad.grid[3][3] = EntityType::Enterprise;
    g.enterprise.energy = 5000.0;
    let mq = |q: Coord, s: Coord| -> Action {
        ActionMove {
            warp: 1.0,
            dest_quad: q,
            dest_sector: s,
            ..Default::default()
        }
        .into()
    };
    let ev = g.dispatch(&mq([5, 6], [2, 7])).unwrap();
    assert_eq!(g.enterprise.quad, [5, 6]);
    assert_eq!(g.enterprise.sector, [2, 7]);
    assert_eq!(g.current_quad.grid[3][3], EntityType::Empty);
    assert!(!ev.is_empty() && ev[0].event_type() == "ShipMoved");

    g.enterprise.quad = [1, 1];
    g.enterprise.sector = [2, 2];
    g.current_quad.grid[2][2] = EntityType::Enterprise;
    g.enterprise.energy = 5000.0;
    g.dispatch(&mq([7, 8], [0, 0])).unwrap();
    assert_eq!(g.enterprise.quad, [7, 8]);
    assert_eq!(g.enterprise.sector, [4, 4]);

    for dq in [[0, 1], [9, 5], [4, 0], [4, 9]] {
        assert!(g.dispatch(&mq(dq, [0, 0])).is_err(), "{dq:?}");
    }
    for ds in [[0, 1], [9, 5], [4, 0], [4, 9]] {
        assert!(g.dispatch(&mq([3, 3], ds)).is_err(), "{ds:?}");
    }
}

#[test]
fn test_dispatch_move_errors_and_warp_limits() {
    let mut g = new_game();
    g.enterprise.devices[DeviceID::Warp.index()] = 2.0;
    assert!(g.dispatch(&mv(0.0, 0.0)).is_err());
    assert!(g.dispatch(&mv(0.0, 5.0)).is_err());
    g.enterprise.devices[DeviceID::Warp.index()] = 10.0;
    assert!(g.dispatch(&mv(0.0, 2.0)).is_err());
    g.enterprise.devices[DeviceID::Warp.index()] = 0.0;
    g.enterprise.energy = 1.0;
    assert!(g.dispatch(&mv(0.0, 8.0)).is_err());
}

#[test]
fn test_event_types() {
    let events = vec![
        Event::ShieldTransfer(Default::default()),
        Event::TorpedoFired(Default::default()),
        Event::TorpedoHit(Default::default()),
        Event::PhaserFired(Default::default()),
        Event::PhaserHit(Default::default()),
        Event::Docked(Default::default()),
        Event::ShipMoved(Default::default()),
        Event::ObstacleEncountered(Default::default()),
        Event::ConditionChanged(Default::default()),
        Event::GameOver(EventGameOver {
            reason: GameOverReason::Won,
            score: 0.0,
        }),
        Event::KlingonCounterAttack(EventKlingonCounterAttack {
            enemy_id: 1,
            damage: 50.0,
            ..Default::default()
        }),
        Event::SubsystemDamaged(EventSubsystemDamaged {
            device: DeviceID::Warp,
            repair_time: 3.5,
        }),
        Event::SubsystemRepaired(EventSubsystemRepaired {
            device: DeviceID::Warp,
        }),
        Event::LRScanCompleted(Default::default()),
        Event::StarbaseSurveillance(Default::default()),
    ];
    let expected = [
        "ShieldTransfer",
        "TorpedoFired",
        "TorpedoHit",
        "PhaserFired",
        "PhaserHit",
        "Docked",
        "ShipMoved",
        "ObstacleEncountered",
        "ConditionChanged",
        "GameOver",
        "KlingonCounterAttack",
        "SubsystemDamaged",
        "SubsystemRepaired",
        "LRScanCompleted",
        "StarbaseSurveillance",
    ];
    for (i, e) in events.iter().enumerate() {
        assert_eq!(e.event_type(), expected[i], "event {i}");
    }
}

#[test]
fn test_action_lr_scan_operational() {
    let mut g = new_game();
    g.enterprise.quad = [3, 3];
    g.enterprise.devices[DeviceID::LRSensors.index()] = 0.0;
    let ev = g.dispatch(&ActionLRScan.into()).unwrap();
    assert_eq!(ev.len(), 1);
    let s = lr_scan(&ev[0]);
    assert!(!s.relayed_by_base);
    assert_eq!(s.scanned_quads.len(), 9);
    for dr in -1..=1i32 {
        for dc in -1..=1i32 {
            assert!(g.chart_discovered[(3 + dr) as usize][(3 + dc) as usize]);
        }
    }
}

#[test]
fn test_action_lr_scan_damaged_undocked() {
    let mut g = new_game();
    g.enterprise.devices[DeviceID::LRSensors.index()] = 2.5;
    g.enterprise.condition = ConditionType::Green;
    assert!(g.dispatch(&ActionLRScan.into()).is_err());
}

#[test]
fn test_action_lr_scan_damaged_docked() {
    let mut g = new_game();
    g.enterprise.devices[DeviceID::LRSensors.index()] = 2.5;
    g.enterprise.condition = ConditionType::Docked;
    let ev = g.dispatch(&ActionLRScan.into()).unwrap();
    assert!(lr_scan(&ev[0]).relayed_by_base);
}

#[test]
fn test_action_lr_scan_corner_quadrant() {
    let mut g = new_game();
    g.enterprise.quad = [1, 1];
    g.enterprise.devices[DeviceID::LRSensors.index()] = 0.0;
    let ev = g.dispatch(&ActionLRScan.into()).unwrap();
    let s = lr_scan(&ev[0]);
    assert_eq!(s.scanned_quads.len(), 4);
    for c in [[1, 1], [1, 2], [2, 1], [2, 2]] {
        assert!(g.chart_discovered[c[0]][c[1]], "{c:?}");
    }
}

#[test]
fn test_action_dock_surveillance() {
    let mut g = new_game();
    g.current_quad.starbase = Some([4, 4]);
    g.current_quad.grid[4][4] = EntityType::Starbase;
    g.enterprise.sector = [4, 5];
    g.enterprise.condition = ConditionType::Green;
    g.galaxy_chart[5][5] = 15;
    let ev = g.dispatch(&ActionDock.into()).unwrap();
    let mut found = false;
    for e in &ev {
        if let Event::StarbaseSurveillance(s) = e {
            found = true;
            assert_eq!(s.starbase_coord, [4, 4]);
        }
    }
    assert!(found, "EventStarbaseSurveillance was not emitted");
    for dr in -1..=1i32 {
        for dc in -1..=1i32 {
            assert!(g.chart_discovered[(5 + dr) as usize][(5 + dc) as usize]);
        }
    }
}

#[test]
fn test_dispatch_move_inter_quadrant_populates_grid() {
    let mut g = new_game();
    g.enterprise.quad = [1, 1];
    g.enterprise.sector = [2, 2];
    g.current_quad.grid[2][2] = EntityType::Enterprise;
    g.enterprise.energy = 5000.0;
    let (dq, ds): (Coord, Coord) = ([3, 5], [4, 4]);
    g.galaxy_chart[3][5] = 214;
    let jump = |q: Coord, s: Coord| -> Action {
        ActionMove {
            dest_quad: q,
            dest_sector: s,
            warp: 1.0,
            ..Default::default()
        }
        .into()
    };
    let ev = g.dispatch(&jump(dq, ds)).unwrap();
    assert_eq!(g.enterprise.quad, dq);
    assert_eq!(g.enterprise.sector, ds);
    assert_eq!(g.current_quad.grid[4][4], EntityType::Enterprise);
    let sb = g.current_quad.starbase.expect("starbase");
    assert_eq!(
        g.current_quad.grid[sb[0] as usize][sb[1] as usize],
        EntityType::Starbase
    );
    assert_eq!(g.current_quad.klingons.len(), 2);
    for k in g.current_quad.klingons.iter() {
        let s = k.as_ref().unwrap().borrow().sector;
        let e = g.current_quad.grid[s[0] as usize][s[1] as usize];
        assert!(
            e == EntityType::Klingon || e == EntityType::Commander,
            "{e:?}"
        );
    }
    assert_eq!(g.current_quad.stars.len(), 4);
    for s in g.current_quad.stars.iter() {
        assert_eq!(
            g.current_quad.grid[s[0] as usize][s[1] as usize],
            EntityType::Star
        );
    }
    assert_eq!(g.enterprise.condition, ConditionType::Red);
    assert!(g.chart_discovered[3][5]);
    let m = ship_moved(&ev[0]);
    assert!(m.from_quad == [1, 1] && m.to_quad == dq);

    let kid = g.current_quad.klingons[0].as_ref().unwrap().borrow().id;
    let pe = g
        .dispatch(
            &ActionFirePhasers {
                energy: 0.0,
                manual_allocation: alloc(&[(kid, 2000.0)]),
            }
            .into(),
        )
        .unwrap();
    assert!(
        pe.iter()
            .any(|e| matches!(e, Event::PhaserHit(h) if h.destroyed)),
        "{pe:?}"
    );
    assert_eq!(g.galaxy_chart[3][5], 114);

    g.dispatch(&jump([1, 1], [2, 2])).unwrap();
    g.dispatch(&jump(dq, ds)).unwrap();
    assert_eq!(g.current_quad.klingons.len(), 1);
}
