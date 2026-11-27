//! Super Star Trek command-line entry point (port of `cmd/sst/main.go` and
//! `pkg/classic`).
//!
//! [`run`] reproduces Go's `run(args, in, out, errOut)`: the `--classic`
//! fast path, Go `flag` parsing ([`flag`]), configuration merging, and
//! launching the TUI through an injectable program runner
//! ([`set_run_program`], Go's `runProgram` variable).

pub mod classic;
pub mod flag;

use std::cell::RefCell;
use std::io::{Read, Write};
use std::rc::Rc;

use sst_engine::gofmt;
use sst_engine::gotime::{Clock, SystemClock};
use sst_engine::{
    default_rules_for_profile, game_ref, get_scenario, list_scenarios, persist::new_tour,
    DifficultyProfile, GameLength, GameState, ScenarioID, SkillLevel, SurveillanceMode,
};
use sst_tui::config::{self, AudioMode, AUDIO_MODE_OFF};
use sst_tui::theme;
use sst_tuikit::tea::{with_alt_screen, with_mouse_cell_motion, Program, ProgramOption};

/// Release version: the cargo package version, unless a release build
/// overrides it by setting `SST_VERSION` at build time.
pub const VERSION: &str = match option_env!("SST_VERSION") {
    Some(v) => v,
    None => env!("CARGO_PKG_VERSION"),
};

/// Build commit (Go `commit`), from `SST_COMMIT` at build time.
pub const COMMIT: &str = match option_env!("SST_COMMIT") {
    Some(v) => v,
    None => "none",
};

/// Build date (Go `date`), from `SST_BUILD_DATE` at build time.
pub const DATE: &str = match option_env!("SST_BUILD_DATE") {
    Some(v) => v,
    None => "unknown",
};

/// Go `runProgram`: runs the TUI model with the given options.
pub type RunProgramFn = Box<dyn FnMut(sst_tui::Model, &[ProgramOption]) -> Result<(), String>>;

thread_local! {
    static RUN_PROGRAM: RefCell<Option<RunProgramFn>> = const { RefCell::new(None) };
}

/// Replaces the program runner for this thread (`None` restores the real
/// bubbletea runtime) and returns the previous override, like assigning
/// Go's `runProgram` variable in tests.
pub fn set_run_program(f: Option<RunProgramFn>) -> Option<RunProgramFn> {
    RUN_PROGRAM.with(|r| std::mem::replace(&mut *r.borrow_mut(), f))
}

fn default_run_program(m: sst_tui::Model, opts: &[ProgramOption]) -> Result<(), String> {
    Program::new(m, opts)
        .run()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

fn run_program(m: sst_tui::Model, opts: &[ProgramOption]) -> Result<(), String> {
    let hook = RUN_PROGRAM.with(|r| r.borrow_mut().take());
    match hook {
        Some(mut f) => {
            let res = f(m, opts);
            RUN_PROGRAM.with(|r| {
                let mut slot = r.borrow_mut();
                if slot.is_none() {
                    *slot = Some(f);
                }
            });
            res
        }
        None => default_run_program(m, opts),
    }
}

/// Go `isClassic(args)`.
pub fn is_classic(args: &[String]) -> bool {
    args.iter().any(|arg| {
        arg == "--classic"
            || arg == "-classic"
            || arg.starts_with("--classic=")
            || arg.starts_with("-classic=")
    })
}

/// Go `run(args, in, out, errOut)`: returns the process exit code.
pub fn run(
    args: &[String],
    stdin: &mut dyn Read,
    out: &mut dyn Write,
    err_out: &mut dyn Write,
) -> i32 {
    if is_classic(args) {
        return classic::run_classic_cli(stdin, out, args);
    }

    let mut fs = flag::FlagSet::new("sst");
    let help_to_stdout = args
        .iter()
        .any(|a| a == "--help" || a == "-h" || a == "-help");
    let _ = fs.bool("classic", false, "run in teletype plain mode");
    let version_flag = fs.bool("version", false, "print version information and exit");
    fs.bool_var(&version_flag, "v", false, "shorthand for --version");
    let theme_name = fs.string("theme", "modern", "initial theme name (modern, lcars, crt)");
    let mode_flag = fs.string("mode", "auto", "theme color mode (auto, dark, light)");
    let seed = fs.int64("seed", 0, "PRNG seed (0 for random)");
    let difficulty = fs.string(
        "difficulty",
        "normal",
        "difficulty profile (casual, normal, hardcore, nightmare, expert, emeritus)",
    );
    let surveillance = fs.string(
        "surveillance",
        "",
        "surveillance extent (full, classic, local, blackout)",
    );
    let sensor_degradation = fs.bool(
        "sensor-degradation",
        true,
        "enable two-tier sensor degradation curve",
    );
    let repair_mult = fs.float64(
        "repair-multiplier",
        1.0,
        "subsystem repair duration multiplier",
    );
    let klingon_cloak = fs.bool(
        "klingon-cloak",
        false,
        "enable Klingon commander tactical cloaking",
    );
    let anomalies = fs.bool(
        "anomalies",
        false,
        "enable spatial anomalies & environmental hazards",
    );
    let no_anomalies = fs.bool(
        "no-anomalies",
        false,
        "disable spatial anomalies & environmental hazards",
    );
    let adversaries_flag = fs.bool(
        "adversaries",
        false,
        "enable Romulan and Tholian adversary factions in standard game",
    );
    fs.bool_var(&adversaries_flag, "a", false, "shorthand for --adversaries");
    let sound = fs.bool("sound", true, "enable retro procedural audio and sound FX");
    let no_sound = fs.bool(
        "no-sound",
        false,
        "disable retro procedural audio and sound FX",
    );
    let flag_volume = fs.int("volume", -1, "Set audio playback volume (0-100)");
    let flag_mute = fs.bool("mute", false, "Mute all audio playback");
    let flag_audio_mode = fs.string("audio-mode", "", "Set audio mode (auto, native, bell, off)");
    let scenario_flag = fs.string("scenario", "", "launch specific tactical scenario");
    fs.string_var(&scenario_flag, "s", "", "shorthand for --scenario");
    let list_scenarios_flag = fs.bool(
        "list-scenarios",
        false,
        "display available tactical scenarios",
    );
    let tour_flag = fs.bool(
        "tour",
        false,
        "Launch in Starfleet Career & Campaign (Patrol Tour) mode",
    );
    fs.bool_var(&tour_flag, "campaign", false, "alias for --tour");

    let parse_res = if help_to_stdout {
        fs.parse(args, out)
    } else {
        fs.parse(args, err_out)
    };
    if let Err(e) = parse_res {
        return if e == flag::FlagError::Help { 0 } else { 1 };
    }

    if version_flag.get() {
        let _ = writeln!(
            out,
            "sst version {VERSION} (commit: {COMMIT}, built at: {DATE})"
        );
        return 0;
    }

    if list_scenarios_flag.get() {
        let _ = writeln!(out, "Available Tactical Scenarios:");
        for sc in list_scenarios() {
            let _ = writeln!(
                out,
                "  {} [{}] {} - {}",
                gofmt::pad_right(sc.id.as_str(), 16),
                sc.difficulty,
                sc.name,
                sc.description
            );
        }
        return 0;
    }

    let mut sc = None;
    let scenario_name = scenario_flag.borrow().clone();
    if !scenario_name.is_empty() {
        match get_scenario(&ScenarioID::new(scenario_name.clone())) {
            Some(s) => sc = Some(s),
            None => {
                let _ = writeln!(
                    err_out,
                    "Error: unknown scenario {}",
                    gofmt::quote(&scenario_name)
                );
                return 1;
            }
        }
    }

    let parsed_mode = match theme::parse_color_mode(&mode_flag.borrow()) {
        Ok(m) => m,
        Err(e) => {
            let _ = writeln!(err_out, "Error: {e}");
            return 1;
        }
    };

    let mut visited = std::collections::BTreeSet::new();
    fs.visit(|f| {
        visited.insert(f.name.clone());
    });
    let visited = |n: &str| visited.contains(n);

    if visited("anomalies") && visited("no-anomalies") {
        let _ = writeln!(
            err_out,
            "Error: cannot specify both --anomalies and --no-anomalies"
        );
        return 1;
    }

    if visited("sound") && visited("no-sound") {
        let _ = writeln!(err_out, "Error: cannot specify both --sound and --no-sound");
        return 1;
    }

    let mut cfg = config::load_config().unwrap_or_else(|_| config::default_config());

    if visited("sound") {
        cfg.muted = !sound.get();
    }
    if visited("no-sound") {
        cfg.muted = no_sound.get();
    }
    if visited("mute") {
        cfg.muted = flag_mute.get();
    }
    if visited("volume") {
        // Go int is 64-bit; Normalize clamps to 0..=100 either way.
        cfg.volume = flag_volume.get().clamp(i32::MIN as i64, i32::MAX as i64) as i32;
    }
    if visited("audio-mode") {
        cfg.audio_mode = AudioMode(flag_audio_mode.borrow().clone());
        if cfg.audio_mode.as_str() == AUDIO_MODE_OFF {
            cfg.muted = true;
        }
    }
    cfg.normalize();

    let mut rules = default_rules_for_profile(&DifficultyProfile::new(difficulty.borrow().clone()));
    if visited("surveillance") {
        rules.surveillance = SurveillanceMode::new(surveillance.borrow().clone());
        rules.profile = DifficultyProfile::custom();
    }
    if visited("sensor-degradation") {
        rules.sensor_degradation = sensor_degradation.get();
        rules.profile = DifficultyProfile::custom();
    }
    if visited("repair-multiplier") {
        rules.repair_multiplier = repair_mult.get();
        rules.profile = DifficultyProfile::custom();
    }
    if visited("klingon-cloak") {
        rules.klingon_cloak = klingon_cloak.get();
        rules.profile = DifficultyProfile::custom();
    }
    if visited("anomalies") {
        rules.spatial_anomalies = anomalies.get();
        rules.profile = DifficultyProfile::custom();
    }
    if visited("no-anomalies") {
        rules.spatial_anomalies = !no_anomalies.get();
        rules.profile = DifficultyProfile::custom();
    }
    if visited("adversaries") || visited("a") {
        rules.adversaries = adversaries_flag.get();
        rules.profile = DifficultyProfile::custom();
    }

    let mut s = seed.get();
    if s == 0 {
        s = SystemClock.now().unix_nano();
    }

    let selected_theme = theme::get_theme(&theme_name.borrow()).with_color_mode(parsed_mode);

    let mut p = if tour_flag.get() {
        let tour = Rc::new(RefCell::new(new_tour(s)));
        sst_tui::new_model_with_tour(Some(tour), selected_theme)
    } else {
        let game = match &sc {
            Some(sc) => sc.build(s),
            None => {
                let mut g = GameState::new_game_with_options(
                    s,
                    SkillLevel::Good,
                    GameLength::Medium,
                    rules,
                );
                let (q, sec) = (g.enterprise.quad, g.enterprise.sector);
                g.populate_quadrant(q, sec);
                g
            }
        };
        sst_tui::new_model(Some(game_ref(game)), selected_theme)
    };
    p.set_sound_enabled(!cfg.muted);
    if let Some(pl) = &p.audio_player {
        pl.set_volume(cfg.volume);
    }
    if let Err(e) = run_program(p, &[with_alt_screen(), with_mouse_cell_motion()]) {
        let _ = writeln!(err_out, "Error running game: {e}");
        return 1;
    }
    0
}
