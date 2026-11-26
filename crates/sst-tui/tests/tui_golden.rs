//! Port of `tests/tui_golden_test.go`: renders the root model in fixed
//! states and compares byte-for-byte with `testdata/golden/tui/*.golden`.
//! Set `SST_UPDATE_GOLDEN=1` to rewrite the golden files.

use std::collections::BTreeMap;
use std::sync::Mutex;

use sst_engine::{
    ptr, ConditionType, DeviceID, EntityType, GameLength, GameRef, GameState, Klingon, SkillLevel,
};
use sst_tui::anim::CellOverride;
use sst_tui::components::savebrowser;
use sst_tui::theme::{self, ColorMode, Theme};
use sst_tui::{Model, MODAL_COMMAND_PALETTE, MODAL_GALACTIC_CHART, MODAL_SAVE_BROWSER};
use sst_tuikit::lipgloss;
use sst_tuikit::tea::{self, KeyMsg, WindowSizeMsg, KEY_F1, KEY_RUNES, KEY_TAB};
use sst_tuikit::termenv;

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn golden_dir() -> String {
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../testdata/golden/tui").to_string()
}

fn compare_or_update(name: &str, actual: &str) {
    let path = format!("{}/{}.golden", golden_dir(), name);
    if std::env::var("SST_UPDATE_GOLDEN").as_deref() == Ok("1") {
        std::fs::write(&path, actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("golden file {path} not found: {e}"));
    if actual != expected {
        let el: Vec<&str> = expected.split('\n').collect();
        let al: Vec<&str> = actual.split('\n').collect();
        let mut msg = format!(
            "golden mismatch for {name}:\nexpected {} lines, got {} lines\n",
            el.len(),
            al.len()
        );
        let mut diffs = 0;
        for i in 0..el.len().max(al.len()) {
            let e = el.get(i).copied().unwrap_or("");
            let a = al.get(i).copied().unwrap_or("");
            if e != a {
                diffs += 1;
                if diffs <= 5 {
                    msg += &format!("line {} diff:\n  want: {:?}\n   got: {:?}\n", i + 1, e, a);
                }
            }
        }
        panic!("{msg}");
    }
}

fn assert_strict_80x24(name: &str, view: &str) {
    let lines: Vec<&str> = view.split('\n').collect();
    assert_eq!(lines.len(), 24, "{name}: expected exact height of 24 lines");
    for (i, l) in lines.iter().enumerate() {
        let w = lipgloss::width(l);
        assert!(
            w <= 80,
            "{name}: line {} exceeds maximum width of 80 (got {w})",
            i + 1
        );
    }
}

/// Runs `f` with an isolated HOME/XDG_CONFIG_HOME and the Ascii profile.
fn with_env(f: impl FnOnce()) {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    sst_audio::set_test_mode(true);
    let tmp = std::env::temp_dir().join(format!(
        "sst-golden-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let home = tmp.join("home");
    std::fs::create_dir_all(&home).unwrap();
    std::env::set_var("HOME", &home);
    std::env::set_var("XDG_CONFIG_HOME", home.join(".config"));
    lipgloss::set_color_profile(termenv::Ascii);
    lipgloss::set_has_dark_background(true);
    theme::detect::set_fixed_detection(true);
    f();
    let _ = std::fs::remove_dir_all(&tmp);
}

fn new_game() -> GameState {
    GameState::new_game(12345, SkillLevel::Good, GameLength::Medium)
}

fn new_test_model_with_game(w: i32, h: i32, g: GameRef, th: Option<Theme>) -> Model {
    let th = match th {
        Some(t) if t.color_mode() == ColorMode::Auto => t.with_color_mode(ColorMode::Dark),
        Some(t) => t,
        None => theme::get_theme("modern").with_color_mode(ColorMode::Dark),
    };
    let m = sst_tui::new_model(Some(g), th);
    let (m, _) = m.update_model(tea::msg(WindowSizeMsg {
        width: w,
        height: h,
    }));
    m
}

fn new_test_model(w: i32, h: i32, th: Theme) -> Model {
    new_test_model_with_game(w, h, sst_engine::game_ref(new_game()), Some(th))
}

fn themed(name: &str, mode: ColorMode) -> Theme {
    theme::get_theme(name).with_color_mode(mode)
}

fn runes(s: &str) -> tea::Msg {
    let mut k = KeyMsg::new(KEY_RUNES);
    k.runes = s.chars().collect();
    tea::msg(k)
}

fn dashboard(name: &str, theme_name: &str, mode: ColorMode) {
    with_env(|| {
        let m = new_test_model(80, 24, themed(theme_name, mode));
        let v = m.view();
        assert_strict_80x24(name, &v);
        compare_or_update(name, &v);
    });
}

#[test]
fn modern_dashboard_80x24() {
    dashboard("modern_dashboard_80x24", "modern", ColorMode::Dark);
}

#[test]
fn lcars_dashboard_80x24() {
    dashboard("lcars_dashboard_80x24", "lcars", ColorMode::Dark);
}

#[test]
fn crt_dashboard_80x24() {
    dashboard("crt_dashboard_80x24", "crt", ColorMode::Dark);
}

#[test]
fn modern_dashboard_80x24_light() {
    dashboard("modern_dashboard_80x24_light", "modern", ColorMode::Light);
}

#[test]
fn lcars_dashboard_80x24_light() {
    dashboard("lcars_dashboard_80x24_light", "lcars", ColorMode::Light);
}

#[test]
fn crt_dashboard_80x24_light() {
    dashboard("crt_dashboard_80x24_light", "crt", ColorMode::Light);
}

#[test]
fn modern_dashboard_100x30() {
    with_env(|| {
        let m = new_test_model(100, 30, themed("modern", ColorMode::Dark));
        compare_or_update("modern_dashboard_100x30", &m.view());
    });
}

#[test]
fn modal_target_lock() {
    with_env(|| {
        let mut g = new_game();
        let k1 = ptr(Klingon {
            id: 1,
            sector: [4, 7],
            energy: 300.0,
            ..Default::default()
        });
        let k2 = ptr(Klingon {
            id: 2,
            sector: [6, 2],
            energy: 250.0,
            ..Default::default()
        });
        g.current_quad.klingons = vec![k1, k2].into();
        g.current_quad.grid[4][7] = EntityType::Klingon;
        g.current_quad.grid[6][2] = EntityType::Klingon;
        let gr = sst_engine::game_ref(g);
        let mut m = new_test_model_with_game(80, 24, gr.clone(), None);
        {
            let g = gr.borrow();
            m.target_lock.set_state(
                g.enterprise.sector,
                g.enterprise.energy,
                g.enterprise.torpedoes,
                &g.current_quad.klingons,
                [0, 0],
            );
        }
        m.active_modal = sst_tui::MODAL_TARGET_LOCK;
        compare_or_update("modal_target_lock", &m.view());
    });
}

#[test]
fn modal_command_palette() {
    with_env(|| {
        let mut m = new_test_model_with_game(80, 24, sst_engine::game_ref(new_game()), None);
        m.command_palette.reset();
        m.active_modal = MODAL_COMMAND_PALETTE;
        compare_or_update("modal_command_palette", &m.view());
    });
}

#[test]
fn modal_save_browser() {
    with_env(|| {
        let dir = std::env::temp_dir().join(format!("sst-golden-saves-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut m = new_test_model_with_game(80, 24, sst_engine::game_ref(new_game()), None);
        m.save_browser = savebrowser::new(themed("modern", ColorMode::Dark), dir.to_str().unwrap());
        m.active_modal = MODAL_SAVE_BROWSER;
        compare_or_update("modal_save_browser", &m.view());
        let _ = std::fs::remove_dir_all(&dir);
    });
}

#[test]
fn modal_galactic_chart() {
    with_env(|| {
        let mut g = new_game();
        g.enterprise.quad = [3, 3];
        g.galaxy_chart[3][3] = 3;
        g.chart_discovered[3][3] = true;
        g.galaxy_chart[2][5] = 105;
        g.chart_discovered[2][5] = true;
        g.galaxy_chart[3][4] = 12;
        g.chart_discovered[3][4] = true;
        let gr = sst_engine::game_ref(g);
        let mut m = new_test_model_with_game(80, 24, gr.clone(), None);
        {
            let g = gr.borrow();
            m.galactic_chart.set_state(
                g.enterprise.quad,
                g.galaxy_chart,
                g.chart_discovered,
                g.chart_known_bases,
                false,
            );
        }
        m.active_modal = MODAL_GALACTIC_CHART;
        compare_or_update("modal_galactic_chart", &m.view());
    });
}

#[test]
fn modal_damage_schematic() {
    with_env(|| {
        let mut g = new_game();
        g.enterprise.devices[DeviceID::Computer.0 as usize] = 2.1;
        g.enterprise.devices[DeviceID::PhotonTubes.0 as usize] = 1.4;
        let m = new_test_model_with_game(80, 24, sst_engine::game_ref(g), None);
        let (m, _) = m.update_model(runes("d"));
        compare_or_update("modal_damage_schematic", &m.view());
    });
}

#[test]
fn lrs_damaged_dashboard() {
    with_env(|| {
        let mut g = new_game();
        g.enterprise.devices[DeviceID::LRSensors.0 as usize] = 3.5;
        let m = new_test_model_with_game(80, 24, sst_engine::game_ref(g), None);
        compare_or_update("lrs_damaged_dashboard_80x24", &m.view());
    });
}

#[test]
fn sector_reticle_selected() {
    with_env(|| {
        let mut m = new_test_model_with_game(80, 24, sst_engine::game_ref(new_game()), None);
        m.selected_sector = [4, 5];
        compare_or_update("sector_reticle_selected", &m.view());
    });
}

#[test]
fn condition_red_alert() {
    with_env(|| {
        let mut g = new_game();
        g.enterprise.condition = ConditionType::Red;
        g.current_quad.klingons = vec![ptr(Klingon {
            id: 1,
            sector: [3, 4],
            energy: 400.0,
            ..Default::default()
        })]
        .into();
        g.current_quad.grid[3][4] = EntityType::Klingon;
        let m = new_test_model_with_game(80, 24, sst_engine::game_ref(g), None);
        compare_or_update("condition_red_alert", &m.view());
    });
}

#[test]
fn size_warning_dialog() {
    with_env(|| {
        let m = new_test_model_with_game(70, 20, sst_engine::game_ref(new_game()), None);
        compare_or_update("size_warning_dialog", &m.view());
    });
}

#[test]
fn modal_hall_of_fame() {
    with_env(|| {
        let mut g = new_game();
        g.metrics.klingons_killed = 8;
        g.metrics.commanders_killed = 2;
        g.metrics.casualties = 12;
        let m = new_test_model_with_game(80, 24, sst_engine::game_ref(g), None);
        let (m, _) = m.update_model(runes("h"));
        compare_or_update("modal_hall_of_fame_telemetry", &m.view());
        let (m, _) = m.update_model(tea::msg(KeyMsg::new(KEY_TAB)));
        compare_or_update("modal_hall_of_fame_leaderboard", &m.view());
    });
}

#[test]
fn modal_manual() {
    with_env(|| {
        let m = new_test_model_with_game(80, 24, sst_engine::game_ref(new_game()), None);
        let (m, _) = m.update_model(tea::msg(KeyMsg::new(KEY_F1)));
        compare_or_update("modal_manual_systems", &m.view());
        let (m, _) = m.update_model(runes("3"));
        compare_or_update("modal_manual_combat", &m.view());
    });
}

#[test]
fn combat_animations() {
    with_env(|| {
        let mut m = new_test_model_with_game(80, 24, sst_engine::game_ref(new_game()), None);
        let mut torp = BTreeMap::new();
        torp.insert(
            [3, 3],
            CellOverride {
                glyph: " O ".into(),
                style: lipgloss::new_style().foreground(lipgloss::color("9")),
            },
        );
        m.grid.set_anim_overrides(torp);
        assert_strict_80x24("anim_torpedo_flight", &m.view());
        compare_or_update("anim_torpedo_flight", &m.view());

        let mut ph = BTreeMap::new();
        for c in [[1, 2], [1, 3]] {
            ph.insert(
                c,
                CellOverride {
                    glyph: "---".into(),
                    style: lipgloss::new_style().foreground(lipgloss::color("11")),
                },
            );
        }
        m.grid.set_anim_overrides(ph);
        assert_strict_80x24("anim_phaser_beam", &m.view());
        compare_or_update("anim_phaser_beam", &m.view());
    });
}
