//! Port of `github.com/sahilm/fuzzy` v0.1.1 (Sublime-style fuzzy matching
//! used by `list.DefaultFilter`). `matched_indexes` are byte offsets, as in
//! Go.

use crate::gosort;
use crate::unicode::{go_is_lower, go_is_upper};

/// `fuzzy.Match`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Match {
    pub str: String,
    pub index: i32,
    pub matched_indexes: Vec<i32>,
    pub score: i32,
}

const FIRST_CHAR_MATCH_BONUS: i32 = 10;
const MATCH_FOLLOWING_SEPARATOR_BONUS: i32 = 20;
const CAMEL_CASE_MATCH_BONUS: i32 = 20;
const ADJACENT_MATCH_BONUS: i32 = 5;
const UNMATCHED_LEADING_CHAR_PENALTY: i32 = -5;
const MAX_UNMATCHED_LEADING_CHAR_PENALTY: i32 = -15;

const SEPARATORS: &[char] = &['/', '-', '_', ' ', '.', '\\'];

/// `fuzzy.Find`: matches sorted best first (Go `sort.Stable` with the
/// non-strict `Score >=` ordering).
pub fn find(pattern: &str, data: &[String]) -> Vec<Match> {
    let mut m = find_no_sort(pattern, data);
    sort_matches(&mut m);
    m
}

/// `sort.Stable(Matches)`.
pub fn sort_matches(m: &mut [Match]) {
    gosort::stable(m, |a, b| a.score >= b.score);
}

/// Go `utf8.DecodeRuneInString` returning (rune, size); invalid -> U+FFFD.
fn decode(s: &str, i: usize) -> (char, usize) {
    match s.get(i..).and_then(|t| t.chars().next()) {
        Some(c) => (c, c.len_utf8()),
        None => ('\u{FFFD}', 1),
    }
}

/// `fuzzy.FindNoSort`.
pub fn find_no_sort(pattern: &str, data: &[String]) -> Vec<Match> {
    if pattern.is_empty() {
        return Vec::new();
    }
    let runes: Vec<char> = pattern.chars().collect();
    let mut matches = Vec::new();
    for (i, s) in data.iter().enumerate() {
        let mut m = Match {
            str: s.clone(),
            index: i as i32,
            matched_indexes: Vec::with_capacity(runes.len()),
            score: 0,
        };
        let mut pattern_index = 0usize;
        let mut best_score: i32 = -1;
        let mut matched_index: i32 = -1;
        let mut curr_adjacent_match_bonus = 0;
        let mut last: char = '\0';
        let mut last_index: i32 = 0;
        let (mut nextc, mut next_size) = if s.is_empty() {
            ('\u{FFFD}', 0)
        } else {
            decode(s, 0)
        };
        let bytes = s.as_bytes();
        let mut j = 0usize;
        while j < bytes.len() {
            let (candidate, candidate_size) = (nextc, next_size);
            if equal_fold(candidate, runes[pattern_index]) {
                let mut score = 0;
                if j == 0 {
                    score += FIRST_CHAR_MATCH_BONUS;
                }
                if go_is_lower(last) && go_is_upper(candidate) {
                    score += CAMEL_CASE_MATCH_BONUS;
                }
                if j != 0 && SEPARATORS.contains(&last) {
                    score += MATCH_FOLLOWING_SEPARATOR_BONUS;
                }
                if let Some(&last_match) = m.matched_indexes.last() {
                    let bonus =
                        adjacent_char_bonus(last_index, last_match, curr_adjacent_match_bonus);
                    score += bonus;
                    curr_adjacent_match_bonus += bonus;
                }
                if score > best_score {
                    best_score = score;
                    matched_index = j as i32;
                }
            }
            let nextp = if pattern_index < runes.len() - 1 {
                runes[pattern_index + 1]
            } else {
                '\0'
            };
            if j + candidate_size < bytes.len() {
                if bytes[j + candidate_size] < 0x80 {
                    nextc = bytes[j + candidate_size] as char;
                    next_size = 1;
                } else {
                    let d = decode(s, j + candidate_size);
                    nextc = d.0;
                    next_size = d.1;
                }
            } else {
                nextc = '\0';
                next_size = 0;
            }
            if (equal_fold(nextp, nextc) || nextc == '\0') && matched_index > -1 {
                if m.matched_indexes.is_empty() {
                    let penalty = matched_index * UNMATCHED_LEADING_CHAR_PENALTY;
                    best_score += penalty.max(MAX_UNMATCHED_LEADING_CHAR_PENALTY);
                }
                m.score += best_score;
                m.matched_indexes.push(matched_index);
                best_score = -1;
                pattern_index += 1;
            }
            last_index = j as i32;
            last = candidate;
            j += candidate_size.max(1);
        }
        let penalty = m.matched_indexes.len() as i32 - bytes.len() as i32;
        m.score += penalty;
        if m.matched_indexes.len() == runes.len() {
            matches.push(m);
        }
    }
    matches
}

/// `equalFold`: Unicode simple case folding equality (exact for ASCII;
/// single-rune upper/lower mappings elsewhere).
fn equal_fold(tr: char, sr: char) -> bool {
    if tr == sr {
        return true;
    }
    let (tr, sr) = if tr < sr { (sr, tr) } else { (tr, sr) };
    if tr.is_ascii() {
        return sr.is_ascii_uppercase() && tr as u32 == sr as u32 + ('a' as u32 - 'A' as u32);
    }
    let one = |it: &mut dyn Iterator<Item = char>| -> Option<char> {
        let c = it.next()?;
        if it.next().is_some() {
            None
        } else {
            Some(c)
        }
    };
    let lt = one(&mut tr.to_lowercase());
    let ls = one(&mut sr.to_lowercase());
    let ut = one(&mut tr.to_uppercase());
    let us = one(&mut sr.to_uppercase());
    (lt.is_some() && lt == ls) || (ut.is_some() && ut == us) || lt == Some(sr) || ls == Some(tr)
}

fn adjacent_char_bonus(i: i32, last_match: i32, current_bonus: i32) -> i32 {
    if last_match == i {
        return current_bonus * 2 + ADJACENT_MATCH_BONUS;
    }
    0
}
