//! Byte-for-byte parity against Go reference data recorded by
//! `cmd/wasm/zz_paritygen_test.go` (testdata/parity/wasm).
//!
//! * `sessions.txt`: scripted sessions driven like the JS bridge (`sstInit`,
//!   `sstCommand`, `sstSave`, `sstLoad`, `sstReset`).
//! * `events.txt`: `FormatCombatEvents` over synthetic event lists.
//! * `states.txt`: SRS/LRS/chart/damage formatters over randomized states,
//!   plus the scenario catalog and briefings.

use std::panic::{catch_unwind, AssertUnwindSafe};

use sst_engine::*;
use sst_wasm::*;

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../testdata/parity/wasm/");

fn read(name: &str) -> Vec<u8> {
    std::fs::read(format!("{DIR}{name}")).unwrap_or_else(|e| panic!("{name}: {e}"))
}

enum Item {
    Case {
        name: String,
        reseed: bool,
        header: String,
    },
    End,
    Rec {
        op: String,
        arg: String,
        out: String,
    },
}

fn parse(data: &[u8]) -> Vec<Item> {
    let mut items = Vec::new();
    let mut pos = 0;
    while pos < data.len() {
        let nl = pos
            + data[pos..]
                .iter()
                .position(|&b| b == b'\n')
                .expect("header newline");
        let header = std::str::from_utf8(&data[pos..nl]).expect("utf8 header");
        pos = nl + 1;
        let parts: Vec<&str> = header.split(' ').collect();
        match parts[0] {
            "CASE" => items.push(Item::Case {
                name: parts[1].to_string(),
                reseed: parts[2] == "1",
                header: header.to_string(),
            }),
            "END" => items.push(Item::End),
            op => {
                let alen: usize = parts[1].parse().unwrap();
                let olen: usize = parts[2].parse().unwrap();
                let arg = String::from_utf8(data[pos..pos + alen].to_vec()).unwrap();
                let out = String::from_utf8(data[pos + alen..pos + alen + olen].to_vec()).unwrap();
                pos += alen + olen;
                assert_eq!(data[pos], b'\n', "record terminator after {header}");
                pos += 1;
                items.push(Item::Rec {
                    op: op.to_string(),
                    arg,
                    out,
                });
            }
        }
    }
    items
}

#[derive(Default)]
struct Stats {
    cases: usize,
    steps: usize,
    bytes: usize,
    stopped_at_go_panic: usize,
    rng_null_resaves: usize,
}

const RNG_NULL: &str = "{\"RNG\":null,";

#[test]
fn parity_sessions() {
    let items = parse(&read("sessions.txt"));
    let mut st = Stats::default();
    let mut session: Option<Session> = None;
    let mut last_save = String::new();
    let mut name = String::new();
    let mut skip = false;
    let mut raw_go = false;
    // SST_PARITY_RECORD=<path> re-records the transcript after an intentional
    // behaviour change: each output becomes what the session now produces
    // (unchanged outputs, Go-panic tails and the RNG:null re-saves keep their
    // recorded bytes, so only changed cases differ).
    let record = std::env::var("SST_PARITY_RECORD")
        .ok()
        .filter(|p| !p.is_empty());
    let mut rec: Vec<u8> = Vec::new();
    let push_rec = |rec: &mut Vec<u8>, op: &str, arg: &str, out: &str| {
        rec.extend_from_slice(format!("{op} {} {}\n{arg}{out}\n", arg.len(), out.len()).as_bytes());
    };
    for item in items {
        match item {
            Item::Case {
                name: n,
                reseed,
                header,
            } => {
                rec.extend_from_slice(header.as_bytes());
                rec.push(b'\n');
                name = n;
                skip = false;
                last_save.clear();
                st.cases += 1;
                // Raw-Go cases (reseed = 0) diverge only after a Go panic,
                // where the comparison stops, and in the `RNG` field of a
                // re-save (see `Session::load`).
                raw_go = !reseed;
            }
            Item::End => rec.extend_from_slice(b"END\n"),
            Item::Rec { op, arg, out } if skip => push_rec(&mut rec, &op, &arg, &out),
            Item::Rec { op, arg, out } => {
                let ctx = format!("case {name}, {op} {arg:?}");
                let got = match op.as_str() {
                    "new" => {
                        let (seed, diff) = arg.split_once(' ').unwrap_or((&arg, ""));
                        let s =
                            Session::new(seed.parse().unwrap(), Some(DifficultyProfile::new(diff)));
                        let o = format_srs(Some(s.game()));
                        session = Some(s);
                        o
                    }
                    "reset" => {
                        let s = Session::new(0, Some(DifficultyProfile::normal()));
                        let o = format_srs(Some(s.game()));
                        session = Some(s);
                        o
                    }
                    "cmd" => session.as_mut().unwrap().execute(&arg),
                    "panic" => {
                        // Go panicked here. With the PRNG-reseed deviation the
                        // Rust session must not panic; the rest of the case
                        // is Go-only behaviour.
                        let s = session.as_mut().unwrap();
                        let r = catch_unwind(AssertUnwindSafe(|| s.execute(&arg)));
                        assert!(r.is_ok(), "{ctx}: Rust panicked where Go did ({out})");
                        assert!(
                            out.contains("nil pointer dereference"),
                            "{ctx}: unexpected Go panic {out}"
                        );
                        st.stopped_at_go_panic += 1;
                        skip = true;
                        push_rec(&mut rec, &op, &arg, &out);
                        continue;
                    }
                    "save" => {
                        let o = session.as_ref().unwrap().save().unwrap_or_default();
                        last_save = o.clone();
                        o
                    }
                    "load" => {
                        let data = if arg == "@last" {
                            last_save.clone()
                        } else {
                            arg.clone()
                        };
                        match session.as_mut().unwrap().load(&data) {
                            Ok(()) => "ok".to_string(),
                            Err(_) => "err".to_string(),
                        }
                    }
                    other => panic!("unknown op {other}"),
                };
                let recorded = out.clone();
                let mut out = out;
                if raw_go && op == "save" && out.starts_with(RNG_NULL) {
                    // Deviation: a loaded state without a PRNG is reseeded,
                    // so it re-saves as `"RNG":{}` instead of Go's `null`.
                    out = format!("{{\"RNG\":{{}},{}", &out[RNG_NULL.len()..]);
                    st.rng_null_resaves += 1;
                }
                if record.is_some() {
                    let keep = if got == out { &recorded } else { &got };
                    push_rec(&mut rec, &op, &arg, keep);
                    continue;
                }
                assert!(
                    got == out,
                    "{ctx}\n--- go ---\n{out:?}\n--- rust ---\n{got:?}"
                );
                st.steps += 1;
                st.bytes += out.len();
            }
        }
    }
    if let Some(p) = &record {
        std::fs::write(p, &rec).unwrap();
        return;
    }
    eprintln!(
        "wasm session parity: {} cases, {} steps, {} bytes compared, {} cases stopped at a Go panic, {} RNG:null re-saves",
        st.cases, st.steps, st.bytes, st.stopped_at_go_panic, st.rng_null_resaves
    );
    assert!(st.cases >= 100 && st.steps >= 7000);
}

fn coord(it: &mut std::str::SplitWhitespace) -> Coord {
    [
        it.next().unwrap().parse().unwrap(),
        it.next().unwrap().parse().unwrap(),
    ]
}

fn num(it: &mut std::str::SplitWhitespace) -> f64 {
    it.next().unwrap().parse().unwrap()
}

fn int(it: &mut std::str::SplitWhitespace) -> i32 {
    it.next().unwrap().parse().unwrap()
}

/// Parses one line of the generator's event DSL.
fn event(line: &str) -> Event {
    let (kind, rest) = line.split_once(' ').unwrap_or((line, ""));
    let mut it = rest.split_whitespace();
    match kind {
        "TorpedoFired" => Event::TorpedoFired(EventTorpedoFired {
            origin: coord(&mut it),
            angle: num(&mut it),
        }),
        "TorpedoHit" => Event::TorpedoHit(EventTorpedoHit {
            target: coord(&mut it),
            entity: EntityType(int(&mut it)),
            damage: num(&mut it),
            destroyed: int(&mut it) == 1,
        }),
        "PhaserFired" => Event::PhaserFired(EventPhaserFired {
            energy: num(&mut it),
        }),
        "PhaserHit" => Event::PhaserHit(EventPhaserHit {
            target: coord(&mut it),
            damage: num(&mut it),
            destroyed: int(&mut it) == 1,
            ..Default::default()
        }),
        "ShieldTransfer" => Event::ShieldTransfer(EventShieldTransfer {
            new_shields: num(&mut it),
            new_energy: num(&mut it),
        }),
        "ShipMoved" => Event::ShipMoved(EventShipMoved {
            from_quad: coord(&mut it),
            to_quad: coord(&mut it),
            to_sector: coord(&mut it),
            warp: 1.0,
            ..Default::default()
        }),
        "ObstacleEncountered" => Event::ObstacleEncountered(EventObstacleEncountered {
            sector: coord(&mut it),
            entity: EntityType::Star,
        }),
        "Docked" => Event::Docked(EventDocked {
            starbase: coord(&mut it),
        }),
        "KlingonCounterAttack" => Event::KlingonCounterAttack(EventKlingonCounterAttack {
            enemy_id: 1,
            damage: num(&mut it),
            ..Default::default()
        }),
        "SubsystemDamaged" => Event::SubsystemDamaged(EventSubsystemDamaged {
            device: DeviceID(int(&mut it)),
            repair_time: num(&mut it),
        }),
        "SubsystemRepaired" => Event::SubsystemRepaired(EventSubsystemRepaired {
            device: DeviceID(int(&mut it)),
        }),
        "HazardTriggered" => Event::HazardTriggered(EventHazardTriggered {
            hazard_type: "x".into(),
            description: rest.to_string(),
            ..Default::default()
        }),
        "SingularityAbsorption" => Event::SingularityAbsorption(EventSingularityAbsorption {
            weapon: "torpedo".into(),
            ..Default::default()
        }),
        "WormholeJump" => Event::WormholeJump(EventWormholeJump {
            to_quad: coord(&mut it),
            to_sector: coord(&mut it),
            ..Default::default()
        }),
        "AnomalyDiscovered" => Event::AnomalyDiscovered(EventAnomalyDiscovered {
            quad: [0, 0],
            env: EnvironmentType(int(&mut it)),
        }),
        "GameOver" => Event::GameOver(EventGameOver {
            reason: GameOverReason(int(&mut it)),
            score: num(&mut it),
        }),
        "ConditionChanged" => Event::ConditionChanged(EventConditionChanged {
            from: ConditionType::Green,
            to: ConditionType::Red,
        }),
        "HelpCalled" => Event::HelpCalled(EventHelpCalled {}),
        other => panic!("unknown event {other}"),
    }
}

#[test]
fn parity_combat_events() {
    let (mut cases, mut bytes) = (0, 0);
    for item in parse(&read("events.txt")) {
        let Item::Rec { op, arg, out } = item else {
            continue;
        };
        assert_eq!(op, "events");
        let events: Vec<Event> = if arg.is_empty() {
            Vec::new()
        } else {
            arg.split('\n').map(event).collect()
        };
        let got = format_combat_events(&events);
        assert!(
            got == out,
            "events {arg:?}\n--- go ---\n{out:?}\n--- rust ---\n{got:?}"
        );
        cases += 1;
        bytes += out.len();
    }
    eprintln!("wasm event parity: {cases} event lists, {bytes} bytes compared");
    assert_eq!(cases, 600);
}

#[test]
fn parity_states_and_scenarios() {
    let (mut cases, mut bytes) = (0, 0);
    for item in parse(&read("states.txt")) {
        let Item::Rec { op, arg, out } = item else {
            continue;
        };
        let got = match op.as_str() {
            "state" => {
                let g = decode_session(&arg).expect("state json");
                [
                    format_srs(Some(&g)),
                    format_lrs(Some(&g)),
                    format_chart(Some(&g)),
                    format_damages(Some(&g)),
                ]
                .join("\0")
            }
            "scenarios" => format_scenarios(),
            "briefing" => {
                let sc = get_scenario(&ScenarioID::new(arg.clone())).expect("scenario");
                format_scenario_briefing(Some(&sc))
            }
            other => panic!("unknown op {other}"),
        };
        assert!(
            got == out,
            "{op} {}\n--- go ---\n{out:?}\n--- rust ---\n{got:?}",
            &arg[..arg.len().min(80)]
        );
        cases += 1;
        bytes += out.len();
    }
    eprintln!("wasm formatter parity: {cases} states/scenario records, {bytes} bytes compared");
    assert!(cases >= 100);
}
