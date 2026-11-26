//! Klingon wolf-pack tactics (port of `pkg/engine/klingon_pack.go`).

use std::collections::BTreeSet;

use crate::events::*;
use crate::gomath;
use crate::state::{cell, cell_mut, GameState};
use crate::types::*;

/// Go `DetectCrossfireBracket`: two active Klingons at >= 60 degrees around the Enterprise.
pub fn detect_crossfire_bracket(g: Option<&GameState>) -> bool {
    let Some(g) = g else { return false };
    if g.current_quad.enemies.len() < 2 {
        return false;
    }
    let active: Vec<Coord> = g
        .current_quad
        .enemies
        .iter()
        .flatten()
        .filter_map(|e| {
            let e = e.borrow();
            (e.faction == FactionType::Klingon && e.energy > 0.0).then_some(e.sector)
        })
        .collect();
    if active.len() < 2 {
        return false;
    }
    let ent = g.enterprise.sector;
    for i in 0..active.len() {
        for j in i + 1..active.len() {
            let v1x = (active[i][0] - ent[0]) as f64;
            let v1y = (active[i][1] - ent[1]) as f64;
            let v2x = (active[j][0] - ent[0]) as f64;
            let v2y = (active[j][1] - ent[1]) as f64;
            let mag1 = gomath::hypot(v1x, v1y);
            let mag2 = gomath::hypot(v2x, v2y);
            if mag1 == 0.0 || mag2 == 0.0 {
                continue;
            }
            let dot = v1x * v2x + v1y * v2y;
            let cos_theta = (dot / (mag1 * mag2)).clamp(-1.0, 1.0);
            let angle = gomath::acos(cos_theta) * gomath::DEG_PER_RAD;
            if angle >= 60.0 {
                return true;
            }
        }
    }
    false
}

/// Go `CalculatePackDamageMultiplier`: 1.35 under crossfire, else 1.0.
pub fn calculate_pack_damage_multiplier(g: Option<&GameState>) -> f64 {
    if detect_crossfire_bracket(g) {
        1.35
    } else {
        1.0
    }
}

/// Go `ExecuteCommanderScreening`: escorts interpose between commander and Enterprise.
pub fn execute_commander_screening(g: &mut GameState) -> Vec<Event> {
    let mut events = Vec::new();
    let ent = g.enterprise.sector;
    let mut screened: BTreeSet<i32> = BTreeSet::new();
    let list = g.current_quad.enemies.to_vec();
    for cmd in list.iter() {
        let Some(cmd) = cmd else { continue };
        let (is_cmd, faction, energy, cs, cid) = {
            let c = cmd.borrow();
            (c.is_commander, c.faction, c.energy, c.sector, c.id)
        };
        if !is_cmd || faction != FactionType::Klingon || energy <= 0.0 {
            continue;
        }
        let target: Coord = if cs[0] == ent[0] {
            let (mut s, mut e) = (ent[1], cs[1]);
            if s > e {
                std::mem::swap(&mut s, &mut e);
            }
            [cs[0], gomath::round((s + e) as f64 / 2.0) as i32]
        } else if cs[1] == ent[1] {
            let (mut s, mut e) = (ent[0], cs[0]);
            if s > e {
                std::mem::swap(&mut s, &mut e);
            }
            [gomath::round((s + e) as f64 / 2.0) as i32, cs[1]]
        } else {
            continue;
        };

        for raider in g.current_quad.enemies.to_vec().iter() {
            let Some(raider) = raider else { continue };
            let (r_cmd, r_fac, r_en, r_id, rs) = {
                let r = raider.borrow();
                (r.is_commander, r.faction, r.energy, r.id, r.sector)
            };
            if r_cmd || r_fac != FactionType::Klingon || r_en <= 0.0 || screened.contains(&r_id) {
                continue;
            }
            if ((rs[0] - target[0]) as f64).abs() <= 1.0
                && ((rs[1] - target[1]) as f64).abs() <= 1.0
                && cell(&g.current_quad.grid, target) == EntityType::Empty
            {
                *cell_mut(&mut g.current_quad.grid, rs) = EntityType::Empty;
                *cell_mut(&mut g.current_quad.grid, target) = EntityType::Klingon;
                raider.borrow_mut().sector = target;
                screened.insert(r_id);
                events.push(Event::KlingonScreening(EventKlingonScreening {
                    raider_id: r_id,
                    commander_id: cid,
                    interposed: target,
                }));
                break;
            }
        }
    }
    events
}
