//! Shared helpers for the parity tests: loading the record files written by
//! `tools/paritygen/tuikit` (see writer.go there for the format).

#![allow(dead_code)]

use std::collections::BTreeMap;

/// One record: field name -> raw bytes.
#[derive(Debug, Default)]
pub struct Record(pub BTreeMap<String, Vec<u8>>);

impl Record {
    /// A field as UTF-8 text (`None` if missing).
    pub fn get(&self, k: &str) -> Option<&str> {
        self.0
            .get(k)
            .map(|v| std::str::from_utf8(v).unwrap_or_else(|_| panic!("field {k} is not UTF-8")))
    }
    /// A field as raw bytes.
    pub fn bytes(&self, k: &str) -> &[u8] {
        self.0
            .get(k)
            .map(|v| v.as_slice())
            .unwrap_or_else(|| panic!("missing field {k}"))
    }
    /// All UTF-8 fields.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &str)> {
        self.0
            .iter()
            .filter_map(|(k, v)| std::str::from_utf8(v).ok().map(|s| (k, s)))
    }
}

pub fn path(name: &str) -> String {
    format!(
        "{}/../../testdata/parity/tuikit/{}",
        env!("CARGO_MANIFEST_DIR"),
        name
    )
}

/// Parses `@key len\n<bytes>\n ... @@\n` records.
pub fn load(name: &str) -> Vec<Record> {
    let data = std::fs::read(path(name)).unwrap_or_else(|e| panic!("reading {name}: {e}"));
    let mut out = Vec::new();
    let mut cur = Record::default();
    let mut i = 0;
    while i < data.len() {
        let nl = data[i..].iter().position(|&b| b == b'\n').expect("newline") + i;
        let header = std::str::from_utf8(&data[i..nl]).expect("header utf8");
        i = nl + 1;
        if header == "@@" {
            out.push(std::mem::take(&mut cur));
            continue;
        }
        let header = header.strip_prefix('@').expect("field header");
        let (key, len) = header.rsplit_once(' ').expect("key len");
        let len: usize = len.parse().expect("len");
        let val = data[i..i + len].to_vec();
        i += len + 1; // value + '\n'
        cur.0.insert(key.to_string(), val);
    }
    out
}

pub fn get<'a>(r: &'a Record, k: &str) -> &'a str {
    r.get(k)
        .unwrap_or_else(|| panic!("missing field {k} in {r:?}"))
}

/// Accumulates mismatches so a test reports many failures at once.
#[derive(Default)]
pub struct Checker {
    pub cases: usize,
    pub bytes: usize,
    pub failures: Vec<String>,
}

impl Checker {
    pub fn check(&mut self, what: impl FnOnce() -> String, got: &str, want: &str) {
        self.cases += 1;
        self.bytes += want.len();
        if got != want && self.failures.len() < 30 {
            self.failures
                .push(format!("{}\n  got:  {:?}\n  want: {:?}", what(), got, want));
        } else if got != want {
            self.failures.push(String::new());
        }
    }
    pub fn finish(&self, name: &str) {
        eprintln!(
            "{name}: {} cases, {} bytes compared",
            self.cases, self.bytes
        );
        if !self.failures.is_empty() {
            let shown: Vec<&String> = self.failures.iter().filter(|s| !s.is_empty()).collect();
            panic!(
                "{name}: {} mismatches of {} cases:\n{}",
                self.failures.len(),
                self.cases,
                shown
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join("\n")
            );
        }
    }
}

/// Go `strconv.Quote` (`%q`) for the strings used in the records.
pub fn go_quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '\x07' => out.push_str("\\a"),
            '\x08' => out.push_str("\\b"),
            '\x0c' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\x0b' => out.push_str("\\v"),
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                out.push_str(&format!("\\x{:02x}", c as u32))
            }
            c if c.is_control() || c == '\u{FFFD}' && false => {
                if (c as u32) < 0x10000 {
                    out.push_str(&format!("\\u{:04x}", c as u32))
                } else {
                    out.push_str(&format!("\\U{:08x}", c as u32))
                }
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
