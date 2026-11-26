//! Port of `pkg/tui/components/commandbar/bar_test.go`.

use sst_tuikit::lipgloss;
use sst_tuikit::tea::{msg, KEY_RUNES};
use sst_tuikit::termenv::Profile;

use super::*;
use crate::theme::detect;

fn setup() {
    detect::set_fixed_detection(true);
    lipgloss::set_color_profile(Profile::Ascii);
}

fn dt() -> Option<Theme> {
    Some(theme::default_theme())
}

fn key(t: sst_tuikit::tea::KeyType) -> Msg {
    msg(KeyMsg::new(t))
}

fn runes(s: &str) -> Msg {
    msg(KeyMsg::runes(s))
}

#[test]
fn new_and_alias() {
    setup();
    let m1 = new(dt());
    let m2 = new_command_bar(dt());
    assert_eq!(m1.prompt, "COMMAND> ");
    assert_eq!(m2.prompt, "COMMAND> ");
    assert!(m1.messages().is_empty());
    assert_eq!(m1.theme().name(), "modern");
    assert_eq!(new(None).theme().name(), "modern");
}

#[test]
fn add_message_and_scrolling_buffer() {
    setup();
    let mut m = new(dt());
    for i in 1..=4 {
        m.add_message(&format!("msg {i}"));
    }
    let msgs = m.messages();
    assert_eq!(msgs.len(), 4);
    assert_eq!((msgs[0].as_str(), msgs[3].as_str()), ("msg 1", "msg 4"));
    m.add_message("msg 5");
    let msgs = m.messages();
    assert_eq!(msgs.len(), 4);
    assert_eq!((msgs[0].as_str(), msgs[3].as_str()), ("msg 2", "msg 5"));
    m.add_message("msg 6\nmsg 7");
    let msgs = m.messages();
    assert_eq!(msgs.len(), 4);
    assert_eq!((msgs[2].as_str(), msgs[3].as_str()), ("msg 6", "msg 7"));
    m.clear_messages();
    assert!(m.messages().is_empty());
}

#[test]
fn set_theme() {
    setup();
    let mut m = new(dt());
    m.set_theme(Some(theme::get_theme("lcars")));
    assert_eq!(m.theme().name(), "lcars");
    m.set_theme(None);
    assert_eq!(m.theme().name(), "modern");
}

#[test]
fn focus_and_blur() {
    setup();
    let mut m = new(dt());
    let _ = m.focus();
    assert!(m.focused());
    m.blur();
    assert!(!m.focused());
}

#[test]
fn update_typing() {
    setup();
    let mut m = new(dt());
    let _ = m.focus();
    for c in ["n", "a", "v"] {
        let _ = m.update(&runes(c));
    }
    assert_eq!(m.value(), "nav");
}

#[test]
fn update_enter_with_text() {
    setup();
    let mut m = new(dt());
    m.set_value("nav 1.5 2");
    let cmd = m.update(&key(KEY_ENTER)).expect("cmd on Enter with text");
    assert_eq!(m.value(), "");
    let r = cmd.call().expect("msg");
    let sub = r
        .downcast_ref::<CommandSubmittedMsg>()
        .expect("CommandSubmittedMsg");
    assert_eq!(sub.text, "nav 1.5 2");
}

#[test]
fn update_enter_empty() {
    setup();
    let mut m = new(dt());
    m.set_value("");
    assert!(m.update(&key(KEY_ENTER)).is_none());
    m.set_value("   ");
    assert!(m.update(&key(KEY_ENTER)).is_none());
}

#[test]
fn view() {
    setup();
    let mut m = new(dt());
    m.add_message("Enterprise systems online.");
    m.add_message("Sensors detect 3 Klingons.");
    let v = m.view();
    assert!(v.contains("COMMAND> "));
    assert!(v.contains("Enterprise systems online."));
    assert!(v.contains("Sensors detect 3 Klingons."));
    m.set_width(80);
    assert!(m.view().contains("COMMAND> "));
}

#[test]
fn add_message_trailing_newlines() {
    setup();
    let mut m = new(dt());
    m.add_message("Single line with newline\n");
    m.add_message("Line with CRLF\r\n");
    m.add_message("First of multi\nSecond of multi\n");
    let msgs = m.messages();
    assert_eq!(
        msgs,
        [
            "Single line with newline",
            "Line with CRLF",
            "First of multi",
            "Second of multi"
        ]
    );
    m.add_message("\n\n\r\n");
    assert_eq!(m.messages().len(), 4);
}

#[test]
fn command_bar_history() {
    setup();
    let mut cb = new(dt());
    cb.set_value("nav 1 2");
    let _ = cb.update(&key(KEY_ENTER));
    cb.set_value("tor 3 4");
    let _ = cb.update(&key(KEY_ENTER));
    cb.set_value("ph");
    for (k, want) in [
        (KEY_UP, "tor 3 4"),
        (KEY_UP, "nav 1 2"),
        (KEY_UP, "nav 1 2"),
        (KEY_DOWN, "tor 3 4"),
        (KEY_DOWN, "ph"),
        (KEY_DOWN, "ph"),
    ] {
        let _ = cb.update(&key(k));
        assert_eq!(cb.value(), want);
    }
}

#[test]
fn command_bar_history_deduplication() {
    setup();
    let mut cb = new(dt());
    for v in ["status", "status", "nav 1 2"] {
        cb.set_value(v);
        let _ = cb.update(&key(KEY_ENTER));
    }
    for want in ["nav 1 2", "status", "status"] {
        let _ = cb.update(&key(KEY_UP));
        assert_eq!(cb.value(), want);
    }
}

#[test]
fn command_bar_history_empty() {
    setup();
    let mut cb = new(dt());
    cb.set_value("draft");
    let _ = cb.update(&key(KEY_UP));
    assert_eq!(cb.value(), "draft");
    let _ = cb.update(&key(KEY_DOWN));
    assert_eq!(cb.value(), "draft");
}

#[test]
fn command_bar_tab_completion() {
    setup();
    let mut cb = new(dt());
    cb.set_value("ph");
    let _ = cb.update(&key(KEY_TAB));
    assert_eq!(cb.value(), "pha ");
    cb.set_value("s");
    for want in ["saves", "she ", "srscan", "status", "saves"] {
        let _ = cb.update(&key(KEY_TAB));
        assert_eq!(cb.value(), want);
    }
    let _ = cb.update(&msg(KeyMsg {
        typ: KEY_RUNES,
        runes: vec!['x'],
        ..Default::default()
    }));
    cb.set_value("doc");
    let _ = cb.update(&key(KEY_TAB));
    assert_eq!(cb.value(), "doc");
    cb.set_value("xyz");
    let _ = cb.update(&key(KEY_TAB));
    assert_eq!(cb.value(), "xyz");
    cb.set_value("");
    let _ = cb.update(&key(KEY_TAB));
    assert_eq!(cb.value(), "");
}

#[test]
fn command_bar_visual_bell() {
    setup();
    lipgloss::set_color_profile(Profile::TrueColor);
    let mut cb = new(dt());
    cb.set_width(80);
    assert!(!cb.visual_bell());
    let normal = cb.view();
    cb.set_visual_bell(true);
    assert!(cb.visual_bell());
    assert_ne!(cb.view(), normal);
    cb.set_visual_bell(false);
    assert!(!cb.visual_bell());
    assert_eq!(cb.view(), normal);
    lipgloss::set_color_profile(Profile::Ascii);
}

/// Go's copy semantics: the root model's `NewModel` hands the audio bell
/// callback a *copy* of the bar, so setting the bell on that copy never
/// reaches the rendered bar. Clones reproduce that.
#[test]
fn copies_do_not_share_visual_bell() {
    setup();
    let mut cb = new(dt());
    let stored = cb.clone();
    cb.set_visual_bell(true);
    assert!(!stored.visual_bell());
}
