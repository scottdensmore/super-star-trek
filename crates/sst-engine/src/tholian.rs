//! Tholian web spinners (port of `pkg/engine/tholian.go`).

use crate::events::*;
use crate::json::GoVec;
use crate::state::{cell, cell_mut, ptr, GameState, TholianWebSegment};
use crate::types::*;

/// Go `TholianTurn`.
pub fn tholian_turn(g: &mut GameState) -> Vec<Event> {
    let mut events = Vec::new();
    let tholians: Vec<_> = g
        .current_quad
        .enemies
        .iter()
        .flatten()
        .filter(|e| {
            let e = e.borrow();
            e.faction == FactionType::Tholian && e.energy > 0.0
        })
        .cloned()
        .collect();

    if tholians.is_empty() {
        if !g.current_quad.web_segments.is_empty() {
            let count = g.current_quad.web_segments.len() as i32;
            let segs = g.current_quad.web_segments.to_vec();
            for ws in segs.iter().flatten() {
                let c = ws.borrow().coord;
                if cell(&g.current_quad.grid, c) == EntityType::TholianWeb {
                    *cell_mut(&mut g.current_quad.grid, c) = EntityType::Empty;
                }
            }
            g.current_quad.web_segments = GoVec::nil();
            return vec![Event::WebCollapsed(EventWebCollapsed {
                segments_count: count,
            })];
        }
        return Vec::new();
    }

    for t in tholians.iter() {
        let (old, phase, tid) = {
            let tb = t.borrow();
            (tb.sector, tb.special_state, tb.id)
        };
        let (dr, dc) = get_perimeter_step(old, phase);
        let new = [(old[0] + dr).clamp(1, 8), (old[1] + dc).clamp(1, 8)];
        if cell(&g.current_quad.grid, new) == EntityType::Empty {
            *cell_mut(&mut g.current_quad.grid, old) = EntityType::TholianWeb;
            g.current_quad.web_segments.push(ptr(TholianWebSegment {
                coord: old,
                strength: 250.0,
            }));
            *cell_mut(&mut g.current_quad.grid, new) = EntityType::Tholian;
            t.borrow_mut().sector = new;
            events.push(Event::WebSegmentLaid(EventWebSegmentLaid {
                spinner_id: tid,
                coord: old,
            }));
        }
    }
    events
}

/// Go `getPerimeterStep`: clockwise (phase 0) or counter-clockwise patrol step.
pub fn get_perimeter_step(current: Coord, phase: i32) -> (i32, i32) {
    if phase == 0 {
        if current[0] == 1 && current[1] < 8 {
            return (0, 1);
        }
        if current[1] == 8 && current[0] < 8 {
            return (1, 0);
        }
        if current[0] == 8 && current[1] > 1 {
            return (0, -1);
        }
        return (-1, 0);
    }
    if current[0] == 1 && current[1] > 1 {
        return (0, -1);
    }
    if current[1] == 1 && current[0] < 8 {
        return (1, 0);
    }
    if current[0] == 8 && current[1] < 8 {
        return (0, 1);
    }
    (-1, 0)
}

/// Go `CalculateWebContainment` (percentage, 20 segments = 100%).
pub fn calculate_web_containment(g: Option<&GameState>) -> f64 {
    let Some(g) = g else { return 0.0 };
    let count = g.current_quad.web_segments.len();
    let c = (count as f64 / 20.0) * 100.0;
    if c > 100.0 {
        100.0
    } else {
        c
    }
}

/// Go `DamageWebSegment`.
pub fn damage_web_segment(g: &mut GameState, coord: Coord, damage: f64) -> (bool, Vec<Event>) {
    if damage <= 0.0 {
        return (false, Vec::new());
    }
    let segs = g.current_quad.web_segments.to_vec();
    for (i, ws) in segs.iter().enumerate() {
        let Some(ws) = ws else { continue };
        if ws.borrow().coord == coord {
            let strength = {
                let mut w = ws.borrow_mut();
                w.strength -= damage;
                w.strength
            };
            if strength <= 0.0 {
                *cell_mut(&mut g.current_quad.grid, coord) = EntityType::Empty;
                g.current_quad.web_segments.remove(i);
                return (true, vec![Event::WebBreached(EventWebBreached { coord })]);
            }
            return (false, Vec::new());
        }
    }
    (false, Vec::new())
}
