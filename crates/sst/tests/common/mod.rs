//! Shared helpers for the CLI tests: a process-wide lock (HOME and other
//! environment variables are global), temporary HOME directories, and a
//! capturing `run_program` hook (Go tests assign `runProgram`).
#![allow(dead_code)]

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Mutex, MutexGuard};

use sst_tuikit::tea::ProgramOption;

static LOCK: Mutex<()> = Mutex::new(());

/// Serializes tests touching the environment and isolates HOME.
pub struct Env {
    _guard: MutexGuard<'static, ()>,
    saved: Vec<(&'static str, Option<std::ffi::OsString>)>,
    dirs: Vec<PathBuf>,
}

impl Env {
    pub fn new() -> Env {
        let guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        sst_audio::set_test_mode(true);
        let keys = ["HOME", "XDG_CONFIG_HOME", "COLORFGBG", "TERM_PROGRAM", "TZ"];
        let saved = keys.iter().map(|k| (*k, std::env::var_os(k))).collect();
        let mut env = Env {
            _guard: guard,
            saved,
            dirs: Vec::new(),
        };
        env.fresh_home();
        std::env::set_var("COLORFGBG", "15;0");
        std::env::remove_var("TERM_PROGRAM");
        std::env::set_var("TZ", "UTC");
        sst_tui::theme::detect::set_fixed_detection(true);
        sst_tuikit::lipgloss::set_color_profile(sst_tuikit::termenv::Ascii);
        sst_tuikit::lipgloss::set_has_dark_background(true);
        env
    }

    /// Points HOME (and XDG_CONFIG_HOME) at a new empty directory.
    pub fn fresh_home(&mut self) -> PathBuf {
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("sst-cli-test-{}-{}", std::process::id(), n));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::env::set_var("HOME", &dir);
        std::env::set_var("XDG_CONFIG_HOME", dir.join(".config"));
        self.dirs.push(dir.clone());
        dir
    }
}

impl Drop for Env {
    fn drop(&mut self) {
        super_star_trek::set_run_program(None);
        for (k, v) in &self.saved {
            match v {
                Some(v) => std::env::set_var(k, v),
                None => std::env::remove_var(k),
            }
        }
        for d in &self.dirs {
            let _ = std::fs::remove_dir_all(d);
        }
    }
}

/// What the capturing runner saw.
#[derive(Default)]
pub struct Captured {
    pub calls: usize,
    pub model: Option<sst_tui::Model>,
    pub opts: Vec<ProgramOption>,
}

/// Installs a runner that records the model and options and returns `result`.
pub fn capture(result: Result<(), String>) -> Rc<RefCell<Captured>> {
    let c = Rc::new(RefCell::new(Captured::default()));
    let c2 = c.clone();
    super_star_trek::set_run_program(Some(Box::new(move |m, opts| {
        let mut cc = c2.borrow_mut();
        cc.calls += 1;
        cc.model = Some(m);
        cc.opts = opts.to_vec();
        result.clone()
    })));
    c
}

/// Go `run(args, strings.NewReader(""), &out, &errOut)`.
pub fn run(args: &[&str]) -> (i32, String, String) {
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let mut input: &[u8] = b"";
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = super_star_trek::run(&args, &mut input, &mut out, &mut err);
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}
