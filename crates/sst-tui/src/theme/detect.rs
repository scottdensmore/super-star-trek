//! Terminal background detection (port of `pkg/tui/theme/detect.go`,
//! `detect_unix.go`, `detect_other.go`).
//!
//! Go keeps the injectable hooks (`osGetenv`, `runtimeGOOS`, `oscDetector`,
//! ...) and the caches in package globals. Here they live in a per-thread
//! [`DetectState`] (the tuikit event loop runs `Update`/`View` and all
//! commands on one thread, and lipgloss' dark-background flag is per-thread
//! too), which also isolates parallel tests. Tests replace hooks with
//! [`with_state`]; component and model tests can pin the result with
//! [`set_detect_impl`] (Go `detectDarkBackgroundImpl`).

use std::cell::RefCell;
use std::io::Read;
use std::process::{Command, Stdio};
use std::rc::Rc;
use std::time::{Duration, Instant};

use sst_engine::gofmt;
use sst_tuikit::lipgloss;

/// `func() bool` hook.
pub type BoolFn = Rc<dyn Fn() -> bool>;
/// `os.Getenv` hook.
pub type GetenvFn = Rc<dyn Fn(&str) -> String>;
/// `queryAppleScriptTerminalBackground` hook (the timeout replaces Go's context).
pub type AppleScriptFn = Rc<dyn Fn(Duration) -> Result<Vec<u8>, String>>;
/// `queryOSCDarkBackground` hook: `(isDark, ok)`.
pub type OscFn = Rc<dyn Fn(Duration) -> (bool, bool)>;

/// Go's detection globals (hooks and caches).
pub struct DetectState {
    pub os_getenv: GetenvFn,
    /// Go `runtime.GOOS` naming ("darwin", "linux", "windows", "js", ...).
    pub runtime_goos: String,
    pub darwin_detector: BoolFn,
    pub darwin_system_dark_detector: BoolFn,
    pub apple_script_detector: AppleScriptFn,
    pub osc_detector: OscFn,
    /// Go `detectDarkBackgroundImpl`; `None` means `detectDarkBackgroundInternal`.
    pub detect_impl: Option<BoolFn>,

    pub detect_last_check: Option<Instant>,
    pub detect_cached_val: bool,
    pub detect_has_cached: bool,
    pub detect_cache_ttl: Duration,

    pub darwin_last_check: Option<Instant>,
    pub darwin_cached_val: bool,
    pub darwin_cache_ttl: Duration,
}

/// Go `runtime.GOOS` of the build target.
pub fn go_os() -> &'static str {
    match std::env::consts::OS {
        "macos" => "darwin",
        "" if cfg!(target_arch = "wasm32") => "js",
        other => other,
    }
}

impl Default for DetectState {
    fn default() -> Self {
        DetectState {
            os_getenv: Rc::new(|k| std::env::var(k).unwrap_or_default()),
            runtime_goos: go_os().to_string(),
            darwin_detector: Rc::new(detect_darwin_terminal_dark_background),
            darwin_system_dark_detector: Rc::new(is_darwin_system_dark),
            apple_script_detector: Rc::new(query_apple_script_terminal_background),
            osc_detector: Rc::new(query_osc_dark_background),
            detect_impl: None,
            detect_last_check: None,
            detect_cached_val: false,
            detect_has_cached: false,
            detect_cache_ttl: Duration::from_secs(2),
            darwin_last_check: None,
            darwin_cached_val: false,
            darwin_cache_ttl: Duration::from_secs(1),
        }
    }
}

thread_local! {
    static STATE: RefCell<DetectState> = RefCell::new(DetectState::default());
}

/// Runs `f` with mutable access to this thread's detection state. Do not
/// call detection functions from inside `f`.
pub fn with_state<R>(f: impl FnOnce(&mut DetectState) -> R) -> R {
    STATE.with(|s| f(&mut s.borrow_mut()))
}

/// Sets Go's `detectDarkBackgroundImpl` (None restores the real detector),
/// returning the previous override.
pub fn set_detect_impl(f: Option<BoolFn>) -> Option<BoolFn> {
    with_state(|s| std::mem::replace(&mut s.detect_impl, f))
}

/// Pins [`detect_dark_background`] to `v` on this thread (also setting
/// lipgloss' dark-background flag like the real detector does).
pub fn set_fixed_detection(v: bool) {
    set_detect_impl(Some(Rc::new(move || {
        lipgloss::set_has_dark_background(v);
        v
    })));
}

/// Resets this thread's detection state (hooks and caches) to the defaults.
pub fn reset_state() {
    with_state(|s| *s = DetectState::default());
}

fn getenv(k: &str) -> String {
    let f = with_state(|s| s.os_getenv.clone());
    f(k)
}

fn expired(last: Option<Instant>, ttl: Duration) -> bool {
    match last {
        Some(t) => t.elapsed() >= ttl,
        None => true,
    }
}

/// `DetectDarkBackground`: true if the terminal has a dark background.
pub fn detect_dark_background() -> bool {
    match with_state(|s| s.detect_impl.clone()) {
        Some(f) => f(),
        None => detect_dark_background_internal(),
    }
}

/// `ResetDetectionCache`: clears the background detection cache.
pub fn reset_detection_cache() {
    with_state(|s| {
        s.detect_has_cached = false;
        s.detect_last_check = None;
    });
}

/// Go `detectDarkBackgroundInternal`: cached detection that also calls
/// `lipgloss.SetHasDarkBackground`.
pub fn detect_dark_background_internal() -> bool {
    let cached = with_state(|s| {
        if s.detect_has_cached
            && !s.detect_cache_ttl.is_zero()
            && !expired(s.detect_last_check, s.detect_cache_ttl)
        {
            Some(s.detect_cached_val)
        } else {
            None
        }
    });
    if let Some(val) = cached {
        lipgloss::set_has_dark_background(val);
        return val;
    }

    let val = detect_dark_background_uncached();

    with_state(|s| {
        s.detect_cached_val = val;
        s.detect_has_cached = true;
        s.detect_last_check = Some(Instant::now());
    });

    lipgloss::set_has_dark_background(val);
    val
}

fn detect_dark_background_uncached() -> bool {
    // 1. Explicit COLORFGBG environment variable.
    let color_fgbg = getenv("COLORFGBG");
    if color_fgbg.contains(';') {
        let bg_str = color_fgbg.split(';').next_back().unwrap_or("").trim();
        if let Ok(bg) = gofmt::atoi(bg_str) {
            return !(bg == 7 || bg >= 11);
        }
    }

    let goos = with_state(|s| s.runtime_goos.clone());

    // 2. Darwin & Apple_Terminal: Terminal.app does not support OSC 11 queries.
    if goos == "darwin" && getenv("TERM_PROGRAM") == "Apple_Terminal" {
        let cached = with_state(|s| {
            if !s.darwin_cache_ttl.is_zero() && !expired(s.darwin_last_check, s.darwin_cache_ttl) {
                Some(s.darwin_cached_val)
            } else {
                None
            }
        });
        if let Some(v) = cached {
            return v;
        }
        let det = with_state(|s| s.darwin_detector.clone());
        let is_dark = det();
        with_state(|s| {
            s.darwin_cached_val = is_dark;
            s.darwin_last_check = Some(Instant::now());
        });
        return is_dark;
    }

    // 3. Fast OSC 11 terminal query (50ms timeout).
    let osc = with_state(|s| s.osc_detector.clone());
    let (is_dark, ok) = osc(Duration::from_millis(50));
    if ok {
        return is_dark;
    }

    // 4. Heuristics when OSC 11 is unsupported or unanswered.
    if goos == "darwin" {
        let f = with_state(|s| s.darwin_system_dark_detector.clone());
        return f();
    }
    if getenv("TERM_PROGRAM") == "Apple_Terminal" {
        return false;
    }

    // Standard default: dark background.
    true
}

/// Go `detectDarwinTerminalDarkBackground`: queries Terminal.app via
/// AppleScript, falling back to the macOS system appearance.
pub fn detect_darwin_terminal_dark_background() -> bool {
    let (script, system_dark) = with_state(|s| {
        (
            s.apple_script_detector.clone(),
            s.darwin_system_dark_detector.clone(),
        )
    });
    if let Ok(out) = script(Duration::from_millis(250)) {
        let out_str = String::from_utf8_lossy(&out);
        let out_str = out_str.trim();
        if let (is_dark, true) = parse_apple_script_rgb(out_str) {
            if is_dark {
                return true;
            }
            // Default (pure white) profiles render dark in macOS Dark Mode.
            if is_apple_script_default_white(out_str) && system_dark() {
                return true;
            }
            return false;
        }
    }
    system_dark()
}

/// Runs a command like Go `exec.CommandContext(ctx, ...).Output()` with a
/// context timeout: stdout on success, an error on failure, non-zero exit or
/// timeout (the process is killed).
fn command_output(name: &str, args: &[&str], timeout: Duration) -> Result<Vec<u8>, String> {
    let mut child = Command::new(name)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;
    let mut stdout = child.stdout.take();
    let reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(out) = stdout.as_mut() {
            let _ = out.read_to_end(&mut buf);
        }
        buf
    });
    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(st)) => break st,
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = reader.join();
                    return Err("signal: killed".into());
                }
                std::thread::sleep(Duration::from_millis(2));
            }
            Err(e) => return Err(e.to_string()),
        }
    };
    let out = reader.join().unwrap_or_default();
    if status.success() {
        Ok(out)
    } else {
        Err(format!("exit status {status}"))
    }
}

/// Go `queryAppleScriptTerminalBackground`.
pub fn query_apple_script_terminal_background(timeout: Duration) -> Result<Vec<u8>, String> {
    command_output(
        "osascript",
        &[
            "-e",
            r#"tell application "Terminal" to get background color of selected tab of front window"#,
        ],
        timeout,
    )
}

/// Go `isDarwinSystemDark`: macOS appearance is Dark Mode.
pub fn is_darwin_system_dark() -> bool {
    match command_output(
        "defaults",
        &["read", "-g", "AppleInterfaceStyle"],
        Duration::from_millis(150),
    ) {
        Ok(out) => String::from_utf8_lossy(&out).trim() == "Dark",
        Err(_) => false,
    }
}

fn parse_three_floats(s: &str) -> Option<(f64, f64, f64)> {
    let parts: Vec<&str> = s.trim().split(',').collect();
    if parts.len() != 3 {
        return None;
    }
    let r = gofmt::parse_float(parts[0].trim()).ok()?;
    let g = gofmt::parse_float(parts[1].trim()).ok()?;
    let b = gofmt::parse_float(parts[2].trim()).ok()?;
    Some((r, g, b))
}

/// Go `isAppleScriptDefaultWhite`: RGB is (near) 65535,65535,65535.
pub fn is_apple_script_default_white(s: &str) -> bool {
    match parse_three_floats(s) {
        Some((r, g, b)) => r >= 65530.0 && g >= 65530.0 && b >= 65530.0,
        None => false,
    }
}

/// Go `parseAppleScriptRGB`: `(isDark, ok)` from 16-bit "r, g, b".
pub fn parse_apple_script_rgb(s: &str) -> (bool, bool) {
    match parse_three_floats(s) {
        Some((r, g, b)) => {
            let lum = (0.299 * r + 0.587 * g + 0.114 * b) / 65535.0;
            (lum < 0.5, true)
        }
        None => (false, false),
    }
}

/// Go `queryOSCDarkBackground` (unix): sends an OSC 11 query to `/dev/tty`
/// in raw mode and waits up to `timeout` for the reply.
#[cfg(unix)]
pub fn query_osc_dark_background(timeout: Duration) -> (bool, bool) {
    use sst_tuikit::term;

    let tty = match term::Tty::open() {
        Ok(t) => t,
        Err(_) => return (false, false),
    };
    let fd = tty.fd();
    let old = match term::make_raw(fd) {
        Ok(s) => s,
        Err(_) => return (false, false),
    };
    let result = (|| {
        if term::write_all(fd, b"\x1b]11;?\x1b\\").is_err() {
            return (false, false);
        }
        match term::wait_readable(fd, timeout) {
            Ok(true) => {}
            _ => return (false, false),
        }
        let mut buf = [0u8; 128];
        let nr = match term::read(fd, &mut buf) {
            Ok(n) if n > 0 => n,
            _ => return (false, false),
        };
        parse_osc11_response(&String::from_utf8_lossy(&buf[..nr]))
    })();
    let _ = term::restore(fd, &old);
    drop(tty);
    result
}

/// Go `queryOSCDarkBackground` (windows/js/wasm): unsupported.
#[cfg(not(unix))]
pub fn query_osc_dark_background(_timeout: Duration) -> (bool, bool) {
    (false, false)
}

/// Go `parseOSC11Response`: parses "\x1b]11;rgb:rrrr/gggg/bbbb\x1b\\".
pub fn parse_osc11_response(s: &str) -> (bool, bool) {
    let idx = match s.find("rgb:") {
        Some(i) => i,
        None => return (false, false),
    };
    let s = s[idx + 4..].trim_end_matches(['\x1b', '\\', '\x07', '\r', '\n', ' ']);
    let parts: Vec<&str> = s.split('/').collect();
    if parts.len() != 3 {
        return (false, false);
    }
    let (r, g, b) = match (
        parse_hex_component(parts[0]),
        parse_hex_component(parts[1]),
        parse_hex_component(parts[2]),
    ) {
        (Some(r), Some(g), Some(b)) => (r, g, b),
        _ => return (false, false),
    };
    let lum = 0.299 * r + 0.587 * g + 0.114 * b;
    (lum < 0.5, true)
}

/// Go `parseHexComponent`: hex value scaled by its digit count to 0..=1.
pub fn parse_hex_component(hex: &str) -> Option<f64> {
    if hex.is_empty() || !hex.bytes().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let val = u64::from_str_radix(hex, 16).ok()?;
    // Go: float64((uint64(1) << (len*4)) - 1); shifts >= 64 give 0, so the
    // subtraction wraps to MaxUint64.
    let shift = (hex.len() * 4) as u32;
    let max_val = 1u64.checked_shl(shift).unwrap_or(0).wrapping_sub(1) as f64;
    if max_val <= 0.0 {
        return None;
    }
    Some(val as f64 / max_val)
}
