//! Kobayashi Maru scenario (port of `pkg/engine/scenario_kobayashi.go`).

use crate::json::GoVec;
use crate::prng::PRNG;
use crate::rules::{default_rules_for_profile, DifficultyProfile};
use crate::scenarios::ScenarioID;
use crate::score::{compute_score, ScoreBreakdown};
use crate::state::{ptr, GameState, Klingon, QuadrantState};
use crate::types::*;

pub(crate) fn find_empty_sector(g: &mut GameState) -> Coord {
    loop {
        let r = g.rng().intn(8) + 1;
        let c = g.rng().intn(8) + 1;
        if g.current_quad.grid[r as usize][c as usize] == EntityType::Empty {
            return [r, c];
        }
    }
}

/// Go `BuildKobayashiMaru`.
pub fn build_kobayashi_maru(seed: i64) -> GameState {
    let rng = PRNG::new(seed);
    let rules = default_rules_for_profile(&DifficultyProfile::normal());
    let mut g =
        GameState::new_game_with_options(seed, SkillLevel::Expert, GameLength::Medium, rules);
    g.rng = Some(rng);
    g.scenario = ScenarioID::kobayashi_maru();
    g.enterprise.quad = [4, 4];
    g.enterprise.sector = [4, 4];
    g.enterprise.energy = 5000.0;
    g.enterprise.shields = 2500.0;
    g.enterprise.torpedoes = 10;
    g.enterprise.condition = ConditionType::Red;

    g.remaining_starbases = 0;
    for r in 1..=8 {
        for c in 1..=8 {
            g.galaxy_chart[r][c] = (g.galaxy_chart[r][c] / 100) * 100 + (g.galaxy_chart[r][c] % 10);
            g.chart_known_bases[r][c] = false;
        }
    }

    g.current_quad = QuadrantState::default();
    g.current_quad.grid[4][4] = EntityType::Enterprise;

    g.current_quad.klingons = GoVec::empty();
    for i in 0..3 {
        let sec = find_empty_sector(&mut g);
        let energy = 400.0 + g.rng().float64() * 100.0;
        g.current_quad.grid[sec[0] as usize][sec[1] as usize] = EntityType::Klingon;
        g.current_quad.klingons.push(ptr(Klingon {
            id: i + 1,
            sector: sec,
            energy,
            ..Default::default()
        }));
    }

    g.current_quad.stars = GoVec::empty();
    for _ in 0..3 {
        let sec = find_empty_sector(&mut g);
        g.current_quad.grid[sec[0] as usize][sec[1] as usize] = EntityType::Star;
        g.current_quad.stars.push(sec);
    }

    g.galaxy_chart[4][4] = 300 + g.current_quad.stars.len() as i32;
    g.chart_discovered[4][4] = true;
    g.remaining_klingons = 100;
    g
}

/// Go `EvaluateKobayashiMaru`: wave reinforcement; ends only on loss.
pub fn evaluate_kobayashi_maru(g: &mut GameState) -> (bool, bool, GameOverReason) {
    if g.enterprise.energy <= 0.0 || g.time_remaining <= 0.0 {
        return (true, false, GameOverReason::Lost);
    }
    if g.enterprise.quad == [4, 4] {
        let mut active = GoVec::nil();
        for k in g.current_quad.klingons.iter().flatten() {
            let kb = k.borrow();
            if kb.energy > 0.0
                && g.current_quad.grid[kb.sector[0] as usize][kb.sector[1] as usize]
                    == EntityType::Klingon
            {
                active.push(Some(k.clone()));
            }
        }
        g.current_quad.klingons = active;

        while g.current_quad.klingons.len() < 3 {
            let mut edge = Vec::new();
            for r in 1..=8 {
                for c in 1..=8 {
                    if (r == 1 || r == 8 || c == 1 || c == 8)
                        && g.current_quad.grid[r as usize][c as usize] == EntityType::Empty
                    {
                        edge.push([r, c]);
                    }
                }
            }
            let spawn: Coord = if !edge.is_empty() {
                let mut idx = 0;
                if g.rng.is_some() {
                    idx = g.rng().intn(edge.len() as i32) as usize;
                }
                edge[idx]
            } else {
                let mut any = Vec::new();
                for r in 1..=8 {
                    for c in 1..=8 {
                        if g.current_quad.grid[r as usize][c as usize] == EntityType::Empty {
                            any.push([r, c]);
                        }
                    }
                }
                if any.is_empty() {
                    break;
                }
                let mut idx = 0;
                if g.rng.is_some() {
                    idx = g.rng().intn(any.len() as i32) as usize;
                }
                any[idx]
            };
            let mut max_id = 0;
            for k in g.current_quad.klingons.iter() {
                let id = k.as_ref().expect("nil pointer dereference").borrow().id;
                if id > max_id {
                    max_id = id;
                }
            }
            let mut energy = 400.0;
            if g.rng.is_some() {
                energy += g.rng().float64() * 100.0;
            }
            g.current_quad.grid[spawn[0] as usize][spawn[1] as usize] = EntityType::Klingon;
            g.current_quad.klingons.push(ptr(Klingon {
                id: max_id + 1,
                sector: spawn,
                energy,
                ..Default::default()
            }));
            g.galaxy_chart[4][4] += 100;
        }
    }
    (false, false, GameOverReason::Lost)
}

/// Go `ComputeScoreKobayashiMaru`: commendation-based rank.
pub fn compute_score_kobayashi_maru(g: &GameState, _won: bool) -> ScoreBreakdown {
    let mut sb = compute_score(Some(g), false);
    let elapsed = g.stardate - g.initial_stardate;
    let kills =
        g.metrics.klingons_killed + g.metrics.commanders_killed + g.metrics.super_commanders_killed;
    let (title, badge) = if kills >= 10 || elapsed >= 8.0 {
        ("[COMM-4] Admiral's Citation for Gallantry", "[COMM-4]")
    } else if kills >= 6 || elapsed >= 4.0 {
        ("[COMM-3] Starfleet Cross of Honor", "[COMM-3]")
    } else if kills >= 3 || elapsed >= 2.0 {
        ("[COMM-2] Commendation for Tactical Excellence", "[COMM-2]")
    } else {
        ("[COMM-1] Cadet Commendation for Valor", "[COMM-1]")
    };
    sb.rank_title = title.into();
    sb.rank_badge = badge.into();
    sb
}
