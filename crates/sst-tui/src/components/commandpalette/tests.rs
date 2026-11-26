//! Ports of `palette_test.go`.

use sst_tuikit::lipgloss;
use sst_tuikit::tea::{self, KeyMsg, Msg, WindowSizeMsg};

use super::*;
use crate::theme::{detect, CrtTheme, LcarsTheme};

fn setup() -> Theme {
    detect::set_fixed_detection(true);
    default_theme()
}

fn key(t: tea::KeyType) -> Msg {
    tea::msg(KeyMsg::new(t))
}

fn runes(s: &str) -> Msg {
    tea::msg(KeyMsg::runes(s))
}

fn lines(v: &str) -> Vec<String> {
    v.trim_end_matches('\n')
        .split('\n')
        .map(String::from)
        .collect()
}

#[test]
fn catalog() {
    let m = new(setup(), 56, 16);
    let items = m.items();
    assert_eq!(items.len(), 22);
    let expected: &[(&str, &str, bool)] = &[
        ("TOR", "tor ", true),
        ("PHA", "pha ", true),
        ("SHE", "she ", true),
        ("TARGET", "target", false),
        ("NAV", "nav ", true),
        ("DOC", "doc", false),
        ("SRSCAN", "srscan", false),
        ("LRSCAN", "lrscan", false),
        ("STATUS", "status", false),
        ("DAM", "dam", false),
        ("CHART", "chart", false),
        ("SAVES", "saves", false),
        ("SCENARIOS", "scenarios", false),
        ("THEME: Modern", "theme modern", false),
        ("THEME: LCARS", "theme lcars", false),
        ("THEME: CRT", "theme crt", false),
        ("THEME MODE AUTO", "theme mode auto", false),
        ("THEME MODE DARK", "theme mode dark", false),
        ("THEME MODE LIGHT", "theme mode light", false),
        ("HELP", "help", false),
        ("HELP NAV", "help nav", false),
        ("QUIT", "quit", false),
    ];
    for (title, prefix, param) in expected {
        let it = items
            .iter()
            .find(|i| i.title() == *title)
            .unwrap_or_else(|| panic!("missing {title}"));
        assert_eq!(it.prefix(), *prefix);
        assert_eq!(it.parameterized(), *param);
        assert!(!it.description().is_empty());
        assert!(!it.filter_value().is_empty());
    }
    let chart = items.iter().find(|i| i.title() == "CHART").unwrap();
    assert_eq!(
        chart.description(),
        "Interactive 8x8 galactic star chart and warp planner (Ctrl+G)"
    );
}

fn type_str(m: &mut Model, s: &str) {
    for c in s.chars() {
        if let Some(cmd) = m.update(&runes(&c.to_string())) {
            if let Some(msg) = cmd.call() {
                let _ = m.update(&msg);
            }
        }
    }
}

#[test]
fn fuzzy_filter() {
    let mut m = new(setup(), 56, 16);
    type_str(&mut m, "pha");
    assert_eq!(m.filter_value(), "pha");
    assert!(!m.visible_items().is_empty());
    assert_eq!(m.selected_item().unwrap().title(), "PHA");

    m.reset();
    assert_eq!(m.filter_value(), "");
    assert_eq!(m.visible_items().len(), 22);

    type_str(&mut m, "nav");
    assert!(!m.visible_items().is_empty());
    assert_eq!(m.selected_item().unwrap().title(), "NAV");

    let _ = m.update(&key(tea::KEY_BACKSPACE));
    assert_eq!(m.filter_value(), "na");
}

fn selected(cmd: Option<Cmd>) -> CommandSelectedMsg {
    let msg = cmd.expect("cmd").call().expect("msg");
    msg.downcast_ref::<CommandSelectedMsg>()
        .expect("CommandSelectedMsg")
        .clone()
}

#[test]
fn selection() {
    let th = setup();
    let mut m = new(th, 56, 16);
    for c in "pha".chars() {
        let _ = m.update(&runes(&c.to_string()));
    }
    let sel = selected(m.update(&key(tea::KEY_ENTER)));
    assert_eq!(sel.command_prefix, "pha ");
    assert!(sel.parameterized);

    m.reset();
    for c in "doc".chars() {
        let _ = m.update(&runes(&c.to_string()));
    }
    let sel = selected(m.update(&key(tea::KEY_ENTER)));
    assert_eq!(sel.command_prefix, "doc");
    assert!(!sel.parameterized);
}

#[test]
fn close() {
    let mut m = new(setup(), 56, 16);
    let msg = m.update(&key(tea::KEY_ESC)).unwrap().call().unwrap();
    assert!(msg.downcast_ref::<ClosePaletteMsg>().is_some());
}

#[test]
fn navigation() {
    let mut m = new(setup(), 56, 16);
    assert_eq!(m.selected_item().unwrap().title(), "TOR");
    let _ = m.update(&key(tea::KEY_DOWN));
    assert_eq!(m.selected_item().unwrap().title(), "PHA");
    let _ = m.update(&key(tea::KEY_UP));
    assert_eq!(m.selected_item().unwrap().title(), "TOR");
}

fn assert_dims(view: &str, w: i32, h: usize) {
    let ls = lines(view);
    assert_eq!(ls.len(), h, "view:\n{view}");
    for (i, l) in ls.iter().enumerate() {
        assert_eq!(lipgloss::width(l), w, "line {i}: {l:?}");
    }
}

#[test]
fn view_dimensions_and_layout() {
    let m = new(setup(), 56, 16);
    let view = m.view();
    assert!(view.contains("COMMAND PALETTE"));
    assert_dims(&view, 56, 16);
}

#[test]
fn theme_and_size() {
    setup();
    let mut m = new(None, 0, 0);
    assert!(m.selected_item().is_some());
    m.set_theme(Theme::from(LcarsTheme::default()));
    m.set_size(60, 20);
    assert_dims(&m.view(), 60, 20);
    m.set_theme(Theme::from(CrtTheme::default()));
    assert!(!m.view().is_empty());
}

#[test]
fn no_match() {
    let mut m = new(setup(), 56, 16);
    for c in "zzzznotfound".chars() {
        let _ = m.update(&runes(&c.to_string()));
    }
    assert!(m.visible_items().is_empty());
    assert!(m.selected_item().is_none());
    assert!(m.update(&key(tea::KEY_ENTER)).is_none());
}

#[test]
fn rune_safe_backspace_and_window_size() {
    let mut m = new(setup(), 56, 16);
    for r in ['🚀', '🎯', 'é', '漢', '字'] {
        let _ = m.update(&runes(&r.to_string()));
    }
    assert_eq!(m.filter_value(), "🚀🎯é漢字");
    for want in ["🚀🎯é漢", "🚀🎯é", "🚀🎯", "🚀", "", ""] {
        let _ = m.update(&key(tea::KEY_BACKSPACE));
        assert_eq!(m.filter_value(), want);
    }
    let _ = m.update(&tea::msg(WindowSizeMsg {
        width: 120,
        height: 40,
    }));
    assert_dims(&m.view(), 56, 16);
}
