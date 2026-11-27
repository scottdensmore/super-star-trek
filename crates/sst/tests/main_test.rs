//! Port of `cmd/sst/main_test.go`.

mod common;

use std::cell::Ref;

use common::{capture, run, Env};
use sst_engine::{DifficultyProfile, GameState, ScenarioID, SurveillanceMode};
use sst_tui::theme::ColorMode;
use sst_tuikit::tea::{Program, WITH_ALT_SCREEN, WITH_MOUSE_CELL_MOTION};
use super_star_trek::flag::FlagSet;
use super_star_trek::is_classic;

fn game(m: &sst_tui::Model) -> Ref<'_, GameState> {
    m.game.as_ref().expect("game").borrow()
}

#[test]
fn test_is_classic() {
    let tests: &[(&str, &[&str], bool)] = &[
        ("empty args", &[], false),
        ("classic long flag", &["--classic"], true),
        ("classic short flag", &["-classic"], true),
        ("classic long flag with value", &["--classic=true"], true),
        ("classic short flag with value", &["-classic=true"], true),
        (
            "classic flag preceded by other flags",
            &["-seed", "123", "--classic"],
            true,
        ),
        (
            "classic flag followed by classic flags",
            &["-classic", "-seed", "456"],
            true,
        ),
        ("tui theme flag only", &["--theme", "lcars"], false),
        ("tui seed flag only", &["-seed", "123"], false),
        (
            "flag that merely has classic in name",
            &["--classic-rock"],
            false,
        ),
    ];
    for (name, args, want) in tests {
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        assert_eq!(is_classic(&args), *want, "{name}: isClassic({args:?})");
    }
}

#[test]
fn test_run_classic_mode() {
    let _env = Env::new();
    let (code, out, _) = run(&["--classic"]);
    assert_eq!(code, 0, "expected exit code 0 for --classic");
    assert!(
        out.contains("Super Star Trek\n"),
        "expected classic banner in output, got {out:?}"
    );
    assert!(
        out.contains("Shields:"),
        "expected shield telemetry, got {out:?}"
    );

    let (code, out, _) = run(&["-classic", "-seed", "999"]);
    assert_eq!(code, 0, "expected exit code 0 for -classic -seed 999");
    assert!(out.contains("Super Star Trek\n"), "got {out:?}");
}

#[test]
fn test_run_classic_mode_unknown_flag() {
    let _env = Env::new();
    let (code, _, _) = run(&["--classic", "-unknownflag"]);
    assert_ne!(
        code, 0,
        "expected non-zero exit code for unknown classic flag"
    );
}

#[test]
fn test_run_tui_mode_valid_flags() {
    let _env = Env::new();
    let c = capture(Ok(()));
    let (code, _, _) = run(&["--theme", "lcars", "-seed", "42"]);
    assert_eq!(code, 0);
    assert_eq!(c.borrow().calls, 1, "expected runProgram to be called");

    let (code, _, _) = run(&["--theme", "crt"]);
    assert_eq!(code, 0);
    assert_eq!(c.borrow().calls, 2, "expected runProgram for crt theme");

    let (code, _, _) = run(&[]);
    assert_eq!(code, 0);
    assert_eq!(c.borrow().calls, 3, "expected runProgram with no args");
}

#[test]
fn test_run_tui_help() {
    let _env = Env::new();
    let (code, _, _) = run(&["--help"]);
    assert_eq!(code, 0, "expected exit code 0 for --help");
    let (code, _, _) = run(&["-h"]);
    assert_eq!(code, 0, "expected exit code 0 for -h");
}

#[test]
fn test_run_tui_unknown_flag() {
    let _env = Env::new();
    let (code, _, err) = run(&["--unsupported-flag"]);
    assert_ne!(code, 0);
    assert!(!err.is_empty(), "expected error message in stderr");
}

#[test]
fn test_run_tui_program_error() {
    let _env = Env::new();
    let _c = capture(Err("terminal failure".into()));
    let (code, _, err) = run(&[]);
    assert_eq!(code, 1, "expected exit code 1 when runProgram fails");
    assert!(err.contains("terminal failure"), "got {err:?}");
}

#[test]
fn test_run_tui_mouse_cell_motion_option() {
    let _env = Env::new();
    let c = capture(Ok(()));
    let (code, _, _) = run(&[]);
    assert_eq!(code, 0);
    let cc = c.borrow_mut();
    let m = cc.model.clone().unwrap();
    let opts = cc.opts.clone();
    drop(cc);
    let p = Program::new(m, &opts);
    let mask = p.startup_options();
    assert_ne!(
        mask & WITH_MOUSE_CELL_MOTION,
        0,
        "expected WithMouseCellMotion: {mask:b}"
    );
    assert_ne!(
        mask & WITH_ALT_SCREEN,
        0,
        "expected WithAltScreen: {mask:b}"
    );
}

#[test]
fn test_cli_flags_difficulty_and_surveillance() {
    let _env = Env::new();
    let c = capture(Ok(()));
    let (code, _, err) = run(&["--difficulty=nightmare", "--seed=12345"]);
    assert_eq!(code, 0, "run failed: {err}");
    let m = c.borrow().model.clone().unwrap();
    let g = game(&m);
    assert_eq!(g.rules.profile, DifficultyProfile::nightmare());
    assert_eq!(g.rules.surveillance, SurveillanceMode::blackout());
    assert!(
        g.rules.klingon_cloak,
        "expected KlingonCloak=true for nightmare"
    );
    assert_eq!(g.rules.repair_multiplier, 2.0);
}

#[test]
fn test_cli_flags_overrides() {
    let _env = Env::new();
    let c = capture(Ok(()));
    let (code, _, err) = run(&[
        "--difficulty=nightmare",
        "--surveillance=full",
        "--klingon-cloak=false",
        "--repair-multiplier=1.25",
        "--sensor-degradation=false",
    ]);
    assert_eq!(code, 0, "run failed: {err}");
    let m = c.borrow().model.clone().unwrap();
    let g = game(&m);
    assert_eq!(g.rules.profile, DifficultyProfile::custom());
    assert_eq!(g.rules.surveillance, SurveillanceMode::full());
    assert!(!g.rules.klingon_cloak);
    assert_eq!(g.rules.repair_multiplier, 1.25);
    assert!(!g.rules.sensor_degradation);
}

#[test]
fn test_cli_flags_spatial_anomalies() {
    let _env = Env::new();
    let c = capture(Ok(()));
    let tests: &[(&str, &[&str], bool, DifficultyProfile)] = &[
        (
            "default normal difficulty has anomalies disabled",
            &["-seed", "42"],
            false,
            DifficultyProfile::normal(),
        ),
        (
            "enable anomalies with --anomalies",
            &["--anomalies", "-seed", "42"],
            true,
            DifficultyProfile::custom(),
        ),
        (
            "enable anomalies with -anomalies",
            &["-anomalies", "-seed", "42"],
            true,
            DifficultyProfile::custom(),
        ),
        (
            "disable anomalies with --no-anomalies on nightmare",
            &["--difficulty=nightmare", "--no-anomalies", "-seed", "42"],
            false,
            DifficultyProfile::custom(),
        ),
        (
            "disable anomalies with -no-anomalies",
            &["-difficulty=nightmare", "-no-anomalies", "-seed", "42"],
            false,
            DifficultyProfile::custom(),
        ),
        (
            "anomalies flag set to false",
            &["--difficulty=nightmare", "--anomalies=false", "-seed", "42"],
            false,
            DifficultyProfile::custom(),
        ),
    ];
    for (name, args, want_anom, want_prof) in tests {
        let (code, _, err) = run(args);
        assert_eq!(code, 0, "{name}: run failed: {err}");
        let m = c.borrow().model.clone().unwrap();
        let g = game(&m);
        assert_eq!(g.rules.spatial_anomalies, *want_anom, "{name}");
        assert_eq!(&g.rules.profile, want_prof, "{name}");
    }
}

#[test]
fn test_cli_flags_spatial_anomalies_mutual_exclusion() {
    let _env = Env::new();
    let c = capture(Ok(()));
    let (code, _, err) = run(&["--anomalies", "--no-anomalies"]);
    assert_eq!(code, 1);
    assert_eq!(c.borrow().calls, 0, "expected runProgram NOT to be called");
    let want = "Error: cannot specify both --anomalies and --no-anomalies";
    assert!(
        err.contains(want),
        "expected stderr to contain {want:?}, got {err:?}"
    );
}

#[test]
fn test_cli_flags_sound() {
    let _env = Env::new();
    let c = capture(Ok(()));
    let tests: &[(&str, &[&str], bool)] = &[
        ("default has sound enabled", &["-seed", "42"], true),
        (
            "enable sound explicitly with --sound",
            &["--sound", "-seed", "42"],
            true,
        ),
        (
            "enable sound explicitly with -sound",
            &["-sound", "-seed", "42"],
            true,
        ),
        (
            "disable sound with --no-sound",
            &["--no-sound", "-seed", "42"],
            false,
        ),
        (
            "disable sound with -no-sound",
            &["-no-sound", "-seed", "42"],
            false,
        ),
        (
            "sound flag set to false with --sound=false",
            &["--sound=false", "-seed", "42"],
            false,
        ),
        (
            "no-sound flag set to false with --no-sound=false",
            &["--no-sound=false", "-seed", "42"],
            true,
        ),
    ];
    for (name, args, want) in tests {
        let (code, _, err) = run(args);
        assert_eq!(code, 0, "{name}: run failed: {err}");
        let m = c.borrow().model.clone().unwrap();
        assert_eq!(m.sound_enabled(), *want, "{name}");
    }
}

#[test]
fn test_cli_flags_sound_mutual_exclusion() {
    let _env = Env::new();
    let c = capture(Ok(()));
    let want = "Error: cannot specify both --sound and --no-sound";
    let (code, _, err) = run(&["--sound", "--no-sound"]);
    assert_eq!(code, 1);
    assert_eq!(c.borrow().calls, 0);
    assert!(err.contains(want), "got {err:?}");

    let (code, _, err) = run(&["-sound", "-no-sound"]);
    assert_eq!(code, 1);
    assert_eq!(c.borrow().calls, 0);
    assert!(err.contains(want), "got {err:?}");
}

#[test]
fn test_cli_mode_flag() {
    let _env = Env::new();
    struct Case {
        name: &'static str,
        args: &'static [&'static str],
        expected_err: bool,
        expected_mode: ColorMode,
        err_substring: &'static str,
    }
    let ok = |name, args, mode| Case {
        name,
        args,
        expected_err: false,
        expected_mode: mode,
        err_substring: "",
    };
    let tests = [
        ok("flag mode light", &["-mode", "light"][..], ColorMode::Light),
        ok("flag mode dark", &["-mode", "dark"], ColorMode::Dark),
        ok("flag mode auto", &["-mode", "auto"], ColorMode::Auto),
        ok(
            "long flag with equals light",
            &["--mode=light"],
            ColorMode::Light,
        ),
        ok(
            "long flag with equals dark",
            &["--mode=dark"],
            ColorMode::Dark,
        ),
        ok(
            "flag mode case insensitive",
            &["-mode", "DARK"],
            ColorMode::Dark,
        ),
        ok(
            "default without mode flag defaults to auto",
            &[],
            ColorMode::Auto,
        ),
        ok(
            "combined theme and mode",
            &["-theme", "lcars", "-mode", "light"],
            ColorMode::Light,
        ),
        Case {
            name: "invalid mode",
            args: &["-mode", "invalid"],
            expected_err: true,
            expected_mode: ColorMode::Auto,
            err_substring: "invalid color mode",
        },
        Case {
            name: "invalid mode unknown",
            args: &["--mode=solar"],
            expected_err: true,
            expected_mode: ColorMode::Auto,
            err_substring: "invalid color mode",
        },
    ];
    for tt in tests {
        let c = capture(Ok(()));
        let (code, _, err) = run(tt.args);
        if tt.expected_err {
            assert_ne!(code, 0, "{}", tt.name);
            assert_eq!(c.borrow().calls, 0, "{}: runProgram called", tt.name);
            assert!(err.contains(tt.err_substring), "{}: got {err:?}", tt.name);
        } else {
            assert_eq!(code, 0, "{}: {err}", tt.name);
            assert_eq!(c.borrow().calls, 1, "{}", tt.name);
            let m = c.borrow().model.clone().unwrap();
            assert_eq!(
                m.theme.unwrap().color_mode(),
                tt.expected_mode,
                "{}",
                tt.name
            );
        }
    }
}

#[test]
fn test_cli_list_scenarios() {
    let _env = Env::new();
    let (code, out, _) = run(&["--list-scenarios"]);
    assert_eq!(code, 0);
    assert!(
        out.contains("kobayashi-maru") && out.contains("mutara-nebula"),
        "expected scenario list, got: {out}"
    );
}

#[test]
fn test_cli_scenario_launch_unknown() {
    let _env = Env::new();
    let (code, _, err) = run(&["--scenario=nonexistent"]);
    assert_eq!(code, 1);
    assert!(err.contains("unknown scenario"), "got: {err}");
}

#[test]
fn test_cli_scenario_launch() {
    let _env = Env::new();
    let c = capture(Ok(()));
    let (code, _, err) = run(&["--scenario", "mutara-nebula"]);
    assert_eq!(code, 0, "{err}");
    let m = c.borrow().model.clone().unwrap();
    assert_eq!(game(&m).scenario, ScenarioID::mutara_nebula());
}

#[test]
fn test_cli_scenario_launch_shorthand() {
    let _env = Env::new();
    let c = capture(Ok(()));
    let (code, _, err) = run(&["-s", "kobayashi"]);
    assert_eq!(code, 0, "{err}");
    let m = c.borrow().model.clone().unwrap();
    assert_eq!(game(&m).scenario, ScenarioID::kobayashi_maru());
}

#[test]
fn test_main_version_flag() {
    let _env = Env::new();
    for flag in ["--version", "-v"] {
        let (code, out, err) = run(&[flag]);
        assert_eq!(code, 0, "{flag}: stderr {err}");
        assert!(out.contains("sst version"), "{out:?}");
        assert!(out.contains("commit:"), "{out:?}");
        assert!(out.contains("built at:"), "{out:?}");
    }
}

#[test]
fn version_defaults_to_the_cargo_package_version() {
    // Release builds override it with SST_VERSION; otherwise it is exactly
    // the crate version (Go reported a hard-coded "2.0.0-dev").
    if option_env!("SST_VERSION").is_none() {
        assert_eq!(super_star_trek::VERSION, env!("CARGO_PKG_VERSION"));
    }
    let _env = Env::new();
    let (code, out, _) = run(&["--version"]);
    assert_eq!(code, 0);
    assert_eq!(
        out,
        format!(
            "sst version {} (commit: {}, built at: {})\n",
            super_star_trek::VERSION,
            super_star_trek::COMMIT,
            super_star_trek::DATE
        )
    );
    assert!(!out.contains("-dev"), "{out:?}");
}

#[test]
fn test_main_tour_flag() {
    let _env = Env::new();
    let (code, out, _) = run(&["--help"]);
    assert_eq!(code, 0);
    assert!(out.contains("--tour"), "expected --tour in help:\n{out}");
}

#[test]
fn test_cli_tour_launch() {
    let _env = Env::new();
    let c = capture(Ok(()));
    let (code, _, err) = run(&["--tour", "-seed", "42"]);
    assert_eq!(code, 0, "{err}");
    let m = c.borrow().model.clone().unwrap();
    let tour = m.tour.as_ref().expect("expected active Tour in model");
    assert!(tour.borrow().active, "expected active Tour in model");
    assert_eq!(tour.borrow().current_sector_index, 0);
    assert!(m.game.is_some(), "expected Game initialized in model");
}

#[test]
fn test_cli_campaign_launch() {
    let _env = Env::new();
    let c = capture(Ok(()));
    let (code, _, err) = run(&["--campaign", "-seed", "42"]);
    assert_eq!(code, 0, "{err}");
    let m = c.borrow().model.clone().unwrap();
    let tour = m.tour.as_ref().expect("expected active Tour in model");
    assert!(tour.borrow().active);
}

fn check_adversaries(tests: &[(&str, &[&str], bool, DifficultyProfile)]) {
    let c = capture(Ok(()));
    for (name, args, want_adv, want_prof) in tests {
        let (code, _, err) = run(args);
        assert_eq!(code, 0, "{name}: {err}");
        let m = c.borrow().model.clone().unwrap();
        let g = game(&m);
        assert_eq!(g.rules.adversaries, *want_adv, "{name}");
        assert_eq!(&g.rules.profile, want_prof, "{name}");
    }
}

#[test]
fn test_cli_flags_adversaries() {
    let _env = Env::new();
    check_adversaries(&[
        (
            "default normal difficulty has adversaries disabled",
            &["-seed", "42"],
            false,
            DifficultyProfile::normal(),
        ),
        (
            "enable adversaries with --adversaries",
            &["--adversaries", "-seed", "42"],
            true,
            DifficultyProfile::custom(),
        ),
        (
            "enable adversaries with -adversaries",
            &["-adversaries", "-seed", "42"],
            true,
            DifficultyProfile::custom(),
        ),
        (
            "enable adversaries with -a shorthand",
            &["-a", "-seed", "42"],
            true,
            DifficultyProfile::custom(),
        ),
        (
            "explicit adversaries false",
            &["--adversaries=false", "-seed", "42"],
            false,
            DifficultyProfile::custom(),
        ),
    ]);
}

#[test]
fn test_cli_flags_difficulty_expert_and_emeritus() {
    let _env = Env::new();
    check_adversaries(&[
        (
            "expert difficulty enables adversaries by default",
            &["--difficulty=expert", "-seed", "42"],
            true,
            DifficultyProfile::expert(),
        ),
        (
            "emeritus difficulty enables adversaries by default",
            &["--difficulty=emeritus", "-seed", "42"],
            true,
            DifficultyProfile::emeritus(),
        ),
        (
            "expert difficulty with adversaries disabled override",
            &["--difficulty=expert", "--adversaries=false", "-seed", "42"],
            false,
            DifficultyProfile::custom(),
        ),
    ]);
}

struct AudioFlags {
    volume: i64,
    mute: bool,
    audio_mode: String,
}

fn parse_flags(args: &[&str]) -> AudioFlags {
    let mut fs = FlagSet::new("sst");
    let flag_volume = fs.int("volume", -1, "Set audio playback volume (0-100)");
    let flag_mute = fs.bool("mute", false, "Mute all audio playback");
    let flag_audio_mode = fs.string("audio-mode", "", "Set audio mode (auto, native, bell, off)");
    let _ = fs.bool("classic", false, "");
    let _ = fs.bool("version", false, "");
    let _ = fs.bool("v", false, "");
    let _ = fs.string("theme", "modern", "");
    let _ = fs.string("mode", "auto", "");
    let _ = fs.int64("seed", 0, "");
    let _ = fs.string("difficulty", "normal", "");
    let _ = fs.string("surveillance", "", "");
    let _ = fs.bool("sensor-degradation", true, "");
    let _ = fs.float64("repair-multiplier", 1.0, "");
    let _ = fs.bool("klingon-cloak", false, "");
    let _ = fs.bool("anomalies", false, "");
    let _ = fs.bool("no-anomalies", false, "");
    let _ = fs.bool("adversaries", false, "");
    let _ = fs.bool("a", false, "");
    let _ = fs.bool("sound", true, "");
    let _ = fs.bool("no-sound", false, "");
    let _ = fs.string("scenario", "", "");
    let _ = fs.string("s", "", "");
    let _ = fs.bool("list-scenarios", false, "");
    let _ = fs.bool("tour", false, "");
    let _ = fs.bool("campaign", false, "");
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let _ = fs.parse(&args, &mut std::io::sink());
    let audio_mode = flag_audio_mode.borrow().clone();
    AudioFlags {
        volume: flag_volume.get(),
        mute: flag_mute.get(),
        audio_mode,
    }
}

#[test]
fn test_cli_audio_flags() {
    let f = parse_flags(&["--volume=50", "--mute", "--audio-mode=bell"]);
    assert_eq!(f.volume, 50);
    assert!(f.mute);
    assert_eq!(f.audio_mode, "bell");

    let d = parse_flags(&[]);
    assert_eq!(d.volume, -1);
    assert!(!d.mute);
    assert_eq!(d.audio_mode, "");

    let c = parse_flags(&["-volume", "0", "-mute=false", "--audio-mode=native"]);
    assert_eq!(c.volume, 0);
    assert!(!c.mute);
    assert_eq!(c.audio_mode, "native");
}

#[test]
fn test_run_audio_flags() {
    let mut env = Env::new();
    let c = capture(Ok(()));
    let tests: &[(&str, &[&str], i32, bool, bool)] = &[
        (
            "volume override to 45",
            &["--volume=45", "-seed", "42"],
            45,
            false,
            true,
        ),
        (
            "volume override to 0",
            &["--volume=0", "-seed", "42"],
            0,
            false,
            true,
        ),
        (
            "negative volume clamped to 0",
            &["--volume=-5", "-seed", "42"],
            0,
            false,
            true,
        ),
        (
            "mute flag sets mute true and sound disabled",
            &["--mute", "-seed", "42"],
            80,
            true,
            false,
        ),
        (
            "volume and mute combined",
            &["--volume=60", "--mute", "-seed", "42"],
            60,
            true,
            false,
        ),
        (
            "audio-mode off mutes audio",
            &["--audio-mode=off", "-seed", "42"],
            80,
            true,
            false,
        ),
        (
            "audio-mode bell keeps audio enabled",
            &["--audio-mode=bell", "-seed", "42"],
            80,
            false,
            true,
        ),
    ];
    for (name, args, vol, muted, enabled) in tests {
        env.fresh_home();
        let (code, _, err) = run(args);
        assert_eq!(code, 0, "{name}: {err}");
        let m = c.borrow().model.clone().unwrap();
        let p = m
            .audio_player
            .as_ref()
            .expect("expected non-nil AudioPlayer");
        assert_eq!(p.volume(), *vol, "{name}: volume");
        assert_eq!(p.is_muted(), *muted, "{name}: muted");
        assert_eq!(m.sound_enabled(), *enabled, "{name}: enabled");
    }
}

#[test]
fn test_run_config_audio_overrides() {
    let _env = Env::new();
    let c = capture(Ok(()));
    let mut cfg = sst_tui::config::default_config();
    cfg.volume = 35;
    cfg.muted = true;
    cfg.audio_mode = sst_tui::config::AudioMode::native();
    cfg.theme = "modern".into();
    cfg.anim_speed = "normal".into();
    sst_tui::config::save_config(cfg).expect("failed to save test config");

    let (code, _, err) = run(&["-seed", "42"]);
    assert_eq!(code, 0, "{err}");
    let m = c.borrow().model.clone().unwrap();
    let p = m.audio_player.clone().unwrap();
    assert_eq!(p.volume(), 35, "expected volume 35 from config");
    assert!(p.is_muted(), "expected muted true from config");

    let (code, _, err) = run(&["--volume=90", "-seed", "42"]);
    assert_eq!(code, 0, "{err}");
    let m = c.borrow().model.clone().unwrap();
    let p = m.audio_player.clone().unwrap();
    assert_eq!(p.volume(), 90, "expected volume 90 from CLI override");
    assert!(p.is_muted(), "expected muted true to remain from config");

    let (code, _, err) = run(&["--sound", "-seed", "42"]);
    assert_eq!(code, 0, "{err}");
    let m = c.borrow().model.clone().unwrap();
    assert!(
        !m.audio_player.clone().unwrap().is_muted(),
        "expected muted false from --sound override"
    );
}

#[test]
fn test_main_help_contains_audio_flags() {
    let _env = Env::new();
    let (code, out, _) = run(&["--help"]);
    assert_eq!(code, 0);
    for f in ["-volume", "-mute", "-audio-mode"] {
        assert!(out.contains(f), "expected {f} in help output:\n{out}");
    }
}
