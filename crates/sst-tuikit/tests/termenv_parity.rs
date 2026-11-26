//! termenv environment detection parity (`testdata/parity/tuikit/termenv.txt`):
//! ColorProfile / EnvColorProfile / EnvNoColor for TERM x COLORTERM x
//! NO_COLOR/CLICOLOR/CLICOLOR_FORCE/CI/... x TTY, and the COLORFGBG
//! background fallback behind HasDarkBackground.

mod common;

use std::collections::HashMap;

use common::{get, load, Checker};
use sst_tuikit::termenv::{Color, Output};

fn parse_env(s: &str) -> HashMap<String, String> {
    s.split(';')
        .filter(|kv| !kv.is_empty())
        .filter_map(|kv| kv.split_once('='))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn seq(c: &Color, bg: bool) -> String {
    let t = match c {
        Color::NoColor => "termenv.NoColor",
        Color::ANSI(_) => "termenv.ANSIColor",
        Color::ANSI256(_) => "termenv.ANSI256Color",
        Color::RGB(_) => "termenv.RGBColor",
    };
    format!("{t}:{}", c.sequence(bg))
}

#[test]
fn termenv_parity() {
    let recs = load("termenv.txt");
    let mut c = Checker::default();
    for r in &recs {
        // COLORFGBG values like "15;default;0" contain ';': rebuild from the
        // known keys instead of splitting blindly.
        let raw = get(r, "env");
        let mut env = parse_env(raw);
        if let Some(i) = raw.find("COLORFGBG=") {
            let rest = &raw[i + "COLORFGBG=".len()..];
            let end = rest.find(";TERM=").unwrap_or(rest.len());
            env.insert("COLORFGBG".into(), rest[..end].to_string());
        }
        let getenv = move |k: &str| env.get(k).cloned().unwrap_or_default();
        let tty = get(r, "tty") == "true";
        let o = Output::new(&getenv, None).with_tty(tty);
        match get(r, "op") {
            "profile" => {
                c.check(
                    || format!("{r:?}"),
                    &(o.color_profile() as i32).to_string(),
                    get(r, "profile"),
                );
                c.check(
                    || format!("{r:?}"),
                    &(o.env_color_profile() as i32).to_string(),
                    get(r, "envprofile"),
                );
                c.check(
                    || format!("{r:?}"),
                    &o.env_no_color().to_string(),
                    get(r, "nocolor"),
                );
            }
            "background" => {
                c.check(
                    || format!("{r:?}"),
                    &seq(&o.background_color(), true),
                    get(r, "bg"),
                );
                c.check(
                    || format!("{r:?}"),
                    &o.has_dark_background().to_string(),
                    get(r, "dark"),
                );
            }
            other => panic!("unknown op {other}"),
        }
    }
    c.finish("termenv");
}
