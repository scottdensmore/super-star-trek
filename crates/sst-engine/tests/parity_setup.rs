//! Parity: game creation, quadrant population, scenarios, tours, scoring
//! and leaderboards against Go transcripts.

mod common;
use common::*;

use sst_engine::json::{self, GoVec};
use sst_engine::*;

#[derive(Debug, Clone, Default)]
struct GameCfg {
    seed: i64,
    skill: SkillLevel,
    length: GameLength,
    rules: GameRules,
}

sst_engine::go_json_struct!(GameCfg, "gameCfg", "main.gameCfg" {
    seed => "Seed",
    skill => "Skill",
    length => "Length",
    rules => "Rules",
});

#[test]
fn new_game_and_populate_quadrant_match_go() {
    let d = data("newgame.txt");
    let mut l = Lines::new(&d);
    let mut g = GameState::default();
    let (mut cfgs, mut pops) = (0, 0);
    while let Some(line) = l.next() {
        let (kind, rest) = line.split_once(' ').unwrap();
        match kind {
            "cfg" => {
                let mut c = GameCfg::default();
                json::unmarshal(rest.as_bytes(), &mut c).unwrap();
                g = GameState::new_game_with_options(c.seed, c.skill, c.length, c.rules);
                l.check_full(&g);
                cfgs += 1;
            }
            "q" => {
                let n: Vec<i32> = rest
                    .split(' ')
                    .take(4)
                    .map(|x| x.parse().unwrap())
                    .collect();
                g.populate_quadrant([n[0], n[1]], [n[2], n[3]]);
                let h = rest.split(' ').nth(4).unwrap();
                assert_eq!(state_hash(&g), h, "populate {rest}");
                if l.peek().is_some_and(|p| p.starts_with("S ")) {
                    l.check_full(&g);
                }
                pops += 1;
            }
            "newgame" => {
                let s: i64 = rest.parse().unwrap();
                l.check_full(&GameState::new_game(s, SkillLevel::Fair, GameLength::Long));
            }
            "newgamewithseed" => {
                let s: i64 = rest.parse().unwrap();
                l.check_full(&GameState::new_game_with_seed(s));
            }
            _ => panic!("bad line {line}"),
        }
    }
    eprintln!(
        "newgame parity: {cfgs} configs, {pops} quadrant populations, {} bytes",
        l.bytes_compared
    );
}

fn won_reason(s: &Scenario, g: &mut GameState) -> String {
    let (done, won, reason) = (s.evaluate.unwrap())(g);
    let sb = (s.compute_score.unwrap())(g, won);
    format!(
        "{} {} {} {} {}",
        done,
        won,
        reason.as_i32(),
        marshal(&sb),
        state_hash(g)
    )
}

#[test]
fn scenarios_match_go() {
    let d = data("scenarios.txt");
    let mut l = Lines::new(&d);
    let mut g = GameState::default();
    let mut current: Option<std::sync::Arc<Scenario>> = None;
    let mut builds = 0;
    while let Some(line) = l.next() {
        let (kind, rest) = line.split_once(' ').unwrap();
        match kind {
            "meta" => {
                let id = rest
                    .split("\"id\":\"")
                    .nth(1)
                    .unwrap()
                    .split('"')
                    .next()
                    .unwrap();
                let s = get_scenario(&ScenarioID::new(id)).unwrap();
                assert_same("meta", rest, &marshal(&*s));
            }
            "get" => {
                // get "<q>" ok id
                let (q, tail) = parse_quoted(rest);
                let mut it = tail.split(' ');
                let ok = it.next().unwrap();
                let id = it.next().unwrap_or("");
                let r = get_scenario(&ScenarioID::new(q.clone()));
                assert_eq!(r.is_some().to_string(), ok, "get {q:?}");
                assert_eq!(r.map(|s| s.id.0.clone()).unwrap_or_default(), id);
            }
            "build" => {
                let mut it = rest.split(' ');
                let id = it.next().unwrap();
                let seed: i64 = it.next().unwrap().parse().unwrap();
                let s = get_scenario(&ScenarioID::new(id)).unwrap();
                g = s.build(seed);
                l.check_full(&g);
                current = Some(s);
                builds += 1;
            }
            "eval" => {
                let s = current.clone().unwrap();
                let (label, want) = rest.split_once(' ').unwrap();
                match label {
                    "initial" => {}
                    "kill" => {
                        let k = g.current_quad.klingons[0].clone().unwrap();
                        let sec = k.borrow().sector;
                        g.current_quad.grid[sec[0] as usize][sec[1] as usize] = EntityType::Empty;
                        g.current_quad.klingons.vec_mut().remove(0);
                        g.metrics.klingons_killed += 1;
                        g.stardate += 1.25;
                    }
                    "moved" => g.enterprise.quad = [3, 3],
                    "timeout" => {
                        g.enterprise.quad = [4, 4];
                        g.time_remaining = 0.0;
                    }
                    "energy" => {
                        g.time_remaining = 5.0;
                        g.enterprise.energy = 0.0;
                    }
                    "starbase" => {
                        g.enterprise.energy = 100.0;
                        g.metrics.starbases_destroyed = 1;
                    }
                    _ => panic!("label {label}"),
                }
                assert_same(&format!("eval {label}"), want, &won_reason(&s, &mut g));
            }
            "score" => {
                let s = current.clone().unwrap();
                let (won, want) = rest.split_once(' ').unwrap();
                let sb = (s.compute_score.unwrap())(&g, won == "true");
                assert_same("score", want, &marshal(&sb));
            }
            _ => panic!("bad line {line}"),
        }
    }
    eprintln!(
        "scenario parity: {builds} builds, {} bytes",
        l.bytes_compared
    );
}

/// Parses a Go `%q` prefix, returning (unquoted, rest after space).
fn parse_quoted(s: &str) -> (String, &str) {
    let b = s.as_bytes();
    assert_eq!(b[0], b'"');
    let mut i = 1;
    let mut out = String::new();
    while b[i] != b'"' {
        if b[i] == b'\\' {
            i += 1;
        }
        out.push(b[i] as char);
        i += 1;
    }
    let rest = s[i + 1..].strip_prefix(' ').unwrap_or("");
    (out, rest)
}

struct PcgRand(sst_engine::prng::Rand);

impl PcgRand {
    fn new(a: u64, b: u64) -> Self {
        PcgRand(sst_engine::prng::Rand::new(sst_engine::prng::Pcg::new(
            a, b,
        )))
    }
    fn intn(&mut self, n: i64) -> i64 {
        self.0.int_n(n)
    }
    fn float64(&mut self) -> f64 {
        self.0.float64()
    }
}

fn refit_ids() -> Vec<RefitID> {
    let mut v = vec![RefitID::new("bogus")];
    v.extend(refit_catalog().iter().map(|d| d.id.clone()));
    v
}

fn err_str(r: Result<(), EngineError>) -> String {
    match r {
        Ok(()) => "<nil>".into(),
        Err(e) => e.0,
    }
}

#[test]
fn tours_match_go() {
    let d = data("tours.txt");
    let mut l = Lines::new(&d);
    l.check(
        "catalog ",
        &marshal(&GoVec::from_vec(refit_catalog().to_vec())),
    );
    for id in refit_ids() {
        let want = l.expect(&format!("def {} ", id));
        match get_refit_definition(&id) {
            Some(def) => assert_same("def", want, &marshal(def)),
            None => assert_eq!(want, "null"),
        }
    }
    let mut r = PcgRand::new(7, 8);
    let mut tours = 0;
    for (ti, seed) in [1i64, 42, 1000, 12345, 777, 5, 31337, 9]
        .into_iter()
        .enumerate()
    {
        let t = tour_ref(TourState::new_at(seed, 0));
        l.check(&format!("tour {seed} "), &marshal_go_tour(&t));
        let gr = t.borrow_mut().start_current_sector(None);
        l.check("start ", "<nil>");
        let mut g = gr.unwrap();
        l.check_full(&g.borrow());
        let mut step = 0;
        while step < 12 && t.borrow().active && !t.borrow().completed {
            let (c, f, b) = t.borrow_mut().evaluate_sector();
            l.check("evaluate ", &format!("{c} {f} {b}"));
            match r.intn(5) {
                0 => g.borrow_mut().klingons_remaining = 0,
                1 => {
                    let mut gb = g.borrow_mut();
                    gb.remaining_klingons = 0;
                    gb.enterprise.devices[DeviceID::Shields.index()] = 1.5;
                }
                2 => {
                    let mut gb = g.borrow_mut();
                    gb.current_quad.enemies = GoVec::nil();
                    gb.current_quad.klingons = GoVec::nil();
                    gb.days_remaining = r.float64() * 40.0;
                }
                3 => {
                    let mut gb = g.borrow_mut();
                    gb.score = r.intn(900) as i32;
                    gb.enterprise.damage[DeviceID::Warp.index()] = 2.0;
                }
                _ => {
                    if ti % 3 == 2 && step > 3 {
                        let reason = GameOverReason::from_i32(1 + r.intn(5) as i32);
                        let mut gb = g.borrow_mut();
                        gb.game_over = true;
                        gb.game_over_reason = reason;
                    }
                }
            }
            let (c, f, b) = t.borrow_mut().evaluate_sector();
            l.check("evaluate ", &format!("{c} {f} {b}"));
            if c {
                t.borrow_mut().advance_to_drydock(b);
                for _ in 0..6 {
                    let ids = refit_ids();
                    let id = ids[r.intn(ids.len() as i64) as usize].clone();
                    let res = purchase_refit(Some(&mut t.borrow_mut()), &id);
                    l.check("purchase ", &format!("{} {}", id, err_str(res)));
                }
                let res = t.borrow_mut().disembark_to_next_sector();
                let (es, some) = match &res {
                    Ok(o) => ("<nil>".to_string(), o.is_some()),
                    Err(e) => (e.0.clone(), false),
                };
                l.check("disembark ", &format!("{es} {some}"));
                if let Ok(Some(ng)) = res {
                    if ng.borrow().current_quad.enemies.is_empty() {
                        let (q, s) = (ng.borrow().enterprise.quad, ng.borrow().enterprise.sector);
                        ng.borrow_mut().populate_quadrant(q, s);
                    }
                    g = ng;
                    l.check_full(&g.borrow());
                }
            }
            let res = t.borrow_mut().disembark_to_next_sector();
            l.check(
                "disembark-again ",
                &match res {
                    Ok(_) => "<nil>".to_string(),
                    Err(e) => e.0,
                },
            );
            l.check("tourstate ", &fnv_hex(marshal_go_tour(&t).as_bytes()));
            step += 1;
        }
        let (rank, medals) = calculate_tour_commission(Some(&t.borrow()));
        l.check("commission ", &format!("{} {}", rank, marshal(&medals)));
        l.check("final ", &marshal_go_tour(&t));
        let mut lb = Leaderboard::new();
        let mut rec = lb.record_tour_at(&t.borrow(), ["", "Excelsior"][ti % 2], GoTime::default());
        rec.date = "FIXED".into();
        lb.tour_records[0].date = "FIXED".into();
        l.check("record ", &format!("{} {}", marshal(&rec), marshal(&lb)));
        tours += 1;
    }
    let t = tour_ref(TourState::new_at(55, 0));
    for idx in [3, 2, 1, 0, 4, -1] {
        let res = t.borrow_mut().start_current_sector(Some(idx));
        let es = match &res {
            Ok(_) => "<nil>".to_string(),
            Err(e) => e.0.clone(),
        };
        l.check(&format!("startidx {idx} "), &es);
        if let Ok(g) = res {
            l.check_full(&g.borrow());
        }
    }
    t.borrow_mut().installed_refits = None;
    t.borrow_mut().requisition_points = 5000;
    for id in refit_ids() {
        let res = purchase_refit(Some(&mut t.borrow_mut()), &id);
        l.check(
            "purchase ",
            &format!("{} {} {}", id, err_str(res), t.borrow().requisition_points),
        );
    }
    l.check(
        "purchase-nil ",
        &err_str(purchase_refit(None, &RefitID::new("x"))),
    );
    for id in refit_ids() {
        for _ in 0..3 {
            let _ = purchase_refit(Some(&mut t.borrow_mut()), &id);
        }
    }
    l.check("maxed ", &marshal_go_tour(&t));
    let g = t.borrow_mut().start_current_sector(Some(2)).unwrap();
    l.check_full(&g.borrow());
    l.check(
        "torpdmg ",
        &format!(
            "{} {}",
            gofmt::fmt_v(calculate_torpedo_damage(Some(&g.borrow()), 500.0)),
            gofmt::fmt_v(calculate_shield_damage_absorption(Some(&g.borrow()), 300.0))
        ),
    );
    let (rank, medals) = calculate_tour_commission(None);
    l.check("commission-nil ", &format!("{} {}", rank, marshal(&medals)));
    for sc in 0..=5 {
        t.borrow_mut().sectors_completed = sc;
        let (rank, medals) = calculate_tour_commission(Some(&t.borrow()));
        l.check(
            &format!("commission {sc} "),
            &format!("{} {}", rank, marshal(&medals)),
        );
    }
    assert!(l.peek().is_none());
    eprintln!("tour parity: {tours} tours, {} bytes", l.bytes_compared);
}

use sst_engine::gofmt;
use sst_engine::gotime::GoTime;

#[test]
fn score_and_leaderboard_match_go() {
    let d = data("score.txt");
    let mut l = Lines::new(&d);
    for s in [
        -1000, -1, 0, 1, 99, 100, 199, 200, 349, 350, 499, 500, 749, 750, 999, 1000, 5000,
    ] {
        let (t, b) = rank_for_score(s);
        l.check(&format!("rank {s} "), &format!("{t} {b}"));
    }
    let mut r = PcgRand::new(9, 10);
    for i in 0..80 {
        let mut g = GameState::new_game(
            i,
            SkillLevel::from_i32(r.intn(6) as i32),
            GameLength::Medium,
        );
        g.stardate = g.initial_stardate + (r.float64() - 0.2) * 20.0;
        let m = &mut g.metrics;
        m.klingons_killed = r.intn(20) as i32;
        m.commanders_killed = r.intn(4) as i32;
        m.super_commanders_killed = r.intn(2) as i32;
        m.romulans_killed = r.intn(3) as i32;
        m.romulans_surrendered = r.intn(3) as i32;
        m.starbases_destroyed = r.intn(2) as i32;
        m.stars_destroyed = r.intn(3) as i32;
        m.planets_destroyed = r.intn(2) as i32;
        m.casualties = r.intn(300) as i32;
        m.help_calls = r.intn(3) as i32;
        m.starships_lost = r.intn(2) as i32;
        let won = r.intn(2) == 0;
        l.check(
            "score ",
            &format!(
                "{} {} {}",
                marshal(&g.metrics),
                won,
                marshal(&compute_score(Some(&g), won))
            ),
        );
        l.check(
            "scoregame ",
            &format!(
                "{} {} {} {}",
                gofmt::fmt_v(g.stardate),
                gofmt::fmt_v(g.initial_stardate),
                g.skill.as_i32(),
                marshal(&compute_score(Some(&g), !won))
            ),
        );
    }
    l.check("score-nil ", &marshal(&compute_score(None, true)));
    let mut lb = Leaderboard::default_board();
    l.check("default ", &marshal(&lb));
    l.check("new ", &marshal(&Leaderboard::new()));
    for i in 0..40 {
        let sc = r.intn(1500) as i32 - 100;
        let q = lb.qualifies(sc);
        let mut e = ScoreEntry {
            captain_name: format!("Cap{i}"),
            score: sc,
            rank: "[X]".into(),
            stardate: i as f64 * 1.5,
            ..Default::default()
        };
        if i % 3 == 0 {
            e.skill = "Good".into();
            e.game_won = true;
        }
        lb.add(e);
        l.check(
            "add ",
            &format!("{sc} {q} {}", fnv_hex(marshal(&lb).as_bytes())),
        );
    }
    l.check("after ", &marshal(&lb));
    l.check(
        "qualifies-empty ",
        &Leaderboard::new().qualifies(-5).to_string(),
    );
    for id in [
        "kobayashi-maru",
        "km",
        "mutara-nebula",
        "siege",
        "unknown",
        "",
    ] {
        l.check(
            &format!("scenario-default {} ", gofmt::quote(id)),
            &marshal(&default_scenario_leaderboard(&ScenarioID::new(id))),
        );
    }
    let mut n = 0;
    while l.peek().is_some() {
        let input = hex_decode(l.expect("unmarshal "));
        let mut lb = Leaderboard::default();
        let res = json::unmarshal(&input, &mut lb);
        let es = match res {
            Ok(()) => "<nil>".to_string(),
            Err(e) => e.to_string(),
        };
        l.check("result ", &format!("{} {}", es, marshal(&lb)));
        let ind = json::marshal_indent(&lb, "", "  ").unwrap();
        l.check(
            "indent ",
            &ind.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        );
        n += 1;
    }
    eprintln!(
        "score/leaderboard parity: {n} decode cases, {} bytes",
        l.bytes_compared
    );
}
