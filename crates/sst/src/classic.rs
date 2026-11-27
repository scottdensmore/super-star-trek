//! Teletype mode (port of `pkg/classic`): a small stub that prints the
//! banner and the initial energy/shield status.

use std::io::{Read, Write};

use sst_engine::gofmt::fmt_f;
use sst_engine::{Event, EventShieldTransfer, GameLength, GameState, SkillLevel};

use crate::flag::FlagSet;

/// Receives engine events and prints them as classic teletype output
/// (Go `TeletypeSubscriber`).
pub struct TeletypeSubscriber<'a> {
    w: &'a mut dyn Write,
}

impl<'a> TeletypeSubscriber<'a> {
    /// Go `NewTeletypeSubscriber`.
    pub fn new(w: &'a mut dyn Write) -> Self {
        TeletypeSubscriber { w }
    }

    /// Go `HandleEvent`.
    pub fn handle_event(&mut self, e: &Event) {
        match e {
            Event::ShieldTransfer(evt) => {
                let _ = writeln!(
                    self.w,
                    "Energy: {}  Shields: {}",
                    fmt_f(evt.new_energy, 0),
                    fmt_f(evt.new_shields, 0)
                );
            }
            Event::TorpedoFired(evt) => {
                let _ = writeln!(self.w, "Track: course {}", fmt_f(evt.angle, 2));
            }
            Event::TorpedoHit(evt) => {
                if evt.destroyed {
                    let _ = writeln!(self.w, "*** Klingon destroyed ***");
                } else {
                    let _ = writeln!(self.w, "Hit: {} units", fmt_f(evt.damage, 0));
                }
            }
            _ => {}
        }
    }
}

/// Go `RunClassicCLI(in, out, args)`: returns the exit status.
pub fn run_classic_cli(_in: &mut dyn Read, out: &mut dyn Write, args: &[String]) -> i32 {
    let mut fs = FlagSet::new("sst");
    let classic_mode = fs.bool("classic", false, "run in teletype plain mode");
    let seed = fs.int64("seed", 0, "PRNG seed (0 for random)");
    if fs.parse(args, out).is_err() {
        return 1;
    }

    if seed.get() == 0 {
        seed.set(12345);
    }

    let game = GameState::new_game(seed.get(), SkillLevel::Good, GameLength::Medium);

    if classic_mode.get() {
        let _ = writeln!(out, "Super Star Trek");
        let mut sub = TeletypeSubscriber::new(out);
        sub.handle_event(&Event::ShieldTransfer(EventShieldTransfer {
            new_shields: game.enterprise.shields,
            new_energy: game.enterprise.energy,
        }));
    } else {
        let _ = writeln!(
            out,
            "Teletype mode is off - run sst without --classic for the bridge console."
        );
    }
    0
}
