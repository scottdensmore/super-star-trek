//! Helpers reproducing Go `fmt` / `strconv` formatting that Rust's
//! `format!` does not match: `%v`/`%g`/`%e` of floats, `%q`, rune-counted
//! padding, `strconv.FormatFloat`, `strconv.ParseFloat`/`Atoi` syntax.

/// Decimal digits and decimal-point position of a finite, positive float:
/// value = 0.d1d2d3... * 10^dp (Go strconv `decimalSlice`). Zero has no digits.
fn decimal_from_e(s: &str) -> (Vec<u8>, i32) {
    // s is Rust `{:e}`-style: "d.ddde[-]X"
    let (mant, exp) = s.split_once('e').unwrap();
    let exp: i32 = exp.parse().unwrap();
    let mut digits: Vec<u8> = mant.bytes().filter(|b| *b != b'.').collect();
    while digits.len() > 1 && *digits.last().unwrap() == b'0' {
        digits.pop();
    }
    (digits, exp + 1)
}

fn shortest_decimal(abs: f64) -> (Vec<u8>, i32) {
    if abs == 0.0 {
        return (vec![], 0);
    }
    decimal_from_e(&format!("{:e}", abs))
}

fn fixed_sig_decimal(abs: f64, sig: usize) -> (Vec<u8>, i32) {
    if abs == 0.0 {
        return (vec![], 0);
    }
    decimal_from_e(&format!("{:.*e}", sig.saturating_sub(1), abs))
}

fn fmt_e(neg: bool, d: &[u8], dp: i32, prec: i32, upper: bool) -> String {
    let mut out = String::new();
    if neg {
        out.push('-');
    }
    let nd = d.len() as i32;
    out.push(if nd == 0 { '0' } else { d[0] as char });
    if prec > 0 {
        out.push('.');
        let mut i = 1;
        let m = nd.min(prec + 1);
        while i < m {
            out.push(d[i as usize] as char);
            i += 1;
        }
        while i <= prec {
            out.push('0');
            i += 1;
        }
    }
    out.push(if upper { 'E' } else { 'e' });
    let mut exp = dp - 1;
    if nd == 0 {
        exp = 0;
    }
    let (sign, e) = if exp < 0 { ('-', -exp) } else { ('+', exp) };
    out.push(sign);
    if e < 10 {
        out.push('0');
    }
    out.push_str(&e.to_string());
    out
}

fn fmt_fixed(neg: bool, d: &[u8], dp: i32, prec: i32) -> String {
    let mut out = String::new();
    if neg {
        out.push('-');
    }
    let nd = d.len() as i32;
    if dp > 0 {
        let m = dp.min(nd);
        for i in 0..m {
            out.push(d[i as usize] as char);
        }
        for _ in m..dp {
            out.push('0');
        }
    } else {
        out.push('0');
    }
    if prec > 0 {
        out.push('.');
        for i in 1..=prec {
            let j = dp + i - 1;
            let ch = if j < 0 || j >= nd {
                b'0'
            } else {
                d[j as usize]
            };
            out.push(ch as char);
        }
    }
    out
}

/// Go `strconv.FormatFloat(f, fmt, prec, 64)` for `fmt` in `'f' 'e' 'E' 'g' 'G'`.
pub fn format_float(f: f64, fmt: char, prec: i32) -> String {
    if f.is_nan() {
        return "NaN".to_string();
    }
    if f.is_infinite() {
        return if f > 0.0 { "+Inf" } else { "-Inf" }.to_string();
    }
    let neg = f.is_sign_negative();
    let abs = f.abs();
    let shortest = prec < 0;
    match fmt {
        'f' => {
            if shortest {
                let (d, dp) = shortest_decimal(abs);
                fmt_fixed(neg, &d, dp, (d.len() as i32 - dp).max(0))
            } else {
                let s = format!("{:.*}", prec as usize, abs);
                if neg {
                    format!("-{s}")
                } else {
                    s
                }
            }
        }
        'e' | 'E' => {
            let upper = fmt == 'E';
            if shortest {
                let (d, dp) = shortest_decimal(abs);
                fmt_e(neg, &d, dp, (d.len() as i32 - 1).max(0), upper)
            } else {
                let (d, dp) = fixed_sig_decimal(abs, prec as usize + 1);
                fmt_e(neg, &d, dp, prec, upper)
            }
        }
        'g' | 'G' => {
            let upper = fmt == 'G';
            let (d, dp) = if shortest {
                shortest_decimal(abs)
            } else {
                fixed_sig_decimal(abs, if prec == 0 { 1 } else { prec as usize })
            };
            let nd = d.len() as i32;
            let mut p = if shortest {
                nd
            } else if prec == 0 {
                1
            } else {
                prec
            };
            let mut eprec = p;
            if eprec > nd && nd >= dp {
                eprec = nd;
            }
            if shortest {
                eprec = 6;
            }
            let exp = dp - 1;
            if exp < -4 || exp >= eprec {
                if p > nd {
                    p = nd;
                }
                return fmt_e(neg, &d, dp, p - 1, upper);
            }
            if p > dp {
                p = nd;
            }
            fmt_fixed(neg, &d, dp, (p - dp).max(0))
        }
        _ => format!("%!{fmt}(float64={f})"),
    }
}

/// Go `fmt` `%v` / `%g` of a float64.
pub fn fmt_v(f: f64) -> String {
    format_float(f, 'g', -1)
}

/// Go `fmt` `%e` of a float64 (precision 6).
pub fn fmt_e6(f: f64) -> String {
    format_float(f, 'e', 6)
}

/// Go `fmt` `%.Nf`.
pub fn fmt_f(f: f64, prec: usize) -> String {
    if f.is_nan() {
        return "NaN".into();
    }
    if f.is_infinite() {
        return if f > 0.0 { "+Inf" } else { "-Inf" }.into();
    }
    format!("{:.*}", prec, f)
}

fn is_print(c: char) -> bool {
    let u = c as u32;
    if u < 0x20 || (0x7f..0xa0).contains(&u) {
        return false;
    }
    !matches!(
        u,
        0xad | 0x200b..=0x200f | 0x2028..=0x202e | 0x2060..=0x206f | 0xfeff | 0xfff9..=0xfffb
            | 0xe000..=0xf8ff
    ) && !(0xd800..0xe000).contains(&u)
}

/// Go `strconv.Quote` / `fmt` `%q` of a string.
pub fn quote(s: &str) -> String {
    quote_bytes(s.as_bytes())
}

/// Go `strconv.Quote` over raw bytes (invalid UTF-8 becomes `\xNN`).
pub fn quote_bytes(b: &[u8]) -> String {
    let mut out = String::with_capacity(b.len() + 2);
    out.push('"');
    let mut i = 0;
    while i < b.len() {
        let (ch, width) = match decode_rune(&b[i..]) {
            Some((c, w)) => (Some(c), w),
            None => (None, 1),
        };
        match ch {
            None => {
                out.push_str(&format!("\\x{:02x}", b[i]));
            }
            Some(c) => match c {
                '\u{7}' => out.push_str("\\a"),
                '\u{8}' => out.push_str("\\b"),
                '\u{c}' => out.push_str("\\f"),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                '\u{b}' => out.push_str("\\v"),
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                c if is_print(c) => out.push(c),
                c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                    out.push_str(&format!("\\x{:02x}", c as u32))
                }
                c if (c as u32) < 0x10000 => out.push_str(&format!("\\u{:04x}", c as u32)),
                c => out.push_str(&format!("\\U{:08x}", c as u32)),
            },
        }
        i += width;
    }
    out.push('"');
    out
}

fn decode_rune(b: &[u8]) -> Option<(char, usize)> {
    let n = match b[0] {
        0x00..=0x7f => 1,
        0xc2..=0xdf => 2,
        0xe0..=0xef => 3,
        0xf0..=0xf4 => 4,
        _ => return None,
    };
    if b.len() < n {
        return None;
    }
    std::str::from_utf8(&b[..n])
        .ok()
        .and_then(|s| s.chars().next())
        .map(|c| (c, n))
}

/// Go `%-Ns`: left-justify to `width` runes.
pub fn pad_right(s: &str, width: usize) -> String {
    let n = s.chars().count();
    if n >= width {
        return s.to_string();
    }
    format!("{}{}", s, " ".repeat(width - n))
}

/// Go `%Ns`: right-justify to `width` runes.
pub fn pad_left(s: &str, width: usize) -> String {
    let n = s.chars().count();
    if n >= width {
        return s.to_string();
    }
    format!("{}{}", " ".repeat(width - n), s)
}

/// Go `strconv.ParseFloat(s, 64)` (decimal and hex forms, `inf`, `nan`,
/// `_` digit separators). Returns `Err` on syntax or range errors; see
/// [`parse_float_go`] for Go's value-plus-error result.
pub fn parse_float(s: &str) -> Result<f64, ParseNumError> {
    let (v, e) = parse_float_go(s);
    match e {
        Some(e) => Err(e),
        None => Ok(v),
    }
}

/// Go `strconv.ParseFloat(s, 64)` returning both the value (0 on syntax
/// errors, ±Inf on overflow) and the error.
pub fn parse_float_go(s: &str) -> (f64, Option<ParseNumError>) {
    match parse_float_full(s) {
        Ok(v) => (v, None),
        Err((kind, v)) => (
            v,
            Some(ParseNumError {
                func: "ParseFloat",
                num: s.to_string(),
                kind,
            }),
        ),
    }
}

/// Error from [`parse_float`] / [`atoi`], formatted like Go's `*strconv.NumError`.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseNumError {
    pub func: &'static str,
    pub num: String,
    pub kind: NumErrKind,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NumErrKind {
    Syntax,
    Range,
}

impl std::fmt::Display for ParseNumError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self.kind {
            NumErrKind::Syntax => "invalid syntax",
            NumErrKind::Range => "value out of range",
        };
        write!(
            f,
            "strconv.{}: parsing {}: {}",
            self.func,
            quote(&self.num),
            msg
        )
    }
}

impl std::error::Error for ParseNumError {}

/// Port of strconv `underscoreOK`.
fn underscore_ok(s: &[u8]) -> bool {
    let mut saw = b'^';
    let mut i = 0;
    if !s.is_empty() && (s[0] == b'-' || s[0] == b'+') {
        i = 1;
    }
    let mut hex = false;
    if s.len() - i >= 2 && s[i] == b'0' && matches!(s[i + 1] | 0x20, b'b' | b'o' | b'x') {
        hex = (s[i + 1] | 0x20) == b'x';
        i += 2;
        saw = b'0';
    }
    while i < s.len() {
        let c = s[i];
        let l = c | 0x20;
        if c.is_ascii_digit() || (hex && (b'a'..=b'f').contains(&l)) {
            saw = b'0';
            i += 1;
            continue;
        }
        if c == b'_' {
            if saw != b'0' {
                return false;
            }
            saw = b'_';
            i += 1;
            continue;
        }
        if saw == b'_' {
            return false;
        }
        saw = b'!';
        i += 1;
    }
    saw != b'_'
}

fn parse_float_full(s: &str) -> Result<f64, (NumErrKind, f64)> {
    const SYN: (NumErrKind, f64) = (NumErrKind::Syntax, 0.0);
    if s.is_empty() {
        return Err(SYN);
    }
    let b = s.as_bytes();
    // specials
    {
        let (neg, rest) = match b[0] {
            b'+' => (false, &s[1..]),
            b'-' => (true, &s[1..]),
            _ => (false, s),
        };
        let lower = rest.to_ascii_lowercase();
        if lower == "inf" || lower == "infinity" {
            return Ok(if neg {
                f64::NEG_INFINITY
            } else {
                f64::INFINITY
            });
        }
        if lower == "nan" && rest.len() == s.len() {
            return Ok(crate::gomath::nan());
        }
    }
    if s.contains('_') && !underscore_ok(b) {
        return Err(SYN);
    }
    let clean: String = s.chars().filter(|c| *c != '_').collect();
    let (neg, body) = match clean.as_bytes().first() {
        Some(b'+') => (false, &clean[1..]),
        Some(b'-') => (true, &clean[1..]),
        _ => (false, &clean[..]),
    };
    let bb = body.as_bytes();
    if bb.len() > 2 && bb[0] == b'0' && (bb[1] | 0x20) == b'x' {
        let v = parse_hex_float(&body[2..]).map_err(|k| (k, 0.0))?;
        if v.is_infinite() {
            let inf = if neg {
                f64::NEG_INFINITY
            } else {
                f64::INFINITY
            };
            return Err((NumErrKind::Range, inf));
        }
        return Ok(if neg { -v } else { v });
    }
    let mut i = 0;
    let mut saw_digits = false;
    while i < bb.len() && bb[i].is_ascii_digit() {
        i += 1;
        saw_digits = true;
    }
    if i < bb.len() && bb[i] == b'.' {
        i += 1;
        while i < bb.len() && bb[i].is_ascii_digit() {
            i += 1;
            saw_digits = true;
        }
    }
    if !saw_digits {
        return Err(SYN);
    }
    if i < bb.len() && (bb[i] | 0x20) == b'e' {
        i += 1;
        if i < bb.len() && (bb[i] == b'+' || bb[i] == b'-') {
            i += 1;
        }
        let st = i;
        while i < bb.len() && bb[i].is_ascii_digit() {
            i += 1;
        }
        if i == st {
            return Err(SYN);
        }
    }
    if i != bb.len() {
        return Err(SYN);
    }
    let v: f64 = body.parse().map_err(|_| SYN)?;
    let v = if neg { -v } else { v };
    if v.is_infinite() {
        return Err((NumErrKind::Range, v));
    }
    Ok(v)
}

fn parse_hex_float(s: &str) -> Result<f64, NumErrKind> {
    // mantissa hex digits [. hex digits] p[+-]dec
    let b = s.as_bytes();
    let mut i = 0;
    let mut mant: u128 = 0;
    let mut exp: i64 = 0;
    let mut saw = false;
    while i < b.len() && b[i].is_ascii_hexdigit() {
        let d = (b[i] as char).to_digit(16).unwrap() as u128;
        if mant >> 116 == 0 {
            mant = mant * 16 + d;
        } else {
            exp += 4;
        }
        i += 1;
        saw = true;
    }
    if i < b.len() && b[i] == b'.' {
        i += 1;
        while i < b.len() && b[i].is_ascii_hexdigit() {
            let d = (b[i] as char).to_digit(16).unwrap() as u128;
            if mant >> 116 == 0 {
                mant = mant * 16 + d;
                exp -= 4;
            }
            i += 1;
            saw = true;
        }
    }
    if !saw || i >= b.len() || (b[i] | 0x20) != b'p' {
        return Err(NumErrKind::Syntax);
    }
    i += 1;
    let mut eneg = false;
    if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
        eneg = b[i] == b'-';
        i += 1;
    }
    let st = i;
    let mut e: i64 = 0;
    while i < b.len() && b[i].is_ascii_digit() {
        if e < 100000 {
            e = e * 10 + (b[i] - b'0') as i64;
        }
        i += 1;
    }
    if i == st || i != b.len() {
        return Err(NumErrKind::Syntax);
    }
    exp += if eneg { -e } else { e };
    Ok(crate::gomath::ldexp(mant as f64, exp))
}

/// Go `strconv.Atoi` returning Go's value (clamped on overflow, 0 on syntax error) and error.
pub fn atoi_go(s: &str) -> (i64, Option<ParseNumError>) {
    match atoi(s) {
        Ok(v) => (v, None),
        Err(e) => {
            let v = if e.kind == NumErrKind::Range {
                if s.starts_with('-') {
                    i64::MIN
                } else {
                    i64::MAX
                }
            } else {
                0
            };
            (v, Some(e))
        }
    }
}

/// Go `strconv.Atoi` (64-bit int). Returns the value as i64.
pub fn atoi(s: &str) -> Result<i64, ParseNumError> {
    let err = |kind| ParseNumError {
        func: "Atoi",
        num: s.to_string(),
        kind,
    };
    let b = s.as_bytes();
    if b.is_empty() {
        return Err(err(NumErrKind::Syntax));
    }
    let (neg, digits) = match b[0] {
        b'+' => (false, &b[1..]),
        b'-' => (true, &b[1..]),
        _ => (false, b),
    };
    if digits.is_empty() || !digits.iter().all(|c| c.is_ascii_digit()) {
        return Err(err(NumErrKind::Syntax));
    }
    let mut v: i128 = 0;
    for d in digits {
        v = v * 10 + (*d - b'0') as i128;
        if v > (i64::MAX as i128) + 1 {
            return Err(err(NumErrKind::Range));
        }
    }
    if neg {
        v = -v;
    }
    if v > i64::MAX as i128 || v < i64::MIN as i128 {
        return Err(err(NumErrKind::Range));
    }
    Ok(v as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floats() {
        assert_eq!(fmt_v(1e8), "1e+08");
        assert_eq!(fmt_v(123456789.0), "1.23456789e+08");
        assert_eq!(fmt_v(1e20), "1e+20");
        assert_eq!(fmt_v(0.00001), "1e-05");
        assert_eq!(fmt_v(0.0001), "0.0001");
        assert_eq!(fmt_v(1.5), "1.5");
        assert_eq!(fmt_v(1234567.0), "1.234567e+06");
        assert_eq!(fmt_v(100000.0), "100000");
        assert_eq!(fmt_v(0.0), "0");
        assert_eq!(fmt_v(-2.5), "-2.5");
        assert_eq!(format_float(1e21, 'f', -1), "1000000000000000000000");
        assert_eq!(format_float(1.5e-7, 'e', -1), "1.5e-07");
    }

    #[test]
    fn quoting() {
        assert_eq!(quote("abc"), "\"abc\"");
        assert_eq!(quote("a\"b\n"), "\"a\\\"b\\n\"");
        assert_eq!(quote("\u{1}"), "\"\\x01\"");
    }
}
