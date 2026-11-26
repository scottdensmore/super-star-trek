//! Shared helpers for the modal components' parity tests (included with
//! `#[path]` by each `parity_tests.rs`). Mirrors the `pg*` helpers of the Go
//! `zz_paritygen_test.go` generators.
#![allow(dead_code)]

use sst_engine::gofmt::quote;
use sst_tuikit::lipgloss;
use sst_tuikit::tea::{self, BatchMsg, Cmd, KeyMsg, Msg, WindowSizeMsg};
use sst_tuikit::termenv::Profile;

use crate::theme::{get_theme, ColorMode, Theme, THEME_CRT, THEME_LCARS, THEME_MODERN};

/// Output lines (`<key>\t<%q value>`).
#[derive(Default)]
pub struct Out(pub Vec<String>);

impl Out {
    pub fn w(&mut self, k: &str, v: &str) {
        self.0.push(format!("{k}\t{}", quote(v)));
    }
}

/// Go `pgKey`.
pub fn key(s: &str) -> Msg {
    let k = |t| tea::msg(KeyMsg::new(t));
    match s {
        "up" => k(tea::KEY_UP),
        "down" => k(tea::KEY_DOWN),
        "left" => k(tea::KEY_LEFT),
        "right" => k(tea::KEY_RIGHT),
        "enter" => k(tea::KEY_ENTER),
        "esc" => k(tea::KEY_ESC),
        "tab" => k(tea::KEY_TAB),
        "shift+tab" => k(tea::KEY_SHIFT_TAB),
        "pgup" => k(tea::KEY_PG_UP),
        "pgdown" => k(tea::KEY_PG_DOWN),
        "home" => k(tea::KEY_HOME),
        "end" => k(tea::KEY_END),
        "backspace" => k(tea::KEY_BACKSPACE),
        "delete" => k(tea::KEY_DELETE),
        "space" => k(tea::KEY_SPACE),
        "f1" => k(tea::KEY_F1),
        "ctrl+u" => k(tea::KEY_CTRL_U),
        "ctrl+d" => k(tea::KEY_CTRL_D),
        "ctrl+x" => k(tea::KEY_CTRL_X),
        "alt+a" => {
            let mut km = KeyMsg::runes("a");
            km.alt = true;
            tea::msg(km)
        }
        "winsize" => tea::msg(WindowSizeMsg {
            width: 120,
            height: 40,
        }),
        other => tea::msg(KeyMsg::runes(other)),
    }
}

/// Go `pgCmd`: runs an immediate command and describes its message with
/// `desc`; timers (cursor blinks) are reported as `timeout` like the Go
/// generator's 100ms guard.
pub fn describe_cmd(cmd: Option<Cmd>, desc: &dyn Fn(&Msg) -> Option<String>) -> String {
    let Some(cmd) = cmd else {
        return "nil".into();
    };
    if cmd.is_tick() {
        return "timeout".into();
    }
    match cmd.call() {
        None => "nilmsg".into(),
        Some(m) => {
            if let Some(b) = m.downcast_ref::<BatchMsg>() {
                return format!("batch({})", b.0.len());
            }
            desc(&m).unwrap_or_else(|| panic!("undescribed message"))
        }
    }
}

/// One theme x mode x profile combination (Go `pgCombo`).
#[derive(Clone, Copy)]
pub struct Combo {
    pub name: &'static str,
    pub mode: ColorMode,
    pub pname: &'static str,
    pub prof: Profile,
}

impl Combo {
    /// Applies the profile and returns the theme and the key prefix.
    pub fn apply(&self) -> (Theme, String) {
        lipgloss::set_has_dark_background(true);
        lipgloss::set_color_profile(self.prof);
        (
            get_theme(self.name).with_color_mode(self.mode),
            format!("{}/{}/{}/", self.name, self.mode, self.pname),
        )
    }
    /// Go `full`: the combo that records every intermediate step.
    pub fn full(&self) -> bool {
        self.prof == Profile::Ascii && self.mode == ColorMode::Dark && self.name == THEME_MODERN
    }
}

/// Go `pgCombos`.
pub fn combos() -> Vec<Combo> {
    let mut out = Vec::new();
    for name in [THEME_MODERN, THEME_LCARS, THEME_CRT] {
        for mode in [ColorMode::Dark, ColorMode::Light] {
            for (pname, prof) in [("Ascii", Profile::Ascii), ("TrueColor", Profile::TrueColor)] {
                out.push(Combo {
                    name,
                    mode,
                    pname,
                    prof,
                });
            }
        }
    }
    out.push(Combo {
        name: THEME_MODERN,
        mode: ColorMode::Dark,
        pname: "ANSI256",
        prof: Profile::ANSI256,
    });
    out.push(Combo {
        name: THEME_LCARS,
        mode: ColorMode::Light,
        pname: "ANSI256",
        prof: Profile::ANSI256,
    });
    out
}

/// Byte-for-byte comparison against the Go reference; returns
/// `(lines, bytes)` compared.
pub fn compare(got: &[String], data: &str) -> (usize, usize) {
    let want: Vec<&str> = data.lines().collect();
    for (i, (g, w)) in got.iter().zip(want.iter()).enumerate() {
        assert_eq!(g, w, "parity mismatch at line {}", i + 1);
    }
    assert_eq!(got.len(), want.len(), "parity line count mismatch");
    (want.len(), data.len())
}
