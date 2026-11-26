//! Port of pkg/engine/score_test.go.

use std::collections::BTreeMap;

use sst_engine::json::GoVec;
use sst_engine::*;

#[test]
fn score_calculation_matching_classic_rules() {
    let mut g = GameState::new_game(12345, SkillLevel::Good, GameLength::Medium);
    g.initial_stardate = 2000.0;
    g.stardate = 2010.0;
    let m = &mut g.metrics;
    m.klingons_killed = 8;
    m.commanders_killed = 2;
    m.super_commanders_killed = 1;
    m.romulans_killed = 1;
    m.romulans_surrendered = 2;
    m.starbases_destroyed = 1;
    m.stars_destroyed = 1;
    m.planets_destroyed = 1;
    m.casualties = 15;
    m.help_calls = 1;
    m.starships_lost = 0;
    let s = compute_score(Some(&g), true);
    assert_eq!(s.klingon_points, 80);
    assert_eq!(s.commander_points, 100);
    assert_eq!(s.super_commander_points, 200);
    assert_eq!(s.kill_rate_points, 550, "kill rate {}", s.kill_rate);
    assert_eq!(s.win_bonus, 300);
    assert_eq!(s.total_score, 1077);
    assert_eq!(s.rank_badge, "[FADM]");
}

#[test]
fn score_minimum_five_stardates_clamping_when_lost() {
    let mut g = GameState::new_game(12345, SkillLevel::Good, GameLength::Medium);
    g.initial_stardate = 2000.0;
    g.stardate = 2001.0;
    g.metrics.klingons_killed = 2;
    let s = compute_score(Some(&g), false);
    assert_eq!(s.elapsed_stardates, 5.0);
    assert_eq!(s.kill_rate_points, 200);
    assert_eq!(s.win_bonus, 0);
    assert_eq!(s.total_score, 220);
    assert_eq!(s.rank_badge, "[CDR]");
}

#[test]
fn score_ranks() {
    let tests = [
        (-10, "[DISHONOR]", "Dishonorable Discharge"),
        (50, "[CADET]", "Starfleet Cadet"),
        (150, "[LT]", "Lieutenant"),
        (250, "[CDR]", "Commander"),
        (400, "[CAPT]", "Captain"),
        (600, "[COMM]", "Commodore"),
        (800, "[RADM]", "Rear Admiral"),
        (1200, "[FADM]", "Fleet Admiral"),
    ];
    for (score, badge, title) in tests {
        assert_eq!(rank_for_score(score), (title, badge), "score {score}");
    }
}

#[test]
fn score_skill_win_bonuses() {
    let skills = [
        (SkillLevel::Novice, 100),
        (SkillLevel::Fair, 200),
        (SkillLevel::Good, 300),
        (SkillLevel::Expert, 400),
        (SkillLevel::Emeritus, 500),
    ];
    for (skill, want) in skills {
        let mut g = GameState::new_game(12345, skill, GameLength::Medium);
        g.initial_stardate = 2000.0;
        g.stardate = 2010.0;
        assert_eq!(compute_score(Some(&g), true).win_bonus, want, "{skill:?}");
    }
}

#[test]
fn score_metric_tracking_in_combat_and_actions() {
    let mut g = GameState::new_game(12345, SkillLevel::Good, GameLength::Medium);
    g.enterprise.energy = 5000.0;
    g.enterprise.torpedoes = 10;
    g.enterprise.sector = [1, 1];
    g.current_quad.klingons = GoVec::from_vec(vec![
        ptr(Klingon {
            id: 1,
            sector: [1, 2],
            energy: 200.0,
            ..Default::default()
        }),
        ptr(Klingon {
            id: 2,
            sector: [1, 3],
            energy: 200.0,
            is_commander: true,
            ..Default::default()
        }),
    ]);
    g.current_quad.grid[1][2] = EntityType::Klingon;
    g.current_quad.grid[1][3] = EntityType::Commander;

    let torp = ActionFireTorpedo {
        angle: 0.0,
        ..Default::default()
    };
    torp.execute(&mut g).unwrap();
    assert_eq!(g.metrics.klingons_killed, 1);

    let mut alloc = BTreeMap::new();
    alloc.insert(2, 1000.0);
    ActionFirePhasers {
        manual_allocation: alloc,
        ..Default::default()
    }
    .execute(&mut g)
    .unwrap();
    assert_eq!(g.metrics.commanders_killed, 1);

    g.current_quad.starbase = Some([1, 4]);
    g.current_quad.grid[1][4] = EntityType::Starbase;
    torp.execute(&mut g).unwrap();
    assert_eq!(g.metrics.starbases_destroyed, 1);

    ActionCallHelp.execute(&mut g).unwrap();
    assert_eq!(g.metrics.help_calls, 1);
}
