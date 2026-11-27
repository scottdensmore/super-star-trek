//! Unified adversary turn and targeted phasers (port of `pkg/engine/tactics.go`).

use std::rc::Rc;

use crate::actions::{
    dec_galaxy_klingons, evaluate_victory, phaser_targets, ActionResult, EngineError, PhaserTarget,
};
use crate::combat::{compute_phaser_damage, klingon_counter_attack, temp_klingon};
use crate::events::*;
use crate::geometry::distance;
use crate::klingon_pack::{calculate_pack_damage_multiplier, execute_commander_screening};
use crate::romulan::{advance_plasma_torpedoes, intercept_plasma_torpedo, romulan_turn};
use crate::state::{
    cell, cell_mut, remove_enemy_klingon, sync_enemies_from_klingons, sync_quadrant_enemies,
    GameState, Klingon,
};
use crate::tholian::{damage_web_segment, tholian_turn};
use crate::types::*;

/// Go `ComputeDistance`.
pub fn compute_distance(c1: Coord, c2: Coord) -> f64 {
    distance(c1, c2)
}

/// Go `UnifiedAdversaryTurn`: plasma torpedoes, Klingon screening and
/// crossfire, Romulans, Tholians, then a legacy-list resync.
pub fn unified_adversary_turn(g: &mut GameState) -> Vec<Event> {
    // Carry phaser/torpedo damage and kills on the legacy Klingon list over
    // to Enemies before acting on (and finally resyncing from) Enemies.
    sync_enemies_from_klingons(&mut g.current_quad);
    let mut events = Vec::new();
    events.extend(advance_plasma_torpedoes(g));
    events.extend(execute_commander_screening(g));

    let mult = calculate_pack_damage_multiplier(Some(g));
    let list = g.current_quad.enemies.to_vec();
    for e in list.iter().flatten() {
        let (faction, energy) = {
            let eb = e.borrow();
            (eb.faction, eb.energy)
        };
        if faction == FactionType::Klingon && energy > 0.0 {
            let k = {
                let eb = e.borrow();
                temp_klingon(Klingon {
                    id: eb.id,
                    sector: eb.sector,
                    is_commander: eb.is_commander,
                    is_cloaked: eb.is_cloaked,
                    ..Default::default()
                })
            };
            let base = energy * 0.35 * mult;
            events.extend(klingon_counter_attack(g, &k, base));
            e.borrow_mut().is_cloaked = k.borrow().is_cloaked;
        }
    }

    let list = g.current_quad.enemies.to_vec();
    for e in list.iter().flatten() {
        let (faction, energy) = {
            let eb = e.borrow();
            (eb.faction, eb.energy)
        };
        if faction == FactionType::Romulan && energy > 0.0 {
            events.extend(romulan_turn(g, Some(e)));
        }
    }

    events.extend(tholian_turn(g));
    sync_quadrant_enemies(&mut g.current_quad);
    // Go never checked for the Enterprise's destruction here, so Romulan,
    // plasma and pack fire could drive its energy negative without ending the
    // game; end it the way KlingonTurn does.
    if g.enterprise.energy <= 0.0 {
        g.enterprise.energy = 0.0;
        events.push(Event::GameOver(EventGameOver {
            reason: GameOverReason::Lost,
            score: 0.0,
        }));
    }
    events
}

/// The hint added to a weapons refusal when every hostile in the quadrant is
/// cloaked: a torpedo fired along a line still hits a cloaked ship (and
/// decloaks it), which is the only way to fight it.
pub const CLOAK_HINT: &str =
    "Fire photon torpedoes along its last bearing to force it out of cloak.";

/// Whether no hostile in the current quadrant is visible: every Klingon on
/// the legacy list and every surviving ship in `Enemies` is cloaked.
pub fn only_cloaked_hostiles(g: &GameState) -> bool {
    let klingons = g
        .current_quad
        .klingons
        .iter()
        .flatten()
        .all(|k| k.borrow().is_cloaked);
    let enemies = g.current_quad.enemies.iter().flatten().all(|e| {
        let e = e.borrow();
        e.is_cloaked || e.energy <= 0.0
    });
    klingons && enemies
}

/// The target-lock refusal for a cloaked ship at `t` (a cloaked Klingon
/// commander or Romulan), shared by focused phasers and
/// [`crate::combat::ActionTorpedoDirect`]. When no visible ship is left to
/// fight it carries [`CLOAK_HINT`] on a second line.
pub fn cloaked_at(g: &GameState, t: Coord) -> Option<String> {
    let klingon = g
        .current_quad
        .klingons
        .iter()
        .flatten()
        .any(|k| k.borrow().sector == t && k.borrow().is_cloaked);
    let enemy = g.current_quad.enemies.iter().flatten().any(|e| {
        let e = e.borrow();
        e.sector == t && e.is_cloaked && e.energy > 0.0
    });
    if !(klingon || enemy) {
        return None;
    }
    let why = "TARGET LOCK FAILED: CLOAKED VESSEL";
    Some(if only_cloaked_hostiles(g) {
        format!("{why}\n{CLOAK_HINT}")
    } else {
        why.to_string()
    })
}

/// The visible ship at `t` for focused phasers: a Klingon on the legacy
/// list (where phasers, torpedoes and `KlingonTurn` keep Klingon damage, in
/// every game) or a Romulan or Tholian in `Enemies`.
fn focused_target(g: &GameState, t: Coord) -> Option<PhaserTarget> {
    phaser_targets(g, false).into_iter().find(|p| match p {
        PhaserTarget::Klingon(k) => k.borrow().sector == t && k.borrow().energy > 0.0,
        PhaserTarget::Adversary(_) => p.sector() == t,
    })
}

/// Go `ActionPhaserDirect`: focused phasers at a sector (ship, plasma torpedo or web).
///
/// Go looked for the target only in `Enemies`, which is empty in a game
/// without adversaries, so focused phasers at a Klingon drained the energy
/// and did nothing; it also hit cloaked Romulans that target lock refuses.
/// The target is now any visible ship ([`focused_target`]); a cloaked ship
/// refuses the lock like [`crate::combat::ActionTorpedoDirect`], before any
/// energy is spent.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ActionPhaserDirect {
    pub target_sector: Coord,
    pub energy: f64,
}

impl ActionPhaserDirect {
    pub fn execute(&self, g: &mut GameState) -> ActionResult {
        if self.energy <= 0.0 {
            return Err(EngineError::new("phaser energy must be positive"));
        }
        if g.enterprise.energy < self.energy {
            return Err(EngineError::new("insufficient energy for phasers"));
        }
        let t = self.target_sector;
        if t[0] < 1 || t[0] > 8 || t[1] < 1 || t[1] > 8 {
            return Err(EngineError::new("target sector out of bounds"));
        }
        if let Some(why) = cloaked_at(g, t) {
            return Err(EngineError::new(why));
        }
        g.enterprise.energy -= self.energy;
        let mut events = Vec::new();
        match cell(&g.current_quad.grid, t) {
            EntityType::PlasmaTorpedo => {
                let (ok, ev) = intercept_plasma_torpedo(g, t, self.energy);
                if ok {
                    events.extend(ev);
                    g.metrics.romulans_surrendered += 1;
                }
            }
            EntityType::TholianWeb => {
                let (ok, ev) = damage_web_segment(g, t, self.energy);
                if ok {
                    events.extend(ev);
                }
            }
            _ => {
                let Some(target) = focused_target(g, t) else {
                    return Ok(events);
                };
                let dist = compute_distance(g.enterprise.sector, t);
                let dmg = compute_phaser_damage(self.energy, dist);
                let (damage, dead) = target.absorb(dmg, false);
                let (id, faction) = (target.id(), target.faction());
                events.push(Event::PhaserHit(EventPhaserHit {
                    target: t,
                    klingon_id: id,
                    damage,
                    destroyed: dead,
                    faction,
                }));
                match &target {
                    PhaserTarget::Klingon(k) => {
                        if dead {
                            if k.borrow().is_commander {
                                g.metrics.commanders_killed += 1;
                            } else {
                                g.metrics.klingons_killed += 1;
                            }
                            g.current_quad
                                .klingons
                                .vec_mut()
                                .retain(|o| !o.as_ref().is_some_and(|o| Rc::ptr_eq(o, k)));
                            remove_enemy_klingon(&mut g.current_quad, id);
                        } else {
                            sync_enemies_from_klingons(&mut g.current_quad);
                        }
                    }
                    PhaserTarget::Adversary(e) => {
                        if dead {
                            if faction == FactionType::Romulan {
                                g.metrics.romulans_killed += 1;
                            }
                            g.current_quad
                                .enemies
                                .vec_mut()
                                .retain(|o| !o.as_ref().is_some_and(|o| Rc::ptr_eq(o, e)));
                        }
                    }
                }
                if dead {
                    *cell_mut(&mut g.current_quad.grid, t) = EntityType::Empty;
                    if g.remaining_klingons > 0 {
                        g.remaining_klingons -= 1;
                    }
                    g.klingons_remaining = g.remaining_klingons;
                    dec_galaxy_klingons(g);
                    evaluate_victory(g, true, &mut events);
                }
            }
        }
        Ok(events)
    }
}
