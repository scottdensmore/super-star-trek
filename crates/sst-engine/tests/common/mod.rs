//! Shared helpers for the Go parity tests (data under testdata/parity/engine,
//! produced by tools/paritygen/engine).
#![allow(dead_code)]

use sst_engine::json;
use sst_engine::{GameState, PRNG};

/// Reads a transcript; `SST_PARITY_DIR` overrides the directory (e.g. for
/// full-detail transcripts generated with `SST_PARITY_FULL=1`).
pub fn data(name: &str) -> String {
    let dir = std::env::var("SST_PARITY_DIR").unwrap_or_else(|_| {
        format!(
            "{}/../../testdata/parity/engine",
            env!("CARGO_MANIFEST_DIR")
        )
    });
    let path = format!("{dir}/{name}");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"))
}

pub fn hex_decode(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

pub fn hex_str(s: &str) -> String {
    String::from_utf8(hex_decode(s)).expect("utf-8")
}

pub fn fnv_hex(b: &[u8]) -> String {
    format!("{:016x}", json::fnv1a64(b))
}

pub fn rng_state(p: &Option<PRNG>) -> String {
    match p {
        None => "nil".into(),
        Some(p) => match p.state() {
            None => "nilsrc".into(),
            Some((hi, lo)) => format!("{:016x}{:016x}", hi, lo),
        },
    }
}

pub fn to_s(b: Vec<u8>) -> String {
    String::from_utf8(b).unwrap()
}

pub fn marshal<T: json::GoJson + ?Sized>(v: &T) -> String {
    match json::marshal(v) {
        Ok(b) => to_s(b),
        Err(e) => format!("ERR {}", e),
    }
}

/// The state's JSON as the Go edition wrote it: without the legacy Klingon
/// list, which saves carry since format 3 and [`state_parts`] hashes
/// separately anyway.
pub fn marshal_go_state(g: &GameState) -> String {
    let mut g = g.clone();
    g.current_quad.klingons = sst_engine::json::GoVec::nil();
    marshal(&g)
}

/// [`marshal_go_state`] for a tour: its current game without the Klingon list.
pub fn marshal_go_tour(t: &sst_engine::TourRef) -> String {
    let mut tc = t.borrow().clone();
    if let Some(g) = &tc.current_game_state {
        let mut gc = g.borrow().clone();
        gc.current_quad.klingons = sst_engine::json::GoVec::nil();
        tc.current_game_state = Some(sst_engine::game_ref(gc));
    }
    marshal(&tc)
}

pub fn state_parts(g: &GameState) -> (String, String, String) {
    (
        marshal_go_state(g),
        marshal(&g.current_quad.klingons),
        rng_state(&g.rng),
    )
}

pub fn state_hash(g: &GameState) -> String {
    let (s, k, r) = state_parts(g);
    fnv_hex(format!("{s}|{k}|{r}").as_bytes())
}

/// Reports the first difference between two strings with context.
pub fn diff(label: &str, want: &str, got: &str) -> String {
    let pos = want
        .bytes()
        .zip(got.bytes())
        .position(|(a, b)| a != b)
        .unwrap_or(want.len().min(got.len()));
    let lo = pos.saturating_sub(80);
    format!(
        "{label}: mismatch at byte {pos}\n want: ...{}\n  got: ...{}",
        &want[lo..(pos + 80).min(want.len())],
        &got[lo..(pos + 80).min(got.len())]
    )
}

#[track_caller]
pub fn assert_same(label: &str, want: &str, got: &str) {
    if want != got {
        panic!("{}", diff(label, want, got));
    }
}

/// Line cursor over a transcript.
///
/// Record mode (`SST_PARITY_RECORD=<path>`) re-records a transcript after an
/// intentional behaviour change: the harness runs the recorded inputs as
/// usual, but instead of comparing, every checked line is written with the
/// value the engine now produces (original lines with the checked prefix are
/// consumed, so cases whose behaviour did not change come out byte for byte
/// identical). Input lines are copied. Harnesses whose transcripts change
/// shape call [`Lines::inputs`] to name their input prefixes, so stale output
/// lines that the new run no longer produces are dropped. The result is
/// written to `<path>` when the cursor is dropped.
pub struct Lines<'a> {
    pub lines: Vec<&'a str>,
    pub pos: usize,
    pub bytes_compared: usize,
    pub rec: Option<(String, Vec<String>)>,
    inputs: Option<&'static [&'static str]>,
}

impl Drop for Lines<'_> {
    fn drop(&mut self) {
        if let Some((path, out)) = self.rec.take() {
            let mut text = out.join("\n");
            text.push('\n');
            std::fs::write(&path, text).unwrap_or_else(|e| panic!("write {path}: {e}"));
            eprintln!("recorded {} lines to {path}", out.len());
        }
    }
}

impl<'a> Lines<'a> {
    pub fn new(s: &'a str) -> Self {
        Lines {
            lines: s.lines().collect(),
            pos: 0,
            bytes_compared: 0,
            rec: std::env::var("SST_PARITY_RECORD")
                .ok()
                .filter(|p| !p.is_empty())
                .map(|p| (p, Vec::new())),
            inputs: None,
        }
    }

    /// Names the prefixes of input lines (record mode only skips other lines
    /// when it reads an input).
    pub fn inputs(mut self, prefixes: &'static [&'static str]) -> Self {
        self.inputs = Some(prefixes);
        self
    }

    pub fn recording(&self) -> bool {
        self.rec.is_some()
    }

    /// Appends a line to the recording.
    pub fn record(&mut self, line: String) {
        if let Some((_, out)) = &mut self.rec {
            out.push(line);
        }
    }

    pub fn peek(&self) -> Option<&'a str> {
        self.lines.get(self.pos).copied()
    }

    pub fn next(&mut self) -> Option<&'a str> {
        if self.rec.is_some() {
            if let Some(inputs) = self.inputs {
                while let Some(l) = self.peek() {
                    if inputs.iter().any(|p| l.starts_with(p)) {
                        break;
                    }
                    self.pos += 1;
                }
            }
            let l = self.peek();
            self.pos += 1;
            if let Some(l) = l {
                self.record(l.to_string());
            }
            return l;
        }
        let l = self.peek();
        self.pos += 1;
        l
    }

    /// Un-reads the line just returned by [`Lines::next`].
    pub fn unread(&mut self) {
        self.pos -= 1;
        if let Some((_, out)) = &mut self.rec {
            out.pop();
        }
    }

    /// Next line, which must start with `prefix` (returns the remainder).
    /// In record mode a non-matching line is left in place (an inserted
    /// line) and "" is returned.
    /// In record mode the consumed line is an input and is copied.
    #[track_caller]
    pub fn expect(&mut self, prefix: &str) -> &'a str {
        if self.rec.is_some() {
            let before = self.pos;
            let r = self.consume(prefix);
            if self.pos > before {
                self.record(self.lines[before].to_string());
            }
            return r;
        }
        self.consume(prefix)
    }

    /// [`Lines::expect`] without recording.
    #[track_caller]
    fn consume(&mut self, prefix: &str) -> &'a str {
        if self.rec.is_some() {
            return match self.peek().and_then(|l| l.strip_prefix(prefix)) {
                Some(r) => {
                    self.pos += 1;
                    r
                }
                None => "",
            };
        }
        let l = self
            .next()
            .unwrap_or_else(|| panic!("unexpected end, wanted {prefix:?}"));
        match l.strip_prefix(prefix) {
            Some(r) => r,
            None => panic!(
                "line {}: wanted prefix {:?}, got {:?}",
                self.pos,
                prefix,
                &l[..l.len().min(120)]
            ),
        }
    }

    /// Compares an expected line `prefix + got` (records it in record mode).
    #[track_caller]
    pub fn check(&mut self, prefix: &str, got: &str) {
        let want = self.consume(prefix);
        if self.rec.is_some() {
            self.record(format!("{prefix}{got}"));
            return;
        }
        self.bytes_compared += want.len();
        if want != got {
            panic!(
                "{}",
                diff(&format!("line {} ({prefix})", self.pos), want, got)
            );
        }
    }

    /// Compares S/K/R full-state lines.
    #[track_caller]
    pub fn check_full(&mut self, g: &GameState) {
        let (s, k, r) = state_parts(g);
        self.check("S ", &s);
        self.check("K ", &k);
        self.check("R ", &r);
    }
}
