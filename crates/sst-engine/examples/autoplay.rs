//! Headless autoplay balance check: plays seeded games with a simple
//! scripted captain and prints win rates per difficulty profile.
//!
//! `cargo run --release -p sst-engine --example autoplay [games-per-profile]`

use sst_engine::*;

const HOSTILE: [EntityType; 5] = [
    EntityType::Klingon,
    EntityType::Commander,
    EntityType::SuperCommander,
    EntityType::Romulan,
    EntityType::Tholian,
];

fn hostiles(g: &GameState) -> Vec<Coord> {
    let mut v = Vec::new();
    for r in 1..=8i32 {
        for c in 1..=8i32 {
            if HOSTILE.contains(&g.current_quad.grid[r as usize][c as usize]) {
                v.push([r, c]);
            }
        }
    }
    let e = g.enterprise.sector;
    v.sort_by(|a, b| distance(e, *a).partial_cmp(&distance(e, *b)).unwrap());
    v
}

fn nearest_quad(g: &GameState, pred: impl Fn(i32) -> bool) -> Option<Coord> {
    let q = g.enterprise.quad;
    let mut best: Option<(f64, Coord)> = None;
    for r in 1..=8 {
        for c in 1..=8 {
            if [r, c] == q || !pred(g.galaxy_chart[r as usize][c as usize]) {
                continue;
            }
            let d = distance(q, [r, c]);
            if best.is_none_or(|(bd, _)| d < bd) {
                best = Some((d, [r, c]));
            }
        }
    }
    best.map(|b| b.1)
}

#[derive(Default)]
struct Outcome {
    won: bool,
    destroyed: bool,
    timeout: bool,
    kills: i32,
    days: f64,
}

fn play(seed: i64, profile: &DifficultyProfile, phasers_first: bool) -> Outcome {
    let rules = default_rules_for_profile(profile);
    let mut g = GameState::new_game_with_options(seed, SkillLevel::Good, GameLength::Medium, rules);
    let (q, s) = (g.enterprise.quad, g.enterprise.sector);
    g.populate_quadrant(q, s);
    let start_k = g.remaining_klingons;
    let mut out = Outcome::default();
    let mut stuck = 0;
    for _ in 0..600 {
        if g.game_over || g.time_remaining <= 0.0 {
            break;
        }
        let hs = hostiles(&g);
        let e = g.enterprise.clone();
        let mut tries: Vec<Action> = Vec::new();
        if e.condition == ConditionType::Docked && !hs.is_empty() {
            // Undock by moving one sector.
        }
        if !hs.is_empty() && e.condition != ConditionType::Docked {
            if e.shields < 500.0 && e.energy > 1500.0 {
                tries.push(
                    ActionShields {
                        amount: (1000.0 - e.shields).min(e.energy - 1000.0),
                    }
                    .into(),
                );
            }
            let pha = ActionFirePhasers {
                energy: (e.energy * 0.3).clamp(200.0, 1200.0),
                ..Default::default()
            };
            let tor = ActionFireTorpedo {
                target: hs[0],
                ..Default::default()
            };
            if phasers_first && e.energy > 800.0 {
                tries.push(pha.into());
                tries.push(tor.into());
            } else {
                tries.push(tor.into());
                if e.energy > 800.0 {
                    tries.push(pha.into());
                }
            }
        }
        if let Some(sb) = g.current_quad.starbase {
            if e.condition != ConditionType::Docked && (e.energy < 2500.0 || e.torpedoes < 3) {
                tries.push(ActionDock.into());
                for (dr, dc) in [
                    (0, 1),
                    (0, -1),
                    (1, 0),
                    (-1, 0),
                    (1, 1),
                    (-1, -1),
                    (1, -1),
                    (-1, 1),
                ] {
                    let t = [sb[0] + dr, sb[1] + dc];
                    if (1..=8).contains(&t[0])
                        && (1..=8).contains(&t[1])
                        && g.current_quad.grid[t[0] as usize][t[1] as usize] == EntityType::Empty
                    {
                        tries.push(
                            ActionMove {
                                dest_sector: t,
                                warp: 0.05,
                                ..Default::default()
                            }
                            .into(),
                        );
                        break;
                    }
                }
            }
        }
        let low = e.energy < 2000.0 || e.torpedoes < 2;
        let target = if low {
            nearest_quad(&g, |v| (v % 100) / 10 > 0).or_else(|| nearest_quad(&g, |v| v >= 100))
        } else {
            nearest_quad(&g, |v| v >= 100).or_else(|| nearest_quad(&g, |v| (v % 100) / 10 > 0))
        };
        if let Some(t) = target {
            if hs.is_empty() && e.shields > 0.0 {
                // Shields double the warp cost: drop them for the jump.
                tries.push(ActionShields { amount: -e.shields }.into());
            }
            let warp = if e.energy > 3000.0 { 5.0 } else { 4.0 };
            tries.push(
                ActionMove {
                    dest_quad: t,
                    warp,
                    ..Default::default()
                }
                .into(),
            );
            tries.push(
                ActionMove {
                    dest_quad: t,
                    warp: 2.0,
                    ..Default::default()
                }
                .into(),
            );
        }
        let mut acted = false;
        for a in tries {
            if let Ok(mut events) = g.dispatch(&a) {
                execute_klingon_turn_if_active(&mut g, &a, &mut events);
                for ev in &events {
                    if let Event::GameOver(go) = ev {
                        g.game_over = true;
                        if go.reason == GameOverReason::Won {
                            g.game_won = true;
                        }
                    }
                }
                acted = true;
                break;
            }
        }
        if !acted {
            stuck += 1;
            if stuck > 5 {
                break;
            }
        } else {
            stuck = 0;
        }
    }
    out.won = g.game_won;
    out.destroyed = g.game_over && !g.game_won;
    out.timeout = !g.game_over && g.time_remaining <= 0.0;
    out.kills = start_k - g.remaining_klingons;
    out.days = g.stardate - g.initial_stardate;
    out
}

fn main() {
    let n: i64 = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or(200);
    for phasers_first in [true, false] {
        println!(
            "strategy: {}",
            if phasers_first {
                "phasers first"
            } else {
                "torpedoes first"
            }
        );
        for p in [
            "casual",
            "normal",
            "hardcore",
            "nightmare",
            "expert",
            "emeritus",
        ] {
            let prof = DifficultyProfile::new(p.to_string());
            let (mut wins, mut kills, mut days, mut dead, mut late) = (0, 0, 0.0, 0, 0);
            for seed in 1..=n {
                let o = play(seed * 7919, &prof, phasers_first);
                wins += o.won as i32;
                dead += o.destroyed as i32;
                late += o.timeout as i32;
                kills += o.kills;
                days += o.days;
            }
            println!(
                "  {p:<10} adversaries={:<5} win {:>5.1}%  destroyed {:>5.1}%  out of time {:>5.1}%  avg kills {:>5.2}  avg days {:>5.1}",
                default_rules_for_profile(&prof).adversaries,
                100.0 * wins as f64 / n as f64,
                100.0 * dead as f64 / n as f64,
                100.0 * late as f64 / n as f64,
                kills as f64 / n as f64,
                days / n as f64
            );
        }
    }
}
