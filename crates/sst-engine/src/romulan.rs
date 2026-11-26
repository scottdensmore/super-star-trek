//! Romulan stealth tactics and plasma torpedoes (port of `pkg/engine/romulan.go`).

use crate::combat::{
    compute_phaser_damage, klingon_counter_attack, resolve_shield_hit, temp_klingon,
};
use crate::events::*;
use crate::gomath;
use crate::json::GoVec;
use crate::state::{cell, cell_mut, ptr, EnemyRef, GameState, Klingon, PlasmaTorpedo};
use crate::types::*;

/// Go `RomulanTurn`.
pub fn romulan_turn(g: &mut GameState, r: Option<&EnemyRef>) -> Vec<Event> {
    let Some(r) = r else { return Vec::new() };
    if r.borrow().faction != FactionType::Romulan {
        return Vec::new();
    }
    let mut events = Vec::new();

    if r.borrow().is_cloaked {
        {
            let mut rb = r.borrow_mut();
            if rb.cloak_turns > 0 {
                rb.cloak_turns -= 1;
            }
        }
        if r.borrow().cloak_turns == 0 {
            let (rid, rs) = {
                let mut rb = r.borrow_mut();
                rb.is_cloaked = false;
                (rb.id, rb.sector)
            };
            *cell_mut(&mut g.current_quad.grid, rs) = EntityType::Romulan;
            events.push(Event::RomulanDecloak(EventRomulanDecloak {
                romulan_id: rid,
                sector: rs,
            }));
            let torp_id = 1000 + g.current_quad.plasma_torpedoes.len() as i32 + 1;
            let initial_yield = 1000.0;
            g.current_quad.plasma_torpedoes.push(ptr(PlasmaTorpedo {
                id: torp_id,
                source_id: rid,
                sector: rs,
                energy: initial_yield,
                target_sector: g.enterprise.sector,
                turns_in_flight: 0,
            }));
            events.push(Event::PlasmaLaunched(EventPlasmaLaunched {
                torpedo_id: torp_id,
                source_id: rid,
                sector: rs,
                energy: initial_yield,
            }));
            return events;
        }

        let rs = r.borrow().sector;
        let es = g.enterprise.sector;
        let dr = (es[0] - rs[0]).signum();
        let dc = (es[1] - rs[1]).signum();
        let old = rs;
        let new = [rs[0] + dr, rs[1] + dc];
        if (1..=8).contains(&new[0])
            && (1..=8).contains(&new[1])
            && cell(&g.current_quad.grid, new) == EntityType::Empty
        {
            if cell(&g.current_quad.grid, rs) == EntityType::Romulan {
                *cell_mut(&mut g.current_quad.grid, rs) = EntityType::Empty;
            }
            r.borrow_mut().sector = new;
        }
        return vec![Event::EntityMove(EventEntityMove {
            target: EntityType::Romulan,
            from: old,
            to: r.borrow().sector,
        })];
    }

    let (rs, rid, ren) = {
        let rb = r.borrow();
        (rb.sector, rb.id, rb.energy)
    };
    let es = g.enterprise.sector;
    let dist = gomath::hypot((rs[0] - es[0]) as f64, (rs[1] - es[1]) as f64);
    let dmg = compute_phaser_damage(ren * 0.4, dist);
    let k = temp_klingon(Klingon {
        id: rid,
        sector: rs,
        ..Default::default()
    });
    events.extend(klingon_counter_attack(g, &k, dmg));

    {
        let mut rb = r.borrow_mut();
        rb.is_cloaked = true;
        rb.cloak_turns = 3;
    }
    let rs = r.borrow().sector;
    if cell(&g.current_quad.grid, rs) == EntityType::Romulan {
        *cell_mut(&mut g.current_quad.grid, rs) = EntityType::Empty;
    }
    events
}

/// Go `AdvancePlasmaTorpedoes`.
pub fn advance_plasma_torpedoes(g: &mut GameState) -> Vec<Event> {
    let mut events = Vec::new();
    let mut survivors = GoVec::nil();
    let list = g.current_quad.plasma_torpedoes.to_vec();
    for ptp in list.iter() {
        let pt = ptp
            .as_ref()
            .expect("runtime error: invalid memory address or nil pointer dereference");
        {
            let mut p = pt.borrow_mut();
            p.turns_in_flight += 1;
            p.energy *= 0.80;
        }
        let (pe, turns, ps, pid) = {
            let p = pt.borrow();
            (p.energy, p.turns_in_flight, p.sector, p.id)
        };
        if pe < 200.0 || turns >= 5 {
            if cell(&g.current_quad.grid, ps) == EntityType::PlasmaTorpedo {
                *cell_mut(&mut g.current_quad.grid, ps) = EntityType::Empty;
            }
            events.push(Event::PlasmaDissipated(EventPlasmaDissipated {
                torpedo_id: pid,
                sector: ps,
            }));
            continue;
        }
        let es = g.enterprise.sector;
        pt.borrow_mut().target_sector = es;
        let old = ps;
        let dr = (es[0] - ps[0]).signum();
        let dc = (es[1] - ps[1]).signum();
        let new = [ps[0] + dr, ps[1] + dc];
        if new == es {
            if cell(&g.current_quad.grid, old) == EntityType::PlasmaTorpedo {
                *cell_mut(&mut g.current_quad.grid, old) = EntityType::Empty;
            }
            let half = pe * 0.5;
            let (shield_dmg, overflow) = resolve_shield_hit(&mut g.enterprise, half);
            g.enterprise.energy -= half;
            let hull = half + overflow;
            events.push(Event::PlasmaImpact(EventPlasmaImpact {
                torpedo_id: pid,
                shield_damage: shield_dmg,
                hull_damage: hull,
            }));
            continue;
        }
        let empty = |g: &GameState, c: Coord| {
            (1..=8).contains(&c[0])
                && (1..=8).contains(&c[1])
                && cell(&g.current_quad.grid, c) == EntityType::Empty
        };
        let move_to = |g: &mut GameState, c: Coord| {
            if cell(&g.current_quad.grid, old) == EntityType::PlasmaTorpedo {
                *cell_mut(&mut g.current_quad.grid, old) = EntityType::Empty;
            }
            *cell_mut(&mut g.current_quad.grid, c) = EntityType::PlasmaTorpedo;
            pt.borrow_mut().sector = c;
        };
        if empty(g, new) {
            move_to(g, new);
        } else {
            let alt1 = [ps[0] + dr, ps[1]];
            let alt2 = [ps[0], ps[1] + dc];
            if dr != 0 && empty(g, alt1) {
                move_to(g, alt1);
            } else if dc != 0 && empty(g, alt2) {
                move_to(g, alt2);
            }
        }
        events.push(Event::PlasmaMoved(EventPlasmaMoved {
            torpedo_id: pid,
            from: old,
            to: pt.borrow().sector,
            yield_: pt.borrow().energy,
        }));
        survivors.push(Some(pt.clone()));
    }
    g.current_quad.plasma_torpedoes = survivors;
    events
}

/// Go `InterceptPlasmaTorpedo`.
pub fn intercept_plasma_torpedo(
    g: &mut GameState,
    target: Coord,
    damage: f64,
) -> (bool, Vec<Event>) {
    if damage <= 0.0 {
        return (false, Vec::new());
    }
    let list = g.current_quad.plasma_torpedoes.to_vec();
    for (i, ptp) in list.iter().enumerate() {
        let pt = ptp
            .as_ref()
            .expect("runtime error: invalid memory address or nil pointer dereference");
        let (ps, pe, pid) = {
            let p = pt.borrow();
            (p.sector, p.energy, p.id)
        };
        if ps == target && damage >= pe * 0.75 {
            if cell(&g.current_quad.grid, ps) == EntityType::PlasmaTorpedo {
                *cell_mut(&mut g.current_quad.grid, ps) = EntityType::Empty;
            }
            g.current_quad.plasma_torpedoes.remove(i);
            return (
                true,
                vec![Event::PlasmaIntercepted(EventPlasmaIntercepted {
                    torpedo_id: pid,
                    sector: target,
                    weapon: "point-defense".into(),
                })],
            );
        }
    }
    (false, Vec::new())
}
