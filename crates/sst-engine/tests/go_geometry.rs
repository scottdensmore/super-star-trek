//! Port of pkg/engine/geometry_test.go.
#![allow(clippy::needless_range_loop)] // loops mirror the Go indexing

use sst_engine::persist;
use sst_engine::*;
use std::f64::consts::PI;

fn temp_dir(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("sst-go-geometry-{}-{}", std::process::id(), name));
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn test_coordinate_geometry() {
    let dist = distance([1, 1], [4, 5]);
    assert!((dist - 5.0).abs() <= 0.0001, "got {dist}");
    let game = GameState::new_game(12345, SkillLevel::Good, GameLength::Medium);
    assert!(game.enterprise.energy > 0.0 && game.enterprise.torpedoes == 10);
}

#[test]
fn test_coord_accessors() {
    let c: Coord = [3, 7];
    assert_eq!(row(c), 3);
    assert_eq!(col(c), 7);
}

#[test]
fn test_distance_calculations() {
    let cases: [(&str, Coord, Coord, f64); 5] = [
        ("identical coords", [3, 3], [3, 3], 0.0),
        ("horizontal distance", [2, 1], [2, 6], 5.0),
        ("vertical distance", [1, 4], [7, 4], 6.0),
        ("diagonal 3-4-5", [1, 1], [4, 5], 5.0),
        (
            "full board diagonal",
            [1, 1],
            [8, 8],
            gomath::hypot(7.0, 7.0),
        ),
    ];
    for (name, a, b, want) in cases {
        let got = distance(a, b);
        assert!(
            (got - want).abs() <= 0.0001,
            "{name}: got {got}, want {want}"
        );
    }
}

#[test]
fn test_bearing_calculations() {
    let cases: [(&str, Coord, Coord, f64); 5] = [
        ("due east (right)", [4, 1], [4, 7], 0.0),
        ("due north (up)", [4, 1], [1, 1], PI / 2.0),
        ("due west (left)", [4, 7], [4, 1], PI),
        ("due south (down)", [1, 1], [4, 1], 3.0 * PI / 2.0),
        ("northeast (45 deg)", [4, 4], [1, 7], PI / 4.0),
    ];
    for (name, a, b, want) in cases {
        let got = bearing(a, b);
        assert!(
            (got - want).abs() <= 0.0001,
            "{name}: got {got}, want {want}"
        );
    }
}

#[test]
fn test_new_game_initialization() {
    let g = GameState::new_game(42, SkillLevel::Expert, GameLength::Long);
    assert_eq!(g.skill, SkillLevel::Expert);
    assert_eq!(g.length, GameLength::Long);
    assert_eq!(g.enterprise.energy, 5000.0);
    assert_eq!(g.enterprise.shields, 0.0);
    assert_eq!(g.enterprise.torpedoes, 10);
    assert_eq!(g.enterprise.condition, ConditionType::Green);
    assert!(g.stardate >= 2000.0 && g.stardate < 3000.0);
    assert_eq!(g.initial_stardate, g.stardate);
    assert_eq!(g.time_remaining, 30.0);
    assert_eq!(g.remaining_klingons, 15);
    assert_eq!(g.remaining_starbases, 3);
    for c in [g.enterprise.quad, g.enterprise.sector] {
        assert!((1..=8).contains(&c[0]) && (1..=8).contains(&c[1]), "{c:?}");
    }
}

#[test]
fn test_enums_and_constants() {
    assert_eq!(EntityType::Empty.as_i32(), 0);
    assert_eq!(EntityType::BlackHole.as_i32(), 8);
    assert_eq!(ConditionType::Green.as_i32(), 0);
    assert_eq!(ConditionType::Docked.as_i32(), 3);
    assert_eq!(DeviceID::Warp.as_i32(), 0);
    assert_eq!(NUM_DEVICES, 8);
    assert_eq!(SkillLevel::Novice.as_i32(), 1);
    assert_eq!(SkillLevel::Emeritus.as_i32(), 5);
    assert_eq!(GameLength::Short.as_i32(), 1);
    assert_eq!(GameLength::Long.as_i32(), 3);
}

#[test]
fn test_procedural_galaxy_generation() {
    let g = GameState::new_game(12345, SkillLevel::Good, GameLength::Medium);
    let (mut bases, mut kl) = (0, 0);
    for r in 1..=8 {
        for c in 1..=8 {
            let v = g.galaxy_chart[r][c];
            let (k, b, s) = (v / 100, (v % 100) / 10, v % 10);
            assert!((1..=9).contains(&s), "[{r},{c}] stars {s}");
            bases += b;
            kl += k;
            if b > 0 {
                assert!(g.chart_known_bases[r][c], "[{r},{c}] base not known");
            }
        }
    }
    assert_eq!(bases, g.remaining_starbases);
    assert_eq!(kl, g.remaining_klingons);
    let q = g.enterprise.quad;
    assert!(g.chart_discovered[q[0] as usize][q[1] as usize]);
}

#[test]
fn test_save_roundtrip_discovery_and_bases() {
    let dir = temp_dir("disc");
    let path = dir.join("TESTDISC.TRK");
    let mut g = GameState::new_game(12345, SkillLevel::Good, GameLength::Medium);
    g.chart_discovered[2][3] = true;
    g.chart_known_bases[4][5] = true;
    persist::save_state(&g, path.to_str().unwrap()).expect("save");
    let loaded = persist::load_game(path.to_str().unwrap()).expect("load");
    let l = loaded.borrow();
    assert_eq!(l.galaxy_chart, g.galaxy_chart);
    assert_eq!(l.chart_discovered, g.chart_discovered);
    assert_eq!(l.chart_known_bases, g.chart_known_bases);
}
