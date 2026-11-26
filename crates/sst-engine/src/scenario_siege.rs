//! Starbase Under Siege scenario (port of `pkg/engine/scenario_siege.go`).

use crate::prng::PRNG;
use crate::rules::{default_rules_for_profile, DifficultyProfile};
use crate::scenarios::ScenarioID;
use crate::score::{compute_score, ScoreBreakdown};
use crate::state::GameState;
use crate::types::*;

/// Go `BuildStarbaseSiege`.
pub fn build_starbase_siege(seed: i64) -> GameState {
    let rng = PRNG::new(seed);
    let rules = default_rules_for_profile(&DifficultyProfile::normal());
    let mut g =
        GameState::new_game_with_options(seed, SkillLevel::Expert, GameLength::Short, rules);
    g.rng = Some(rng);
    g.scenario = ScenarioID::starbase_siege();
    g.enterprise.quad = [2, 2];
    g.enterprise.sector = [4, 4];
    g.enterprise.energy = 5000.0;
    g.enterprise.shields = 1500.0;
    g.enterprise.torpedoes = 10;
    g.time_remaining = 6.0;
    g.remaining_starbases = 1;
    g.remaining_klingons = 3;

    for r in 1..=8 {
        for c in 1..=8 {
            g.galaxy_chart[r][c] %= 10;
            g.chart_known_bases[r][c] = false;
        }
    }

    let mut stars = g.galaxy_chart[4][4] % 10;
    if stars == 0 {
        stars = 3;
    }
    g.galaxy_chart[4][4] = 310 + stars;
    g.chart_known_bases[4][4] = true;

    g.populate_quadrant([2, 2], [4, 4]);
    g.enterprise.condition = ConditionType::Green;
    g
}

/// Go `EvaluateStarbaseSiege`.
pub fn evaluate_starbase_siege(g: &mut GameState) -> (bool, bool, GameOverReason) {
    if g.remaining_starbases == 0
        || g.metrics.starbases_destroyed > 0
        || (g.galaxy_chart[4][4] % 100) / 10 == 0
    {
        return (true, false, GameOverReason::Lost);
    }
    if g.time_remaining <= 0.0 || g.enterprise.energy <= 0.0 {
        return (true, false, GameOverReason::Lost);
    }
    let total =
        g.metrics.klingons_killed + g.metrics.commanders_killed + g.metrics.super_commanders_killed;
    if total >= 3
        || (g.remaining_klingons == 0
            && (g.enterprise.quad != [4, 4] || g.current_quad.klingons.is_empty()))
    {
        return (true, true, GameOverReason::Won);
    }
    (false, false, GameOverReason::Lost)
}

/// Go `ComputeScoreStarbaseSiege`.
pub fn compute_score_starbase_siege(g: &GameState, won: bool) -> ScoreBreakdown {
    compute_score(Some(g), won)
}
