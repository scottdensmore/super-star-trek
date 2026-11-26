//! Byte-for-byte parity with `testdata/parity/components/savebrowser.txt`
//! (recorded by `pkg/tui/components/savebrowser/zz_paritygen_test.go`).
//!
//! The Go generator works in a temp working directory with a relative
//! `saves` directory. Here [`FixtureFs`] maps the same relative names onto
//! a private temp root (tests run in parallel, so no `chdir`), writes real
//! save files with real modification times, and reports those times in UTC
//! like the Go run (`time.Local = time.UTC`).

#[path = "../modal_parity_util.rs"]
#[allow(clippy::duplicate_mod)]
mod util;

use std::cell::RefCell;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, UNIX_EPOCH};

use sst_engine::gofmt::quote;
use sst_engine::state::GameState;
use sst_engine::types::GameLength;
use util::*;

use super::*;

const DATA: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testdata/parity/components/savebrowser.txt"
));

struct FixtureFs {
    root: PathBuf,
}

impl FixtureFs {
    fn real(&self, p: &str) -> PathBuf {
        self.root.join(p)
    }
}

impl SaveFs for FixtureFs {
    fn read_dir(&self, dir: &str) -> Result<Vec<DirEntry>, String> {
        let real = self.real(dir);
        let rd = fs::read_dir(&real).map_err(|e| persist::path_error("open", dir, &e).0)?;
        let mut out: Vec<DirEntry> = rd
            .map(|e| {
                let e = e.unwrap();
                DirEntry {
                    name: e.file_name().to_string_lossy().into_owned(),
                    is_dir: e.file_type().unwrap().is_dir(),
                }
            })
            .collect();
        out.sort_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));
        Ok(out)
    }
    fn inspect_save(&self, path: &str) -> Result<SaveMetadata, EngineError> {
        let real = self.real(path);
        let mut meta = persist::inspect_save(&real.to_string_lossy())?;
        meta.path = path.to_string();
        meta.filename = gopath::base(path);
        meta.mod_time.offset = 0;
        Ok(meta)
    }
    fn remove(&self, path: &str) -> Result<(), String> {
        fs::remove_file(self.real(path)).map_err(|e| persist::path_error("remove", path, &e).0)
    }
}

struct Save {
    name: &'static str,
    kind: &'static str,
    skill: i32,
    cond: i32,
    stardate: f64,
    klingons: i32,
    mod_time: (u32, u32, u32, u32),
}

const fn s(
    name: &'static str,
    kind: &'static str,
    skill: i32,
    cond: i32,
    stardate: f64,
    klingons: i32,
    mod_time: (u32, u32, u32, u32),
) -> Save {
    Save {
        name,
        kind,
        skill,
        cond,
        stardate,
        klingons,
        mod_time,
    }
}

const FIXTURE: &[Save] = &[
    s("ALPHA.TRK", "env", 1, 0, 2100.0, 15, (3, 1, 10, 0)),
    s("bravo.trk", "env", 2, 1, 2200.55, 9, (3, 2, 11, 30)),
    s("CHARLIE.TRK", "legacy", 3, 2, 2300.25, 3, (1, 15, 8, 5)),
    s(
        "ENTERPRISE_LONG_NAME.TRK",
        "env",
        4,
        3,
        3456.78,
        0,
        (12, 31, 23, 59),
    ),
    s("DELTA.TRK", "env", 5, 0, 1999.94, 120, (3, 2, 11, 30)),
    s("ECHO.TRK", "env", 0, 9, -4.5, -1, (7, 4, 0, 1)),
    s("FOXTROT.TRK", "env", 3, 1, 2001.0, 7, (2, 28, 13, 13)),
    s("GOLF.TRK", "legacy", 1, 3, 2002.0, 8, (6, 6, 6, 6)),
    s("ÉCLAT★LONGNAME.TRK", "env", 2, 2, 2003.0, 1, (9, 9, 9, 9)),
    s("HOTEL.TRK", "env", 4, 0, 2004.5, 2, (10, 10, 10, 10)),
    s("corrupt.TRK", "corrupt", 0, 0, 0.0, 0, (11, 11, 11, 11)),
    s("notes.txt", "other", 0, 0, 0.0, 0, (11, 12, 11, 11)),
    s("subdir.TRK", "dir", 0, 0, 0.0, 0, (11, 13, 11, 11)),
];

fn build(root: &std::path::Path, dir: &str, saves: &[Save]) {
    let d = root.join(dir);
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    for sv in saves {
        let p = d.join(sv.name);
        let ps = p.to_string_lossy().into_owned();
        match sv.kind {
            "env" | "legacy" => {
                let mut g = GameState::new_game(42, SkillLevel::Good, GameLength::Short);
                g.skill = SkillLevel(sv.skill);
                g.enterprise.condition = ConditionType(sv.cond);
                g.stardate = sv.stardate;
                g.remaining_klingons = sv.klingons;
                if sv.kind == "env" {
                    persist::save_game(&Rc::new(RefCell::new(g)), &ps).unwrap();
                } else {
                    persist::save_state(&g, &ps).unwrap();
                }
            }
            "corrupt" => fs::write(&p, b"{this is not json").unwrap(),
            "other" => fs::write(&p, b"hello").unwrap(),
            _ => {
                fs::create_dir_all(&p).unwrap();
                continue;
            }
        }
        let (mo, dd, h, mi) = sv.mod_time;
        let t = GoTime::date(2026, mo, dd, h, mi, 0, 0, 0);
        let f = fs::File::options().write(true).open(&p).unwrap();
        f.set_modified(UNIX_EPOCH + Duration::from_secs(t.unix as u64))
            .unwrap();
    }
}

fn desc(m: &Msg) -> Option<String> {
    if let Some(v) = m.downcast_ref::<LoadGameMsg>() {
        return Some(format!("LoadGameMsg path={}", quote(&v.path)));
    }
    m.downcast_ref::<CloseBrowserMsg>()
        .map(|_| "CloseBrowserMsg".to_string())
}

fn state(m: &Model) -> String {
    let names: Vec<&str> = m.saves.iter().map(|s| s.filename.as_str()).collect();
    format!(
        "cursor={} deleting={} err={} saves={}",
        m.cursor,
        m.deleting,
        quote(&m.error_message),
        names.join("|")
    )
}

fn err_str(r: Result<(), String>) -> String {
    match r {
        Ok(()) => "<nil>".into(),
        Err(e) => e,
    }
}

const SEQUENCES: &[&[&str]] = &[
    &[
        "down", "down", "down", "down", "down", "down", "down", "down", "down", "down", "up", "up",
        "up", "k", "K", "j", "J", "enter", "esc",
    ],
    &[
        "d", "down", "up", "j", "esc", "D", "enter", "delete", "delete", "down", "d", "enter",
        "end", "x", "enter",
    ],
    &[
        "j",
        "j",
        "d",
        "rm:CHARLIE.TRK",
        "enter",
        "down",
        "enter",
        "d",
        "esc",
        "esc",
    ],
    &[
        "end", "down", "down", "down", "down", "down", "down", "down", "down", "down", "d",
        "enter", "d", "enter", "d", "enter", "up", "enter",
    ],
];

fn generate(root: &std::path::Path) -> Vec<String> {
    let fsv: Rc<dyn SaveFs> = Rc::new(FixtureFs {
        root: root.to_path_buf(),
    });
    let mk = |th: Option<Theme>, dir: &str| new_with_fs(th, dir, fsv.clone());
    let mut o = Out::default();
    for c in combos() {
        let (th, pre) = c.apply();
        let full = c.full();
        for (si, seq) in SEQUENCES.iter().enumerate() {
            if !full && si != 0 && si != 2 {
                continue;
            }
            build(root, "saves", FIXTURE);
            let mut m = mk(Some(th), "saves");
            let err = m.refresh();
            let p0 = format!("{pre}seq{si}/");
            o.w(&format!("{p0}refresh"), &err_str(err));
            o.w(&format!("{p0}init"), &m.view());
            for (ki, k) in seq.iter().enumerate() {
                let p = format!("{pre}seq{si}/{ki}/{k}/");
                if let Some(name) = k.strip_prefix("rm:") {
                    let _ = fs::remove_file(root.join("saves").join(name));
                    o.w(&format!("{p}state"), &state(&m));
                    continue;
                }
                let cmd = m.update(&key(k));
                o.w(&format!("{p}msg"), &describe_cmd(cmd, &desc));
                o.w(&format!("{p}state"), &state(&m));
                if full || ki == seq.len() - 1 {
                    o.w(&format!("{p}view"), &m.view());
                }
            }
        }
        if !full {
            continue;
        }
        build(root, "saves", FIXTURE);
        for wd in [62, 80, 30, 2, 1, 0, -5] {
            let mut m = mk(Some(th), "saves");
            let _ = m.refresh();
            m.set_size(wd, 10);
            o.w(&format!("{pre}size/{wd}"), &m.view());
        }
        build(root, "empty", &[]);
        let mut m = mk(Some(th), "empty");
        o.w(&format!("{pre}empty/refresh"), &err_str(m.refresh()));
        o.w(&format!("{pre}empty/view"), &m.view());
        for k in ["d", "delete", "enter", "down", "up"] {
            let cmd = m.update(&key(k));
            o.w(
                &format!("{pre}empty/{k}"),
                &format!("{} {}", describe_cmd(cmd, &desc), state(&m)),
            );
        }
        let mut m = mk(Some(th), "missing-dir");
        o.w(&format!("{pre}missing/refresh"), &err_str(m.refresh()));
        o.w(&format!("{pre}missing/view"), &m.view());
        o.w(&format!("{pre}missing/state"), &state(&m));
        let m = mk(None, "");
        o.w(&format!("{pre}dot/dir"), &m.directory);
        let mut m = mk(Some(th), "saves");
        let _ = m.refresh();
        m.cursor = 8;
        m.deleting = true;
        build(root, "saves", &FIXTURE[..3]);
        let _ = m.refresh();
        o.w(&format!("{pre}clamp"), &state(&m));
        m.reset();
        o.w(&format!("{pre}reset"), &state(&m));
        m.set_theme(None);
        o.w(
            &format!("{pre}theme"),
            &format!("{} {}", m.theme.name(), m.theme.color_mode()),
        );
    }
    o.0
}

#[test]
fn savebrowser_parity() {
    crate::theme::detect::set_fixed_detection(true);
    let root = std::env::temp_dir().join(format!("sst-savebrowser-parity-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let got = generate(&root);
    let _ = fs::remove_dir_all(&root);
    let (lines, bytes) = compare(&got, DATA);
    eprintln!("savebrowser parity: {lines} lines, {bytes} bytes");
}
