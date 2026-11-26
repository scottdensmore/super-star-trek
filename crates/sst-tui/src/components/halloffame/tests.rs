//! Ports of `halloffame_test.go`.

use sst_tuikit::tea::{self, KeyMsg};

use super::*;
use crate::theme::{detect, CrtTheme};

fn setup() -> Theme {
    detect::set_fixed_detection(true);
    default_theme()
}

fn key(t: tea::KeyType) -> Msg {
    tea::msg(KeyMsg::new(t))
}

fn runes(s: &str) -> Msg {
    tea::msg(KeyMsg::runes(s))
}

fn assert_box(view: &str) -> Vec<String> {
    let lines: Vec<String> = view.split('\n').map(String::from).collect();
    assert_eq!(lines.len(), 18);
    for (i, l) in lines.iter().enumerate() {
        assert_eq!(ansi::string_width(l), 66, "line {i}: {l:?}");
    }
    lines
}

fn closes(cmd: Option<Cmd>) -> bool {
    cmd.and_then(|c| c.call())
        .is_some_and(|m| m.downcast_ref::<CloseModalMsg>().is_some())
}

#[test]
fn dimensions_and_tab_switching() {
    let mut m = new(setup(), 66, 18, "");
    let score = ScoreBreakdown {
        klingons_killed: 5,
        klingon_points: 50,
        kill_rate: 1.0,
        kill_rate_points: 500,
        total_score: 550,
        rank_badge: "[COMM]".into(),
        rank_title: "Commodore".into(),
        elapsed_stardates: 5.0,
        ..Default::default()
    };
    m.set_state(
        score,
        Some(Rc::new(RefCell::new(Leaderboard::default_board()))),
        false,
    );

    let view1 = m.view();
    let lines1 = assert_box(&view1);
    assert!(view1.contains("MISSION DEBRIEF") && view1.contains("CURRENT NET SCORE"));
    for (idx, suffix) in [
        (2, "(-   0)│"),
        (3, "(-   0)│"),
        (4, "(-   0)│"),
        (5, "(-   0)│"),
        (6, "(-   0)│"),
        (7, "(-   0)│"),
        (8, "  5.0│"),
    ] {
        assert!(
            lines1[idx].ends_with(suffix),
            "line {idx}: {:?}",
            lines1[idx]
        );
    }

    let _ = m.update(&key(tea::KEY_TAB));
    let view2 = m.view();
    let lines2 = assert_box(&view2);
    assert!(view2.contains("HALL OF FAME") && view2.contains("James T. Kirk"));
    assert!(lines2[1].contains("STARDATE") && lines2[1].contains("DATE"));

    let mut back = m.clone();
    let _ = back.update(&key(tea::KEY_SHIFT_TAB));
    assert_eq!(back.active_tab, TAB_TELEMETRY);
    let mut s = m.clone();
    let _ = s.update(&runes("shift+tab"));
}

#[test]
fn dismissal_keys() {
    let mut m = new(setup(), 66, 18, "");
    for msg in [key(tea::KEY_ESC), runes("q"), runes("h")] {
        assert!(closes(m.update(&msg)));
    }
}

#[test]
fn name_submission() {
    let th = setup();
    let dir = std::env::temp_dir().join(format!("sst-hof-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let score_file = dir.join("highscores.json");
    let _ = std::fs::remove_file(&score_file);
    let path = score_file.to_string_lossy().into_owned();
    let mut m = new(th, 66, 18, &path);
    let score = ScoreBreakdown {
        klingons_killed: 10,
        klingon_points: 100,
        total_score: 1500,
        rank_badge: "[FADM]".into(),
        rank_title: "Fleet Admiral".into(),
        elapsed_stardates: 8.0,
        game_won: true,
        ..Default::default()
    };
    let lb = Rc::new(RefCell::new(Leaderboard::default_board()));
    m.set_state(score, Some(lb.clone()), true);
    let view = m.view();
    assert_box(&view);
    assert!(view.contains("ENTER CALLSIGN"));

    for c in "Picard".chars() {
        let _ = m.update(&runes(&c.to_string()));
    }
    let msg = m.update(&key(tea::KEY_ENTER)).expect("cmd").call().unwrap();
    let rec = msg
        .downcast_ref::<ScoreRecordedMsg>()
        .expect("ScoreRecordedMsg");
    assert_eq!(rec.entry.captain_name, "Picard");
    assert_eq!(rec.entry.score, 1500);
    // Shared pointer: the caller's leaderboard sees the new entry.
    assert_eq!(lb.borrow().entries[0].captain_name, "Picard");

    let loaded = persist::load_leaderboard(Some(&path)).expect("load");
    assert!(!loaded.entries.is_empty());
    assert_eq!(loaded.entries[0].captain_name, "Picard");

    assert!(closes(m.update(&key(tea::KEY_ENTER))));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn tab_keys_and_themes() {
    let mut m = new(setup(), 66, 18, "");
    let _ = m.update(&runes("2"));
    assert!(m.view().contains("HALL OF FAME"));
    let _ = m.update(&runes("1"));
    assert!(m.view().contains("MISSION DEBRIEF"));
    m.set_theme(Theme::from(CrtTheme::default()));
    assert_eq!(m.theme.name(), "crt");
}

#[test]
fn edge_cases() {
    setup();
    let mut m = new(None, 0, 0, "");
    assert_eq!((m.width, m.height), (66, 18));
    m.set_theme(None);
    assert_eq!(m.theme, default_theme());

    let neg = ScoreBreakdown {
        total_score: -50,
        ..Default::default()
    };
    m.set_state(neg.clone(), None, false);
    assert!(m.view().contains("[DISHONOR]"));

    m.set_state(neg, None, true);
    assert!(m.prompt_name);
    let cmd = m.update(&key(tea::KEY_ESC));
    assert!(!m.prompt_name);
    assert!(closes(cmd));

    m.set_state(
        ScoreBreakdown {
            total_score: 2000,
            ..Default::default()
        },
        None,
        true,
    );
    let msg = m.update(&key(tea::KEY_ENTER)).expect("cmd").call().unwrap();
    let rec = msg.downcast_ref::<ScoreRecordedMsg>().unwrap();
    assert_eq!(rec.entry.captain_name, "Unknown Captain");
    assert!(!m.prompt_name);
}

#[test]
fn extreme_scores_layout_integrity() {
    let mut m = new(setup(), 66, 18, "");
    let extreme = ScoreBreakdown {
        klingons_killed: 50,
        klingon_points: 5000,
        commanders_killed: 10,
        commander_points: 1000,
        super_commanders_killed: 5,
        super_commander_points: 1000,
        romulans_killed: 20,
        romulan_points: 800,
        romulans_surrendered: 5,
        surrendered_points: 150,
        casualties: 150,
        casualty_penalty: 1500,
        starbases_lost: 2,
        starbase_penalty: 200,
        help_calls: 5,
        help_penalty: 225,
        planets_destroyed: 3,
        planet_penalty: 30,
        stars_destroyed: 4,
        star_penalty: 20,
        starships_lost: 1,
        starship_penalty: 100,
        kill_rate: 25.50,
        kill_rate_points: 12750,
        win_bonus: 2500,
        total_score: 15000,
        rank_badge: "[FADM]".into(),
        rank_title: "Fleet Admiral".into(),
        elapsed_stardates: 123.4,
        game_won: true,
    };
    m.set_state(
        extreme,
        Some(Rc::new(RefCell::new(Leaderboard::default_board()))),
        false,
    );
    let lines = assert_box(&m.view());
    for l in &lines[1..=16] {
        assert!(l.starts_with('│') && l.ends_with('│'), "{l:?}");
    }
    assert!(lines[7].ends_with("(- 100)│"), "{:?}", lines[7]);
    assert!(lines[7].contains("(+12750)"));
    assert!(lines[8].ends_with(" 123.4│"), "{:?}", lines[8]);
    assert!(lines[8].contains("(+2500)"));
}
