//! Shared helpers for the sectorgrid / statuspanel / commandbar / targetlock /
//! damageschematic / galacticchart parity tests. Mirrors the helper block
//! appended to each `zz_paritygen_test.go` (`pgVariants`, `pgEach`, `pgLite`,
//! `pgRot`, `pgKey`, `pgGames`). Included with `#[path]` by each test module.
#![allow(dead_code)]

use std::rc::Rc;

use sst_engine::gofmt::quote;
use sst_engine::{
    build_kobayashi_maru, build_mutara_nebula, build_starbase_siege, default_rules_for_profile,
    ptr, sync_quadrant_enemies, ConditionType, DeviceID, DifficultyProfile, GameLength, GameRules,
    GameState, PlasmaTorpedo, SkillLevel, TholianWebSegment, ANIM_SPEED_OFF,
};
use sst_tuikit::lipgloss;
use sst_tuikit::tea::*;
use sst_tuikit::termenv::Profile;

use crate::theme::{self, detect, ColorMode, Theme};

/// Output lines in the generator's format (`key\t%q`).
pub struct Out {
    pub lines: Vec<String>,
    rot: usize,
}

impl Out {
    pub fn new() -> Out {
        lipgloss::set_has_dark_background(true);
        lipgloss::set_color_profile(Profile::Ascii);
        detect::set_detect_impl(Some(Rc::new(|| panic!("background detection called"))));
        Out {
            lines: Vec::new(),
            rot: 0,
        }
    }

    pub fn w(&mut self, k: &str, v: &str) {
        self.lines.push(format!("{k}\t{}", quote(v)));
    }

    /// `pgEach`.
    pub fn each(&mut self, key: &str, mut render: impl FnMut(Theme) -> String) {
        for (i, (vk, th)) in variants().into_iter().enumerate() {
            let mut profs = vec![("ascii", Profile::Ascii), ("truecolor", Profile::TrueColor)];
            if i == 0 {
                profs.push(("ansi256", Profile::ANSI256));
            }
            for (pk, p) in profs {
                lipgloss::set_color_profile(p);
                let v = render(th);
                self.w(&format!("{key}/{vk}/{pk}"), &v);
            }
        }
        lipgloss::set_color_profile(Profile::Ascii);
    }

    /// `pgLite`.
    pub fn lite(&mut self, key: &str, mut render: impl FnMut(Theme) -> String) {
        let vs = variants();
        for (vi, pk, p) in [
            (0, "truecolor", Profile::TrueColor),
            (3, "truecolor", Profile::TrueColor),
            (4, "ascii", Profile::Ascii),
        ] {
            lipgloss::set_color_profile(p);
            let v = render(vs[vi].1);
            self.w(&format!("{key}/{}/{pk}", vs[vi].0), &v);
        }
        lipgloss::set_color_profile(Profile::Ascii);
    }

    /// `pgRot`.
    pub fn rot(&mut self, key: &str, mut render: impl FnMut(Theme) -> String) {
        let vs = variants();
        let (vk, th) = vs[self.rot % vs.len()].clone();
        self.rot += 1;
        lipgloss::set_color_profile(Profile::TrueColor);
        let v = render(th);
        self.w(&format!("{key}/{vk}/truecolor"), &v);
        lipgloss::set_color_profile(Profile::Ascii);
    }
}

/// Compares generated lines with the recorded reference, byte for byte.
pub fn check(data: &str, got: &[String]) {
    let want: Vec<&str> = data.lines().collect();
    for (i, (w, g)) in want.iter().zip(got.iter()).enumerate() {
        if w != g {
            let wk = w.split('\t').next().unwrap_or("");
            let gk = g.split('\t').next().unwrap_or("");
            panic!(
                "line {} differs (want key {wk}, got key {gk})\nwant: {w}\n got: {g}",
                i + 1
            );
        }
    }
    assert_eq!(want.len(), got.len(), "line count");
    let bytes: usize = want.iter().map(|l| l.len() + 1).sum();
    eprintln!("parity: {} lines, {} bytes compared", want.len(), bytes);
}

/// `pgVariants`.
pub fn variants() -> Vec<(String, Theme)> {
    let mut out = Vec::new();
    for name in ["modern", "lcars", "crt"] {
        for mode in [ColorMode::Dark, ColorMode::Light] {
            out.push((
                format!("{name}-{}", mode.as_str()),
                theme::get_theme(name).with_color_mode(mode),
            ));
        }
    }
    out
}

/// `pgKey`.
pub fn key(s: &str) -> KeyMsg {
    match s {
        "enter" => KeyMsg::new(KEY_ENTER),
        "esc" => KeyMsg::new(KEY_ESC),
        "tab" => KeyMsg::new(KEY_TAB),
        "shift+tab" => KeyMsg::new(KEY_SHIFT_TAB),
        "up" => KeyMsg::new(KEY_UP),
        "down" => KeyMsg::new(KEY_DOWN),
        "left" => KeyMsg::new(KEY_LEFT),
        "right" => KeyMsg::new(KEY_RIGHT),
        "backspace" => KeyMsg::new(KEY_BACKSPACE),
        "space" => KeyMsg::new(KEY_SPACE),
        "home" => KeyMsg::new(KEY_HOME),
        "end" => KeyMsg::new(KEY_END),
        "delete" => KeyMsg::new(KEY_DELETE),
        "ctrl+a" => KeyMsg::new(KEY_CTRL_A),
        "ctrl+e" => KeyMsg::new(KEY_CTRL_E),
        "ctrl+u" => KeyMsg::new(KEY_CTRL_U),
        "ctrl+k" => KeyMsg::new(KEY_CTRL_K),
        "ctrl+w" => KeyMsg::new(KEY_CTRL_W),
        "alt+b" => KeyMsg {
            alt: true,
            ..KeyMsg::runes("b")
        },
        "f2" => KeyMsg::new(KEY_F2),
        _ => KeyMsg::runes(s),
    }
}

/// `pgKey` boxed as a message.
pub fn key_msg(s: &str) -> Msg {
    msg(key(s))
}

/// `pgHot`.
pub fn hot(g: &mut GameState, min: i32) {
    for r in 1..=8 {
        for c in 1..=8 {
            if g.galaxy_chart[r][c] / 100 >= min {
                let sec = g.enterprise.sector;
                g.populate_quadrant([r as i32, c as i32], sec);
                return;
            }
        }
    }
    let (q, s) = (g.enterprise.quad, g.enterprise.sector);
    g.populate_quadrant(q, s);
}

/// `pgAdvRules`.
pub fn adv_rules() -> GameRules {
    let mut r = default_rules_for_profile(&DifficultyProfile::normal());
    r.adversaries = true;
    r.klingon_cloak = true;
    r.spatial_anomalies = true;
    r
}

fn populate_own(g: &mut GameState) {
    let (q, s) = (g.enterprise.quad, g.enterprise.sector);
    g.populate_quadrant(q, s);
}

/// `pgGames`.
pub fn games() -> Vec<(String, GameState)> {
    let mut out = Vec::new();
    for seed in [1i64, 7, 42, 99, 314, 2024, 12345, 777] {
        let skill = SkillLevel((1 + seed % 5) as i32);
        let length = GameLength((1 + seed % 3) as i32);
        let mut g = GameState::new_game(seed, skill, length);
        populate_own(&mut g);
        out.push((format!("seed{seed}"), g));
        let mut h = GameState::new_game(seed, skill, length);
        hot(&mut h, 2);
        out.push((format!("hot{seed}"), h));
    }
    for (i, prof) in [
        DifficultyProfile::casual(),
        DifficultyProfile::hardcore(),
        DifficultyProfile::nightmare(),
        DifficultyProfile::expert(),
        DifficultyProfile::emeritus(),
    ]
    .into_iter()
    .enumerate()
    {
        let mut g = GameState::new_game_with_options(
            5 + i as i64,
            SkillLevel::Good,
            GameLength::Medium,
            default_rules_for_profile(&prof),
        );
        hot(&mut g, 1);
        out.push((format!("profile-{}", prof.as_str()), g));
    }
    for seed in [11i64, 23, 58, 64, 90] {
        let mut g = GameState::new_game_with_options(
            seed,
            SkillLevel::Expert,
            GameLength::Long,
            adv_rules(),
        );
        hot(&mut g, 1);
        out.push((format!("adv{seed}"), g));
    }
    out.push(("kobayashi".into(), build_kobayashi_maru(3)));
    out.push(("mutara".into(), build_mutara_nebula(4)));
    out.push(("siege".into(), build_starbase_siege(5)));

    let mut hz =
        GameState::new_game_with_options(42, SkillLevel::Good, GameLength::Medium, adv_rules());
    hot(&mut hz, 2);
    let target = hz.enterprise.sector;
    hz.current_quad.plasma_torpedoes = vec![
        ptr(PlasmaTorpedo {
            id: 1,
            source_id: 1,
            sector: [2, 2],
            energy: 300.0,
            target_sector: target,
            ..Default::default()
        }),
        ptr(PlasmaTorpedo {
            id: 2,
            source_id: 1,
            sector: [7, 6],
            energy: 0.0,
            ..Default::default()
        }),
        ptr(PlasmaTorpedo {
            id: 3,
            source_id: 2,
            sector: [5, 8],
            energy: 120.5,
            ..Default::default()
        }),
    ]
    .into();
    for (i, c) in [[1, 1], [1, 2], [1, 3], [2, 3], [3, 3]]
        .into_iter()
        .enumerate()
    {
        hz.current_quad.web_segments.push(ptr(TholianWebSegment {
            coord: c,
            strength: 50.0 * i as f64,
        }));
    }
    if let Some(Some(e)) = hz.current_quad.enemies.first() {
        e.borrow_mut().is_cloaked = true;
    }
    sync_quadrant_enemies(&mut hz.current_quad);
    out.push(("hazards".into(), hz));

    let mut dm = GameState::new_game(99, SkillLevel::Fair, GameLength::Short);
    hot(&mut dm, 1);
    for (i, d) in dm.enterprise.devices.iter_mut().enumerate() {
        *d = i as f64 * 0.75;
    }
    dm.enterprise.energy = 1234.5;
    dm.enterprise.shields = 2600.0;
    dm.enterprise.torpedoes = 3;
    dm.enterprise.condition = ConditionType::Red;
    out.push(("damaged".into(), dm));

    let mut dk = GameState::new_game(314, SkillLevel::Good, GameLength::Medium);
    populate_own(&mut dk);
    dk.enterprise.condition = ConditionType::Docked;
    dk.enterprise.devices[DeviceID::LRSensors.as_i32() as usize] = 3.5;
    dk.enterprise.devices[DeviceID::Computer.as_i32() as usize] = 1.25;
    out.push(("docked".into(), dk));

    let mut lrs = GameState::new_game(2024, SkillLevel::Good, GameLength::Medium);
    hot(&mut lrs, 1);
    lrs.enterprise.devices[DeviceID::LRSensors.as_i32() as usize] = 0.5;
    lrs.enterprise.condition = ConditionType::Yellow;
    lrs.enterprise.energy = -10.0;
    lrs.enterprise.torpedoes = 0;
    out.push(("lrs-degraded".into(), lrs));

    let mut off = GameState::new_game(7, SkillLevel::Good, GameLength::Medium);
    off.rules.anim_speed = ANIM_SPEED_OFF;
    off.rules.sensor_degradation = false;
    off.enterprise.condition = ConditionType::Red;
    off.enterprise.quad = [8, 8];
    off.enterprise.devices[DeviceID::LRSensors.as_i32() as usize] = 1.0;
    out.push(("animoff-corner".into(), off));
    out
}
