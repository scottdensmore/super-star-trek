//! Combat mechanics (port of `pkg/engine/combat.go`).

use std::cell::RefCell;
use std::rc::Rc;

use crate::actions::{ActionFireTorpedo, EngineError};
use crate::events::*;
use crate::gomath;
use crate::refits::calculate_shield_damage_absorption;
use crate::state::{cell, cell_mut, Enterprise, GameState, Klingon, KlingonRef, QuadrantState};
use crate::types::*;

/// Go `ComputePhaserDamage`.
pub fn compute_phaser_damage(energy: f64, dist: f64) -> f64 {
    if energy <= 0.0 {
        return 0.0;
    }
    let dist = if dist <= 0.0 { 0.1 } else { dist };
    energy * (1.0 / dist)
}

/// Go `TraceTorpedoPath`: returns (hit cell, entity, hit?).
pub fn trace_torpedo_path(
    origin: Coord,
    angle: f64,
    quad: Option<&QuadrantState>,
) -> (Coord, EntityType, bool) {
    let Some(quad) = quad else {
        return ([0, 0], EntityType::Empty, false);
    };
    if origin[0] < 1 || origin[0] > 8 || origin[1] < 1 || origin[1] > 8 {
        return ([0, 0], EntityType::Empty, false);
    }
    let mut r = origin[0] as f64;
    let mut c = origin[1] as f64;
    let dr = -gomath::sin(angle) * 0.25;
    let dc = gomath::cos(angle) * 0.25;
    for _ in 0..40 {
        r += dr;
        c += dc;
        let ir = gomath::go_int(gomath::round(r));
        let ic = gomath::go_int(gomath::round(c));
        if !(1..=8).contains(&ir) || !(1..=8).contains(&ic) {
            return ([0, 0], EntityType::Empty, false);
        }
        let (ir, ic) = (ir as i32, ic as i32);
        if ir == origin[0] && ic == origin[1] {
            continue;
        }
        let cl = cell(&quad.grid, [ir, ic]);
        if cl != EntityType::Empty {
            return ([ir, ic], cl, true);
        }
    }
    ([0, 0], EntityType::Empty, false)
}

/// Go `ResolveShieldHit`: returns (shield damage absorbed, hull damage).
pub fn resolve_shield_hit(enterprise: &mut Enterprise, damage: f64) -> (f64, f64) {
    if damage <= 0.0 {
        return (0.0, 0.0);
    }
    if enterprise.shields < 0.0 {
        enterprise.shields = 0.0;
    }
    if enterprise.shields >= damage {
        enterprise.shields -= damage;
        return (damage, 0.0);
    }
    let absorbed = enterprise.shields;
    let remainder = damage - absorbed;
    enterprise.shields = 0.0;
    enterprise.energy -= remainder;
    (absorbed, remainder)
}

/// Go `ActionTorpedoDirect`: target-lock torpedo on a sector.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ActionTorpedoDirect {
    pub target_sector: Coord,
}

impl ActionTorpedoDirect {
    pub fn execute(&self, g: &mut GameState) -> Result<Vec<Event>, EngineError> {
        for k in g.current_quad.klingons.iter() {
            let k = k.as_ref().expect("nil pointer dereference").borrow();
            if k.sector == self.target_sector && k.is_cloaked {
                return Err(EngineError::new("TARGET LOCK FAILED: CLOAKED VESSEL"));
            }
        }
        ActionFireTorpedo {
            target: self.target_sector,
            ..Default::default()
        }
        .execute(g)
    }
}

/// Go `DecloakKlingon` (the Go `g` parameter is unused).
pub fn decloak_klingon(k: Option<&KlingonRef>) -> Vec<Event> {
    let Some(k) = k else { return Vec::new() };
    let mut k = k.borrow_mut();
    if !k.is_cloaked {
        return Vec::new();
    }
    k.is_cloaked = false;
    vec![Event::KlingonCloakState(EventKlingonCloakState {
        klingon_id: k.id,
        cloaked: false,
    })]
}

/// Go `CloakKlingon`.
pub fn cloak_klingon(g: Option<&GameState>, k: Option<&KlingonRef>) -> Vec<Event> {
    let Some(k) = k else { return Vec::new() };
    let mut k = k.borrow_mut();
    if k.is_cloaked {
        return Vec::new();
    }
    if let Some(g) = g {
        if !g.rules.klingon_cloak {
            return Vec::new();
        }
    }
    if !k.is_commander {
        return Vec::new();
    }
    k.is_cloaked = true;
    vec![Event::KlingonCloakState(EventKlingonCloakState {
        klingon_id: k.id,
        cloaked: true,
    })]
}

/// Go `KlingonCounterAttack`.
pub fn klingon_counter_attack(g: &mut GameState, k: &KlingonRef, damage: f64) -> Vec<Event> {
    if damage <= 0.0 {
        return Vec::new();
    }
    let mut events = Vec::new();
    if k.borrow().is_cloaked {
        events.extend(decloak_klingon(Some(k)));
    }
    let mut effective = damage;
    if g.enterprise.shields > 0.0 {
        effective = calculate_shield_damage_absorption(Some(g), damage);
    }
    let (shield_dmg, hull_damage) = resolve_shield_hit(&mut g.enterprise, effective);
    if hull_damage > 0.0 {
        let mut casualties = gomath::ceil(hull_damage / 50.0) as i32;
        if casualties < 1 {
            casualties = 1;
        }
        g.metrics.casualties += casualties;
    }
    events.push(Event::KlingonCounterAttack(EventKlingonCounterAttack {
        enemy_id: k.borrow().id,
        damage,
        shield_damage: shield_dmg,
        hull_damage,
    }));
    events
}

/// Convenience: a temporary `&Klingon{...}` handle (Go creates throwaway pointers).
pub fn temp_klingon(k: Klingon) -> KlingonRef {
    Rc::new(RefCell::new(k))
}

/// Go `MoveKlingon`.
pub fn move_klingon(g: &mut GameState, k: &KlingonRef, dest: Coord) -> Vec<Event> {
    if dest[0] < 1 || dest[0] > 8 || dest[1] < 1 || dest[1] > 8 {
        return Vec::new();
    }
    let cl = cell(&g.current_quad.grid, dest);
    if cl != EntityType::Empty && cl != EntityType::BlackHole {
        return Vec::new();
    }
    let (orig_sector, is_commander, kid) = {
        let kb = k.borrow();
        (kb.sector, kb.is_commander, kb.id)
    };
    let mut orig_entity = EntityType::Klingon;
    if orig_sector[0] >= 1 && orig_sector[0] <= 8 && orig_sector[1] >= 1 && orig_sector[1] <= 8 {
        let grid_ent = cell(&g.current_quad.grid, orig_sector);
        if grid_ent == EntityType::Klingon
            || grid_ent == EntityType::Commander
            || grid_ent == EntityType::SuperCommander
        {
            orig_entity = grid_ent;
            *cell_mut(&mut g.current_quad.grid, orig_sector) = EntityType::Empty;
        } else if is_commander {
            orig_entity = EntityType::Commander;
        }
    } else if is_commander {
        orig_entity = EntityType::Commander;
    }

    if cl == EntityType::BlackHole {
        let pos = g
            .current_quad
            .klingons
            .iter()
            .position(|kl| kl.as_ref().expect("nil pointer dereference").borrow().id == kid);
        if let Some(i) = pos {
            g.current_quad.klingons.remove(i);
        }
        if g.remaining_klingons > 0 {
            g.remaining_klingons -= 1;
        }
        g.klingons_remaining = g.remaining_klingons;
        let (qr, qc) = (g.enterprise.quad[0], g.enterprise.quad[1]);
        if (1..=8).contains(&qr)
            && (1..=8).contains(&qc)
            && g.galaxy_chart[qr as usize][qc as usize] >= 100
        {
            g.galaxy_chart[qr as usize][qc as usize] -= 100;
        }
        if orig_entity == EntityType::SuperCommander {
            g.metrics.super_commanders_killed += 1;
        } else if orig_entity == EntityType::Commander || is_commander {
            g.metrics.commanders_killed += 1;
        } else {
            g.metrics.klingons_killed += 1;
        }
        return vec![Event::SingularityAbsorption(EventSingularityAbsorption {
            sector: dest,
            target: orig_entity,
            weapon: "singularity".into(),
        })];
    }

    *cell_mut(&mut g.current_quad.grid, dest) = orig_entity;
    k.borrow_mut().sector = dest;
    Vec::new()
}

/// Go `KlingonTurn`: nebula shield collapse, black-hole absorption, return fire.
pub fn klingon_turn(g: &mut GameState) -> Vec<Event> {
    let mut events = Vec::new();
    let (qr, qc) = (g.enterprise.quad[0], g.enterprise.quad[1]);
    let in_quad = (1..=8).contains(&qr) && (1..=8).contains(&qc);
    let in_nebula = in_quad && g.quadrant_env[qr as usize][qc as usize] == EnvironmentType::Nebula;
    if in_nebula {
        for k in g.current_quad.klingons.iter().flatten() {
            k.borrow_mut().shields = 0.0;
        }
    }

    let mut survivors = crate::json::GoVec::nil();
    let list = g.current_quad.klingons.to_vec();
    for k in list.iter() {
        let Some(k) = k else { continue };
        let (sector, is_commander) = {
            let kb = k.borrow();
            (kb.sector, kb.is_commander)
        };
        if sector[0] >= 1
            && sector[0] <= 8
            && sector[1] >= 1
            && sector[1] <= 8
            && cell(&g.current_quad.grid, sector) == EntityType::BlackHole
        {
            if g.remaining_klingons > 0 {
                g.remaining_klingons -= 1;
            }
            g.klingons_remaining = g.remaining_klingons;
            if in_quad && g.galaxy_chart[qr as usize][qc as usize] >= 100 {
                g.galaxy_chart[qr as usize][qc as usize] -= 100;
            }
            if is_commander {
                g.metrics.commanders_killed += 1;
            } else {
                g.metrics.klingons_killed += 1;
            }
            let target = if is_commander {
                EntityType::Commander
            } else {
                EntityType::Klingon
            };
            events.push(Event::SingularityAbsorption(EventSingularityAbsorption {
                sector,
                target,
                weapon: "singularity".into(),
            }));
        } else {
            survivors.push(Some(k.clone()));
        }
    }
    g.current_quad.klingons = survivors;

    if g.enterprise.condition != ConditionType::Docked {
        let list = g.current_quad.klingons.to_vec();
        for k in list.iter() {
            let Some(k) = k else { continue };
            if k.borrow().energy <= 0.0 {
                continue;
            }
            let dist = crate::geometry::distance(g.enterprise.sector, k.borrow().sector);
            let mut dustfac = 0.80;
            if g.rng.is_some() {
                dustfac += 0.05 * g.rng().float64();
            } else {
                dustfac = 0.825;
            }
            let mut damage = gomath::round(k.borrow().energy * gomath::pow(dustfac, dist));
            if damage < 10.0 {
                damage = 10.0;
            }
            k.borrow_mut().energy *= 0.75;
            events.extend(klingon_counter_attack(g, k, damage));
            if g.enterprise.energy <= 0.0 {
                g.enterprise.energy = 0.0;
                events.push(Event::GameOver(EventGameOver {
                    reason: GameOverReason::Lost,
                    score: 0.0,
                }));
                break;
            }
        }
    }
    events
}
