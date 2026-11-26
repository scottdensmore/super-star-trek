//! Mutara Nebula scenario (port of `pkg/engine/scenario_mutara.go`).

use crate::json::GoVec;
use crate::prng::PRNG;
use crate::rules::{default_rules_for_profile, DifficultyProfile};
use crate::scenario_kobayashi::find_empty_sector;
use crate::scenarios::ScenarioID;
use crate::score::{compute_score, ScoreBreakdown};
use crate::state::{ptr, GameState, Klingon, QuadrantState};
use crate::types::*;

/// Go `BuildMutaraNebula`.
pub fn build_mutara_nebula(seed: i64) -> GameState {
    let rng = PRNG::new(seed);
    let mut rules = default_rules_for_profile(&DifficultyProfile::normal());
    rules.klingon_cloak = true;
    let mut g =
        GameState::new_game_with_options(seed, SkillLevel::Expert, GameLength::Short, rules);
    g.rng = Some(rng);
    g.scenario = ScenarioID::mutara_nebula();
    g.enterprise.quad = [5, 5];
    g.enterprise.sector = [4, 4];
    g.enterprise.energy = 5000.0;
    g.enterprise.shields = 0.0;
    g.enterprise.torpedoes = 10;
    g.enterprise.condition = ConditionType::Red;
    g.quadrant_env[5][5] = EnvironmentType::Nebula;

    g.remaining_starbases = 0;
    for r in 1..=8 {
        for c in 1..=8 {
            g.galaxy_chart[r][c] %= 10;
            g.chart_known_bases[r][c] = false;
        }
    }

    g.current_quad = QuadrantState::default();
    g.current_quad.grid[4][4] = EntityType::Enterprise;

    let cmd = find_empty_sector(&mut g);
    g.current_quad.grid[cmd[0] as usize][cmd[1] as usize] = EntityType::SuperCommander;
    g.current_quad.klingons = GoVec::from_vec(vec![ptr(Klingon {
        id: 1,
        sector: cmd,
        energy: 1200.0,
        shields: 0.0,
        is_commander: true,
        is_cloaked: true,
    })]);

    g.current_quad.stars = GoVec::empty();
    for _ in 0..3 {
        let sec = find_empty_sector(&mut g);
        g.current_quad.grid[sec[0] as usize][sec[1] as usize] = EntityType::Star;
        g.current_quad.stars.push(sec);
    }

    g.galaxy_chart[5][5] = 100 + g.current_quad.stars.len() as i32;
    g.chart_discovered[5][5] = true;
    g.remaining_klingons = 1;
    g
}

/// Go `EvaluateMutaraNebula`.
pub fn evaluate_mutara_nebula(g: &mut GameState) -> (bool, bool, GameOverReason) {
    if g.metrics.super_commanders_killed >= 1
        || g.metrics.commanders_killed >= 1
        || g.metrics.klingons_killed >= 1
    {
        return (true, true, GameOverReason::Won);
    }
    if g.enterprise.quad != [5, 5] {
        return (true, false, GameOverReason::Lost);
    }
    if g.enterprise.energy <= 0.0 || g.time_remaining <= 0.0 {
        return (true, false, GameOverReason::Lost);
    }
    (false, false, GameOverReason::Lost)
}

/// Go `ComputeScoreMutaraNebula`.
pub fn compute_score_mutara_nebula(g: &GameState, won: bool) -> ScoreBreakdown {
    compute_score(Some(g), won)
}
