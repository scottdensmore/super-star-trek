//! The browser session and the terminal console run the same game: from the
//! same starting state, the same commands give the same state after every
//! turn, including the Romulan, Tholian, plasma and web turns of the
//! adversary profiles (testdata/README.md, fix 19).

use sst_engine::*;
use sst_tui::new_model;
use sst_wasm::Session;

/// The console reads its config and writes its Hall of Fame under `HOME`;
/// point both at a scratch directory.
fn scratch_home() -> std::path::PathBuf {
    sst_audio::set_test_mode(true);
    let home = std::env::temp_dir().join(format!("sst-terminal-parity-{}", std::process::id()));
    std::fs::create_dir_all(&home).unwrap();
    std::env::set_var("HOME", &home);
    std::env::set_var("XDG_CONFIG_HOME", home.join(".config"));
    home
}

/// Visible hostiles in the current quadrant, Klingons first.
fn visible_hostiles(g: &GameState) -> Vec<Coord> {
    let mut out: Vec<Coord> = g
        .current_quad
        .klingons
        .iter()
        .flatten()
        .filter(|k| !k.borrow().is_cloaked)
        .map(|k| k.borrow().sector)
        .collect();
    for e in g.current_quad.enemies.iter().flatten() {
        let e = e.borrow();
        if !e.is_cloaked && e.energy > 0.0 && !out.contains(&e.sector) {
            out.push(e.sector);
        }
    }
    out
}

fn adversaries_present(g: &GameState) -> bool {
    g.current_quad
        .enemies
        .iter()
        .flatten()
        .any(|e| e.borrow().faction != FactionType::Klingon && e.borrow().energy > 0.0)
        || !g.current_quad.plasma_torpedoes.is_empty()
        || !g.current_quad.web_segments.is_empty()
}

/// The next command of a simple captain: keep shields up, fight what is
/// visible (phasers, then a torpedo at the nearest ship, then a sector move
/// to give the enemy a turn) and otherwise warp on to the next quadrant.
fn next_command(g: &GameState, step: usize, route: &mut std::slice::Iter<'_, Coord>) -> String {
    if g.enterprise.shields < 400.0 && g.enterprise.energy > 1500.0 {
        return "she 800".into();
    }
    let hostiles = visible_hostiles(g);
    if let Some(t) = hostiles.first() {
        return match step % 3 {
            0 => "pha 400".into(),
            1 if g.enterprise.torpedoes > 0 => format!("tor {} {}", t[0], t[1]),
            _ => {
                let s = g.enterprise.sector;
                let r = if s[0] < 8 { s[0] + 1 } else { s[0] - 1 };
                format!("nav s {} {}", r, s[1])
            }
        };
    }
    if adversaries_present(g) {
        // Cloaked Romulans, plasma or webs but nothing to shoot at: hold
        // position and let them act.
        return "she 50".into();
    }
    match route.next() {
        Some(q) => format!("nav q {} {} 4", q[0], q[1]),
        None => "she 10".into(),
    }
}

fn state(g: &GameState) -> String {
    encode_session(g).expect("encode")
}

/// Plays `turns` commands through a browser session and the console from
/// the same start; returns how many of them ran with adversaries present.
fn play(seed: i64, profile: &str, turns: usize) -> usize {
    let mut browser = Session::new(seed, Some(DifficultyProfile::new(profile)));
    let terminal = game_ref(browser.game().clone());
    let mut console = new_model(Some(terminal.clone()), None);

    let route: Vec<Coord> = (1..=8)
        .flat_map(|r| (1..=8).map(move |c| [r, c]))
        .filter(|q| *q != browser.game().enterprise.quad)
        .collect();
    let mut route = route.iter();
    let mut adversary_turns = 0;
    for step in 0..turns {
        let cmd = next_command(browser.game(), step, &mut route);
        if adversaries_present(browser.game()) {
            adversary_turns += 1;
        }
        let out = browser.execute(&cmd);
        let _ = console.handle_command(&cmd);
        // The console records a game over in the state (for its Hall of
        // Fame); the browser only reports it. Everything else must match.
        let over = terminal.borrow().game_over;
        let mut want = browser.game().clone();
        if over {
            let t = terminal.borrow();
            want.game_won = t.game_won;
            want.game_over = t.game_over;
            want.game_over_reason = t.game_over_reason;
        }
        assert!(
            state(&want) == state(&terminal.borrow()),
            "seed {seed} {profile}: state differs after step {step} {cmd:?}\nbrowser output: {out:?}"
        );
        if over {
            break;
        }
    }
    adversary_turns
}

#[test]
fn browser_and_terminal_play_the_same_adversary_game() {
    let home = scratch_home();
    let mut adversary_turns = 0;
    for seed in [7, 42, 2024, 99999] {
        for profile in ["expert", "emeritus"] {
            adversary_turns += play(seed, profile, 150);
        }
    }
    let _ = std::fs::remove_dir_all(&home);
    // The scripted games must actually give the adversaries turns.
    assert!(
        adversary_turns >= 20,
        "only {adversary_turns} turns with Romulans, Tholians, plasma or webs"
    );
}
