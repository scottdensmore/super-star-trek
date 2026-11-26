//! Byte-for-byte parity with `testdata/parity/components/halloffame.txt`
//! (recorded by `pkg/tui/components/halloffame/zz_paritygen_test.go`).
//! The Go generator rewrites `time.Now()` entry dates to 2401-02-03
//! 04:05:06 UTC; here that is the injected clock.

#[path = "../modal_parity_util.rs"]
#[allow(clippy::duplicate_mod)]
mod util;

use sst_engine::gofmt::{fmt_v, quote};
use sst_engine::gotime::{FixedClock, GoTime};
use sst_engine::json::GoVec;
use util::*;

use super::*;
use crate::theme::{get_theme, ColorMode};

const DATA: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testdata/parity/components/halloffame.txt"
));

fn desc(m: &Msg) -> Option<String> {
    if m.downcast_ref::<CloseModalMsg>().is_some() {
        return Some("CloseModalMsg".into());
    }
    m.downcast_ref::<ScoreRecordedMsg>().map(|v| {
        let e = &v.entry;
        format!(
            "ScoreRecordedMsg name={} score={} rank={} sd={} won={} skill={}",
            quote(&e.captain_name),
            e.score,
            quote(&e.rank),
            fmt_v(e.stardate),
            e.game_won,
            quote(&e.skill)
        )
    })
}

fn scores() -> Vec<ScoreBreakdown> {
    let d = ScoreBreakdown::default;
    vec![
        d(),
        ScoreBreakdown {
            klingons_killed: 5,
            klingon_points: 50,
            kill_rate: 1.0,
            kill_rate_points: 500,
            total_score: 550,
            rank_badge: "[COMM]".into(),
            rank_title: "Commodore".into(),
            elapsed_stardates: 5.0,
            ..d()
        },
        ScoreBreakdown {
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
        },
        ScoreBreakdown {
            total_score: -50,
            ..d()
        },
        ScoreBreakdown {
            klingons_killed: 123,
            klingon_points: 123456,
            casualties: 9999,
            casualty_penalty: 99999,
            starships_lost: 3,
            starship_penalty: 30000,
            kill_rate: 1234.5678,
            kill_rate_points: 999999,
            win_bonus: 123456,
            total_score: 1234567,
            elapsed_stardates: 9999.95,
            game_won: true,
            ..d()
        },
        ScoreBreakdown {
            klingons_killed: 7,
            klingon_points: 70,
            total_score: 175,
            elapsed_stardates: 0.04,
            kill_rate: 0.005,
            rank_title: "Custom Title Only".into(),
            ..d()
        },
    ]
}

fn boards() -> Vec<Leaderboard> {
    let d = GoTime::date(2299, 12, 31, 23, 59, 0, 0, 0);
    let e = |name: &str, score, rank: &str, sd, date| ScoreEntry {
        captain_name: name.into(),
        score,
        rank: rank.into(),
        stardate: sd,
        date,
        ..Default::default()
    };
    let long = vec![
        e(
            "Captain Jean-Luc Picard of the Enterprise",
            123456,
            "[ADMIRAL-X]",
            41153.75,
            d,
        ),
        e("艦長 スールー 光", -42, "[DISHONOR]", -1.25, d),
        e("", 0, "", 0.0, GoTime::default()),
    ];
    let many = (0..12)
        .map(|i| {
            e(
                &format!("Officer {i:02}"),
                1000 - i * 37,
                "[LT]",
                i as f64 * 1.5,
                d,
            )
        })
        .collect();
    let lb = |v: Vec<ScoreEntry>| Leaderboard {
        entries: GoVec::from_vec(v),
        tour_records: GoVec::nil(),
    };
    vec![Leaderboard::default_board(), lb(vec![]), lb(long), lb(many)]
}

fn shared(lb: Leaderboard) -> LeaderboardRef {
    Rc::new(RefCell::new(lb))
}

const WIDTHS: &[i32] = &[66, 80, 40, 30, 12, 5];

const TAB_KEYS: &[&str] = &[
    "tab",
    "tab",
    "shift+tab",
    "right",
    "left",
    "2",
    "1",
    "x",
    "down",
    "esc",
    "enter",
    "q",
    "Q",
    "h",
    "H",
    "space",
    " ",
];

const ENTRIES: &[&[&str]] = &[
    &[
        "P",
        "i",
        "c",
        "a",
        "r",
        "d",
        "backspace",
        "d",
        "enter",
        "enter",
    ],
    &["space", "enter"],
    &["x", "esc", "tab"],
    &[
        "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R",
        "S", "T", "U", "V", "left", "left", "Z", "home", "<", "end", "enter",
    ],
    &["tab", "1", "q", "winsize", "enter"],
];

fn clocked(m: &mut Model) {
    m.set_clock(Rc::new(FixedClock(GoTime::date(2401, 2, 3, 4, 5, 6, 0, 0))));
}

fn generate() -> Vec<String> {
    let mut o = Out::default();
    for c in combos() {
        let (th, pre) = c.apply();
        let full = c.full();
        for (si, sc) in scores().into_iter().enumerate() {
            for (wi, wd) in WIDTHS.iter().enumerate() {
                if !full && (wi > 0 || si != 2) {
                    continue;
                }
                let mut m = new(th, *wd, 18, "");
                m.set_state(sc.clone(), None, false);
                let p = format!("{pre}score{si}/w{wd}/");
                o.w(&format!("{p}tab1"), &m.view());
                m.active_tab = TAB_LEADERBOARD;
                o.w(&format!("{p}tab2"), &m.view());
            }
        }
        for (bi, lb) in boards().into_iter().enumerate() {
            if !full && bi != 2 {
                continue;
            }
            let mut m = new(th, 66, 18, "");
            m.set_state(scores()[1].clone(), Some(shared(lb)), false);
            m.active_tab = TAB_LEADERBOARD;
            o.w(&format!("{pre}board{bi}"), &m.view());
        }
        let mut m = new(th, 66, 18, "");
        m.set_state(scores()[1].clone(), None, false);
        for (ki, k) in TAB_KEYS.iter().enumerate() {
            let cmd = m.update(&key(k));
            let p = format!("{pre}keys/{ki}/{k}/");
            o.w(&format!("{p}msg"), &describe_cmd(cmd, &desc));
            o.w(&format!("{p}tab"), &m.active_tab.to_string());
        }
        for (ei, seq) in ENTRIES.iter().enumerate() {
            if !full && ei > 0 {
                continue;
            }
            let lb = shared(Leaderboard::default_board());
            let mut m = new(th, 66, 18, "");
            clocked(&mut m);
            m.set_state(scores()[2].clone(), Some(lb.clone()), true);
            o.w(&format!("{pre}entry{ei}/init"), &m.view());
            for (ki, k) in seq.iter().enumerate() {
                let cmd = m.update(&key(k));
                let p = format!("{pre}entry{ei}/{ki}/{k}/");
                o.w(&format!("{p}msg"), &describe_cmd(cmd, &desc));
                o.w(
                    &format!("{p}state"),
                    &format!(
                        "prompt={} tab={} value={} entries={}",
                        m.prompt_name,
                        m.active_tab,
                        quote(&m.text_input.value()),
                        lb.borrow().entries.len()
                    ),
                );
                if full || ki == seq.len() - 1 {
                    o.w(&format!("{p}view"), &m.view());
                }
            }
        }
        if full {
            let mut m = new(None, 0, 0, "");
            o.w(
                &format!("{pre}zero/size"),
                &format!("{}x{}", m.width, m.height),
            );
            m.set_theme(th);
            m.set_state(
                ScoreBreakdown {
                    total_score: 2000,
                    ..Default::default()
                },
                None,
                true,
            );
            o.w(&format!("{pre}zero/prompt"), &m.view());
            m.set_theme(get_theme("lcars").with_color_mode(ColorMode::Light));
            o.w(&format!("{pre}zero/retheme"), &m.view());
            m.leaderboard = None;
            m.set_state(
                ScoreBreakdown {
                    total_score: 10,
                    ..Default::default()
                },
                None,
                false,
            );
            m.active_tab = TAB_LEADERBOARD;
            o.w(&format!("{pre}zero/nillb"), &m.view());
            m.leaderboard = None;
            o.w(&format!("{pre}zero/nillb2"), &m.view());
            m.set_theme(None);
            o.w(
                &format!("{pre}zero/theme"),
                &format!("{} {}", m.theme.name(), m.theme.color_mode()),
            );
        }
    }
    o.0
}

#[test]
fn halloffame_parity() {
    crate::theme::detect::set_fixed_detection(true);
    let (lines, bytes) = compare(&generate(), DATA);
    eprintln!("halloffame parity: {lines} lines, {bytes} bytes");
}
