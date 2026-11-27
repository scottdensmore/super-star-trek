//! TUI transcript parity: replays the scripted sessions recorded from Go by
//! `pkg/tui/zz_paritygen_test.go` (`testdata/parity/tui/*.txt`) and compares
//! the FNV-1a 64 hash of `View()` (and the full frame where stored), the
//! number of pending timers and the quit flag after every step.
//!
//! The driver mirrors the Go generator: immediate commands run synchronously
//! (depth-first, batches in order), timers stay pending until a `tick` step
//! fires all of them in creation order, and the clock is fixed at the
//! recorded instant `T` (double clicks, scenario seeds, record dates).

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sst_engine::gotime::{FixedClock, GoTime};
use sst_engine::{
    default_rules_for_profile, game_ref, get_scenario, persist, DifficultyProfile, EventGameOver,
    GameLength, GameOverReason, GameState, ScenarioID, SkillLevel,
};
use sst_tui::components::commandbar::CommandSubmittedMsg;
use sst_tui::theme::{self, ColorMode};
use sst_tui::Model;
use sst_tuikit::lipgloss;
use sst_tuikit::tea::{
    self, BatchMsg, Cmd, KeyMsg, KeyType, MouseAction, MouseButton, MouseMsg, Msg, QuitMsg,
    SequenceMsg, Tick, WindowSizeMsg, MOUSE_ACTION_PRESS, MOUSE_BUTTON_LEFT,
};
use sst_tuikit::termenv;

fn fnv1a64(b: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &c in b {
        h ^= c as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Decodes a Go `%q` string literal.
fn go_unquote(s: &str) -> String {
    let b: Vec<char> = s.chars().collect();
    assert!(
        b.len() >= 2 && b[0] == '"' && b[b.len() - 1] == '"',
        "bad quoted {s}"
    );
    let mut out = String::new();
    let mut bytes: Vec<u8> = Vec::new();
    let flush = |bytes: &mut Vec<u8>, out: &mut String| {
        if !bytes.is_empty() {
            out.push_str(&String::from_utf8_lossy(bytes));
            bytes.clear();
        }
    };
    let mut i = 1;
    let end = b.len() - 1;
    let hex = |cs: &[char]| u32::from_str_radix(&cs.iter().collect::<String>(), 16).unwrap();
    while i < end {
        let c = b[i];
        if c != '\\' {
            flush(&mut bytes, &mut out);
            out.push(c);
            i += 1;
            continue;
        }
        i += 1;
        let e = b[i];
        i += 1;
        match e {
            'a' => out.push('\x07'),
            'b' => out.push('\x08'),
            'f' => out.push('\x0c'),
            'n' => out.push('\n'),
            'r' => out.push('\r'),
            't' => out.push('\t'),
            'v' => out.push('\x0b'),
            '\\' => out.push('\\'),
            '"' => out.push('"'),
            '\'' => out.push('\''),
            'x' => {
                bytes.push(hex(&b[i..i + 2]) as u8);
                i += 2;
                continue;
            }
            'u' => {
                out.push(char::from_u32(hex(&b[i..i + 4])).unwrap());
                i += 4;
            }
            'U' => {
                out.push(char::from_u32(hex(&b[i..i + 8])).unwrap());
                i += 8;
            }
            '0'..='7' => {
                let v =
                    u32::from_str_radix(&b[i - 1..i + 2].iter().collect::<String>(), 8).unwrap();
                bytes.push(v as u8);
                i += 2;
                continue;
            }
            other => panic!("bad escape \\{other} in {s}"),
        }
        flush(&mut bytes, &mut out);
    }
    flush(&mut bytes, &mut out);
    out
}

/// Splits a step line into tokens, keeping `"..."` literals whole.
fn tokens(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let cs: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < cs.len() {
        if cs[i] == ' ' {
            i += 1;
            continue;
        }
        let start = i;
        if cs[i] == '"' {
            i += 1;
            while i < cs.len() && cs[i] != '"' {
                if cs[i] == '\\' {
                    i += 1;
                }
                i += 1;
            }
            i += 1;
        } else {
            while i < cs.len() && cs[i] != ' ' {
                i += 1;
            }
        }
        out.push(cs[start..i].iter().collect());
    }
    out
}

struct Expect {
    line: String,
    hash: String,
    pend: usize,
    quit: bool,
    frame: Option<Vec<u8>>,
}

struct Session {
    name: String,
    profile: String,
    setup: String,
    steps: Vec<Expect>,
    /// Byte range of the session in its file (for re-recording).
    span: (usize, usize),
}

struct Reader<'a> {
    d: &'a [u8],
    p: usize,
}

impl Reader<'_> {
    fn line(&mut self) -> Option<String> {
        if self.p >= self.d.len() {
            return None;
        }
        let rest = &self.d[self.p..];
        let n = rest.iter().position(|&c| c == b'\n').unwrap_or(rest.len());
        let l = String::from_utf8(rest[..n].to_vec()).unwrap();
        self.p += n + 1;
        Some(l)
    }
    fn peek_starts(&self, pre: &str) -> bool {
        self.d[self.p..].starts_with(pre.as_bytes())
    }
    fn bytes(&mut self, n: usize) -> Vec<u8> {
        let v = self.d[self.p..self.p + n].to_vec();
        self.p += n + 1; // trailing '\n'
        v
    }
}

fn parse_file(data: &[u8]) -> (String, i64, Vec<Session>) {
    let mut r = Reader { d: data, p: 0 };
    let mut date = String::new();
    let mut t = 0i64;
    let mut sessions = Vec::new();
    while let Some(l) = r.line() {
        if let Some(v) = l.strip_prefix("date ") {
            date = v.to_string();
        } else if let Some(v) = l.strip_prefix("clock_unix_nano ") {
            t = v.parse().unwrap();
        } else if let Some(v) = l.strip_prefix("session ") {
            let start = r.p - l.len() - 1;
            let (name, profile) = v.rsplit_once(' ').unwrap();
            let setup = r.line().unwrap();
            let setup = setup.strip_prefix("setup ").unwrap().to_string();
            let mut s = Session {
                name: name.into(),
                profile: profile.into(),
                setup,
                steps: Vec::new(),
                span: (start, 0),
            };
            loop {
                let l = r.line().unwrap();
                if l == "end" {
                    break;
                }
                let step = l.strip_prefix("step ").expect("step line").to_string();
                let eq = r.line().unwrap();
                let parts: Vec<&str> = eq.split(' ').collect();
                assert_eq!(parts[0], "=");
                let hash = parts[1].to_string();
                let pend = parts[2].strip_prefix("pend=").unwrap().parse().unwrap();
                let quit = parts[3] == "quit=1";
                let mut frame = None;
                if r.peek_starts("frame ") {
                    let fl = r.line().unwrap();
                    let n: usize = fl.strip_prefix("frame ").unwrap().parse().unwrap();
                    frame = Some(r.bytes(n));
                }
                s.steps.push(Expect {
                    line: step,
                    hash,
                    pend,
                    quit,
                    frame,
                });
            }
            s.span.1 = r.p;
            sessions.push(s);
        } else if !l.is_empty() && !l.starts_with('#') {
            panic!("unexpected line {l:?}");
        }
    }
    (date, t, sessions)
}

struct Driver {
    m: Model,
    pending: Vec<Tick>,
    queue: VecDeque<Msg>,
    quit: bool,
    now: SystemTime,
}

impl Driver {
    fn expand(&mut self, c: Option<Cmd>) {
        let Some(c) = c else { return };
        match c {
            Cmd::Func(f) => {
                if let Some(m) = f() {
                    self.expand_msg(m);
                }
            }
            Cmd::Tick(t) => self.pending.push(t),
            Cmd::Batch(v) | Cmd::Sequence(v) => {
                for c in v {
                    self.expand(Some(c));
                }
            }
        }
    }

    fn expand_msg(&mut self, m: Msg) {
        let m = match m.downcast::<BatchMsg>() {
            Ok(b) => {
                for c in b.0 {
                    self.expand(Some(c));
                }
                return;
            }
            Err(m) => m,
        };
        let m = match m.downcast::<SequenceMsg>() {
            Ok(s) => {
                for c in s.0 {
                    self.expand(Some(c));
                }
                return;
            }
            Err(m) => m,
        };
        self.queue.push_back(m);
    }

    fn process(&mut self) {
        while let Some(msg) = self.queue.pop_front() {
            if msg.downcast_ref::<QuitMsg>().is_some() {
                self.quit = true;
                continue;
            }
            let c = self.m.update_in_place(msg);
            self.expand(c);
        }
    }

    fn send(&mut self, m: Msg) {
        self.queue.push_back(m);
        self.process();
    }

    fn left_press(&mut self, x: i32, y: i32) {
        self.send(tea::msg(MouseMsg {
            x,
            y,
            button: MOUSE_BUTTON_LEFT,
            action: MOUSE_ACTION_PRESS,
            ..Default::default()
        }));
    }

    fn tick(&mut self) {
        let timers: Vec<Tick> = std::mem::take(&mut self.pending);
        for t in timers {
            let r = if t.is_cancelled() {
                t.cancelled_msg()
            } else {
                t.fire(self.now)
            };
            if let Some(m) = r {
                self.queue.push_back(m);
            }
        }
        self.process();
    }
}

fn kv(setup: &str) -> std::collections::BTreeMap<String, String> {
    setup
        .split(' ')
        .filter_map(|p| p.split_once('='))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn build_model(setup: &str, clock_t: GoTime) -> Model {
    let kv = kv(setup);
    let get = |k: &str| kv.get(k).cloned().unwrap_or_default();
    let mode = match get("mode").as_str() {
        "light" => ColorMode::Light,
        "dark" => ColorMode::Dark,
        "auto" => ColorMode::Auto,
        _ => ColorMode::Unset,
    };
    let th = theme::get_theme(&get("theme")).with_color_mode(mode);
    let seed: i64 = get("seed").parse().unwrap_or(0);
    let mut m = match get("kind").as_str() {
        "game" => {
            let mut rules = default_rules_for_profile(&DifficultyProfile::new(get("profile")));
            if get("adv") == "1" {
                rules.adversaries = true;
                rules.profile = DifficultyProfile::custom();
            }
            if get("anom") == "1" {
                rules.spatial_anomalies = true;
                rules.profile = DifficultyProfile::custom();
            }
            if get("cloak") == "1" {
                rules.klingon_cloak = true;
                rules.profile = DifficultyProfile::custom();
            }
            if let Some(a) = kv.get("anim") {
                rules.anim_speed = a.parse().unwrap();
            }
            let mut g =
                GameState::new_game_with_options(seed, SkillLevel::Good, GameLength::Medium, rules);
            let (q, s) = (g.enterprise.quad, g.enterprise.sector);
            g.populate_quadrant(q, s);
            sst_tui::new_model(Some(game_ref(g)), th)
        }
        "scenario" => {
            let sc = get_scenario(&ScenarioID::new(get("id"))).expect("scenario");
            let g = sc.build(seed);
            sst_tui::new_model(Some(game_ref(g)), th)
        }
        "tour" => {
            let tour = persist::new_tour(seed);
            sst_tui::new_model_with_tour(Some(Rc::new(std::cell::RefCell::new(tour))), th)
        }
        "newgame" => {
            let g = GameState::new_game(seed, SkillLevel::Good, GameLength::Medium);
            sst_tui::new_model(Some(game_ref(g)), th)
        }
        "nil" => sst_tui::new_model(None, th),
        k => panic!("unknown kind {k}"),
    };
    m.set_clock(Rc::new(FixedClock(clock_t)));
    if kv.contains_key("sound") {
        m.set_sound_enabled(get("sound") == "1");
    }
    if let Some(v) = kv.get("volume") {
        if let Some(p) = &m.audio_player {
            p.set_volume(v.parse().unwrap());
        }
    }
    m
}

fn touch_files(work: &Path, t: SystemTime) {
    fn walk(p: &Path, out: &mut Vec<PathBuf>) {
        let Ok(rd) = std::fs::read_dir(p) else { return };
        for e in rd.flatten() {
            let ft = e.file_type().unwrap();
            if ft.is_dir() {
                walk(&e.path(), out);
            } else if ft.is_file() {
                out.push(e.path());
            }
        }
    }
    let mut files = Vec::new();
    walk(work, &mut files);
    files.sort_by(|a, b| {
        a.as_os_str()
            .as_encoded_bytes()
            .cmp(b.as_os_str().as_encoded_bytes())
    });
    for (i, p) in files.iter().enumerate() {
        let mt = t - Duration::from_secs(60 * i as u64);
        if let Ok(f) = std::fs::File::options().write(true).open(p) {
            let _ = f.set_times(std::fs::FileTimes::new().set_accessed(mt).set_modified(mt));
        }
    }
}

struct Stats {
    sessions: usize,
    steps: usize,
    frames: usize,
    bytes: usize,
    mismatches: Vec<String>,
}

fn run_step(d: &mut Driver, step: &str) {
    let tk = tokens(step);
    // tk[0] is the step index.
    let op = tk[1].as_str();
    let i = |n: usize| -> i32 { tk[n].parse().unwrap() };
    match op {
        "init" => {
            let c = d.m.init();
            d.expand(c);
            d.process();
        }
        "size" => d.send(tea::msg(WindowSizeMsg {
            width: i(2),
            height: i(3),
        })),
        "key" => {
            let mut k = KeyMsg::new(KeyType(i(2)));
            k.runes = go_unquote(&tk[3]).chars().collect();
            k.alt = tk[4] == "1";
            d.send(tea::msg(k));
        }
        "mouse" => d.send(tea::msg(MouseMsg {
            x: i(2),
            y: i(3),
            button: MouseButton(i(4)),
            action: MouseAction(i(5)),
            ..Default::default()
        })),
        "click" => {
            d.m.last_click_time = GoTime::default();
            d.left_press(i(2), i(3));
        }
        "dblclick" => {
            d.m.last_click_time = GoTime::default();
            d.left_press(i(2), i(3));
            d.left_press(i(2), i(3));
        }
        "submit" => d.send(tea::msg(CommandSubmittedMsg {
            text: go_unquote(&tk[2]),
        })),
        "gameover" => d.send(tea::msg(EventGameOver {
            reason: GameOverReason(i(2)),
            score: tk[3].parse().unwrap(),
        })),
        "tick" => d.tick(),
        other => panic!("unknown op {other} in {step:?}"),
    }
}

fn first_diff(a: &[u8], b: &[u8]) -> String {
    let al: Vec<&[u8]> = a.split(|&c| c == b'\n').collect();
    let bl: Vec<&[u8]> = b.split(|&c| c == b'\n').collect();
    for i in 0..al.len().max(bl.len()) {
        let x = al.get(i).copied().unwrap_or(b"");
        let y = bl.get(i).copied().unwrap_or(b"");
        if x != y {
            return format!(
                "line {}:\n  want: {:?}\n   got: {:?}",
                i + 1,
                String::from_utf8_lossy(x),
                String::from_utf8_lossy(y)
            );
        }
    }
    "(no line diff)".into()
}

/// Runs a session. In record mode every step runs (no early stop) and the
/// session's transcript text with the values now produced is returned when
/// any step differs.
fn run_session(
    s: &Session,
    t_nanos: i64,
    root: &Path,
    idx: usize,
    st: &mut Stats,
    record: bool,
) -> Option<Vec<u8>> {
    let dir = root.join(format!("s{idx}"));
    let home = dir.join("home");
    let work = dir.join("work");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&work).unwrap();
    std::env::set_var("HOME", &home);
    std::env::set_var("XDG_CONFIG_HOME", home.join(".config"));
    std::env::set_current_dir(&work).unwrap();
    theme::detect::reset_state();
    theme::detect::set_fixed_detection(true);
    lipgloss::set_has_dark_background(true);
    lipgloss::set_color_profile(if s.profile == "ascii" {
        termenv::Ascii
    } else {
        termenv::TrueColor
    });

    if let Some(n) = kv(&s.setup).get("saves") {
        for i in 1..=n.parse::<i64>().unwrap() {
            let g = game_ref(GameState::new_game(
                1000 + i,
                SkillLevel::Good,
                GameLength::Medium,
            ));
            let path = work.join(format!("MISSION{i}.TRK"));
            persist::save_game(&g, path.to_str().unwrap()).unwrap();
        }
    }

    let secs = t_nanos.div_euclid(1_000_000_000);
    let nsec = t_nanos.rem_euclid(1_000_000_000) as u32;
    let clock_t = GoTime::from_unix(secs, nsec, 0);
    let now = UNIX_EPOCH + Duration::new(secs as u64, nsec);
    let mut d = Driver {
        m: build_model(&s.setup, clock_t),
        pending: Vec::new(),
        queue: VecDeque::new(),
        quit: false,
        now,
    };
    st.sessions += 1;
    let mut rec = format!("session {} {}\nsetup {}\n", s.name, s.profile, s.setup).into_bytes();
    let mut changed = false;
    for e in &s.steps {
        run_step(&mut d, &e.line);
        touch_files(&work, now);
        let view = d.m.view();
        st.steps += 1;
        let h = format!("{:016x}", fnv1a64(view.as_bytes()));
        let mut problems = Vec::new();
        if h != e.hash {
            problems.push(format!("hash {h} != {}", e.hash));
        }
        if d.pending.len() != e.pend {
            problems.push(format!("pend {} != {}", d.pending.len(), e.pend));
        }
        if d.quit != e.quit {
            problems.push(format!("quit {} != {}", d.quit, e.quit));
        }
        if let Some(f) = &e.frame {
            st.frames += 1;
            st.bytes += f.len();
            if f.as_slice() != view.as_bytes() {
                problems.push(first_diff(f, view.as_bytes()));
            }
        }
        if record {
            rec.extend_from_slice(
                format!(
                    "step {}\n= {h} pend={} quit={}\n",
                    e.line,
                    d.pending.len(),
                    if d.quit { 1 } else { 0 }
                )
                .as_bytes(),
            );
            if e.frame.is_some() {
                rec.extend_from_slice(format!("frame {}\n", view.len()).as_bytes());
                rec.extend_from_slice(view.as_bytes());
                rec.push(b'\n');
            }
            changed |= !problems.is_empty();
            continue;
        }
        if !problems.is_empty() {
            st.mismatches.push(format!(
                "{} [{}] step {:?}: {}",
                s.name,
                s.profile,
                e.line,
                problems.join("; ")
            ));
            break;
        }
    }
    let _ = std::env::set_current_dir(root);
    let _ = std::fs::remove_dir_all(&dir);
    rec.extend_from_slice(b"end\n");
    changed.then_some(rec)
}

#[test]
fn tui_transcript_parity() {
    sst_audio::set_test_mode(true);
    std::env::set_var("TZ", "UTC");
    std::env::set_var("COLORFGBG", "15;0");
    std::env::remove_var("TERM_PROGRAM");
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../testdata/parity/tui");
    let root = std::env::temp_dir().join(format!("sst-tui-parity-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let filter = std::env::var("SST_PARITY_FILTER").unwrap_or_default();
    // SST_PARITY_RECORD=<dir> re-records the sessions whose behaviour changed
    // (after an intentional change) and writes each transcript to <dir>;
    // unchanged sessions are copied byte for byte.
    let record_dir = std::env::var("SST_PARITY_RECORD")
        .ok()
        .filter(|d| !d.is_empty());

    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "txt"))
        .collect();
    files.sort();
    assert!(!files.is_empty(), "no parity files in {dir}");

    let mut st = Stats {
        sessions: 0,
        steps: 0,
        frames: 0,
        bytes: 0,
        mismatches: Vec::new(),
    };
    let mut idx = 0;
    for f in &files {
        let data = std::fs::read(f).unwrap();
        let (_date, t, sessions) = parse_file(&data);
        let mut out = Vec::new();
        let mut copied = 0;
        for s in &sessions {
            idx += 1;
            if !filter.is_empty() && !s.name.contains(&filter) {
                continue;
            }
            let rec = run_session(s, t, &root, idx, &mut st, record_dir.is_some());
            if let Some(rec) = rec {
                out.extend_from_slice(&data[copied..s.span.0]);
                out.extend_from_slice(&rec);
                copied = s.span.1;
                eprintln!("re-recorded {} [{}]", s.name, s.profile);
            }
        }
        if let Some(rd) = &record_dir {
            out.extend_from_slice(&data[copied..]);
            let dest = Path::new(rd).join(f.file_name().unwrap());
            std::fs::write(&dest, &out).unwrap();
        }
    }
    let _ = std::fs::remove_dir_all(&root);
    eprintln!(
        "tui parity: {} sessions, {} steps, {} full frames ({} bytes) compared, {} mismatching sessions",
        st.sessions,
        st.steps,
        st.frames,
        st.bytes,
        st.mismatches.len()
    );
    if !st.mismatches.is_empty() {
        let shown: Vec<&String> = st.mismatches.iter().take(15).collect();
        panic!(
            "{} sessions diverge:\n{}",
            st.mismatches.len(),
            shown
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}
