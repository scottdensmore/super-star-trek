//! Port of pkg/tui/parser/parser_test.go.
#![allow(clippy::approx_constant)]

use sst_engine::parser::{parse_command, ParsedCommand};
use sst_engine::*;

fn mv(res: &ParsedCommand, input: &str) -> ActionMove {
    assert!(
        res.error.is_none(),
        "ParseCommand({input:?}) unexpected error: {:?}",
        res.error
    );
    match &res.action {
        Some(Action::Move(m)) => m.clone(),
        other => panic!("ParseCommand({input:?}) action is {other:?}, want ActionMove"),
    }
}

fn tor(res: &ParsedCommand, input: &str) -> ActionFireTorpedo {
    assert!(
        res.error.is_none(),
        "ParseCommand({input:?}) unexpected error: {:?}",
        res.error
    );
    match &res.action {
        Some(Action::FireTorpedo(t)) => t.clone(),
        other => panic!("ParseCommand({input:?}) action is {other:?}, want ActionFireTorpedo"),
    }
}

fn pha(res: &ParsedCommand, input: &str) -> ActionFirePhasers {
    assert!(
        res.error.is_none(),
        "ParseCommand({input:?}) unexpected error: {:?}",
        res.error
    );
    match &res.action {
        Some(Action::FirePhasers(p)) => p.clone(),
        other => panic!("ParseCommand({input:?}) action is {other:?}, want ActionFirePhasers"),
    }
}

fn assert_errors(inputs: &[&str]) {
    for input in inputs {
        let res = parse_command(input);
        assert!(
            res.error.is_some(),
            "ParseCommand({input:?}) expected error, got nil (action: {:?})",
            res.action
        );
    }
}

#[test]
fn parse_commands_brief() {
    let res = parse_command("nav 1.5 2");
    let m = mv(&res, "nav 1.5 2");
    assert!(
        m.course == 1.5 && m.warp == 2.0,
        "mismatch in parsed nav action: {m:?}"
    );
    let res = parse_command("pha 500");
    assert_eq!(pha(&res, "pha 500").energy, 500.0);
    assert_eq!(parse_command("theme lcars").special, "theme lcars");
    assert_eq!(parse_command("quit").special, "quit");
}

#[test]
fn parse_nav_course_warp() {
    let tests = [
        ("nav 1.5 2", 1.5, 2.0),
        ("NAV 0.5 4.5", 0.5, 4.5),
        ("move 2.0 1.0", 2.0, 1.0),
        ("nav course 3.14 2", 3.14, 2.0),
        ("nav c 0.0 5", 0.0, 5.0),
        ("nav 1 10", 1.0, 10.0),
    ];
    for (input, course, warp) in tests {
        let m = mv(&parse_command(input), input);
        assert!(
            (m.course - course).abs() <= 1e-6,
            "{input}: Course = {}",
            m.course
        );
        assert!((m.warp - warp).abs() <= 1e-6, "{input}: Warp = {}", m.warp);
        assert_eq!(m.dest_sector, [0, 0], "{input}: DestSector");
    }
}

#[test]
fn parse_nav_sector() {
    let tests = [
        ("nav 4 5", [4, 5]),
        ("NAV 1 8", [1, 8]),
        ("move 8 1", [8, 1]),
        ("nav sector 2 3", [2, 3]),
        ("nav s 6 7", [6, 7]),
        ("nav 1 2", [1, 2]),
    ];
    for (input, sector) in tests {
        let m = mv(&parse_command(input), input);
        assert_eq!(m.dest_sector, sector, "{input}");
        assert_eq!(m.warp, 1.0, "{input}");
    }
}

#[test]
fn parse_nav_errors() {
    assert_errors(&[
        "nav",
        "nav 1",
        "nav abc def",
        "nav 1.5 -2",
        "nav 1 2 3 4",
        "nav s 9 9",
        "nav s 0 1",
    ]);
}

#[test]
fn parse_torpedo() {
    // Course
    for (input, angle) in [
        ("tor 1.5", 1.5),
        ("tor 0", 0.0),
        ("torpedo 3.14159", 3.14159),
        ("TOR 4.2", 4.2),
    ] {
        let t = tor(&parse_command(input), input);
        assert!(
            (t.angle - angle).abs() <= 1e-6,
            "{input}: Angle = {}",
            t.angle
        );
        assert_eq!(t.target, [0, 0], "{input}: Target");
    }
    // SectorTarget
    for (input, target) in [
        ("tor 4 7", [4, 7]),
        ("TOR 1 1", [1, 1]),
        ("torpedo 8 8", [8, 8]),
    ] {
        assert_eq!(tor(&parse_command(input), input).target, target, "{input}");
    }
    // CourseDirection
    for (input, dir) in [
        ("tor c 5", 5.0),
        ("tor course 3", 3.0),
        ("TOR C 1", 1.0),
        ("torpedo course 7.5", 7.5),
    ] {
        let t = tor(&parse_command(input), input);
        assert!(
            (t.direction - dir).abs() <= 1e-6,
            "{input}: Direction = {}",
            t.direction
        );
    }
    // Errors
    assert_errors(&[
        "tor",
        "tor abc",
        "tor 1 2 3",
        "tor 9 4",
        "tor 4 0",
        "tor -1 5",
        "tor 4 9",
    ]);
}

#[test]
fn parse_phasers() {
    for (input, e) in [
        ("pha 500", 500.0),
        ("phaser 1000", 1000.0),
        ("PHASERS 250.5", 250.5),
    ] {
        let p = pha(&parse_command(input), input);
        assert!(
            (p.energy - e).abs() <= 1e-6,
            "{input}: Energy = {}",
            p.energy
        );
    }
    assert_errors(&["pha", "pha abc", "pha -50", "pha 0", "pha 100 200"]);
}

#[test]
fn parse_shields() {
    for (input, amount) in [
        ("she 500", 500.0),
        ("shield -200", -200.0),
        ("shields 0", 0.0),
        ("SHE 1000", 1000.0),
    ] {
        let res = parse_command(input);
        assert!(res.error.is_none(), "{input}: {:?}", res.error);
        match &res.action {
            Some(Action::Shields(s)) => assert!((s.amount - amount).abs() <= 1e-6, "{input}"),
            other => panic!("{input}: action is {other:?}, want ActionShields"),
        }
    }
    assert_errors(&["she", "she abc", "she 100 200"]);
}

#[test]
fn parse_dock() {
    for input in ["doc", "dock", "DOC", "DOCK"] {
        let res = parse_command(input);
        assert!(res.error.is_none(), "{input}");
        assert!(
            matches!(res.action, Some(Action::Dock(_))),
            "{input}: {:?}",
            res.action
        );
    }
}

#[test]
fn parse_special_commands() {
    let tests = [
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
    ];
    for (input, want) in tests {
        let res = parse_command(input);
        assert!(res.error.is_none(), "{input}");
        assert_eq!(res.special, want, "{input}");
    }
}

#[test]
fn parse_unknown_and_empty() {
    assert_errors(&["", "   ", "\t\n", "xyz 123", "foo", "fire"]);
}

#[test]
fn parse_nav_quadrant() {
    let ok = [
        ("nav q 3 5", [3, 5], 1.0),
        ("NAV QUAD 1 8", [1, 8], 1.0),
        ("move quadrant 7 2", [7, 2], 1.0),
        ("nav q 4 6 2.5", [4, 6], 2.5),
    ];
    for (input, quad, warp) in ok {
        let m = mv(&parse_command(input), input);
        assert_eq!(m.dest_quad, quad, "{input}");
        assert!((m.warp - warp).abs() <= 1e-6, "{input}");
    }
    assert_errors(&[
        "nav q 0 5",
        "nav q 9 1",
        "nav q 4 9",
        "nav q abc 2",
        "nav q 2 xyz",
        "nav q 2 3 -1",
    ]);
}

#[test]
fn parse_help_context() {
    let tests = [
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
    ];
    for (input, want) in tests {
        let res = parse_command(input);
        assert!(res.error.is_none(), "{input}");
        assert_eq!(res.special, want, "{input}");
    }
}

#[test]
fn parse_options_command() {
    for cmd in ["opts", "options", "settings"] {
        assert_eq!(parse_command(cmd).special, "options", "{cmd}");
    }
}

#[test]
fn parse_scenarios_command() {
    for cmd in ["scenarios", "scen", "SCENARIOS", "SCEN"] {
        assert_eq!(parse_command(cmd).special, "scenarios", "{cmd}");
    }
}
