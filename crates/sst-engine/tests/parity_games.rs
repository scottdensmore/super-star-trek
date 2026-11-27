//! Parity: hundreds of scripted games driven like pkg/tui/update.go (TUI)
//! and cmd/wasm/session.go (WASM session), compared step by step with the
//! Go transcripts (tools/paritygen/engine/games.go).

mod common;
use common::*;

use sst_engine::json::{self, GoVec, Value};
use sst_engine::parser::{fields, parse_command, trim_space};
use sst_engine::*;

struct Sim<'a, 'b> {
    mode: String,
    g: GameRef,
    tour: Option<TourRef>,
    lb: Leaderboard,
    seed: i64,
    over: bool,
    l: &'a mut Lines<'b>,
    steps: usize,
}

fn refit_ids() -> Vec<RefitID> {
    let mut v = vec![RefitID::new("bogus")];
    v.extend(refit_catalog().iter().map(|d| d.id.clone()));
    v
}

fn game_over(events: &[Event]) -> Option<EventGameOver> {
    events.iter().find_map(|e| match e {
        Event::GameOver(g) => Some(g.clone()),
        _ => None,
    })
}

impl Sim<'_, '_> {
    /// P/E/X lines are either full (`P {...}`) or hashed (`P# <fnv>`).
    fn detail(&mut self, tag: &str, payload: &str) {
        if self.l.recording() {
            let full = format!("{tag} ");
            let keep_full = self.l.peek().is_some_and(|x| x.starts_with(&full));
            let hashed = self
                .l
                .peek()
                .is_some_and(|x| x.starts_with(&format!("{tag}# ")));
            if keep_full || hashed {
                self.l.pos += 1;
            }
            // SST_PARITY_RECORD_FULL=1 writes every payload in full (for
            // inspecting a re-recorded case; do not commit that output).
            let keep_full = keep_full || std::env::var_os("SST_PARITY_RECORD_FULL").is_some();
            let line = if keep_full {
                format!("{tag} {payload}")
            } else {
                format!("{tag}# {}", fnv_hex(payload.as_bytes()))
            };
            self.l.record(line);
            return;
        }
        let line = self.l.next().expect("detail line");
        if let Some(h) = line.strip_prefix(&format!("{tag}# ")) {
            self.l.bytes_compared += payload.len();
            assert_eq!(
                h,
                fnv_hex(payload.as_bytes()),
                "line {}: {tag} hash, got payload {payload}",
                self.l.pos
            );
        } else if let Some(full) = line.strip_prefix(&format!("{tag} ")) {
            self.l.bytes_compared += full.len();
            assert_same(&format!("line {} {tag}", self.l.pos), full, payload);
        } else {
            panic!("line {}: wanted {tag}, got {line}", self.l.pos);
        }
    }

    fn evaluate_tour(&mut self) {
        let Some(t) = self.tour.clone() else { return };
        if t.borrow().in_drydock || !t.borrow().active {
            return;
        }
        t.borrow_mut().current_game_state = Some(self.g.clone());
        let (cleared, failed, bounty) = t.borrow_mut().evaluate_sector();
        if cleared || failed {
            self.l.check("T ", &format!("{cleared} {failed} {bounty}"));
        }
        if cleared {
            t.borrow_mut().advance_to_drydock(bounty);
            for i in 0..4 {
                // The Go harness drew the refit from its command RNG; take it from the transcript.
                let line = self.l.peek().unwrap();
                let id = if self.l.recording() && !line.starts_with("B ") {
                    // A re-recorded case clears a sector where Go did not:
                    // pick a refit deterministically.
                    let ids = refit_ids();
                    ids[(self.steps + i) % ids.len()].clone()
                } else {
                    RefitID::new(line[2..].split(' ').next().unwrap())
                };
                assert!(refit_ids().contains(&id));
                let res = purchase_refit(Some(&mut t.borrow_mut()), &id);
                let es = match res {
                    Ok(()) => "<nil>".to_string(),
                    Err(e) => e.0,
                };
                self.l.check("B ", &format!("{id} {es}"));
            }
            let res = t.borrow_mut().disembark_to_next_sector();
            let (es, some) = match &res {
                Ok(o) => ("<nil>".to_string(), o.is_some()),
                Err(e) => (e.0.clone(), false),
            };
            self.l.check("D ", &format!("{es} {some}"));
            match res {
                Ok(Some(ng)) => {
                    if ng.borrow().current_quad.enemies.is_empty() {
                        let (q, s) = (ng.borrow().enterprise.quad, ng.borrow().enterprise.sector);
                        ng.borrow_mut().populate_quadrant(q, s);
                    }
                    self.g = ng;
                    self.l.check_full(&self.g.borrow());
                }
                _ => {
                    if t.borrow().completed {
                        self.lb.record_tour_at(
                            &t.borrow(),
                            "Enterprise",
                            gotime::GoTime::default(),
                        );
                        let n = self.lb.tour_records.len();
                        self.lb.tour_records[n - 1].date = String::new();
                        self.l.check("C ", &marshal(&self.lb));
                        self.over = true;
                    }
                }
            }
            self.l.check("TS ", &marshal_go_tour(&t));
            return;
        }
        if failed {
            self.lb
                .record_tour_at(&t.borrow(), "Enterprise", gotime::GoTime::default());
            let n = self.lb.tour_records.len();
            self.lb.tour_records[n - 1].date = String::new();
            self.l.check("F ", &marshal(&self.lb));
            self.l.check("TS ", &marshal_go_tour(&t));
            self.over = true;
        }
    }

    fn handle_game_over(&mut self, ev: &EventGameOver) {
        {
            let mut g = self.g.borrow_mut();
            g.game_over = true;
            g.game_over_reason = ev.reason;
            if ev.reason == GameOverReason::Won {
                g.game_won = true;
            }
        }
        let sb = {
            let g = self.g.borrow();
            compute_score(Some(&g), g.game_won)
        };
        self.l
            .check("G ", &format!("{} {}", ev.reason.as_i32(), marshal(&sb)));
        let active = self
            .tour
            .as_ref()
            .is_some_and(|t| !t.borrow().in_drydock && t.borrow().active);
        if active {
            self.evaluate_tour();
        }
        match &self.tour {
            None => self.over = true,
            Some(t) => {
                if !self.over && !t.borrow().in_drydock && !t.borrow().active {
                    self.over = true;
                }
            }
        }
    }

    fn step_tui(&mut self, input: &str) {
        let trimmed = trim_space(input).to_lowercase();
        if trimmed == "lrscan" {
            let res = self.g.borrow_mut().dispatch(&Action::LRScan(ActionLRScan));
            match res {
                Err(e) => self.detail("X", &e.0),
                Ok(ev) => self.detail("E", &events_to_json(&ev)),
            }
        } else {
            let parsed = parse_command(input);
            self.detail("P", &parsed.to_json());
            if parsed.error.is_none() && parsed.special.is_empty() {
                if let Some(action) = &parsed.action {
                    let res = self.g.borrow_mut().dispatch(action);
                    match res {
                        Err(e) => self.detail("X", &e.0),
                        Ok(mut events) => {
                            execute_klingon_turn_if_active(
                                &mut self.g.borrow_mut(),
                                action,
                                &mut events,
                            );
                            self.detail("E", &events_to_json(&events));
                            if let Some(ev) = game_over(&events) {
                                self.handle_game_over(&ev);
                            }
                        }
                    }
                }
            }
        }
        if !self.over {
            self.evaluate_tour();
        }
    }

    /// Engine-level operations from `extra()` in the Go harness.
    fn extra(&mut self, line: &str) {
        let (tag, rest) = line.split_once(' ').unwrap();
        match tag {
            "M" => {
                let n: Vec<i32> = rest.split(' ').map(|x| x.parse().unwrap()).collect();
                self.g.borrow_mut().current_quad.grid[n[1] as usize][n[2] as usize] =
                    EntityType::from_i32(n[0]);
            }
            "A" => {
                let (ty, js) = rest.split_once(' ').unwrap();
                let action: Action = match ty {
                    "engine.ActionTorpedoDirect" => {
                        let mut a = ActionTorpedoDirect::default();
                        json::unmarshal(js.as_bytes(), &mut a).unwrap();
                        a.into()
                    }
                    "engine.ActionCallHelp" => ActionCallHelp.into(),
                    "engine.ActionKlingonCounterAttack" => {
                        let mut a = ActionKlingonCounterAttack::default();
                        json::unmarshal(js.as_bytes(), &mut a).unwrap();
                        a.into()
                    }
                    _ => panic!("action {ty}"),
                };
                assert_same("action json", js, &marshal_action(&action));
                let res = self.g.borrow_mut().dispatch(&action);
                match res {
                    Err(e) => self.detail("X", &e.0),
                    Ok(mut events) => {
                        execute_klingon_turn_if_active(
                            &mut self.g.borrow_mut(),
                            &action,
                            &mut events,
                        );
                        self.detail("E", &events_to_json(&events));
                        if let Some(ev) = game_over(&events) {
                            self.handle_game_over(&ev);
                        }
                    }
                }
                if !self.over {
                    self.evaluate_tour();
                }
            }
            "V" => {
                let (el, devs) = rest.split_once(' ').unwrap();
                let mut d = [0f64; NUM_DEVICES];
                json::unmarshal(devs.as_bytes(), &mut d).unwrap();
                let mut g = self.g.borrow_mut();
                g.enterprise.devices = d;
                g.advance_turn(el.parse().unwrap());
            }
            "MK" => {
                let n: Vec<i32> = rest.split(' ').map(|x| x.parse().unwrap()).collect();
                // A re-recorded case may have lost the Klingon the Go harness
                // picked (destroyed earlier than in Go): the op is a no-op.
                let k = self
                    .g
                    .borrow()
                    .current_quad
                    .klingons
                    .get(n[0] as usize)
                    .cloned()
                    .flatten();
                let ev = match k {
                    Some(k) => move_klingon(&mut self.g.borrow_mut(), &k, [n[1], n[2]]),
                    None => Vec::new(),
                };
                self.detail("E", &events_to_json(&ev));
            }
            _ => panic!("extra {line}"),
        }
    }

    fn step_wasm(&mut self, input: &str) {
        let trimmed = trim_space(input);
        if trimmed.is_empty() {
            return;
        }
        let tokens = fields(trimmed);
        let cmd = tokens[0].to_lowercase();
        match cmd.as_str() {
            "help" | "?" | "srs" | "srscan" | "status" | "lrs" | "lrscan" | "chart" | "dam"
            | "damages" | "quit" | "exit" | "q" => {
                self.l.check("W ", "bypass");
                return;
            }
            "scenario" => {
                if tokens.len() == 1 || tokens[1].to_lowercase() == "list" {
                    self.l.check("W ", "list");
                    return;
                }
                let id = if tokens.len() > 2 {
                    tokens[1..].join("-")
                } else {
                    tokens[1].to_string()
                };
                match get_scenario(&ScenarioID::new(id.clone())) {
                    None => self.l.check("W ", &format!("unknown {id}")),
                    Some(sc) => {
                        self.g = game_ref(sc.build(self.seed));
                        self.l.check("W ", &format!("build {}", sc.id));
                    }
                }
                return;
            }
            _ => {}
        }
        let parsed = parse_command(trimmed);
        self.detail("P", &parsed.to_json());
        if parsed.error.is_some() || !parsed.special.is_empty() {
            return;
        }
        let Some(action) = &parsed.action else { return };
        let res = self.g.borrow_mut().dispatch(action);
        match res {
            Err(e) => self.detail("X", &e.0),
            Ok(mut events) => {
                session_counter_attack(&mut self.g.borrow_mut(), action, &mut events);
                self.detail("E", &events_to_json(&events));
                if game_over(&events).is_some() {
                    self.over = true;
                }
            }
        }
    }
}

fn marshal_action(a: &Action) -> String {
    let j = a.to_json();
    // strip {"Type":"...","Data": ... }
    let i = j.find("\"Data\":").unwrap() + 7;
    j[i..j.len() - 1].to_string()
}

fn get<'v>(v: &'v Value, k: &str) -> &'v Value {
    v.get(k).unwrap_or_else(|| panic!("missing {k}"))
}

fn as_i64(v: &Value) -> i64 {
    match v {
        Value::Number(s) => s.parse().unwrap(),
        _ => panic!("not a number"),
    }
}

fn as_str(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        _ => panic!("not a string"),
    }
}

#[test]
fn scripted_games_match_go() {
    let d = data("games.txt");
    let mut l = Lines::new(&d).inputs(&["game ", "> ", "M ", "A ", "V ", "MK ", "end", "S "]);
    let (mut games, mut commands, mut overs, mut extras) = (0, 0, 0, 0);
    let mut kinds = std::collections::BTreeMap::new();
    while let Some(header) = l.next() {
        let rest = header.strip_prefix("game ").expect("game header");
        let mut it = rest.splitn(4, ' ');
        let _n: u64 = it.next().unwrap().parse().unwrap();
        let mode = it.next().unwrap().to_string();
        let kind = it.next().unwrap().to_string();
        let params = json::parse(it.next().unwrap().as_bytes()).unwrap();
        *kinds.entry(format!("{mode}/{kind}")).or_insert(0) += 1;
        let seed = as_i64(get(&params, "seed"));
        let mut tour = None;
        let (g, seed) = match kind.as_str() {
            "game" => {
                let mut rules = GameRules::default();
                let mut dec = json::Decoder::new("engine.GameRules");
                json::GoJson::decode_json(&mut rules, get(&params, "rules"), &mut dec);
                let mut g = GameState::new_game_with_options(
                    seed,
                    SkillLevel::Good,
                    GameLength::Medium,
                    rules,
                );
                let (q, s) = (g.enterprise.quad, g.enterprise.sector);
                g.populate_quadrant(q, s);
                (game_ref(g), seed)
            }
            "scenario" => {
                let sc = get_scenario(&ScenarioID::new(as_str(get(&params, "id")))).unwrap();
                (game_ref(sc.build(seed)), seed)
            }
            "tour" => {
                let t = tour_ref(TourState::new_at(seed, 0));
                let g = t.borrow_mut().start_current_sector(None).unwrap();
                if g.borrow().current_quad.enemies.is_empty() {
                    let (q, s) = (g.borrow().enterprise.quad, g.borrow().enterprise.sector);
                    g.borrow_mut().populate_quadrant(q, s);
                }
                tour = Some(t);
                (g, seed)
            }
            "session" => {
                let seed = if seed == 0 { 12345 } else { seed };
                let diff = DifficultyProfile::new(as_str(get(&params, "difficulty")));
                let mut g = GameState::new_game_with_options(
                    seed,
                    SkillLevel::Good,
                    GameLength::Medium,
                    default_rules_for_profile(&diff),
                );
                let (q, s) = (g.enterprise.quad, g.enterprise.sector);
                g.populate_quadrant(q, s);
                (game_ref(g), seed)
            }
            _ => panic!("kind {kind}"),
        };
        if l.peek().is_some_and(|x| x.starts_with("S ")) {
            l.check_full(&g.borrow());
        } else {
            l.check("H ", &state_hash(&g.borrow()));
        }
        let mut sim = Sim {
            mode: mode.clone(),
            g,
            tour,
            lb: Leaderboard::new(),
            seed,
            over: false,
            l: &mut l,
            steps: 0,
        };
        loop {
            let line = sim.l.next().unwrap();
            if line == "end" {
                break;
            }
            if line.starts_with("S ") {
                sim.l.unread();
                let g = sim.g.clone();
                sim.l.check_full(&g.borrow());
                continue;
            }
            if ["M ", "A ", "V ", "MK "]
                .iter()
                .any(|p| line.starts_with(p))
            {
                sim.extra(line);
                let h = state_hash(&sim.g.borrow());
                sim.l.check("H ", &h);
                extras += 1;
                continue;
            }
            let input = hex_str(
                line.strip_prefix("> ")
                    .unwrap_or_else(|| panic!("line {}: {line}", sim.l.pos)),
            );
            if sim.mode == "tui" {
                sim.step_tui(&input);
            } else {
                sim.step_wasm(&input);
            }
            let h = state_hash(&sim.g.borrow());
            sim.l.check("H ", &h);
            sim.steps += 1;
        }
        commands += sim.steps;
        if sim.over {
            overs += 1;
        }
        let g = sim.g.clone();
        sim.l.check_full(&g.borrow());
        games += 1;
        let _ = GoVec::<i32>::nil();
    }
    eprintln!(
        "game parity: {games} games ({kinds:?}), {commands} commands + {extras} engine-level ops, {overs} reached game over/tour end, {} bytes compared",
        l.bytes_compared
    );
    assert!(games >= 250);
}
