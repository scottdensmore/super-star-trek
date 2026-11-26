//! Maps engine events to sounds (port of `pkg/audio/dispatcher.go`).

use std::rc::Rc;

use sst_engine::{ConditionType, Event, GameOverReason};

use crate::sound::*;

/// Plays the sound associated with each engine event (Go `Dispatcher`).
pub struct Dispatcher {
    player: PlayerRef,
}

impl Dispatcher {
    /// Go `NewDispatcher`; `None` uses a [`NullPlayer`].
    pub fn new(player: Option<PlayerRef>) -> Self {
        Dispatcher {
            player: player.unwrap_or_else(|| Rc::new(NullPlayer::new())),
        }
    }

    /// The player sounds are sent to.
    pub fn player(&self) -> &PlayerRef {
        &self.player
    }

    /// Go `DispatchEvents`: does nothing when the player is muted.
    pub fn dispatch_events(&self, events: &[Event]) {
        if self.player.is_muted() {
            return;
        }
        for event in events {
            if let Some(s) = sound_for_event(event) {
                self.player.play(s);
            }
        }
    }
}

/// The sound the dispatcher plays for `event`, if any.
pub fn sound_for_event(event: &Event) -> Option<SoundID> {
    Some(match event {
        Event::PhaserFired(_) => SOUND_PHASER,
        Event::TorpedoFired(_) => SOUND_TORPEDO_LAUNCH,
        Event::PhaserHit(e) if e.destroyed || e.damage > 0.0 => SOUND_EXPLOSION,
        Event::TorpedoHit(e) if e.destroyed || e.damage > 0.0 => SOUND_EXPLOSION,
        Event::KlingonCounterAttack(e) if e.damage > 0.0 => SOUND_DAMAGE,
        Event::SubsystemDamaged(_) => SOUND_DAMAGE,
        Event::ShieldTransfer(_) => SOUND_SHIELDS,
        Event::Docked(_) => SOUND_DOCK,
        Event::ConditionChanged(e) if e.to == ConditionType::Red => SOUND_RED_ALERT,
        Event::AnomalyDiscovered(_) => SOUND_RED_ALERT,
        Event::WormholeJump(_) | Event::ShipMoved(_) => SOUND_WARP,
        Event::GameOver(e) => {
            if e.reason == GameOverReason::Won {
                SOUND_VICTORY
            } else {
                SOUND_DEFEAT
            }
        }
        Event::KlingonCloakState(e) => {
            if e.cloaked {
                SOUND_CLOAK
            } else {
                SOUND_DECLOAK
            }
        }
        Event::RomulanDecloak(_) => SOUND_DECLOAK,
        Event::PlasmaLaunched(_) => SOUND_PLASMA_LAUNCH,
        Event::PlasmaImpact(_) => SOUND_PLASMA_IMPACT,
        Event::PlasmaIntercepted(_) => SOUND_POINT_DEFENSE,
        Event::WebSegmentLaid(_) => SOUND_THOLIAN_WEB,
        Event::WebBreached(_) | Event::WebCollapsed(_) => SOUND_WEB_BREACHED,
        Event::HelpCalled(_) | Event::StarbaseSurveillance(_) => SOUND_COMM_CHIME,
        Event::LRScanCompleted(_) => SOUND_COMPUTER_BEEP,
        _ => return None,
    })
}
