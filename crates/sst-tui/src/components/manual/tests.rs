//! Ports of `manual_test.go`.

use sst_tuikit::ansi;
use sst_tuikit::tea::{self, KeyMsg};

use super::*;
use crate::theme::{detect, LcarsTheme};

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

fn assert_box(view: &str) {
    let lines: Vec<&str> = view.split('\n').collect();
    assert_eq!(lines.len(), 18);
    for (i, l) in lines.iter().enumerate() {
        assert_eq!(ansi::string_width(l), 66, "line {i}: {l:?}");
    }
    assert!(lines[0].starts_with('┌') && lines[0].ends_with('┐'));
    assert!(lines[17].starts_with('└') && lines[17].ends_with('┘'));
    for l in &lines[1..=16] {
        assert!(l.starts_with('│') && l.ends_with('│'), "{l:?}");
    }
}

#[test]
fn dimensions_and_layout_budget() {
    let mut m = new(setup(), 66, 18);
    for i in 0..m.chapters.len() {
        m.selected_idx = i as i32;
        let view = m.view();
        let lines: Vec<&str> = view.split('\n').collect();
        assert_eq!(lines.len(), 18);
        for l in lines {
            assert_eq!(ansi::string_width(l), 66);
        }
    }
}

#[test]
fn border_integrity() {
    assert_box(&new(setup(), 66, 18).view());
}

#[test]
fn curriculum_chapters() {
    let m = new(setup(), 66, 18);
    assert_eq!(m.chapters.len(), 8);
    let ids = [
        "systems",
        "nav",
        "combat",
        "shields",
        "starbases",
        "tactics",
        "scoring",
        "commands",
    ];
    for (i, id) in ids.iter().enumerate() {
        assert_eq!(m.chapters[i].id, *id);
        assert!(!m.chapters[i].lines.is_empty());
        for l in &m.chapters[i].lines {
            assert!(ansi::string_width(l) <= 43, "{id}: {l:?}");
        }
    }
}

#[test]
fn select_chapter() {
    let mut m = new(setup(), 66, 18);
    m.select_chapter("combat");
    assert_eq!(m.selected_idx, 2);
    m.select_chapter("7");
    assert_eq!(m.selected_idx, 6);
    m.select_chapter("unknown");
    assert_eq!(m.selected_idx, 6);
}

#[test]
fn set_theme() {
    let mut m = new(setup(), 66, 18);
    m.set_theme(Theme::from(LcarsTheme::default()));
    assert_eq!(m.theme.name(), "lcars");
}

#[test]
fn non_standard_dimensions_constraint() {
    assert_box(&new(setup(), 80, 24).view());
}

#[test]
fn dual_pane_focus() {
    let mut m = new(setup(), 66, 18);
    assert_eq!(m.focus, FocusPane::FocusChapters);
    let steps: &[(Msg, FocusPane)] = &[
        (key(tea::KEY_TAB), FocusPane::FocusContent),
        (key(tea::KEY_TAB), FocusPane::FocusChapters),
        (key(tea::KEY_SHIFT_TAB), FocusPane::FocusContent),
        (key(tea::KEY_SHIFT_TAB), FocusPane::FocusChapters),
        (key(tea::KEY_RIGHT), FocusPane::FocusContent),
        (key(tea::KEY_LEFT), FocusPane::FocusChapters),
        (key(tea::KEY_ENTER), FocusPane::FocusContent),
    ];
    for (msg, want) in steps {
        let _ = m.update(msg);
        assert_eq!(m.focus, *want);
    }
    // Esc in FocusContent returns to the chapter list without closing.
    assert!(m.update(&key(tea::KEY_ESC)).is_none());
    assert_eq!(m.focus, FocusPane::FocusChapters);
    let _ = m.update(&runes("l"));
    assert_eq!(m.focus, FocusPane::FocusContent);
    let _ = m.update(&runes("h"));
    assert_eq!(m.focus, FocusPane::FocusChapters);
}

#[test]
fn keyboard_navigation_and_jumps() {
    let mut m = new(setup(), 66, 18);
    let n = m.chapters.len() as i32;
    let steps: Vec<(Msg, i32)> = vec![
        (key(tea::KEY_DOWN), 1),
        (runes("j"), 2),
        (runes("k"), 1),
        (key(tea::KEY_UP), 0),
        (key(tea::KEY_UP), n - 1),
        (key(tea::KEY_DOWN), 0),
        (runes("5"), 4),
        (runes("1"), 0),
        (runes("8"), 7),
    ];
    for (msg, want) in steps {
        let _ = m.update(&msg);
        assert_eq!(m.selected_idx, want);
    }
}

#[test]
fn scroll_bounds_clamping() {
    let mut m = new(setup(), 66, 18);
    m.focus = FocusPane::FocusContent;
    let sel = m.selected_idx as usize;
    let off = |m: &Model| m.scroll_offsets[sel];
    let _ = m.update(&key(tea::KEY_UP));
    assert_eq!(off(&m), 0);
    let _ = m.update(&runes("k"));
    assert_eq!(off(&m), 0);
    let _ = m.update(&key(tea::KEY_DOWN));
    assert_eq!(off(&m), 1);
    let _ = m.update(&runes("j"));
    assert_eq!(off(&m), 2);
    let _ = m.update(&key(tea::KEY_END));
    let max_offset = (m.chapters[sel].lines.len() as i32 - 14).max(0);
    assert_eq!(off(&m), max_offset);
    let _ = m.update(&key(tea::KEY_DOWN));
    assert_eq!(off(&m), max_offset);
    let _ = m.update(&key(tea::KEY_HOME));
    assert_eq!(off(&m), 0);
    m.scroll_offsets[sel] = 5;
    let _ = m.update(&runes("g"));
    assert_eq!(off(&m), 0);
    let _ = m.update(&runes("G"));
    assert_eq!(off(&m), max_offset);
    let _ = m.update(&key(tea::KEY_PG_UP));
    assert_eq!(off(&m), (max_offset - 10).max(0));
    m.scroll_offsets[sel] = 12;
    let _ = m.update(&runes("b"));
    assert_eq!(off(&m), 2);
    let _ = m.update(&key(tea::KEY_CTRL_U));
    assert_eq!(off(&m), 0);
    let pg_down = 10.min(max_offset);
    let _ = m.update(&key(tea::KEY_PG_DOWN));
    assert_eq!(off(&m), pg_down);
    m.scroll_offsets[sel] = 0;
    let _ = m.update(&key(tea::KEY_SPACE));
    assert_eq!(off(&m), pg_down);
    m.scroll_offsets[sel] = 0;
    let _ = m.update(&key(tea::KEY_CTRL_D));
    assert_eq!(off(&m), pg_down);
}

fn closes(m: &mut Model, msg: &Msg) -> bool {
    m.update(msg)
        .and_then(|c| c.call())
        .is_some_and(|r| r.downcast_ref::<CloseModalMsg>().is_some())
}

#[test]
fn dismissal_keys() {
    let mut m = new(setup(), 66, 18);
    for msg in [
        key(tea::KEY_ESC),
        runes("q"),
        runes("Q"),
        key(tea::KEY_F1),
        runes("?"),
    ] {
        assert!(closes(&mut m, &msg));
    }
    m.focus = FocusPane::FocusContent;
    for msg in [runes("q"), runes("Q"), key(tea::KEY_F1), runes("?")] {
        assert!(closes(&mut m, &msg));
    }
}
