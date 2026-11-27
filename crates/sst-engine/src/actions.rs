//! Player actions (port of `pkg/engine/actions.go`). Go's `Action`
//! interface becomes the [`Action`] enum; each Go action struct keeps its
//! name and has an `execute` method.

use std::collections::BTreeMap;
use std::fmt;

use crate::combat::*;
use crate::events::*;
use crate::geometry::{bearing, distance};
use crate::gomath;
use crate::json::GoVec;
use crate::refits::{calculate_torpedo_damage, RefitID};
use crate::scenarios::{get_scenario, ScenarioID};
use crate::score::compute_score;
use crate::state::{
    cell, cell_mut, remove_enemy_klingon, sync_quadrant_enemies, EnemyRef, GameState, KlingonRef,
};
use crate::tactics::ActionPhaserDirect;
use crate::types::*;
use crate::{romulan, tholian};

/// A Go `error` produced by the engine (`errors.New` / `fmt.Errorf` text).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineError(pub String);

impl EngineError {
    pub fn new(s: impl Into<String>) -> Self {
        EngineError(s.into())
    }
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for EngineError {}

/// Result of executing an action: events or a Go error.
pub type ActionResult = Result<Vec<Event>, EngineError>;

/// Go `Action` interface.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Shields(ActionShields),
    Dock(ActionDock),
    FireTorpedo(ActionFireTorpedo),
    FirePhasers(ActionFirePhasers),
    Move(ActionMove),
    LRScan(ActionLRScan),
    KlingonCounterAttack(ActionKlingonCounterAttack),
    CallHelp(ActionCallHelp),
    TorpedoDirect(ActionTorpedoDirect),
    PhaserDirect(ActionPhaserDirect),
}

impl Action {
    /// Go `Action.Execute`.
    pub fn execute(&self, g: &mut GameState) -> ActionResult {
        match self {
            Action::Shields(a) => a.execute(g),
            Action::Dock(a) => a.execute(g),
            Action::FireTorpedo(a) => a.execute(g),
            Action::FirePhasers(a) => a.execute(g),
            Action::Move(a) => a.execute(g),
            Action::LRScan(a) => a.execute(g),
            Action::KlingonCounterAttack(a) => a.execute(g),
            Action::CallHelp(a) => a.execute(g),
            Action::TorpedoDirect(a) => a.execute(g),
            Action::PhaserDirect(a) => a.execute(g),
        }
    }

    /// Go type name (e.g. `engine.ActionMove`), as printed by `%T`.
    pub fn type_name(&self) -> &'static str {
        match self {
            Action::Shields(_) => "engine.ActionShields",
            Action::Dock(_) => "engine.ActionDock",
            Action::FireTorpedo(_) => "engine.ActionFireTorpedo",
            Action::FirePhasers(_) => "engine.ActionFirePhasers",
            Action::Move(_) => "engine.ActionMove",
            Action::LRScan(_) => "engine.ActionLRScan",
            Action::KlingonCounterAttack(_) => "engine.ActionKlingonCounterAttack",
            Action::CallHelp(_) => "engine.ActionCallHelp",
            Action::TorpedoDirect(_) => "engine.ActionTorpedoDirect",
            Action::PhaserDirect(_) => "engine.ActionPhaserDirect",
        }
    }
}

macro_rules! from_action {
    ($t:ident, $v:ident) => {
        impl From<$t> for Action {
            fn from(a: $t) -> Action {
                Action::$v(a)
            }
        }
    };
}
from_action!(ActionShields, Shields);
from_action!(ActionDock, Dock);
from_action!(ActionFireTorpedo, FireTorpedo);
from_action!(ActionFirePhasers, FirePhasers);
from_action!(ActionMove, Move);
from_action!(ActionLRScan, LRScan);
from_action!(ActionKlingonCounterAttack, KlingonCounterAttack);
from_action!(ActionCallHelp, CallHelp);
from_action!(ActionTorpedoDirect, TorpedoDirect);
from_action!(ActionPhaserDirect, PhaserDirect);

fn kl(p: &Option<KlingonRef>) -> &KlingonRef {
    p.as_ref()
        .expect("runtime error: invalid memory address or nil pointer dereference")
}

fn in_nebula(g: &GameState) -> bool {
    let q = g.enterprise.quad;
    g.quadrant_env[q[0] as usize][q[1] as usize] == EnvironmentType::Nebula
}

pub(crate) fn dec_galaxy_klingons(g: &mut GameState) {
    let (qr, qc) = (g.enterprise.quad[0], g.enterprise.quad[1]);
    if (1..=8).contains(&qr)
        && (1..=8).contains(&qc)
        && g.galaxy_chart[qr as usize][qc as usize] >= 100
    {
        g.galaxy_chart[qr as usize][qc as usize] -= 100;
    }
}

fn cloak_all_commanders(g: &GameState, events: &mut Vec<Event>, except: Option<&KlingonRef>) {
    for k in g.current_quad.klingons.iter().flatten() {
        if let Some(e) = except {
            if std::rc::Rc::ptr_eq(e, k) {
                continue;
            }
        }
        let (cmd, cloaked) = {
            let kb = k.borrow();
            (kb.is_commander, kb.is_cloaked)
        };
        if cmd && !cloaked {
            events.extend(cloak_klingon(Some(g), Some(k)));
        }
    }
}

/// Go `ActionShields` (alias `ActionTransferShields`): positive moves energy to shields.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ActionShields {
    pub amount: f64,
}

/// Go `ActionTransferShields = ActionShields`.
pub type ActionTransferShields = ActionShields;

impl ActionShields {
    pub fn execute(&self, g: &mut GameState) -> ActionResult {
        if self.amount > 0.0 && in_nebula(g) {
            return Err(EngineError::new("sensors indicate extreme particle ionization: shields cannot hold cohesive geometry in nebula"));
        }
        if self.amount > 0.0 && g.enterprise.energy < self.amount {
            return Err(EngineError::new("insufficient energy for shield transfer"));
        }
        if self.amount < 0.0 && g.enterprise.shields < -self.amount {
            return Err(EngineError::new(
                "insufficient shield energy to transfer to engines",
            ));
        }
        g.enterprise.energy -= self.amount;
        g.enterprise.shields += self.amount;
        Ok(vec![Event::ShieldTransfer(EventShieldTransfer {
            new_shields: g.enterprise.shields,
            new_energy: g.enterprise.energy,
        })])
    }
}

/// Go `ActionDock`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ActionDock;

impl ActionDock {
    pub fn execute(&self, g: &mut GameState) -> ActionResult {
        if g.enterprise.condition == ConditionType::Docked {
            return Err(EngineError::new("already docked"));
        }
        let Some(sb) = g.current_quad.starbase else {
            return Err(EngineError::new("no starbase in current quadrant"));
        };
        let ent = g.enterprise.sector;
        let dr = ent[0] - sb[0];
        let dc = ent[1] - sb[1];
        if !(-1..=1).contains(&dr) || !(-1..=1).contains(&dc) || (dr == 0 && dc == 0) {
            return Err(EngineError::new("enterprise not adjacent to starbase"));
        }
        g.enterprise.condition = ConditionType::Docked;
        let mut max_energy = g.enterprise.max_energy;
        if max_energy <= 0.0 {
            max_energy = 5000.0;
        }
        g.enterprise.energy = max_energy;
        let mut max_torp = g.enterprise.max_torpedoes;
        if max_torp <= 0 {
            max_torp = 10;
        }
        g.enterprise.torpedoes = max_torp;
        for d in g.enterprise.devices.iter_mut() {
            *d = 0.0;
        }

        let mut surv_mode = g.rules.surveillance.clone();
        if surv_mode.as_str().is_empty() {
            surv_mode = crate::rules::SurveillanceMode::classic();
        }
        let mut updated = 0;
        match surv_mode.as_str() {
            "full" => {
                for r in 1..=8 {
                    for c in 1..=8 {
                        if (g.galaxy_chart[r][c] % 100) / 10 > 0 {
                            g.chart_known_bases[r][c] = true;
                        }
                        if !g.chart_discovered[r][c] {
                            g.chart_discovered[r][c] = true;
                            updated += 1;
                        }
                    }
                }
            }
            "local" => {
                let eq = g.enterprise.quad;
                g.chart_known_bases[eq[0] as usize][eq[1] as usize] = true;
                for dr in -1..=1 {
                    for dc in -1..=1 {
                        let (nr, nc) = (eq[0] + dr, eq[1] + dc);
                        if (1..=8).contains(&nr)
                            && (1..=8).contains(&nc)
                            && !g.chart_discovered[nr as usize][nc as usize]
                        {
                            g.chart_discovered[nr as usize][nc as usize] = true;
                            updated += 1;
                        }
                    }
                }
            }
            "blackout" => {
                let eq = g.enterprise.quad;
                g.chart_known_bases[eq[0] as usize][eq[1] as usize] = true;
            }
            _ => {
                for r in 1..=8i32 {
                    for c in 1..=8i32 {
                        if (g.galaxy_chart[r as usize][c as usize] % 100) / 10 > 0 {
                            g.chart_known_bases[r as usize][c as usize] = true;
                            for dr in -1..=1 {
                                for dc in -1..=1 {
                                    let (nr, nc) = (r + dr, c + dc);
                                    if (1..=8).contains(&nr)
                                        && (1..=8).contains(&nc)
                                        && !g.chart_discovered[nr as usize][nc as usize]
                                    {
                                        g.chart_discovered[nr as usize][nc as usize] = true;
                                        updated += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(vec![
            Event::Docked(EventDocked { starbase: sb }),
            Event::StarbaseSurveillance(EventStarbaseSurveillance {
                starbase_coord: sb,
                updated_quads: updated,
                mode: surv_mode,
            }),
        ])
    }
}

/// Go `ActionFireTorpedo`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ActionFireTorpedo {
    pub target: Coord,
    pub angle: f64,
    pub direction: f64,
}

impl ActionFireTorpedo {
    pub fn execute(&self, g: &mut GameState) -> ActionResult {
        if g.enterprise.torpedoes <= 0 {
            return Err(EngineError::new("no photon torpedoes left"));
        }
        if g.enterprise.condition != ConditionType::Docked {
            g.enterprise.torpedoes -= 1;
        }
        let mut angle = self.angle;
        if self.target != [0, 0] {
            angle = bearing(g.enterprise.sector, self.target);
        } else if self.direction != 0.0 {
            angle = (self.direction - 1.0) * gomath::PI / 4.0;
        }
        let mut events = vec![Event::TorpedoFired(EventTorpedoFired {
            origin: g.enterprise.sector,
            angle,
        })];

        let (hit_coord, hit_entity, hit) = trace_torpedo_with_cloaked(g, angle);
        if !hit {
            if g.rules.klingon_cloak {
                cloak_all_commanders(g, &mut events, None);
            }
            return Ok(events);
        }

        let mut damage = calculate_torpedo_damage(Some(g), 500.0);
        // What the target's shields absorbed; `damage` ends up as what got
        // through (Go reported the full torpedo yield on every hit that did
        // not destroy the ship, whatever the shields took).
        let mut shield_damage = 0.0;
        let mut destroyed = false;
        let mut hit_commander: Option<KlingonRef> = None;

        match hit_entity {
            EntityType::BlackHole => {
                events.push(Event::SingularityAbsorption(EventSingularityAbsorption {
                    sector: hit_coord,
                    target: EntityType::BlackHole,
                    weapon: "torpedo".into(),
                }));
                if g.rules.klingon_cloak {
                    cloak_all_commanders(g, &mut events, None);
                }
                return Ok(events);
            }
            EntityType::Klingon | EntityType::Commander | EntityType::SuperCommander => {
                let mut target: Option<KlingonRef> = None;
                let mut target_index = 0usize;
                for (i, k) in g.current_quad.klingons.iter().enumerate() {
                    if kl(k).borrow().sector == hit_coord {
                        target = Some(kl(k).clone());
                        target_index = i;
                        break;
                    }
                }
                if let Some(tk) = &target {
                    hit_commander = Some(tk.clone());
                    if tk.borrow().is_cloaked {
                        events.extend(decloak_klingon(Some(tk)));
                    }
                    if in_nebula(g) {
                        tk.borrow_mut().shields = 0.0;
                    }
                    let mut t = tk.borrow_mut();
                    let mut effective = damage;
                    if t.shields > 0.0 {
                        if t.shields >= damage {
                            shield_damage = damage;
                            t.shields -= damage;
                            effective = 0.0;
                        } else {
                            shield_damage = t.shields;
                            effective = damage - t.shields;
                            t.shields = 0.0;
                        }
                    }
                    if effective >= t.energy {
                        destroyed = true;
                        damage = t.energy;
                        t.energy = 0.0;
                        let id = t.id;
                        drop(t);
                        g.current_quad.klingons.remove(target_index);
                        remove_enemy_klingon(&mut g.current_quad, id);
                    } else {
                        t.energy -= effective;
                        damage = effective;
                    }
                } else {
                    destroyed = true;
                }
                if destroyed {
                    *cell_mut(&mut g.current_quad.grid, hit_coord) = EntityType::Empty;
                    if g.remaining_klingons > 0 {
                        g.remaining_klingons -= 1;
                    }
                    g.klingons_remaining = g.remaining_klingons;
                    dec_galaxy_klingons(g);
                    let tk_cmd = target.as_ref().is_some_and(|t| t.borrow().is_commander);
                    if hit_entity == EntityType::SuperCommander {
                        g.metrics.super_commanders_killed += 1;
                    } else if hit_entity == EntityType::Commander || tk_cmd {
                        g.metrics.commanders_killed += 1;
                    } else {
                        g.metrics.klingons_killed += 1;
                    }
                }
            }
            EntityType::Star => {
                damage = 0.0;
                destroyed = false;
            }
            EntityType::Starbase => {
                damage = 500.0;
                destroyed = true;
                g.current_quad.starbase = None;
                *cell_mut(&mut g.current_quad.grid, hit_coord) = EntityType::Empty;
                if g.remaining_starbases > 0 {
                    g.remaining_starbases -= 1;
                }
                let (qr, qc) = (g.enterprise.quad[0], g.enterprise.quad[1]);
                if (1..=8).contains(&qr)
                    && (1..=8).contains(&qc)
                    && (g.galaxy_chart[qr as usize][qc as usize] % 100) / 10 > 0
                {
                    g.galaxy_chart[qr as usize][qc as usize] -= 10;
                }
                g.metrics.starbases_destroyed += 1;
            }
            EntityType::Romulan | EntityType::Tholian => {
                let mut target = None;
                let mut target_index = 0usize;
                for (i, e) in g.current_quad.enemies.iter().enumerate() {
                    if let Some(e) = e {
                        if e.borrow().sector == hit_coord {
                            target = Some(e.clone());
                            target_index = i;
                            break;
                        }
                    }
                }
                if let Some(te) = &target {
                    if te.borrow().is_cloaked {
                        let id = {
                            let mut tb = te.borrow_mut();
                            tb.is_cloaked = false;
                            tb.id
                        };
                        // The hit reveals the ship (it stays visible until it
                        // cloaks again on its turn).
                        *cell_mut(&mut g.current_quad.grid, hit_coord) = EntityType::Romulan;
                        events.push(Event::RomulanDecloak(EventRomulanDecloak {
                            romulan_id: id,
                            sector: hit_coord,
                        }));
                    }
                    if in_nebula(g) {
                        te.borrow_mut().shields = 0.0;
                    }
                    let mut t = te.borrow_mut();
                    let mut effective = damage;
                    if t.shields > 0.0 {
                        if t.shields >= damage {
                            shield_damage = damage;
                            t.shields -= damage;
                            effective = 0.0;
                        } else {
                            shield_damage = t.shields;
                            effective = damage - t.shields;
                            t.shields = 0.0;
                        }
                    }
                    if effective >= t.energy {
                        destroyed = true;
                        damage = t.energy;
                        t.energy = 0.0;
                        drop(t);
                        g.current_quad.enemies.remove(target_index);
                    } else {
                        t.energy -= effective;
                        damage = effective;
                    }
                } else {
                    destroyed = true;
                }
                if destroyed {
                    *cell_mut(&mut g.current_quad.grid, hit_coord) = EntityType::Empty;
                    if target
                        .as_ref()
                        .is_some_and(|t| t.borrow().faction == FactionType::Romulan)
                    {
                        g.metrics.romulans_killed += 1;
                    }
                    sync_quadrant_enemies(&mut g.current_quad);
                    if g.remaining_klingons > 0 {
                        g.remaining_klingons -= 1;
                    }
                    g.klingons_remaining = g.remaining_klingons;
                    dec_galaxy_klingons(g);
                }
            }
            EntityType::PlasmaTorpedo => {
                let (intercepted, ev) =
                    romulan::intercept_plasma_torpedo(g, hit_coord, gomath::max(damage, 1000.0));
                if intercepted {
                    destroyed = true;
                    events.extend(ev);
                    g.metrics.romulans_surrendered += 1;
                } else {
                    destroyed = false;
                }
            }
            EntityType::TholianWeb => {
                let (breached, ev) = tholian::damage_web_segment(g, hit_coord, damage);
                if breached {
                    destroyed = true;
                    events.extend(ev);
                } else {
                    destroyed = false;
                }
            }
            _ => {
                damage = 0.0;
                destroyed = false;
            }
        }

        events.push(Event::TorpedoHit(EventTorpedoHit {
            target: hit_coord,
            entity: hit_entity,
            damage,
            destroyed,
            shield_damage,
        }));

        if g.rules.klingon_cloak {
            cloak_all_commanders(g, &mut events, hit_commander.as_ref());
        }

        Ok(evaluate_scenario(g, events))
    }
}

/// Traces a torpedo from the Enterprise, treating cloaked Romulans as solid.
///
/// A cloaked Romulan leaves its sector empty on the grid, so Go's trace flew
/// straight through it and the decloak-on-hit handling below was dead code: a
/// cloaked Romulan could only be hit during the single turn it was visible.
/// Like a cloaked Klingon commander (which keeps its grid cell), a cloaked
/// Romulan cannot be target-locked but blind fire along its line hits it.
fn trace_torpedo_with_cloaked(g: &GameState, angle: f64) -> (Coord, EntityType, bool) {
    let cloaked: Vec<Coord> = g
        .current_quad
        .enemies
        .iter()
        .flatten()
        .filter_map(|e| {
            let e = e.borrow();
            let s = e.sector;
            (e.faction == FactionType::Romulan
                && e.is_cloaked
                && e.energy > 0.0
                && (1..=8).contains(&s[0])
                && (1..=8).contains(&s[1])
                && cell(&g.current_quad.grid, s) == EntityType::Empty)
                .then_some(s)
        })
        .collect();
    if cloaked.is_empty() {
        return trace_torpedo_path(g.enterprise.sector, angle, Some(&g.current_quad));
    }
    let mut view = g.current_quad.clone();
    for s in cloaked {
        *cell_mut(&mut view.grid, s) = EntityType::Romulan;
    }
    trace_torpedo_path(g.enterprise.sector, angle, Some(&view))
}

/// Go `ActionFirePhasers`. `manual_allocation` (Go `map[int]float64`) is
/// iterated in key order; Go's random map order only affects the float sum
/// of the total energy (never produced by the parser).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ActionFirePhasers {
    pub energy: f64,
    pub manual_allocation: BTreeMap<i32, f64>,
}

impl ActionFirePhasers {
    pub fn execute(&self, g: &mut GameState) -> ActionResult {
        if self.energy <= 0.0 && self.manual_allocation.is_empty() {
            return Err(EngineError::new("phaser energy must be positive"));
        }
        let mut total = self.energy;
        if !self.manual_allocation.is_empty() {
            total = 0.0;
            for e in self.manual_allocation.values() {
                if *e > 0.0 {
                    total += *e;
                }
            }
        }
        if total <= 0.0 {
            return Err(EngineError::new("phaser energy must be positive"));
        }
        if g.enterprise.energy < total {
            return Err(EngineError::new("insufficient energy to fire phasers"));
        }
        if g.enterprise.condition == ConditionType::Docked {
            return Err(EngineError::new("cannot fire phasers while docked"));
        }
        let manual = !self.manual_allocation.is_empty();
        let targets = phaser_targets(g, manual);
        if targets.is_empty() {
            // A cloaked commander shows on the scanners as an unknown
            // contact; say why the phasers will not fire at it.
            if g.current_quad
                .klingons
                .iter()
                .flatten()
                .any(|k| k.borrow().is_cloaked)
            {
                return Err(EngineError::new(
                    "phasers cannot lock on to a cloaked vessel",
                ));
            }
            return Err(EngineError::new("no enemy ships in quadrant"));
        }
        g.enterprise.energy -= total;
        let mut events = vec![Event::PhaserFired(EventPhaserFired { energy: total })];

        let per_target = total / targets.len() as f64;
        let mut adversary_killed = false;
        for target in targets.iter() {
            let alloc = if manual {
                match self.manual_allocation.get(&target.id()) {
                    Some(a) if *a > 0.0 => *a,
                    _ => continue,
                }
            } else {
                per_target
            };
            let nebula = in_nebula(g);
            let dist = distance(g.enterprise.sector, target.sector());
            let damage = compute_phaser_damage(alloc, dist);
            let (damage, destroyed) = target.absorb(damage, nebula);
            let (sector, id, faction) = (target.sector(), target.id(), target.faction());
            if destroyed {
                *cell_mut(&mut g.current_quad.grid, sector) = EntityType::Empty;
                if g.remaining_klingons > 0 {
                    g.remaining_klingons -= 1;
                }
                g.klingons_remaining = g.remaining_klingons;
                dec_galaxy_klingons(g);
                match target {
                    PhaserTarget::Klingon(k) => {
                        remove_enemy_klingon(&mut g.current_quad, id);
                        if k.borrow().is_commander {
                            g.metrics.commanders_killed += 1;
                        } else {
                            g.metrics.klingons_killed += 1;
                        }
                    }
                    PhaserTarget::Adversary(_) => {
                        adversary_killed = true;
                        if faction == FactionType::Romulan {
                            g.metrics.romulans_killed += 1;
                        }
                    }
                }
            }
            events.push(Event::PhaserHit(EventPhaserHit {
                target: sector,
                klingon_id: id,
                damage,
                destroyed,
                faction,
            }));
            if let PhaserTarget::Klingon(k) = target {
                if k.borrow().is_cloaked {
                    events.extend(decloak_klingon(Some(k)));
                }
            }
        }
        if manual && g.rules.klingon_cloak {
            for kp in g.current_quad.klingons.iter() {
                let k = kl(kp);
                let (id, cmd, cloaked, energy) = {
                    let kb = k.borrow();
                    (kb.id, kb.is_commander, kb.is_cloaked, kb.energy)
                };
                let unalloc = !matches!(self.manual_allocation.get(&id), Some(a) if *a > 0.0);
                if unalloc && cmd && !cloaked && energy > 0.0 {
                    events.extend(cloak_klingon(Some(g), Some(k)));
                }
            }
        }

        let mut survivors = GoVec::nil();
        for kp in g.current_quad.klingons.iter() {
            if kl(kp).borrow().energy > 0.0 {
                survivors.push(kp.clone());
            }
        }
        g.current_quad.klingons = survivors;
        if adversary_killed {
            let mut remaining = GoVec::nil();
            for e in g.current_quad.enemies.iter() {
                let dead = e.as_ref().is_some_and(|e| {
                    let e = e.borrow();
                    e.faction != FactionType::Klingon && e.energy <= 0.0
                });
                if !dead {
                    remaining.push(e.clone());
                }
            }
            g.current_quad.enemies = remaining;
        }

        Ok(evaluate_scenario(g, events))
    }
}

/// A ship phasers can hit: a Klingon on the legacy list, or a Romulan or
/// Tholian in `Enemies`.
pub(crate) enum PhaserTarget {
    Klingon(KlingonRef),
    Adversary(EnemyRef),
}

impl PhaserTarget {
    pub(crate) fn id(&self) -> i32 {
        match self {
            PhaserTarget::Klingon(k) => k.borrow().id,
            PhaserTarget::Adversary(e) => e.borrow().id,
        }
    }

    pub(crate) fn sector(&self) -> Coord {
        match self {
            PhaserTarget::Klingon(k) => k.borrow().sector,
            PhaserTarget::Adversary(e) => e.borrow().sector,
        }
    }

    pub(crate) fn faction(&self) -> FactionType {
        match self {
            PhaserTarget::Klingon(_) => FactionType::Klingon,
            PhaserTarget::Adversary(e) => e.borrow().faction,
        }
    }

    /// Applies `damage` (shields first; a nebula collapses them) and returns
    /// the damage to report (the remaining energy for a kill) and whether
    /// the ship was destroyed.
    pub(crate) fn absorb(&self, damage: f64, nebula: bool) -> (f64, bool) {
        fn hit(shields: &mut f64, energy: &mut f64, damage: f64, nebula: bool) -> (f64, bool) {
            if nebula {
                *shields = 0.0;
            }
            let mut effective = damage;
            if *shields > 0.0 {
                if *shields >= damage {
                    *shields -= damage;
                    effective = 0.0;
                } else {
                    effective = damage - *shields;
                    *shields = 0.0;
                }
            }
            if effective >= *energy {
                let left = *energy;
                *energy = 0.0;
                (left, true)
            } else {
                *energy -= effective;
                (damage, false)
            }
        }
        match self {
            PhaserTarget::Klingon(k) => {
                let mut k = k.borrow_mut();
                let k = &mut *k;
                hit(&mut k.shields, &mut k.energy, damage, nebula)
            }
            PhaserTarget::Adversary(e) => {
                let mut e = e.borrow_mut();
                let e = &mut *e;
                hit(&mut e.shields, &mut e.energy, damage, nebula)
            }
        }
    }
}

/// The ships spread phasers divide their energy between: every uncloaked
/// Klingon (legacy list order), then every live, uncloaked Romulan and
/// Tholian (`Enemies` order). Go only ever targeted the legacy Klingon list,
/// so Romulans and Tholians were immune to spread fire, and it hit (and
/// decloaked) cloaked commanders, which no weapon can lock on to. A manual
/// allocation names its Klingons by ID, so `manual` keeps Go's rule for
/// them: cloaked Klingons are included (and a hit decloaks them).
pub(crate) fn phaser_targets(g: &GameState, manual: bool) -> Vec<PhaserTarget> {
    let mut targets = Vec::new();
    for k in g.current_quad.klingons.iter() {
        let k = kl(k);
        if manual || !k.borrow().is_cloaked {
            targets.push(PhaserTarget::Klingon(k.clone()));
        }
    }
    for e in g.current_quad.enemies.iter().flatten() {
        let eb = e.borrow();
        if eb.faction != FactionType::Klingon && eb.energy > 0.0 && !eb.is_cloaked {
            targets.push(PhaserTarget::Adversary(e.clone()));
        }
    }
    targets
}

/// Go `ActionMove`: course in radians (0 = east, pi/2 = north), warp factor,
/// optional destination sector/quadrant.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ActionMove {
    pub course: f64,
    pub warp: f64,
    pub dest_sector: Coord,
    pub dest_quad: Coord,
}

fn black_hole_death(
    g: &mut GameState,
    from_sector: Coord,
    at: Coord,
    energy_needed: f64,
    time_used: f64,
    mut events: Vec<Event>,
) -> ActionResult {
    g.enterprise.energy -= energy_needed;
    if g.enterprise.energy < 0.0 {
        g.enterprise.energy = 0.0;
    }
    g.stardate += time_used;
    g.time_remaining -= time_used;
    *cell_mut(&mut g.current_quad.grid, from_sector) = EntityType::Empty;
    g.enterprise.sector = at;
    events.push(Event::SingularityAbsorption(EventSingularityAbsorption {
        sector: at,
        target: EntityType::Enterprise,
        weapon: "ship".into(),
    }));
    events.push(Event::GameOver(EventGameOver {
        reason: GameOverReason::Lost,
        score: 0.0,
    }));
    Ok(events)
}

impl ActionMove {
    pub fn execute(&self, g: &mut GameState) -> ActionResult {
        if self.warp <= 0.0 {
            return Err(EngineError::new("warp factor must be positive"));
        }
        if g.enterprise.devices[DeviceID::Warp.index()] >= 10.0 {
            return Err(EngineError::new("warp engines inoperative"));
        }
        if g.enterprise.devices[DeviceID::Warp.index()] > 0.0 && self.warp > 4.0 {
            return Err(EngineError::new(
                "warp engines damaged; maximum speed is warp 4",
            ));
        }
        let quadrants = self.warp;
        let mut factor = self.warp;
        if factor < 1.0 {
            factor = 1.0;
        }
        let shields_mult = if g.enterprise.shields > 0.0 { 2.0 } else { 1.0 };

        let from_quad = g.enterprise.quad;
        let from_sector = g.enterprise.sector;
        let mut to_quad = from_quad;
        let mut to_sector = from_sector;

        let mut in_gravity_well = is_near_black_hole(&g.current_quad.grid, from_sector);
        if !in_gravity_well
            && self.dest_sector != [0, 0]
            && (self.dest_quad == [0, 0] || self.dest_quad == from_quad)
            && is_near_black_hole(&g.current_quad.grid, self.dest_sector)
        {
            in_gravity_well = true;
        }
        let mut gravity_mult = if in_gravity_well { 2.0 } else { 1.0 };

        let mut energy_needed = quadrants * factor * factor * factor * shields_mult * gravity_mult;
        if g.enterprise.energy < energy_needed {
            return Err(EngineError::new("insufficient energy for warp movement"));
        }
        let time_used = 10.0 * quadrants / (factor * factor);
        let mut events: Vec<Event> = Vec::new();

        if self.dest_quad != [0, 0] {
            let dq = self.dest_quad;
            if dq[0] < 1 || dq[0] > 8 || dq[1] < 1 || dq[1] > 8 {
                return Err(EngineError::new("destination quadrant out of bounds"));
            }
            to_quad = dq;
            if self.dest_sector != [0, 0] {
                let ds = self.dest_sector;
                if ds[0] < 1 || ds[0] > 8 || ds[1] < 1 || ds[1] > 8 {
                    return Err(EngineError::new("destination sector out of bounds"));
                }
                if to_quad == from_quad {
                    let cl = cell(&g.current_quad.grid, ds);
                    if cl == EntityType::BlackHole {
                        return black_hole_death(
                            g,
                            from_sector,
                            ds,
                            energy_needed,
                            time_used,
                            Vec::new(),
                        );
                    }
                    if cl == EntityType::Wormhole {
                        return handle_wormhole_jump(
                            g,
                            self,
                            from_quad,
                            from_sector,
                            ds,
                            energy_needed,
                            time_used,
                            events,
                        );
                    }
                    if cl != EntityType::Empty && ds != from_sector {
                        return Err(EngineError::new("destination sector is occupied"));
                    }
                }
                to_sector = ds;
            } else {
                to_sector = [4, 4];
                if to_quad == from_quad
                    && cell(&g.current_quad.grid, to_sector) != EntityType::Empty
                    && to_sector != from_sector
                {
                    return Err(EngineError::new("destination sector is occupied"));
                }
            }
        } else if self.dest_sector != [0, 0] {
            let ds = self.dest_sector;
            if ds[0] < 1 || ds[0] > 8 || ds[1] < 1 || ds[1] > 8 {
                return Err(EngineError::new("destination sector out of bounds"));
            }
            let cl = cell(&g.current_quad.grid, ds);
            if cl == EntityType::BlackHole {
                return black_hole_death(g, from_sector, ds, energy_needed, time_used, Vec::new());
            }
            if cl == EntityType::Wormhole {
                return handle_wormhole_jump(
                    g,
                    self,
                    from_quad,
                    from_sector,
                    ds,
                    energy_needed,
                    time_used,
                    events,
                );
            }
            if cl != EntityType::Empty {
                return Err(EngineError::new("destination sector is occupied"));
            }
            to_sector = ds;
        } else {
            let dr = -gomath::sin(self.course);
            let dc = gomath::cos(self.course);
            let mut num_steps = gomath::go_int(gomath::round(self.warp * 8.0));
            if num_steps < 1 {
                num_steps = 1;
            }
            let current_r = from_sector[0] as f64;
            let current_c = from_sector[1] as f64;
            let mut hit_obstacle = false;
            let mut exited_quad = false;
            let (mut drift_r, mut drift_c) = (0i64, 0i64);
            for step in 1..=num_steps {
                let mut next_r = gomath::go_int(gomath::round(current_r + step as f64 * dr))
                    .wrapping_add(drift_r);
                let mut next_c = gomath::go_int(gomath::round(current_c + step as f64 * dc))
                    .wrapping_add(drift_c);

                if g.quadrant_env[from_quad[0] as usize][from_quad[1] as usize]
                    == EnvironmentType::IonStorm
                    && g.rng.is_some()
                    && g.rng().float64() < 0.25
                {
                    let mut delta = 1;
                    if g.rng().float64() < 0.5 {
                        delta = -1;
                    }
                    if dc.abs() >= dr.abs() {
                        next_r = next_r.wrapping_add(delta);
                        drift_r += delta;
                    } else {
                        next_c = next_c.wrapping_add(delta);
                        drift_c += delta;
                    }
                    events.push(Event::HazardTriggered(EventHazardTriggered {
                        hazard_type: "ion_storm_drift".into(),
                        description: "Ion storm turbulence deflected course".into(),
                        energy_drain: 0.0,
                    }));
                }

                if !(1..=8).contains(&next_r) || !(1..=8).contains(&next_c) {
                    exited_quad = true;
                    break;
                }
                let (next_r, next_c) = (next_r as i32, next_c as i32);
                let cl = cell(&g.current_quad.grid, [next_r, next_c]);
                if cl == EntityType::BlackHole {
                    return black_hole_death(
                        g,
                        from_sector,
                        [next_r, next_c],
                        energy_needed,
                        time_used,
                        events,
                    );
                }
                if cl == EntityType::Wormhole {
                    return handle_wormhole_jump(
                        g,
                        self,
                        from_quad,
                        from_sector,
                        [next_r, next_c],
                        energy_needed,
                        time_used,
                        events,
                    );
                }
                if is_near_black_hole(&g.current_quad.grid, [next_r, next_c]) && !in_gravity_well {
                    in_gravity_well = true;
                    gravity_mult = 2.0;
                    energy_needed =
                        quadrants * factor * factor * factor * shields_mult * gravity_mult;
                }
                if cl != EntityType::Empty && cl != EntityType::Enterprise {
                    events.push(Event::ObstacleEncountered(EventObstacleEncountered {
                        sector: [next_r, next_c],
                        entity: cl,
                    }));
                    hit_obstacle = true;
                    break;
                }
                to_sector = [next_r, next_c];
            }

            if !hit_obstacle && exited_quad {
                let total_r = 8 * (from_quad[0] - 1) + from_sector[0];
                let total_c = 8 * (from_quad[1] - 1) + from_sector[1];
                let dest_r = gomath::go_int(gomath::round(total_r as f64 + num_steps as f64 * dr))
                    .clamp(1, 64) as i32;
                let dest_c = gomath::go_int(gomath::round(total_c as f64 + num_steps as f64 * dc))
                    .clamp(1, 64) as i32;
                to_quad = [(dest_r - 1) / 8 + 1, (dest_c - 1) / 8 + 1];
                to_sector = [(dest_r - 1) % 8 + 1, (dest_c - 1) % 8 + 1];
            }
        }

        // Apply movement mutations
        g.enterprise.energy -= energy_needed;
        if g.enterprise.energy <= 0.0 {
            g.enterprise.energy = 0.0;
            events.push(Event::GameOver(EventGameOver {
                reason: GameOverReason::Energy,
                score: 0.0,
            }));
        }
        g.stardate += time_used;
        g.time_remaining -= time_used;

        if let Some(refits) = &g.active_refits {
            let tier = refits.get(&RefitID::damage_nanites()).copied().unwrap_or(0);
            if tier > 0 {
                let boost = tier as f64 * 0.5;
                for d in g.enterprise.damage.iter_mut() {
                    if *d > 0.0 {
                        *d = gomath::builtin_max(0.0, *d - boost);
                    }
                }
                for d in g.enterprise.devices.iter_mut() {
                    if *d > 0.0 {
                        *d = gomath::builtin_max(0.0, *d - boost);
                    }
                }
            }
        }

        if g.enterprise.condition == ConditionType::Docked {
            g.enterprise.condition = ConditionType::Green;
        }

        if to_quad != from_quad {
            enter_nebula(g, to_quad, &mut events);
            g.populate_quadrant(to_quad, to_sector);
        } else {
            *cell_mut(&mut g.current_quad.grid, from_sector) = EntityType::Empty;
            *cell_mut(&mut g.current_quad.grid, to_sector) = EntityType::Enterprise;
            g.enterprise.sector = to_sector;
            g.enterprise.quad = to_quad;
        }

        events.insert(
            0,
            Event::ShipMoved(EventShipMoved {
                from_quad,
                to_quad,
                from_sector,
                to_sector,
                warp: self.warp,
                energy_used: energy_needed,
                time_used,
            }),
        );

        if to_quad == from_quad && g.rules.klingon_cloak {
            cloak_all_commanders(g, &mut events, None);
        }
        Ok(evaluate_scenario(g, events))
    }
}

fn enter_nebula(g: &mut GameState, to_quad: Coord, events: &mut Vec<Event>) {
    if g.quadrant_env[to_quad[0] as usize][to_quad[1] as usize] == EnvironmentType::Nebula {
        if g.enterprise.energy > 0.0 {
            g.enterprise.energy += g.enterprise.shields;
            if g.enterprise.energy > 5000.0 {
                g.enterprise.energy = 5000.0;
            }
        }
        g.enterprise.shields = 0.0;
        events.push(Event::AnomalyDiscovered(EventAnomalyDiscovered {
            quad: to_quad,
            env: EnvironmentType::Nebula,
        }));
    }
}

/// True when `events` record the destruction of a hostile ship (torpedo or
/// phaser kill, or a Klingon swallowed by a singularity).
pub fn events_destroy_hostile(events: &[Event]) -> bool {
    events.iter().any(|e| match e {
        Event::TorpedoHit(h) => {
            h.destroyed
                && matches!(
                    h.entity,
                    EntityType::Klingon
                        | EntityType::Commander
                        | EntityType::SuperCommander
                        | EntityType::Romulan
                        | EntityType::Tholian
                )
        }
        Event::PhaserHit(h) => h.destroyed,
        Event::SingularityAbsorption(a) => matches!(
            a.target,
            EntityType::Klingon | EntityType::Commander | EntityType::SuperCommander
        ),
        _ => false,
    })
}

/// Classic victory check for games without a scenario (the Go engine had
/// none, so destroying every Klingon never ended a normal game): when
/// `killed` reports that this turn destroyed a hostile and no hostiles remain,
/// the game is won and a `GameOver(Won)` with the classic score (including
/// the skill-based win bonus) is appended. Scenario games keep their own
/// evaluation; a turn that already ended the game is left alone.
pub fn evaluate_victory(g: &mut GameState, killed: bool, events: &mut Vec<Event>) {
    if g.scenario != ScenarioID::none() || g.game_over || !killed || g.remaining_klingons > 0 {
        return;
    }
    if events.iter().any(|e| matches!(e, Event::GameOver(_))) {
        return;
    }
    g.game_won = true;
    let score = compute_score(Some(g), true).total_score as f64;
    events.push(Event::GameOver(EventGameOver {
        reason: GameOverReason::Won,
        score,
    }));
}

/// Go `evaluateScenario`: applies an active scenario's win/loss evaluation
/// (for games without a scenario, the classic victory check).
pub fn evaluate_scenario(g: &mut GameState, mut events: Vec<Event>) -> Vec<Event> {
    if g.scenario == ScenarioID::none() {
        let killed = events_destroy_hostile(&events);
        evaluate_victory(g, killed, &mut events);
        return events;
    }
    let Some(s) = get_scenario(&g.scenario) else {
        return events;
    };
    let Some(eval) = &s.evaluate else {
        return events;
    };
    let (done, won, reason) = eval(g);
    if done {
        g.game_won = won;
        let score_val = match &s.compute_score {
            Some(cs) => cs(g, won).total_score as f64,
            None => compute_score(Some(g), won).total_score as f64,
        };
        let mut found = false;
        for ev in events.iter_mut() {
            if let Event::GameOver(_) = ev {
                *ev = Event::GameOver(EventGameOver {
                    reason,
                    score: score_val,
                });
                found = true;
                break;
            }
        }
        if !found {
            events.push(Event::GameOver(EventGameOver {
                reason,
                score: score_val,
            }));
        }
    }
    events
}

/// Go `isNearBlackHole`.
pub fn is_near_black_hole(grid: &[[EntityType; 9]; 9], sector: Coord) -> bool {
    for dr in -1..=1 {
        for dc in -1..=1 {
            let r = sector[0] + dr;
            let c = sector[1] + dc;
            if (1..=8).contains(&r)
                && (1..=8).contains(&c)
                && grid[r as usize][c as usize] == EntityType::BlackHole
            {
                return true;
            }
        }
    }
    false
}

#[allow(clippy::too_many_arguments)]
fn handle_wormhole_jump(
    g: &mut GameState,
    a: &ActionMove,
    from_quad: Coord,
    from_sector: Coord,
    wormhole_sector: Coord,
    energy_needed: f64,
    time_used: f64,
    mut events: Vec<Event>,
) -> ActionResult {
    if g.rng.is_none() {
        return Err(EngineError::new(
            "cannot execute wormhole jump: RNG is not initialized",
        ));
    }
    let mut target_quad = [g.rng().intn(8) + 1, g.rng().intn(8) + 1];
    let mut target_sector = [g.rng().intn(8) + 1, g.rng().intn(8) + 1];
    while target_quad == from_quad && target_sector == wormhole_sector {
        target_quad = [g.rng().intn(8) + 1, g.rng().intn(8) + 1];
        target_sector = [g.rng().intn(8) + 1, g.rng().intn(8) + 1];
    }

    g.enterprise.energy -= energy_needed;
    if g.enterprise.energy <= 0.0 {
        g.enterprise.energy = 0.0;
        events.push(Event::GameOver(EventGameOver {
            reason: GameOverReason::Energy,
            score: 0.0,
        }));
    }
    g.stardate += time_used + 0.2;
    g.time_remaining -= time_used + 0.2;
    if g.enterprise.condition == ConditionType::Docked {
        g.enterprise.condition = ConditionType::Green;
    }

    let mut jump = EventWormholeJump {
        from_quad,
        from_sector,
        to_quad: target_quad,
        to_sector: target_sector,
        time_delta: 0.2,
    };
    events.push(Event::WormholeJump(jump.clone()));

    if target_quad != from_quad {
        enter_nebula(g, target_quad, &mut events);
        g.populate_quadrant(target_quad, target_sector);
    } else {
        loop {
            let c = cell(&g.current_quad.grid, target_sector);
            if (c != EntityType::Empty && c != EntityType::Enterprise)
                || target_sector == wormhole_sector
            {
                target_sector = [g.rng().intn(8) + 1, g.rng().intn(8) + 1];
            } else {
                break;
            }
        }
        jump.to_sector = target_sector;
        let n = events.len();
        events[n - 1] = Event::WormholeJump(jump);
        *cell_mut(&mut g.current_quad.grid, from_sector) = EntityType::Empty;
        *cell_mut(&mut g.current_quad.grid, target_sector) = EntityType::Enterprise;
        g.enterprise.sector = target_sector;
        g.enterprise.quad = target_quad;
    }

    events.insert(
        0,
        Event::ShipMoved(EventShipMoved {
            from_quad,
            to_quad: target_quad,
            from_sector,
            to_sector: target_sector,
            warp: a.warp,
            energy_used: energy_needed,
            time_used: time_used + 0.2,
        }),
    );

    if target_quad == from_quad && g.rules.klingon_cloak {
        cloak_all_commanders(g, &mut events, None);
    }
    Ok(evaluate_scenario(g, events))
}

/// Go `ActionLRScan`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ActionLRScan;

impl ActionLRScan {
    pub fn execute(&self, g: &mut GameState) -> ActionResult {
        let lrs = g.enterprise.devices[DeviceID::LRSensors.index()];
        if g.enterprise.condition != ConditionType::Docked && lrs >= 2.0 {
            return Err(EngineError::new("long-range sensors damaged"));
        }
        let relayed = g.enterprise.condition == ConditionType::Docked && lrs > 0.0;
        let degraded = g.rules.sensor_degradation && lrs > 0.0 && !relayed;
        let center = g.enterprise.quad;
        let mut scanned = Vec::new();
        let mut readings = [[0i32; 3]; 3];
        let mut map = std::collections::BTreeMap::new();
        for dr in -1..=1i32 {
            for dc in -1..=1i32 {
                let r = center[0] + dr;
                let c = center[1] + dc;
                let (ri, ci) = ((dr + 1) as usize, (dc + 1) as usize);
                if (1..=8).contains(&r) && (1..=8).contains(&c) {
                    g.chart_discovered[r as usize][c as usize] = true;
                    scanned.push([r, c]);
                    if g.quadrant_env[r as usize][c as usize] == EnvironmentType::Nebula {
                        readings[ri][ci] = -1;
                        map.insert([r, c], -1);
                    } else {
                        let v = g.galaxy_chart[r as usize][c as usize];
                        readings[ri][ci] = v;
                        map.insert([r, c], v);
                    }
                } else {
                    readings[ri][ci] = -1;
                    map.insert([r, c], -1);
                }
            }
        }
        Ok(vec![Event::LRScanCompleted(EventLRScanCompleted {
            center_quad: center,
            scanned_quads: scanned,
            relayed_by_base: relayed,
            degraded,
            readings,
            readings_map: map,
        })])
    }

    /// Go `ActionLRScan.ReadQuadrant`.
    pub fn read_quadrant(&self, g: Option<&GameState>, qr: i32, qc: i32) -> i32 {
        scan_quadrant(g, qr, qc)
    }
}

/// Go `ScanQuadrant`: LRS value or -1 (nebula / out of bounds).
pub fn scan_quadrant(g: Option<&GameState>, qr: i32, qc: i32) -> i32 {
    let Some(g) = g else { return -1 };
    if !(1..=8).contains(&qr) || !(1..=8).contains(&qc) {
        return -1;
    }
    if g.quadrant_env[qr as usize][qc as usize] == EnvironmentType::Nebula {
        return -1;
    }
    g.galaxy_chart[qr as usize][qc as usize]
}

/// Go `ActionKlingonCounterAttack`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ActionKlingonCounterAttack {
    pub enemy_id: i32,
    pub damage: f64,
}

impl ActionKlingonCounterAttack {
    pub fn execute(&self, g: &mut GameState) -> ActionResult {
        if self.damage <= 0.0 {
            return Err(EngineError::new("counter attack damage must be positive"));
        }
        let mut attacker = None;
        for k in g.current_quad.klingons.iter() {
            if kl(k).borrow().id == self.enemy_id {
                attacker = Some(kl(k).clone());
                break;
            }
        }
        let Some(attacker) = attacker else {
            return Err(EngineError::new("attacker not found in quadrant"));
        };
        let events = klingon_counter_attack(g, &attacker, self.damage);
        Ok(evaluate_scenario(g, events))
    }
}

/// Go `ActionCallHelp`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ActionCallHelp;

impl ActionCallHelp {
    pub fn execute(&self, g: &mut GameState) -> ActionResult {
        g.metrics.help_calls += 1;
        Ok(vec![Event::HelpCalled(EventHelpCalled {})])
    }
}

// --- Go encoding/json encodings of the action structs (untagged Go field names) ---

use crate::json::{Decoder, Encoder, GoJson, Value};

crate::go_json_struct!(ActionShields, "ActionShields", "engine.ActionShields" { amount => "Amount" });
crate::go_json_struct!(ActionDock, "ActionDock", "engine.ActionDock" {});
crate::go_json_struct!(ActionFireTorpedo, "ActionFireTorpedo", "engine.ActionFireTorpedo" { target => "Target", angle => "Angle", direction => "Direction" });
crate::go_json_struct!(ActionMove, "ActionMove", "engine.ActionMove" { course => "Course", warp => "Warp", dest_sector => "DestSector", dest_quad => "DestQuad" });
crate::go_json_struct!(ActionLRScan, "ActionLRScan", "engine.ActionLRScan" {});
crate::go_json_struct!(ActionKlingonCounterAttack, "ActionKlingonCounterAttack", "engine.ActionKlingonCounterAttack" { enemy_id => "EnemyID", damage => "Damage" });
crate::go_json_struct!(ActionCallHelp, "ActionCallHelp", "engine.ActionCallHelp" {});
crate::go_json_struct!(ActionTorpedoDirect, "ActionTorpedoDirect", "engine.ActionTorpedoDirect" { target_sector => "TargetSector" });
crate::go_json_struct!(ActionPhaserDirect, "ActionPhaserDirect", "engine.ActionPhaserDirect" { target_sector => "TargetSector", energy => "Energy" });

/// `ManualAllocation` is a Go `map[int]float64`; an empty map encodes as
/// `null` (nil) and keys are sorted as strings, like Go.
impl GoJson for ActionFirePhasers {
    fn encode_json(&self, e: &mut Encoder) {
        let mut st = e.begin_object();
        e.field(&mut st, "Energy");
        e.float(self.energy);
        e.field(&mut st, "ManualAllocation");
        if self.manual_allocation.is_empty() {
            e.null();
        } else {
            let mut keys: Vec<(String, f64)> = self
                .manual_allocation
                .iter()
                .map(|(k, v)| (k.to_string(), *v))
                .collect();
            keys.sort_by(|a, b| a.0.cmp(&b.0));
            let mut s2 = e.begin_object();
            for (k, v) in keys {
                e.field(&mut s2, &k);
                e.float(v);
            }
            e.end_object();
        }
        e.end_object();
    }
    fn decode_json(&mut self, v: &Value, d: &mut Decoder) {
        if let Value::Object(m) = v {
            for (k, val) in m {
                match crate::json::match_field(&["Energy", "ManualAllocation"], k) {
                    Some(0) => self.energy.decode_json(val, d),
                    Some(1) => {
                        if let Value::Object(entries) = val {
                            for (mk, mv) in entries {
                                let mut f = 0.0;
                                f.decode_json(mv, d);
                                if let Ok(key) = mk.parse::<i32>() {
                                    self.manual_allocation.insert(key, f);
                                }
                            }
                        } else if let Value::Null = val {
                            self.manual_allocation.clear();
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    fn go_type_name() -> String {
        "engine.ActionFirePhasers".into()
    }
}

impl Action {
    /// `{"Type": "<%T>", "Data": <action>}` (parity transcript format).
    pub fn to_json(&self) -> String {
        let mut e = Encoder::default();
        let mut st = e.begin_object();
        e.field(&mut st, "Type");
        e.string(self.type_name());
        e.field(&mut st, "Data");
        match self {
            Action::Shields(a) => a.encode_json(&mut e),
            Action::Dock(a) => a.encode_json(&mut e),
            Action::FireTorpedo(a) => a.encode_json(&mut e),
            Action::FirePhasers(a) => a.encode_json(&mut e),
            Action::Move(a) => a.encode_json(&mut e),
            Action::LRScan(a) => a.encode_json(&mut e),
            Action::KlingonCounterAttack(a) => a.encode_json(&mut e),
            Action::CallHelp(a) => a.encode_json(&mut e),
            Action::TorpedoDirect(a) => a.encode_json(&mut e),
            Action::PhaserDirect(a) => a.encode_json(&mut e),
        }
        e.end_object();
        match e.err {
            Some(err) => format!("ERR {}", err),
            None => String::from_utf8(e.buf).expect("utf-8"),
        }
    }
}
