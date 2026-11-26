//! The subset of Go's `flag` package used by `cmd/sst` and `pkg/classic`
//! (`FlagSet` with `ContinueOnError`), reproducing its parsing rules, error
//! messages and `PrintDefaults` output byte for byte.
//!
//! Flags defined with `*_var` share storage with an earlier flag (Go
//! `fs.BoolVar(p, ...)` on an existing pointer), so values are `Rc` cells.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::fmt;
use std::io::Write;
use std::rc::Rc;

use sst_engine::gofmt;

/// Go `flag.ErrHelp` and the errors returned by `failf`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlagError {
    /// `-help` or `-h` was given but not defined.
    Help,
    /// Any other parse error (the message Go prints).
    Other(String),
}

impl fmt::Display for FlagError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FlagError::Help => f.write_str("flag: help requested"),
            FlagError::Other(s) => f.write_str(s),
        }
    }
}

impl std::error::Error for FlagError {}

/// A flag's storage (Go `flag.Value` implementations).
#[derive(Clone)]
pub enum Value {
    Bool(Rc<Cell<bool>>),
    /// `intValue` (Go `int`, 64-bit).
    Int(Rc<Cell<i64>>),
    Int64(Rc<Cell<i64>>),
    Float64(Rc<Cell<f64>>),
    Str(Rc<RefCell<String>>),
}

impl Value {
    /// Go `Value.String()`.
    pub fn string(&self) -> String {
        match self {
            Value::Bool(b) => b.get().to_string(),
            Value::Int(i) | Value::Int64(i) => i.get().to_string(),
            Value::Float64(f) => gofmt::format_float(f.get(), 'g', -1),
            Value::Str(s) => s.borrow().clone(),
        }
    }

    fn zero_string(&self) -> &'static str {
        match self {
            Value::Bool(_) => "false",
            Value::Int(_) | Value::Int64(_) | Value::Float64(_) => "0",
            Value::Str(_) => "",
        }
    }

    fn is_bool(&self) -> bool {
        matches!(self, Value::Bool(_))
    }

    /// Go `Value.Set`; errors are `errParse` / `errRange` messages.
    fn set(&self, s: &str) -> Result<(), &'static str> {
        match self {
            Value::Bool(b) => match parse_bool(s) {
                Some(v) => {
                    b.set(v);
                    Ok(())
                }
                None => {
                    // Go stores the zero value from ParseBool even on error.
                    b.set(false);
                    Err("parse error")
                }
            },
            Value::Int(i) | Value::Int64(i) => {
                let (v, e) = parse_int(s);
                i.set(v);
                e.map_or(Ok(()), Err)
            }
            Value::Float64(f) => {
                let (v, e) = gofmt::parse_float_go(s);
                f.set(v);
                match e {
                    None => Ok(()),
                    Some(e) => match e.kind {
                        gofmt::NumErrKind::Syntax => Err("parse error"),
                        gofmt::NumErrKind::Range => Err("value out of range"),
                    },
                }
            }
            Value::Str(st) => {
                *st.borrow_mut() = s.to_string();
                Ok(())
            }
        }
    }
}

/// Go `flag.Flag`.
#[derive(Clone)]
pub struct Flag {
    pub name: String,
    pub usage: String,
    pub value: Value,
    pub def_value: String,
}

/// Go `flag.FlagSet` with `ContinueOnError` handling.
pub struct FlagSet {
    name: String,
    formal: BTreeMap<String, Flag>,
    actual: BTreeMap<String, ()>,
    args: Vec<String>,
    parsed: bool,
}

impl FlagSet {
    /// Go `flag.NewFlagSet(name, flag.ContinueOnError)`.
    pub fn new(name: &str) -> FlagSet {
        FlagSet {
            name: name.to_string(),
            formal: BTreeMap::new(),
            actual: BTreeMap::new(),
            args: Vec::new(),
            parsed: false,
        }
    }

    fn define(&mut self, value: Value, name: &str, usage: &str) {
        let f = Flag {
            name: name.to_string(),
            usage: usage.to_string(),
            def_value: value.string(),
            value,
        };
        if self.formal.contains_key(name) {
            panic!("{} flag redefined: {}", self.name, name);
        }
        self.formal.insert(name.to_string(), f);
    }

    /// Go `fs.Bool`.
    pub fn bool(&mut self, name: &str, value: bool, usage: &str) -> Rc<Cell<bool>> {
        let p = Rc::new(Cell::new(false));
        self.bool_var(&p, name, value, usage);
        p
    }

    /// Go `fs.BoolVar` (sets `*p = value`).
    pub fn bool_var(&mut self, p: &Rc<Cell<bool>>, name: &str, value: bool, usage: &str) {
        p.set(value);
        self.define(Value::Bool(p.clone()), name, usage);
    }

    /// Go `fs.Int`.
    pub fn int(&mut self, name: &str, value: i64, usage: &str) -> Rc<Cell<i64>> {
        let p = Rc::new(Cell::new(value));
        self.define(Value::Int(p.clone()), name, usage);
        p
    }

    /// Go `fs.Int64`.
    pub fn int64(&mut self, name: &str, value: i64, usage: &str) -> Rc<Cell<i64>> {
        let p = Rc::new(Cell::new(value));
        self.define(Value::Int64(p.clone()), name, usage);
        p
    }

    /// Go `fs.Float64`.
    pub fn float64(&mut self, name: &str, value: f64, usage: &str) -> Rc<Cell<f64>> {
        let p = Rc::new(Cell::new(value));
        self.define(Value::Float64(p.clone()), name, usage);
        p
    }

    /// Go `fs.String`.
    pub fn string(&mut self, name: &str, value: &str, usage: &str) -> Rc<RefCell<String>> {
        let p = Rc::new(RefCell::new(String::new()));
        self.string_var(&p, name, value, usage);
        p
    }

    /// Go `fs.StringVar` (sets `*p = value`).
    pub fn string_var(&mut self, p: &Rc<RefCell<String>>, name: &str, value: &str, usage: &str) {
        *p.borrow_mut() = value.to_string();
        self.define(Value::Str(p.clone()), name, usage);
    }

    /// Go `fs.Lookup`.
    pub fn lookup(&self, name: &str) -> Option<&Flag> {
        self.formal.get(name)
    }

    /// Go `fs.Parsed`.
    pub fn parsed(&self) -> bool {
        self.parsed
    }

    /// Go `fs.Args`: the arguments left after parsing.
    pub fn args(&self) -> &[String] {
        &self.args
    }

    /// Go `fs.Visit`: flags that were set, in lexicographical order.
    pub fn visit(&self, mut f: impl FnMut(&Flag)) {
        for name in self.actual.keys() {
            f(&self.formal[name]);
        }
    }

    /// Go `fs.VisitAll`: all flags in lexicographical order.
    pub fn visit_all(&self, mut f: impl FnMut(&Flag)) {
        for fl in self.formal.values() {
            f(fl);
        }
    }

    /// Go `fs.PrintDefaults`.
    pub fn print_defaults(&self, out: &mut dyn Write) {
        for flag in self.formal.values() {
            let mut b = format!("  -{}", flag.name);
            let (name, usage) = unquote_usage(flag);
            if !name.is_empty() {
                b.push(' ');
                b.push_str(&name);
            }
            if b.len() <= 4 {
                b.push('\t');
            } else {
                b.push_str("\n    \t");
            }
            b.push_str(&usage.replace('\n', "\n    \t"));
            if flag.def_value != flag.value.zero_string() {
                if matches!(flag.value, Value::Str(_)) {
                    b.push_str(&format!(" (default {})", gofmt::quote(&flag.def_value)));
                } else {
                    b.push_str(&format!(" (default {})", flag.def_value));
                }
            }
            b.push('\n');
            let _ = out.write_all(b.as_bytes());
        }
    }

    /// Go's `defaultUsage`.
    pub fn usage(&self, out: &mut dyn Write) {
        if self.name.is_empty() {
            let _ = out.write_all(b"Usage:\n");
        } else {
            let _ = writeln!(out, "Usage of {}:", self.name);
        }
        self.print_defaults(out);
    }

    fn failf(&self, out: &mut dyn Write, msg: String) -> FlagError {
        let _ = writeln!(out, "{msg}");
        self.usage(out);
        FlagError::Other(msg)
    }

    fn parse_one(&mut self, out: &mut dyn Write) -> Result<bool, FlagError> {
        if self.args.is_empty() {
            return Ok(false);
        }
        let s = self.args[0].clone();
        let sb = s.as_bytes();
        if sb.len() < 2 || sb[0] != b'-' {
            return Ok(false);
        }
        let mut num_minuses = 1;
        if sb[1] == b'-' {
            num_minuses += 1;
            if sb.len() == 2 {
                self.args.remove(0);
                return Ok(false);
            }
        }
        let mut name = &s[num_minuses..];
        if name.is_empty() || name.starts_with('-') || name.starts_with('=') {
            return Err(self.failf(out, format!("bad flag syntax: {s}")));
        }
        self.args.remove(0);
        let mut has_value = false;
        let mut value = String::new();
        if let Some(i) = name[1..].find('=') {
            let i = i + 1;
            value = name[i + 1..].to_string();
            has_value = true;
            name = &name[..i];
        }
        let name = name.to_string();
        let Some(flag) = self.formal.get(&name).cloned() else {
            if name == "help" || name == "h" {
                self.usage(out);
                return Err(FlagError::Help);
            }
            return Err(self.failf(out, format!("flag provided but not defined: -{name}")));
        };
        if flag.value.is_bool() {
            if has_value {
                if let Err(e) = flag.value.set(&value) {
                    return Err(self.failf(
                        out,
                        format!(
                            "invalid boolean value {} for -{}: {}",
                            gofmt::quote(&value),
                            name,
                            e
                        ),
                    ));
                }
            } else if let Err(e) = flag.value.set("true") {
                return Err(self.failf(out, format!("invalid boolean flag {name}: {e}")));
            }
        } else {
            if !has_value && !self.args.is_empty() {
                has_value = true;
                value = self.args.remove(0);
            }
            if !has_value {
                return Err(self.failf(out, format!("flag needs an argument: -{name}")));
            }
            if let Err(e) = flag.value.set(&value) {
                return Err(self.failf(
                    out,
                    format!(
                        "invalid value {} for flag -{}: {}",
                        gofmt::quote(&value),
                        name,
                        e
                    ),
                ));
            }
        }
        self.actual.insert(name, ());
        Ok(true)
    }

    /// Go `fs.Parse(arguments)` with `ContinueOnError`; diagnostics and usage
    /// go to `out` (Go's `fs.SetOutput`).
    pub fn parse(&mut self, arguments: &[String], out: &mut dyn Write) -> Result<(), FlagError> {
        self.parsed = true;
        self.args = arguments.to_vec();
        loop {
            match self.parse_one(out) {
                Ok(true) => continue,
                Ok(false) => return Ok(()),
                Err(e) => return Err(e),
            }
        }
    }
}

/// Go `flag.UnquoteUsage`.
pub fn unquote_usage(flag: &Flag) -> (String, String) {
    let usage = &flag.usage;
    if let Some(i) = usage.find('`') {
        if let Some(j) = usage[i + 1..].find('`') {
            let j = i + 1 + j;
            let name = usage[i + 1..j].to_string();
            let u = format!("{}{}{}", &usage[..i], name, &usage[j + 1..]);
            return (name, u);
        }
    }
    let name = match flag.value {
        Value::Bool(_) => "",
        Value::Float64(_) => "float",
        Value::Int(_) | Value::Int64(_) => "int",
        Value::Str(_) => "string",
    };
    (name.to_string(), usage.clone())
}

/// Go `strconv.ParseBool`.
pub fn parse_bool(s: &str) -> Option<bool> {
    match s {
        "1" | "t" | "T" | "TRUE" | "true" | "True" => Some(true),
        "0" | "f" | "F" | "FALSE" | "false" | "False" => Some(false),
        _ => None,
    }
}

fn underscore_ok(s: &str) -> bool {
    let mut s = s.as_bytes();
    let mut saw = b'^';
    let mut i = 0;
    if !s.is_empty() && (s[0] == b'-' || s[0] == b'+') {
        s = &s[1..];
    }
    let mut hex = false;
    if s.len() >= 2 && s[0] == b'0' && matches!(s[1].to_ascii_lowercase(), b'b' | b'o' | b'x') {
        i = 2;
        saw = b'0';
        hex = s[1].eq_ignore_ascii_case(&b'x');
    }
    while i < s.len() {
        let c = s[i];
        if c.is_ascii_digit() || (hex && matches!(c.to_ascii_lowercase(), b'a'..=b'f')) {
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

enum UintErr {
    Syntax,
    Range,
}

/// Go `strconv.ParseUint(s, 0, 64)`.
fn parse_uint0(s: &str) -> Result<u64, (u64, UintErr)> {
    if s.is_empty() {
        return Err((0, UintErr::Syntax));
    }
    let s0 = s;
    let b = s.as_bytes();
    let mut base: u64 = 10;
    let mut digits = b;
    if b[0] == b'0' {
        if b.len() >= 3 && b[1].eq_ignore_ascii_case(&b'b') {
            base = 2;
            digits = &b[2..];
        } else if b.len() >= 3 && b[1].eq_ignore_ascii_case(&b'o') {
            base = 8;
            digits = &b[2..];
        } else if b.len() >= 3 && b[1].eq_ignore_ascii_case(&b'x') {
            base = 16;
            digits = &b[2..];
        } else {
            base = 8;
            digits = &b[1..];
        }
    }
    let cutoff = u64::MAX / base + 1;
    let max_val = u64::MAX;
    let mut underscores = false;
    let mut n: u64 = 0;
    for &c in digits {
        let d = if c == b'_' {
            underscores = true;
            continue;
        } else if c.is_ascii_digit() {
            c - b'0'
        } else if c.to_ascii_lowercase().is_ascii_lowercase() {
            c.to_ascii_lowercase() - b'a' + 10
        } else {
            return Err((0, UintErr::Syntax));
        };
        if d as u64 >= base {
            return Err((0, UintErr::Syntax));
        }
        if n >= cutoff {
            return Err((max_val, UintErr::Range));
        }
        n *= base;
        let (n1, over) = n.overflowing_add(d as u64);
        if over || n1 > max_val {
            return Err((max_val, UintErr::Range));
        }
        n = n1;
    }
    if underscores && !underscore_ok(s0) {
        return Err((0, UintErr::Syntax));
    }
    Ok(n)
}

/// Go `strconv.ParseInt(s, 0, 64)` mapped through the flag package's
/// `numError` (`parse error` / `value out of range`). Returns Go's value too.
pub fn parse_int(s: &str) -> (i64, Option<&'static str>) {
    if s.is_empty() {
        return (0, Some("parse error"));
    }
    let mut rest = s;
    let mut neg = false;
    if let Some(r) = s.strip_prefix('+') {
        rest = r;
    } else if let Some(r) = s.strip_prefix('-') {
        rest = r;
        neg = true;
    }
    let un = match parse_uint0(rest) {
        Ok(v) => v,
        Err((_, UintErr::Syntax)) => return (0, Some("parse error")),
        Err((v, UintErr::Range)) => v,
    };
    // A ParseUint range error yields MaxUint64, which fails both checks below.
    let cutoff: u64 = 1 << 63;
    if !neg && un >= cutoff {
        return ((cutoff - 1) as i64, Some("value out of range"));
    }
    if neg && un > cutoff {
        return (i64::MIN, Some("value out of range"));
    }
    let n = if neg {
        (un as i64).wrapping_neg()
    } else {
        un as i64
    };
    (n, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_int_go_semantics() {
        assert_eq!(parse_int("42"), (42, None));
        assert_eq!(parse_int("0x2A"), (42, None));
        assert_eq!(parse_int("017"), (15, None));
        assert_eq!(parse_int("0b101"), (5, None));
        assert_eq!(parse_int("0o17"), (15, None));
        assert_eq!(parse_int("1_000"), (1000, None));
        assert_eq!(parse_int("_1000").1, Some("parse error"));
        assert_eq!(parse_int("-9223372036854775808"), (i64::MIN, None));
        assert_eq!(
            parse_int("9223372036854775808"),
            (i64::MAX, Some("value out of range"))
        );
        assert_eq!(parse_int("abc").1, Some("parse error"));
        assert_eq!(parse_int("1.5").1, Some("parse error"));
        assert_eq!(parse_int("0").0, 0);
        assert_eq!(parse_int("08").1, Some("parse error"));
    }

    #[test]
    fn print_defaults_format() {
        let mut fs = FlagSet::new("x");
        fs.bool("a", false, "short bool");
        fs.string("name", "def", "a `who` to greet");
        fs.float64("f", 1.0, "factor");
        fs.int("n", 0, "count");
        let mut out = Vec::new();
        fs.print_defaults(&mut out);
        assert_eq!(
            String::from_utf8(out).unwrap(),
            "  -a\tshort bool\n  -f float\n    \tfactor (default 1)\n  -n int\n    \tcount\n  -name who\n    \ta who to greet (default \"def\")\n"
        );
    }
}
