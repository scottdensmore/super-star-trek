//! Port of the second half of `pkg/tui/model_test.go` (tests from
//! `TestModel_OptionsModal_Hotkey` onwards).

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime};

use sst_audio::{Dispatcher, Player, SoundID, SOUND_PHASER, SOUND_SHIELDS, SOUND_TORPEDO_LAUNCH};
use sst_engine::{
    ptr, ConditionType, DeviceID, EntityType, Event, EventConditionChanged, EventGameOver,
    EventKlingonCounterAttack, EventShieldTransfer, EventSubsystemDamaged, EventTorpedoHit,
    GameLength, GameOverReason, GameState, Klingon, ScenarioID, ScoreEntry, SkillLevel,
};
use sst_tui::anim::TickMsg;
use sst_tui::components::optionsmodal::Row;
use sst_tui::components::{
    commandbar, commandpalette, damageschematic, halloffame, scenariomodal, targetlock,
};
use sst_tui::theme;
use sst_tui::{
    Model, VisualBellExpireMsg, MODAL_DAMAGE_SCHEMATIC, MODAL_HALL_OF_FAME, MODAL_MANUAL,
    MODAL_NONE, MODAL_SCENARIO,
};
use sst_tuikit::ansi;
use sst_tuikit::lipgloss;
use sst_tuikit::tea::{
    self, Cmd, KeyMsg, Msg, KEY_CTRL_D, KEY_CTRL_H, KEY_CTRL_S, KEY_DOWN, KEY_ENTER, KEY_ESC,
    KEY_F1, KEY_F2, KEY_RIGHT, KEY_RUNES, KEY_SPACE,
};
use sst_tuikit::termenv;

static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Runs `f` with an isolated HOME/XDG_CONFIG_HOME, the Ascii profile and a
/// fixed (dark) background detection.
///
/// The body runs on a thread with a large stack: `Model` is ~100 KB and
/// unoptimized builds keep every shadowed copy on the stack.
fn with_env(f: impl FnOnce() + Send + 'static) {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    sst_audio::set_test_mode(true);
    let tmp = std::env::temp_dir().join(format!(
        "sst-go-model-b-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let home = tmp.join("home");
    std::fs::create_dir_all(&home).unwrap();
    std::env::set_var("HOME", &home);
    std::env::set_var("XDG_CONFIG_HOME", home.join(".config"));
    let res = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            lipgloss::set_color_profile(termenv::Ascii);
            lipgloss::set_has_dark_background(true);
            theme::detect::set_fixed_detection(true);
            f();
        })
        .unwrap()
        .join();
    let _ = std::fs::remove_dir_all(&tmp);
    if let Err(e) = res {
        std::panic::resume_unwind(e);
    }
}

fn new_game() -> GameState {
    GameState::new_game(12345, SkillLevel::Good, GameLength::Medium)
}

fn nm(g: GameState) -> Model {
    sst_tui::new_model(Some(sst_engine::game_ref(g)), theme::default_theme())
}

fn key(typ: tea::KeyType) -> Msg {
    tea::msg(KeyMsg::new(typ))
}

fn runes(s: &str) -> Msg {
    let mut k = KeyMsg::new(KEY_RUNES);
    k.runes = s.chars().collect();
    tea::msg(k)
}

fn upd(m: &Model, msg: Msg) -> (Model, Option<Cmd>) {
    m.update_model(msg)
}

/// Go `mod.handleCommand(s)` (value receiver: works on a copy).
fn hc(m: &Model, s: &str) -> (Model, Option<Cmd>) {
    let mut c = m.clone();
    let cmd = c.handle_command(s);
    (c, cmd)
}

/// Go `cmd()`: timers fire right away instead of blocking.
fn call(cmd: Cmd) -> Option<Msg> {
    match cmd {
        Cmd::Tick(t) => t.fire(SystemTime::now()),
        other => other.call(),
    }
}

/// Go `if cmd != nil { m, _ = m.Update(cmd()) }`.
fn follow(m: Model, cmd: Option<Cmd>) -> Model {
    match cmd.and_then(call) {
        Some(msg) => m.update_model(msg).0,
        None => m,
    }
}

fn game_ref(m: &Model) -> std::cell::Ref<'_, GameState> {
    m.game.as_ref().expect("game").borrow()
}

fn assert_80x24(view: &str) {
    let lines: Vec<&str> = view.split('\n').collect();
    assert_eq!(lines.len(), 24, "expected 24 lines");
    for (i, l) in lines.iter().enumerate() {
        assert_eq!(ansi::string_width(l), 80, "line {i} width");
    }
}

#[test]
fn model_options_modal_hotkey() {
    with_env(|| {
        let g = new_game();
        let profile = g.rules.profile.clone();
        let m = nm(g);
        let (m, _) = upd(&m, runes("o"));
        assert!(m.show_options, "expected showOptions after 'o'");
        assert!(!m.command_bar.focused(), "expected CommandBar blurred");
        assert_eq!(m.options_modal.rules().profile, profile);

        let (m, _) = upd(&m, key(KEY_ESC));
        assert!(!m.show_options, "expected showOptions false after Esc");
        assert!(m.command_bar.focused(), "expected CommandBar focused");

        let (m, _) = upd(&m, runes("O"));
        assert!(m.show_options, "expected showOptions after 'O'");
    });
}

#[test]
fn model_options_modal_commands() {
    with_env(|| {
        for c in ["opts", "options", "settings"] {
            let m = nm(new_game());
            let (m, _) = upd(
                &m,
                tea::msg(commandbar::CommandSubmittedMsg { text: c.into() }),
            );
            assert!(m.show_options, "expected showOptions after {c:?}");
            assert!(!m.command_bar.focused(), "expected CommandBar blurred");
        }
    });
}

#[test]
fn model_options_modal_cycle_and_sync_rules() {
    with_env(|| {
        let m = nm(new_game());
        let (m, _) = upd(&m, runes("o"));
        let (m, _) = upd(&m, key(KEY_RIGHT));
        assert_eq!(m.options_modal.rules().profile, "hardcore");
        let (m, _) = upd(&m, runes("q"));
        assert!(!m.show_options);
        assert_eq!(game_ref(&m).rules.profile, "hardcore");
        assert!(m.command_bar.focused());
        assert!(!m.options_modal.closed);
    });
}

#[test]
fn model_options_modal_view_rendering() {
    with_env(|| {
        let mut m = nm(new_game());
        m.show_options = true;
        let v = m.view();
        assert!(v.contains("STARFLEET CONFIGURATION & RULES"), "{v}");
        assert!(v.contains("Difficulty Profile"), "{v}");
        assert!(v.contains("NORMAL"), "{v}");
    });
}

#[test]
fn model_options_modal_theme_propagation() {
    with_env(|| {
        let mut m = nm(new_game());
        m.show_options = true;
        let (m, _) = upd(&m, key(KEY_F2));
        assert_eq!(m.theme.unwrap().name(), "lcars");
        assert_eq!(m.options_modal.theme.name(), "lcars");
    });
}

#[test]
fn model_damage_schematic_modal_open_and_dismiss() {
    with_env(|| {
        let m = nm(new_game());
        let (m, _) = hc(&m, "dam");
        assert_eq!(m.active_modal, MODAL_DAMAGE_SCHEMATIC);
        let v = m.view();
        assert_80x24(&v);
        assert!(v.contains("DAMAGE CONTROL SCHEMATIC"));

        let (m2, cmd) = upd(&m, key(KEY_ESC));
        let m2 = follow(m2, cmd);
        assert_eq!(m2.active_modal, MODAL_NONE);
        assert!(m2.command_bar.focused());
    });
}

#[test]
fn model_damage_schematic_modal_commands() {
    with_env(|| {
        for c in ["dam", "damage", "damages"] {
            let m = nm(new_game());
            let (m, _) = hc(&m, c);
            assert_eq!(m.active_modal, MODAL_DAMAGE_SCHEMATIC, "{c}");
        }
    });
}

#[test]
fn model_damage_schematic_modal_hotkey() {
    with_env(|| {
        let m = nm(new_game());
        let (md, _) = upd(&m, runes("d"));
        assert_eq!(md.active_modal, MODAL_DAMAGE_SCHEMATIC);
        let (mc, cmd) = upd(&md, runes("d"));
        let mc = follow(mc, cmd);
        assert_eq!(mc.active_modal, MODAL_NONE);
        assert!(mc.command_bar.focused());

        let (mu, _) = upd(&m, runes("D"));
        assert_eq!(mu.active_modal, MODAL_DAMAGE_SCHEMATIC);
        let (mc, cmd) = upd(&mu, runes("q"));
        let mc = follow(mc, cmd);
        assert_eq!(mc.active_modal, MODAL_NONE);

        let (mcd, _) = upd(&m, key(KEY_CTRL_D));
        assert_eq!(mcd.active_modal, MODAL_DAMAGE_SCHEMATIC);
        let (mc, cmd) = upd(&mcd, key(KEY_ENTER));
        let mc = follow(mc, cmd);
        assert_eq!(mc.active_modal, MODAL_NONE);

        let mut mt = m.clone();
        mt.command_bar.set_value("com");
        let (ma, _) = upd(&mt, runes("d"));
        assert_eq!(ma.active_modal, MODAL_NONE);
        assert_eq!(ma.command_bar.value(), "comd");
    });
}

#[test]
fn model_damage_schematic_modal_close_modal_msg() {
    with_env(|| {
        let m = nm(new_game());
        let (m, _) = hc(&m, "dam");
        assert_eq!(m.active_modal, MODAL_DAMAGE_SCHEMATIC);
        let (m, _) = upd(&m, tea::msg(damageschematic::CloseModalMsg));
        assert_eq!(m.active_modal, MODAL_NONE);
        assert!(m.command_bar.focused());
    });
}

#[test]
fn model_damage_schematic_modal_theme_propagation() {
    with_env(|| {
        let m = nm(new_game());
        let (m, _) = hc(&m, "dam");
        assert_eq!(m.active_modal, MODAL_DAMAGE_SCHEMATIC);
        let (m, _) = upd(&m, key(KEY_F2));
        assert_eq!(m.theme.unwrap().name(), "lcars");
        assert_eq!(m.active_modal, MODAL_DAMAGE_SCHEMATIC);
    });
}

#[test]
fn model_hall_of_fame_open_and_dismiss() {
    with_env(|| {
        let m = nm(new_game());
        let (m, _) = hc(&m, "score");
        assert_eq!(m.active_modal, MODAL_HALL_OF_FAME);
        let v = m.view();
        assert_80x24(&v);
        assert!(v.contains("MISSION DEBRIEF"));
        let (m2, cmd) = upd(&m, runes("h"));
        let m2 = follow(m2, cmd);
        assert_eq!(m2.active_modal, MODAL_NONE);
        assert!(m2.command_bar.focused());
    });
}

#[test]
fn model_hall_of_fame_hotkeys() {
    with_env(|| {
        let m = nm(new_game());
        let (mh, _) = upd(&m, runes("h"));
        assert_eq!(mh.active_modal, MODAL_HALL_OF_FAME);
        let (mc, cmd) = upd(&mh, key(KEY_ESC));
        let mc = follow(mc, cmd);
        assert_eq!(mc.active_modal, MODAL_NONE);

        let (mu, _) = upd(&m, runes("H"));
        assert_eq!(mu.active_modal, MODAL_HALL_OF_FAME);
        let (mc, cmd) = upd(&mu, runes("q"));
        let mc = follow(mc, cmd);
        assert_eq!(mc.active_modal, MODAL_NONE);

        let (mch, _) = upd(&m, key(KEY_CTRL_H));
        assert_eq!(mch.active_modal, MODAL_HALL_OF_FAME);

        let mut mt = m.clone();
        mt.command_bar.set_value("nav");
        let (ma, _) = upd(&mt, runes("h"));
        assert_eq!(ma.active_modal, MODAL_NONE);
        assert_eq!(ma.command_bar.value(), "navh");
    });
}

#[test]
fn model_hall_of_fame_commands() {
    with_env(|| {
        for c in ["score", "scores", "halloffame", "hof"] {
            let m = nm(new_game());
            let (m, _) = hc(&m, c);
            assert_eq!(m.active_modal, MODAL_HALL_OF_FAME, "{c}");
        }
    });
}

#[test]
fn model_hall_of_fame_dismissal_keys() {
    with_env(|| {
        let m = nm(new_game());
        let (ms, _) = hc(&m, "score");
        let (mc, _) = upd(&ms, tea::msg(halloffame::CloseModalMsg));
        assert_eq!(mc.active_modal, MODAL_NONE);
        assert!(mc.command_bar.focused());

        for k in [runes("q"), key(KEY_ESC)] {
            let (ms, _) = hc(&m, "score");
            let (mc, cmd) = upd(&ms, k);
            let mc = follow(mc, cmd);
            assert_eq!(mc.active_modal, MODAL_NONE);
            assert!(mc.command_bar.focused());
        }
    });
}

#[test]
fn model_hall_of_fame_theme_propagation() {
    with_env(|| {
        let m = nm(new_game());
        let (m, _) = hc(&m, "score");
        assert_eq!(m.active_modal, MODAL_HALL_OF_FAME);
        let (m, _) = upd(&m, key(KEY_F2));
        assert_eq!(m.theme.unwrap().name(), "lcars");
        assert_eq!(m.active_modal, MODAL_HALL_OF_FAME);
    });
}

#[test]
fn model_hall_of_fame_game_over_flow() {
    with_env(|| {
        let mut gw = new_game();
        gw.metrics.klingons_killed = 10;
        gw.game_won = true;
        let mw = nm(gw);
        let (mw, _) = upd(
            &mw,
            tea::msg(EventGameOver {
                reason: GameOverReason::Won,
                score: 0.0,
            }),
        );
        assert_eq!(mw.active_modal, MODAL_HALL_OF_FAME);
        let v = mw.view();
        assert!(v.contains("ENTER CALLSIGN:"), "{v}");

        let mut gl = new_game();
        gl.metrics.casualties = 1000;
        gl.metrics.starbases_destroyed = 5;
        gl.game_won = false;
        let ml = nm(gl);
        let (ml, _) = upd(
            &ml,
            tea::msg(EventGameOver {
                reason: GameOverReason::Destroyed,
                score: 0.0,
            }),
        );
        assert_eq!(ml.active_modal, MODAL_HALL_OF_FAME);
        let v = ml.view();
        assert!(!v.contains("ENTER CALLSIGN:"), "{v}");
        assert!(v.contains("MISSION DEBRIEF"));
    });
}

#[test]
fn model_hall_of_fame_score_recorded_msg() {
    with_env(|| {
        let m = nm(new_game());
        let entry = ScoreEntry {
            captain_name: "Kirk".into(),
            score: 1200,
            rank: "[ADM]".into(),
            ..Default::default()
        };
        let (m, _) = upd(&m, tea::msg(halloffame::ScoreRecordedMsg { entry }));
        let msgs = m.command_bar.messages();
        assert!(
            msgs.iter()
                .any(|s| s.contains("Score recorded") && s.contains("Kirk")),
            "{msgs:?}"
        );
    });
}

#[test]
fn model_hall_of_fame_text_input_keystrokes_no_lag() {
    with_env(|| {
        let mut m = nm(new_game());
        m.open_hall_of_fame(true);
        assert_eq!(m.active_modal, MODAL_HALL_OF_FAME);

        let start = Instant::now();
        for r in "Spock".chars() {
            let (m2, cmd) = upd(&m, runes(&r.to_string()));
            m = m2;
            assert!(
                cmd.is_some(),
                "expected non-nil cmd for cursor blink on {r}"
            );
        }
        assert!(start.elapsed() < Duration::from_millis(100) * 10);
        let v = m.view();
        assert!(v.contains("Spock"), "{v}");

        let (m2, cmd) = upd(&m, key(KEY_ENTER));
        m = m2;
        let cmd = cmd.expect("expected cmd from Enter");
        let msg = call(cmd).expect("msg");
        let sm = msg
            .downcast_ref::<halloffame::ScoreRecordedMsg>()
            .expect("expected ScoreRecordedMsg")
            .clone();
        assert_eq!(sm.entry.captain_name, "Spock");
        let (m2, _) = upd(&m, tea::msg(sm));
        m = m2;
        let msgs = m.command_bar.messages();
        assert!(
            msgs.iter()
                .any(|s| s.contains("Score recorded") && s.contains("Spock")),
            "{msgs:?}"
        );
    });
}

#[test]
fn model_game_over_logging_no_duplicates() {
    with_env(|| {
        let mut g = new_game();
        g.metrics.klingons_killed = 10;
        g.game_won = true;
        let gr = sst_engine::game_ref(g);
        let m = sst_tui::new_model(Some(gr.clone()), theme::default_theme());
        let (me, _) = upd(
            &m,
            tea::msg(EventGameOver {
                reason: GameOverReason::Won,
                score: 1000.0,
            }),
        );
        let count = me
            .command_bar
            .messages()
            .iter()
            .filter(|s| s.contains("MISSION ACCOMPLISHED"))
            .count();
        assert_eq!(count, 1, "{:?}", me.command_bar.messages());

        let events = vec![
            Event::TorpedoHit(EventTorpedoHit {
                target: [3, 3],
                destroyed: true,
                ..Default::default()
            }),
            Event::GameOver(EventGameOver {
                reason: GameOverReason::Won,
                score: 500.0,
            }),
        ];
        let mut m2 = sst_tui::new_model(Some(gr), theme::default_theme());
        m2.log_events(&events);
        let mut m2e = m2.clone();
        m2e.handle_game_over(EventGameOver {
            reason: GameOverReason::Won,
            score: 500.0,
        });
        let count2 = m2e
            .command_bar
            .messages()
            .iter()
            .filter(|s| s.contains("MISSION ACCOMPLISHED"))
            .count();
        assert_eq!(count2, 1);
    });
}

#[test]
fn model_manual_open_and_dismiss() {
    with_env(|| {
        let m = nm(new_game());
        let (m, _) = hc(&m, "help");
        assert_eq!(m.active_modal, MODAL_MANUAL);
        assert_80x24(&m.view());
        let (m2, cmd) = upd(&m, key(KEY_ESC));
        let m2 = follow(m2, cmd);
        assert_eq!(m2.active_modal, MODAL_NONE);
    });
}

#[test]
fn model_manual_topic_jumps() {
    with_env(|| {
        let m = nm(new_game());
        let (mt, _) = hc(&m, "help tor");
        assert_eq!(mt.active_modal, MODAL_MANUAL);
        assert!(mt.view().contains("WEAPONS & COMBAT"));
        let (mn, _) = hc(&m, "man nav");
        assert!(mn.view().contains("NAVIGATION"));
    });
}

#[test]
fn model_manual_hotkeys() {
    with_env(|| {
        let m = nm(new_game());
        let (mut mq, _) = upd(&m, runes("?"));
        assert_eq!(mq.active_modal, MODAL_MANUAL);
        mq.active_modal = MODAL_NONE;
        let (mf, _) = upd(&mq, key(KEY_F1));
        assert_eq!(mf.active_modal, MODAL_MANUAL);
    });
}

#[test]
fn model_combat_animation_key_skip() {
    with_env(|| {
        let m = nm(new_game());
        let (m, cmd) = hc(&m, "tor 1 1");
        assert!(
            m.active_anim.is_some(),
            "expected activeAnim on torpedo fire"
        );
        assert!(cmd.is_some(), "expected TickCmd");
        let (ms, _) = upd(&m, runes(" "));
        assert!(ms.active_anim.is_none(), "expected activeAnim cleared");
    });
}

#[test]
fn model_combat_animation_speed_option() {
    with_env(|| {
        let m = nm(new_game());
        let (m, _) = hc(&m, "anim off");
        assert_eq!(game_ref(&m).rules.anim_speed, 0);
        let (mn, _) = hc(&m, "tor 1 1");
        assert!(mn.active_anim.is_none());
    });
}

#[test]
fn model_combat_animation_tick_msg() {
    with_env(|| {
        let m = nm(new_game());
        let (mut m, cmd) = hc(&m, "tor 1 1");
        assert!(m.active_anim.is_some());
        assert!(cmd.is_some());
        let anim_id = m.anim_id;
        let mut step = 1;
        while m.active_anim.is_some() {
            let (m2, next) = upd(&m, tea::msg(TickMsg { anim_id, step }));
            m = m2;
            step += 1;
            if m.active_anim.is_none() {
                assert!(next.is_none(), "expected nil cmd after animation finished");
                break;
            }
            assert!(next.is_some(), "expected non-nil nextCmd while in progress");
            assert!(step <= 50, "animation did not terminate within 50 steps");
        }
        assert!(m.active_anim.is_none());
    });
}

fn klingon_game(energy: f64) -> GameState {
    let mut g = new_game();
    g.current_quad.klingons = vec![ptr(Klingon {
        id: 1,
        sector: [2, 2],
        energy,
        ..Default::default()
    })]
    .into();
    g.current_quad.grid[2][2] = EntityType::Klingon;
    g.enterprise.sector = [5, 5];
    g.enterprise.energy = 3000.0;
    g
}

#[test]
fn model_combat_animation_phaser() {
    with_env(|| {
        let m = nm(klingon_game(200.0));
        let (m, cmd) = hc(&m, "pha 200");
        assert!(
            m.active_anim.is_some(),
            "expected activeAnim on phaser fire"
        );
        assert!(cmd.is_some());
        let (ms, _) = upd(&m, runes("x"));
        assert!(ms.active_anim.is_none());
    });
}

#[test]
fn model_combat_animation_anim_commands() {
    with_env(|| {
        let mut m = nm(new_game());
        for (c, want) in [
            ("anim fast", 1),
            ("anim cinematic", 3),
            ("anim normal", 2),
            ("anim", 3),
            ("anim", 0),
            ("anim", 1),
        ] {
            m = hc(&m, c).0;
            assert_eq!(game_ref(&m).rules.anim_speed, want, "{c}");
        }
    });
}

#[test]
fn model_combat_animation_target_lock_msg() {
    with_env(|| {
        let m = nm(klingon_game(2000.0));
        let (mut m, cmd) = upd(
            &m,
            tea::msg(targetlock::FireTorpedoMsg {
                target: [2, 2],
                bearing: 0.0,
            }),
        );
        assert!(
            m.active_anim.is_some(),
            "expected activeAnim on FireTorpedoMsg"
        );
        assert!(cmd.is_some());
        m.active_anim = None;
        m.grid.clear_anim_overrides();
        let (m, cmd) = upd(&m, tea::msg(targetlock::FirePhasersMsg { energy: 200.0 }));
        assert!(
            m.active_anim.is_some(),
            "expected activeAnim on FirePhasersMsg"
        );
        assert!(cmd.is_some());
    });
}

#[test]
fn model_scenario_modal_integration() {
    with_env(|| {
        let m = nm(new_game());
        let (ms, _) = hc(&m, "scenarios");
        assert_eq!(ms.active_modal, MODAL_SCENARIO);
        let v = ms.view();
        assert!(v.contains("TACTICAL CHALLENGE SIMULATOR"), "{v}");
        assert!(v.contains("Kobayashi Maru"));
        assert!(v.contains("[EXTREME]"));

        let (ms, _) = upd(&ms, key(KEY_DOWN));
        assert_eq!(
            ms.scenario_modal.selected_scenario().unwrap().id,
            ScenarioID::mutara_nebula()
        );

        let (ml, cmd) = upd(&ms, key(KEY_ENTER));
        let cmd = cmd.expect("expected command on Enter");
        let launch = call(cmd).expect("launch msg");
        let (ml, _) = upd(&ml, launch);
        assert_eq!(ml.active_modal, MODAL_NONE);
        assert_eq!(game_ref(&ml).scenario, ScenarioID::mutara_nebula());

        let (ms2, _) = hc(&ml, "scen");
        assert_eq!(ms2.active_modal, MODAL_SCENARIO);

        let (md, cmd) = upd(&ms2, runes("q"));
        let md = follow(md, cmd);
        assert_eq!(md.active_modal, MODAL_NONE);

        let (mp, _) = upd(
            &md,
            tea::msg(commandpalette::CommandSelectedMsg {
                command_prefix: "scenarios".into(),
                parameterized: false,
            }),
        );
        assert_eq!(mp.active_modal, MODAL_SCENARIO);

        let (mc, _) = upd(&mp, tea::msg(scenariomodal::MsgCloseScenarioModal));
        assert_eq!(mc.active_modal, MODAL_NONE);
    });
}

#[derive(Default)]
struct MockState {
    played: Vec<SoundID>,
    muted: bool,
    volume: i32,
    vol_set: bool,
}

#[derive(Default)]
struct MockAudioPlayer(RefCell<MockState>);

impl MockAudioPlayer {
    fn played(&self) -> Vec<SoundID> {
        self.0.borrow().played.clone()
    }
}

impl Player for MockAudioPlayer {
    fn play(&self, s: SoundID) {
        let mut st = self.0.borrow_mut();
        if !st.muted {
            st.played.push(s);
        }
    }
    fn set_muted(&self, m: bool) {
        self.0.borrow_mut().muted = m;
    }
    fn is_muted(&self) -> bool {
        self.0.borrow().muted
    }
    fn set_volume(&self, v: i32) {
        let mut st = self.0.borrow_mut();
        st.volume = v;
        st.vol_set = true;
    }
    fn volume(&self) -> i32 {
        let st = self.0.borrow();
        if !st.vol_set && st.volume == 0 {
            return 80;
        }
        st.volume
    }
}

fn with_mock(m: &mut Model) -> Rc<MockAudioPlayer> {
    let mock = Rc::new(MockAudioPlayer::default());
    m.audio_player = Some(mock.clone());
    m.audio_dispatcher = Some(Rc::new(Dispatcher::new(Some(mock.clone()))));
    mock
}

#[test]
fn audio_hotkey_toggle() {
    with_env(|| {
        let mut m = nm(new_game());
        let mock = with_mock(&mut m);
        assert!(m.sound_enabled());
        assert!(m.status.sound_enabled());
        assert!(!mock.is_muted());

        let (m2, _) = upd(&m, key(KEY_CTRL_S));
        m = m2;
        assert!(!m.sound_enabled());
        assert!(!m.status.sound_enabled());
        assert!(mock.is_muted());
        assert!(m
            .command_bar
            .messages()
            .last()
            .unwrap()
            .contains("*** Audio: Muted ***"));

        let (m2, _) = upd(&m, key(KEY_CTRL_S));
        m = m2;
        assert!(m.sound_enabled());
        assert!(m.status.sound_enabled());
        assert!(!mock.is_muted());
        assert!(m
            .command_bar
            .messages()
            .last()
            .unwrap()
            .contains("*** Audio: Enabled ***"));

        let (m2, _) = upd(&m, runes("m"));
        m = m2;
        assert!(!m.sound_enabled());
        assert!(mock.is_muted());

        let (m2, _) = upd(&m, runes("M"));
        m = m2;
        assert!(m.sound_enabled());
        assert!(!mock.is_muted());

        m.command_bar.set_value("tor");
        let (m2, _) = upd(&m, runes("m"));
        m = m2;
        assert!(m.sound_enabled());
        assert_eq!(m.command_bar.value(), "torm");
    });
}

#[test]
fn audio_options_modal_sync() {
    with_env(|| {
        let mut m = nm(new_game());
        let mock = with_mock(&mut m);
        assert!(m.sound_enabled());

        let (m2, _) = upd(&m, runes("o"));
        m = m2;
        assert!(m.show_options);
        assert!(m.options_modal.audio_enabled());

        m.options_modal.selected_row = Row::Audio;
        let (m2, _) = upd(&m, key(KEY_RIGHT));
        m = m2;
        assert!(!m.options_modal.audio_enabled());

        m.options_modal.selected_row = Row::Done;
        let (m2, _) = upd(&m, key(KEY_ENTER));
        m = m2;
        assert!(!m.show_options);
        assert!(!m.sound_enabled());
        assert!(!m.status.sound_enabled());
        assert!(mock.is_muted());

        let (m2, _) = upd(&m, runes("o"));
        m = m2;
        assert!(!m.options_modal.audio_enabled());

        m.options_modal.selected_row = Row::Audio;
        let (m2, _) = upd(&m, key(KEY_SPACE));
        m = m2;
        assert!(m.options_modal.audio_enabled());

        let (m2, _) = upd(&m, key(KEY_ESC));
        m = m2;
        assert!(!m.show_options);
        assert!(m.sound_enabled());
        assert!(m.status.sound_enabled());
        assert!(!mock.is_muted());
    });
}

#[test]
fn audio_event_dispatch() {
    with_env(|| {
        let mut g = new_game();
        g.enterprise.torpedoes = 10;
        g.enterprise.energy = 4000.0;
        g.enterprise.shields = 1000.0;
        g.current_quad.klingons = vec![ptr(Klingon {
            id: 1,
            sector: [1, 2],
            energy: 500.0,
            ..Default::default()
        })]
        .into();
        let mut m = nm(g);
        let mock = with_mock(&mut m);

        m = hc(&m, "tor 1 1").0;
        assert!(
            mock.played().contains(&SOUND_TORPEDO_LAUNCH),
            "{:?}",
            mock.played()
        );
        m = hc(&m, "pha 200").0;
        assert!(mock.played().contains(&SOUND_PHASER), "{:?}", mock.played());
        m = hc(&m, "she 100").0;
        assert!(
            mock.played().contains(&SOUND_SHIELDS),
            "{:?}",
            mock.played()
        );

        m.set_sound_enabled(false);
        let before = mock.played().len();
        let _ = hc(&m, "she 50");
        assert_eq!(mock.played().len(), before, "{:?}", mock.played());
    });
}

#[test]
fn model_visual_bell_activation_and_expiration() {
    with_env(|| {
        let mut m = nm(new_game());
        assert!(!m.command_bar.visual_bell());

        let cmd = m.log_events(&[Event::TorpedoHit(EventTorpedoHit {
            target: [3, 4],
            damage: 120.0,
            destroyed: false,
            ..Default::default()
        })]);
        assert!(m.command_bar.visual_bell());
        let cmd = cmd.expect("expected cmd for visual bell expiration");
        assert!(cmd.is_tick());
        let expire = call(cmd).expect("msg");
        assert!(expire.downcast_ref::<VisualBellExpireMsg>().is_some());

        let (m2, _) = upd(&m, tea::msg(VisualBellExpireMsg));
        m = m2;
        assert!(!m.command_bar.visual_bell());

        let cmd = m.log_events(&[Event::KlingonCounterAttack(EventKlingonCounterAttack {
            enemy_id: 1,
            damage: 50.0,
            ..Default::default()
        })]);
        assert!(m.command_bar.visual_bell());
        assert!(cmd.is_some());

        m.command_bar.set_visual_bell(false);
        let cmd = m.log_events(&[Event::SubsystemDamaged(EventSubsystemDamaged {
            device: DeviceID::Phasers,
            repair_time: 3.5,
        })]);
        assert!(m.command_bar.visual_bell());
        assert!(cmd.is_some());

        m.command_bar.set_visual_bell(false);
        let cmd = m.log_events(&[Event::ConditionChanged(EventConditionChanged {
            from: ConditionType::Yellow,
            to: ConditionType::Red,
        })]);
        assert!(m.command_bar.visual_bell());
        assert!(cmd.is_some());

        m.command_bar.set_visual_bell(false);
        let cmd = m.log_events(&[Event::ShieldTransfer(EventShieldTransfer {
            new_shields: 500.0,
            new_energy: 2500.0,
        })]);
        assert!(!m.command_bar.visual_bell());
        assert!(cmd.is_none());
    });
}

#[test]
fn model_visual_bell_via_command_execution() {
    with_env(|| {
        let mut g = new_game();
        g.enterprise.sector = [4, 4];
        g.enterprise.energy = 3000.0;
        g.current_quad.klingons = vec![ptr(Klingon {
            id: 1,
            sector: [4, 6],
            energy: 200.0,
            ..Default::default()
        })]
        .into();
        g.current_quad.grid[4][6] = EntityType::Klingon;
        let m = nm(g);
        let (m, cmd) = upd(
            &m,
            tea::msg(commandbar::CommandSubmittedMsg {
                text: "pha 300".into(),
            }),
        );
        assert!(m.command_bar.visual_bell());
        assert!(cmd.is_some());
        let (m, _) = upd(&m, tea::msg(VisualBellExpireMsg));
        assert!(!m.command_bar.visual_bell());
    });
}

#[test]
fn model_new_model_player_init() {
    with_env(|| {
        let m = nm(new_game());
        assert!(m.audio_player.is_some());
        assert!(m.audio_dispatcher.is_some());
    });
}
