//! `bubbles/runeutil`: input sanitising.

/// `runeutil.Sanitizer` (with `ReplaceTabs`/`ReplaceNewlines` options).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sanitizer {
    replace_new_line: Vec<char>,
    replace_tab: Vec<char>,
}

impl Default for Sanitizer {
    fn default() -> Self {
        Sanitizer {
            replace_new_line: vec!['\n'],
            replace_tab: "    ".chars().collect(),
        }
    }
}

impl Sanitizer {
    /// `NewSanitizer(ReplaceTabs(tab), ReplaceNewlines(nl))`.
    pub fn new(tab: &str, nl: &str) -> Self {
        Sanitizer {
            replace_new_line: nl.chars().collect(),
            replace_tab: tab.chars().collect(),
        }
    }

    /// `Sanitize`: drops U+FFFD and control characters, replaces CR/LF and
    /// tabs.
    pub fn sanitize(&self, runes: &[char]) -> Vec<char> {
        let mut dst = Vec::with_capacity(runes.len());
        for &r in runes {
            match r {
                '\u{FFFD}' => {}
                '\r' | '\n' => dst.extend_from_slice(&self.replace_new_line),
                '\t' => dst.extend_from_slice(&self.replace_tab),
                c if is_go_control(c) => {}
                c => dst.push(c),
            }
        }
        dst
    }
}

/// Go `unicode.IsControl` (C0, DEL and C1).
pub(crate) fn is_go_control(c: char) -> bool {
    let u = c as u32;
    u < 0x20 || (0x7f..=0x9f).contains(&u)
}
