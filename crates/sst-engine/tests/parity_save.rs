//! Parity: save files (MarshalIndent bytes, envelopes, legacy formats) and
//! loading through the real persistence functions.

mod common;
use common::*;

use std::collections::BTreeMap;

use sst_engine::gofmt::fmt_v;
use sst_engine::json;
use sst_engine::persist;
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

fn err_s<T>(r: &Result<T, EngineError>) -> String {
    match r {
        Ok(_) => "<nil>".into(),
        Err(e) => e.0.clone(),
    }
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn opt_hash(g: Option<&GameRef>) -> String {
    match g {
        Some(g) => state_hash(&g.borrow()),
        None => {
            // Go stateParts(nil)
            fnv_hex(b"null|null|nil")
        }
    }
}

fn tmp(name: &str) -> String {
    let dir = std::env::temp_dir().join(format!("sst-engine-parity-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name).to_string_lossy().into_owned()
}

fn inspect_line(m: &SaveMetadata) -> String {
    format!(
        "{} {} {} {} {}",
        m.skill.as_i32(),
        fmt_v(m.stardate),
        fmt_v(m.time_remaining),
        m.condition.as_i32(),
        m.klingons_left
    )
}

#[test]
fn save_files_match_go() {
    let d = data("save.txt");
    let mut l = Lines::new(&d);
    let mut bytes = 0usize;
    for n in [
        "",
        "  ",
        "a",
        "abcdefghi",
        "abcdefghij",
        "1abc",
        "_x",
        "dir/name.TRK",
        "name.trk",
        ".TRK",
        "Z.tar.gz",
        "ok.json",
        "/abs/path/B12",
        "é",
    ] {
        let r = validate_save_filename(n);
        l.check(&format!("validate {} ", gofmt::quote(n)), &err_s(&r));
    }
    let mut tour_state: Option<TourRef> = None;
    let mut tour_game: Option<GameRef> = None;
    let mut states = 0;
    while l.peek().is_some_and(|x| x.starts_with("src ")) {
        let src = l.expect("src ");
        let g: GameRef = if let Some(rest) = src.strip_prefix("cfg ") {
            let (extra, cfg) = rest.split_once(' ').unwrap();
            let mut c = GameCfg::default();
            json::unmarshal(cfg.as_bytes(), &mut c).unwrap();
            let mut g = GameState::new_game_with_options(c.seed, c.skill, c.length, c.rules);
            let (q, s) = (g.enterprise.quad, g.enterprise.sector);
            g.populate_quadrant(q, s);
            if extra == "true" {
                let mut m = BTreeMap::new();
                m.insert(RefitID::damage_nanites(), 2);
                m.insert(RefitID::deflector_grid(), 1);
                g.active_refits = Some(m);
                g.game_over = true;
                g.game_over_reason = GameOverReason::Energy;
            }
            game_ref(g)
        } else if let Some(id) = src.strip_prefix("scenario ") {
            game_ref(get_scenario(&ScenarioID::new(id)).unwrap().build(11))
        } else {
            let t = tour_ref(TourState::new_at(4242, 0));
            let g = t.borrow_mut().start_current_sector(Some(3)).unwrap();
            tour_state = Some(t);
            tour_game = Some(g.clone());
            g
        };
        let idx = l.expect("state ").to_string();
        let gs = g.borrow().clone();
        l.check("compact ", &marshal(&gs));
        l.check("klingons ", &marshal(&gs.current_quad.klingons));
        let legacy = encode_legacy_save(&gs).unwrap();
        let env = encode_save_game(&g).unwrap();
        l.check(
            "legacyhash ",
            &format!("{} {}", fnv_hex(&legacy), legacy.len()),
        );
        if l.peek().is_some_and(|x| x.starts_with("legacy ")) {
            l.check("legacy ", &hex(&legacy));
        }
        l.check("envhash ", &format!("{} {}", fnv_hex(&env), env.len()));
        bytes += legacy.len() + env.len();
        for data in [&legacy, &env] {
            let path = tmp(&format!("state{idx}.TRK"));
            std::fs::write(&path, data).unwrap();
            let lg = persist::load_game(&path);
            l.check(
                "load ",
                &format!("{} {}", err_s(&lg), opt_hash(lg.as_ref().ok())),
            );
            let m = persist::inspect_save(&path);
            if let Ok(m) = &m {
                l.check("inspect ", &format!("<nil> {}", inspect_line(m)));
            }
            let lt = persist::load_tour_game(&path);
            let (h, has_t) = match &lt {
                Ok((g2, t2)) => (opt_hash(g2.as_ref()), t2.is_some()),
                Err(_) => (opt_hash(None), false),
            };
            l.check("loadtour ", &format!("{} {} {}", err_s(&lt), h, has_t));
        }
        states += 1;
    }
    let tour = tour_state.unwrap();
    let tg = tour_game.unwrap();
    let tenv = encode_tour_save(Some(&tg), Some(&tour)).unwrap();
    l.check("tourenv ", &hex(&tenv));
    {
        let mut m = BTreeMap::new();
        m.insert(RefitID::dilithium_core(), 3);
        m.insert(RefitID::torpedo_bays(), 1);
        tour.borrow_mut().installed_refits = Some(m);
    }
    let tenv2 = encode_tour_save(Some(&tg), Some(&tour)).unwrap();
    l.check("tourenv2 ", &hex(&tenv2));
    let only = tour_ref(TourState {
        id: TourID("t".into()),
        seed: 3,
        ..Default::default()
    });
    let tonly = encode_tour_save(None, Some(&only)).unwrap();
    l.check("touronly ", &hex(&tonly));
    bytes += tenv.len() + tenv2.len() + tonly.len();

    // round-trip via the persistence layer too
    let p = tmp("roundtrip.TRK");
    persist::save_tour_game(&p, Some(&tg), Some(&tour)).unwrap();
    assert_eq!(std::fs::read(&p).unwrap(), tenv2);

    let mut cases = 0;
    while l.peek().is_some_and(|x| x.starts_with("case ")) {
        let c = hex_decode(l.expect("case "));
        let path = tmp(&format!("sstcase{cases}.TRK"));
        std::fs::write(&path, &c).unwrap();
        let lg = persist::load_game(&path);
        l.check("load ", &err_s(&lg));
        if let Ok(g) = &lg {
            l.check_full(&g.borrow());
        }
        match persist::inspect_save(&path) {
            Ok(m) => l.check("inspect ", &format!("<nil> {}", inspect_line(&m))),
            Err(e) => l.check("inspect ", &e.0.replace(&path, "PATH")),
        }
        let lt = persist::load_tour_game(&path);
        l.check("loadtour ", &err_s(&lt).replace(&path, "PATH"));
        if let Ok((g2, t2)) = &lt {
            if let Some(g2) = g2 {
                l.check_full(&g2.borrow());
            }
            if let Some(t2) = t2 {
                l.check("T ", &marshal(t2));
                let alias = match (&t2.borrow().current_game_state, g2) {
                    (Some(a), Some(b)) => std::rc::Rc::ptr_eq(a, b),
                    _ => false,
                };
                l.check("alias ", &alias.to_string());
            }
        }
        let mut sg = GameState::default();
        let r = json::unmarshal(&c, &mut sg);
        l.check(
            "unmarshal ",
            &match r {
                Ok(()) => "<nil>".to_string(),
                Err(e) => e.to_string(),
            },
        );
        l.check_full(&sg);
        cases += 1;
    }
    let missing = tmp("sst-does-not-exist.TRK");
    l.check(
        "missing ",
        &err_s(&persist::load_game(&missing)).replace(&missing, "PATH"),
    );
    l.check(
        "missingtour ",
        &err_s(&persist::load_tour_game(&missing)).replace(&missing, "PATH"),
    );
    l.check(
        "missinginspect ",
        &err_s(&persist::inspect_save(&missing)).replace(&missing, "PATH"),
    );
    assert!(l.peek().is_none());
    eprintln!(
        "save parity: {states} states, {cases} decode cases, {bytes} save-file bytes, {} transcript bytes",
        l.bytes_compared
    );
}

use sst_engine::gofmt;
