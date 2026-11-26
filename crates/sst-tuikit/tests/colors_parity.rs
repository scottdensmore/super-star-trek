//! termenv colour degradation parity (`testdata/parity/tuikit/colors.txt`).
//!
//! The exhaustive #rrggbb -> ANSI256 mapping is stored as one FNV-1a hash per
//! block of 65536 colours. The default test checks 4 blocks (debug builds are
//! slow); `cargo test --release -p sst-tuikit -- --ignored` checks all 256.

mod common;

use common::{get, load, Checker};
use sst_tuikit::lipgloss;
use sst_tuikit::termenv::{self, Color, Profile};

fn fnv1a64(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn block(blk: usize) -> Vec<u8> {
    (0..65536usize)
        .map(|i| {
            let hex = format!("#{:06x}", (blk << 16) | i);
            match termenv::ANSI256.color(&hex) {
                Some(Color::ANSI256(v)) => v as u8,
                other => panic!("unexpected {other:?}"),
            }
        })
        .collect()
}

fn seq(c: Option<Color>, bg: bool) -> String {
    match c {
        None => "nil".into(),
        Some(c) => {
            let t = match c {
                Color::NoColor => "termenv.NoColor",
                Color::ANSI(_) => "termenv.ANSIColor",
                Color::ANSI256(_) => "termenv.ANSI256Color",
                Color::RGB(_) => "termenv.RGBColor",
            };
            format!("{t}:{}", c.sequence(bg))
        }
    }
}

fn profile(i: &str) -> Profile {
    match i {
        "0" => Profile::TrueColor,
        "1" => Profile::ANSI256,
        "2" => Profile::ANSI,
        _ => Profile::Ascii,
    }
}

fn check_blocks(all: bool) {
    let recs = load("colors.txt");
    let mut c = Checker::default();
    for r in &recs {
        if get(r, "op") != "block" {
            continue;
        }
        let blk: usize = get(r, "r").parse().unwrap();
        if !all && !blk.is_multiple_of(85) {
            continue;
        }
        let got = format!("{:016x}", fnv1a64(&block(blk)));
        c.check(|| format!("block {blk}"), &got, get(r, "ansi256"));
        c.bytes += 65536 - 16;
    }
    c.finish(if all {
        "colors_all_blocks"
    } else {
        "colors_blocks"
    });
}

#[test]
fn colors_blocks_sampled() {
    check_blocks(false);
}

#[test]
#[ignore]
fn colors_blocks_exhaustive() {
    check_blocks(true);
}

#[test]
fn colors_parity() {
    let recs = load("colors.txt");
    let mut c = Checker::default();
    for r in &recs {
        let op = get(r, "op");
        match op {
            "block" => {}
            "sample" => {
                let blk: usize = get(r, "r").parse().unwrap();
                let mut s = String::new();
                for i in (0..65536usize).step_by(97) {
                    let hex = format!("#{:06x}", (blk << 16) | i);
                    if let Some(Color::ANSI256(v)) = termenv::ANSI256.color(&hex) {
                        s.push_str(&format!("{v},"));
                    }
                }
                c.check(|| format!("sample {blk}"), &s, get(r, "ansi256"));
            }
            "ansi256to16" => {
                let mut s = String::new();
                for i in 0..256 {
                    s.push_str(&seq(termenv::ANSI.color(&i.to_string()), false));
                    s.push('\n');
                }
                c.check(|| "ansi256to16".into(), &s, get(r, "out"));
            }
            "short" => {
                let mut s = String::new();
                for i in 0..4096 {
                    let hex = format!("#{i:03x}");
                    for p in [
                        Profile::TrueColor,
                        Profile::ANSI256,
                        Profile::ANSI,
                        Profile::Ascii,
                    ] {
                        s.push_str(&seq(p.color(&hex), i % 2 == 0));
                        s.push(' ');
                    }
                    s.push('\n');
                }
                c.check(
                    || "short hash".into(),
                    &format!("{:016x}", fnv1a64(s.as_bytes())),
                    get(r, "hash"),
                );
                c.check(|| "short head".into(), &s[..4096], get(r, "head"));
                c.bytes += s.len();
            }
            "profilecolor" => {
                let input = get(r, "in");
                let p = profile(get(r, "p"));
                if get(r, "fg") == "panic" {
                    continue;
                }
                c.check(
                    || format!("{r:?}"),
                    &seq(p.color(input), false),
                    get(r, "fg"),
                );
                c.check(
                    || format!("{r:?}"),
                    &seq(p.color(input), true),
                    get(r, "bg"),
                );
            }
            "rgba" => {
                lipgloss::set_color_profile(Profile::TrueColor);
                let (a, b, cc, d) = lipgloss::color(get(r, "in")).rgba();
                c.check(
                    || format!("{r:?}"),
                    &format!("{a} {b} {cc} {d}"),
                    get(r, "out"),
                );
            }
            "rgba_adaptive" => {
                lipgloss::set_color_profile(Profile::TrueColor);
                lipgloss::set_has_dark_background(get(r, "dark") == "true");
                let (a, b, cc, d) = lipgloss::adaptive_color("#102030", "#405060").rgba();
                c.check(
                    || format!("{r:?}"),
                    &format!("{a} {b} {cc} {d}"),
                    get(r, "out"),
                );
            }
            "rgba_nocolor" => {
                let (a, b, cc, d) = lipgloss::TerminalColor::NoColor.rgba();
                c.check(
                    || format!("{r:?}"),
                    &format!("{a} {b} {cc} {d}"),
                    get(r, "out"),
                );
            }
            other => panic!("unknown op {other}"),
        }
    }
    c.finish("colors");
}
