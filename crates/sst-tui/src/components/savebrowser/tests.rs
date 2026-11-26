//! Ports of `browser_test.go` (real filesystem through [`OsFs`]).

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use sst_engine::state::GameState;
use sst_engine::types::GameLength;
use sst_tuikit::tea::{self, KeyMsg};

use super::*;
use crate::theme::detect;

/// Go `t.TempDir()`.
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> TempDir {
        static N: AtomicU32 = AtomicU32::new(0);
        let p = std::env::temp_dir().join(format!(
            "sst-savebrowser-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        TempDir(p)
    }
    fn path(&self) -> String {
        self.0.to_string_lossy().into_owned()
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn create_test_save(dir: &TempDir, filename: &str, stardate: f64, skill: SkillLevel) -> String {
    let mut g = GameState::new_game(42, skill, GameLength::Short);
    g.stardate = stardate;
    let p = gopath::join(&[&dir.path(), filename]);
    persist::save_state(&g, &p).unwrap();
    p
}

fn th() -> Theme {
    detect::set_fixed_detection(true);
    default_theme()
}

fn key(t: tea::KeyType) -> Msg {
    tea::msg(KeyMsg::new(t))
}

fn runes(s: &str) -> Msg {
    tea::msg(KeyMsg::runes(s))
}

#[test]
fn listing_and_navigation() {
    let d = TempDir::new();
    create_test_save(&d, "GAME1.TRK", 3100.0, SkillLevel::Novice);
    create_test_save(&d, "GAME2.TRK", 3200.0, SkillLevel::Good);
    let mut m = new(th(), &d.path());
    m.refresh().unwrap();
    assert_eq!(m.saves.len(), 2);
    assert_eq!(m.cursor, 0);
    for (msg, want) in [
        (key(tea::KEY_DOWN), 1),
        (key(tea::KEY_DOWN), 1),
        (key(tea::KEY_UP), 0),
    ] {
        let _ = m.update(&msg);
        assert_eq!(m.cursor, want);
    }
}

#[test]
fn load_game_msg() {
    let d = TempDir::new();
    let p = create_test_save(&d, "LOADME.TRK", 3300.0, SkillLevel::Expert);
    let mut m = new(th(), &d.path());
    let _ = m.refresh();
    let msg = m.update(&key(tea::KEY_ENTER)).expect("cmd").call().unwrap();
    let load = msg.downcast_ref::<LoadGameMsg>().expect("LoadGameMsg");
    assert_eq!(gopath::base(&load.path), gopath::base(&p));
}

#[test]
fn delete_flow() {
    let d = TempDir::new();
    let p = create_test_save(&d, "DELME.TRK", 3400.0, SkillLevel::Fair);
    let mut m = new(th(), &d.path());
    let _ = m.refresh();
    let _ = m.update(&runes("d"));
    assert!(m.deleting);
    assert!(m.view().contains("Delete 'DELME.TRK'?"));
    let _ = m.update(&key(tea::KEY_ESC));
    assert!(!m.deleting);
    let _ = m.update(&runes("d"));
    let _ = m.update(&key(tea::KEY_ENTER));
    assert!(!m.deleting);
    assert!(!std::path::Path::new(&p).exists());
    assert!(m.saves.is_empty());
}

#[test]
fn close_msg() {
    let d = TempDir::new();
    let mut m = new(th(), &d.path());
    let _ = m.refresh();
    let msg = m.update(&key(tea::KEY_ESC)).expect("cmd").call().unwrap();
    assert!(msg.downcast_ref::<CloseBrowserMsg>().is_some());
}

#[test]
fn jk_navigation() {
    let d = TempDir::new();
    create_test_save(&d, "S1.TRK", 3100.0, SkillLevel::Novice);
    create_test_save(&d, "S2.TRK", 3200.0, SkillLevel::Good);
    let mut m = new(th(), &d.path());
    let _ = m.refresh();
    for (k, want) in [("j", 1), ("j", 1), ("k", 0), ("k", 0)] {
        let _ = m.update(&runes(k));
        assert_eq!(m.cursor, want);
    }
}

#[test]
fn empty_directory() {
    th();
    let d = TempDir::new();
    let mut m = new(None, &d.path());
    assert!(m.refresh().is_ok());
    assert!(m.saves.is_empty());
    assert!(m.view().contains("No saved missions (*.TRK) found."));
    assert!(m.update(&key(tea::KEY_ENTER)).is_none());
    let _ = m.update(&key(tea::KEY_DELETE));
    assert!(!m.deleting);
}

#[test]
fn reset_and_configuration() {
    let d = TempDir::new();
    create_test_save(&d, "RESET.TRK", 3500.0, SkillLevel::Emeritus);
    let mut m = new(th(), &d.path());
    let _ = m.refresh();
    m.cursor = 1;
    m.deleting = true;
    m.error_message = "some error".into();
    m.reset();
    assert_eq!(
        (m.cursor, m.deleting, m.error_message.as_str()),
        (0, false, "")
    );
    m.set_theme(None);
    assert_eq!(m.theme, default_theme());
    m.set_size(80, 24);
    assert_eq!((m.width, m.height), (80, 24));
}

#[test]
fn invalid_directory() {
    let mut m = new(th(), "/path/that/does/not/exist/hopefully");
    assert!(m.refresh().is_err());
    assert!(!m.error_message.is_empty());
}

#[test]
fn narrow_width_no_panic() {
    let d = TempDir::new();
    let mut m = new(th(), &d.path());
    m.set_size(1, 1);
    let _ = m.view();
}

#[test]
fn delete_failure_preserves_error() {
    let d = TempDir::new();
    let p = create_test_save(&d, "NODELETE.TRK", 3400.0, SkillLevel::Fair);
    let mut m = new(th(), &d.path());
    let _ = m.refresh();
    fs::remove_file(&p).unwrap();
    let _ = m.update(&runes("d"));
    assert!(m.deleting);
    let _ = m.update(&key(tea::KEY_ENTER));
    assert!(
        m.error_message.contains("Delete failed"),
        "{}",
        m.error_message
    );
    assert!(m.view().contains("Delete failed"));
}

#[test]
fn unicode_filename_truncation() {
    let d = TempDir::new();
    let name = "日本語ミッション保存ファイル.TRK";
    let p = create_test_save(&d, "VALID.TRK", 3141.5, SkillLevel::Good);
    fs::rename(&p, d.0.join(name)).unwrap();
    let mut m = new(th(), &d.path());
    m.refresh().unwrap();
    assert_eq!(m.saves.len(), 1);
    let view = m.view();
    let truncated: String = name.chars().take(10).collect();
    assert!(view.contains(&truncated), "{view}");
}

#[test]
fn key_delete_toggle() {
    let d = TempDir::new();
    create_test_save(&d, "DELTOGGLE.TRK", 3500.0, SkillLevel::Fair);
    let mut m = new(th(), &d.path());
    let _ = m.refresh();
    let _ = m.update(&key(tea::KEY_DELETE));
    assert!(m.deleting);
    let _ = m.update(&key(tea::KEY_DELETE));
    assert!(!m.deleting);
}
