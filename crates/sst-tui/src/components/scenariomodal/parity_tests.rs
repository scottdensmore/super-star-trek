//! Byte-for-byte parity with `testdata/parity/components/scenariomodal.txt`
//! (recorded by `pkg/tui/components/scenariomodal/zz_paritygen_test.go`).
//! The Go generator reads boards from a temp config dir; here the injected
//! loader returns what those files decode to.

#[path = "../modal_parity_util.rs"]
#[allow(clippy::duplicate_mod)]
mod util;

use sst_engine::gotime::GoTime;
use sst_engine::json::GoVec;
use sst_engine::leaderboard::default_scenario_leaderboard;
use sst_tuikit::lipgloss;
use sst_tuikit::termenv::Profile;
use util::*;

use super::*;
use crate::theme::{get_theme, ColorMode};

const DATA: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testdata/parity/components/scenariomodal.txt"
));

const SEQUENCE: &[&str] = &[
    "down", "j", "enter", "down", "k", "up", "up", "enter", "x", "tab", "esc", "q", "Q", "left",
];

fn desc(m: &Msg) -> Option<String> {
    if let Some(v) = m.downcast_ref::<MsgLaunchScenario>() {
        return Some(format!("MsgLaunchScenario id={}", v.scenario_id));
    }
    m.downcast_ref::<MsgCloseScenarioModal>()
        .map(|_| "MsgCloseScenarioModal".to_string())
}

fn custom_mutara() -> Leaderboard {
    let d = GoTime::date(2370, 6, 15, 0, 0, 0, 0, 0);
    let mut lb = Leaderboard {
        entries: GoVec::from_vec(vec![
            ScoreEntry {
                captain_name: "Lt. Worf".into(),
                score: 640,
                rank: "[LT]".into(),
                stardate: 47.25,
                date: d,
                ..Default::default()
            },
            ScoreEntry {
                captain_name: "Captain Jean-Luc Picard of the USS Enterprise-D".into(),
                score: 99999,
                rank: "[ADMIRAL-OF-THE-FLEET]".into(),
                stardate: 1234.56,
                date: d,
                game_won: true,
                ..Default::default()
            },
        ]),
        tour_records: GoVec::nil(),
    };
    lb.normalize_loaded(false);
    lb
}

fn loader(tag: &'static str) -> LeaderboardLoader {
    Rc::new(move |id: &ScenarioID| {
        if tag == "default" {
            return Ok(default_scenario_leaderboard(id));
        }
        if *id == ScenarioID::mutara_nebula() {
            Ok(custom_mutara())
        } else if *id == ScenarioID::kobayashi_maru() {
            Err(EngineError::new("read: is a directory"))
        } else if tag == "custom" {
            Ok(Leaderboard {
                entries: GoVec::empty(),
                tour_records: GoVec::nil(),
            })
        } else {
            Ok(default_scenario_leaderboard(id))
        }
    })
}

fn state(o: &mut Out, p: &str, m: &Model, view: bool) {
    let sel = m
        .selected_scenario()
        .map(|s| s.id.0.clone())
        .unwrap_or_else(|| "nil".into());
    o.w(&format!("{p}selected"), &sel);
    if view {
        o.w(&format!("{p}view"), &m.view());
    }
}

fn generate() -> Vec<String> {
    let mut o = Out::default();
    for tag in ["default", "custom", "corrupt"] {
        for c in combos() {
            let (th, pre) = c.apply();
            let pre = format!("{pre}{tag}/");
            let full = c.full();
            let mut m = new_with_loader(th, 72, 18, loader(tag));
            if tag != "default" {
                m.reset();
            }
            for i in 0..3 {
                if full || i == 1 || tag == "default" {
                    o.w(&format!("{pre}init/{i}"), &m.view());
                }
                let _ = m.update(&key("down"));
            }
            for (ki, k) in SEQUENCE.iter().enumerate() {
                let cmd = m.update(&key(k));
                let p = format!("{pre}seq/{ki}/{k}/");
                o.w(&format!("{p}msg"), &describe_cmd(cmd, &desc));
                state(&mut o, &p, &m, full);
            }
        }
    }

    lipgloss::set_color_profile(Profile::Ascii);
    let th = get_theme("lcars").with_color_mode(ColorMode::Dark);
    // The temp config dir still holds the boards of the "corrupt" run.
    let mut m = new_with_loader(th, 0, -5, loader("corrupt"));
    m.set_dimensions(80, 24);
    m.set_size(-1, 30);
    o.w("sized", &format!("{}x{}", m.width, m.height));
    m.set_theme(get_theme("crt").with_color_mode(ColorMode::Light));
    o.w("sized/view", &m.view());

    let mut z = Model {
        loader: loader("default"),
        ..Default::default()
    };
    o.w(
        "zero/selected",
        &z.selected_scenario().is_none().to_string(),
    );
    let cmd = z.update(&key("enter"));
    o.w("zero/enter", &describe_cmd(cmd, &desc));
    let cmd = z.update(&key("down"));
    o.w("zero/down", &describe_cmd(cmd, &desc));
    // Go's zero Model has a nil theme, which renders with DefaultTheme();
    // under the Ascii profile the palette does not matter.
    o.w("zero/view", &z.view());
    o.0
}

#[test]
fn scenariomodal_parity() {
    crate::theme::detect::set_fixed_detection(true);
    // SST_PARITY_RECORD=<path> re-records the reference after an
    // intentional change (every line is regenerated; `git diff` shows which
    // moved).
    if let Some(path) = std::env::var("SST_PARITY_RECORD")
        .ok()
        .filter(|p| !p.is_empty())
    {
        let mut out = generate().join("\n");
        out.push('\n');
        std::fs::write(&path, out).unwrap();
        return;
    }
    let (lines, bytes) = compare(&generate(), DATA);
    eprintln!("scenariomodal parity: {lines} lines, {bytes} bytes");
}
