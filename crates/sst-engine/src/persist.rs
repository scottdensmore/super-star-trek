//! Everything in `pkg/engine` that touches the filesystem, the environment
//! or the wall clock: save/load (`save.go`), leaderboard storage
//! (`leaderboard.go`) and the real-clock entry points (`NewTour(0)`,
//! `RecordTour`, `AddScenarioScore`). Error strings mimic Go's
//! (`open <path>: no such file or directory`, ...).

use std::fs;
use std::io::{self, Write};

use crate::actions::EngineError;
use crate::campaign::{TourRef, TourState};
use crate::gopath;
use crate::gotime::{Clock, GoTime, SystemClock};
use crate::json;
use crate::leaderboard::{
    add_scenario_score_to, default_scenario_leaderboard, normalize_scenario_id, Leaderboard,
    ScoreEntry, TourRecord,
};
use crate::save::{self, SaveMetadata};
use crate::scenarios::ScenarioID;
use crate::state::{GameRef, GameState};

/// Go's lowercase errno text (`syscall.Errno.Error()`) for common codes.
fn errno_text(e: &io::Error) -> String {
    #[cfg(unix)]
    if let Some(code) = e.raw_os_error() {
        let t = match code {
            libc::ENOENT => "no such file or directory",
            libc::EACCES => "permission denied",
            libc::EISDIR => "is a directory",
            libc::ENOTDIR => "not a directory",
            libc::EEXIST => "file exists",
            libc::ENOSPC => "no space left on device",
            libc::EROFS => "read-only file system",
            libc::EPERM => "operation not permitted",
            libc::ENOTEMPTY => "directory not empty",
            libc::EXDEV => "cross-device link",
            _ => "",
        };
        if !t.is_empty() {
            return t.to_string();
        }
    }
    let s = e.to_string();
    match s.find(" (os error") {
        Some(i) => s[..i].to_lowercase(),
        None => s.to_lowercase(),
    }
}

/// Formats an I/O error like Go's `*fs.PathError`: `<op> <path>: <err>`.
pub fn path_error(op: &str, path: &str, e: &io::Error) -> EngineError {
    EngineError(format!("{} {}: {}", op, path, errno_text(e)))
}

fn read_file(path: &str) -> Result<Vec<u8>, (io::Error, EngineError)> {
    match fs::metadata(path) {
        Ok(m) if m.is_dir() => {
            let e = io::Error::from_raw_os_error(21);
            let pe = EngineError(format!("read {}: is a directory", path));
            return Err((e, pe));
        }
        _ => {}
    }
    fs::read(path).map_err(|e| {
        let pe = path_error("open", path, &e);
        (e, pe)
    })
}

fn write_file(path: &str, data: &[u8], mode: u32) -> Result<(), EngineError> {
    let mut opts = fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(mode);
    }
    #[cfg(not(unix))]
    let _ = mode;
    let mut f = opts.open(path).map_err(|e| path_error("open", path, &e))?;
    f.write_all(data).map_err(|e| path_error("write", path, &e))
}

/// Go `GameState.Save(path)`: bare state, `.TRK` appended when missing.
pub fn save_state(g: &GameState, path: &str) -> Result<(), EngineError> {
    let path = save::resolve_save_path(path)?;
    let data = save::encode_legacy_save(g).map_err(|e| EngineError(e.to_string()))?;
    write_file(&path, &data, 0o644)
}

/// Go `SaveGame(g, path)` (envelope version 2).
pub fn save_game(g: &GameRef, path: &str) -> Result<(), EngineError> {
    let path = path.trim();
    if path.is_empty() {
        return Err(EngineError::new("file path cannot be empty"));
    }
    let data = save::encode_save_game(g)?;
    write_file(path, &data, 0o644)
}

/// Go `LoadGame(path)`.
pub fn load_game(path: &str) -> Result<GameRef, EngineError> {
    let data = read_file(path).map_err(|e| e.1)?;
    save::decode_load_game(&data).map_err(|e| EngineError(e.to_string()))
}

fn mod_time(m: &fs::Metadata) -> GoTime {
    let t = m
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok());
    match t {
        Some(d) => {
            let unix = d.as_secs() as i64;
            let local = SystemClock.now();
            GoTime::from_unix(unix, d.subsec_nanos(), local.offset)
        }
        None => GoTime::default(),
    }
}

/// Go `InspectSave(path)`.
pub fn inspect_save(path: &str) -> Result<SaveMetadata, EngineError> {
    let meta = fs::metadata(path).map_err(|e| path_error("stat", path, &e))?;
    let data = read_file(path).map_err(|e| e.1)?;
    save::decode_inspect_save(&data, path, mod_time(&meta))
}

/// Go `SaveTourGame(filename, g, t)` (mode 0600).
pub fn save_tour_game(
    filename: &str,
    g: Option<&GameRef>,
    t: Option<&TourRef>,
) -> Result<(), EngineError> {
    let data = save::encode_tour_save(g, t)?;
    write_file(filename, &data, 0o600)
}

/// Go `LoadTourGame(filename)`.
pub fn load_tour_game(filename: &str) -> Result<(Option<GameRef>, Option<TourRef>), EngineError> {
    let data = read_file(filename)
        .map_err(|e| EngineError(format!("failed to read save file: {}", e.1)))?;
    save::decode_tour_game(&data, filename)
}

/// Go `DefaultLeaderboardPath()`.
pub fn default_leaderboard_path() -> String {
    let get = |k: &str| std::env::var(k).unwrap_or_default();
    let mut config_dir = get("XDG_CONFIG_HOME");
    if config_dir.is_empty() {
        let home = get("HOME");
        if !home.is_empty() {
            config_dir = gopath::join(&[&home, ".config"]);
        } else if cfg!(windows) {
            config_dir = get("AppData");
        } else if cfg!(target_os = "macos") || cfg!(target_os = "ios") {
            // os.UserConfigDir needs $HOME here; it is empty.
        } else {
            // os.UserConfigDir: $XDG_CONFIG_HOME (empty) or $HOME/.config (empty) -> error
        }
    }
    if !config_dir.is_empty() {
        return gopath::join(&[&config_dir, "super-star-trek", "highscores.json"]);
    }
    "./.sst-scores.json".into()
}

fn recover_corrupt(path: &str, data: &[u8]) {
    let corrupt = format!("{}.corrupt", path);
    if fs::rename(path, &corrupt).is_err() {
        let _ = fs::write(&corrupt, data);
        let _ = fs::remove_file(path);
    }
}

/// Go `LoadLeaderboard(path...)`: `None`/empty uses the default path.
pub fn load_leaderboard(path: Option<&str>) -> Result<Leaderboard, EngineError> {
    let target = match path {
        Some(p) if !p.is_empty() => p.to_string(),
        _ => default_leaderboard_path(),
    };
    let data = match read_file(&target) {
        Ok(d) => d,
        Err((e, pe)) => {
            if e.kind() == io::ErrorKind::NotFound {
                return Ok(Leaderboard::default_board());
            }
            return Err(pe);
        }
    };
    let mut lb = Leaderboard::default();
    if json::unmarshal(&data, &mut lb).is_err() {
        recover_corrupt(&target, &data);
        return Ok(Leaderboard::default_board());
    }
    lb.normalize_loaded(true);
    Ok(lb)
}

static TMP_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Go `Leaderboard.Save(path)`: atomic write via a temp file and rename.
pub fn save_leaderboard(lb: &Leaderboard, path: &str) -> Result<(), EngineError> {
    let dir = gopath::dir(path);
    fs::create_dir_all(&dir).map_err(|e| path_error("mkdir", &dir, &e))?;
    let data = json::marshal_indent(lb, "", "  ").map_err(|e| EngineError(e.to_string()))?;
    let (tmp_path, mut f) = loop {
        let n = TMP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0);
        let name = format!("highscores-{}{}{}.tmp", std::process::id(), nanos, n);
        let p = gopath::join(&[&dir, &name]);
        let mut opts = fs::OpenOptions::new();
        opts.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        match opts.open(&p) {
            Ok(f) => break (p, f),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(path_error("open", &p, &e)),
        }
    };
    let cleanup = |p: &str| {
        let _ = fs::remove_file(p);
    };
    if let Err(e) = f.write_all(&data) {
        cleanup(&tmp_path);
        return Err(path_error("write", &tmp_path, &e));
    }
    if let Err(e) = f.sync_all() {
        cleanup(&tmp_path);
        return Err(path_error("sync", &tmp_path, &e));
    }
    // `File` has no `Drop` impl on wasm32-unknown-unknown.
    #[allow(clippy::drop_non_drop)]
    drop(f);
    if let Err(e) = fs::rename(&tmp_path, path) {
        cleanup(&tmp_path);
        return Err(EngineError(format!(
            "rename {} {}: {}",
            tmp_path,
            path,
            errno_text(&e)
        )));
    }
    Ok(())
}

/// Go `ScenarioLeaderboardPath(id)`.
pub fn scenario_leaderboard_path(id: &ScenarioID) -> String {
    let norm = normalize_scenario_id(id);
    let dir = gopath::dir(&default_leaderboard_path());
    gopath::join(&[&dir, &format!("leaderboard_{}.json", norm.0)])
}

/// Go `LoadScenarioLeaderboard(id)`.
pub fn load_scenario_leaderboard(id: &ScenarioID) -> Result<Leaderboard, EngineError> {
    let path = scenario_leaderboard_path(id);
    let data = match read_file(&path) {
        Ok(d) => d,
        Err((e, pe)) => {
            if e.kind() == io::ErrorKind::NotFound {
                return Ok(default_scenario_leaderboard(id));
            }
            return Err(pe);
        }
    };
    let mut lb = Leaderboard::default();
    if json::unmarshal(&data, &mut lb).is_err() {
        recover_corrupt(&path, &data);
        return Ok(default_scenario_leaderboard(id));
    }
    lb.normalize_loaded(false);
    Ok(lb)
}

/// Go `SaveScenarioLeaderboard(id, lb)`.
pub fn save_scenario_leaderboard(id: &ScenarioID, lb: &Leaderboard) -> Result<(), EngineError> {
    save_leaderboard(lb, &scenario_leaderboard_path(id))
}

/// Go `AddScenarioScore(id, entry)` with an injectable clock.
pub fn add_scenario_score_with_clock(
    id: &ScenarioID,
    entry: ScoreEntry,
    clock: &dyn Clock,
) -> Result<(i32, bool), EngineError> {
    let norm = normalize_scenario_id(id);
    let mut lb = load_scenario_leaderboard(&norm)?;
    let (rank, added) = add_scenario_score_to(&mut lb, &norm, entry, clock.now());
    if !added {
        return Ok((0, false));
    }
    save_scenario_leaderboard(&norm, &lb)?;
    Ok((rank, true))
}

/// Go `AddScenarioScore(id, entry)` (wall clock).
pub fn add_scenario_score(id: &ScenarioID, entry: ScoreEntry) -> Result<(i32, bool), EngineError> {
    add_scenario_score_with_clock(id, entry, &SystemClock)
}

/// Go `Leaderboard.RecordTour(tour, callsign)` (wall clock, local time).
pub fn record_tour(lb: &mut Leaderboard, tour: &TourState, callsign: &str) -> TourRecord {
    lb.record_tour_at(tour, callsign, SystemClock.now())
}

/// Go `NewTour(seed)` (a zero seed uses `time.Now().UnixNano()`).
pub fn new_tour(seed: i64) -> TourState {
    TourState::new_at(seed, SystemClock.now().unix_nano())
}
