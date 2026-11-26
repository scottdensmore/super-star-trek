//! Port of `pkg/classic/runner_test.go`, `pkg/classic/subscriber_test.go`
//! and `tests/golden_test.go`.

use sst_engine::{
    ActionShields, EntityType, Event, EventShieldTransfer, EventTorpedoFired, EventTorpedoHit,
    GameLength, GameState, SkillLevel,
};
use super_star_trek::classic::{run_classic_cli, TeletypeSubscriber};

fn classic(args: &[&str]) -> (i32, String) {
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let mut input: &[u8] = b"";
    let mut out = Vec::new();
    let code = run_classic_cli(&mut input, &mut out, &args);
    (code, String::from_utf8(out).unwrap())
}

fn handle(e: Event) -> String {
    let mut buf = Vec::new();
    TeletypeSubscriber::new(&mut buf).handle_event(&e);
    String::from_utf8(buf).unwrap()
}

#[test]
fn test_run_classic_cli_classic_mode() {
    let (code, out) = classic(&["-classic", "-seed", "12345"]);
    assert_eq!(code, 0);
    assert!(out.contains("Super Star Trek (Go Edition)"), "got {out:?}");
    assert!(
        out.contains("Energy:") && out.contains("Shields:"),
        "got {out:?}"
    );
}

#[test]
fn test_run_classic_cli_default_placeholder() {
    let (code, out) = classic(&[]);
    assert_eq!(code, 0);
    assert_eq!(
        out,
        "Charmbracelet TUI placeholder - use --classic for teletype mode.\n"
    );
}

#[test]
fn test_run_classic_cli_invalid_flag() {
    let (code, _) = classic(&["-nonexistent-flag"]);
    assert_ne!(code, 0);
}

#[test]
fn test_teletype_event_formatting() {
    let out = handle(Event::ShieldTransfer(EventShieldTransfer {
        new_shields: 1500.0,
        new_energy: 3500.0,
    }));
    assert!(
        out.contains("Shields: 1500") && out.contains("Energy: 3500"),
        "unexpected subscriber format: {out:?}"
    );
}

#[test]
fn test_teletype_torpedo_events() {
    let out = handle(Event::TorpedoFired(EventTorpedoFired {
        origin: [4, 1],
        angle: 1.57,
    }));
    assert_eq!(out, "Track: course 1.57\n");

    let out = handle(Event::TorpedoHit(EventTorpedoHit {
        target: [4, 7],
        entity: EntityType::Klingon,
        damage: 500.0,
        destroyed: true,
    }));
    assert_eq!(out, "*** Klingon destroyed ***\n");

    let out = handle(Event::TorpedoHit(EventTorpedoHit {
        target: [4, 7],
        entity: EntityType::Klingon,
        damage: 250.0,
        destroyed: false,
    }));
    assert_eq!(out, "Hit: 250 units\n");
}

/// `tests/golden_test.go` `TestGoldenParityBasics`.
#[test]
fn test_golden_parity_basics() {
    let mut game = GameState::new_game(12345, SkillLevel::Good, GameLength::Medium);
    let events = game
        .dispatch(&ActionShields { amount: 500.0 }.into())
        .expect("action failed");
    let mut out = Vec::new();
    let mut sub = TeletypeSubscriber::new(&mut out);
    for e in &events {
        sub.handle_event(e);
    }
    assert_eq!(
        String::from_utf8(out).unwrap(),
        "Energy: 4500  Shields: 500\n"
    );
}
