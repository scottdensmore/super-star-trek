//! Parity: PRNG streams, Go math / fmt / strconv / json scalars, parser.

mod common;
use common::*;

use sst_engine::gofmt::{self, atoi_go, format_float, parse_float_go, quote_bytes};
use sst_engine::gomath;
use sst_engine::json;
use sst_engine::parser::parse_command;
use sst_engine::PRNG;

fn bits(s: &str) -> f64 {
    f64::from_bits(u64::from_str_radix(s, 16).unwrap())
}

#[test]
fn prng_streams_match_go() {
    let d = data("prng.txt");
    let mut p = PRNG::new(0);
    let mut draws = 0;
    let mut seeds = 0;
    for line in d.lines() {
        let parts: Vec<&str> = line.split(' ').collect();
        match parts[0] {
            "seed" => {
                p = PRNG::new(parts[1].parse().unwrap());
                seeds += 1;
            }
            "f" => assert_eq!(
                format!("{:016x}", p.float64().to_bits()),
                parts[1],
                "{line}"
            ),
            "r" => assert_eq!(
                format!("{:016x}", p.rand_real().to_bits()),
                parts[1],
                "{line}"
            ),
            "i" => {
                let n: i64 = parts[1].parse().unwrap();
                let v = p.rand().int_n(n);
                assert_eq!(v.to_string(), parts[2], "{line}");
            }
            "state" => assert_eq!(rng_state(&Some(p.clone())), parts[1]),
            _ => panic!("bad line {line}"),
        }
        draws += 1;
    }
    eprintln!("prng parity: {seeds} seeds, {draws} draws");
    assert!(seeds >= 50);
}

#[test]
fn math_and_fmt_match_go() {
    let d = data("math.txt");
    let mut n = 0;
    for line in d.lines() {
        let (kind, rest) = line.split_once(' ').unwrap();
        n += 1;
        match kind {
            "fmt" => {
                let f: Vec<&str> = rest.split('|').collect();
                let x = bits(f[0]);
                let json = match json::marshal(&x) {
                    Ok(b) => String::from_utf8(b).unwrap(),
                    Err(e) => format!("ERR {e}"),
                };
                let got = [
                    gofmt::fmt_v(x),
                    gofmt::fmt_v(x),
                    gofmt::fmt_e6(x),
                    gofmt::fmt_f(x, 1),
                    gofmt::fmt_f(x, 2),
                    gofmt::fmt_f(x, 0),
                    gofmt::pad_left(&gofmt::fmt_f(x, 1), 5),
                    gofmt::pad_left(&format_float(x, 'g', 3), 8),
                    format_float(x, 'e', 3),
                    format_float(x, 'f', -1),
                    format_float(x, 'g', 4),
                    json,
                ];
                for (i, g) in got.iter().enumerate() {
                    assert_eq!(g, f[i + 1], "fmt field {} of {line}", i + 1);
                }
            }
            "str" => {
                let f: Vec<&str> = rest.split('|').collect();
                let raw = hex_decode(f[0]);
                assert_eq!(quote_bytes(&raw), f[1], "quote {line}");
                if let Ok(s) = std::str::from_utf8(&raw) {
                    let mut e = json::Encoder::default();
                    e.string(s);
                    assert_eq!(String::from_utf8(e.buf).unwrap(), f[2], "json {line}");
                }
            }
            "num" => {
                let f: Vec<&str> = rest.split('|').collect();
                let s = hex_str(f[0]);
                let (v, e) = parse_float_go(&s);
                assert_eq!(
                    format!("{:016x}", v.to_bits()),
                    f[1],
                    "ParseFloat value {s:?}"
                );
                assert_eq!(
                    e.map_or("<nil>".to_string(), |e| e.to_string()),
                    f[2],
                    "ParseFloat err {s:?}"
                );
                let (n, e) = atoi_go(&s);
                assert_eq!(n.to_string(), f[3], "Atoi value {s:?}");
                assert_eq!(
                    e.map_or("<nil>".to_string(), |e| e.to_string()),
                    f[4],
                    "Atoi err {s:?}"
                );
            }
            _ => {
                let a: Vec<f64> = rest.split(' ').map(bits).collect();
                let want = a[a.len() - 1].to_bits();
                let got = match kind {
                    "sin" => gomath::sin(a[0]),
                    "cos" => gomath::cos(a[0]),
                    "atan" => gomath::atan(a[0]),
                    "exp" => gomath::exp(a[0]),
                    "log" => gomath::log(a[0]),
                    "log10" => gomath::log10(a[0]),
                    "round" => gomath::round(a[0]),
                    "acos" => gomath::acos(a[0]),
                    "asin" => gomath::asin(a[0]),
                    "atan2" => gomath::atan2(a[0], a[1]),
                    "hypot" => gomath::hypot(a[0], a[1]),
                    "mod" => gomath::mod_(a[0], a[1]),
                    "max" => gomath::max(a[0], a[1]),
                    "min" => gomath::min(a[0], a[1]),
                    "pow" => gomath::pow(a[0], a[1]),
                    _ => panic!("unknown {kind}"),
                };
                let same = got.to_bits() == want || (got.is_nan() && f64::from_bits(want).is_nan());
                assert!(
                    same,
                    "{kind}{:?}: want {:016x} got {:016x}",
                    &a[..a.len() - 1],
                    want,
                    got.to_bits()
                );
            }
        }
    }
    eprintln!("math parity: {n} lines");
}

#[test]
fn parser_matches_go() {
    let d = data("parser.txt");
    let mut l = Lines::new(&d);
    let mut n = 0;
    while let Some(line) = l.next() {
        let input = hex_str(line.strip_prefix("in ").unwrap());
        l.check("P ", &parse_command(&input).to_json());
        n += 1;
    }
    eprintln!("parser parity: {n} inputs, {} bytes", l.bytes_compared);
    assert!(n > 300);
}
