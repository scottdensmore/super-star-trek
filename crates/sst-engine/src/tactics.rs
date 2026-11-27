//! Unified adversary turn and targeted phasers (port of `pkg/engine/tactics.go`).

use crate::actions::{dec_galaxy_klingons, evaluate_victory, ActionResult, EngineError};
use crate::combat::{compute_phaser_damage, klingon_counter_attack, temp_klingon};
use crate::events::*;
use crate::geometry::distance;
use crate::json::GoVec;
use crate::klingon_pack::{calculate_pack_damage_multiplier, execute_commander_screening};
use crate::romulan::{advance_plasma_torpedoes, intercept_plasma_torpedo, romulan_turn};
use crate::state::{
    cell, cell_mut, sync_enemies_from_klingons, sync_quadrant_enemies, GameState, Klingon,
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

/// Go `ActionPhaserDirect`: focused phasers at a sector (ship, plasma torpedo or web).
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
        g.enterprise.energy -= self.energy;
        sync_enemies_from_klingons(&mut g.current_quad);
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
                let dist = compute_distance(g.enterprise.sector, t);
                let dmg = compute_phaser_damage(self.energy, dist);
                let list = g.current_quad.enemies.to_vec();
                for e in list.iter().flatten() {
                    let hit = {
                        let eb = e.borrow();
                        eb.sector == t && eb.energy > 0.0
                    };
                    if !hit {
                        continue;
                    }
                    let energy_before = e.borrow().energy;
                    let dead = {
                        let mut eb = e.borrow_mut();
                        eb.shields -= dmg;
                        if eb.shields < 0.0 {
                            eb.energy += eb.shields;
                            eb.shields = 0.0;
                        }
                        if eb.energy <= 0.0 {
                            eb.energy = 0.0;
                            true
                        } else {
                            false
                        }
                    };
                    let (sector, faction, id, is_cmd) = {
                        let eb = e.borrow();
                        (eb.sector, eb.faction, eb.id, eb.is_commander)
                    };
                    if faction == FactionType::Klingon {
                        // Go left the legacy Klingon list (which phasers,
                        // torpedoes and KlingonTurn use) untouched here, and
                        // never counted a Klingon kill.
                        events.push(Event::PhaserHit(EventPhaserHit {
                            target: sector,
                            klingon_id: id,
                            damage: if dead { energy_before } else { dmg },
                            destroyed: dead,
                        }));
                        if dead {
                            if is_cmd {
                                g.metrics.commanders_killed += 1;
                            } else {
                                g.metrics.klingons_killed += 1;
                            }
                            dec_galaxy_klingons(g);
                        }
                    }
                    if dead {
                        *cell_mut(&mut g.current_quad.grid, sector) = EntityType::Empty;
                        if faction == FactionType::Romulan {
                            g.metrics.romulans_killed += 1;
                        }
                        let mut remaining = GoVec::nil();
                        for other in g.current_quad.enemies.iter().flatten() {
                            let ob = other.borrow();
                            if ob.id != id && ob.energy > 0.0 {
                                remaining.push(Some(other.clone()));
                            }
                        }
                        g.current_quad.enemies = remaining;
                        sync_quadrant_enemies(&mut g.current_quad);
                        if g.remaining_klingons > 0 {
                            g.remaining_klingons -= 1;
                        }
                        g.klingons_remaining = g.remaining_klingons;
                        evaluate_victory(g, true, &mut events);
                    } else if faction == FactionType::Klingon {
                        sync_quadrant_enemies(&mut g.current_quad);
                    }
                    break;
                }
            }
        }
        Ok(events)
    }
}
