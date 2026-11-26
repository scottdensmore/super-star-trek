//! Terminal bell and native OS players (port of `pkg/audio/player.go`).

use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::sound::*;

static TEST_MODE: AtomicBool = AtomicBool::new(false);

/// Emulates Go's `testing.Testing()`: true inside this crate's unit tests or
/// after [`set_test_mode`]`(true)`. In test mode the players never write the
/// bell to stdout/stderr and never run OS audio binaries, exactly like Go.
pub fn testing() -> bool {
    cfg!(test) || TEST_MODE.load(Ordering::SeqCst)
}

/// Enables or disables test mode (see [`testing`]); call from other crates' tests.
pub fn set_test_mode(on: bool) {
    TEST_MODE.store(on, Ordering::SeqCst);
}

/// Visual bell callback; may be invoked from the native player's worker thread.
pub type VisualBell = Arc<dyn Fn() + Send + Sync>;

/// Destination for the ASCII bell (Go `io.Writer`, possibly `os.Stdout`/`os.Stderr` or nil).
pub enum BellWriter {
    /// Go `nil` writer: nothing is written.
    None,
    Stdout,
    Stderr,
    Writer(Box<dyn Write + Send>),
}

impl BellWriter {
    fn write_bell(&mut self) {
        match self {
            BellWriter::None => {}
            BellWriter::Stdout => {
                if !testing() {
                    let mut o = std::io::stdout();
                    let _ = o.write_all(b"\x07");
                    let _ = o.flush();
                }
            }
            BellWriter::Stderr => {
                if !testing() {
                    let mut o = std::io::stderr();
                    let _ = o.write_all(b"\x07");
                    let _ = o.flush();
                }
            }
            BellWriter::Writer(w) => {
                let _ = w.write_all(b"\x07");
            }
        }
    }
}

struct BellState {
    w: BellWriter,
    muted: bool,
    volume: i32,
}

/// Writes `\a` and calls the visual bell for combat/alert sounds (Go `TerminalBellPlayer`).
pub struct TerminalBellPlayer {
    state: Mutex<BellState>,
    visual_bell: Option<VisualBell>,
}

impl TerminalBellPlayer {
    /// Go `NewTerminalBellPlayer(w, visualBell)`: unmuted, volume 80.
    pub fn new(w: BellWriter, visual_bell: Option<VisualBell>) -> Self {
        TerminalBellPlayer {
            state: Mutex::new(BellState {
                w,
                muted: false,
                volume: 80,
            }),
            visual_bell,
        }
    }
}

/// Sounds that ring the terminal bell.
fn is_bell_sound(sound: &SoundID) -> bool {
    [
        SOUND_RED_ALERT,
        SOUND_DAMAGE,
        SOUND_EXPLOSION,
        SOUND_PHASER,
        SOUND_TORPEDO_LAUNCH,
    ]
    .contains(sound)
}

impl Player for TerminalBellPlayer {
    fn play(&self, sound: SoundID) {
        // Go holds the lock while writing and calling the visual bell.
        let mut st = self.state.lock().unwrap();
        if st.muted || st.volume == 0 {
            return;
        }
        if is_bell_sound(&sound) {
            st.w.write_bell();
            if let Some(vb) = &self.visual_bell {
                vb();
            }
        }
    }

    fn set_muted(&self, muted: bool) {
        self.state.lock().unwrap().muted = muted;
    }

    fn is_muted(&self) -> bool {
        self.state.lock().unwrap().muted
    }

    fn set_volume(&self, vol: i32) {
        self.state.lock().unwrap().volume = clamp_volume(vol);
    }

    fn volume(&self) -> i32 {
        self.state.lock().unwrap().volume
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use native::*;

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::*;
    use crate::wav::{encode_wav, raw_samples, scale_samples, SAMPLE_RATE};
    use std::collections::{HashMap, VecDeque};
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};
    use std::sync::{Condvar, OnceLock};
    use std::time::{Duration, Instant};

    /// Fallback player shared with the worker thread.
    pub type SharedPlayer = Arc<dyn Player + Send + Sync>;

    /// OS playback WAV: volume-scaled samples plus 60 ms of trailing silence
    /// (Go `synthesizeOSPlaybackWav`).
    pub fn synthesize_os_playback_wav(sound: &SoundID, vol: i32) -> Vec<u8> {
        let vol = vol.clamp(0, 100);
        let raw = raw_samples(sound);
        let tail_pad = (SAMPLE_RATE * 60 / 1000) as usize;
        encode_wav(&scale_samples(&raw, vol, tail_pad))
    }

    fn wav_file_cache() -> &'static Mutex<HashMap<(SoundID, i32), PathBuf>> {
        static C: OnceLock<Mutex<HashMap<(SoundID, i32), PathBuf>>> = OnceLock::new();
        C.get_or_init(Default::default)
    }

    /// Go `os.TempDir()`.
    pub fn go_temp_dir() -> PathBuf {
        #[cfg(windows)]
        {
            std::env::temp_dir()
        }
        #[cfg(not(windows))]
        {
            match std::env::var_os("TMPDIR") {
                Some(d) if !d.is_empty() => PathBuf::from(d),
                _ => PathBuf::from("/tmp"),
            }
        }
    }

    fn next_random() -> u32 {
        use std::sync::atomic::AtomicU64;
        static STATE: AtomicU64 = AtomicU64::new(0);
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0)
            ^ ((std::process::id() as u64) << 32);
        let mut x = STATE.fetch_add(0x9E3779B97F4A7C15, Ordering::SeqCst) ^ seed;
        x ^= x >> 33;
        x = x.wrapping_mul(0xff51afd7ed558ccd);
        x ^= x >> 33;
        x as u32
    }

    /// Go `os.CreateTemp(dir, prefix+"*"+suffix)`: mode 0600, random decimal infix.
    pub fn create_temp(
        dir: &Path,
        prefix: &str,
        suffix: &str,
    ) -> std::io::Result<(std::fs::File, PathBuf)> {
        let mut last_err = None;
        for _ in 0..10000 {
            let name = format!("{prefix}{}{suffix}", next_random());
            let path = dir.join(name);
            let mut opts = std::fs::OpenOptions::new();
            opts.read(true).write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                opts.mode(0o600);
            }
            match opts.open(&path) {
                Ok(f) => return Ok((f, path)),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => last_err = Some(e),
                Err(e) => return Err(e),
            }
        }
        Err(last_err.unwrap())
    }

    /// Returns a cached temp WAV path for (sound, vol), writing it on first use
    /// (Go `getOrWriteWavFile`; the "remove after" flag is always false).
    pub fn get_or_write_wav_file(sound: &SoundID, vol: i32) -> std::io::Result<(PathBuf, bool)> {
        let key = (sound.clone(), vol);
        if let Some(p) = wav_file_cache().lock().unwrap().get(&key) {
            if std::fs::metadata(p).is_ok() {
                return Ok((p.clone(), false));
            }
        }
        let data = synthesize_os_playback_wav(sound, vol);
        let (mut f, tmp_path) = create_temp(&go_temp_dir(), "sst-audio-", ".wav")?;
        if let Err(e) = f.write_all(&data) {
            drop(f);
            let _ = std::fs::remove_file(&tmp_path);
            return Err(e);
        }
        drop(f);
        let mut cache = wav_file_cache().lock().unwrap();
        if let Some(existing) = cache.get(&key) {
            if std::fs::metadata(existing).is_ok() {
                let existing = existing.clone();
                drop(cache);
                let _ = std::fs::remove_file(&tmp_path);
                return Ok((existing, false));
            }
        }
        cache.insert(key, tmp_path.clone());
        Ok((tmp_path, false))
    }

    /// Sounds that start a new turn action and flush stale queued sounds.
    pub fn is_primary_action_sound(sound: &SoundID) -> bool {
        [
            SOUND_WARP,
            SOUND_PHASER,
            SOUND_TORPEDO_LAUNCH,
            SOUND_SHIELDS,
            SOUND_DOCK,
            SOUND_COMM_CHIME,
            SOUND_COMPUTER_BEEP,
        ]
        .contains(sound)
    }

    fn base_name(cmd: &str) -> &str {
        // Go filepath.Base (separator '/' on unix, '/' or '\\' on windows).
        let trimmed = cmd.trim_end_matches(|c| c == '/' || (cfg!(windows) && c == '\\'));
        if trimmed.is_empty() {
            return if cmd.is_empty() { "." } else { "/" };
        }
        match trimmed.rfind(|c| c == '/' || (cfg!(windows) && c == '\\')) {
            Some(i) => &trimmed[i + 1..],
            None => trimmed,
        }
    }

    /// Whether `cmd` names afplay, aplay or paplay.
    pub fn is_os_audio_binary(cmd: &str) -> bool {
        matches!(base_name(cmd), "afplay" | "aplay" | "paplay")
    }

    /// Go `exec.LookPath` (unix semantics; results relative to "." are rejected like `ErrDot`).
    pub fn look_path(file: &str) -> Option<String> {
        fn is_executable(p: &Path) -> bool {
            match std::fs::metadata(p) {
                Ok(m) if !m.is_dir() => {
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        m.permissions().mode() & 0o111 != 0
                    }
                    #[cfg(not(unix))]
                    {
                        true
                    }
                }
                _ => false,
            }
        }
        #[cfg(windows)]
        {
            let exts: Vec<String> = std::env::var("PATHEXT")
                .ok()
                .filter(|s| !s.is_empty())
                .map(|s| s.split(';').map(|e| e.to_lowercase()).collect())
                .unwrap_or_else(|| {
                    vec![".com".into(), ".exe".into(), ".bat".into(), ".cmd".into()]
                });
            let path = std::env::var_os("PATH").unwrap_or_default();
            for dir in std::env::split_paths(&path) {
                for e in &exts {
                    let p = dir.join(format!("{file}{e}"));
                    if is_executable(&p) && p.is_absolute() {
                        return Some(p.to_string_lossy().into_owned());
                    }
                }
            }
            None
        }
        #[cfg(not(windows))]
        {
            if file.contains('/') {
                return if is_executable(Path::new(file)) {
                    Some(file.to_string())
                } else {
                    None
                };
            }
            let path = std::env::var("PATH").unwrap_or_default();
            for dir in path.split(':') {
                let dir = if dir.is_empty() { "." } else { dir };
                let p = Path::new(dir).join(file);
                if is_executable(&p) {
                    if !p.is_absolute() {
                        // Go returns ErrDot here; the caller treats it as not found.
                        return None;
                    }
                    return Some(p.to_string_lossy().into_owned());
                }
            }
            None
        }
    }

    /// Go `findOSAudioPlayer`: afplay on macOS, else paplay, else aplay, else "".
    pub fn find_os_audio_player() -> String {
        if cfg!(target_os = "macos") {
            if let Some(p) = look_path("afplay") {
                return p;
            }
        }
        for name in ["paplay", "aplay"] {
            if let Some(p) = look_path(name) {
                return p;
            }
        }
        String::new()
    }

    struct NativeState {
        muted: bool,
        volume: i32,
        cmd_path: String,
        fallback: Option<SharedPlayer>,
        last_sound: SoundID,
        last_time: Option<Instant>,
    }

    struct Inner {
        state: Mutex<NativeState>,
        /// Go `chan SoundID` with capacity 8.
        queue: Mutex<VecDeque<SoundID>>,
        queue_cv: Condvar,
        worker_started: std::sync::Once,
    }

    const QUEUE_CAP: usize = 8;

    /// Plays synthesized WAVs through afplay/paplay/aplay sequentially on a
    /// background worker, falling back to another player (Go `NativeOSPlayer`).
    pub struct NativeOSPlayer {
        inner: Arc<Inner>,
    }

    impl NativeOSPlayer {
        /// Go `NewNativeOSPlayer(w, visualBell)`: detects the OS command, uses a
        /// [`TerminalBellPlayer`] fallback (a [`NullPlayer`] in test mode) and
        /// pre-renders common sounds in the background.
        pub fn new(w: BellWriter, visual_bell: Option<VisualBell>) -> Self {
            let fallback: SharedPlayer = if testing() {
                Arc::new(NullPlayer::new())
            } else {
                Arc::new(TerminalBellPlayer::new(w, visual_bell))
            };
            let p = Self::with_cmd(Some(fallback), find_os_audio_player());
            let cmd = p.inner.state.lock().unwrap().cmd_path.clone();
            if !testing() && is_os_audio_binary(&cmd) {
                std::thread::spawn(|| {
                    for id in [
                        SOUND_WARP,
                        SOUND_PHASER,
                        SOUND_TORPEDO_LAUNCH,
                        SOUND_SHIELDS,
                        SOUND_EXPLOSION,
                        SOUND_DAMAGE,
                    ] {
                        let _ = get_or_write_wav_file(&id, 80);
                    }
                });
            }
            p
        }

        /// Go `newNativeOSPlayerWithCmd(fallback, cmdPath)`.
        pub fn with_cmd(fallback: Option<SharedPlayer>, cmd_path: impl Into<String>) -> Self {
            NativeOSPlayer {
                inner: Arc::new(Inner {
                    state: Mutex::new(NativeState {
                        muted: false,
                        volume: 80,
                        cmd_path: cmd_path.into(),
                        fallback,
                        last_sound: SoundID::default(),
                        last_time: None,
                    }),
                    queue: Mutex::new(VecDeque::new()),
                    queue_cv: Condvar::new(),
                    worker_started: std::sync::Once::new(),
                }),
            }
        }

        /// The detected OS command path ("" when none).
        pub fn cmd_path(&self) -> String {
            self.inner.state.lock().unwrap().cmd_path.clone()
        }

        /// Number of sounds waiting in the worker queue (Go `len(p.queue)`).
        pub fn queued(&self) -> usize {
            self.inner.queue.lock().unwrap().len()
        }

        fn start_worker(inner: &Arc<Inner>) {
            let inner = inner.clone();
            std::thread::spawn(move || loop {
                let sound = {
                    let mut q = inner.queue.lock().unwrap();
                    while q.is_empty() {
                        q = inner.queue_cv.wait(q).unwrap();
                    }
                    q.pop_front().unwrap()
                };
                play_one(&inner, &sound);
                if !inner.queue.lock().unwrap().is_empty() {
                    std::thread::sleep(Duration::from_millis(45));
                }
            });
        }

        /// Queue admission for the OS-binary path (dedupe within 80 ms, flush
        /// backlog on primary actions, drop when the queue is full). Returns
        /// whether the sound was queued.
        fn enqueue(&self, sound: SoundID, now: Instant) -> bool {
            {
                let mut st = self.inner.state.lock().unwrap();
                if sound == st.last_sound
                    && st.last_time.is_some_and(|t| {
                        now.saturating_duration_since(t) < Duration::from_millis(80)
                    })
                {
                    return false;
                }
                st.last_sound = sound.clone();
                st.last_time = Some(now);
                if is_primary_action_sound(&sound) {
                    self.inner.queue.lock().unwrap().clear();
                }
            }
            let mut q = self.inner.queue.lock().unwrap();
            if q.len() >= QUEUE_CAP {
                return false;
            }
            q.push_back(sound);
            self.inner.queue_cv.notify_one();
            true
        }
    }

    fn play_one(inner: &Inner, sound: &SoundID) {
        let (vol, cmd, fallback) = {
            let st = inner.state.lock().unwrap();
            if st.muted || st.volume == 0 {
                return;
            }
            (st.volume, st.cmd_path.clone(), st.fallback.clone())
        };
        let (tmp_path, remove_after) = match get_or_write_wav_file(sound, vol) {
            Ok(v) => v,
            Err(_) => {
                if let Some(fb) = fallback {
                    fb.play(sound.clone());
                }
                return;
            }
        };
        let mut c = Command::new(&cmd);
        if base_name(&cmd) == "aplay" {
            c.arg("-q");
        }
        c.arg(&tmp_path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let failed = run_with_timeout(c, Duration::from_secs(3));
        if failed == Some(true) {
            if let Some(fb) = fallback {
                fb.play(sound.clone());
            }
        }
        if remove_after {
            let _ = std::fs::remove_file(&tmp_path);
        }
    }

    /// Runs the command, killing it after `timeout`. Returns `Some(failed)`,
    /// or `None` when the timeout expired (Go ignores errors once ctx is done).
    fn run_with_timeout(mut c: Command, timeout: Duration) -> Option<bool> {
        let deadline = Instant::now() + timeout;
        let mut child = match c.spawn() {
            Ok(ch) => ch,
            Err(_) => return Some(true),
        };
        loop {
            match child.try_wait() {
                Ok(Some(status)) => return Some(!status.success()),
                Ok(None) => {
                    if Instant::now() >= deadline {
                        let _ = child.kill();
                        let _ = child.wait();
                        return None;
                    }
                    std::thread::sleep(Duration::from_millis(5));
                }
                Err(_) => return Some(true),
            }
        }
    }

    impl Player for NativeOSPlayer {
        fn play(&self, sound: SoundID) {
            let (muted, volume, cmd, fallback) = {
                let st = self.inner.state.lock().unwrap();
                (
                    st.muted,
                    st.volume,
                    st.cmd_path.clone(),
                    st.fallback.clone(),
                )
            };
            if muted || volume == 0 {
                return;
            }
            if cmd.is_empty() {
                if let Some(fb) = fallback {
                    fb.play(sound);
                }
                return;
            }
            let os_binary = is_os_audio_binary(&cmd);
            if testing() && os_binary {
                return;
            }
            if os_binary {
                self.enqueue(sound, Instant::now());
                self.inner
                    .worker_started
                    .call_once(|| Self::start_worker(&self.inner));
                return;
            }
            let inner = self.inner.clone();
            std::thread::spawn(move || play_one(&inner, &sound));
        }

        fn set_muted(&self, muted: bool) {
            let fb = {
                let mut st = self.inner.state.lock().unwrap();
                st.muted = muted;
                st.fallback.clone()
            };
            if let Some(fb) = fb {
                fb.set_muted(muted);
            }
        }

        fn is_muted(&self) -> bool {
            self.inner.state.lock().unwrap().muted
        }

        fn set_volume(&self, vol: i32) {
            let vol = clamp_volume(vol);
            let fb = {
                let mut st = self.inner.state.lock().unwrap();
                st.volume = vol;
                st.fallback.clone()
            };
            if let Some(fb) = fb {
                fb.set_volume(vol);
            }
        }

        fn volume(&self) -> i32 {
            self.inner.state.lock().unwrap().volume
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn enqueue_semantics() {
            let p = NativeOSPlayer::with_cmd(None, "/usr/bin/afplay");
            let t0 = Instant::now();
            assert!(p.enqueue(SOUND_EXPLOSION, t0));
            // Same sound within 80ms is dropped.
            assert!(!p.enqueue(SOUND_EXPLOSION, t0 + Duration::from_millis(79)));
            assert!(p.enqueue(SOUND_EXPLOSION, t0 + Duration::from_millis(80)));
            assert!(p.enqueue(SOUND_DAMAGE, t0 + Duration::from_millis(81)));
            assert_eq!(p.queued(), 3);
            // A primary action clears the backlog before queueing itself.
            assert!(p.enqueue(SOUND_PHASER, t0 + Duration::from_millis(82)));
            assert_eq!(p.queued(), 1);
            // Capacity 8: further sounds are dropped.
            let mut t = t0 + Duration::from_millis(100);
            let alt = [SOUND_EXPLOSION, SOUND_DAMAGE];
            for i in 0..10 {
                t += Duration::from_millis(1);
                p.enqueue(alt[i % 2].clone(), t);
            }
            assert_eq!(p.queued(), 8);
        }

        #[test]
        fn os_binary_detection() {
            assert!(is_os_audio_binary("/usr/bin/afplay"));
            assert!(is_os_audio_binary("aplay"));
            assert!(is_os_audio_binary("/bin/paplay/"));
            assert!(!is_os_audio_binary("true"));
            assert!(!is_os_audio_binary(""));
        }

        #[test]
        fn os_playback_wav_has_tail_pad() {
            let a = crate::synthesize_wav_with_volume(&SOUND_PHASER, 80);
            let b = synthesize_os_playback_wav(&SOUND_PHASER, 80);
            assert_eq!(b.len(), a.len() + 480 * 2);
            assert_eq!(&b[44..a.len()], &a[44..]);
            assert!(b[a.len()..].iter().all(|&x| x == 0));
        }

        #[test]
        fn wav_file_cache_reuses_path() {
            let (p1, rm) = get_or_write_wav_file(&SOUND_COMPUTER_BEEP, 37).unwrap();
            assert!(!rm);
            let (p2, _) = get_or_write_wav_file(&SOUND_COMPUTER_BEEP, 37).unwrap();
            assert_eq!(p1, p2);
            let name = p1.file_name().unwrap().to_string_lossy().into_owned();
            assert!(name.starts_with("sst-audio-") && name.ends_with(".wav"));
            assert_eq!(
                std::fs::read(&p1).unwrap(),
                synthesize_os_playback_wav(&SOUND_COMPUTER_BEEP, 37)
            );
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mode = std::fs::metadata(&p1).unwrap().permissions().mode() & 0o777;
                assert_eq!(mode, 0o600);
            }
            assert_eq!(p1.parent().unwrap(), go_temp_dir().as_path());
        }
    }
}
