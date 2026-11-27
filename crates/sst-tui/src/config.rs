//! Persistent user preferences (port of `pkg/config/config.go`).
//!
//! The file lives at `$HOME/.super-star-trek/config.json` (Go `os.UserHomeDir`
//! semantics: `USERPROFILE` on Windows, falling back to `HOME`). Reading
//! follows Go `encoding/json` (v1) rules: full syntax validation first,
//! exact-then-case-folded key matching, unknown keys ignored, `null` leaves
//! defaults, any syntax or type error yields an error. Writing reproduces
//! `json.MarshalIndent(cfg, "", "  ")` plus a trailing newline, written to a
//! `config-*.tmp` file (mode 0600), synced and atomically renamed.

use std::ffi::OsString;
use std::fmt;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Supported audio output subsystem (Go `type AudioMode string`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct AudioMode(pub String);

impl AudioMode {
    /// Automatically choose the best available output.
    pub fn auto() -> Self {
        AudioMode(AUDIO_MODE_AUTO.into())
    }
    /// OS-level native players (afplay, aplay, paplay).
    pub fn native() -> Self {
        AudioMode(AUDIO_MODE_NATIVE.into())
    }
    /// Terminal ASCII/visual bell.
    pub fn bell() -> Self {
        AudioMode(AUDIO_MODE_BELL.into())
    }
    /// No sound.
    pub fn off() -> Self {
        AudioMode(AUDIO_MODE_OFF.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AudioMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for AudioMode {
    fn from(s: &str) -> Self {
        AudioMode(s.to_string())
    }
}

pub const AUDIO_MODE_AUTO: &str = "auto";
pub const AUDIO_MODE_NATIVE: &str = "native";
pub const AUDIO_MODE_BELL: &str = "bell";
pub const AUDIO_MODE_OFF: &str = "off";

/// Persistent user preferences (Go `Config`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// 0..=100, default 80.
    pub volume: i32,
    pub muted: bool,
    pub audio_mode: AudioMode,
    /// "modern", "lcars" or "crt".
    pub theme: String,
    /// "fast", "normal" or "slow".
    pub anim_speed: String,
}

/// Go `DefaultConfig`.
pub fn default_config() -> Config {
    Config {
        volume: 80,
        muted: false,
        audio_mode: AudioMode::auto(),
        theme: "modern".into(),
        anim_speed: "normal".into(),
    }
}

impl Default for Config {
    fn default() -> Self {
        default_config()
    }
}

/// Go `strings.TrimSpace` + `strings.ToLower`, sufficient for comparing
/// against ASCII keywords (Go's simple lowercase maps U+0130 to 'i' and
/// U+212A to 'k').
fn go_trim_lower(s: &str) -> String {
    s.trim_matches(char::is_whitespace)
        .chars()
        .map(|c| {
            if c.is_ascii() {
                c.to_ascii_lowercase()
            } else if c == '\u{130}' {
                'i'
            } else {
                let mut l = c.to_lowercase();
                match (l.next(), l.next()) {
                    (Some(x), None) => x,
                    _ => c,
                }
            }
        })
        .collect()
}

impl Config {
    /// Go `(*Config).Normalize`: clamps volume and replaces unknown values with defaults.
    pub fn normalize(&mut self) {
        self.volume = self.volume.clamp(0, 100);
        let mode = go_trim_lower(&self.audio_mode.0);
        self.audio_mode = AudioMode(
            match mode.as_str() {
                AUDIO_MODE_AUTO | AUDIO_MODE_NATIVE | AUDIO_MODE_BELL | AUDIO_MODE_OFF => mode,
                _ => AUDIO_MODE_AUTO.into(),
            }
            .to_string(),
        );
        let theme = go_trim_lower(&self.theme);
        self.theme = match theme.as_str() {
            "modern" | "lcars" | "crt" => theme,
            _ => "modern".into(),
        };
        let speed = go_trim_lower(&self.anim_speed);
        self.anim_speed = match speed.as_str() {
            "fast" | "normal" | "slow" => speed,
            _ => "normal".into(),
        };
    }
}

/// Errors from path resolution, I/O and JSON decoding.
#[derive(Debug)]
pub enum ConfigError {
    /// Go `os.UserHomeDir` failure, e.g. "$HOME is not defined".
    NoHome(String),
    Io(std::io::Error),
    /// Go `*json.SyntaxError` (message and byte offset).
    Syntax {
        msg: String,
        offset: usize,
    },
    /// Go `*json.UnmarshalTypeError` message.
    Type(String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::NoHome(m) | ConfigError::Type(m) => f.write_str(m),
            ConfigError::Io(e) => write!(f, "{e}"),
            ConfigError::Syntax { msg, .. } => f.write_str(msg),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<std::io::Error> for ConfigError {
    fn from(e: std::io::Error) -> Self {
        ConfigError::Io(e)
    }
}

// ---------------------------------------------------------------- paths

/// Go `filepath.Clean` for '/'-separated paths.
fn clean_slash(path: &[u8]) -> Vec<u8> {
    if path.is_empty() {
        return b".".to_vec();
    }
    let rooted = path[0] == b'/';
    let n = path.len();
    let mut out: Vec<u8> = Vec::with_capacity(n);
    let (mut r, mut dotdot) = (0usize, 0usize);
    if rooted {
        out.push(b'/');
        r = 1;
        dotdot = 1;
    }
    while r < n {
        if path[r] == b'/' || (path[r] == b'.' && (r + 1 == n || path[r + 1] == b'/')) {
            r += 1;
        } else if path[r] == b'.' && path[r + 1] == b'.' && (r + 2 == n || path[r + 2] == b'/') {
            r += 2;
            if out.len() > dotdot {
                let mut w = out.len() - 1;
                while w > dotdot && out[w] != b'/' {
                    w -= 1;
                }
                out.truncate(w);
            } else if !rooted {
                if !out.is_empty() {
                    out.push(b'/');
                }
                out.extend_from_slice(b"..");
                dotdot = out.len();
            }
        } else {
            if (rooted && out.len() != 1) || (!rooted && !out.is_empty()) {
                out.push(b'/');
            }
            while r < n && path[r] != b'/' {
                out.push(path[r]);
                r += 1;
            }
        }
    }
    if out.is_empty() {
        out.push(b'.');
    }
    out
}

/// Go `filepath.Join(home, ".super-star-trek", "config.json")`.
fn join_config_path(home: OsString) -> PathBuf {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::{OsStrExt, OsStringExt};
        let mut p = home.as_os_str().as_bytes().to_vec();
        p.extend_from_slice(b"/.super-star-trek/config.json");
        PathBuf::from(OsString::from_vec(clean_slash(&p)))
    }
    #[cfg(not(unix))]
    {
        PathBuf::from(home)
            .join(".super-star-trek")
            .join("config.json")
    }
}

/// Go `ConfigPath` with an injected environment lookup (`getenv` returns
/// `None` or an empty string for unset variables).
pub fn config_path_with_env(
    getenv: impl Fn(&str) -> Option<OsString>,
) -> Result<PathBuf, ConfigError> {
    let get = |k: &str| getenv(k).filter(|v| !v.is_empty());
    let (env, enverr) = if cfg!(windows) {
        ("USERPROFILE", "%userprofile%")
    } else {
        ("HOME", "$HOME")
    };
    if let Some(home) = get(env) {
        return Ok(join_config_path(home));
    }
    if cfg!(target_os = "android") {
        return Ok(join_config_path("/sdcard".into()));
    }
    if cfg!(target_os = "ios") {
        return Ok(join_config_path("/".into()));
    }
    if let Some(h) = get("HOME") {
        return Ok(join_config_path(h));
    }
    Err(ConfigError::NoHome(format!("{enverr} is not defined")))
}

/// Go `ConfigPath`: `~/.super-star-trek/config.json`.
pub fn config_path() -> Result<PathBuf, ConfigError> {
    config_path_with_env(|k| std::env::var_os(k))
}

// ---------------------------------------------------------------- load/save

/// Go `LoadConfig`. On error Go returns `DefaultConfig()` alongside the
/// error, so `load_config().unwrap_or_default()` reproduces it. A missing
/// file yields the defaults without error.
pub fn load_config() -> Result<Config, ConfigError> {
    load_config_with_env(|k| std::env::var_os(k))
}

/// [`load_config`] with an injected environment lookup.
pub fn load_config_with_env(
    getenv: impl Fn(&str) -> Option<OsString>,
) -> Result<Config, ConfigError> {
    load_config_from(&config_path_with_env(getenv)?)
}

/// [`load_config`] for an explicit file path.
pub fn load_config_from(path: &Path) -> Result<Config, ConfigError> {
    let data = match std::fs::read(path) {
        Ok(d) => d,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(default_config()),
        Err(e) => return Err(e.into()),
    };
    let mut cfg = decode_config(&data)?;
    cfg.normalize();
    Ok(cfg)
}

/// Go `SaveConfig`: normalizes and atomically writes the configuration.
pub fn save_config(cfg: Config) -> Result<(), ConfigError> {
    save_config_with_env(cfg, |k| std::env::var_os(k))
}

/// [`save_config`] with an injected environment lookup.
pub fn save_config_with_env(
    mut cfg: Config,
    getenv: impl Fn(&str) -> Option<OsString>,
) -> Result<(), ConfigError> {
    cfg.normalize();
    let path = config_path_with_env(getenv)?;
    write_config_file(&cfg, &path)
}

/// [`save_config`] for an explicit file path.
pub fn save_config_to(mut cfg: Config, path: &Path) -> Result<(), ConfigError> {
    cfg.normalize();
    write_config_file(&cfg, path)
}

fn write_config_file(cfg: &Config, path: &Path) -> Result<(), ConfigError> {
    let dir = path
        .parent()
        .filter(|d| !d.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut b = std::fs::DirBuilder::new();
    b.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        b.mode(0o755);
    }
    b.create(dir)?;

    let mut data = marshal_indent(cfg);
    data.push(b'\n');

    let (mut f, tmp_path) = create_temp(dir, "config-", ".tmp")?;
    let result = (|| -> std::io::Result<()> {
        f.write_all(&data)?;
        f.sync_all()?;
        drop(f);
        std::fs::rename(&tmp_path, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp_path);
    }
    result.map_err(ConfigError::Io)
}

/// Go `os.CreateTemp(dir, prefix+"*"+suffix)`: random decimal infix, mode 0600.
fn create_temp(
    dir: &Path,
    prefix: &str,
    suffix: &str,
) -> std::io::Result<(std::fs::File, PathBuf)> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static CTR: AtomicU64 = AtomicU64::new(0);
    let mut last = None;
    for _ in 0..10000 {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0)
            ^ ((std::process::id() as u64) << 32)
            ^ CTR.fetch_add(0x9E3779B97F4A7C15, Ordering::SeqCst);
        let mut x = seed;
        x ^= x >> 33;
        x = x.wrapping_mul(0xff51afd7ed558ccd);
        x ^= x >> 33;
        let path = dir.join(format!("{prefix}{}{suffix}", x as u32));
        let mut o = std::fs::OpenOptions::new();
        o.read(true).write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            o.mode(0o600);
        }
        match o.open(&path) {
            Ok(f) => return Ok((f, path)),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => last = Some(e),
            Err(e) => return Err(e),
        }
    }
    Err(last.unwrap())
}

// ---------------------------------------------------------------- JSON encode

/// Go `encoding/json` string encoding with HTML escaping.
fn encode_string(out: &mut Vec<u8>, s: &[u8]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    out.push(b'"');
    let mut i = 0;
    while i < s.len() {
        let b = s[i];
        if b < 0x80 {
            match b {
                b'"' => out.extend_from_slice(b"\\\""),
                b'\\' => out.extend_from_slice(b"\\\\"),
                0x08 => out.extend_from_slice(b"\\b"),
                0x0c => out.extend_from_slice(b"\\f"),
                b'\n' => out.extend_from_slice(b"\\n"),
                b'\r' => out.extend_from_slice(b"\\r"),
                b'\t' => out.extend_from_slice(b"\\t"),
                c if c < 0x20 || c == b'<' || c == b'>' || c == b'&' => {
                    out.extend_from_slice(b"\\u00");
                    out.push(HEX[(c >> 4) as usize]);
                    out.push(HEX[(c & 0xf) as usize]);
                }
                c => out.push(c),
            }
            i += 1;
            continue;
        }
        let (c, size) = decode_rune(&s[i..]);
        match c {
            None => out.extend_from_slice("\u{FFFD}".as_bytes()),
            Some('\u{2028}') => out.extend_from_slice(b"\\u2028"),
            Some('\u{2029}') => out.extend_from_slice(b"\\u2029"),
            Some(_) => out.extend_from_slice(&s[i..i + size]),
        }
        i += size;
    }
    out.push(b'"');
}

/// Go `json.MarshalIndent(cfg, "", "  ")` (no trailing newline).
pub fn marshal_indent(cfg: &Config) -> Vec<u8> {
    let mut o = Vec::new();
    o.extend_from_slice(b"{\n  \"volume\": ");
    o.extend_from_slice(cfg.volume.to_string().as_bytes());
    o.extend_from_slice(b",\n  \"muted\": ");
    o.extend_from_slice(if cfg.muted { b"true" } else { b"false" });
    o.extend_from_slice(b",\n  \"audio_mode\": ");
    encode_string(&mut o, cfg.audio_mode.0.as_bytes());
    o.extend_from_slice(b",\n  \"theme\": ");
    encode_string(&mut o, cfg.theme.as_bytes());
    o.extend_from_slice(b",\n  \"anim_speed\": ");
    encode_string(&mut o, cfg.anim_speed.as_bytes());
    o.extend_from_slice(b"\n}");
    o
}

// ---------------------------------------------------------------- JSON decode

/// Go `utf8.DecodeRune`: `None` with size 1 for invalid encodings.
fn decode_rune(b: &[u8]) -> (Option<char>, usize) {
    for len in 1..=b.len().min(4) {
        if let Ok(s) = std::str::from_utf8(&b[..len]) {
            return (s.chars().next(), len);
        }
    }
    (None, 1)
}

/// Go `quoteChar` used in scanner error messages.
fn quote_char(c: u8) -> String {
    match c {
        b'\'' => "'\\''".into(),
        b'"' => "'\"'".into(),
        b'\\' => "'\\\\'".into(),
        0x07 => "'\\a'".into(),
        0x08 => "'\\b'".into(),
        0x0c => "'\\f'".into(),
        b'\n' => "'\\n'".into(),
        b'\r' => "'\\r'".into(),
        b'\t' => "'\\t'".into(),
        0x0b => "'\\v'".into(),
        c if c < 0x20 || c == 0x7f => format!("'\\x{c:02x}'"),
        c if (0x80..=0xa0).contains(&c) || c == 0xad => format!("'\\u{c:04x}'"),
        c => format!("'{}'", c as char),
    }
}

fn syntax(c: Option<u8>, ctx: &str, offset: usize) -> ConfigError {
    match c {
        None => ConfigError::Syntax {
            msg: "unexpected end of JSON input".into(),
            offset,
        },
        Some(c) => ConfigError::Syntax {
            msg: format!("invalid character {} {ctx}", quote_char(c)),
            offset,
        },
    }
}

const MAX_NESTING_DEPTH: usize = 10000;

#[inline]
fn is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'\r')
}

/// Validates a string literal starting at `data[i] == '"'`; returns the index after it.
fn scan_string(data: &[u8], mut i: usize) -> Result<usize, ConfigError> {
    i += 1;
    loop {
        let Some(&c) = data.get(i) else {
            return Err(syntax(None, "", i));
        };
        match c {
            b'"' => return Ok(i + 1),
            b'\\' => {
                i += 1;
                let Some(&e) = data.get(i) else {
                    return Err(syntax(None, "", i));
                };
                match e {
                    b'b' | b'f' | b'n' | b'r' | b't' | b'\\' | b'/' | b'"' => i += 1,
                    b'u' => {
                        i += 1;
                        for _ in 0..4 {
                            match data.get(i) {
                                None => return Err(syntax(None, "", i)),
                                Some(h) if h.is_ascii_hexdigit() => i += 1,
                                Some(&h) => {
                                    return Err(syntax(
                                        Some(h),
                                        "in \\u hexadecimal character escape",
                                        i,
                                    ))
                                }
                            }
                        }
                    }
                    _ => return Err(syntax(Some(e), "in string escape code", i)),
                }
            }
            c if c < 0x20 => return Err(syntax(Some(c), "in string literal", i)),
            _ => i += 1,
        }
    }
}

/// Validates a number starting at `data[i]` ('-' or digit); returns the index after it.
fn scan_number(data: &[u8], mut i: usize) -> Result<usize, ConfigError> {
    let digit = |i: usize| data.get(i).is_some_and(u8::is_ascii_digit);
    let need_digit = |i: usize, ctx: &str| -> Result<(), ConfigError> {
        match data.get(i) {
            None => Err(syntax(None, "", i)),
            Some(c) if c.is_ascii_digit() => Ok(()),
            Some(&c) => Err(syntax(Some(c), ctx, i)),
        }
    };
    if data[i] == b'-' {
        i += 1;
        need_digit(i, "in numeric literal")?;
    }
    if data[i] == b'0' {
        i += 1;
    } else {
        while digit(i) {
            i += 1;
        }
    }
    if data.get(i) == Some(&b'.') {
        i += 1;
        need_digit(i, "after decimal point in numeric literal")?;
        while digit(i) {
            i += 1;
        }
    }
    if matches!(data.get(i), Some(b'e' | b'E')) {
        i += 1;
        if matches!(data.get(i), Some(b'+' | b'-')) {
            i += 1;
        }
        need_digit(i, "in exponent of numeric literal")?;
        while digit(i) {
            i += 1;
        }
    }
    Ok(i)
}

fn scan_literal(data: &[u8], i: usize, lit: &[u8]) -> Result<usize, ConfigError> {
    for (k, &want) in lit.iter().enumerate().skip(1) {
        match data.get(i + k) {
            None => return Err(syntax(None, "", i + k)),
            Some(&c) if c == want => {}
            Some(&c) => {
                let ctx = format!(
                    "in literal {} (expecting {})",
                    std::str::from_utf8(lit).unwrap(),
                    quote_char(want)
                );
                return Err(syntax(Some(c), &ctx, i + k));
            }
        }
    }
    Ok(i + lit.len())
}

/// Scans one scalar value or container opener at `data[i]`, pushing onto `stack`.
/// Returns (next index, whether a container was opened).
fn scan_value_start(
    data: &[u8],
    i: usize,
    stack: &mut Vec<u8>,
) -> Result<(usize, bool), ConfigError> {
    let c = data[i];
    match c {
        b'{' | b'[' => {
            stack.push(c);
            if stack.len() > MAX_NESTING_DEPTH {
                return Err(syntax(Some(c), "exceeded max depth", i));
            }
            Ok((i + 1, true))
        }
        b'"' => Ok((scan_string(data, i)?, false)),
        b'-' | b'0'..=b'9' => Ok((scan_number(data, i)?, false)),
        b't' => Ok((scan_literal(data, i, b"true")?, false)),
        b'f' => Ok((scan_literal(data, i, b"false")?, false)),
        b'n' => Ok((scan_literal(data, i, b"null")?, false)),
        _ => Err(syntax(Some(c), "looking for beginning of value", i)),
    }
}

/// Go `checkValid`: full syntax validation without recursion.
fn check_valid(data: &[u8]) -> Result<(), ConfigError> {
    #[derive(Clone, Copy, PartialEq)]
    enum St {
        Value,
        ArrFirst,
        ObjFirst,
        ObjKey,
        Colon,
        After,
    }
    let mut stack: Vec<u8> = Vec::new();
    let mut st = St::Value;
    let mut i = 0;
    loop {
        while i < data.len() && is_space(data[i]) {
            i += 1;
        }
        if st == St::After && stack.is_empty() {
            return match data.get(i) {
                None => Ok(()),
                Some(&c) => Err(syntax(Some(c), "after top-level value", i)),
            };
        }
        let Some(&c) = data.get(i) else {
            return Err(syntax(None, "", i));
        };
        match st {
            St::Value => {
                let (ni, opened) = scan_value_start(data, i, &mut stack)?;
                i = ni;
                st = if !opened {
                    St::After
                } else if c == b'{' {
                    St::ObjFirst
                } else {
                    St::ArrFirst
                };
            }
            St::ArrFirst => {
                if c == b']' {
                    stack.pop();
                    i += 1;
                    st = St::After;
                } else {
                    st = St::Value;
                }
            }
            St::ObjFirst | St::ObjKey => {
                if c == b'}' && st == St::ObjFirst {
                    stack.pop();
                    i += 1;
                    st = St::After;
                } else if c == b'"' {
                    i = scan_string(data, i)?;
                    st = St::Colon;
                } else {
                    return Err(syntax(
                        Some(c),
                        "looking for beginning of object key string",
                        i,
                    ));
                }
            }
            St::Colon => {
                if c != b':' {
                    return Err(syntax(Some(c), "after object key", i));
                }
                i += 1;
                st = St::Value;
            }
            St::After => {
                let top = *stack.last().unwrap();
                match (top, c) {
                    (b'{', b',') => st = St::ObjKey,
                    (b'[', b',') => st = St::Value,
                    (b'{', b'}') | (b'[', b']') => {
                        stack.pop();
                    }
                    (b'{', _) => return Err(syntax(Some(c), "after object key:value pair", i)),
                    _ => return Err(syntax(Some(c), "after array element", i)),
                }
                i += 1;
            }
        }
    }
}

fn getu4(s: &[u8]) -> Option<u32> {
    if s.len() < 6 || s[0] != b'\\' || s[1] != b'u' {
        return None;
    }
    u32::from_str_radix(std::str::from_utf8(&s[2..6]).ok()?, 16).ok()
}

/// Go `unquote` of a validated string literal `s` (including quotes).
fn unquote(s: &[u8]) -> String {
    let s = &s[1..s.len() - 1];
    let mut out = String::with_capacity(s.len());
    let mut r = 0;
    while r < s.len() {
        let c = s[r];
        if c == b'\\' {
            let e = s[r + 1];
            match e {
                b'u' => {
                    let mut rr = getu4(&s[r..]).unwrap();
                    r += 6;
                    if (0xD800..0xE000).contains(&rr) {
                        if let Some(rr1) = getu4(&s[r..]) {
                            if (0xD800..0xDC00).contains(&rr) && (0xDC00..0xE000).contains(&rr1) {
                                let dec = 0x10000 + ((rr - 0xD800) << 10) + (rr1 - 0xDC00);
                                out.push(char::from_u32(dec).unwrap());
                                r += 6;
                                continue;
                            }
                        }
                        rr = 0xFFFD;
                    }
                    out.push(char::from_u32(rr).unwrap_or('\u{FFFD}'));
                }
                _ => {
                    out.push(match e {
                        b'b' => '\u{8}',
                        b'f' => '\u{c}',
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        other => other as char,
                    });
                    r += 2;
                }
            }
        } else if c < 0x80 {
            out.push(c as char);
            r += 1;
        } else {
            let (ch, size) = decode_rune(&s[r..]);
            out.push(ch.unwrap_or('\u{FFFD}'));
            r += size;
        }
    }
    out
}

/// Skips a validated value starting at `data[i]`; returns the index after it.
fn skip_value(data: &[u8], i: usize) -> usize {
    match data[i] {
        b'"' => scan_string(data, i).unwrap(),
        b'-' | b'0'..=b'9' => scan_number(data, i).unwrap(),
        b't' | b'n' => i + 4,
        b'f' => i + 5,
        _ => {
            let mut depth = 0usize;
            let mut i = i;
            loop {
                match data[i] {
                    b'"' => {
                        i = scan_string(data, i).unwrap();
                        continue;
                    }
                    b'{' | b'[' => depth += 1,
                    b'}' | b']' => {
                        depth -= 1;
                        if depth == 0 {
                            return i + 1;
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
        }
    }
}

/// Go `foldName` restricted to what can match an ASCII field name.
fn fold_name(key: &str) -> String {
    key.chars()
        .map(|c| match c {
            'a'..='z' => c.to_ascii_uppercase(),
            '\u{212A}' => 'K',
            '\u{17F}' => 'S',
            _ => c,
        })
        .collect()
}

const FIELDS: [&str; 5] = ["volume", "muted", "audio_mode", "theme", "anim_speed"];
const FIELD_TYPES: [&str; 5] = ["int", "bool", "config.AudioMode", "string", "string"];

fn type_err(value: &str, field: usize) -> ConfigError {
    ConfigError::Type(format!(
        "json: cannot unmarshal {value} into struct field Config.{} of type {}",
        FIELDS[field], FIELD_TYPES[field]
    ))
}

fn value_kind(c: u8) -> &'static str {
    match c {
        b'"' => "string",
        b'{' => "object",
        b'[' => "array",
        b't' | b'f' => "bool",
        _ => "number",
    }
}

fn skip_ws(data: &[u8], mut i: usize) -> usize {
    while i < data.len() && is_space(data[i]) {
        i += 1;
    }
    i
}

/// Go `json.Unmarshal(data, &cfg)` starting from [`default_config`].
pub fn decode_config(data: &[u8]) -> Result<Config, ConfigError> {
    check_valid(data)?;
    let mut cfg = default_config();
    let mut i = skip_ws(data, 0);
    match data[i] {
        b'n' => return Ok(cfg),
        b'{' => {}
        c => {
            return Err(ConfigError::Type(format!(
                "json: cannot unmarshal {} into value of type config.Config",
                value_kind(c)
            )))
        }
    }
    i = skip_ws(data, i + 1);
    if data[i] == b'}' {
        return Ok(cfg);
    }
    loop {
        let end = scan_string(data, i).unwrap();
        let key = unquote(&data[i..end]);
        i = skip_ws(data, end);
        i = skip_ws(data, i + 1); // ':'
        let field = FIELDS.iter().position(|f| *f == key).or_else(|| {
            let folded = fold_name(&key);
            FIELDS.iter().position(|f| fold_name(f) == folded)
        });
        let vstart = i;
        let vend = skip_value(data, i);
        let item = &data[vstart..vend];
        if let Some(f) = field {
            let c = item[0];
            if c != b'n' {
                match (f, c) {
                    (0, b'-' | b'0'..=b'9') => {
                        let text = std::str::from_utf8(item).unwrap();
                        match text.parse::<i64>() {
                            Ok(n) => cfg.volume = n.clamp(i32::MIN as i64, i32::MAX as i64) as i32,
                            Err(_) => return Err(type_err(&format!("number {text}"), f)),
                        }
                    }
                    (1, b't') => cfg.muted = true,
                    (1, b'f') => cfg.muted = false,
                    (2, b'"') => cfg.audio_mode = AudioMode(unquote(item)),
                    (3, b'"') => cfg.theme = unquote(item),
                    (4, b'"') => cfg.anim_speed = unquote(item),
                    _ => return Err(type_err(value_kind(c), f)),
                }
            }
        }
        i = skip_ws(data, vend);
        if data[i] == b',' {
            i = skip_ws(data, i + 1);
        } else {
            return Ok(cfg);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_home(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "sst-config-test-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn env_home(home: &Path) -> impl Fn(&str) -> Option<OsString> + '_ {
        move |k| (k == "HOME" || k == "USERPROFILE").then(|| home.as_os_str().to_owned())
    }

    // ---- config_test.go ----

    #[test]
    fn test_default_config() {
        let cfg = default_config();
        assert_eq!(cfg.volume, 80);
        assert!(!cfg.muted);
        assert_eq!(cfg.audio_mode, AudioMode::auto());
        assert_eq!(cfg.theme, "modern");
        assert_eq!(cfg.anim_speed, "normal");
    }

    #[test]
    fn save_and_load_roundtrip() {
        let home = temp_home("roundtrip");
        let cfg = Config {
            volume: 65,
            muted: true,
            audio_mode: AudioMode::native(),
            theme: "lcars".into(),
            anim_speed: "fast".into(),
        };
        save_config_with_env(cfg.clone(), env_home(&home)).unwrap();
        let loaded = load_config_with_env(env_home(&home)).unwrap();
        assert_eq!(loaded, cfg);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn load_config_missing_file_returns_default() {
        let home = temp_home("missing");
        let loaded = load_config_with_env(env_home(&home)).unwrap();
        assert_eq!(loaded.volume, 80);
        assert_eq!(loaded.theme, "modern");
        assert_eq!(loaded.audio_mode, AudioMode::auto());
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn config_validation_bounds() {
        let mut cfg = Config {
            volume: 150,
            muted: false,
            audio_mode: AudioMode::default(),
            theme: String::new(),
            anim_speed: String::new(),
        };
        cfg.normalize();
        assert_eq!(cfg.volume, 100);
        assert_eq!(cfg.audio_mode, AudioMode::auto());
        assert_eq!(cfg.theme, "modern");
        assert_eq!(cfg.anim_speed, "normal");

        cfg.volume = -20;
        cfg.audio_mode = "invalid_mode".into();
        cfg.theme = "unknown_theme".into();
        cfg.anim_speed = "ultra_hyper".into();
        cfg.normalize();
        assert_eq!(cfg.volume, 0);
        assert_eq!(cfg.audio_mode, AudioMode::auto());
        assert_eq!(cfg.theme, "modern");
        assert_eq!(cfg.anim_speed, "normal");
    }

    #[test]
    fn test_config_path() {
        let home = temp_home("path");
        let p = config_path_with_env(env_home(&home)).unwrap();
        assert_eq!(p, home.join(".super-star-trek").join("config.json"));
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn load_config_corrupted_file_returns_default_with_error() {
        let home = temp_home("corrupt");
        let dir = home.join(".super-star-trek");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("config.json"), b"{invalid json corrupt").unwrap();
        let res = load_config_with_env(env_home(&home));
        let err = res.as_ref().unwrap_err();
        assert_eq!(
            err.to_string(),
            "invalid character 'i' looking for beginning of object key string"
        );
        assert_eq!(res.unwrap_or_default().volume, 80);
        let _ = std::fs::remove_dir_all(&home);
    }

    // ---- parity against Go (testdata/parity/config) ----

    fn parity(name: &str) -> String {
        std::fs::read_to_string(format!(
            "{}/../../testdata/parity/config/{name}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap()
    }

    fn unhex(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }

    #[test]
    fn decode_parity() {
        let data = parity("decode.txt");
        let mut n = 0;
        for line in data.lines() {
            let (h, want) = line.split_once(' ').unwrap();
            let input = unhex(h);
            let got = match decode_config(&input) {
                Ok(mut c) => {
                    c.normalize();
                    format!(
                        "ok {} {} {} {} {}",
                        c.volume, c.muted, c.audio_mode, c.theme, c.anim_speed
                    )
                }
                Err(_) => "err".to_string(),
            };
            assert_eq!(got, want, "input {:?}", String::from_utf8_lossy(&input));
            n += 1;
        }
        assert!(n > 3900, "{n}");
    }

    #[test]
    fn encode_parity() {
        let data = parity("encode.txt");
        let cfgs = [
            default_config(),
            Config {
                volume: 65,
                muted: true,
                audio_mode: AudioMode::native(),
                theme: "lcars".into(),
                anim_speed: "fast".into(),
            },
            Config {
                volume: 0,
                muted: false,
                audio_mode: AudioMode::off(),
                theme: "crt".into(),
                anim_speed: "slow".into(),
            },
            Config {
                volume: 100,
                muted: true,
                audio_mode: AudioMode::bell(),
                theme: "<b>&".into(),
                anim_speed: "x".into(),
            },
            Config {
                volume: -3,
                muted: false,
                audio_mode: "weird".into(),
                theme: String::new(),
                anim_speed: String::new(),
            },
            Config {
                volume: 1000,
                muted: false,
                audio_mode: AudioMode::default(),
                theme: String::new(),
                anim_speed: String::new(),
            },
        ];
        let mut checked = 0;
        for line in data.lines() {
            let p: Vec<&str> = line.split(' ').collect();
            if p[0] == "raw" {
                let w = unhex(p[1]);
                let mut out = Vec::new();
                // Strings may hold invalid UTF-8 in Go; encode the raw bytes directly.
                out.extend_from_slice(
                    b"{\n  \"volume\": -7,\n  \"muted\": true,\n  \"audio_mode\": ",
                );
                encode_string(&mut out, &w);
                out.extend_from_slice(b",\n  \"theme\": ");
                encode_string(&mut out, &w);
                out.extend_from_slice(b",\n  \"anim_speed\": ");
                encode_string(&mut out, &w);
                out.extend_from_slice(b"\n}");
                assert_eq!(out, unhex(p[2]), "raw {:?}", String::from_utf8_lossy(&w));
                if let Ok(s) = String::from_utf8(w) {
                    let c = Config {
                        volume: -7,
                        muted: true,
                        audio_mode: AudioMode(s.clone()),
                        theme: s.clone(),
                        anim_speed: s,
                    };
                    assert_eq!(marshal_indent(&c), unhex(p[2]));
                }
            } else {
                let idx: usize = p[0].parse().unwrap();
                let home = temp_home("encode");
                save_config_with_env(cfgs[idx].clone(), env_home(&home)).unwrap();
                let path = config_path_with_env(env_home(&home)).unwrap();
                assert_eq!(std::fs::read(&path).unwrap(), unhex(p[2]), "config {idx}");
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
                    assert_eq!(format!("{mode:o}"), p[1]);
                    let dmode = std::fs::metadata(path.parent().unwrap())
                        .unwrap()
                        .permissions()
                        .mode()
                        & 0o777;
                    assert_eq!(dmode, 0o755);
                }
                // No temp files left behind.
                let entries: Vec<_> = std::fs::read_dir(path.parent().unwrap()).unwrap().collect();
                assert_eq!(entries.len(), 1);
                let _ = std::fs::remove_dir_all(&home);
            }
            checked += 1;
        }
        assert_eq!(checked, 14);
    }

    #[cfg(unix)]
    #[test]
    fn path_parity() {
        let data = parity("paths.txt");
        for line in data.lines() {
            let (q, rest) = line[1..].split_once("\" ").unwrap();
            let q = q.to_string();
            let got = match config_path_with_env(|k| (k == "HOME").then(|| OsString::from(&q))) {
                Ok(p) => format!("ok {}", p.display()),
                Err(e) => format!("err {e}"),
            };
            assert_eq!(got, rest, "HOME={q:?}");
        }
    }

    #[test]
    fn syntax_error_messages() {
        let msg = |s: &str| decode_config(s.as_bytes()).unwrap_err().to_string();
        assert_eq!(msg(""), "unexpected end of JSON input");
        assert_eq!(
            msg("{\"a\":01}"),
            "invalid character '1' after object key:value pair"
        );
        assert_eq!(
            msg("{\"a\":tru}"),
            "invalid character '}' in literal true (expecting 'e')"
        );
        assert_eq!(msg("{} x"), "invalid character 'x' after top-level value");
        assert_eq!(
            msg("{\"volume\":\"1\"}"),
            "json: cannot unmarshal string into struct field Config.volume of type int"
        );
        assert_eq!(
            msg("[]"),
            "json: cannot unmarshal array into value of type config.Config"
        );
    }
}
