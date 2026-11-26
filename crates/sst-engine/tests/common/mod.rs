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

pub fn state_parts(g: &GameState) -> (String, String, String) {
    (
        marshal(g),
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
pub struct Lines<'a> {
    pub lines: Vec<&'a str>,
    pub pos: usize,
    pub bytes_compared: usize,
}

impl<'a> Lines<'a> {
    pub fn new(s: &'a str) -> Self {
        Lines {
            lines: s.lines().collect(),
            pos: 0,
            bytes_compared: 0,
        }
    }

    pub fn peek(&self) -> Option<&'a str> {
        self.lines.get(self.pos).copied()
    }

    pub fn next(&mut self) -> Option<&'a str> {
        let l = self.peek();
        self.pos += 1;
        l
    }

    /// Next line, which must start with `prefix` (returns the remainder).
    #[track_caller]
    pub fn expect(&mut self, prefix: &str) -> &'a str {
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

    /// Compares an expected line `prefix + got`.
    #[track_caller]
    pub fn check(&mut self, prefix: &str, got: &str) {
        let want = self.expect(prefix);
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
