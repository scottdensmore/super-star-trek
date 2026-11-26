//! Port of the first half of `pkg/tui/model_test.go` (tests starting before
//! line 1450). Wall-clock double-click timing is replaced by an injected
//! `FixedClock` (Go relies on two `Update` calls being < 400ms apart).

use std::rc::Rc;
use std::sync::{Mutex, MutexGuard};

use sst_engine::gotime::{Clock, FixedClock, GoTime};
use sst_engine::{
    game_ref, persist, ptr, ConditionType, Coord, DeviceID, EntityType, GameLength, GameRef,
    GameState, Klingon, SkillLevel,
};
use sst_tui::components::{commandbar, commandpalette, galacticchart, savebrowser, targetlock};
use sst_tui::theme::{self, CrtTheme, LcarsTheme, ModernTheme, Theme};
use sst_tui::*;
use sst_tuikit::lipgloss;
use sst_tuikit::tea::{
    self, Cmd, KeyMsg, KeyType, MouseMsg, Msg, QuitMsg, WindowSizeMsg, KEY_CTRL_C, KEY_CTRL_G,
    KEY_CTRL_O, KEY_CTRL_P, KEY_DOWN, KEY_ENTER, KEY_ESC, KEY_F2, KEY_RIGHT, KEY_RUNES,
    MOUSE_ACTION_PRESS, MOUSE_BUTTON_LEFT,
};
use sst_tuikit::termenv;

static ENV_LOCK: Mutex<()> = Mutex::new(());

struct Env {
    _g: MutexGuard<'static, ()>,
    tmp: std::path::PathBuf,
}

impl Drop for Env {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.tmp);
    }
}

impl Env {
    fn dir(&self, name: &str) -> String {
        let d = self.tmp.join(name);
        std::fs::create_dir_all(&d).unwrap();
        d.to_str().unwrap().to_string()
    }
}

/// Isolated HOME/XDG_CONFIG_HOME, test audio, Ascii profile, fixed detection.
fn env() -> Env {
    let g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    sst_audio::set_test_mode(true);
    let tmp = std::env::temp_dir().join(format!(
        "sst-go-model-a-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&tmp);
    let home = tmp.join("home");
    std::fs::create_dir_all(&home).unwrap();
    std::env::set_var("HOME", &home);
    std::env::set_var("XDG_CONFIG_HOME", home.join(".config"));
    lipgloss::set_color_profile(termenv::Ascii);
    lipgloss::set_has_dark_background(true);
    theme::detect::set_fixed_detection(true);
    Env { _g: g, tmp }
}

fn new_game() -> GameRef {
    game_ref(GameState::new_game(
        12345,
        SkillLevel::Good,
        GameLength::Medium,
    ))
}

fn dt() -> Theme {
    theme::default_theme()
}

fn modern() -> Theme {
    Theme::from(ModernTheme::default())
}

fn key(t: KeyType) -> Msg {
    tea::msg(KeyMsg::new(t))
}

fn runes(s: &str) -> Msg {
    let mut k = KeyMsg::new(KEY_RUNES);
    k.runes = s.chars().collect();
    tea::msg(k)
}

fn size(w: i32, h: i32) -> Msg {
    tea::msg(WindowSizeMsg {
        width: w,
        height: h,
    })
}

fn submit(s: &str) -> Msg {
    tea::msg(commandbar::CommandSubmittedMsg { text: s.into() })
}

fn click(x: i32, y: i32) -> Msg {
    tea::msg(MouseMsg {
        x,
        y,
        button: MOUSE_BUTTON_LEFT,
        action: MOUSE_ACTION_PRESS,
        ..Default::default()
    })
}

fn up(m: &Model, msg: Msg) -> Model {
    m.update_model(msg).0
}

fn is_quit(cmd: Option<Cmd>) -> bool {
    cmd.and_then(Cmd::call)
        .is_some_and(|m| m.downcast_ref::<QuitMsg>().is_some())
}

fn any_msg(m: &Model, f: impl Fn(&str) -> bool) -> bool {
    m.command_bar.messages().iter().any(|s| f(s))
}

fn add_klingon(g: &GameRef, id: i32, sector: Coord, energy: f64, grid: bool) {
    let mut g = g.borrow_mut();
    g.current_quad.klingons = vec![ptr(Klingon {
        id,
        sector,
        energy,
        ..Default::default()
    })]
    .into();
    if grid {
        g.current_quad.grid[sector[0] as usize][sector[1] as usize] = EntityType::Klingon;
    }
}

fn fixed_clock() -> Rc<dyn Clock> {
    Rc::new(FixedClock(GoTime::date(2025, 1, 1, 12, 0, 0, 0, 0)))
}

#[test]
fn model_size_guard() {
    let _e = env();
    let mut m = new_model(Some(new_game()), dt());
    m = up(&m, size(70, 20));
    let view = m.view();
    assert!(view.contains("TERMINAL WINDOW TOO SMALL"), "{view}");
    m = up(&m, size(79, 25));
    assert!(m.view().contains("TERMINAL WINDOW TOO SMALL"));
    m = up(&m, size(85, 23));
    assert!(m.view().contains("TERMINAL WINDOW TOO SMALL"));
    m = up(&m, size(80, 24));
    let view = m.view();
    assert!(
        !view.contains("TOO SMALL") && view.contains("<E>"),
        "{view}"
    );
}

#[test]
fn model_responsive_wide_dashboard() {
    let _e = env();
    let mut m = new_model(Some(new_game()), dt());
    m = up(&m, size(80, 24));
    assert!(!m.view().contains("GALACTIC STAR CHART [K-B-S]"));
    m = up(&m, size(130, 30));
    let v = m.view();
    assert!(v.contains("GALACTIC STAR CHART [K-B-S]"));
    assert!(v.contains("Klingons Left:"));
    assert!(v.contains("Starbases:"));
    assert!(v.contains("Explored:"));
}

#[test]
fn model_theme_toggle() {
    let _e = env();
    let mut m = new_model(Some(new_game()), dt());
    assert_eq!(m.theme.unwrap().name(), "modern");
    m = up(&m, key(KEY_F2));
    assert_eq!(m.theme.unwrap().name(), "lcars");
    assert_eq!(m.grid.theme().name(), "lcars");
    assert_eq!(m.status.theme().name(), "lcars");
    assert_eq!(m.command_bar.theme().name(), "lcars");
    m = up(&m, key(KEY_F2));
    assert_eq!(m.theme.unwrap().name(), "crt");
    m = up(&m, key(KEY_F2));
    assert_eq!(m.theme.unwrap().name(), "modern");
}

#[test]
fn model_quit_handling() {
    let _e = env();
    let mut m = new_model(Some(new_game()), dt());
    let (_, cmd) = m.update_model(key(KEY_CTRL_C));
    assert!(is_quit(cmd), "expected quit on Ctrl+C");
    let (_, cmd) = m.update_model(key(KEY_ESC));
    assert!(is_quit(cmd), "expected quit on Esc with empty input");

    m.command_bar.set_value("some text");
    let (m2, cmd) = m.update_model(key(KEY_ESC));
    m = m2;
    assert!(
        cmd.is_none(),
        "expected no command on Esc when input is non-empty"
    );
    assert_eq!(m.command_bar.value(), "");

    let (_, cmd) = m.update_model(submit("quit"));
    assert!(is_quit(cmd));
    let (_, cmd) = m.update_model(submit("exit"));
    assert!(is_quit(cmd));
}

#[test]
fn model_command_submitted_actions() {
    let _e = env();
    let g = new_game();
    let mut m = new_model(Some(g.clone()), dt());
    let (e0, s0) = {
        let g = g.borrow();
        (g.enterprise.energy, g.enterprise.shields)
    };
    let (m2, cmd) = m.update_model(submit("she 500"));
    m = m2;
    assert!(cmd.is_none(), "unexpected command from she 500");
    assert_eq!(g.borrow().enterprise.shields, 500.0);
    assert_eq!(g.borrow().enterprise.energy, e0 - (500.0 - s0));
    assert!(any_msg(&m, |s| s.contains("Shield") || s.contains("Energy")));

    m = up(&m, submit("she 999999"));
    assert!(any_msg(&m, |s| {
        let l = s.to_lowercase();
        l.contains("energy") || l.contains("shield")
    }));
}

#[test]
fn model_command_submitted_special() {
    let _e = env();
    let mut m = new_model(Some(new_game()), dt());
    m = up(&m, submit("theme"));
    assert_eq!(m.theme.unwrap().name(), "lcars");
    m = up(&m, submit("theme crt"));
    assert_eq!(m.theme.unwrap().name(), "crt");
    m = up(&m, submit("help"));
    assert!(any_msg(&m, |s| {
        let l = s.to_lowercase();
        l.contains("command") || l.contains("nav")
    }));
    m = up(&m, submit("xyz"));
    assert!(any_msg(&m, |s| {
        let l = s.to_lowercase();
        l.contains("unknown") || l.contains("xyz")
    }));
}

#[test]
fn model_key_forwarding() {
    let _e = env();
    let mut m = new_model(Some(new_game()), dt());
    for r in ["p", "h", "a"] {
        m = up(&m, runes(r));
    }
    assert_eq!(m.command_bar.value(), "pha");
}

#[test]
fn model_init() {
    let _e = env();
    let m = new_model(Some(new_game()), dt());
    assert!(
        m.init().is_some(),
        "expected non-nil Init cmd for cursor blink"
    );
}

#[test]
fn model_view_structure() {
    let _e = env();
    let g = new_game();
    let mut m = new_model(Some(g.clone()), dt());
    m = up(&m, size(80, 24));
    let view = m.view();
    for s in [
        "SUPER STAR TREK",
        "USS ENTERPRISE",
        "<E>",
        "CONDITION GREEN",
        "Stardate:",
        "COMMAND>",
    ] {
        assert!(view.contains(s), "view missing {s}:\n{view}");
    }

    let mut mp = m.clone();
    mp.active_modal = MODAL_COMMAND_PALETTE;
    let pv = mp.view();
    for s in [
        "COMMAND PALETTE",
        "SUPER STAR TREK",
        "USS ENTERPRISE",
        "COMMAND>",
    ] {
        assert!(pv.contains(s), "palette view missing {s}:\n{pv}");
    }

    add_klingon(&g, 1, [4, 7], 300.0, true);
    let mt = up(&m, runes("t"));
    assert_eq!(mt.active_modal, MODAL_TARGET_LOCK);
    let tv = mt.view();
    for s in [
        "TACTICAL TARGET LOCK",
        "SUPER STAR TREK",
        "USS ENTERPRISE",
        "COMMAND>",
    ] {
        assert!(tv.contains(s), "target view missing {s}:\n{tv}");
    }
}

#[test]
fn model_nil_game_graceful() {
    let _e = env();
    let mut m = new_model(None, None);
    m = up(&m, size(80, 24));
    let view = m.view();
    assert!(view.contains("SUPER STAR TREK"), "{view}");
    m = up(&m, submit("she 100"));
    assert!(any_msg(&m, |s| {
        let l = s.to_lowercase();
        l.contains("no active game") || l.contains("error")
    }));
}

#[test]
fn model_selected_sector_view_reticle() {
    let _e = env();
    let mut m = new_model(Some(new_game()), dt());
    m = up(&m, size(80, 24));
    assert!(!m.view().contains("[.]"));
    m.selected_sector = [3, 4];
    let v = m.view();
    assert!(
        v.contains("[.]") || v.contains("[E]") || v.contains("[K]"),
        "{v}"
    );
}

#[test]
fn model_open_target_lock_hotkey() {
    let _e = env();
    let g = new_game();
    g.borrow_mut().enterprise.sector = [4, 4];
    add_klingon(&g, 1, [4, 7], 300.0, true);
    let mut m = new_model(Some(g), dt());
    m = up(&m, size(80, 24));
    m = up(&m, runes("t"));
    assert_eq!(m.active_modal, MODAL_TARGET_LOCK);
    assert_eq!(
        m.target_lock.current_target().map(|t| t.klingon_id),
        Some(1)
    );
    assert!(m.view().contains("TACTICAL TARGET LOCK"));
}

#[test]
fn model_open_target_lock_no_enemies() {
    let _e = env();
    let g = new_game();
    g.borrow_mut().current_quad.klingons = Default::default();
    let mut m = new_model(Some(g), dt());
    m = up(&m, size(80, 24));
    m = up(&m, runes("T"));
    assert_eq!(m.active_modal, MODAL_NONE);
    assert!(any_msg(&m, |s| s
        .contains("Sensors detect no hostile targets in sector.")));
}

#[test]
fn model_open_command_palette_hotkey() {
    let _e = env();
    let mut m = new_model(Some(new_game()), dt());
    m = up(&m, size(80, 24));
    m = up(&m, key(KEY_CTRL_P));
    assert_eq!(m.active_modal, MODAL_COMMAND_PALETTE);
    assert!(m.view().contains("COMMAND PALETTE"));
    m.active_modal = MODAL_NONE;
    m = up(&m, runes("/"));
    assert_eq!(m.active_modal, MODAL_COMMAND_PALETTE);
}

#[test]
fn model_modal_dismissal_esc() {
    let _e = env();
    let g = new_game();
    add_klingon(&g, 1, [4, 7], 300.0, false);
    let mut m = new_model(Some(g), dt());
    m = up(&m, runes("t"));
    assert_eq!(m.active_modal, MODAL_TARGET_LOCK);
    m = up(&m, key(KEY_ESC));
    assert_eq!(m.active_modal, MODAL_NONE);
    assert!(m.command_bar.focused());
    m = up(&m, key(KEY_CTRL_P));
    assert_eq!(m.active_modal, MODAL_COMMAND_PALETTE);
    m = up(&m, key(KEY_ESC));
    assert_eq!(m.active_modal, MODAL_NONE);
    assert!(m.command_bar.focused());
}

#[test]
fn model_mouse_click_select_sector() {
    let _e = env();
    let mut m = new_model(Some(new_game()), dt());
    m.set_clock(fixed_clock());
    m = up(&m, click(16, 5));
    assert_eq!(m.selected_sector, [3, 4]);
    assert!(any_msg(&m, |s| s.contains("Target sector selected: [3, 4]")));
}

#[test]
fn model_mouse_click_klingon_opens_hud() {
    let _e = env();
    let g = new_game();
    add_klingon(&g, 2, [3, 4], 300.0, true);
    let mut m = new_model(Some(g), dt());
    m.set_clock(fixed_clock());
    m = up(&m, click(16, 5));
    assert_eq!(m.active_modal, MODAL_TARGET_LOCK);
    assert_eq!(
        m.target_lock.current_target().map(|t| t.coord),
        Some([3, 4])
    );
}

fn impulse_game() -> GameRef {
    let g = new_game();
    {
        let mut gs = g.borrow_mut();
        gs.enterprise.sector = [4, 4];
        gs.current_quad.grid[4][4] = EntityType::Enterprise;
        gs.current_quad.grid[5][5] = EntityType::Empty;
    }
    g
}

#[test]
fn model_mouse_double_click_impulse_move() {
    let _e = env();
    let g = impulse_game();
    let mut m = new_model(Some(g.clone()), dt());
    m.set_clock(fixed_clock());
    m = up(&m, click(20, 7));
    assert_eq!(m.selected_sector, [5, 5]);
    let _m = up(&m, click(20, 7));
    assert_eq!(g.borrow().enterprise.sector, [5, 5]);
}

#[test]
fn model_mouse_double_click_dock() {
    let _e = env();
    let g = new_game();
    {
        let mut gs = g.borrow_mut();
        gs.enterprise.sector = [4, 4];
        gs.current_quad.grid[4][4] = EntityType::Enterprise;
        gs.current_quad.starbase = Some([4, 5]);
        gs.current_quad.grid[4][5] = EntityType::Starbase;
    }
    let mut m = new_model(Some(g.clone()), dt());
    m.set_clock(fixed_clock());
    m = up(&m, click(20, 6));
    let _ = up(&m, click(20, 6));
    assert_eq!(g.borrow().enterprise.condition, ConditionType::Docked);
}

#[test]
fn model_modal_action_dispatch() {
    let _e = env();
    let g = new_game();
    {
        let mut gs = g.borrow_mut();
        gs.enterprise.sector = [4, 4];
        gs.enterprise.torpedoes = 10;
    }
    add_klingon(&g, 1, [4, 7], 1000.0, true);
    let mut m = new_model(Some(g.clone()), dt());
    m.active_modal = MODAL_TARGET_LOCK;
    m = up(
        &m,
        tea::msg(targetlock::FireTorpedoMsg {
            target: [4, 7],
            bearing: 1.0,
        }),
    );
    assert_eq!(m.active_modal, MODAL_NONE);
    assert_eq!(g.borrow().enterprise.torpedoes, 9);
    assert!(m.command_bar.focused());

    m.active_modal = MODAL_TARGET_LOCK;
    let e0 = g.borrow().enterprise.energy;
    m = up(&m, tea::msg(targetlock::FirePhasersMsg { energy: 200.0 }));
    assert_eq!(m.active_modal, MODAL_NONE);
    assert!(g.borrow().enterprise.energy < e0);
}

#[test]
fn model_modal_message_routing_palette() {
    let _e = env();
    let mut m = new_model(Some(new_game()), dt());
    m.active_modal = MODAL_COMMAND_PALETTE;
    m = up(
        &m,
        tea::msg(commandpalette::CommandSelectedMsg {
            command_prefix: "tor ".into(),
            parameterized: true,
        }),
    );
    assert_eq!(m.active_modal, MODAL_NONE);
    assert_eq!(m.command_bar.value(), "tor ");
    assert!(m.command_bar.focused());

    m.active_modal = MODAL_COMMAND_PALETTE;
    let (_, cmd) = m.update_model(tea::msg(commandpalette::CommandSelectedMsg {
        command_prefix: "quit".into(),
        parameterized: false,
    }));
    assert!(cmd.is_some(), "expected quit cmd");

    m.active_modal = MODAL_COMMAND_PALETTE;
    m = up(&m, tea::msg(commandpalette::ClosePaletteMsg));
    assert_eq!(m.active_modal, MODAL_NONE);
    assert!(m.command_bar.focused());
}

#[test]
fn model_mouse_clicks_separated_by_time_do_not_double_click() {
    let _e = env();
    let g = impulse_game();
    let mut m = new_model(Some(g.clone()), dt());
    let clock = fixed_clock();
    m.set_clock(clock.clone());
    m = up(&m, click(20, 7));
    // Simulate passage of time > 400ms.
    let now = clock.now();
    m.last_click_time = GoTime::from_unix(now.unix - 1, now.nsec + 500_000_000, now.offset);
    let _m = up(&m, click(20, 7));
    assert_eq!(g.borrow().enterprise.sector, [4, 4]);
}

#[test]
fn model_open_command_palette_with_input_buffer() {
    let _e = env();
    let mut m = new_model(Some(new_game()), dt());
    m.command_bar.set_value("nav 1.0");
    m = up(&m, key(KEY_CTRL_P));
    assert_eq!(m.active_modal, MODAL_COMMAND_PALETTE);
}

#[test]
fn model_theme_propagation_to_open_overlays() {
    let _e = env();
    let g = new_game();
    add_klingon(&g, 1, [4, 7], 300.0, false);
    let mut m = new_model(Some(g), dt());
    m = up(&m, runes("t"));
    assert_eq!(m.active_modal, MODAL_TARGET_LOCK);
    m = up(&m, key(KEY_F2));
    assert_eq!(m.theme.unwrap().name(), "lcars");
    assert_eq!(m.target_lock.theme().name(), "lcars");
    m = up(&m, key(KEY_ESC));
    m = up(&m, key(KEY_CTRL_P));
    assert_eq!(m.active_modal, MODAL_COMMAND_PALETTE);
    m = up(&m, key(KEY_F2));
    assert_eq!(m.theme.unwrap().name(), "crt");
    assert_eq!(m.command_palette.theme().name(), "crt");
}

#[test]
fn model_mouse_double_click_third_click_does_not_double_click() {
    let _e = env();
    let g = impulse_game();
    let mut m = new_model(Some(g.clone()), dt());
    m.set_clock(fixed_clock());
    m = up(&m, click(20, 7));
    m = up(&m, click(20, 7));
    assert_eq!(g.borrow().enterprise.sector, [5, 5]);
    m = up(&m, click(20, 7));
    assert!(!m.last_click_time.is_zero());
}

fn novice_game() -> GameRef {
    game_ref(GameState::new_game(
        12345,
        SkillLevel::Novice,
        GameLength::Short,
    ))
}

#[test]
fn modal_save_browser_integration() {
    let e = env();
    let dir = e.dir("saves");
    let g = novice_game();
    let save_path = format!("{dir}/TESTSAV.TRK");
    g.borrow_mut().stardate = 3150.0;
    let _ = persist::save_game(&g, &save_path);

    let mut m = new_model(Some(g), dt());
    m.width = 100;
    m.height = 30;
    m.save_browser = savebrowser::new(dt(), &dir);

    m = up(&m, key(KEY_CTRL_O));
    assert_eq!(m.active_modal, MODAL_SAVE_BROWSER);
    m = up(&m, tea::msg(savebrowser::CloseBrowserMsg));
    assert_eq!(m.active_modal, MODAL_NONE);
    m = up(&m, submit("saves"));
    assert_eq!(m.active_modal, MODAL_SAVE_BROWSER);
    m = up(
        &m,
        tea::msg(savebrowser::LoadGameMsg {
            path: save_path.clone(),
        }),
    );
    assert_eq!(m.active_modal, MODAL_NONE);
    assert_eq!(m.game.as_ref().unwrap().borrow().stardate, 3150.0);
    m = up(&m, submit("thaw"));
    assert_eq!(m.active_modal, MODAL_SAVE_BROWSER);
    m.active_modal = MODAL_NONE;
    m = up(&m, submit(&format!("thaw {save_path}")));
    assert_eq!(m.game.as_ref().unwrap().borrow().stardate, 3150.0);
}

#[test]
fn modal_save_browser_theme_propagation() {
    let e = env();
    let dir = e.dir("saves");
    let mut m = new_model(Some(novice_game()), dt());
    m.width = 100;
    m.height = 30;
    m.save_browser = savebrowser::new(dt(), &dir);
    m = up(&m, key(KEY_CTRL_O));
    assert_eq!(m.active_modal, MODAL_SAVE_BROWSER);
    m = up(&m, key(KEY_F2));
    assert_eq!(m.theme.unwrap().name(), "lcars");
    m = up(&m, key(KEY_F2));
    assert_eq!(m.theme.unwrap().name(), "crt");
}

#[test]
fn modal_save_browser_view_overlay() {
    let e = env();
    let dir = e.dir("saves");
    let mut m = new_model(Some(novice_game()), dt());
    m.width = 100;
    m.height = 30;
    m.save_browser = savebrowser::new(dt(), &dir);
    m.active_modal = MODAL_SAVE_BROWSER;
    let v = m.view();
    assert!(v.contains("SAVED MISSIONS"), "{v}");
    assert!(v.contains("SUPER STAR TREK"), "{v}");
}

#[test]
fn modal_save_browser_thaw_error() {
    let _e = env();
    let mut m = new_model(Some(novice_game()), dt());
    m = up(&m, submit("thaw /path/that/does/not/exist.trk"));
    assert!(any_msg(&m, |s| s.contains("Failed to thaw")));
    m.active_modal = MODAL_SAVE_BROWSER;
    m = up(
        &m,
        tea::msg(savebrowser::LoadGameMsg {
            path: "/path/that/does/not/exist.trk".into(),
        }),
    );
    assert_eq!(m.active_modal, MODAL_NONE);
    assert!(any_msg(&m, |s| s.contains("Failed to thaw")));
}

#[test]
fn model_render_header_single_line_no_wrap() {
    let _e = env();
    let themes = [
        modern(),
        Theme::from(LcarsTheme::default()),
        Theme::from(CrtTheme::default()),
    ];
    for th in themes {
        for w in [80, 90, 100, 120] {
            let mut m = new_model(None, th);
            m.width = w;
            m.height = 24;
            let header = m.render_header();
            assert!(
                !header.contains('\n'),
                "{}_w{w}: multi-line header",
                th.name()
            );
            let tag = format!("[Theme: {} (F2)]", th.name().to_uppercase());
            assert!(header.contains(&tag), "{}_w{w}: {header}", th.name());
            assert_eq!(lipgloss::width(&header), w, "{}_w{w}", th.name());
        }
    }
}

#[test]
fn model_help_nav_guidance() {
    let _e = env();
    let m = up(&new_model(Some(new_game()), dt()), submit("help nav"));
    let j = m.command_bar.messages().join("\n");
    assert!(j.contains("NAV:"), "{j}");
    assert!(j.contains("Direct Quad: nav q <r c>"), "{j}");
    assert!(j.contains("Direct Sector: nav s <r c>"), "{j}");
    assert!(j.contains("Vector: nav <course> <warp>"), "{j}");
    assert!(j.contains("0.0=East") && j.contains("1.57=North"), "{j}");
}

#[test]
fn model_nav_quadrant_command() {
    let _e = env();
    let m = up(&new_model(Some(new_game()), dt()), submit("nav q 4 5"));
    assert_eq!(m.game.as_ref().unwrap().borrow().enterprise.quad, [4, 5]);
    assert!(any_msg(&m, |s| s.contains("Quadrant [4,5]")));
}

#[test]
fn model_move_quadrant_updates_sector_grid() {
    let _e = env();
    let g = new_game();
    g.borrow_mut().galaxy_chart[3][5] = 214;
    let mut m = new_model(Some(g), modern());
    m = up(&m, size(80, 24));
    m.selected_sector = [7, 7];
    m = up(&m, submit("nav q 3 5"));
    {
        let g = m.game.as_ref().unwrap().borrow();
        assert_eq!(g.enterprise.quad, [3, 5]);
        assert_eq!(m.selected_sector, [0, 0]);
        assert_eq!(g.current_quad.klingons.len(), 2);
        assert_eq!(g.current_quad.stars.len(), 4);
        assert!(g.current_quad.starbase.is_some());
    }
    let v = m.view();
    for s in ["CONDITION RED", "<E>", ">B<", "+K+", " * "] {
        assert!(v.contains(s), "expected {s:?} in view:\n{v}");
    }
}

#[test]
fn model_galactic_chart_hotkey_and_command() {
    let _e = env();
    let mut m = new_model(Some(new_game()), dt());
    m = up(&m, submit("chart"));
    assert_eq!(m.active_modal, MODAL_GALACTIC_CHART);
    m = up(&m, key(KEY_ESC));
    assert_eq!(m.active_modal, MODAL_NONE);
    m = up(&m, key(KEY_CTRL_G));
    assert_eq!(m.active_modal, MODAL_GALACTIC_CHART);
    assert!(m.view().contains("GALACTIC STAR CHART"));
}

#[test]
fn model_galactic_chart_warp_selection() {
    let _e = env();
    let g = new_game();
    g.borrow_mut().enterprise.quad = [3, 3];
    let mut m = new_model(Some(g), dt());
    m = up(&m, submit("chart"));
    m = up(&m, key(KEY_DOWN));
    m = up(&m, key(KEY_RIGHT));
    m = up(&m, key(KEY_RIGHT));
    let (m2, cmd) = m.update_model(key(KEY_ENTER));
    m = m2;
    let msg = cmd.expect("expected cmd on Enter").call().expect("msg");
    let warp = msg
        .downcast_ref::<galacticchart::WarpToQuadrantMsg>()
        .copied()
        .expect("WarpToQuadrantMsg");
    assert_eq!(warp.dest_quad, [4, 5]);
    assert_eq!(warp.warp, 2.2);
    m = up(&m, msg);
    assert_eq!(m.active_modal, MODAL_NONE);
    assert_eq!(m.game.as_ref().unwrap().borrow().enterprise.quad, [4, 5]);
}

#[test]
fn model_help_chart_and_nav_distance() {
    let _e = env();
    let mut m = new_model(Some(new_game()), dt());
    m = up(&m, submit("help chart"));
    let j = m.command_bar.messages().join("\n");
    assert!(j.contains("CHART:") && j.contains("Ctrl+G"), "{j}");
    m = up(&m, submit("help nav"));
    let j = m.command_bar.messages().join("\n");
    assert!(j.contains("Ctrl+G"), "{j}");
}

#[test]
fn model_galactic_chart_single_key_hotkeys() {
    let _e = env();
    let mut m = new_model(Some(new_game()), dt());
    m = up(&m, runes("c"));
    assert_eq!(m.active_modal, MODAL_GALACTIC_CHART);
    m = up(&m, key(KEY_ESC));
    m = up(&m, runes("C"));
    assert_eq!(m.active_modal, MODAL_GALACTIC_CHART);
    m = up(&m, key(KEY_ESC));
    m.command_bar.set_value("do");
    m = up(&m, runes("c"));
    assert_eq!(m.active_modal, MODAL_NONE);
    assert_eq!(m.command_bar.value(), "doc");
}

#[test]
fn model_galactic_chart_theme_propagation() {
    let _e = env();
    let mut m = new_model(Some(new_game()), dt());
    m = up(&m, submit("chart"));
    assert_eq!(m.active_modal, MODAL_GALACTIC_CHART);
    m = up(&m, key(KEY_F2));
    assert_eq!(m.theme.unwrap().name(), "lcars");
    assert_eq!(m.galactic_chart.theme().unwrap().name(), "lcars");
    m = up(&m, key(KEY_F2));
    assert_eq!(m.theme.unwrap().name(), "crt");
    assert_eq!(m.galactic_chart.theme().unwrap().name(), "crt");
}

#[test]
fn model_help_map_command() {
    let _e = env();
    let m = up(&new_model(Some(new_game()), dt()), submit("help map"));
    let j = m.command_bar.messages().join("\n");
    assert!(j.contains("CHART:") && j.contains("Ctrl+G"), "{j}");
}

#[test]
fn model_lr_scan_command() {
    let _e = env();
    let g = new_game();
    {
        let mut gs = g.borrow_mut();
        gs.enterprise.devices[DeviceID::LRSensors.0 as usize] = 0.0;
        for row in gs.chart_discovered.iter_mut().skip(1) {
            for cell in row.iter_mut().skip(1) {
                *cell = false;
            }
        }
    }
    let m = new_model(Some(g.clone()), modern());
    let mut md = m.clone();
    md.handle_command("lrscan");
    let msgs = md.command_bar.messages();
    assert!(
        msgs.last()
            .is_some_and(|s| s.contains("Long-range scan complete")),
        "{msgs:?}"
    );
    {
        let gs = g.borrow();
        let q = gs.enterprise.quad;
        assert!(gs.chart_discovered[q[0] as usize][q[1] as usize]);
    }
    g.borrow_mut().enterprise.devices[DeviceID::LRSensors.0 as usize] = 2.0;
    let mut mdd = md.clone();
    mdd.handle_command("lrscan");
    let msgs = mdd.command_bar.messages();
    assert!(
        msgs.last()
            .is_some_and(|s| s.contains("LONG-RANGE SENSORS DAMAGED")),
        "{msgs:?}"
    );
}

#[test]
fn model_dock_surveillance() {
    let _e = env();
    let g = new_game();
    {
        let mut gs = g.borrow_mut();
        gs.current_quad.starbase = Some([4, 4]);
        gs.current_quad.grid[4][4] = EntityType::Starbase;
        gs.enterprise.sector = [4, 5];
        gs.enterprise.condition = ConditionType::Green;
    }
    let mut m = new_model(Some(g), modern());
    m.handle_command("dock");
    assert!(any_msg(&m, |s| s.contains("surveillance")));
}

#[test]
fn model_dynamic_damage_report() {
    let _e = env();
    let g = new_game();
    {
        let mut gs = g.borrow_mut();
        gs.enterprise.devices[DeviceID::LRSensors.0 as usize] = 3.2;
        gs.enterprise.devices[DeviceID::Computer.0 as usize] = 1.5;
    }
    let mut m = new_model(Some(g), modern());
    m.handle_command("dam");
    assert_eq!(m.active_modal, MODAL_DAMAGE_SCHEMATIC);
    let v = m.damage_schematic.view();
    assert!(v.contains("LRS") && v.contains("Computer"), "{v}");
}

#[test]
fn model_galactic_chart_warp_blocked() {
    let _e = env();
    let g = new_game();
    g.borrow_mut().enterprise.devices[DeviceID::Computer.0 as usize] = 2.0;
    let mut m = new_model(Some(g), modern());
    m.open_galactic_chart();
    assert_eq!(m.active_modal, MODAL_GALACTIC_CHART);
    let md = up(
        &m,
        tea::msg(galacticchart::WarpBlockedMsg {
            reason: "COMPUTER DAMAGED, USE A POCKET CALCULATOR.".into(),
        }),
    );
    assert_eq!(md.active_modal, MODAL_NONE);
    let msgs = md.command_bar.messages();
    assert!(
        msgs.last().is_some_and(|s| s.contains("COMPUTER DAMAGED")),
        "{msgs:?}"
    );
}

#[test]
fn model_galactic_chart_mouse_click() {
    let _e = env();
    let g = new_game();
    g.borrow_mut().enterprise.quad = [3, 3];
    let mut m = new_model(Some(g), dt());
    m.set_clock(fixed_clock());
    m = up(&m, size(80, 24));
    m = up(&m, submit("chart"));
    assert_eq!(m.active_modal, MODAL_GALACTIC_CHART);
    m = up(&m, click(31, 8));
    assert_eq!(m.galactic_chart.cursor(), [2, 3]);
    assert_eq!(m.active_modal, MODAL_GALACTIC_CHART);
    m = up(&m, click(31, 8));
    assert_eq!(m.active_modal, MODAL_NONE);
    assert_eq!(m.game.as_ref().unwrap().borrow().enterprise.quad, [2, 3]);
}
