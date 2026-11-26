//! Shared helpers for the ported pkg/tui Go tests (go_update, go_journeys, go_tui).
#![allow(dead_code)]

use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Mutex;

use sst_engine::gotime::{FixedClock, GoTime};
use sst_engine::{GameLength, GameRef, GameState, SkillLevel};
use sst_tui::theme::{self, ColorMode, Theme};
use sst_tui::Model;
use sst_tuikit::lipgloss;
use sst_tuikit::tea::{self, Cmd, KeyMsg, KeyType, Msg, KEY_RUNES};
use sst_tuikit::termenv;

static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Runs `f` with an isolated HOME/XDG_CONFIG_HOME (the Go tests used the real
/// HOME), the Ascii profile, fixed dark-background detection and test audio.
///
/// The body runs on a thread with a 64 MiB stack: the root model is ~100 KB
/// and unoptimized builds keep several copies per update on the stack, which
/// overflows the default 2 MiB test-thread stack in long journeys.
pub fn with_env(f: impl FnOnce(&PathBuf) + Send) {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::thread::scope(|s| {
        let h = std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn_scoped(s, || with_env_inner(f))
            .unwrap();
        if let Err(e) = h.join() {
            std::panic::resume_unwind(e);
        }
    });
}

fn with_env_inner(f: impl FnOnce(&PathBuf)) {
    sst_audio::set_test_mode(true);
    let tmp = std::env::temp_dir().join(format!(
        "sst-gotests-c-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let home = tmp.join("home");
    std::fs::create_dir_all(&home).unwrap();
    std::env::set_var("HOME", &home);
    std::env::set_var("XDG_CONFIG_HOME", home.join(".config"));
    lipgloss::set_color_profile(termenv::Ascii);
    lipgloss::set_has_dark_background(true);
    theme::detect::set_fixed_detection(true);
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(&home)));
    let _ = std::fs::remove_dir_all(&tmp);
    if let Err(e) = r {
        std::panic::resume_unwind(e);
    }
}

pub fn new_game() -> GameState {
    GameState::new_game(12345, SkillLevel::Good, GameLength::Medium)
}

pub fn gref(g: GameState) -> GameRef {
    sst_engine::game_ref(g)
}

pub fn modern(mode: ColorMode) -> Theme {
    theme::get_theme("modern").with_color_mode(mode)
}

/// A fixed clock (replaces Go's wall-clock `time.Now()` for scenario seeds).
pub fn fixed_clock() -> Rc<FixedClock> {
    Rc::new(FixedClock(GoTime::date(2025, 1, 1, 12, 0, 0, 0, 0)))
}

pub fn key(t: KeyType) -> Msg {
    tea::msg(KeyMsg::new(t))
}

pub fn runes(s: &str) -> Msg {
    let mut k = KeyMsg::new(KEY_RUNES);
    k.runes = s.chars().collect();
    tea::msg(k)
}

pub fn up(m: &Model, msg: Msg) -> (Model, Option<Cmd>) {
    m.update_model(msg)
}

/// Go `m.Update(cmd())` for an immediate command: runs it (without timers)
/// and feeds every produced message back into the model.
pub fn run_cmd(mut m: Model, cmd: Option<Cmd>) -> Model {
    if let Some(c) = cmd {
        let (msgs, _ticks) = c.run_immediate();
        for msg in msgs {
            m = m.update_model(msg).0;
        }
    }
    m
}

pub fn submit(m: &Model, text: &str) -> (Model, Option<Cmd>) {
    m.update_model(tea::msg(
        sst_tui::components::commandbar::CommandSubmittedMsg { text: text.into() },
    ))
}

pub fn msgs(m: &Model) -> Vec<String> {
    m.command_bar.messages()
}

pub fn any_msg(m: &Model, needle: &str) -> bool {
    msgs(m).iter().any(|s| s.contains(needle))
}

pub fn last_msg_contains(m: &Model, needle: &str) -> bool {
    msgs(m).last().is_some_and(|s| s.contains(needle))
}
