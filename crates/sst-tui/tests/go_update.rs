//! Port of `pkg/tui/update_test.go`.

mod support_c;

use std::rc::Rc;

use sst_engine::{
    persist, ptr, ConditionType, DeviceID, EntityType, FactionType, Klingon, RefitID,
};
use sst_tui::components::{commandpalette, drydockmodal, optionsmodal, targetlock};
use sst_tui::theme::{self, ColorMode};
use sst_tui::*;
use sst_tuikit::tea::{self, KEY_CTRL_T, KEY_DOWN, KEY_ENTER, KEY_RIGHT};
use support_c::*;

#[test]
fn color_mode_keyboard_toggle() {
    with_env(|_| {
        let m = new_model(Some(gref(new_game())), modern(ColorMode::Auto));
        assert_eq!(m.theme.unwrap().color_mode(), ColorMode::Auto);

        let (u, _) = up(&m, key(KEY_CTRL_T));
        assert_eq!(u.theme.unwrap().color_mode(), ColorMode::Dark);
        assert!(
            last_msg_contains(&u, "Color mode set to dark (modern)"),
            "{:?}",
            msgs(&u)
        );

        let (u2, _) = up(&u, runes("ctrl+t"));
        assert_eq!(u2.theme.unwrap().color_mode(), ColorMode::Light);
        assert!(last_msg_contains(&u2, "Color mode set to light (modern)"));

        let (u3, _) = up(&u2, runes("shift+f2"));
        assert_eq!(u3.theme.unwrap().color_mode(), ColorMode::Auto);
        assert!(last_msg_contains(&u3, "Color mode set to auto (modern)"));
    });
}

#[test]
fn color_mode_keyboard_toggle_modals() {
    with_env(|_| {
        let mut m = new_model(Some(gref(new_game())), modern(ColorMode::Auto));
        m.active_modal = MODAL_TARGET_LOCK;
        let (mut u, _) = up(&m, key(KEY_CTRL_T));
        assert_eq!(u.theme.unwrap().color_mode(), ColorMode::Dark);
        assert_eq!(u.active_modal, MODAL_TARGET_LOCK);

        u.active_modal = MODAL_NONE;
        u.show_options = true;
        let (u2, _) = up(&u, runes("shift+f2"));
        assert_eq!(u2.theme.unwrap().color_mode(), ColorMode::Light);
        assert!(u2.show_options);
    });
}

#[test]
fn color_mode_text_commands() {
    let cases = [
        ("theme mode dark", ColorMode::Dark),
        ("theme mode light", ColorMode::Light),
        ("theme mode auto", ColorMode::Auto),
        ("color dark", ColorMode::Dark),
        ("color light", ColorMode::Light),
        ("color auto", ColorMode::Auto),
        ("THEME MODE DARK", ColorMode::Dark),
        ("COLOR LIGHT", ColorMode::Light),
    ];
    for (cmd, want) in cases {
        with_env(|_| {
            let mut m = new_model(Some(gref(new_game())), modern(ColorMode::Auto));
            m.handle_command(cmd);
            assert_eq!(m.theme.unwrap().color_mode(), want, "{cmd}");
            let expected = format!("Color mode set to {} (modern)", want);
            assert!(last_msg_contains(&m, &expected), "{cmd}: {:?}", msgs(&m));
        });
    }
}

#[test]
fn dynamic_background_adaptation_on_window_size() {
    with_env(|_| {
        let g = gref(new_game());
        let mut m = new_model(Some(g.clone()), modern(ColorMode::Auto));
        let current = theme::detect::detect_dark_background();
        m.last_dark_bg = !current;
        let (u, _) = up(
            &m,
            tea::msg(tea::WindowSizeMsg {
                width: 80,
                height: 24,
            }),
        );
        assert_eq!(u.last_dark_bg, current);

        let mut mf = new_model(Some(g), modern(ColorMode::Dark));
        mf.last_dark_bg = !current;
        let (uf, _) = up(
            &mf,
            tea::msg(tea::WindowSizeMsg {
                width: 80,
                height: 24,
            }),
        );
        assert_eq!(uf.theme.unwrap().color_mode(), ColorMode::Dark);
    });
}

#[test]
fn color_mode_command_palette_selection() {
    with_env(|_| {
        let mut m = new_model(Some(gref(new_game())), modern(ColorMode::Auto));
        m.active_modal = MODAL_COMMAND_PALETTE;
        let sel = |p: &str| {
            tea::msg(commandpalette::CommandSelectedMsg {
                command_prefix: p.into(),
                parameterized: false,
            })
        };
        let (u, _) = up(&m, sel("theme mode dark"));
        assert_eq!(u.active_modal, MODAL_NONE);
        assert_eq!(u.theme.unwrap().color_mode(), ColorMode::Dark);
        let (u2, _) = up(&u, sel("theme mode light"));
        assert_eq!(u2.theme.unwrap().color_mode(), ColorMode::Light);
        let (u3, _) = up(&u2, sel("theme mode auto"));
        assert_eq!(u3.theme.unwrap().color_mode(), ColorMode::Auto);
    });
}

#[test]
fn color_mode_options_modal_integration() {
    with_env(|_| {
        let m = new_model(Some(gref(new_game())), modern(ColorMode::Light));
        let (mut u, _) = up(&m, runes("o"));
        assert!(u.show_options);
        assert_eq!(u.options_modal.color_mode(), ColorMode::Light);
        u.options_modal.set_color_mode(ColorMode::Auto);
        let (u2, _) = up(&u, runes("q"));
        assert!(!u2.show_options);
        assert_eq!(u2.theme.unwrap().color_mode(), ColorMode::Auto);
    });
}

#[test]
fn theme_name_command_preserves_color_mode() {
    with_env(|_| {
        let mut m = new_model(Some(gref(new_game())), modern(ColorMode::Light));
        m.handle_command("theme lcars");
        assert_eq!(m.theme.unwrap().name(), "lcars");
        assert_eq!(m.theme.unwrap().color_mode(), ColorMode::Light);
    });
}

#[test]
fn color_mode_options_modal_key_interaction() {
    with_env(|_| {
        let m = new_model(Some(gref(new_game())), modern(ColorMode::Auto));
        let (mut u, _) = up(&m, runes("o"));
        for _ in 0..optionsmodal::Row::ColorMode.0 {
            u = up(&u, key(KEY_DOWN)).0;
        }
        let (mut u2, _) = up(&u, key(KEY_RIGHT));
        assert_eq!(u2.options_modal.color_mode(), ColorMode::Dark);
        while u2.options_modal.selected_row != optionsmodal::Row::Done {
            u2 = up(&u2, key(KEY_DOWN)).0;
        }
        let (u3, _) = up(&u2, key(KEY_ENTER));
        assert!(!u3.show_options);
        assert_eq!(u3.theme.unwrap().color_mode(), ColorMode::Dark);
    });
}

#[test]
fn type_she_command_and_enter() {
    with_env(|_| {
        let g = gref(new_game());
        let mut m = new_model(Some(g.clone()), theme::default_theme());
        for ch in "she 2500".chars() {
            m = up(&m, runes(&ch.to_string())).0;
        }
        let (m2, cmd) = up(&m, key(KEY_ENTER));
        let m3 = run_cmd(m2, cmd);
        assert_ne!(m3.active_modal, MODAL_GALACTIC_CHART);
        assert_eq!(g.borrow().enterprise.shields, 2500.0);
    });
}

#[test]
fn klingon_counter_attack_in_combat() {
    with_env(|_| {
        let mut g = new_game();
        g.enterprise.sector = [4, 1];
        g.enterprise.shields = 1000.0;
        g.enterprise.energy = 3000.0;
        g.current_quad.klingons = vec![ptr(Klingon {
            id: 1,
            sector: [4, 5],
            energy: 500.0,
            ..Default::default()
        })]
        .into();
        g.current_quad.grid[4][5] = EntityType::Klingon;
        let g = gref(g);
        let mut m = new_model(Some(g.clone()), theme::default_theme());
        m.handle_command("pha 100");
        assert!(any_msg(&m, "Klingon #1 returned fire"), "{:?}", msgs(&m));
        assert!(g.borrow().enterprise.shields < 1000.0);
        assert!(any_msg(&m, "Shields absorbed"), "{:?}", msgs(&m));

        let mut g2 = new_game();
        g2.enterprise.sector = [4, 1];
        g2.enterprise.shields = 0.0;
        g2.enterprise.energy = 3000.0;
        g2.current_quad.klingons = vec![ptr(Klingon {
            id: 2,
            sector: [4, 5],
            energy: 500.0,
            ..Default::default()
        })]
        .into();
        g2.current_quad.grid[4][5] = EntityType::Klingon;
        let g2 = gref(g2);
        let mut m2 = new_model(Some(g2.clone()), theme::default_theme());
        m2.handle_command("pha 100");
        assert!(g2.borrow().enterprise.energy < 3000.0);
        assert!(any_msg(&m2, "HULL HIT: -"), "{:?}", msgs(&m2));
    });
}

#[test]
fn dock_command_doc_and_dock() {
    with_env(|_| {
        let mut g = new_game();
        g.current_quad.starbase = Some([4, 4]);
        g.current_quad.grid[4][4] = EntityType::Starbase;
        g.enterprise.sector = [4, 5];
        g.enterprise.condition = ConditionType::Green;
        g.enterprise.energy = 2000.0;
        g.enterprise.torpedoes = 3;
        g.enterprise.devices[DeviceID::Phasers.0 as usize] = 4.5;
        let m = new_model(Some(gref(g)), theme::default_theme());

        let mut md = m.clone();
        md.handle_command("doc");
        assert_eq!(md.active_modal, MODAL_NONE);
        {
            let g = md.game.as_ref().unwrap().borrow();
            assert_eq!(g.enterprise.condition, ConditionType::Docked);
            assert_eq!(g.enterprise.energy, 5000.0);
            assert_eq!(g.enterprise.torpedoes, 10);
            assert_eq!(g.enterprise.devices[DeviceID::Phasers.0 as usize], 0.0);
        }

        let mut g2 = new_game();
        g2.enterprise.sector = [1, 1];
        g2.current_quad.starbase = Some([8, 8]);
        g2.current_quad.grid[8][8] = EntityType::Starbase;
        let mut m2 = new_model(Some(gref(g2)), theme::default_theme());
        m2.handle_command("doc");
        assert_eq!(m2.active_modal, MODAL_NONE);
        assert!(
            msgs(&m2).iter().any(|s| s
                .to_lowercase()
                .contains("enterprise not adjacent to starbase")),
            "{:?}",
            msgs(&m2)
        );

        let mut m3 = m.clone();
        m3.handle_command("docs");
        assert_eq!(m3.active_modal, MODAL_MANUAL);
        let mut m4 = m.clone();
        m4.handle_command("help doc");
        assert_eq!(m4.active_modal, MODAL_MANUAL);
    });
}

#[test]
fn torpedo_damage_and_miss_logging() {
    with_env(|_| {
        let mut g = new_game();
        g.enterprise.sector = [4, 5];
        g.enterprise.torpedoes = 10;
        let k = ptr(Klingon {
            id: 1,
            sector: [4, 1],
            energy: 1000.0,
            ..Default::default()
        });
        g.current_quad.klingons = vec![k.clone()].into();
        g.current_quad.grid[4][1] = EntityType::Klingon;
        let mut m = new_model(Some(gref(g)), theme::default_theme());
        m.active_modal = MODAL_TARGET_LOCK;
        let fire = || {
            tea::msg(targetlock::FireTorpedoMsg {
                target: [4, 1],
                bearing: 5.0,
            })
        };
        let (m1, _) = up(&m, fire());
        assert_eq!(k.as_ref().unwrap().borrow().energy, 375.0);
        assert!(any_msg(&m1, "500 units damage"), "{:?}", msgs(&m1));

        let (m2, _) = up(&m1, fire());
        assert!(
            any_msg(&m2, "Target destroyed at [4,1] (375 damage)")
                || any_msg(&m2, "Target destroyed at [4,1] (500 damage)"),
            "{:?}",
            msgs(&m2)
        );

        let mut g3 = new_game();
        g3.enterprise.sector = [1, 1];
        g3.enterprise.torpedoes = 5;
        for cell in g3.current_quad.grid[1].iter_mut().skip(1) {
            *cell = EntityType::Empty;
        }
        let mut m3 = new_model(Some(gref(g3)), theme::default_theme());
        m3.handle_command("tor 0");
        assert!(any_msg(&m3, "Torpedo missed."), "{:?}", msgs(&m3));

        let mut g4 = new_game();
        g4.enterprise.sector = [4, 5];
        g4.enterprise.torpedoes = 5;
        let k4 = ptr(Klingon {
            id: 2,
            sector: [4, 1],
            energy: 800.0,
            ..Default::default()
        });
        g4.current_quad.klingons = vec![k4.clone()].into();
        g4.current_quad.grid[4][1] = EntityType::Klingon;
        let mut m4 = new_model(Some(gref(g4)), theme::default_theme());
        m4.handle_command("tor c 5");
        assert_eq!(k4.as_ref().unwrap().borrow().energy, 225.0);
    });
}

#[test]
fn model_tour_commands() {
    with_env(|_| {
        let mut m = new_model(Some(gref(new_game())), theme::default_theme());
        m.handle_command("tour");
        assert!(any_msg(&m, "No active patrol tour"), "{:?}", msgs(&m));

        let tour = Rc::new(std::cell::RefCell::new(persist::new_tour(42)));
        let mut mt = new_model_with_tour(Some(tour), theme::default_theme());
        mt.handle_command("orders");
        assert!(any_msg(&mt, "STARFLEET PATROL ORDERS"), "{:?}", msgs(&mt));
    });
}

#[test]
fn model_tour_header_gauge() {
    with_env(|_| {
        let tour = Rc::new(std::cell::RefCell::new(persist::new_tour(42)));
        let mut m = new_model_with_tour(Some(tour), theme::default_theme());
        m.width = 80;
        m.height = 24;
        let v = m.view();
        assert!(v.contains("TOUR: SEC 1/4"), "{v}");
    });
}

fn installed(m: &Model, id: &RefitID) -> i32 {
    m.tour
        .as_ref()
        .unwrap()
        .borrow()
        .installed_refits
        .as_ref()
        .and_then(|r| r.get(id).copied())
        .unwrap_or(0)
}

#[test]
fn model_drydock_modal_transition_and_disembark() {
    with_env(|_| {
        let tour = Rc::new(std::cell::RefCell::new(persist::new_tour(12345)));
        let mut m = new_model_with_tour(Some(tour), theme::default_theme());
        m.width = 80;
        m.height = 24;
        m.game.as_ref().unwrap().borrow_mut().klingons_remaining = 0;
        let (mut m, _) = up(&m, key(KEY_ENTER));
        assert!(m.tour.as_ref().unwrap().borrow().in_drydock);
        assert!(m.tour.as_ref().unwrap().borrow().requisition_points > 0);
        let v = m.view();
        assert!(v.contains("STARBASE 01 DRYDOCK & REFIT FACILITY"), "{v}");

        m.drydock.tour = m.tour.clone();
        let (m, _) = up(&m, key(KEY_ENTER));
        assert_eq!(installed(&m, &RefitID::dilithium_core()), 1);

        let (m, _) = up(&m, tea::msg(drydockmodal::DisembarkMsg));
        let t = m.tour.as_ref().unwrap().borrow();
        assert!(!t.in_drydock);
        assert_eq!(t.current_sector_index, 1);
        assert_eq!(
            m.game.as_ref().unwrap().borrow().enterprise.max_energy,
            3500.0
        );
    });
}

#[test]
fn model_tour_failure_recording() {
    with_env(|_| {
        let tour = Rc::new(std::cell::RefCell::new(persist::new_tour(54321)));
        let mut m = new_model_with_tour(Some(tour), theme::default_theme());
        m.player_callsign = "Defiant".into();
        let (m, _) = up(
            &m,
            tea::msg(sst_engine::EventGameOver {
                reason: sst_engine::GameOverReason::Destroyed,
                score: 120.0,
            }),
        );
        {
            let t = m.tour.as_ref().unwrap().borrow();
            assert!(t.failed);
            assert!(!t.active);
            assert_eq!(t.failure_reason, sst_engine::GameOverReason::Destroyed);
        }
        let lb = persist::load_leaderboard(Some(&persist::default_leaderboard_path()))
            .expect("failed to load leaderboard");
        assert!(
            lb.tour_records
                .iter()
                .any(|r| r.callsign == "Defiant" && r.rank == "Commander (KIA)"),
            "{:?}",
            lb.tour_records
        );
    });
}

#[test]
fn model_tour_preserves_sector_enemies() {
    with_env(|_| {
        let tour = Rc::new(std::cell::RefCell::new(persist::new_tour(42)));
        let m = new_model_with_tour(Some(tour), theme::default_theme());
        let faction0 = |m: &Model| {
            let g = m.game.as_ref().unwrap().borrow();
            assert_eq!(g.current_quad.enemies.len(), 2);
            let f = g.current_quad.enemies[0].as_ref().unwrap().borrow().faction;
            f
        };
        assert_eq!(faction0(&m), FactionType::Romulan);

        m.game.as_ref().unwrap().borrow_mut().klingons_remaining = 0;
        let (m, _) = up(&m, key(KEY_ENTER));
        assert!(m.tour.as_ref().unwrap().borrow().in_drydock);
        let (m, _) = up(&m, tea::msg(drydockmodal::DisembarkMsg));
        assert_eq!(faction0(&m), FactionType::Tholian);
    });
}
