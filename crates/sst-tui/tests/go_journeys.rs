//! Port of `pkg/tui/journey_test.go`, `adversary_journey_test.go`,
//! `tour_journey_test.go` and `audio_journey_test.go`.

mod support_c;

use std::cell::RefCell;
use std::rc::Rc;

use sst_engine::{
    calculate_pack_damage_multiplier, compute_score_kobayashi_maru, detect_crossfire_bracket,
    evaluate_kobayashi_maru, evaluate_mutara_nebula, evaluate_starbase_siege, persist, ptr,
    sync_quadrant_enemies, ConditionType, DeviceID, EnemyVessel, EntityType, EnvironmentType,
    FactionType, GameLength, GameOverReason, GameState, Klingon, PlasmaTorpedo, RefitID,
    ScenarioID, SkillLevel, TholianWebSegment,
};
use sst_tui::components::{drydockmodal, optionsmodal, scenariomodal};
use sst_tui::config;
use sst_tui::theme;
use sst_tui::*;
use sst_tuikit::tea::{self, KEY_ENTER, KEY_ESC, KEY_LEFT};
use support_c::*;

fn sized(m: Model, w: i32, h: i32) -> Model {
    m.update_model(tea::msg(tea::WindowSizeMsg {
        width: w,
        height: h,
    }))
    .0
}

// ---- journey_test.go ----

#[test]
fn tui_journey_moving() {
    with_env(|_| {
        let g = gref(new_game());
        let m = sized(new_model(Some(g.clone()), theme::default_theme()), 120, 30);
        let (start_quad, start_sector, initial_energy) = {
            let g = g.borrow();
            (g.enterprise.quad, g.enterprise.sector, g.enterprise.energy)
        };
        let mut target = [start_sector[0] + 1, start_sector[1]];
        if target[0] > 8 {
            target[0] = start_sector[0] - 1;
        }
        g.borrow_mut().current_quad.grid[target[0] as usize][target[1] as usize] =
            EntityType::Empty;
        let (m, _) = submit(&m, &format!("nav s {} {}", target[0], target[1]));
        {
            let g = g.borrow();
            assert_eq!(g.enterprise.sector, target);
            assert!(g.enterprise.energy < initial_energy);
            assert_eq!(
                g.current_quad.grid[target[0] as usize][target[1] as usize],
                EntityType::Enterprise
            );
        }

        let mut dest = [start_quad[0] + 1, start_quad[1]];
        if dest[0] > 8 {
            dest[0] = start_quad[0] - 1;
        }
        let prev_sd = g.borrow().stardate;
        let (m, _) = submit(&m, &format!("nav q {} {}", dest[0], dest[1]));
        {
            let g = g.borrow();
            assert_eq!(g.enterprise.quad, dest);
            assert!(g.stardate > prev_sd);
            assert!(g.chart_discovered[dest[0] as usize][dest[1] as usize]);
        }
        let v = m.view();
        assert!(v.contains("<E>"));
        assert!(v.contains(&format!("[{},{}]", dest[0], dest[1])));
    });
}

#[test]
fn tui_journey_fighting() {
    with_env(|_| {
        let g = gref(new_game());
        let m = sized(new_model(Some(g.clone()), theme::default_theme()), 120, 30);
        let k1 = ptr(Klingon {
            id: 1,
            sector: [4, 7],
            energy: 200.0,
            ..Default::default()
        });
        let k2 = ptr(Klingon {
            id: 2,
            sector: [6, 2],
            energy: 350.0,
            ..Default::default()
        });
        {
            let mut g = g.borrow_mut();
            g.enterprise.sector = [4, 4];
            g.enterprise.energy = 5000.0;
            g.enterprise.shields = 0.0;
            g.enterprise.torpedoes = 10;
            g.enterprise.condition = ConditionType::Red;
            g.current_quad.klingons = vec![k1.clone(), k2.clone()].into();
            g.current_quad.grid[4][7] = EntityType::Klingon;
            g.current_quad.grid[6][2] = EntityType::Klingon;
        }
        let ke = |k: &sst_engine::Ptr<Klingon>| k.as_ref().unwrap().borrow().energy;

        let (m, _) = submit(&m, "she 2000");
        {
            let g = g.borrow();
            assert!(g.enterprise.energy <= 3000.0);
            assert!(g.enterprise.shields > 0.0 && g.enterprise.shields < 2000.0);
        }

        let prev = ke(&k1);
        let (m, _) = submit(&m, "pha 400");
        assert!(!(ke(&k1) >= prev && ke(&k2) >= 350.0));

        let (m, _) = submit(&m, "tor 4 7");
        {
            let g = g.borrow();
            assert_eq!(g.enterprise.torpedoes, 9);
            assert_ne!(g.current_quad.grid[4][7], EntityType::Klingon);
        }

        let (m, _) = submit(&m, "tor 6 2");
        if g.borrow().current_quad.grid[6][2] == EntityType::Klingon {
            let _ = submit(&m, "tor 6 2");
        }
        let g = g.borrow();
        assert_eq!(g.current_quad.klingons.len(), 0);
        assert_eq!(g.current_quad.grid[4][7], EntityType::Empty);
        assert_eq!(g.current_quad.grid[6][2], EntityType::Empty);
    });
}

#[test]
fn tui_journey_docking() {
    with_env(|_| {
        let g = gref(new_game());
        let m = sized(new_model(Some(g.clone()), theme::default_theme()), 120, 30);
        {
            let mut g = g.borrow_mut();
            g.current_quad.starbase = Some([3, 4]);
            g.current_quad.grid[3][4] = EntityType::Starbase;
            g.current_quad.grid[3][5] = EntityType::Enterprise;
            g.enterprise.sector = [3, 5];
            g.enterprise.energy = 1450.0;
            g.enterprise.torpedoes = 2;
            g.enterprise.shields = 1000.0;
            g.enterprise.devices[DeviceID::Warp.0 as usize] = 4.2;
            g.enterprise.devices[DeviceID::Computer.0 as usize] = 1.8;
            g.enterprise.devices[DeviceID::PhotonTubes.0 as usize] = 2.5;
        }
        let (m, _) = submit(&m, "doc");
        {
            let g = g.borrow();
            assert_eq!(g.enterprise.condition, ConditionType::Docked);
            assert_eq!(g.enterprise.energy, 5000.0);
            assert_eq!(g.enterprise.torpedoes, 10);
            for (d, v) in g.enterprise.devices.iter().enumerate() {
                assert_eq!(*v, 0.0, "device {d}");
            }
        }
        assert!(m.view().contains("DOCKED"));
    });
}

fn launch(id: ScenarioID) -> Model {
    let mut m = sized(
        new_model(Some(gref(new_game())), theme::default_theme()),
        120,
        30,
    );
    // Go seeds the scenario from time.Now(); inject a fixed clock instead.
    m.set_clock(fixed_clock());
    m.update_model(tea::msg(scenariomodal::MsgLaunchScenario {
        scenario_id: id,
    }))
    .0
}

#[test]
fn tui_journey_scenario_kobayashi_maru() {
    with_env(|_| {
        let m = launch(ScenarioID::kobayashi_maru());
        let g = m.game.clone().unwrap();
        assert_eq!(g.borrow().scenario, ScenarioID::kobayashi_maru());
        assert_eq!(g.borrow().enterprise.quad, [4, 4]);
        assert_eq!(g.borrow().current_quad.klingons.len(), 3);
        let target = g.borrow().current_quad.klingons[0].clone().unwrap();
        let sec = target.borrow().sector;
        target.borrow_mut().energy = 10.0;
        let (mut m, _) = submit(&m, &format!("tor {} {}", sec[0], sec[1]));
        if m.game.as_ref().unwrap().borrow().metrics.klingons_killed < 1 {
            m = submit(&m, "pha 500").0;
        }
        let g = m.game.clone().unwrap();
        assert!(g.borrow().metrics.klingons_killed >= 1);
        assert_eq!(g.borrow().current_quad.klingons.len(), 3);

        g.borrow_mut().enterprise.energy = 0.0;
        let (done, won, reason) = evaluate_kobayashi_maru(&mut g.borrow_mut());
        assert!(done && !won && reason == GameOverReason::Lost);
        let score = compute_score_kobayashi_maru(&g.borrow(), false);
        assert!(
            score.rank_title.contains("Commendation") || score.rank_title.contains("Citation"),
            "{}",
            score.rank_title
        );
    });
}

#[test]
fn tui_journey_scenario_mutara_nebula() {
    with_env(|_| {
        let m = launch(ScenarioID::mutara_nebula());
        let g = m.game.clone().unwrap();
        assert_eq!(g.borrow().scenario, ScenarioID::mutara_nebula());
        assert_eq!(g.borrow().quadrant_env[5][5], EnvironmentType::Nebula);
        assert_eq!(g.borrow().enterprise.shields, 0.0);
        let (m, _) = submit(&m, "she 1000");
        let g = m.game.clone().unwrap();
        assert_eq!(g.borrow().enterprise.shields, 0.0);
        assert!(!g.borrow().current_quad.klingons.is_empty());
        let k = g.borrow().current_quad.klingons[0].clone().unwrap();
        let sec = k.borrow().sector;
        k.borrow_mut().energy = 10.0;
        let (mut m, _) = submit(&m, &format!("tor {} {}", sec[0], sec[1]));
        if !m
            .game
            .as_ref()
            .unwrap()
            .borrow()
            .current_quad
            .klingons
            .is_empty()
        {
            m = submit(&m, "pha 500").0;
        }
        let g = m.game.clone().unwrap();
        let (done, won, reason) = evaluate_mutara_nebula(&mut g.borrow_mut());
        assert!(done && won && reason == GameOverReason::Won);
    });
}

#[test]
fn tui_journey_scenario_starbase_siege() {
    with_env(|_| {
        let m = launch(ScenarioID::starbase_siege());
        let g = m.game.clone().unwrap();
        assert_eq!(g.borrow().scenario, ScenarioID::starbase_siege());
        assert_eq!(g.borrow().enterprise.quad, [2, 2]);
        let (m, _) = submit(&m, "nav q 4 4 6");
        let g = m.game.clone().unwrap();
        assert_eq!(g.borrow().enterprise.quad, [4, 4]);
        assert!(g.borrow().current_quad.starbase.is_some());
        assert_eq!(g.borrow().current_quad.klingons.len(), 3);
        {
            let mut gm = g.borrow_mut();
            for k in gm.current_quad.klingons.iter().flatten() {
                k.borrow_mut().energy = 10.0;
            }
            gm.enterprise.energy = 3000.0;
        }
        let (m, _) = submit(&m, "pha 2000");
        let g = m.game.clone().unwrap();
        let (done, won, reason) = evaluate_starbase_siege(&mut g.borrow_mut());
        assert!(done && won && reason == GameOverReason::Won);
    });
}

// ---- adversary_journey_test.go ----

#[test]
fn adversary_journey_tactical_factions_workflow() {
    with_env(|_| {
        let tour = Rc::new(RefCell::new(persist::new_tour(987654)));
        let m = new_model_with_tour(Some(tour), theme::get_theme(theme::THEME_MODERN));
        assert!(m.tour.as_ref().unwrap().borrow().active);
        assert!(m.game.as_ref().unwrap().borrow().current_quad.enemies.len() >= 2);
        let (m, _) = up(&m, key(KEY_ENTER));
        let (m, _) = up(&m, runes("s"));
        assert!(!m.game.as_ref().unwrap().borrow().game_over);
    });
}

#[test]
fn adversary_journey_point_defense_interception() {
    with_env(|_| {
        let mut g = new_game();
        g.rules.adversaries = true;
        g.enterprise.sector = [4, 4];
        g.enterprise.energy = 4000.0;
        g.enterprise.shields = 1000.0;
        g.current_quad.plasma_torpedoes = vec![ptr(PlasmaTorpedo {
            id: 1001,
            source_id: 201,
            sector: [4, 6],
            energy: 750.0,
            target_sector: g.enterprise.sector,
            turns_in_flight: 1,
        })]
        .into();
        g.current_quad.grid[4][6] = EntityType::PlasmaTorpedo;
        let g = gref(g);
        let m = new_model(Some(g.clone()), theme::get_theme(theme::THEME_MODERN));
        let sv = m.status.view(Some(&g.borrow()));
        assert!(sv.contains("INCOMING PLASMA TORPEDO TRACKING"), "{sv}");
        let gv = m.grid.view_with_game(Some(&g.borrow()));
        assert!(gv.contains("*P*"), "{gv}");

        let (m, _) = submit(&m, "pha 800 4 6");
        let g = m.game.clone().unwrap();
        assert_eq!(g.borrow().current_quad.plasma_torpedoes.len(), 0);
        assert_eq!(g.borrow().current_quad.grid[4][6], EntityType::Empty);
        let sv = m.status.view(Some(&g.borrow()));
        assert!(!sv.contains("INCOMING PLASMA TORPEDO TRACKING"));
    });
}

#[test]
fn adversary_journey_tholian_web_breach_and_escape() {
    with_env(|_| {
        let mut g = new_game();
        g.rules.adversaries = true;
        g.enterprise.sector = [4, 4];
        g.enterprise.energy = 4000.0;
        g.enterprise.torpedoes = 5;
        g.current_quad.enemies = vec![ptr(EnemyVessel {
            id: 301,
            faction: FactionType::Tholian,
            sector: [1, 1],
            energy: 800.0,
            special_state: 0,
            ..Default::default()
        })]
        .into();
        g.current_quad.grid[1][1] = EntityType::Tholian;
        g.current_quad.web_segments = vec![ptr(TholianWebSegment {
            coord: [4, 5],
            strength: 150.0,
        })]
        .into();
        g.current_quad.grid[4][5] = EntityType::TholianWeb;
        sync_quadrant_enemies(&mut g.current_quad);
        let g = gref(g);
        let m = new_model(Some(g.clone()), theme::get_theme(theme::THEME_MODERN));
        let sv = m.status.view(Some(&g.borrow()));
        assert!(sv.contains("THOLIAN WEB ENCLOSURE"), "{sv}");

        let (m, _) = submit(&m, "nav s 4 5");
        assert_eq!(m.game.as_ref().unwrap().borrow().enterprise.sector, [4, 4]);

        let (m, _) = submit(&m, "tor 4 5");
        {
            let g = m.game.as_ref().unwrap().borrow();
            for ws in g.current_quad.web_segments.iter().flatten() {
                assert_ne!(ws.borrow().coord, [4, 5]);
            }
            assert_eq!(g.current_quad.grid[4][5], EntityType::Empty);
        }

        let (m, _) = submit(&m, "nav s 4 5");
        let g = m.game.as_ref().unwrap().borrow();
        assert_eq!(g.enterprise.sector, [4, 5]);
        assert_eq!(g.current_quad.grid[4][5], EntityType::Enterprise);
    });
}

#[test]
fn adversary_journey_klingon_pack_crossfire_and_screening() {
    with_env(|_| {
        let mut g = new_game();
        g.rules.adversaries = true;
        g.enterprise.sector = [4, 1];
        g.enterprise.shields = 2000.0;
        g.enterprise.energy = 4000.0;
        let mk = |id, sector, energy, cmdr| {
            ptr(EnemyVessel {
                id,
                faction: FactionType::Klingon,
                sector,
                energy,
                is_commander: cmdr,
                ..Default::default()
            })
        };
        let commander = mk(101, [4, 7], 800.0, true);
        let escort = mk(102, [3, 4], 400.0, false);
        let flanker = mk(103, [7, 4], 400.0, false);
        g.current_quad.enemies = vec![commander, escort.clone(), flanker].into();
        g.current_quad.grid[4][7] = EntityType::Commander;
        g.current_quad.grid[3][4] = EntityType::Klingon;
        g.current_quad.grid[7][4] = EntityType::Klingon;
        sync_quadrant_enemies(&mut g.current_quad);
        let g = gref(g);
        let m = new_model(Some(g.clone()), theme::get_theme(theme::THEME_MODERN));

        assert!(detect_crossfire_bracket(Some(&g.borrow())));
        assert_eq!(calculate_pack_damage_multiplier(Some(&g.borrow())), 1.35);
        let sv = m.status.view(Some(&g.borrow()));
        assert!(sv.contains("CROSSFIRE BRACKET ACTIVE (+35% DMG)"), "{sv}");
        let initial = g.borrow().enterprise.shields;

        let (m, _) = submit(&m, "she 500");
        assert_eq!(escort.as_ref().unwrap().borrow().sector, [4, 4]);
        let g = m.game.as_ref().unwrap().borrow();
        assert_eq!(g.current_quad.grid[4][4], EntityType::Klingon);
        assert!(g.enterprise.shields < initial + 500.0);
    });
}

// ---- tour_journey_test.go ----

#[test]
fn tour_journey_multi_sector_and_refit_workflow() {
    with_env(|_| {
        let tour = Rc::new(RefCell::new(persist::new_tour(98765)));
        let m = new_model_with_tour(Some(tour), theme::get_theme(theme::THEME_MODERN));
        assert!(m.tour.as_ref().unwrap().borrow().active);
        {
            let mut g = m.game.as_ref().unwrap().borrow_mut();
            for row in g.galaxy.iter_mut() {
                for q in row.iter_mut().flatten() {
                    q.klingons = 0;
                    q.commanders = 0;
                }
            }
            g.klingons_remaining = 0;
        }
        let (mut m, _) = up(&m, key(KEY_ENTER));
        assert!(m.tour.as_ref().unwrap().borrow().in_drydock);
        assert!(m.tour.as_ref().unwrap().borrow().requisition_points > 0);

        m.drydock.tour = m.tour.clone();
        let (m, _) = up(&m, key(KEY_ENTER));
        let tier = m
            .tour
            .as_ref()
            .unwrap()
            .borrow()
            .installed_refits
            .as_ref()
            .and_then(|r| r.get(&RefitID::dilithium_core()).copied())
            .unwrap_or(0);
        assert_eq!(tier, 1);

        let (m, _) = up(&m, tea::msg(drydockmodal::DisembarkMsg));
        assert!(!m.tour.as_ref().unwrap().borrow().in_drydock);
        assert_eq!(m.tour.as_ref().unwrap().borrow().current_sector_index, 1);
        assert_eq!(
            m.game.as_ref().unwrap().borrow().enterprise.max_energy,
            3500.0
        );
    });
}

// ---- audio_journey_test.go ----

#[test]
fn audio_and_presentation_journey() {
    with_env(|home| {
        let g = gref(GameState::new_game(
            1234,
            SkillLevel::Novice,
            GameLength::Short,
        ));
        let mut m = new_model(Some(g.clone()), theme::default_theme());
        let p = m.audio_player.clone().unwrap();
        assert!(p.is_muted() || p.volume() == 80);
        assert!(!p.is_muted());
        assert_eq!(m.status.audio_volume(), 80);

        m = up(&m, runes("O")).0;
        assert!(m.options_modal.active() && m.show_options);

        m.options_modal.selected_row = optionsmodal::Row::Volume;
        m = up(&m, key(KEY_LEFT)).0;
        m = up(&m, key(KEY_LEFT)).0;
        assert_eq!(m.options_modal.volume(), 60);

        m.options_modal.selected_row = optionsmodal::Row::SoundTest;
        let sound_cmd = m.options_modal.update(&key(KEY_ENTER));
        match sound_cmd {
            None => panic!("expected non-nil tea.Cmd for sound audition"),
            Some(c) => {
                let msg = c.call().expect("message");
                let pm = msg
                    .downcast_ref::<optionsmodal::PlaySoundMsg>()
                    .expect("PlaySoundMsg");
                assert_eq!(pm.sound, sst_audio::SOUND_PHASER);
            }
        }

        m = up(&m, key(KEY_ESC)).0;
        assert!(!m.options_modal.active());
        assert!(!m.show_options);
        let cfg_path = home.join(".super-star-trek").join("config.json");
        assert!(cfg_path.exists(), "expected {cfg_path:?}");
        assert_eq!(config::load_config().unwrap().volume, 60);

        m = up(&m, runes("o")).0;
        assert!(m.show_options && m.options_modal.active());
        assert_eq!(m.options_modal.volume(), 60);
        m.options_modal.selected_row = optionsmodal::Row::Volume;
        m = up(&m, key(KEY_LEFT)).0;
        m = up(&m, key(KEY_LEFT)).0;
        assert_eq!(m.options_modal.volume(), 40);
        m = up(&m, key(KEY_ESC)).0;
        assert!(!m.show_options && !m.options_modal.active());
        assert_eq!(m.audio_player.as_ref().unwrap().volume(), 40);
        assert_eq!(m.status.audio_volume(), 40);
        assert_eq!(config::load_config().unwrap().volume, 40);

        m = up(&m, runes("O")).0;
        m.options_modal.selected_row = optionsmodal::Row::Volume;
        for _ in 0..4 {
            m = up(&m, key(KEY_LEFT)).0;
        }
        assert_eq!(m.options_modal.volume(), 0);
        m = up(&m, key(KEY_ESC)).0;
        assert_eq!(m.audio_player.as_ref().unwrap().volume(), 0);
        m = up(&m, runes("O")).0;
        assert_eq!(m.options_modal.volume(), 0);
        m = up(&m, key(KEY_ESC)).0;

        {
            let mut gm = m.game.as_ref().unwrap().borrow_mut();
            gm.enterprise.energy = 3000.0;
            gm.enterprise.torpedoes = 10;
        }
        m = submit(&m, "tor 1 1").0;
        let a = m.active_anim.take().expect("expected tactical animation");
        a.borrow_mut().skip();

        let m_new = new_model(Some(g), theme::default_theme());
        assert_eq!(m_new.audio_player.as_ref().unwrap().volume(), 0);
        assert_eq!(m_new.status.audio_volume(), 0);
    });
}
