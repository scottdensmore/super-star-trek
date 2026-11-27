//! Teletype formatters for the browser terminal (port of `cmd/wasm/formatter.go`).
//!
//! Every function returns exactly the bytes Go produces: ANSI SGR codes and
//! `\r\n` line endings for xterm.js. Go's `nil` `*GameState` becomes `None`.

use std::fmt::Write as _;

use sst_engine::gofmt::{fmt_f, pad_right};
use sst_engine::{
    col, row, ConditionType, DeviceID, EntityType, EnvironmentType, Event, FactionType,
    GameOverReason, GameState, Scenario, NUM_DEVICES,
};

pub const ANSI_RESET: &str = "\x1b[0m";
pub const ANSI_GREEN: &str = "\x1b[32m";
pub const ANSI_YELLOW: &str = "\x1b[33m";
pub const ANSI_RED: &str = "\x1b[31;1m";
pub const ANSI_CYAN: &str = "\x1b[36m";
pub const ANSI_BOLD: &str = "\x1b[1m";
pub const ANSI_DIM: &str = "\x1b[2m";
pub const ANSI_MAGENTA: &str = "\x1b[35m";

/// Go `deviceNames`, indexed by `DeviceID`.
pub const DEVICE_NAMES: [&str; NUM_DEVICES] = [
    "Warp Engines",
    "Short Range Sensors",
    "Long Range Sensors",
    "Phaser Control",
    "Photon Tubes",
    "Damage Control",
    "Shield Control",
    "Library Computer",
];

fn device_name(dev: DeviceID) -> &'static str {
    let i = dev.as_i32();
    if i >= 0 && (i as usize) < DEVICE_NAMES.len() {
        DEVICE_NAMES[i as usize]
    } else {
        "Subsystem"
    }
}

/// Go `FormatSRS`: 8x8 sector grid paired with a telemetry status panel.
pub fn format_srs(g: Option<&GameState>) -> String {
    let Some(g) = g else {
        return String::new();
    };
    let mut b = String::new();
    let q = g.enterprise.quad;
    let _ = write!(
        b,
        "{ANSI_CYAN}=== SHORT RANGE SENSOR SCAN [{},{}] ==={ANSI_RESET}\r\n",
        row(q),
        col(q)
    );
    for r in 1..=8i32 {
        let _ = write!(b, "{ANSI_DIM}{r}{ANSI_RESET} ");
        for c in 1..=8i32 {
            if [r, c] == g.enterprise.sector {
                let _ = write!(b, "{ANSI_CYAN}<E>{ANSI_RESET} ");
                continue;
            }
            let cell = g.current_quad.grid[r as usize][c as usize];
            let glyph = match cell {
                EntityType::Enterprise => Some((ANSI_CYAN, "<E>")),
                EntityType::Klingon | EntityType::Commander | EntityType::SuperCommander => {
                    Some((ANSI_RED, "+K+"))
                }
                EntityType::Starbase => Some((ANSI_YELLOW, ">B<")),
                EntityType::Star => Some((ANSI_GREEN, " * ")),
                EntityType::Planet => Some((ANSI_GREEN, " O ")),
                EntityType::BlackHole => Some((ANSI_CYAN, " @ ")),
                EntityType::Wormhole => Some((ANSI_CYAN, ">W<")),
                _ => None,
            };
            match glyph {
                Some((color, s)) => {
                    let _ = write!(b, "{color}{s}{ANSI_RESET} ");
                }
                None => b.push_str(" .  "),
            }
        }
        match r {
            1 => {
                let _ = write!(b, "  Stardate:   {}", fmt_f(g.stardate, 1));
            }
            2 => {
                let (color, word) = match g.enterprise.condition {
                    ConditionType::Yellow => (ANSI_YELLOW, "YELLOW"),
                    ConditionType::Red => (ANSI_RED, "RED"),
                    ConditionType::Docked => (ANSI_CYAN, "DOCKED"),
                    _ => (ANSI_GREEN, "GREEN"),
                };
                let _ = write!(b, "  CONDITION:  {color}{word}{ANSI_RESET}");
            }
            3 => {
                let s = g.enterprise.sector;
                let _ = write!(b, "  Sector:     [{},{}]", row(s), col(s));
            }
            4 => {
                let _ = write!(b, "  Energy:     {} / 5000", fmt_f(g.enterprise.energy, 0));
            }
            5 => {
                let _ = write!(b, "  Shields:    {} / 2500", fmt_f(g.enterprise.shields, 0));
            }
            6 => {
                let _ = write!(b, "  Torpedoes:  {}", g.enterprise.torpedoes);
            }
            7 => {
                let _ = write!(b, "  Klingons:   {}", g.remaining_klingons);
            }
            _ => {
                let _ = write!(b, "  Starbases:  {}", g.remaining_starbases);
            }
        }
        b.push_str("\r\n");
    }
    b
}

/// Go `FormatLRS`: 3x3 surrounding quadrant radar scan.
pub fn format_lrs(g: Option<&GameState>) -> String {
    let Some(g) = g else {
        return String::new();
    };
    let mut b = String::new();
    let _ = write!(
        b,
        "{ANSI_CYAN}=== LONG RANGE SENSOR SCAN ==={ANSI_RESET}\r\n"
    );
    b.push_str("-------------------\r\n");
    let eq = g.enterprise.quad;
    for dr in -1..=1 {
        b.push_str(": ");
        for dc in -1..=1 {
            let qr = row(eq) + dr;
            let qc = col(eq) + dc;
            if !(1..=8).contains(&qr) || !(1..=8).contains(&qc) {
                b.push_str("*** : ");
            } else {
                let _ = write!(b, "{:03} : ", g.galaxy_chart[qr as usize][qc as usize]);
            }
        }
        b.push_str("\r\n-------------------\r\n");
    }
    b
}

/// Go `FormatChart`: the 8x8 galactic star chart.
pub fn format_chart(g: Option<&GameState>) -> String {
    let Some(g) = g else {
        return String::new();
    };
    let mut b = String::new();
    let _ = write!(b, "{ANSI_CYAN}=== GALACTIC STAR CHART ==={ANSI_RESET}\r\n");
    b.push_str("    1   2   3   4   5   6   7   8\r\n");
    b.push_str("  ---------------------------------\r\n");
    for r in 1..=8i32 {
        let _ = write!(b, "{r} |");
        for c in 1..=8i32 {
            let (ru, cu) = (r as usize, c as usize);
            if r == row(g.enterprise.quad) && c == col(g.enterprise.quad) {
                let _ = write!(b, "{ANSI_CYAN}{:03}{ANSI_RESET}|", g.galaxy_chart[ru][cu]);
            } else if g.chart_discovered[ru][cu] {
                let _ = write!(b, "{:03}|", g.galaxy_chart[ru][cu]);
            } else if g.chart_known_bases[ru][cu] {
                let _ = write!(b, "{ANSI_YELLOW}.B.{ANSI_RESET}|");
            } else {
                b.push_str("...|");
            }
        }
        b.push_str("\r\n  ---------------------------------\r\n");
    }
    b
}

/// Go `FormatDamages`: damaged subsystem statuses.
pub fn format_damages(g: Option<&GameState>) -> String {
    let Some(g) = g else {
        return String::new();
    };
    let mut b = String::new();
    let _ = write!(
        b,
        "{ANSI_CYAN}=== DAMAGE CONTROL REPORT ==={ANSI_RESET}\r\n"
    );
    for (dev, name) in DEVICE_NAMES.iter().enumerate() {
        let dmg = g.enterprise.devices[dev];
        let status = if dmg > 0.0 {
            format!(
                "{ANSI_RED}DAMAGED (Repair in {} stardates){ANSI_RESET}",
                fmt_f(dmg, 1)
            )
        } else {
            format!("{ANSI_GREEN}OPERATIONAL{ANSI_RESET}")
        };
        let _ = write!(b, "{} : {status}\r\n", pad_right(name, 22));
    }
    b
}

/// Go `FormatCombatEvents`: combat events as teletype output.
pub fn format_combat_events(events: &[Event]) -> String {
    let mut b = String::new();
    for ev in events {
        match ev {
            Event::TorpedoFired(e) => {
                let _ = write!(
                    b,
                    "{ANSI_YELLOW}[TORPEDO TRACK]{ANSI_RESET} Fired on course {} from [{},{}]\r\n",
                    fmt_f(e.angle, 2),
                    row(e.origin),
                    col(e.origin)
                );
            }
            Event::TorpedoHit(e) => {
                let (r, c) = (row(e.target), col(e.target));
                if e.destroyed {
                    let _ = write!(
                        b,
                        "{ANSI_RED}*** KLINGON WARSHIP DESTROYED AT [{r},{c}] ***{ANSI_RESET}\r\n"
                    );
                } else if e.entity == EntityType::Star {
                    let _ = write!(b, "Torpedo absorbed by star at [{r},{c}]\r\n");
                } else if e.entity == EntityType::Planet {
                    let _ = write!(b, "Torpedo absorbed by planet atmosphere at [{r},{c}]\r\n");
                } else {
                    let _ = write!(
                        b,
                        "Torpedo hit Klingon at [{r},{c}]: {} units damage\r\n",
                        fmt_f(e.damage, 0)
                    );
                }
            }
            Event::PhaserFired(e) => {
                let _ = write!(
                    b,
                    "{ANSI_YELLOW}[PHASERS FIRED]{ANSI_RESET} Allocated {} units\r\n",
                    fmt_f(e.energy, 0)
                );
            }
            Event::PhaserHit(e) => {
                if e.destroyed {
                    let ship = match e.faction {
                        FactionType::Romulan => "ROMULAN",
                        FactionType::Tholian => "THOLIAN",
                        _ => "KLINGON",
                    };
                    let _ = write!(
                        b,
                        "{ANSI_RED}*** {ship} DESTROYED BY PHASER FIRE ***{ANSI_RESET}\r\n"
                    );
                } else {
                    let _ = write!(
                        b,
                        "Phaser beam hit target at [{},{}]: {} units\r\n",
                        row(e.target),
                        col(e.target),
                        fmt_f(e.damage, 0)
                    );
                }
            }
            Event::ShieldTransfer(e) => {
                let _ = write!(
                    b,
                    "Deflector Shields: {}  |  Total energy: {}\r\n",
                    fmt_f(e.new_shields, 0),
                    fmt_f(e.new_energy, 0)
                );
            }
            Event::ShipMoved(e) => {
                if e.from_quad != e.to_quad {
                    let _ = write!(
                        b,
                        "{ANSI_GREEN}[WARP ENGINES ENGAGED]{ANSI_RESET} Arrived at quadrant [{},{}] sector [{},{}]\r\n",
                        row(e.to_quad),
                        col(e.to_quad),
                        row(e.to_sector),
                        col(e.to_sector)
                    );
                } else {
                    let _ = write!(
                        b,
                        "{ANSI_GREEN}[WARP ENGINES ENGAGED]{ANSI_RESET} Arrived at sector [{},{}]\r\n",
                        row(e.to_sector),
                        col(e.to_sector)
                    );
                }
            }
            Event::ObstacleEncountered(e) => {
                let _ = write!(
                    b,
                    "{ANSI_RED}[NAVIGATION HAZARD]{ANSI_RESET} Blocked at sector [{},{}]\r\n",
                    row(e.sector),
                    col(e.sector)
                );
            }
            Event::Docked(e) => {
                let _ = write!(
                    b,
                    "{ANSI_CYAN}[STARBASE DOCKING]{ANSI_RESET} Docked with Starbase at [{},{}]. Shields dropped, energy and torpedoes replenished.\r\n",
                    row(e.starbase),
                    col(e.starbase)
                );
            }
            Event::KlingonCounterAttack(e) => {
                let _ = write!(
                    b,
                    "{ANSI_RED}[RETURN FIRE]{ANSI_RESET} Klingon returns fire: {} units damage\r\n",
                    fmt_f(e.damage, 0)
                );
            }
            Event::SubsystemDamaged(e) => {
                let _ = write!(
                    b,
                    "{ANSI_RED}[DAMAGE]{ANSI_RESET} {} damaged! Repair in {} stardates\r\n",
                    device_name(e.device),
                    fmt_f(e.repair_time, 1)
                );
            }
            Event::SubsystemRepaired(e) => {
                let _ = write!(
                    b,
                    "{ANSI_GREEN}[REPAIR]{ANSI_RESET} {} has been repaired.\r\n",
                    device_name(e.device)
                );
            }
            Event::HazardTriggered(e) => {
                let _ = write!(
                    b,
                    "{ANSI_YELLOW}*** HAZARD: {} ***{ANSI_RESET}\r\n",
                    e.description
                );
            }
            Event::SingularityAbsorption(_) => {
                let _ = write!(
                    b,
                    "{ANSI_MAGENTA}*** GRAVITATIONAL SINGULARITY: Photon torpedo absorbed into event horizon! ***{ANSI_RESET}\r\n"
                );
            }
            Event::WormholeJump(e) => {
                let _ = write!(
                    b,
                    "{ANSI_CYAN}*** SUBSPACE RIFT: Wormhole transit completed to Quadrant [{}, {}] Sector [{}, {}]! ***{ANSI_RESET}\r\n",
                    e.to_quad[0], e.to_quad[1], e.to_sector[0], e.to_sector[1]
                );
            }
            Event::AnomalyDiscovered(e) => match e.env {
                EnvironmentType::Nebula => {
                    let _ = write!(
                        b,
                        "{ANSI_MAGENTA}*** ENVIRONMENT ALERT: Entering Mutara Nebula. Electromagnetic dispersion drops shields to 0! ***{ANSI_RESET}\r\n"
                    );
                }
                EnvironmentType::IonStorm => {
                    let _ = write!(
                        b,
                        "{ANSI_YELLOW}*** ENVIRONMENT ALERT: Entering Ion Storm! High energy particle flux detected. ***{ANSI_RESET}\r\n"
                    );
                }
                _ => {
                    let _ = write!(
                        b,
                        "{ANSI_MAGENTA}*** ENVIRONMENT ALERT: Spatial anomaly detected! ***{ANSI_RESET}\r\n"
                    );
                }
            },
            Event::GameOver(e) => {
                if e.reason == GameOverReason::Won {
                    let _ = write!(
                        b,
                        "{ANSI_GREEN}*** FEDERATION MISSION ACCOMPLISHED ***{ANSI_RESET} (Score: {})\r\n",
                        fmt_f(e.score, 0)
                    );
                } else {
                    let _ = write!(
                        b,
                        "{ANSI_RED}*** MISSION TERMINATED ***{ANSI_RESET} (Score: {})\r\n",
                        fmt_f(e.score, 0)
                    );
                }
            }
            _ => {}
        }
    }
    b
}

/// Go `FormatScenarios`: the catalog of tactical scenarios.
pub fn format_scenarios() -> String {
    let mut b = String::new();
    let _ = write!(b, "{ANSI_CYAN}=== TACTICAL SCENARIOS ==={ANSI_RESET}\r\n");
    for sc in sst_engine::list_scenarios() {
        let _ = write!(
            b,
            "  {ANSI_YELLOW}{}{ANSI_RESET} [{}] - {}\r\n",
            pad_right(&sc.id.0, 16),
            sc.difficulty,
            sc.name
        );
        let _ = write!(b, "    {ANSI_DIM}{}{ANSI_RESET}\r\n", sc.description);
    }
    b.push_str("\r\nType 'scenario <id>' to launch.\r\n");
    b
}

/// Go `FormatScenarioBriefing`: a mission briefing for terminal output.
pub fn format_scenario_briefing(sc: Option<&Scenario>) -> String {
    let Some(sc) = sc else {
        return String::new();
    };
    let mut b = String::new();
    let _ = write!(
        b,
        "{ANSI_CYAN}=== MISSION BRIEFING: {} ==={ANSI_RESET}\r\n",
        sc.name.to_uppercase()
    );
    if !sc.subtitle.is_empty() {
        let _ = write!(b, "{ANSI_YELLOW}{}{ANSI_RESET}\r\n\r\n", sc.subtitle);
    }
    for line in sc.briefing.iter() {
        let _ = write!(b, "{line}\r\n");
    }
    b.push_str("\r\n");
    b
}
