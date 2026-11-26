//! Port of `pkg/tui/parser_test.go` (tests the `tui.ParseCommand` alias).
#![allow(clippy::approx_constant)]

use sst_engine::{Action, ActionFirePhasers, ActionFireTorpedo, ActionMove, ActionShields};
use sst_tui::parse_command;

fn mv(input: &str) -> ActionMove {
    let r = parse_command(input);
    assert!(
        r.error.is_none(),
        "ParseCommand({input:?}) unexpected error: {:?}",
        r.error
    );
    match r.action {
        Some(Action::Move(m)) => m,
        other => panic!("ParseCommand({input:?}) action is {other:?}, want ActionMove"),
    }
}

fn tor(input: &str) -> ActionFireTorpedo {
    let r = parse_command(input);
    assert!(
        r.error.is_none(),
        "ParseCommand({input:?}) unexpected error: {:?}",
        r.error
    );
    match r.action {
        Some(Action::FireTorpedo(t)) => t,
        other => panic!("ParseCommand({input:?}) action is {other:?}, want ActionFireTorpedo"),
    }
}

fn pha(input: &str) -> ActionFirePhasers {
    let r = parse_command(input);
    assert!(
        r.error.is_none(),
        "ParseCommand({input:?}) unexpected error: {:?}",
        r.error
    );
    match r.action {
        Some(Action::FirePhasers(p)) => p,
        other => panic!("ParseCommand({input:?}) action is {other:?}, want ActionFirePhasers"),
    }
}

fn she(input: &str) -> ActionShields {
    let r = parse_command(input);
    assert!(
        r.error.is_none(),
        "ParseCommand({input:?}) unexpected error: {:?}",
        r.error
    );
    match r.action {
        Some(Action::Shields(s)) => s,
        other => panic!("ParseCommand({input:?}) action is {other:?}, want ActionShields"),
    }
}

fn assert_err(input: &str) {
    let r = parse_command(input);
    assert!(
        r.error.is_some(),
        "ParseCommand({input:?}) expected error, got nil (action: {:?})",
        r.action
    );
}

fn special(input: &str) -> String {
    let r = parse_command(input);
    assert!(
        r.error.is_none(),
        "ParseCommand({input:?}) unexpected error: {:?}",
        r.error
    );
    r.special
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-6
}

#[test]
fn parse_commands_brief() {
    let m = mv("nav 1.5 2");
    assert!(m.course == 1.5 && m.warp == 2.0);
    assert_eq!(pha("pha 500").energy, 500.0);
    assert_eq!(parse_command("theme lcars").special, "theme lcars");
    assert_eq!(parse_command("quit").special, "quit");
}

#[test]
fn parse_nav_course_warp() {
    for (input, c, w) in [
        ("nav 1.5 2", 1.5, 2.0),
        ("NAV 0.5 4.5", 0.5, 4.5),
        ("move 2.0 1.0", 2.0, 1.0),
        ("nav course 3.14 2", 3.14, 2.0),
        ("nav c 0.0 5", 0.0, 5.0),
        ("nav 1 10", 1.0, 10.0),
    ] {
        let m = mv(input);
        assert!(close(m.course, c), "{input} Course = {}", m.course);
        assert!(close(m.warp, w), "{input} Warp = {}", m.warp);
        assert_eq!(m.dest_sector, [0, 0], "{input}");
    }
}

#[test]
fn parse_nav_sector() {
    for (input, s) in [
        ("nav 4 5", [4, 5]),
        ("NAV 1 8", [1, 8]),
        ("move 8 1", [8, 1]),
        ("nav sector 2 3", [2, 3]),
        ("nav s 6 7", [6, 7]),
        ("nav 1 2", [1, 2]),
    ] {
        let m = mv(input);
        assert_eq!(m.dest_sector, s, "{input}");
        assert_eq!(m.warp, 1.0, "{input}");
    }
}

#[test]
fn parse_nav_errors() {
    for input in [
        "nav",
        "nav 1",
        "nav abc def",
        "nav 1.5 -2",
        "nav 1 2 3 4",
        "nav s 9 9",
        "nav s 0 1",
    ] {
        assert_err(input);
    }
}

#[test]
fn parse_torpedo() {
    for (input, a) in [
        ("tor 1.5", 1.5),
        ("tor 0", 0.0),
        ("torpedo 3.14159", 3.14159),
        ("TOR 4.2", 4.2),
    ] {
        let t = tor(input);
        assert!(close(t.angle, a), "{input} Angle = {}", t.angle);
        assert_eq!(t.target, [0, 0], "{input}");
    }
    for (input, tg) in [
        ("tor 4 7", [4, 7]),
        ("TOR 1 1", [1, 1]),
        ("torpedo 8 8", [8, 8]),
    ] {
        assert_eq!(tor(input).target, tg, "{input}");
    }
    for input in [
        "tor",
        "tor abc",
        "tor 1 2 3",
        "tor 9 4",
        "tor 4 0",
        "tor -1 5",
        "tor 4 9",
    ] {
        assert_err(input);
    }
}

#[test]
fn parse_phasers() {
    for (input, e) in [
        ("pha 500", 500.0),
        ("phaser 1000", 1000.0),
        ("PHASERS 250.5", 250.5),
    ] {
        assert!(close(pha(input).energy, e), "{input}");
    }
    for input in ["pha", "pha abc", "pha -50", "pha 0", "pha 100 200"] {
        assert_err(input);
    }
}

#[test]
fn parse_shields() {
    for (input, a) in [
        ("she 500", 500.0),
        ("shield -200", -200.0),
        ("shields 0", 0.0),
        ("SHE 1000", 1000.0),
    ] {
        assert!(close(she(input).amount, a), "{input}");
    }
    for input in ["she", "she abc", "she 100 200"] {
        assert_err(input);
    }
}

#[test]
fn parse_dock() {
    for input in ["doc", "dock", "DOC", "DOCK"] {
        let r = parse_command(input);
        assert!(r.error.is_none(), "{input}");
        assert!(
            matches!(r.action, Some(Action::Dock(_))),
            "{input}: {:?}",
            r.action
        );
    }
}

#[test]
fn parse_special_commands() {
    for (input, want) in [
        ("theme", "theme"),
        ("theme lcars", "theme lcars"),
        ("theme CRT", "theme crt"),
        ("THEME modern", "theme modern"),
        ("help", "help"),
        ("?", "help"),
        ("commands", "help"),
        ("quit", "quit"),
        ("exit", "quit"),
        ("q", "quit"),
    ] {
        assert_eq!(special(input), want, "{input}");
    }
}

#[test]
fn parse_unknown_and_empty() {
    for input in ["", "   ", "\t\n", "xyz 123", "foo", "fire"] {
        assert_err(input);
    }
}

#[test]
fn parse_nav_quadrant() {
    for (input, q, w) in [
        ("nav q 3 5", [3, 5], 1.0),
        ("NAV QUAD 1 8", [1, 8], 1.0),
        ("move quadrant 7 2", [7, 2], 1.0),
        ("nav q 4 6 2.5", [4, 6], 2.5),
    ] {
        let m = mv(input);
        assert_eq!(m.dest_quad, q, "{input}");
        assert!(close(m.warp, w), "{input}");
    }
    for input in [
        "nav q 0 5",
        "nav q 9 1",
        "nav q 4 9",
        "nav q abc 2",
        "nav q 2 xyz",
        "nav q 2 3 -1",
    ] {
        assert_err(input);
    }
}

#[test]
fn parse_help_context() {
    for (input, want) in [
        ("help", "help"),
        ("?", "help"),
        ("commands", "help"),
        ("help nav", "help nav"),
        ("help move", "help nav"),
        ("help warp", "help nav"),
        ("help tor", "help tor"),
        ("help torpedo", "help tor"),
        ("help pha", "help pha"),
        ("help phasers", "help pha"),
        ("help she", "help she"),
        ("help shields", "help she"),
        ("help doc", "help doc"),
        ("help dock", "help doc"),
        ("help saves", "help saves"),
        ("help thaw", "help saves"),
    ] {
        assert_eq!(special(input), want, "{input}");
    }
}

#[test]
fn parse_options_command() {
    for input in ["opts", "options", "settings"] {
        assert_eq!(parse_command(input).special, "options", "{input}");
    }
}
