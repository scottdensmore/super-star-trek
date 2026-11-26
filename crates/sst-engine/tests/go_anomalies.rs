//! Port of pkg/engine/anomalies_test.go.
#![allow(clippy::needless_range_loop)] // loops mirror the Go indexing

use sst_engine::*;

#[test]
fn test_anomalies_generation_disabled_by_default() {
    let g = GameState::new_game(42, SkillLevel::Good, GameLength::Medium);
    assert!(!g.rules.spatial_anomalies);
    for r in 1..=8 {
        for c in 1..=8 {
            assert_eq!(g.quadrant_env[r][c], EnvironmentType::Normal, "[{r},{c}]");
        }
    }
}

#[test]
fn test_anomalies_generation_enabled() {
    let rules = default_rules_for_profile(&DifficultyProfile::hardcore());
    assert!(rules.spatial_anomalies);
    let g = GameState::new_game_with_options(1337, SkillLevel::Expert, GameLength::Medium, rules);
    let (mut neb, mut ion) = (0, 0);
    for r in 1..=8 {
        for c in 1..=8 {
            match g.quadrant_env[r][c] {
                EnvironmentType::Nebula => neb += 1,
                EnvironmentType::IonStorm => ion += 1,
                _ => {}
            }
        }
    }
    assert!((2..=4).contains(&neb), "nebulae {neb}");
    assert!((2..=4).contains(&ion), "ion storms {ion}");
    let q = g.enterprise.quad;
    assert_eq!(
        g.quadrant_env[q[0] as usize][q[1] as usize],
        EnvironmentType::Normal
    );
}
