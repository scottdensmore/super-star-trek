//! Port of pkg/engine/rules_test.go.
#![allow(clippy::needless_range_loop)] // loops mirror the Go indexing

use sst_engine::persist;
use sst_engine::*;

fn temp_dir(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("sst-go-rules-{}-{}", std::process::id(), name));
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn test_difficulty_presets() {
    let c = default_rules_for_profile(&DifficultyProfile::casual());
    assert!(
        c.surveillance == SurveillanceMode::full()
            && !c.sensor_degradation
            && c.repair_multiplier == 0.75
            && !c.klingon_cloak
            && c.time_margin == 1.25,
        "casual {c:?}"
    );
    let n = default_rules_for_profile(&DifficultyProfile::normal());
    assert!(
        n.surveillance == SurveillanceMode::classic()
            && n.sensor_degradation
            && n.repair_multiplier == 1.0
            && !n.klingon_cloak
            && n.time_margin == 1.0,
        "normal {n:?}"
    );
    let h = default_rules_for_profile(&DifficultyProfile::hardcore());
    assert!(
        h.surveillance == SurveillanceMode::local()
            && h.sensor_degradation
            && h.repair_multiplier == 1.5
            && h.klingon_cloak
            && h.time_margin == 0.8,
        "hardcore {h:?}"
    );
    let m = default_rules_for_profile(&DifficultyProfile::nightmare());
    assert!(
        m.surveillance == SurveillanceMode::blackout()
            && m.sensor_degradation
            && m.repair_multiplier == 2.0
            && m.klingon_cloak
            && m.time_margin == 0.6,
        "nightmare {m:?}"
    );
}

fn known_bases(g: &GameState) -> i32 {
    let mut n = 0;
    for r in 1..=8 {
        for c in 1..=8 {
            if g.chart_known_bases[r][c] {
                n += 1;
            }
        }
    }
    n
}

#[test]
fn test_new_game_with_options_blackout_hides_starbases() {
    let rules = default_rules_for_profile(&DifficultyProfile::nightmare());
    let g = GameState::new_game_with_options(12345, SkillLevel::Good, GameLength::Medium, rules);
    assert_eq!(g.rules.surveillance, SurveillanceMode::blackout());
    assert_eq!(g.time_remaining, 30.0 * 0.60);
    assert_eq!(known_bases(&g), 0);
}

#[test]
fn test_new_game_with_options_normal_charts_starbases() {
    let rules = default_rules_for_profile(&DifficultyProfile::normal());
    let g = GameState::new_game_with_options(12345, SkillLevel::Good, GameLength::Medium, rules);
    assert_eq!(known_bases(&g), g.remaining_starbases);
}

#[test]
fn test_save_load_game_rules_persistence_and_backward_compatibility() {
    let dir = temp_dir("persist");
    let save_path = dir.join("TESTSAVE.TRK");
    let rules = GameRules {
        profile: DifficultyProfile::custom(),
        surveillance: SurveillanceMode::local(),
        sensor_degradation: true,
        repair_multiplier: 1.75,
        klingon_cloak: true,
        time_margin: 0.9,
        ..Default::default()
    };
    let g =
        GameState::new_game_with_options(999, SkillLevel::Expert, GameLength::Long, rules.clone());
    persist::save_state(&g, save_path.to_str().unwrap()).expect("save");
    let loaded = persist::load_game(save_path.to_str().unwrap()).expect("load");
    assert_eq!(loaded.borrow().rules, rules);

    // Go marshals map[string]interface{} with sorted keys.
    let data = r#"{"Length":2,"RemainingKlingons":15,"RemainingStarbases":3,"Skill":3}"#;
    let old_path = dir.join("OLDSAVE.TRK");
    std::fs::write(&old_path, data).unwrap();
    let old = persist::load_game(old_path.to_str().unwrap()).expect("load old");
    let r = &old.borrow().rules;
    assert!(
        r.profile == DifficultyProfile::normal() && r.surveillance == SurveillanceMode::classic(),
        "{r:?}"
    );
}
