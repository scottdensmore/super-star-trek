//! Port of `pkg/tui/adversaries_tui_test.go` and `pkg/tui/tui_test.go`.

mod support_c;

use sst_engine::{
    ptr, sync_quadrant_enemies, Action, ActionFireTorpedo, ActionPhaserDirect, ActionShields,
    ConditionType, EnemyVessel, EntityType, Event, FactionType, GameState, Klingon, PlasmaTorpedo,
    TholianWebSegment, ANIM_SPEED_NORMAL, ANIM_SPEED_OFF,
};
use sst_tui::components::{sectorgrid, statuspanel};
use sst_tui::theme;
use sst_tui::*;
use sst_tuikit::tea::{self, KEY_TAB};
use support_c::*;

fn adversary_glyph_game() -> GameState {
    let mut g = GameState::new_game_with_seed(100);
    g.current_quad.grid[1][1] = EntityType::Romulan;
    g.current_quad.grid[2][2] = EntityType::Tholian;
    g.current_quad.grid[3][3] = EntityType::PlasmaTorpedo;
    g.current_quad.grid[4][4] = EntityType::TholianWeb;
    g
}

#[test]
fn tui_adversary_glyphs_rendering() {
    with_env(|_| {
        for name in [theme::THEME_MODERN, theme::THEME_LCARS, theme::THEME_CRT] {
            let grid = sectorgrid::new(Some(theme::get_theme(name)));
            let g = adversary_glyph_game();
            let v = grid.view_with_game(Some(&g));
            for glyph in ["+R+", "<T>", "*P*", ":::"] {
                assert!(v.contains(glyph), "[{name}] expected {glyph} in grid view");
            }
        }
    });
}

#[test]
fn tui_adversary_glyphs_selected_reticle() {
    with_env(|_| {
        let grid = sectorgrid::new(Some(theme::get_theme(theme::THEME_MODERN)));
        let g = adversary_glyph_game();
        for (c, want) in [
            ([1, 1], "[R]"),
            ([2, 2], "[T]"),
            ([3, 3], "[P]"),
            ([4, 4], "[:]"),
        ] {
            let v = grid.view(Some(&g.current_quad), g.enterprise.sector, c);
            assert!(v.contains(want), "expected {want}, view:\n{v}");
        }
    });
}

#[test]
fn tui_status_panel_hazard_alerts() {
    with_env(|_| {
        let panel = statuspanel::new(Some(theme::get_theme(theme::THEME_MODERN)), &[]);
        let mut g = GameState::new_game_with_seed(200);

        g.current_quad.plasma_torpedoes = vec![ptr(PlasmaTorpedo {
            id: 1001,
            sector: [3, 4],
            energy: 850.0,
            ..Default::default()
        })]
        .into();
        let v = panel.view(Some(&g));
        assert!(v.contains("⚠️ INCOMING PLASMA TORPEDO TRACKING"), "{v}");

        g.current_quad.plasma_torpedoes = Default::default();
        g.current_quad.web_segments = vec![
            ptr(TholianWebSegment {
                coord: [2, 2],
                strength: 100.0,
            }),
            ptr(TholianWebSegment {
                coord: [2, 3],
                strength: 100.0,
            }),
        ]
        .into();
        let v = panel.view(Some(&g));
        assert!(v.contains("⚠️ THOLIAN WEB ENCLOSURE:"), "{v}");

        g.current_quad.web_segments = Default::default();
        g.enterprise.sector = [4, 4];
        g.current_quad.enemies = vec![
            ptr(EnemyVessel {
                id: 1,
                faction: FactionType::Klingon,
                sector: [1, 4],
                energy: 500.0,
                ..Default::default()
            }),
            ptr(EnemyVessel {
                id: 2,
                faction: FactionType::Klingon,
                sector: [4, 7],
                energy: 500.0,
                ..Default::default()
            }),
        ]
        .into();
        sync_quadrant_enemies(&mut g.current_quad);
        let v = panel.view(Some(&g));
        assert!(v.contains("⚠️ CROSSFIRE BRACKET ACTIVE (+35% DMG)"), "{v}");
    });
}

#[test]
fn tui_hostile_turn_unified_adversary_turn() {
    with_env(|_| {
        let mut g = new_game();
        g.rules.adversaries = true;
        let g = gref(g);
        let mut m = new_model(Some(g.clone()), theme::default_theme());
        {
            let mut gm = g.borrow_mut();
            let target = gm.enterprise.sector;
            gm.current_quad.plasma_torpedoes = vec![ptr(PlasmaTorpedo {
                id: 1001,
                sector: [2, 4],
                target_sector: target,
                energy: 1000.0,
                turns_in_flight: 1,
                ..Default::default()
            })]
            .into();
            gm.current_quad.grid[2][4] = EntityType::PlasmaTorpedo;
        }
        let mut events: Vec<Event> = Vec::new();
        m.execute_klingon_turn_if_active(
            &Action::Shields(ActionShields { amount: 100.0 }),
            &mut events,
        );
        assert_ne!(
            g.borrow().current_quad.grid[2][4],
            EntityType::PlasmaTorpedo
        );
    });
}

fn torpedo_collision(seed: i64, ent: EntityType, setup: impl FnOnce(&mut GameState)) {
    let mut g = GameState::new_game_with_seed(seed);
    g.enterprise.sector = [4, 1];
    g.enterprise.torpedoes = 5;
    g.current_quad.grid[4][5] = ent;
    setup(&mut g);
    let events = ActionFireTorpedo {
        direction: 1.0,
        ..Default::default()
    }
    .execute(&mut g)
    .expect("unexpected torpedo error");
    assert_eq!(g.current_quad.grid[4][5], EntityType::Empty, "{ent:?}");
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::TorpedoHit(h) if h.entity == ent && h.destroyed)),
        "expected EventTorpedoHit for {ent:?} Destroyed=true"
    );
    if ent == EntityType::PlasmaTorpedo {
        assert_eq!(g.current_quad.plasma_torpedoes.len(), 0);
    }
    if ent == EntityType::TholianWeb {
        assert_eq!(g.current_quad.web_segments.len(), 0);
    }
}

#[test]
fn engine_action_fire_torpedo_direct_collisions_adversaries() {
    with_env(|_| {
        torpedo_collision(300, EntityType::Romulan, |g| {
            g.current_quad.enemies = vec![ptr(EnemyVessel {
                id: 201,
                faction: FactionType::Romulan,
                sector: [4, 5],
                energy: 200.0,
                ..Default::default()
            })]
            .into();
        });
        torpedo_collision(301, EntityType::Tholian, |g| {
            g.current_quad.enemies = vec![ptr(EnemyVessel {
                id: 301,
                faction: FactionType::Tholian,
                sector: [4, 5],
                energy: 200.0,
                ..Default::default()
            })]
            .into();
        });
        torpedo_collision(302, EntityType::PlasmaTorpedo, |g| {
            g.current_quad.plasma_torpedoes = vec![ptr(PlasmaTorpedo {
                id: 1001,
                sector: [4, 5],
                energy: 800.0,
                ..Default::default()
            })]
            .into();
        });
        torpedo_collision(303, EntityType::TholianWeb, |g| {
            g.current_quad.web_segments = vec![ptr(TholianWebSegment {
                coord: [4, 5],
                strength: 150.0,
            })]
            .into();
        });
    });
}

#[test]
fn tui_target_lock_adversary_cycling() {
    with_env(|_| {
        let mut g = GameState::new_game_with_seed(500);
        g.enterprise.sector = [4, 4];
        let ev = |id, faction, sector, cloaked| {
            ptr(EnemyVessel {
                id,
                faction,
                sector,
                energy: 400.0,
                is_cloaked: cloaked,
                ..Default::default()
            })
        };
        g.current_quad.enemies = vec![
            ev(201, FactionType::Romulan, [2, 4], false),
            ev(202, FactionType::Romulan, [2, 5], true),
            ev(301, FactionType::Tholian, [6, 4], false),
        ]
        .into();
        g.current_quad.plasma_torpedoes = vec![ptr(PlasmaTorpedo {
            id: 1001,
            sector: [3, 4],
            energy: 850.0,
            ..Default::default()
        })]
        .into();
        g.current_quad.web_segments = vec![ptr(TholianWebSegment {
            coord: [5, 4],
            strength: 120.0,
        })]
        .into();
        let g = gref(g);
        let mut m = new_model(Some(g.clone()), theme::default_theme());
        m.target_lock.set_state_from_game(Some(&g.borrow()), [0, 0]);
        let mut names = Vec::new();
        for _ in 0..4 {
            if let Some(cur) = m.target_lock.current_target() {
                names.push(cur.name.clone());
            }
            let _ = m.target_lock.update(&key(KEY_TAB));
        }
        let joined = names.join(" | ");
        for want in [
            "ROMULAN RAIDER",
            "THOLIAN SPINNER",
            "PLASMA TORPEDO",
            "THOLIAN WEB FILAMENT",
        ] {
            assert!(joined.contains(want), "expected {want} in {joined}");
        }
        assert!(!joined.contains("#202"), "{joined}");
    });
}

#[test]
fn tui_point_defense_parser_and_execution() {
    with_env(|_| {
        for input in ["pha 400 3 5", "pha 400 3,5"] {
            let p = parse_command(input);
            assert!(p.error.is_none(), "{input}: {:?}", p.error);
            match p.action {
                Some(Action::PhaserDirect(ActionPhaserDirect {
                    energy,
                    target_sector,
                })) => {
                    assert_eq!(energy, 400.0);
                    assert_eq!(target_sector, [3, 5]);
                }
                other => panic!("mismatched parsed action: {other:?}"),
            }
        }

        let mut g = GameState::new_game_with_seed(600);
        g.enterprise.sector = [4, 4];
        g.enterprise.energy = 4000.0;
        g.current_quad.grid[3][4] = EntityType::PlasmaTorpedo;
        g.current_quad.plasma_torpedoes = vec![ptr(PlasmaTorpedo {
            id: 1001,
            sector: [3, 4],
            energy: 800.0,
            ..Default::default()
        })]
        .into();
        let m = new_model(Some(gref(g)), theme::default_theme());
        let (m, _) = submit(&m, "pha 800 3,4");
        let g = m.game.as_ref().unwrap().borrow();
        assert_eq!(g.current_quad.grid[3][4], EntityType::Empty);
        assert_eq!(g.current_quad.plasma_torpedoes.len(), 0);
    });
}

// ---- tui_test.go ----

#[test]
fn model_red_alert_pulse() {
    with_env(|_| {
        let mut g = new_game();
        g.enterprise.condition = ConditionType::Red;
        g.rules.anim_speed = ANIM_SPEED_NORMAL;
        let m = new_model(Some(gref(g)), theme::default_theme());
        assert!(
            m.init().is_some(),
            "expected non-nil Init cmd when condition is Red"
        );
        assert_eq!(m.red_alert_cycle, 0);

        let (m, next) = up(&m, tea::msg(RedAlertPulseMsg));
        assert_eq!(m.red_alert_cycle, 1);
        assert!(next.is_some());
        let (m, next) = up(&m, tea::msg(RedAlertPulseMsg));
        assert_eq!(m.red_alert_cycle, 2);
        assert!(next.is_some());
        assert!(m.view().contains("CONDITION RED"));

        m.game.as_ref().unwrap().borrow_mut().rules.anim_speed = ANIM_SPEED_OFF;
        let (mut m, stop) = up(&m, tea::msg(RedAlertPulseMsg));
        assert!(stop.is_none(), "expected nil cmd when AnimSpeed is Off");

        {
            let mut g = m.game.as_ref().unwrap().borrow_mut();
            g.rules.anim_speed = ANIM_SPEED_NORMAL;
            g.enterprise.condition = ConditionType::Red;
        }
        m.red_alert_active = false;
        assert!(m.check_red_alert_cmd(ConditionType::Green).is_some());
    });
}

#[test]
fn model_multi_target_phaser_animation() {
    with_env(|_| {
        let mut g = new_game();
        g.enterprise.sector = [4, 4];
        g.enterprise.energy = 3000.0;
        g.rules.anim_speed = ANIM_SPEED_NORMAL;
        g.current_quad.klingons = vec![
            ptr(Klingon {
                id: 1,
                sector: [4, 7],
                energy: 200.0,
                ..Default::default()
            }),
            ptr(Klingon {
                id: 2,
                sector: [1, 4],
                energy: 200.0,
                ..Default::default()
            }),
        ]
        .into();
        g.current_quad.grid[4][7] = EntityType::Klingon;
        g.current_quad.grid[1][4] = EntityType::Klingon;
        let m = new_model(Some(gref(g)), theme::default_theme());
        let (m, cmd) = submit(&m, "pha 400");
        assert!(
            cmd.is_some(),
            "expected non-nil animation cmd on phaser fire"
        );
        let a = m.active_anim.clone().expect("expected activeAnim");
        let a = a.borrow();
        let frames = a.frames();
        assert_eq!(frames.len(), 4);
        let f1 = &frames[0];
        for c in [[4, 7], [1, 4]] {
            let ov = f1.overrides.get(&c);
            assert!(
                ov.is_some_and(|o| o.glyph == "<K>"),
                "expected '<K>' at {c:?}"
            );
        }
        assert!(f1.overrides.get(&[4, 5]).is_some_and(|o| o.glyph == "---"));
        assert!(f1.overrides.get(&[2, 4]).is_some_and(|o| o.glyph == " | "));
    });
}
