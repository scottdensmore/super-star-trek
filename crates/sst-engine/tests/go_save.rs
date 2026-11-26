//! Port of pkg/engine/save_test.go.

use std::rc::Rc;

use sst_engine::json::GoVec;
use sst_engine::*;

fn temp_dir(name: &str) -> String {
    let d = std::env::temp_dir().join(format!("sst-go-save-{}-{}", std::process::id(), name));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d.to_string_lossy().into_owned()
}

fn join(a: &str, b: &str) -> String {
    format!("{a}/{b}")
}

fn exists(p: &str) -> bool {
    std::fs::metadata(p).is_ok()
}

#[test]
fn spock_filename_validation() {
    for f in ["GAME", "save1", "TrekGame", "A", "GAME.TRK", "test.trk"] {
        assert!(
            validate_save_filename(f).is_ok(),
            "expected valid filename {f:?}"
        );
    }
    for f in [
        "123game",
        "*save*",
        "toolongfilename",
        "",
        "   ",
        ".trk",
        "toolongname.trk",
        "🚀game",
        "★trek",
    ] {
        assert!(
            validate_save_filename(f).is_err(),
            "expected error for {f:?}"
        );
    }
    let err = validate_save_filename("1bad").unwrap_err();
    assert_eq!(
        err.0,
        "spock- \"Captain, file names must begin with an alphabetic letter (A-Z).\""
    );
    let err = validate_save_filename("toolongfilename").unwrap_err();
    assert!(err.0.contains("exceed 9 characters"));
    let err = validate_save_filename("").unwrap_err();
    assert!(err.0.contains("cannot be empty"));
}

#[test]
fn game_save_and_load_roundtrip() {
    let dir = temp_dir("roundtrip");
    let path = join(&dir, "TESTSAVE.TRK");
    let mut orig = GameState::new_game(9999, SkillLevel::Expert, GameLength::Long);
    orig.enterprise.energy = 4242.0;
    orig.enterprise.shields = 500.0;
    orig.enterprise.torpedoes = 8;
    orig.stardate = 3141.5;
    orig.remaining_klingons = 12;
    persist::save_state(&orig, &path).expect("save");

    let lr = persist::load_game(&path).expect("load");
    let mut l = lr.borrow_mut();
    assert_eq!(l.enterprise.energy, 4242.0);
    assert_eq!(l.enterprise.shields, 500.0);
    assert_eq!(l.enterprise.torpedoes, 8);
    assert_eq!(l.stardate, 3141.5);
    assert_eq!(l.remaining_klingons, 12);
    assert_eq!(l.skill, SkillLevel::Expert);
    assert_eq!(l.length, GameLength::Long);
    assert!(l.rng.is_some(), "expected non-nil RNG after LoadGame");
    let val = l.rng().intn(100);
    let mut expected = PRNG::new(l.stardate as i64);
    assert_eq!(val, expected.intn(100));
}

#[test]
fn scenario_save_and_load_roundtrip() {
    let dir = temp_dir("scenario");
    let path = join(&dir, "SCENMUT.TRK");
    let mut orig = GameState::new_game(1234, SkillLevel::Expert, GameLength::Short);
    orig.scenario = ScenarioID::mutara_nebula();
    persist::save_state(&orig, &path).expect("save");
    let l = persist::load_game(&path).expect("load");
    assert_eq!(l.borrow().scenario, ScenarioID::mutara_nebula());

    let none_path = join(&dir, "SCENNONE.TRK");
    let mut orig_none = GameState::new_game(5678, SkillLevel::Good, GameLength::Medium);
    orig_none.scenario = ScenarioID::none();
    persist::save_state(&orig_none, &none_path).expect("save none");
    let data = std::fs::read_to_string(&none_path).unwrap();
    assert!(
        !data.contains("\"scenario\""),
        "expected scenario omitted: {data}"
    );
    let l = persist::load_game(&none_path).expect("load none");
    assert_eq!(l.borrow().scenario, ScenarioID::none());
}

#[test]
fn game_save_auto_append_extension() {
    let dir = temp_dir("autoext");
    let mut orig = GameState::new_game(1234, SkillLevel::Good, GameLength::Medium);
    orig.enterprise.energy = 3333.0;
    persist::save_state(&orig, &join(&dir, "AUTOSAVE")).expect("save");
    let expected = join(&dir, "AUTOSAVE.TRK");
    assert!(exists(&expected), "expected {expected} to exist");
    let l = persist::load_game(&expected).expect("load");
    assert_eq!(l.borrow().enterprise.energy, 3333.0);
}

#[test]
fn game_save_invalid_filename() {
    let dir = temp_dir("invalidname");
    let orig = GameState::new_game(1234, SkillLevel::Fair, GameLength::Short);
    assert!(persist::save_state(&orig, &join(&dir, "123BADNAME.TRK")).is_err());
}

#[test]
fn load_game_not_found() {
    let dir = temp_dir("notfound");
    assert!(persist::load_game(&join(&dir, "NONEXISTENT.TRK")).is_err());
}

#[test]
fn load_game_corrupted() {
    let dir = temp_dir("corrupted");
    let p = join(&dir, "CORRUPT.TRK");
    std::fs::write(&p, b"not valid json {").unwrap();
    assert!(persist::load_game(&p).is_err());
}

#[test]
fn game_save_trailing_whitespace() {
    let dir = temp_dir("trailing");
    let mut orig = GameState::new_game(1234, SkillLevel::Good, GameLength::Medium);
    orig.enterprise.energy = 3333.0;
    persist::save_state(&orig, &join(&dir, "TESTSAVE   ")).expect("save");
    let expected = join(&dir, "TESTSAVE.TRK");
    assert!(exists(&expected), "expected {expected} to exist");
    let l = persist::load_game(&expected).expect("load");
    assert_eq!(l.borrow().enterprise.energy, 3333.0);
}

#[test]
fn inspect_save() {
    let dir = temp_dir("inspect");
    let path = join(&dir, "TESTSAVE.TRK");
    let mut g = GameState::new_game(12345, SkillLevel::Good, GameLength::Medium);
    g.stardate = 3450.5;
    g.time_remaining = 24.5;
    g.enterprise.condition = ConditionType::Yellow;
    g.remaining_klingons = 7;
    persist::save_state(&g, &path).expect("save");
    let m = persist::inspect_save(&path).expect("inspect");
    assert_eq!(m.filename, "TESTSAVE.TRK");
    assert_eq!(m.path, path);
    assert_eq!(m.skill, SkillLevel::Good);
    assert_eq!(m.stardate, 3450.5);
    assert_eq!(m.time_remaining, 24.5);
    assert_eq!(m.condition, ConditionType::Yellow);
    assert_eq!(m.klingons_left, 7);
    assert!(!m.mod_time.is_zero());

    assert!(persist::inspect_save(&join(&dir, "NONEXIST.TRK")).is_err());
    let corrupt = join(&dir, "CORRUPT.TRK");
    std::fs::write(&corrupt, b"NOT_JSON").unwrap();
    assert!(persist::inspect_save(&corrupt).is_err());
}

#[test]
fn save_load_with_anomalies() {
    let dir = temp_dir("anomalies");
    let path = join(&dir, "ANOMALY.TRK");
    let rules = default_rules_for_profile(&DifficultyProfile::hardcore());
    assert!(rules.spatial_anomalies);
    let orig = GameState::new_game_with_options(777, SkillLevel::Expert, GameLength::Medium, rules);
    let has = (1..=8).any(|r| (1..=8).any(|c| orig.quadrant_env[r][c] != EnvironmentType::Normal));
    assert!(has, "expected seeded anomalies");
    persist::save_state(&orig, &path).expect("save");
    let l = persist::load_game(&path).expect("load");
    let l = l.borrow();
    assert!(l.rules.spatial_anomalies);
    for r in 1..=8 {
        for c in 1..=8 {
            assert_eq!(
                l.quadrant_env[r][c], orig.quadrant_env[r][c],
                "quadrant [{r},{c}]"
            );
        }
    }
}

fn tier(t: &TourState, id: &RefitID) -> i32 {
    t.installed_refits
        .as_ref()
        .and_then(|m| m.get(id).copied())
        .unwrap_or(0)
}

#[test]
fn save_and_load_tour_game() {
    let dir = temp_dir("tour");
    let path = join(&dir, "test_tour.json");
    let tour = tour_ref(TourState::new_at(777, 0));
    let g = tour.borrow_mut().start_current_sector(None).expect("start");
    tour.borrow_mut().requisition_points = 1450;
    tour.borrow_mut()
        .installed_refits
        .as_mut()
        .unwrap()
        .insert(RefitID::dilithium_core(), 1);
    persist::save_tour_game(&path, Some(&g), Some(&tour)).expect("save");

    let (lg, lt) = persist::load_tour_game(&path).expect("load");
    let lt = lt.expect("expected non-nil loadedTour");
    let lg = lg.expect("game");
    assert_eq!(lt.borrow().requisition_points, 1450);
    assert_eq!(tier(&lt.borrow(), &RefitID::dilithium_core()), 1);
    assert_eq!(lg.borrow().enterprise.max_energy, 3500.0);
    let cur = lt
        .borrow()
        .current_game_state
        .clone()
        .expect("current game");
    assert!(
        Rc::ptr_eq(&cur, &lg),
        "expected CurrentGameState to alias loadedGame"
    );
    // Go only checks RNG != nil (it is a non-nil PRNG with a nil source here).
    assert!(cur.borrow().rng.is_some());
}

#[test]
fn save_and_load_tour_game_preserves_damaged_energy() {
    let dir = temp_dir("damaged");
    let path = join(&dir, "damaged_tour.json");
    let tour = tour_ref(TourState::new_at(777, 0));
    let g = tour.borrow_mut().start_current_sector(None).unwrap();
    g.borrow_mut().enterprise.energy = 1234.0;
    g.borrow_mut().enterprise.torpedoes = 4;
    tour.borrow_mut().requisition_points = 1000;
    persist::save_tour_game(&path, Some(&g), Some(&tour)).expect("save");
    let (lg, _) = persist::load_tour_game(&path).expect("load");
    let lg = lg.unwrap();
    assert_eq!(lg.borrow().enterprise.energy, 1234.0);
    assert_eq!(lg.borrow().enterprise.torpedoes, 4);
}

#[test]
fn save_and_load_tour_game_in_drydock() {
    let dir = temp_dir("drydock");
    let path = join(&dir, "drydock_tour.json");
    let tour = tour_ref(TourState::new_at(888, 0));
    {
        let mut t = tour.borrow_mut();
        t.in_drydock = true;
        t.requisition_points = 2000;
        t.installed_refits
            .as_mut()
            .unwrap()
            .insert(RefitID::deflector_grid(), 2);
    }
    persist::save_tour_game(&path, None, Some(&tour)).expect("save");
    let (lg, lt) = persist::load_tour_game(&path).expect("load");
    assert!(lg.is_none(), "expected nil loadedGame");
    let lt = lt.expect("tour");
    assert!(lt.borrow().in_drydock);
    assert_eq!(tier(&lt.borrow(), &RefitID::deflector_grid()), 2);
}

#[test]
fn save_and_load_tour_game_backward_compatibility() {
    let dir = temp_dir("backcompat");
    let legacy = join(&dir, "LEGACY.TRK");
    let mut orig = GameState::new_game(1234, SkillLevel::Good, GameLength::Medium);
    orig.enterprise.energy = 2800.0;
    orig.stardate = 3200.0;
    persist::save_state(&orig, &legacy).expect("save legacy");
    let (lg, lt) = persist::load_tour_game(&legacy).expect("load legacy");
    let lg = lg.expect("game");
    assert!(lt.is_none());
    assert_eq!(lg.borrow().enterprise.energy, 2800.0);
    assert!(lg.borrow().rng.is_some());

    let tour_path = join(&dir, "TOURSAVE.TRK");
    let tour = tour_ref(TourState::new_at(999, 0));
    let g = tour.borrow_mut().start_current_sector(None).unwrap();
    {
        let mut gb = g.borrow_mut();
        gb.stardate = 3500.0;
        gb.time_remaining = 25.0;
        gb.enterprise.condition = ConditionType::Red;
        gb.remaining_klingons = 5;
    }
    tour.borrow_mut()
        .installed_refits
        .as_mut()
        .unwrap()
        .insert(RefitID::dilithium_core(), 2);
    persist::save_tour_game(&tour_path, Some(&g), Some(&tour)).expect("save tour");

    let lg = persist::load_game(&tour_path).expect("LoadGame on tour save");
    assert_eq!(lg.borrow().enterprise.max_energy, 4000.0);
    assert!(lg.borrow().rng.is_some());

    let m = persist::inspect_save(&tour_path).expect("inspect");
    assert_eq!(m.stardate, 3500.0);
    assert_eq!(m.condition, ConditionType::Red);
    assert_eq!(m.klingons_left, 5);
}

#[test]
fn save_and_load_tour_game_errors() {
    let dir = temp_dir("tourerrors");
    assert!(persist::load_tour_game(&join(&dir, "MISSING.TRK")).is_err());
    let corrupt = join(&dir, "CORRUPT.TRK");
    std::fs::write(&corrupt, b"NOT_VALID_JSON{").unwrap();
    assert!(persist::load_tour_game(&corrupt).is_err());
    let empty = join(&dir, "EMPTY.json");
    std::fs::write(&empty, b"{}").unwrap();
    assert!(persist::load_tour_game(&empty).is_err());
    let unrec = join(&dir, "UNRECOGNIZED.json");
    std::fs::write(&unrec, br#"{"some_other_app": true}"#).unwrap();
    assert!(persist::load_tour_game(&unrec).is_err());
}

#[test]
fn save_and_load_preserves_expanded_adversaries_and_hazards() {
    let dir = temp_dir("adv");
    let path = join(&dir, "adversaries_save.json");
    let mut g = GameState::new_game_with_seed(5555);
    let target = g.enterprise.sector;
    g.current_quad.enemies = GoVec::from_vec(vec![ptr(EnemyVessel {
        id: 10,
        faction: FactionType::Romulan,
        sector: [2, 3],
        energy: 1150.0,
        shields: 450.0,
        is_cloaked: true,
        cloak_turns: 2,
        ..Default::default()
    })]);
    g.current_quad.plasma_torpedoes = GoVec::from_vec(vec![ptr(PlasmaTorpedo {
        id: 20,
        source_id: 10,
        sector: [3, 4],
        energy: 880.0,
        target_sector: target,
        turns_in_flight: 1,
    })]);
    g.current_quad.web_segments = GoVec::from_vec(vec![ptr(TholianWebSegment {
        coord: [7, 7],
        strength: 180.0,
    })]);
    g.current_quad.grid[2][3] = EntityType::Romulan;
    g.current_quad.grid[3][4] = EntityType::PlasmaTorpedo;
    g.current_quad.grid[7][7] = EntityType::TholianWeb;
    persist::save_game(&game_ref(g), &path).expect("save");

    let l = persist::load_game(&path).expect("load");
    let l = l.borrow();
    assert_eq!(l.current_quad.enemies.len(), 1);
    let e = l.current_quad.enemies[0].as_ref().unwrap().borrow();
    assert!(e.faction == FactionType::Romulan && e.is_cloaked, "{e:?}");
    let pt = &l.current_quad.plasma_torpedoes;
    assert!(pt.len() == 1 && pt[0].as_ref().unwrap().borrow().energy == 880.0);
    let ws = &l.current_quad.web_segments;
    assert!(ws.len() == 1 && ws[0].as_ref().unwrap().borrow().strength == 180.0);
}

#[test]
fn save_and_load_syncs_klingons_on_load() {
    let dir = temp_dir("sync");
    let path = join(&dir, "klingon_sync.json");
    let mut g = GameState::new_game_with_seed(6666);
    g.current_quad.enemies = GoVec::from_vec(vec![ptr(EnemyVessel {
        id: 1,
        faction: FactionType::Klingon,
        sector: [3, 3],
        energy: 600.0,
        shields: 200.0,
        is_commander: true,
        ..Default::default()
    })]);
    g.current_quad.grid[3][3] = EntityType::Commander;
    persist::save_game(&game_ref(g), &path).expect("save");
    let l = persist::load_game(&path).expect("load");
    let l = l.borrow();
    assert_eq!(l.current_quad.enemies.len(), 1);
    assert_eq!(l.current_quad.klingons.len(), 1);
    let k = l.current_quad.klingons[0].as_ref().unwrap().borrow();
    assert!(
        k.id == 1 && k.sector == [3, 3] && k.energy == 600.0 && k.is_commander,
        "{k:?}"
    );
}

#[test]
fn save_and_load_backward_compatibility_missing_adversary_fields() {
    let dir = temp_dir("oldformat");
    let path = join(&dir, "OLD_FORMAT.TRK");
    let raw = r#"{
		"Enterprise": {
			"Energy": 4500.0,
			"Shields": 250.0,
			"Torpedoes": 8,
			"Condition": 1
		},
		"CurrentQuad": {
			"grid": [[0,0,0,0,0,0,0,0,0],[0,0,0,0,0,0,0,0,0],[0,0,0,0,0,0,0,0,0],[0,0,0,0,0,0,0,0,0],[0,0,0,0,1,0,0,0,0],[0,0,0,0,0,0,0,0,0],[0,0,0,0,0,0,0,0,0],[0,0,0,0,0,0,0,0,0],[0,0,0,0,0,0,0,0,0]]
		},
		"Stardate": 2500.0,
		"TimeRemaining": 20.0,
		"RemainingKlingons": 5
	}"#;
    std::fs::write(&path, raw).unwrap();
    let l = persist::load_game(&path).expect("legacy load");
    let l = l.borrow();
    assert_eq!(l.enterprise.energy, 4500.0);
    assert_eq!(l.current_quad.enemies.len(), 0);
    assert_eq!(l.current_quad.plasma_torpedoes.len(), 0);
    assert_eq!(l.current_quad.web_segments.len(), 0);
    assert_eq!(l.current_quad.klingons.len(), 0);
}

#[test]
fn save_and_load_tour_game_preserves_expanded_adversaries() {
    let dir = temp_dir("touradv");
    let path = join(&dir, "tour_adv.json");
    let tour = tour_ref(TourState::new_at(7777, 0));
    let g = tour.borrow_mut().start_current_sector(None).unwrap();
    {
        let mut gb = g.borrow_mut();
        gb.current_quad.enemies = GoVec::from_vec(vec![ptr(EnemyVessel {
            id: 30,
            faction: FactionType::Tholian,
            sector: [1, 1],
            energy: 800.0,
            special_state: 0,
            ..Default::default()
        })]);
        gb.current_quad.web_segments = GoVec::from_vec(vec![ptr(TholianWebSegment {
            coord: [1, 2],
            strength: 250.0,
        })]);
        gb.current_quad.grid[1][1] = EntityType::Tholian;
        gb.current_quad.grid[1][2] = EntityType::TholianWeb;
    }
    persist::save_tour_game(&path, Some(&g), Some(&tour)).expect("save");
    let (lg, lt) = persist::load_tour_game(&path).expect("load");
    assert!(lt.is_some());
    let lg = lg.unwrap();
    let lg = lg.borrow();
    assert_eq!(lg.current_quad.enemies.len(), 1);
    assert_eq!(
        lg.current_quad.enemies[0]
            .as_ref()
            .unwrap()
            .borrow()
            .faction,
        FactionType::Tholian
    );
    let ws = &lg.current_quad.web_segments;
    assert!(ws.len() == 1 && ws[0].as_ref().unwrap().borrow().strength == 250.0);
}

#[test]
fn save_game_error_paths() {
    let g = game_ref(GameState::new_game_with_seed(123));
    assert!(persist::save_game(&g, "").is_err());
    assert!(persist::save_game(&g, "/invalid/directory/path/save.trk").is_err());
}
