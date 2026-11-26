//! Ports of the Go tests in `cmd/wasm` (session_test.go, formatter_test.go,
//! journey_test.go, main_test.go, bridge_js_test.go), same assertions.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use sst_audio::{
    Player, SoundID, SOUND_DOCK, SOUND_EXPLOSION, SOUND_PHASER, SOUND_SHIELDS,
    SOUND_TORPEDO_LAUNCH, SOUND_WARP,
};
use sst_engine::json::GoVec;
use sst_engine::*;
use sst_wasm::bridge::{AudioHost, JsAudioPlayer, NoHost};
use sst_wasm::*;

// --- mock audio player (session_test.go) ---

#[derive(Default)]
struct MockAudioPlayer {
    sounds: RefCell<Vec<SoundID>>,
    muted: Cell<bool>,
    volume: Cell<i32>,
}

impl Player for MockAudioPlayer {
    fn play(&self, sound: SoundID) {
        self.sounds.borrow_mut().push(sound);
    }
    fn set_muted(&self, muted: bool) {
        self.muted.set(muted);
    }
    fn is_muted(&self) -> bool {
        self.muted.get()
    }
    fn set_volume(&self, vol: i32) {
        self.volume.set(vol);
    }
    fn volume(&self) -> i32 {
        self.volume.get()
    }
}

impl MockAudioPlayer {
    fn has_sound(&self, s: &SoundID) -> bool {
        self.sounds.borrow().contains(s)
    }
    fn clear(&self) {
        self.sounds.borrow_mut().clear();
    }
}

fn with_mock(s: &mut Session) -> Rc<MockAudioPlayer> {
    let mock = Rc::new(MockAudioPlayer::default());
    s.set_audio_player(mock.clone());
    mock
}

fn normal() -> Option<DifficultyProfile> {
    Some(DifficultyProfile::normal())
}

fn klingon(id: i32, sector: Coord, energy: f64) -> Ptr<Klingon> {
    ptr(Klingon {
        id,
        sector,
        energy,
        ..Default::default()
    })
}

// --- main_test.go ---

#[test]
fn bridge_compilation_and_main() {
    register_bridge();
}

// --- session_test.go ---

#[test]
fn session_new_and_execute() {
    let mut s = Session::new(12345, normal());
    let help = s.execute("help");
    assert!(help.contains("COMMANDS:"), "{help}");
    assert!(help.contains("save [slot], load [slot]"), "{help}");
    let status = s.execute("srs");
    assert!(
        status.contains("CONDITION") && status.contains("<E>"),
        "{status}"
    );
    let shield = s.execute("she 500");
    assert!(shield.contains("Shields"), "{shield}");
}

#[test]
fn session_save_and_load() {
    let mut s1 = Session::new(12345, normal());
    s1.execute("she 500");
    let saved = s1.save().expect("save");
    let mut s2 = Session::new(99999, Some(DifficultyProfile::hardcore()));
    s2.load(&saved).expect("load");
    assert_eq!(s2.game().enterprise.shields, s1.game().enterprise.shields);
}

#[test]
fn session_empty_and_invalid_commands() {
    let mut s = Session::new(0, normal());
    assert_eq!(s.execute(""), "");
    assert_eq!(s.execute("   "), "");
    let bad = s.execute("foobar");
    assert!(bad.contains("Error:") || bad.contains("unknown"), "{bad:?}");
    let err = s.execute("she 999999");
    assert!(err.contains("Cannot execute"), "{err:?}");
}

#[test]
fn session_special_commands() {
    let mut s = Session::new(12345, normal());
    assert!(s.execute("lrs").contains("LONG RANGE SENSOR SCAN"));
    assert!(s.execute("chart").contains("GALACTIC STAR CHART"));
    assert!(s.execute("dam").contains("DAMAGE CONTROL REPORT"));
    let nav = s.execute("nav 1 1");
    assert!(nav.contains("<E>"), "{nav:?}");
}

#[test]
fn session_load_corrupted_data() {
    let mut s = Session::new(12345, normal());
    assert!(s.load("this is not valid json").is_err());
}

#[test]
fn session_quit_and_exit() {
    let mut s = Session::new(12345, normal());
    for cmd in ["quit", "exit", "q", "QUIT", "EXIT"] {
        let out = s.execute(cmd);
        assert!(out.contains("Session terminated"), "{cmd}: {out:?}");
    }
}

#[test]
fn session_scenario_commands() {
    let mut s = Session::new(42, None);
    let list = s.execute("scenario list");
    assert!(list.contains("kobayashi-maru"), "{list}");
    let load = s.execute("scenario mutara");
    assert!(load.contains("MUTARA NEBULA"), "{load}");
    assert_eq!(s.game().scenario, ScenarioID::mutara_nebula());
}

#[test]
fn session_scenario_commands_unknown() {
    let mut s = Session::new(42, None);
    let out = s.execute("scenario nonexistent");
    assert!(
        out.to_lowercase().contains("unknown scenario: nonexistent"),
        "{out}"
    );
    let multi = s.execute("scenario starbase assault");
    assert!(
        multi
            .to_lowercase()
            .contains("unknown scenario: starbase-assault"),
        "{multi}"
    );
}

#[test]
fn session_audio_event_dispatch() {
    let mut s = Session::new(12345, normal());
    let mock = with_mock(&mut s);

    mock.clear();
    let out = s.execute("she 200");
    assert!(out.contains("Shields"), "{out}");
    assert!(mock.has_sound(&SOUND_SHIELDS));

    mock.clear();
    s.execute("tor 1");
    assert!(mock.has_sound(&SOUND_TORPEDO_LAUNCH));

    mock.clear();
    let mut k_sector = [3, 3];
    if s.game().enterprise.sector == k_sector {
        k_sector = [4, 4];
    }
    {
        let g = s.game_mut();
        g.current_quad.klingons = GoVec::from_vec(vec![klingon(1, k_sector, 200.0)]);
        g.current_quad.grid[k_sector[0] as usize][k_sector[1] as usize] = EntityType::Klingon;
    }
    let pha = s.execute("pha 100");
    assert!(pha.contains("PHASERS") || pha.contains("Phaser"), "{pha}");
    assert!(mock.has_sound(&SOUND_PHASER));

    mock.clear();
    s.execute("nav 1 1");
    assert!(mock.has_sound(&SOUND_WARP));

    mock.clear();
    {
        let g = s.game_mut();
        let ent = g.enterprise.sector;
        let mut sb_r = ent[0] + 1;
        if sb_r > 8 {
            sb_r = ent[0] - 1;
        }
        g.current_quad.starbase = Some([sb_r, ent[1]]);
        g.current_quad.grid[sb_r as usize][ent[1] as usize] = EntityType::Starbase;
        g.enterprise.condition = ConditionType::Green;
    }
    let dock = s.execute("doc");
    assert!(dock.contains("Docked"), "{dock}");
    assert!(mock.has_sound(&SOUND_DOCK));

    let mut d = Session::new(12345, normal());
    d.execute("she 100");
    d.execute("pha 50");
    d.execute("tor 2");
}

// --- formatter_test.go ---

fn populated(seed: i64) -> GameState {
    let mut g = GameState::new_game(seed, SkillLevel::Good, GameLength::Medium);
    let (q, s) = (g.enterprise.quad, g.enterprise.sector);
    g.populate_quadrant(q, s);
    g
}

#[test]
fn format_srs_basic() {
    let g = populated(12345);
    let out = format_srs(Some(&g));
    assert!(out.contains("<E>"));
    assert!(out.contains("Stardate:") && out.contains("Energy:"));
    assert!(out.contains("\r\n"));
    assert!(out.contains("CONDITION:"));
    assert!(out.contains("\x1b[2m"));
}

#[test]
fn format_srs_conditions() {
    let mut g = populated(12345);
    for (cond, word) in [
        (ConditionType::Green, "GREEN"),
        (ConditionType::Yellow, "YELLOW"),
        (ConditionType::Red, "RED"),
        (ConditionType::Docked, "DOCKED"),
    ] {
        g.enterprise.condition = cond;
        assert!(format_srs(Some(&g)).contains(word), "{word}");
    }
}

#[test]
fn format_srs_glyphs() {
    let mut g = populated(12345);
    g.current_quad.grid[2][3] = EntityType::Klingon;
    g.current_quad.grid[4][5] = EntityType::Starbase;
    g.current_quad.grid[6][7] = EntityType::Star;
    let out = format_srs(Some(&g));
    assert!(out.contains("+K+"));
    assert!(out.contains(">B<"));
    assert!(out.contains(" * "));
}

#[test]
fn format_lrs_basic() {
    let g = GameState::new_game(12345, SkillLevel::Good, GameLength::Medium);
    let out = format_lrs(Some(&g));
    assert!(out.contains("LONG RANGE SENSOR SCAN"));
    assert!(out.contains("-------------------"));
    assert!(out.contains(": "));
}

#[test]
fn format_chart_basic() {
    let mut g = GameState::new_game(12345, SkillLevel::Good, GameLength::Medium);
    g.enterprise.quad = [4, 4];
    g.galaxy_chart[4][4] = 105;
    g.chart_discovered[2][2] = true;
    g.galaxy_chart[2][2] = 203;
    g.chart_known_bases[3][3] = true;
    let out = format_chart(Some(&g));
    assert!(out.contains("=== GALACTIC STAR CHART ==="));
    assert!(out.contains("105"));
    assert!(out.contains("203"));
    assert!(out.contains(".B."));
    assert!(out.contains("..."));
}

#[test]
fn format_damages_basic() {
    let mut g = GameState::new_game(12345, SkillLevel::Good, GameLength::Medium);
    g.enterprise.devices[DeviceID::Warp.as_i32() as usize] = 3.5;
    let out = format_damages(Some(&g));
    assert!(out.contains("=== DAMAGE CONTROL REPORT ==="));
    assert!(out.contains("Warp Engines"));
    assert!(out.contains("DAMAGED (Repair in 3.5 stardates)"));
    assert!(out.contains("OPERATIONAL"));
}

#[test]
fn format_combat_events_basic() {
    let events = vec![
        Event::TorpedoFired(EventTorpedoFired {
            origin: [3, 3],
            angle: 0.0,
        }),
        Event::TorpedoHit(EventTorpedoHit {
            target: [3, 7],
            damage: 350.0,
            destroyed: true,
            ..Default::default()
        }),
    ];
    let out = format_combat_events(&events);
    assert!(
        out.contains("TORPEDO TRACK") && out.contains("DESTROYED"),
        "{out}"
    );
}

#[test]
fn format_combat_events_anomaly_events() {
    let events = vec![
        Event::HazardTriggered(EventHazardTriggered {
            hazard_type: "ion_storm_drift".into(),
            description: "Ion storm turbulence deflected course".into(),
            ..Default::default()
        }),
        Event::SingularityAbsorption(EventSingularityAbsorption {
            sector: [4, 4],
            weapon: "torpedo".into(),
            ..Default::default()
        }),
        Event::WormholeJump(EventWormholeJump {
            from_quad: [1, 1],
            to_quad: [5, 6],
            to_sector: [3, 3],
            ..Default::default()
        }),
    ];
    let out = format_combat_events(&events);
    assert!(out.contains("turbulence deflected course"));
    assert!(out.contains("absorbed into event horizon"));
    assert!(out.contains("Wormhole transit completed"));
}

#[test]
fn format_combat_events_anomaly_discovered() {
    let events = vec![
        Event::AnomalyDiscovered(EventAnomalyDiscovered {
            quad: [2, 3],
            env: EnvironmentType::Nebula,
        }),
        Event::AnomalyDiscovered(EventAnomalyDiscovered {
            quad: [4, 5],
            env: EnvironmentType::IonStorm,
        }),
    ];
    let out = format_combat_events(&events);
    assert!(out.contains("Entering Mutara Nebula"));
    assert!(out.contains("Entering Ion Storm"));
}

#[test]
fn format_srs_wormhole() {
    let mut g = populated(12345);
    g.current_quad.grid[3][3] = EntityType::Wormhole;
    assert!(format_srs(Some(&g)).contains(">W<"));
}

#[test]
fn format_combat_events_all_types() {
    let mv = |fq: Coord, tq: Coord, fs: Coord, ts: Coord, warp: f64| {
        Event::ShipMoved(EventShipMoved {
            from_quad: fq,
            to_quad: tq,
            from_sector: fs,
            to_sector: ts,
            warp,
            ..Default::default()
        })
    };
    let events = vec![
        Event::PhaserFired(EventPhaserFired { energy: 400.0 }),
        Event::PhaserHit(EventPhaserHit {
            target: [2, 2],
            damage: 250.0,
            destroyed: false,
            ..Default::default()
        }),
        Event::PhaserHit(EventPhaserHit {
            target: [2, 3],
            damage: 500.0,
            destroyed: true,
            ..Default::default()
        }),
        Event::ShieldTransfer(EventShieldTransfer {
            new_shields: 1500.0,
            new_energy: 3500.0,
        }),
        mv([1, 1], [1, 1], [2, 2], [4, 4], 1.0),
        mv([1, 1], [2, 2], [4, 4], [1, 1], 2.0),
        Event::ObstacleEncountered(EventObstacleEncountered {
            sector: [5, 5],
            entity: EntityType::Star,
        }),
        Event::Docked(EventDocked { starbase: [3, 3] }),
        Event::KlingonCounterAttack(EventKlingonCounterAttack {
            enemy_id: 1,
            damage: 120.0,
            ..Default::default()
        }),
        Event::SubsystemDamaged(EventSubsystemDamaged {
            device: DeviceID::Shields,
            repair_time: 2.4,
        }),
        Event::SubsystemDamaged(EventSubsystemDamaged {
            device: DeviceID(-1),
            repair_time: 1.0,
        }),
        Event::SubsystemRepaired(EventSubsystemRepaired {
            device: DeviceID::Shields,
        }),
        Event::SubsystemRepaired(EventSubsystemRepaired {
            device: DeviceID(99),
        }),
        Event::GameOver(EventGameOver {
            reason: GameOverReason::Won,
            score: 1250.0,
        }),
        Event::GameOver(EventGameOver {
            reason: GameOverReason::Destroyed,
            score: 450.0,
        }),
    ];
    let out = format_combat_events(&events);
    for sub in [
        "[PHASERS FIRED]",
        "Allocated 400 units",
        "Phaser beam hit target at [2,2]: 250 units",
        "KLINGON DESTROYED BY PHASER FIRE",
        "Deflector Shields: 1500  |  Total energy: 3500",
        "Arrived at sector [4,4]",
        "Arrived at quadrant [2,2] sector [1,1]",
        "[NAVIGATION HAZARD]",
        "[STARBASE DOCKING]",
        "[RETURN FIRE]",
        "[DAMAGE]",
        "Shield Control damaged! Repair in 2.4 stardates",
        "Subsystem damaged! Repair in 1.0 stardates",
        "[REPAIR]",
        "Shield Control has been repaired",
        "Subsystem has been repaired",
        "*** FEDERATION MISSION ACCOMPLISHED ***",
        "*** MISSION TERMINATED ***",
    ] {
        assert!(out.contains(sub), "missing {sub:?} in:\n{out}");
    }
}

#[test]
fn formatters_nil_and_empty() {
    assert_eq!(format_srs(None), "");
    assert_eq!(format_lrs(None), "");
    assert_eq!(format_chart(None), "");
    assert_eq!(format_damages(None), "");
    assert_eq!(format_scenario_briefing(None), "");
    assert_eq!(format_combat_events(&[]), "");
}

// --- journey_test.go ---

#[test]
fn wasm_journey_moving() {
    let mut s = Session::new(12345, normal());
    let mock = with_mock(&mut s);
    let start_sector = s.game().enterprise.sector;
    let start_quad = s.game().enterprise.quad;
    let initial_energy = s.game().enterprise.energy;

    let mut target = [start_sector[0] + 1, start_sector[1]];
    if target[0] > 8 {
        target[0] = start_sector[0] - 1;
    }
    s.game_mut().current_quad.grid[target[0] as usize][target[1] as usize] = EntityType::Empty;
    mock.clear();
    let out = s.execute(&format!("nav s {} {}", target[0], target[1]));
    assert_eq!(s.game().enterprise.sector, target);
    assert!(s.game().enterprise.energy < initial_energy);
    assert!(out.contains("<E>"), "{out}");

    let mut dest = [start_quad[0] + 1, start_quad[1]];
    if dest[0] > 8 {
        dest[0] = start_quad[0] - 1;
    }
    mock.clear();
    let warp = s.execute(&format!("nav q {} {} 6", dest[0], dest[1]));
    assert_eq!(s.game().enterprise.quad, dest);
    assert!(mock.has_sound(&SOUND_WARP));
    assert!(warp.contains("[WARP ENGINES ENGAGED]"), "{warp}");
}

#[test]
fn wasm_journey_fighting() {
    let mut s = Session::new(12345, normal());
    let mock = with_mock(&mut s);
    {
        let g = s.game_mut();
        g.enterprise.sector = [4, 4];
        g.enterprise.energy = 5000.0;
        g.enterprise.shields = 0.0;
        g.enterprise.torpedoes = 10;
        g.enterprise.condition = ConditionType::Red;
        g.current_quad.klingons =
            GoVec::from_vec(vec![klingon(1, [4, 7], 200.0), klingon(2, [6, 2], 350.0)]);
        g.current_quad.grid[4][7] = EntityType::Klingon;
        g.current_quad.grid[6][2] = EntityType::Klingon;
    }

    mock.clear();
    let she = s.execute("she 2000");
    assert!(she.contains("Deflector Shields:"), "{she}");
    assert!(mock.has_sound(&SOUND_SHIELDS));
    assert!(she.contains("[RETURN FIRE]"), "{she}");

    mock.clear();
    let pha = s.execute("pha 400");
    assert!(pha.contains("[PHASERS FIRED]"), "{pha}");
    assert!(mock.has_sound(&SOUND_PHASER));

    mock.clear();
    let tor1 = s.execute("tor 4 7");
    assert!(mock.has_sound(&SOUND_TORPEDO_LAUNCH));
    assert!(tor1.contains("DESTROYED"), "{tor1}");
    assert_ne!(s.game().current_quad.grid[4][7], EntityType::Klingon);

    s.execute("tor 6 2");
    if s.game().current_quad.grid[6][2] == EntityType::Klingon {
        s.execute("tor 6 2");
    }
    assert_eq!(s.game().current_quad.klingons.len(), 0);
}

#[test]
fn wasm_journey_docking() {
    let mut s = Session::new(12345, normal());
    let mock = with_mock(&mut s);
    {
        let g = s.game_mut();
        g.current_quad.starbase = Some([3, 4]);
        g.current_quad.grid[3][4] = EntityType::Starbase;
        g.current_quad.grid[3][5] = EntityType::Enterprise;
        g.enterprise.sector = [3, 5];
        g.enterprise.energy = 1200.0;
        g.enterprise.torpedoes = 2;
        g.enterprise.devices[DeviceID::Warp.as_i32() as usize] = 3.5;
        g.enterprise.devices[DeviceID::Phasers.as_i32() as usize] = 2.0;
    }
    mock.clear();
    let dock = s.execute("doc");
    assert!(mock.has_sound(&SOUND_DOCK));
    assert!(dock.contains("[STARBASE DOCKING]"), "{dock}");
    let g = s.game();
    assert_eq!(g.enterprise.condition, ConditionType::Docked);
    assert_eq!(g.enterprise.energy, 5000.0);
    assert_eq!(g.enterprise.torpedoes, 10);
    assert_eq!(g.enterprise.devices[DeviceID::Warp.as_i32() as usize], 0.0);
    assert_eq!(
        g.enterprise.devices[DeviceID::Phasers.as_i32() as usize],
        0.0
    );
}

fn first_klingon(s: &Session) -> Rc<RefCell<Klingon>> {
    s.game().current_quad.klingons[0].clone().expect("klingon")
}

#[test]
fn wasm_journey_scenario_kobayashi_maru() {
    let mut s = Session::new(12345, None);
    let out = s.execute("scenario kobayashi-maru");
    assert!(out.contains("KOBAYASHI MARU"), "{out}");
    assert_eq!(s.game().scenario, ScenarioID::kobayashi_maru());
    assert_eq!(s.game().current_quad.klingons.len(), 3);

    let k = first_klingon(&s);
    let sec = k.borrow().sector;
    k.borrow_mut().energy = 10.0;
    s.execute(&format!("tor {} {}", sec[0], sec[1]));
    assert_eq!(s.game().current_quad.klingons.len(), 3);

    s.game_mut().enterprise.energy = 0.0;
    let (done, won, reason) = evaluate_kobayashi_maru(s.game_mut());
    assert!(done && !won && reason == GameOverReason::Lost);
    let score = compute_score_kobayashi_maru(s.game(), false);
    assert!(
        score.rank_title.contains("Commendation") || score.rank_title.contains("Citation"),
        "{}",
        score.rank_title
    );
}

#[test]
fn wasm_journey_scenario_mutara_nebula() {
    let mut s = Session::new(12345, None);
    let out = s.execute("scenario mutara");
    assert!(out.contains("MUTARA NEBULA"), "{out}");
    assert_eq!(s.game().scenario, ScenarioID::mutara_nebula());
    assert_eq!(s.game().enterprise.shields, 0.0);

    s.execute("she 1000");
    assert_eq!(s.game().enterprise.shields, 0.0);

    assert!(!s.game().current_quad.klingons.is_empty());
    let k = first_klingon(&s);
    let sec = k.borrow().sector;
    k.borrow_mut().energy = 10.0;
    let tor = s.execute(&format!("tor {} {}", sec[0], sec[1]));
    assert!(tor.contains("MISSION ACCOMPLISHED"), "{tor}");
    let (done, won, reason) = evaluate_mutara_nebula(s.game_mut());
    assert!(done && won && reason == GameOverReason::Won);
}

#[test]
fn wasm_journey_scenario_starbase_siege() {
    let mut s = Session::new(12345, None);
    let out = s.execute("scenario siege");
    assert!(out.contains("STARBASE UNDER SIEGE"), "{out}");
    assert_eq!(s.game().scenario, ScenarioID::starbase_siege());

    s.execute("nav q 4 4 6");
    assert_eq!(s.game().enterprise.quad, [4, 4]);
    assert!(s.game().current_quad.starbase.is_some());

    for k in s.game().current_quad.klingons.iter().flatten() {
        k.borrow_mut().energy = 10.0;
    }
    s.game_mut().enterprise.energy = 3000.0;
    s.execute("pha 2000");
    let (done, won, reason) = evaluate_starbase_siege(s.game_mut());
    assert!(done && won && reason == GameOverReason::Won);
}

#[test]
fn wasm_journey_save_and_load() {
    let mut s1 = Session::new(12345, normal());
    s1.execute("she 800");
    s1.execute("nav 1 1");
    let saved = s1.save().expect("save");
    let mut s2 = Session::new(99999, Some(DifficultyProfile::hardcore()));
    s2.load(&saved).expect("load");
    assert_eq!(s2.game().enterprise.shields, s1.game().enterprise.shields);
    assert_eq!(s2.game().enterprise.sector, s1.game().enterprise.sector);
}

/// The deliberate deviation: after `Load` the PRNG is reseeded, so entering a
/// new quadrant (a random draw) works instead of panicking like Go.
#[test]
fn load_reseeds_prng_instead_of_panicking() {
    let mut s1 = Session::new(12345, normal());
    s1.execute("nav 2 2");
    let saved = s1.save().expect("save");
    let mut s2 = Session::new(1, normal());
    s2.load(&saved).expect("load");
    let rng = s2.game().rng.as_ref().expect("rng");
    assert_eq!(
        rng.state(),
        PRNG::new(s2.game().stardate as i64).state(),
        "reseeded like LoadGame"
    );
    let out = s2.execute("nav q 1 1 6");
    assert!(out.contains("[WARP ENGINES ENGAGED]"), "{out}");
}

// --- bridge_js_test.go (via a fake JS host) ---

#[derive(Default)]
struct FakeHost {
    has_mute_fns: bool,
    has_play_fn: bool,
    js_muted: Cell<bool>,
    set_muted_called_with: Cell<bool>,
    played: RefCell<Vec<String>>,
}

impl AudioHost for FakeHost {
    fn play_sound(&self, id: &str) {
        if self.has_play_fn {
            self.played.borrow_mut().push(id.to_string());
        }
    }
    fn set_muted(&self, muted: bool) {
        if self.has_mute_fns {
            self.js_muted.set(muted);
            self.set_muted_called_with.set(muted);
        }
    }
    fn is_muted(&self) -> Option<bool> {
        self.has_mute_fns.then(|| self.js_muted.get())
    }
    fn set_volume(&self, _vol: i32) {}
    fn get_volume(&self) -> Option<i32> {
        None
    }
}

#[test]
fn js_audio_player_fallback_without_window_functions() {
    let p = JsAudioPlayer::new(NoHost, 0);
    assert!(!p.is_muted());
    p.set_muted(true);
    assert!(p.is_muted());
    p.set_muted(false);
    assert!(!p.is_muted());
}

#[test]
fn js_audio_player_sync_with_window_mute_functions() {
    let host = Rc::new(FakeHost {
        has_mute_fns: true,
        ..Default::default()
    });
    let p = JsAudioPlayer::new(host.clone(), 0);
    p.set_muted(true);
    assert!(host.set_muted_called_with.get() && host.js_muted.get());
    assert!(p.is_muted());
    host.js_muted.set(false);
    assert!(!p.is_muted());
    p.set_muted(false);
    assert!(!p.is_muted());
}

#[test]
fn js_audio_player_play_delegation() {
    let host = Rc::new(FakeHost {
        has_play_fn: true,
        ..Default::default()
    });
    let p = JsAudioPlayer::new(host.clone(), 0);
    p.play(SOUND_PHASER);
    assert_eq!(
        *host.played.borrow(),
        vec![SOUND_PHASER.as_str().to_string()]
    );
    p.set_muted(true);
    p.play(SOUND_EXPLOSION);
    assert_eq!(host.played.borrow().len(), 1);
}

#[test]
fn js_audio_player_volume() {
    let p = JsAudioPlayer::new(NoHost, 80);
    assert_eq!(p.volume(), 80);
    p.set_volume(50);
    assert_eq!(p.volume(), 50);
    p.set_volume(-20);
    assert_eq!(p.volume(), 0);
    p.set_volume(120);
    assert_eq!(p.volume(), 100);
}
