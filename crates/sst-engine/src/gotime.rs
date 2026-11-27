//! Minimal Go `time.Time` model: an instant plus a fixed UTC offset, with
//! Go's JSON (RFC 3339 nano) encoding/decoding and the `Format` layouts the
//! game uses. Monotonic clock readings are not modelled (JSON drops them).

use crate::json::{Decoder, Encoder, GoJson, JsonError, Value};

/// Seconds between 0001-01-01T00:00:00Z and the Unix epoch (Go's zero Time).
const ZERO_UNIX: i64 = -62135596800;

/// Go `time.Time` (instant + zone offset in seconds east of UTC).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GoTime {
    /// Seconds since the Unix epoch.
    pub unix: i64,
    /// Nanoseconds within the second, 0..1e9.
    pub nsec: u32,
    /// Zone offset (seconds east of UTC) used when formatting.
    pub offset: i32,
}

impl Default for GoTime {
    fn default() -> Self {
        GoTime {
            unix: ZERO_UNIX,
            nsec: 0,
            offset: 0,
        }
    }
}

fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let m = m as i64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn days_in_month(y: i64, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ => {
            if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 {
                29
            } else {
                28
            }
        }
    }
}

impl GoTime {
    /// Go `time.Date(y, m, d, hh, mm, ss, nsec, loc)` for a fixed offset.
    #[allow(clippy::too_many_arguments)]
    pub fn date(y: i64, m: u32, d: u32, hh: u32, mm: u32, ss: u32, nsec: u32, offset: i32) -> Self {
        let days = days_from_civil(y, m, d);
        let local = days * 86400 + hh as i64 * 3600 + mm as i64 * 60 + ss as i64;
        GoTime {
            unix: local - offset as i64,
            nsec,
            offset,
        }
    }

    /// Go `time.Unix(sec, nsec)` in the given offset.
    pub fn from_unix(unix: i64, nsec: u32, offset: i32) -> Self {
        GoTime { unix, nsec, offset }
    }

    /// Go `Time.IsZero`.
    pub fn is_zero(&self) -> bool {
        self.unix == ZERO_UNIX && self.nsec == 0
    }

    /// Go `Time.Equal` (same instant).
    pub fn equal(&self, other: &GoTime) -> bool {
        self.unix == other.unix && self.nsec == other.nsec
    }

    /// Go `Time.UnixNano`.
    pub fn unix_nano(&self) -> i64 {
        self.unix
            .wrapping_mul(1_000_000_000)
            .wrapping_add(self.nsec as i64)
    }

    /// Civil fields in the time's own offset: (year, month, day, hour, min, sec).
    pub fn civil(&self) -> (i64, u32, u32, u32, u32, u32) {
        let local = self.unix + self.offset as i64;
        let days = local.div_euclid(86400);
        let secs = local.rem_euclid(86400);
        let (y, m, d) = civil_from_days(days);
        (
            y,
            m,
            d,
            (secs / 3600) as u32,
            ((secs % 3600) / 60) as u32,
            (secs % 60) as u32,
        )
    }

    /// Go `Format("2006-01-02 15:04")`.
    pub fn format_date_minute(&self) -> String {
        let (y, m, d, hh, mm, _) = self.civil();
        format!("{:04}-{:02}-{:02} {:02}:{:02}", y, m, d, hh, mm)
    }

    fn zone_suffix(&self) -> String {
        if self.offset == 0 {
            return "Z".into();
        }
        let (sign, off) = if self.offset < 0 {
            ('-', -self.offset)
        } else {
            ('+', self.offset)
        };
        format!("{}{:02}:{:02}", sign, off / 3600, (off % 3600) / 60)
    }

    /// Go `Format(time.RFC3339Nano)`.
    pub fn format_rfc3339_nano(&self) -> String {
        let (y, m, d, hh, mm, ss) = self.civil();
        let mut s = format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}", y, m, d, hh, mm, ss);
        if self.nsec != 0 {
            let frac = format!("{:09}", self.nsec);
            s.push('.');
            s.push_str(frac.trim_end_matches('0'));
        }
        s.push_str(&self.zone_suffix());
        s
    }

    /// Go `time.Parse(time.RFC3339, s)` (what Go 1.27's JSON decoder uses),
    /// with error messages that describe the expected format.
    pub fn parse_rfc3339(s: &str) -> Result<GoTime, String> {
        let q = crate::gofmt::quote;
        // Go named its layout string ("2006-01-02T15:04:05Z07:00") and
        // layout elements ("2006", "Z07:00"); say what was expected instead.
        let elem_err = |value: &str, elem: &str| {
            let what = match elem {
                "2006" => "a four-digit year",
                "01" => "a two-digit month",
                "02" => "a two-digit day",
                "T" => "the date/time separator \"T\"",
                "15" => "a two-digit hour",
                "04" => "two-digit minutes",
                "05" => "two-digit seconds",
                "Z07:00" => "a time zone (\"Z\" or \"+hh:mm\")",
                "-" | ":" => "a separator",
                _ => "the timestamp",
            };
            format!(
                "invalid timestamp {} (expected RFC 3339, e.g. \"2006-01-02T15:04:05Z\"): cannot read {} as {}",
                q(s),
                q(value),
                what
            )
        };
        let range_err = |what: &str| format!("invalid timestamp {}: {} out of range", q(s), what);
        let is_digit = |v: &str, i: usize| v.as_bytes().get(i).is_some_and(|c| c.is_ascii_digit());
        // getnum(s, fixed)
        let getnum = |v: &str, fixed: bool| -> Option<(u32, usize)> {
            if !is_digit(v, 0) {
                return None;
            }
            if !is_digit(v, 1) {
                if fixed {
                    return None;
                }
                return Some(((v.as_bytes()[0] - b'0') as u32, 1));
            }
            let b = v.as_bytes();
            Some((((b[0] - b'0') * 10 + (b[1] - b'0')) as u32, 2))
        };
        let mut v: &str = s;
        // 2006
        if v.len() < 4 || !is_digit(v, 0) {
            return Err(elem_err(v, "2006"));
        }
        let p = &v[..4];
        let year: i64 = match p.parse::<i64>() {
            Ok(y) if p.bytes().all(|c| c.is_ascii_digit()) => y,
            _ => return Err(elem_err(v, "2006")),
        };
        v = &v[4..];
        let lit = |v: &mut &str, l: &str| -> Result<(), String> {
            if v.starts_with(l) {
                *v = &v[l.len()..];
                Ok(())
            } else {
                Err(elem_err(v, l))
            }
        };
        let num = |v: &mut &str, elem: &str, fixed: bool| -> Result<u32, String> {
            match getnum(v, fixed) {
                Some((n, w)) => {
                    *v = &v[w..];
                    Ok(n)
                }
                None => Err(elem_err(v, elem)),
            }
        };
        lit(&mut v, "-")?;
        let month = num(&mut v, "01", true)?;
        if month == 0 || month > 12 {
            return Err(range_err("month"));
        }
        lit(&mut v, "-")?;
        let day = num(&mut v, "02", true)?;
        if day > 31 {
            return Err(range_err("day"));
        }
        lit(&mut v, "T")?;
        let hour = num(&mut v, "15", false)?;
        if hour >= 24 {
            return Err(range_err("hour"));
        }
        lit(&mut v, ":")?;
        let min = num(&mut v, "04", true)?;
        if min >= 60 {
            return Err(range_err("minute"));
        }
        lit(&mut v, ":")?;
        let sec = num(&mut v, "05", true)?;
        if sec >= 60 {
            return Err(range_err("second"));
        }
        let mut nsec: u32 = 0;
        let vb = v.as_bytes();
        if vb.len() >= 2 && (vb[0] == b'.' || vb[0] == b',') && vb[1].is_ascii_digit() {
            let mut n = 2;
            while n < vb.len() && vb[n].is_ascii_digit() {
                n += 1;
            }
            let digits = &v[1..n];
            let mut val: u64 = 0;
            for (k, c) in digits.bytes().enumerate() {
                if k < 9 {
                    val = val * 10 + (c - b'0') as u64;
                }
            }
            for _ in digits.len()..9 {
                val *= 10;
            }
            nsec = val as u32;
            v = &v[n..];
        }
        // Z07:00
        let offset: i32;
        if v.starts_with('Z') {
            v = &v[1..];
            offset = 0;
        } else {
            let vb = v.as_bytes();
            if vb.len() < 6 || vb[3] != b':' {
                return Err(elem_err(v, "Z07:00"));
            }
            let (Some((hr, _)), Some((mm, _))) = (getnum(&v[1..3], true), getnum(&v[4..6], true))
            else {
                return Err(elem_err(v, "Z07:00"));
            };
            if hr > 24 {
                return Err(range_err("time zone offset hour"));
            }
            if mm > 60 {
                return Err(range_err("time zone offset minute"));
            }
            let o = (hr * 3600 + mm * 60) as i32;
            offset = match vb[0] {
                b'+' => o,
                b'-' => -o,
                _ => return Err(elem_err(v, "Z07:00")),
            };
            v = &v[6..];
        }
        if !v.is_empty() {
            return Err(format!("invalid timestamp {}: extra text: {}", q(s), q(v)));
        }
        if day < 1 || day > days_in_month(year, month) {
            return Err(range_err("day"));
        }
        Ok(GoTime::date(year, month, day, hour, min, sec, nsec, offset))
    }
}

impl GoJson for GoTime {
    fn encode_json(&self, e: &mut Encoder) {
        let (y, ..) = self.civil();
        if !(0..=9999).contains(&y) {
            if e.err.is_none() {
                e.err = Some(JsonError::Other(
                    "json: cannot encode timestamp: year outside of range [0,9999]".into(),
                ));
            }
            e.null();
            return;
        }
        e.string(&self.format_rfc3339_nano());
    }
    fn decode_json(&mut self, v: &Value, d: &mut Decoder) {
        match v {
            Value::Null => {}
            Value::String(s) => match GoTime::parse_rfc3339(s) {
                Ok(t) => *self = t,
                Err(m) => d.save_error(JsonError::Other(m)),
            },
            other => d.type_error(other.kind_name(), "time.Time"),
        }
    }
    fn go_type_name() -> String {
        "time.Time".into()
    }
}

/// Source of the current time (Go `time.Now()`), injectable for tests.
pub trait Clock {
    fn now(&self) -> GoTime;
}

/// Wall clock in the local time zone (UTC on targets without zone info).
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> GoTime {
        let d = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let unix = d.as_secs() as i64;
        GoTime {
            unix,
            nsec: d.subsec_nanos(),
            offset: local_offset(unix),
        }
    }
}

/// A clock that always returns the same instant.
#[derive(Debug, Clone, Copy)]
pub struct FixedClock(pub GoTime);

impl Clock for FixedClock {
    fn now(&self) -> GoTime {
        self.0
    }
}

#[cfg(all(unix, not(target_arch = "wasm32")))]
fn local_offset(unix: i64) -> i32 {
    let t: libc::time_t = unix as libc::time_t;
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    // SAFETY: localtime_r writes into the provided tm and reads t.
    let r = unsafe { libc::localtime_r(&t, &mut tm) };
    if r.is_null() {
        return 0;
    }
    tm.tm_gmtoff as i32
}

#[cfg(not(all(unix, not(target_arch = "wasm32"))))]
fn local_offset(_unix: i64) -> i32 {
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc3339() {
        let t = GoTime::date(2265, 1, 1, 0, 0, 0, 0, 0);
        assert_eq!(t.format_rfc3339_nano(), "2265-01-01T00:00:00Z");
        assert_eq!(
            GoTime::default().format_rfc3339_nano(),
            "0001-01-01T00:00:00Z"
        );
        assert!(GoTime::default().is_zero());
        let p = GoTime::parse_rfc3339("2024-02-29T13:45:01.5-08:00").unwrap();
        assert_eq!(p.format_rfc3339_nano(), "2024-02-29T13:45:01.5-08:00");
        assert_eq!(p.format_date_minute(), "2024-02-29 13:45");
    }
}
