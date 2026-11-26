//! Dispatch and turn advancement (port of `pkg/engine/engine.go`), plus the
//! post-action adversary turn used by the TUI and the WASM session.

use crate::actions::{Action, ActionResult};
use crate::combat::{cloak_klingon, klingon_turn};
use crate::events::Event;
use crate::state::GameState;
use crate::tactics::unified_adversary_turn;
use crate::types::ConditionType;

impl GameState {
    /// Go `GameState.Dispatch`.
    pub fn dispatch(&mut self, action: &Action) -> ActionResult {
        action.execute(self)
    }

    /// Go `GameState.AdvanceTurn`.
    pub fn advance_turn(&mut self, elapsed: f64) {
        advance_repairs(self, elapsed);
        if self.rules.klingon_cloak {
            let list = self.current_quad.klingons.to_vec();
            for k in list.iter().flatten() {
                let (cmd, cloaked) = {
                    let kb = k.borrow();
                    (kb.is_commander, kb.is_cloaked)
                };
                if cmd && !cloaked {
                    cloak_klingon(Some(self), Some(k));
                }
            }
        }
    }
}

/// Go `AdvanceRepairs`.
pub fn advance_repairs(g: &mut GameState, elapsed: f64) {
    if elapsed <= 0.0 {
        return;
    }
    let mut mult = g.rules.repair_multiplier;
    if mult <= 0.0 {
        mult = 1.0;
    }
    let step = elapsed / mult;
    for d in g.enterprise.devices.iter_mut() {
        if *d > 0.0 {
            *d -= step;
            if *d < 0.0 {
                *d = 0.0;
            }
        }
    }
}

fn has_game_over(events: &[Event]) -> bool {
    events.iter().any(|e| matches!(e, Event::GameOver(_)))
}

fn is_inter_quad(events: &[Event]) -> bool {
    events
        .iter()
        .any(|e| matches!(e, Event::ShipMoved(m) if m.from_quad != m.to_quad))
}

/// The TUI's `executeKlingonTurnIfActive` (pkg/tui/update.go): after a
/// turn-consuming action, run `UnifiedAdversaryTurn` (when adversary
/// factions are present) or `KlingonTurn`, appending their events.
pub fn execute_klingon_turn_if_active(g: &mut GameState, action: &Action, events: &mut Vec<Event>) {
    if has_game_over(events) {
        return;
    }
    let should_attack = match action {
        Action::FireTorpedo(_)
        | Action::TorpedoDirect(_)
        | Action::FirePhasers(_)
        | Action::PhaserDirect(_)
        | Action::Shields(_) => true,
        Action::Move(m) => !is_inter_quad(events) && m.dest_quad == [0, 0],
        _ => false,
    };
    if should_attack && g.enterprise.condition != ConditionType::Docked {
        if !g.current_quad.enemies.is_empty()
            || g.rules.adversaries
            || !g.current_quad.plasma_torpedoes.is_empty()
            || !g.current_quad.web_segments.is_empty()
        {
            let k = unified_adversary_turn(g);
            events.extend(k);
        } else if !g.current_quad.klingons.is_empty() {
            let k = klingon_turn(g);
            events.extend(k);
        }
    }
}

/// The WASM session's counter-attack rule (cmd/wasm/session.go `Execute`):
/// only `KlingonTurn`, only for torpedo/phaser/shield actions and
/// intra-quadrant moves.
pub fn session_counter_attack(g: &mut GameState, action: &Action, events: &mut Vec<Event>) {
    if has_game_over(events) {
        return;
    }
    if g.current_quad.klingons.is_empty() || g.enterprise.condition == ConditionType::Docked {
        return;
    }
    let should_attack = match action {
        Action::FireTorpedo(_)
        | Action::TorpedoDirect(_)
        | Action::FirePhasers(_)
        | Action::Shields(_) => true,
        Action::Move(_) => !is_inter_quad(events),
        _ => false,
    };
    if should_attack {
        let k = klingon_turn(g);
        events.extend(k);
    }
}
