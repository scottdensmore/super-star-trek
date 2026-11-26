//! Port of pkg/engine/leaderboard_test.go.
#![allow(clippy::type_complexity)]
//!
//! Skipped (Go nil-receiver semantics with no Rust equivalent): the
//! `nilLB.Qualifies` / `nilLB.Add` / `nilLB.RecordTour` / `RecordTour(nil, ..)`
//! and `SaveScenarioLeaderboard(id, nil)` sub-assertions.

use std::collections::BTreeMap;
use std::sync::Mutex;

use sst_engine::gotime::{Clock, GoTime, SystemClock};
use sst_engine::json::GoVec;
use sst_engine::*;

static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Serialises env mutation and restores HOME / XDG_CONFIG_HOME on drop.
struct EnvGuard {
    _lock: std::sync::MutexGuard<'static, ()>,
    saved: Vec<(&'static str, Option<String>)>,
}

impl EnvGuard {
    fn new() -> Self {
        let lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let saved = ["HOME", "XDG_CONFIG_HOME"]
            .into_iter()
            .map(|k| (k, std::env::var(k).ok()))
            .collect();
        EnvGuard { _lock: lock, saved }
    }

    fn set(&self, k: &str, v: &str) {
        std::env::set_var(k, v);
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (k, v) in &self.saved {
            match v {
                Some(v) => std::env::set_var(k, v),
                None => std::env::remove_var(k),
            }
        }
    }
}

fn temp_dir(name: &str) -> String {
    let d = std::env::temp_dir().join(format!("sst-go-lb-{}-{}", std::process::id(), name));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d.to_string_lossy().into_owned()
}

fn now() -> GoTime {
    SystemClock.now()
}

#[test]
fn leaderboard_default_seed_and_qualifies() {
    let lb = Leaderboard::default_board();
    assert!(!lb.entries.is_empty());
    assert!(lb.entries.len() <= 10);
    let lowest = lb.entries[lb.entries.len() - 1].score;
    assert!(lb.qualifies(lowest + 10));
    assert!(!(lb.qualifies(lowest - 50) && lb.entries.len() >= 10));
}

#[test]
fn leaderboard_save_and_load_atomic() {
    let dir = temp_dir("atomic");
    let path = gopath::join(&[&dir, "highscores.json"]);
    let lb = Leaderboard {
        entries: GoVec::from_vec(vec![
            ScoreEntry {
                captain_name: "Kirk".into(),
                score: 1180,
                rank: "[FADM]".into(),
                date: now(),
                ..Default::default()
            },
            ScoreEntry {
                captain_name: "Spock".into(),
                score: 980,
                rank: "[RADM]".into(),
                date: now(),
                ..Default::default()
            },
        ]),
        tour_records: GoVec::nil(),
    };
    persist::save_leaderboard(&lb, &path).expect("save");
    let loaded = persist::load_leaderboard(Some(&path)).expect("load");
    assert_eq!(loaded.entries.len(), 2);
    assert!(loaded.entries[0].captain_name == "Kirk" && loaded.entries[0].score == 1180);
}

#[test]
fn leaderboard_add_and_truncate_top10() {
    let mut lb = Leaderboard::default_board();
    lb.add(ScoreEntry {
        captain_name: "Picard".into(),
        score: 2000,
        rank: "[FADM]".into(),
        date: now(),
        ..Default::default()
    });
    assert!(lb.entries.len() <= 10);
    assert!(lb.entries[0].captain_name == "Picard" && lb.entries[0].score == 2000);
}

#[test]
fn leaderboard_corrupted_file_recovery() {
    let dir = temp_dir("corrupt");
    let path = gopath::join(&[&dir, "corrupt.json"]);
    std::fs::write(&path, b"{ invalid json ").unwrap();
    let lb = persist::load_leaderboard(Some(&path)).expect("expected clean fallback");
    assert!(!lb.entries.is_empty());
    assert!(
        std::fs::metadata(format!("{path}.corrupt")).is_ok(),
        "expected .corrupt backup"
    );
}

#[test]
fn leaderboard_default_leaderboard_path() {
    let env = EnvGuard::new();
    let tmp = temp_dir("defpath");
    env.set("XDG_CONFIG_HOME", &tmp);
    let path = persist::default_leaderboard_path();
    assert_eq!(
        path,
        gopath::join(&[&tmp, "super-star-trek", "highscores.json"])
    );
    assert!(
        std::fs::metadata(gopath::join(&[&tmp, "super-star-trek"])).is_err(),
        "must not create directories"
    );

    env.set("XDG_CONFIG_HOME", "");
    let home = temp_dir("defpath-home");
    env.set("HOME", &home);
    assert_eq!(
        persist::default_leaderboard_path(),
        gopath::join(&[&home, ".config", "super-star-trek", "highscores.json"])
    );

    env.set("XDG_CONFIG_HOME", "");
    env.set("HOME", "");
    assert_eq!(persist::default_leaderboard_path(), "./.sst-scores.json");
}

#[test]
fn leaderboard_load_non_existent() {
    let dir = temp_dir("nonexist");
    let path = gopath::join(&[&dir, "does_not_exist.json"]);
    let lb = persist::load_leaderboard(Some(&path)).expect("nil error");
    assert!(!lb.entries.is_empty());
}

#[test]
fn leaderboard_qualifies_and_add_edge_cases() {
    let mut empty = Leaderboard::default();
    assert!(empty.qualifies(10));
    empty.add(ScoreEntry {
        captain_name: "First".into(),
        score: 100,
        ..Default::default()
    });
    assert_eq!(empty.entries.len(), 1);

    let names = [
        ("Kirk", 1000),
        ("Spock", 900),
        ("Pike", 800),
        ("Sulu", 700),
        ("Uhura", 600),
        ("Scotty", 500),
        ("McCoy", 400),
        ("Chekov", 300),
        ("Chapel", 200),
        ("Rand", 100),
    ];
    let mut full = Leaderboard {
        entries: names
            .iter()
            .map(|(n, s)| ScoreEntry {
                captain_name: (*n).into(),
                score: *s,
                ..Default::default()
            })
            .collect(),
        tour_records: GoVec::nil(),
    };
    assert!(!full.qualifies(100));
    assert!(full.qualifies(101));
    full.add(ScoreEntry {
        captain_name: "Bones".into(),
        score: 450,
        ..Default::default()
    });
    assert_eq!(full.entries.len(), 10);
    assert!(full.entries[6].captain_name == "Bones" && full.entries[6].score == 450);
    assert!(full.entries.iter().all(|e| e.captain_name != "Rand"));
}

#[test]
fn leaderboard_default_legends_presence() {
    let lb = Leaderboard::default_board();
    assert_eq!(lb.entries.len(), 10);
    let expected = [
        "James T. Kirk",
        "Spock",
        "Christopher Pike",
        "Hikaru Sulu",
        "Nyota Uhura",
        "Montgomery Scott",
        "Leonard McCoy",
        "Pavel Chekov",
        "Christine Chapel",
        "Janice Rand",
    ];
    for (i, name) in expected.iter().enumerate() {
        assert_eq!(lb.entries[i].captain_name, *name, "entry {i}");
    }
    for i in 0..lb.entries.len() - 1 {
        assert!(
            lb.entries[i].score > lb.entries[i + 1].score,
            "entry {i} not greater than next"
        );
    }
    assert!(lb.entries[0].captain_name == "James T. Kirk" && lb.entries[0].score == 1180);
}

#[test]
fn scenario_leaderboards_isolation() {
    let env = EnvGuard::new();
    let tmp = temp_dir("isolation");
    env.set("HOME", &tmp);
    env.set("XDG_CONFIG_HOME", "");
    let entry = ScoreEntry {
        captain_name: "Test Cadet".into(),
        score: 1200,
        rank: "[COMM-3]".into(),
        commendation: "Starfleet Cross of Honor".into(),
        date: now(),
        ..Default::default()
    };
    let (rank, added) =
        persist::add_scenario_score(&ScenarioID::kobayashi_maru(), entry).expect("add");
    assert!(added);
    assert!(rank >= 1, "expected positive rank, got {rank}");

    let std_lb = persist::load_leaderboard(None).expect("std lb");
    assert!(std_lb
        .entries
        .iter()
        .all(|e| e.captain_name != "Test Cadet"));

    let scen = persist::load_scenario_leaderboard(&ScenarioID::kobayashi_maru()).expect("scen lb");
    assert!(scen
        .entries
        .iter()
        .any(|e| e.captain_name == "Test Cadet" && e.commendation == "Starfleet Cross of Honor"));
}

#[test]
fn scenario_leaderboards_defaults() {
    let km = default_scenario_leaderboard(&ScenarioID::kobayashi_maru());
    assert_eq!(km.entries.len(), 3);
    let e = &km.entries;
    assert!(
        e[0].captain_name == "Cadet James T. Kirk"
            && e[0].rank == "[KIRK-AWARD]"
            && e[0].commendation == "Commendation for Original Thinking"
    );
    assert!(
        e[1].captain_name == "Cadet Spock"
            && e[1].rank == "[COMM-3]"
            && e[1].commendation == "Starfleet Cross of Honor"
    );
    assert!(
        e[2].captain_name == "Lieutenant Saavik"
            && e[2].rank == "[COMM-2]"
            && e[2].commendation == "Tactical Excellence"
    );

    let mn = default_scenario_leaderboard(&ScenarioID::mutara_nebula());
    assert_eq!(mn.entries.len(), 2);
    assert!(
        mn.entries[0].captain_name == "Admiral James T. Kirk"
            && mn.entries[0].score == 1500
            && mn.entries[0].game_won
    );
    assert!(
        mn.entries[1].captain_name == "Captain Clark Terrell"
            && mn.entries[1].score == 820
            && !mn.entries[1].game_won
    );

    let ss = default_scenario_leaderboard(&ScenarioID::starbase_siege());
    assert_eq!(ss.entries.len(), 2);
    assert!(
        ss.entries[0].captain_name == "Captain Hikaru Sulu"
            && ss.entries[0].score == 1350
            && ss.entries[0].game_won
    );
    assert!(
        ss.entries[1].captain_name == "Commander Montgomery Scott"
            && ss.entries[1].score == 1100
            && ss.entries[1].game_won
    );

    assert_eq!(
        default_scenario_leaderboard(&ScenarioID::new("unknown-scenario"))
            .entries
            .len(),
        0
    );
}

#[test]
fn scenario_leaderboards_corrupted_file_recovery() {
    let env = EnvGuard::new();
    let tmp = temp_dir("scorrupt");
    env.set("HOME", &tmp);
    env.set("XDG_CONFIG_HOME", "");
    let path = persist::scenario_leaderboard_path(&ScenarioID::kobayashi_maru());
    std::fs::create_dir_all(gopath::dir(&path)).unwrap();
    std::fs::write(&path, b"{ corrupt json").unwrap();
    let lb = persist::load_scenario_leaderboard(&ScenarioID::kobayashi_maru()).expect("fallback");
    assert_eq!(lb.entries.len(), 3);
    assert!(std::fs::metadata(format!("{path}.corrupt")).is_ok());
}

#[test]
fn scenario_leaderboards_add_and_rank_cases() {
    let env = EnvGuard::new();
    let tmp = temp_dir("addrank");
    env.set("HOME", &tmp);
    env.set("XDG_CONFIG_HOME", "");
    let km = ScenarioID::kobayashi_maru();
    let top = ScoreEntry {
        captain_name: "Cheating Cadet".into(),
        score: 3000,
        rank: "[KIRK-AWARD]".into(),
        commendation: "Reprogrammed Simulator".into(),
        ..Default::default()
    };
    let (rank, added) = persist::add_scenario_score(&km, top).expect("add top");
    assert!(added && rank == 1, "rank={rank} added={added}");
    for i in 0..6 {
        let e = ScoreEntry {
            captain_name: format!("Cadet {i}"),
            score: 500 - i * 10,
            ..Default::default()
        };
        let (_, added) = persist::add_scenario_score(&km, e).expect("fill");
        assert!(added, "failed to fill {i}");
    }
    let lb = persist::load_scenario_leaderboard(&km).expect("load");
    assert_eq!(lb.entries.len(), 10);
    let lowest = lb.entries[lb.entries.len() - 1].score;
    let low = ScoreEntry {
        captain_name: "Failed Cadet".into(),
        score: lowest - 50,
        ..Default::default()
    };
    let (rank, added) = persist::add_scenario_score(&km, low).expect("no error");
    assert!(!added && rank == 0, "rank={rank} added={added}");
}

#[test]
fn scenario_leaderboards_alias_and_paths() {
    let env = EnvGuard::new();
    let tmp = temp_dir("alias");
    env.set("HOME", &tmp);
    env.set("XDG_CONFIG_HOME", "");
    let p1 = persist::scenario_leaderboard_path(&ScenarioID::kobayashi_maru());
    let p2 = persist::scenario_leaderboard_path(&ScenarioID::new("km"));
    let p3 = persist::scenario_leaderboard_path(&ScenarioID::new("KOBAYASHI"));
    assert!(p1 == p2 && p1 == p3, "{p1} {p2} {p3}");
    assert_eq!(gopath::base(&p1), "leaderboard_kobayashi-maru.json");
    // Go: SaveScenarioLeaderboard(id, nil) errors — no nil leaderboard in Rust (skipped).
}

#[test]
fn leaderboard_record_tour_and_rank_commission() {
    let mut lb = Leaderboard::new();
    let mut tour = TourState::new_at(888, 0);
    tour.sectors_completed = 4;
    tour.completed = true;
    tour.total_tour_score = 18500;
    tour.installed_refits
        .as_mut()
        .unwrap()
        .insert(RefitID::dilithium_core(), 3);
    let rec = persist::record_tour(&mut lb, &tour, "Kirk");
    assert_eq!(rec.rank, "Admiral of the Fleet");
    assert!(!rec.medals.is_empty());
    assert_eq!(rec.sectors_cleared, 4);
}

#[test]
fn calculate_tour_commission_ranks_and_medals() {
    let dc = RefitID::dilithium_core();
    let tb = RefitID::torpedo_bays();
    let cases: Vec<(&str, i32, BTreeMap<RefitID, i32>, &str, Vec<&str>)> = vec![
        (
            "4 sectors full clear with architect refits",
            4,
            [(dc.clone(), 3), (tb.clone(), 3)].into_iter().collect(),
            "Admiral of the Fleet",
            vec![
                "Starfleet Legion of Honor",
                "Klingon Campaign Ribbon",
                "Vanguard Star",
                "Master Starship Architect",
            ],
        ),
        (
            "3 sectors clear",
            3,
            BTreeMap::new(),
            "Commodore",
            vec!["Starfleet Merit Citation", "Klingon Campaign Ribbon"],
        ),
        (
            "2 sectors clear",
            2,
            BTreeMap::new(),
            "Fleet Captain",
            vec!["Frontier Service Medal"],
        ),
        (
            "1 sector clear",
            1,
            BTreeMap::new(),
            "Captain",
            vec!["Patrol Ribbon"],
        ),
        (
            "0 sectors clear (KIA)",
            0,
            BTreeMap::new(),
            "Commander (KIA)",
            vec![],
        ),
    ];
    for (name, sectors, refits, rank_want, medals_want) in cases {
        let mut tour = TourState::new_at(123, 0);
        tour.sectors_completed = sectors;
        tour.installed_refits = Some(refits);
        let (rank, medals) = calculate_tour_commission(Some(&tour));
        assert_eq!(rank, rank_want, "{name}");
        assert_eq!(medals.len(), medals_want.len(), "{name}: {medals:?}");
        for (i, m) in medals_want.iter().enumerate() {
            assert_eq!(&medals[i], m, "{name}: medal {i}");
        }
    }
    let (rank, medals) = calculate_tour_commission(None);
    assert_eq!(rank, "Cadet");
    assert!(medals.is_empty());
}

#[test]
fn leaderboard_record_tour_edge_cases() {
    let mut lb = Leaderboard::new();
    let mut tour = TourState::new_at(101, 0);
    tour.sectors_completed = 1;
    let rec = lb.record_tour_at(&tour, "", SystemClock.now());
    assert_eq!(rec.callsign, "Enterprise");
    // Go: RecordTour(nil, ..) and nil-receiver RecordTour return nil — not representable (skipped).
}

#[test]
fn leaderboard_save_and_load_tour_records() {
    let dir = temp_dir("tourrec");
    let path = gopath::join(&[&dir, "highscores.json"]);
    let mut lb = Leaderboard::new();
    let mut tour = TourState::new_at(777, 0);
    tour.sectors_completed = 4;
    tour.completed = true;
    tour.total_tour_score = 20000;
    {
        let m = tour.installed_refits.as_mut().unwrap();
        m.insert(RefitID::dilithium_core(), 3);
        m.insert(RefitID::deflector_grid(), 3);
    }
    persist::record_tour(&mut lb, &tour, "Excelsior");
    persist::save_leaderboard(&lb, &path).expect("save");
    let loaded = persist::load_leaderboard(Some(&path)).expect("load");
    assert_eq!(loaded.tour_records.len(), 1);
    let r = &loaded.tour_records[0];
    assert_eq!(r.callsign, "Excelsior");
    assert_eq!(r.rank, "Admiral of the Fleet");
    assert_eq!(r.score, 20000);
    assert_eq!(r.sectors_cleared, 4);
    assert_eq!(r.refits_count, 6);
    assert!(r.completed);
}
