//! Go `encoding/json` (v1, Go 1.27) compatible encoding and decoding.
//!
//! * [`marshal`] / [`marshal_indent`] produce bytes identical to Go's
//!   `json.Marshal` / `json.MarshalIndent(v, prefix, indent)`: struct fields in
//!   declaration order, `omitempty`, HTML escaping of `< > &`, U+2028/2029
//!   escaping, Go float formatting, nil slices/maps/pointers as `null`, sorted
//!   map keys.
//! * [`unmarshal`] follows Go's semantics: the whole input is syntax-checked
//!   first (syntax errors leave the target untouched), keys match struct
//!   fields exactly or case-insensitively, unknown keys are ignored, missing
//!   fields keep their current value, `null` is a no-op except for pointers,
//!   slices and maps (which become nil), and type mismatches are reported as
//!   `json: cannot unmarshal X into Go struct field S.f of type T` (the first
//!   such error is returned after decoding everything else).
//!
//! Types opt in by implementing [`GoJson`]; structs usually use the
//! [`go_json_struct!`](crate::go_json_struct) macro. Go pointers map to
//! `Option<T>` / `Option<Rc<RefCell<T>>>`, Go slices to [`GoVec`] (which
//! remembers nil vs empty), Go maps to `Option<BTreeMap<K, V>>`.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt;
use std::rc::Rc;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// A JSON error, displayed exactly like the corresponding Go error.
#[derive(Debug, Clone, PartialEq)]
pub enum JsonError {
    /// `*json.SyntaxError`.
    Syntax { msg: String, offset: usize },
    /// `*json.UnmarshalTypeError` (already formatted).
    Type(String),
    /// `*json.UnsupportedValueError` and other errors (already formatted).
    Other(String),
}

impl fmt::Display for JsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JsonError::Syntax { msg, .. } => f.write_str(msg),
            JsonError::Type(s) | JsonError::Other(s) => f.write_str(s),
        }
    }
}

impl std::error::Error for JsonError {}

// ---------------------------------------------------------------------------
// Value tree + parser (Go scanner semantics and error messages)
// ---------------------------------------------------------------------------

/// Parsed JSON value. Numbers keep their literal text; objects keep key order
/// (and duplicates, which Go applies in order).
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Number(String),
    String(String),
    Array(Vec<Value>),
    Object(Vec<(String, Value)>),
}

impl Value {
    /// Kind name as used in Go's `UnmarshalTypeError.Value`.
    pub fn kind_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "bool",
            Value::Number(_) => "number",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
        }
    }

    /// Looks up an object member by exact key (last occurrence wins).
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Object(m) => m.iter().rev().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
}

/// Go `strconv.QuoteRune`-style quoting of the offending character.
fn quote_rune(c: char) -> String {
    if c == '\'' {
        return "'\\''".to_string();
    }
    if c == '"' {
        return "'\"'".to_string();
    }
    let s = crate::gofmt::quote(&c.to_string());
    format!("'{}'", &s[1..s.len() - 1])
}

struct Parser<'a> {
    b: &'a [u8],
    i: usize,
}

fn is_space(c: u8) -> bool {
    c == b' ' || c == b'\t' || c == b'\n' || c == b'\r'
}

impl<'a> Parser<'a> {
    fn err(&self, msg: String) -> JsonError {
        JsonError::Syntax {
            msg,
            offset: self.i,
        }
    }

    fn eof(&self) -> JsonError {
        JsonError::Syntax {
            msg: "unexpected end of JSON input".to_string(),
            offset: self.b.len(),
        }
    }

    fn skip_ws(&mut self) {
        while self.i < self.b.len() && is_space(self.b[self.i]) {
            self.i += 1;
        }
    }

    fn invalid(&self, ctx: &str) -> JsonError {
        let rest = &self.b[self.i..];
        let n = rest.len().min(4);
        let c = (1..=n)
            .find_map(|k| {
                std::str::from_utf8(&rest[..k])
                    .ok()
                    .and_then(|s| s.chars().next())
            })
            .unwrap_or(char::from(rest[0]));
        JsonError::Syntax {
            msg: format!("invalid character {} {}", quote_rune(c), ctx),
            offset: self.i + 1,
        }
    }

    fn bad_escape(&self, start: usize) -> JsonError {
        let len = if self.b.get(start + 1) == Some(&b'u') {
            6
        } else {
            2
        };
        let end = (start + len).min(self.b.len());
        JsonError::Syntax {
            msg: format!(
                "invalid escape sequence `{}` in string",
                String::from_utf8_lossy(&self.b[start..end])
            ),
            offset: end,
        }
    }

    fn value(&mut self, depth: usize) -> Result<Value, JsonError> {
        self.skip_ws();
        if self.i >= self.b.len() {
            return Err(self.eof());
        }
        if depth > 10000 {
            return Err(self.err("exceeded max depth".to_string()));
        }
        match self.b[self.i] {
            b'{' => {
                self.i += 1;
                let mut members = Vec::new();
                self.skip_ws();
                if self.i >= self.b.len() {
                    return Err(self.eof());
                }
                if self.b[self.i] == b'}' {
                    self.i += 1;
                    return Ok(Value::Object(members));
                }
                loop {
                    self.skip_ws();
                    if self.i >= self.b.len() {
                        return Err(self.eof());
                    }
                    if self.b[self.i] != b'"' {
                        return Err(self.invalid("looking for beginning of object key string"));
                    }
                    let k = self.string()?;
                    self.skip_ws();
                    if self.i >= self.b.len() {
                        return Err(self.eof());
                    }
                    if self.b[self.i] != b':' {
                        return Err(self.invalid("after object key"));
                    }
                    self.i += 1;
                    let v = self.value(depth + 1)?;
                    members.push((k, v));
                    self.skip_ws();
                    if self.i >= self.b.len() {
                        return Err(self.eof());
                    }
                    match self.b[self.i] {
                        b',' => {
                            self.i += 1;
                        }
                        b'}' => {
                            self.i += 1;
                            return Ok(Value::Object(members));
                        }
                        _ => return Err(self.invalid("after object key:value pair")),
                    }
                }
            }
            b'[' => {
                self.i += 1;
                let mut items = Vec::new();
                self.skip_ws();
                if self.i >= self.b.len() {
                    return Err(self.eof());
                }
                if self.b[self.i] == b']' {
                    self.i += 1;
                    return Ok(Value::Array(items));
                }
                loop {
                    let v = self.value(depth + 1)?;
                    items.push(v);
                    self.skip_ws();
                    if self.i >= self.b.len() {
                        return Err(self.eof());
                    }
                    match self.b[self.i] {
                        b',' => {
                            self.i += 1;
                        }
                        b']' => {
                            self.i += 1;
                            return Ok(Value::Array(items));
                        }
                        _ => return Err(self.invalid("after array element")),
                    }
                }
            }
            b'"' => Ok(Value::String(self.string()?)),
            b't' => self.literal(b"true", Value::Bool(true)),
            b'f' => self.literal(b"false", Value::Bool(false)),
            b'n' => self.literal(b"null", Value::Null),
            b'-' | b'0'..=b'9' => self.number(),
            _ => Err(self.invalid("looking for beginning of value")),
        }
    }

    fn literal(&mut self, word: &[u8], v: Value) -> Result<Value, JsonError> {
        let name = std::str::from_utf8(word).unwrap();
        self.i += 1;
        for &w in &word[1..] {
            if self.i >= self.b.len() {
                return Err(self.eof());
            }
            if self.b[self.i] != w {
                return Err(self.invalid(&format!(
                    "in literal {} (expecting {})",
                    name,
                    quote_rune(char::from(w))
                )));
            }
            self.i += 1;
        }
        Ok(v)
    }

    fn number(&mut self) -> Result<Value, JsonError> {
        let start = self.i;
        if self.b[self.i] == b'-' {
            self.i += 1;
            if self.i >= self.b.len() {
                return Err(self.eof());
            }
            if !self.b[self.i].is_ascii_digit() {
                return Err(self.invalid("in numeric literal"));
            }
        }
        if self.b[self.i] == b'0' {
            self.i += 1;
        } else {
            while self.i < self.b.len() && self.b[self.i].is_ascii_digit() {
                self.i += 1;
            }
        }
        if self.i < self.b.len() && self.b[self.i] == b'.' {
            self.i += 1;
            if self.i >= self.b.len() {
                return Err(self.eof());
            }
            if !self.b[self.i].is_ascii_digit() {
                return Err(self.invalid("in numeric literal"));
            }
            while self.i < self.b.len() && self.b[self.i].is_ascii_digit() {
                self.i += 1;
            }
        }
        if self.i < self.b.len() && (self.b[self.i] == b'e' || self.b[self.i] == b'E') {
            self.i += 1;
            if self.i < self.b.len() && (self.b[self.i] == b'+' || self.b[self.i] == b'-') {
                self.i += 1;
            }
            if self.i >= self.b.len() {
                return Err(self.eof());
            }
            if !self.b[self.i].is_ascii_digit() {
                return Err(self.invalid("in numeric literal"));
            }
            while self.i < self.b.len() && self.b[self.i].is_ascii_digit() {
                self.i += 1;
            }
        }
        Ok(Value::Number(
            String::from_utf8_lossy(&self.b[start..self.i]).into_owned(),
        ))
    }

    fn hex4(&self, at: usize) -> Option<u32> {
        if at + 4 > self.b.len() {
            return None;
        }
        let s = std::str::from_utf8(&self.b[at..at + 4]).ok()?;
        u32::from_str_radix(s, 16).ok()
    }

    fn string(&mut self) -> Result<String, JsonError> {
        // at opening quote
        self.i += 1;
        let mut out: Vec<u8> = Vec::new();
        loop {
            if self.i >= self.b.len() {
                return Err(self.eof());
            }
            let c = self.b[self.i];
            match c {
                b'"' => {
                    self.i += 1;
                    break;
                }
                b'\\' => {
                    let esc_start = self.i;
                    self.i += 1;
                    if self.i >= self.b.len() {
                        return Err(self.eof());
                    }
                    let e = self.b[self.i];
                    match e {
                        b'"' | b'\\' | b'/' => {
                            out.push(e);
                            self.i += 1;
                        }
                        b'b' => {
                            out.push(8);
                            self.i += 1;
                        }
                        b'f' => {
                            out.push(12);
                            self.i += 1;
                        }
                        b'n' => {
                            out.push(b'\n');
                            self.i += 1;
                        }
                        b'r' => {
                            out.push(b'\r');
                            self.i += 1;
                        }
                        b't' => {
                            out.push(b'\t');
                            self.i += 1;
                        }
                        b'u' => {
                            self.i += 1;
                            for k in 0..4 {
                                if self.i + k >= self.b.len() {
                                    self.i += k;
                                    return Err(self.eof());
                                }
                                if !self.b[self.i + k].is_ascii_hexdigit() {
                                    return Err(self.bad_escape(esc_start));
                                }
                            }
                            let mut r = self.hex4(self.i).unwrap();
                            self.i += 4;
                            if (0xD800..0xE000).contains(&r) {
                                // try a following \uDC00..DFFF
                                let mut combined = None;
                                if self.i + 6 <= self.b.len()
                                    && self.b[self.i] == b'\\'
                                    && self.b[self.i + 1] == b'u'
                                {
                                    if let Some(r2) = self.hex4(self.i + 2) {
                                        if r < 0xDC00 && (0xDC00..0xE000).contains(&r2) {
                                            combined = Some(
                                                ((r - 0xD800) << 10 | (r2 - 0xDC00)) + 0x10000,
                                            );
                                        }
                                    }
                                }
                                match combined {
                                    Some(c2) => {
                                        r = c2;
                                        self.i += 6;
                                    }
                                    None => r = 0xFFFD,
                                }
                            }
                            let ch = char::from_u32(r).unwrap_or('\u{FFFD}');
                            let mut buf = [0u8; 4];
                            out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                        }
                        _ => return Err(self.bad_escape(esc_start)),
                    }
                }
                c if c < 0x20 => return Err(self.invalid("in string")),
                _ => {
                    out.push(c);
                    self.i += 1;
                }
            }
        }
        // invalid UTF-8 -> U+FFFD per bad byte (Go utf8.DecodeRune semantics)
        Ok(String::from_utf8(out).unwrap_or_else(|e| lossy_go(e.as_bytes())))
    }
}

fn lossy_go(b: &[u8]) -> String {
    let mut s = String::new();
    let mut i = 0;
    while i < b.len() {
        match std::str::from_utf8(&b[i..]) {
            Ok(rest) => {
                s.push_str(rest);
                break;
            }
            Err(e) => {
                let good = e.valid_up_to();
                s.push_str(std::str::from_utf8(&b[i..i + good]).unwrap());
                s.push('\u{FFFD}');
                i += good + 1;
            }
        }
    }
    s
}

/// Parses and validates a complete JSON document (Go `checkValid` + scanner).
pub fn parse(data: &[u8]) -> Result<Value, JsonError> {
    let mut p = Parser { b: data, i: 0 };
    let v = p.value(0)?;
    p.skip_ws();
    if p.i < data.len() {
        return Err(p.invalid("after top-level value"));
    }
    Ok(v)
}

// ---------------------------------------------------------------------------
// Encoder
// ---------------------------------------------------------------------------

/// Compact JSON writer used by [`GoJson::encode_json`].
#[derive(Debug, Default)]
pub struct Encoder {
    pub buf: Vec<u8>,
    pub err: Option<JsonError>,
}

impl Encoder {
    pub fn raw(&mut self, s: &str) {
        self.buf.extend_from_slice(s.as_bytes());
    }

    pub fn null(&mut self) {
        self.raw("null");
    }

    pub fn bool(&mut self, b: bool) {
        self.raw(if b { "true" } else { "false" });
    }

    pub fn int(&mut self, v: i64) {
        self.raw(&v.to_string());
    }

    pub fn uint(&mut self, v: u64) {
        self.raw(&v.to_string());
    }

    /// Go `floatEncoder` (64-bit).
    pub fn float(&mut self, f: f64) {
        if f.is_nan() || f.is_infinite() {
            if self.err.is_none() {
                self.err = Some(JsonError::Other(format!(
                    "json: unsupported value: {}",
                    crate::gofmt::format_float(f, 'g', -1)
                )));
            }
            self.raw("null");
            return;
        }
        self.raw(&format_json_float(f));
    }

    /// Go `appendString` with HTML escaping.
    pub fn string(&mut self, s: &str) {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        self.buf.push(b'"');
        for ch in s.chars() {
            let c = ch as u32;
            if c < 0x80 {
                let b = c as u8;
                match b {
                    b'\\' | b'"' => {
                        self.buf.push(b'\\');
                        self.buf.push(b);
                    }
                    8 => self.buf.extend_from_slice(b"\\b"),
                    12 => self.buf.extend_from_slice(b"\\f"),
                    b'\n' => self.buf.extend_from_slice(b"\\n"),
                    b'\r' => self.buf.extend_from_slice(b"\\r"),
                    b'\t' => self.buf.extend_from_slice(b"\\t"),
                    b'<' | b'>' | b'&' | 0..=0x1f => {
                        self.buf.extend_from_slice(b"\\u00");
                        self.buf.push(HEX[(b >> 4) as usize]);
                        self.buf.push(HEX[(b & 0xF) as usize]);
                    }
                    _ => self.buf.push(b),
                }
            } else if ch == '\u{2028}' || ch == '\u{2029}' {
                self.buf.extend_from_slice(b"\\u202");
                self.buf.push(HEX[(c & 0xF) as usize]);
            } else {
                let mut tmp = [0u8; 4];
                self.buf
                    .extend_from_slice(ch.encode_utf8(&mut tmp).as_bytes());
            }
        }
        self.buf.push(b'"');
    }

    /// Starts an object; use [`Encoder::field`] for each member and [`Encoder::end_object`].
    pub fn begin_object(&mut self) -> ObjState {
        self.buf.push(b'{');
        ObjState { first: true }
    }

    pub fn field(&mut self, st: &mut ObjState, name: &str) {
        if !st.first {
            self.buf.push(b',');
        }
        st.first = false;
        self.string(name);
        self.buf.push(b':');
    }

    pub fn end_object(&mut self) {
        self.buf.push(b'}');
    }
}

/// Per-object comma state for [`Encoder`].
pub struct ObjState {
    first: bool,
}

/// Go `encoding/json` float formatting (`'f'` within [1e-6, 1e21), else `'e'`
/// with the `e-07` -> `e-7` cleanup).
pub fn format_json_float(f: f64) -> String {
    let abs = f.abs();
    if abs != 0.0 && !(1e-6..1e21).contains(&abs) {
        let mut s = crate::gofmt::format_float(f, 'e', -1);
        let n = s.len();
        let b = s.as_bytes();
        if n >= 4 && b[n - 4] == b'e' && b[n - 3] == b'-' && b[n - 2] == b'0' {
            s.remove(n - 2);
        }
        s
    } else {
        crate::gofmt::format_float(f, 'f', -1)
    }
}

/// Go `json.Indent` applied to compact output.
pub fn indent(src: &[u8], prefix: &str, ind: &str) -> Vec<u8> {
    let mut dst = Vec::with_capacity(src.len() * 2);
    let mut need_indent = false;
    let mut depth = 0usize;
    let mut in_str = false;
    let mut esc = false;
    let newline = |dst: &mut Vec<u8>, depth: usize| {
        dst.push(b'\n');
        dst.extend_from_slice(prefix.as_bytes());
        for _ in 0..depth {
            dst.extend_from_slice(ind.as_bytes());
        }
    };
    for &c in src {
        if in_str {
            dst.push(c);
            if esc {
                esc = false;
            } else if c == b'\\' {
                esc = true;
            } else if c == b'"' {
                in_str = false;
            }
            continue;
        }
        if is_space(c) {
            continue;
        }
        if need_indent && c != b'}' && c != b']' {
            need_indent = false;
            depth += 1;
            newline(&mut dst, depth);
        }
        match c {
            b'"' => {
                in_str = true;
                dst.push(c);
            }
            b'{' | b'[' => {
                need_indent = true;
                dst.push(c);
            }
            b',' => {
                dst.push(c);
                newline(&mut dst, depth);
            }
            b':' => {
                dst.push(c);
                dst.push(b' ');
            }
            b'}' | b']' => {
                if need_indent {
                    need_indent = false;
                } else {
                    depth -= 1;
                    newline(&mut dst, depth);
                }
                dst.push(c);
            }
            _ => dst.push(c),
        }
    }
    dst
}

/// Go `json.Marshal(v)`.
pub fn marshal<T: GoJson + ?Sized>(v: &T) -> Result<Vec<u8>, JsonError> {
    let mut e = Encoder::default();
    v.encode_json(&mut e);
    match e.err {
        Some(err) => Err(err),
        None => Ok(e.buf),
    }
}

/// Go `json.MarshalIndent(v, prefix, indent)`.
pub fn marshal_indent<T: GoJson + ?Sized>(
    v: &T,
    prefix: &str,
    ind: &str,
) -> Result<Vec<u8>, JsonError> {
    let b = marshal(v)?;
    Ok(indent(&b, prefix, ind))
}

// ---------------------------------------------------------------------------
// Decoder
// ---------------------------------------------------------------------------

/// Decoding context: the JSON path (Go 1.27 `encoding/json` is implemented
/// on `encoding/json/v2`, whose type errors report the root Go struct name
/// plus the JSON-pointer-like path of *input* keys and array indices) and
/// the saved first error.
#[derive(Debug, Default)]
pub struct Decoder {
    saved: Option<JsonError>,
    fatal: Option<JsonError>,
    root: String,
    path: Vec<String>,
}

impl Decoder {
    /// A decoder whose root value has Go type `root_type` (e.g. `engine.GameState`).
    pub fn new(root_type: &str) -> Self {
        let root = if root_type.starts_with('[')
            || root_type.starts_with('*')
            || root_type.starts_with("map[")
        {
            String::new()
        } else {
            root_type.rsplit('.').next().unwrap_or("").to_string()
        };
        Decoder {
            root,
            ..Default::default()
        }
    }

    /// Records an `UnmarshalTypeError` (only the first error is kept).
    pub fn type_error(&mut self, value: &str, go_type: &str) {
        if self.saved.is_some() {
            return;
        }
        let msg = if self.path.is_empty() {
            format!(
                "json: cannot unmarshal {} into Go value of type {}",
                value, go_type
            )
        } else {
            let last = self.path.last().unwrap();
            let numeric = !last.is_empty() && last.bytes().all(|c| c.is_ascii_digit());
            format!(
                "json: cannot unmarshal {} into {}{}.{} of type {}",
                value,
                if numeric { "" } else { "Go struct field " },
                self.root,
                self.path.join("."),
                go_type
            )
        };
        self.saved = Some(JsonError::Type(msg));
    }

    /// Records a non-type error (e.g. from `UnmarshalJSON`); decoding continues.
    pub fn save_error(&mut self, err: JsonError) {
        if self.saved.is_none() {
            self.saved = Some(err);
        }
    }

    /// Records an error that aborts decoding.
    pub fn fatal(&mut self, err: JsonError) {
        if self.fatal.is_none() {
            self.fatal = Some(err);
        }
    }

    pub fn is_aborted(&self) -> bool {
        self.fatal.is_some()
    }

    /// Enters an object member / array element (`token` = input key or index).
    pub fn push(&mut self, token: &str) {
        self.path.push(token.to_string());
    }

    pub fn pop(&mut self) {
        self.path.pop();
    }
}

/// Go `json.Unmarshal(data, v)`.
pub fn unmarshal<T: GoJson>(data: &[u8], v: &mut T) -> Result<(), JsonError> {
    let val = parse(data)?;
    let mut d = Decoder::new(&T::go_type_name());
    v.decode_json(&val, &mut d);
    if let Some(e) = d.fatal {
        return Err(e);
    }
    match d.saved {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

/// Case-folding used by Go's struct field matching (`foldName`).
pub fn fold_name(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if c.is_ascii() {
            out.push(c.to_ascii_uppercase());
        } else {
            let up = c.to_uppercase().next().unwrap_or(c);
            let low = up.to_lowercase().next().unwrap_or(up);
            let low = match low {
                '\u{212A}' => 'k',
                '\u{017F}' => 's',
                x => x,
            };
            if low.is_ascii() {
                out.push(low.to_ascii_uppercase());
            } else {
                out.push(low);
            }
        }
    }
    out
}

/// Resolves a JSON key to a field index: exact match first, then case-insensitive.
pub fn match_field(names: &[&str], key: &str) -> Option<usize> {
    if let Some(i) = names.iter().position(|n| *n == key) {
        return Some(i);
    }
    let fk = fold_name(key);
    names.iter().position(|n| fold_name(n) == fk)
}

// ---------------------------------------------------------------------------
// GoJson trait + impls for primitives
// ---------------------------------------------------------------------------

/// A type with Go `encoding/json` behaviour.
pub trait GoJson {
    /// Appends the compact JSON encoding.
    fn encode_json(&self, e: &mut Encoder);
    /// Go's `isEmptyValue` (for `omitempty`).
    fn is_empty_json(&self) -> bool {
        false
    }
    /// Decodes `v` into `self` with Go semantics (merging into existing data).
    fn decode_json(&mut self, v: &Value, d: &mut Decoder);
    /// Go type string used in error messages (e.g. `float64`, `engine.Coord`).
    fn go_type_name() -> String
    where
        Self: Sized;
}

fn decode_int_literal(v: &Value, d: &mut Decoder, ty: &str) -> Option<i64> {
    match v {
        Value::Null => None,
        Value::Number(s) => match s.parse::<i64>() {
            Ok(n) => Some(n),
            Err(_) => {
                d.type_error(&format!("number {s}"), ty);
                None
            }
        },
        other => {
            d.type_error(other.kind_name(), ty);
            None
        }
    }
}

macro_rules! int_json {
    ($t:ty, $name:literal) => {
        impl GoJson for $t {
            fn encode_json(&self, e: &mut Encoder) {
                e.int(*self as i64);
            }
            fn is_empty_json(&self) -> bool {
                *self == 0
            }
            fn decode_json(&mut self, v: &Value, d: &mut Decoder) {
                if let Some(n) = decode_int_literal(v, d, $name) {
                    *self = n as $t;
                }
            }
            fn go_type_name() -> String {
                $name.to_string()
            }
        }
    };
}

int_json!(i32, "int");
int_json!(i64, "int64");

impl GoJson for f64 {
    fn encode_json(&self, e: &mut Encoder) {
        e.float(*self);
    }
    fn is_empty_json(&self) -> bool {
        *self == 0.0
    }
    fn decode_json(&mut self, v: &Value, d: &mut Decoder) {
        match v {
            Value::Null => {}
            Value::Number(s) => match s.parse::<f64>() {
                Ok(f) if f.is_finite() => *self = f,
                _ => d.type_error(&format!("number {s}"), "float64"),
            },
            other => d.type_error(other.kind_name(), "float64"),
        }
    }
    fn go_type_name() -> String {
        "float64".into()
    }
}

impl GoJson for bool {
    fn encode_json(&self, e: &mut Encoder) {
        e.bool(*self);
    }
    fn is_empty_json(&self) -> bool {
        !*self
    }
    fn decode_json(&mut self, v: &Value, d: &mut Decoder) {
        match v {
            Value::Null => {}
            Value::Bool(b) => *self = *b,
            other => d.type_error(other.kind_name(), "bool"),
        }
    }
    fn go_type_name() -> String {
        "bool".into()
    }
}

impl GoJson for String {
    fn encode_json(&self, e: &mut Encoder) {
        e.string(self);
    }
    fn is_empty_json(&self) -> bool {
        self.is_empty()
    }
    fn decode_json(&mut self, v: &Value, d: &mut Decoder) {
        match v {
            Value::Null => {}
            Value::String(s) => *self = s.clone(),
            other => d.type_error(other.kind_name(), "string"),
        }
    }
    fn go_type_name() -> String {
        "string".into()
    }
}

/// Go fixed-size arrays `[N]T`. `[2]int` is reported as `engine.Coord`
/// (the only `[2]int` in the engine).
impl<T: GoJson + Default, const N: usize> GoJson for [T; N] {
    fn encode_json(&self, e: &mut Encoder) {
        e.buf.push(b'[');
        for (i, x) in self.iter().enumerate() {
            if i > 0 {
                e.buf.push(b',');
            }
            x.encode_json(e);
        }
        e.buf.push(b']');
    }
    fn is_empty_json(&self) -> bool {
        N == 0
    }
    fn decode_json(&mut self, v: &Value, d: &mut Decoder) {
        match v {
            Value::Null => {}
            Value::Array(items) => {
                for (i, slot) in self.iter_mut().enumerate() {
                    if i < items.len() {
                        d.push(&i.to_string());
                        slot.decode_json(&items[i], d);
                        d.pop();
                        if d.is_aborted() {
                            return;
                        }
                    } else {
                        *slot = T::default();
                    }
                }
            }
            other => d.type_error(other.kind_name(), &Self::go_type_name()),
        }
    }
    fn go_type_name() -> String {
        let inner = T::go_type_name();
        if N == 2 && inner == "int" {
            return "engine.Coord".into();
        }
        format!("[{}]{}", N, inner)
    }
}

/// Go pointer `*T`.
impl<T: GoJson + Default> GoJson for Option<T> {
    fn encode_json(&self, e: &mut Encoder) {
        match self {
            None => e.null(),
            Some(x) => x.encode_json(e),
        }
    }
    fn is_empty_json(&self) -> bool {
        self.is_none()
    }
    fn decode_json(&mut self, v: &Value, d: &mut Decoder) {
        if let Value::Null = v {
            *self = None;
            return;
        }
        if self.is_none() {
            *self = Some(T::default());
        }
        self.as_mut().unwrap().decode_json(v, d);
    }
    fn go_type_name() -> String {
        format!("*{}", T::go_type_name())
    }
}

/// Shared Go pointer target (`*T` that several owners alias).
impl<T: GoJson + Default> GoJson for Rc<RefCell<T>> {
    fn encode_json(&self, e: &mut Encoder) {
        self.borrow().encode_json(e);
    }
    fn decode_json(&mut self, v: &Value, d: &mut Decoder) {
        self.borrow_mut().decode_json(v, d);
    }
    fn go_type_name() -> String {
        T::go_type_name()
    }
}

/// Go slice: remembers the difference between nil (`null`) and empty (`[]`).
#[derive(Debug, Clone, PartialEq)]
pub struct GoVec<T> {
    items: Vec<T>,
    nil: bool,
}

impl<T> Default for GoVec<T> {
    fn default() -> Self {
        GoVec {
            items: Vec::new(),
            nil: true,
        }
    }
}

impl<T> GoVec<T> {
    /// A nil slice.
    pub fn nil() -> Self {
        Self::default()
    }

    /// A non-nil empty slice (`make([]T, 0)` / `[]T{}`).
    pub fn empty() -> Self {
        GoVec {
            items: Vec::new(),
            nil: false,
        }
    }

    /// A non-nil slice with the given elements.
    pub fn from_vec(items: Vec<T>) -> Self {
        GoVec { items, nil: false }
    }

    pub fn is_nil(&self) -> bool {
        self.nil
    }

    /// Go `append(s, x)`.
    pub fn push(&mut self, x: T) {
        self.nil = false;
        self.items.push(x);
    }

    /// Go `append(s[:i], s[i+1:]...)`.
    pub fn remove(&mut self, i: usize) -> T {
        self.nil = false;
        self.items.remove(i)
    }

    /// Go `s = s[:n]`.
    pub fn truncate(&mut self, n: usize) {
        self.items.truncate(n);
    }

    pub fn as_vec(&self) -> &Vec<T> {
        &self.items
    }

    /// Mutable access to the backing vector (marks the slice non-nil if it grows).
    pub fn vec_mut(&mut self) -> &mut Vec<T> {
        self.nil = false;
        &mut self.items
    }

    pub fn into_vec(self) -> Vec<T> {
        self.items
    }
}

impl<T> std::ops::Deref for GoVec<T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        &self.items
    }
}

impl<T> std::ops::DerefMut for GoVec<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        &mut self.items
    }
}

impl<T> From<Vec<T>> for GoVec<T> {
    fn from(v: Vec<T>) -> Self {
        GoVec::from_vec(v)
    }
}

impl<T> FromIterator<T> for GoVec<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut g = GoVec::nil();
        for x in iter {
            g.push(x);
        }
        g
    }
}

impl<'a, T> IntoIterator for &'a GoVec<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}

impl<T: GoJson + Default> GoJson for GoVec<T> {
    fn encode_json(&self, e: &mut Encoder) {
        if self.nil {
            e.null();
            return;
        }
        e.buf.push(b'[');
        for (i, x) in self.items.iter().enumerate() {
            if i > 0 {
                e.buf.push(b',');
            }
            x.encode_json(e);
        }
        e.buf.push(b']');
    }
    fn is_empty_json(&self) -> bool {
        self.items.is_empty()
    }
    fn decode_json(&mut self, v: &Value, d: &mut Decoder) {
        match v {
            Value::Null => {
                *self = GoVec::nil();
            }
            Value::Array(items) => {
                let old = std::mem::take(&mut self.items);
                let mut old = old.into_iter();
                let mut out = Vec::with_capacity(items.len());
                for (i, it) in items.iter().enumerate() {
                    let mut slot = old.next().unwrap_or_default();
                    d.push(&i.to_string());
                    slot.decode_json(it, d);
                    d.pop();
                    out.push(slot);
                    if d.is_aborted() {
                        break;
                    }
                }
                self.items = out;
                self.nil = false;
            }
            other => d.type_error(other.kind_name(), &Self::go_type_name()),
        }
    }
    fn go_type_name() -> String {
        format!("[]{}", T::go_type_name())
    }
}

/// Map keys: Go string-kinded types.
pub trait GoMapKey: Ord + Clone {
    fn key_string(&self) -> String;
    fn from_key(s: &str) -> Self;
}

impl GoMapKey for String {
    fn key_string(&self) -> String {
        self.clone()
    }
    fn from_key(s: &str) -> Self {
        s.to_string()
    }
}

/// Go `map[K]V` (`None` = nil map). Keys are encoded in sorted order.
impl<K: GoMapKey + GoJson, V: GoJson + Default> GoJson for Option<BTreeMap<K, V>> {
    fn encode_json(&self, e: &mut Encoder) {
        match self {
            None => e.null(),
            Some(m) => {
                let mut entries: Vec<(String, &V)> =
                    m.iter().map(|(k, v)| (k.key_string(), v)).collect();
                entries.sort_by(|a, b| a.0.cmp(&b.0));
                let mut st = e.begin_object();
                for (k, v) in entries {
                    e.field(&mut st, &k);
                    v.encode_json(e);
                }
                e.end_object();
            }
        }
    }
    fn is_empty_json(&self) -> bool {
        self.as_ref().is_none_or(|m| m.is_empty())
    }
    fn decode_json(&mut self, v: &Value, d: &mut Decoder) {
        match v {
            Value::Null => *self = None,
            Value::Object(members) => {
                let m = self.get_or_insert_with(BTreeMap::new);
                for (k, val) in members {
                    let mut elem = V::default();
                    d.push(k);
                    elem.decode_json(val, d);
                    d.pop();
                    if d.is_aborted() {
                        return;
                    }
                    m.insert(K::from_key(k), elem);
                }
            }
            other => d.type_error(other.kind_name(), &Self::go_type_name()),
        }
    }
    fn go_type_name() -> String {
        format!("map[{}]{}", K::go_type_name(), V::go_type_name())
    }
}

/// Implements [`GoJson`] for a Go struct. Usage:
///
/// ```ignore
/// go_json_struct!(MyType, "MyType", "pkg.MyType" {
///     field_a => "FieldA",
///     field_b => "field_b" omitempty,
/// });
/// ```
///
/// Fields tagged `json:"-"` are simply not listed.
#[macro_export]
macro_rules! go_json_struct {
    ($ty:ty, $short:literal, $qual:literal { $($field:ident => $name:literal $($omit:ident)?),* $(,)? }) => {
        impl $crate::json::GoJson for $ty {
            #[allow(unused_mut, unused_variables)]
            fn encode_json(&self, e: &mut $crate::json::Encoder) {
                let mut st = e.begin_object();
                $(
                    if !($crate::go_json_struct!(@omit $($omit)?) && $crate::json::GoJson::is_empty_json(&self.$field)) {
                        e.field(&mut st, $name);
                        $crate::json::GoJson::encode_json(&self.$field, e);
                    }
                )*
                let _ = &mut st;
                e.end_object();
            }
            #[allow(unused_mut, unused_variables, unused_assignments)]
            fn decode_json(&mut self, v: &$crate::json::Value, d: &mut $crate::json::Decoder) {
                const NAMES: &[&str] = &[$($name),*];
                match v {
                    $crate::json::Value::Null => {}
                    $crate::json::Value::Object(members) => {
                        for (k, val) in members {
                            if d.is_aborted() {
                                return;
                            }
                            let Some(idx) = $crate::json::match_field(NAMES, k) else { continue };
                            d.push(k);
                            let mut i = 0usize;
                            $(
                                if i == idx {
                                    $crate::json::GoJson::decode_json(&mut self.$field, val, d);
                                }
                                i += 1;
                            )*
                            let _ = i;
                            d.pop();
                        }
                    }
                    other => d.type_error(other.kind_name(), $qual),
                }
            }
            fn go_type_name() -> String {
                $qual.to_string()
            }
        }
    };
    (@omit omitempty) => { true };
    (@omit) => { false };
}

/// Implements [`GoJson`] for a Go named integer type wrapped as a Rust enum
/// with `as_i32` / `from_i32`.
#[macro_export]
macro_rules! go_json_int_enum {
    ($ty:ty, $qual:literal) => {
        impl $crate::json::GoJson for $ty {
            fn encode_json(&self, e: &mut $crate::json::Encoder) {
                e.int(self.as_i32() as i64);
            }
            fn is_empty_json(&self) -> bool {
                self.as_i32() == 0
            }
            fn decode_json(&mut self, v: &$crate::json::Value, d: &mut $crate::json::Decoder) {
                match v {
                    $crate::json::Value::Null => {}
                    $crate::json::Value::Number(s) => match s.parse::<i64>() {
                        Ok(n) => *self = <$ty>::from_i32(n as i32),
                        Err(_) => d.type_error(&format!("number {}", s), $qual),
                    },
                    other => d.type_error(other.kind_name(), $qual),
                }
            }
            fn go_type_name() -> String {
                $qual.to_string()
            }
        }
    };
}

/// Implements [`GoJson`] (and [`GoMapKey`]) for a Go named string type
/// represented as a `String` newtype with field `.0`.
#[macro_export]
macro_rules! go_json_string_newtype {
    ($ty:ty, $qual:literal) => {
        impl $crate::json::GoJson for $ty {
            fn encode_json(&self, e: &mut $crate::json::Encoder) {
                e.string(&self.0);
            }
            fn is_empty_json(&self) -> bool {
                self.0.is_empty()
            }
            fn decode_json(&mut self, v: &$crate::json::Value, d: &mut $crate::json::Decoder) {
                match v {
                    $crate::json::Value::Null => {}
                    $crate::json::Value::String(s) => self.0 = s.clone(),
                    other => d.type_error(other.kind_name(), $qual),
                }
            }
            fn go_type_name() -> String {
                $qual.to_string()
            }
        }
        impl $crate::json::GoMapKey for $ty {
            fn key_string(&self) -> String {
                self.0.clone()
            }
            fn from_key(s: &str) -> Self {
                Self(s.to_string())
            }
        }
    };
}

/// FNV-1a 64-bit hash (used by parity tests to compare JSON states).
pub fn fnv1a64(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floats() {
        assert_eq!(format_json_float(5000.0), "5000");
        assert_eq!(format_json_float(1e-7), "1e-7");
        assert_eq!(format_json_float(1.5e21), "1.5e+21");
        assert_eq!(format_json_float(0.1), "0.1");
        assert_eq!(format_json_float(-0.0), "-0");
        assert_eq!(format_json_float(1e-10), "1e-10");
    }

    #[test]
    fn indenting() {
        let s = br#"{"a":[],"b":{},"c":[1,2],"d":"x,{"}"#;
        let out = String::from_utf8(indent(s, "", "  ")).unwrap();
        assert_eq!(
            out,
            "{\n  \"a\": [],\n  \"b\": {},\n  \"c\": [\n    1,\n    2\n  ],\n  \"d\": \"x,{\"\n}"
        );
    }

    #[test]
    fn syntax_errors() {
        assert_eq!(
            parse(b"").unwrap_err().to_string(),
            "unexpected end of JSON input"
        );
        assert_eq!(
            parse(b"x").unwrap_err().to_string(),
            "invalid character 'x' looking for beginning of value"
        );
        assert_eq!(
            parse(b"{} x").unwrap_err().to_string(),
            "invalid character 'x' after top-level value"
        );
    }
}
