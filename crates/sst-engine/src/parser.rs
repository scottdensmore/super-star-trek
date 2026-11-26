//! Player command parser (port of `pkg/tui/parser/parser.go`), shared by
//! the TUI and the WASM bridge.

use crate::actions::*;
use crate::gofmt::{atoi, parse_float, quote};
use crate::tactics::ActionPhaserDirect;
use crate::types::Coord;

/// Go `parser.ParsedCommand`: an action, a special TUI command, or an error.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ParsedCommand {
    pub action: Option<Action>,
    pub special: String,
    pub error: Option<EngineError>,
}

fn err(s: impl Into<String>) -> ParsedCommand {
    ParsedCommand {
        error: Some(EngineError(s.into())),
        ..Default::default()
    }
}

fn act(a: impl Into<Action>) -> ParsedCommand {
    ParsedCommand {
        action: Some(a.into()),
        ..Default::default()
    }
}

fn special(s: impl Into<String>) -> ParsedCommand {
    ParsedCommand {
        special: s.into(),
        ..Default::default()
    }
}

/// Go `strconv.Atoi` result restricted to 1..=8 checks done by callers;
/// values outside `i32` are mapped to 0 (always out of range).
fn atoi32(s: &str) -> Option<i32> {
    atoi(s).ok().map(|v| {
        if v < i32::MIN as i64 || v > i32::MAX as i64 {
            0
        } else {
            v as i32
        }
    })
}

/// Go `strings.Fields` (splits on Unicode white space).
pub fn fields(s: &str) -> Vec<&str> {
    s.split(|c: char| is_go_space(c))
        .filter(|t| !t.is_empty())
        .collect()
}

/// Go `unicode.IsSpace`.
pub fn is_go_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{85}' | '\u{a0}'
    ) || (c as u32 > 0xff && c.is_whitespace())
}

/// Go `strings.TrimSpace`.
pub fn trim_space(s: &str) -> &str {
    s.trim_matches(is_go_space)
}

/// Go `ParseCommand`.
pub fn parse_command(input: &str) -> ParsedCommand {
    let trimmed = trim_space(input);
    if trimmed.is_empty() {
        return err("no command entered");
    }
    let tokens = fields(trimmed);
    let cmd = tokens[0].to_lowercase();
    let args = &tokens[1..];
    match cmd.as_str() {
        "nav" | "move" | "warp" => parse_nav(args),
        "tor" | "torpedo" => parse_torpedo(args),
        "pha" | "phaser" | "phasers" => parse_phasers(args),
        "she" | "shield" | "shields" | "def" => parse_shields(args),
        "doc" | "dock" => act(ActionDock),
        "opts" | "options" | "settings" => special("options"),
        "scenarios" | "scen" => special("scenarios"),
        "tour" | "orders" => special(cmd.clone()),
        "theme" => {
            if !args.is_empty() {
                special(format!("theme {}", args[0].to_lowercase()))
            } else {
                special("theme")
            }
        }
        "help" | "?" | "commands" => parse_help(args),
        "quit" | "exit" | "q" => special("quit"),
        _ => err(format!(
            "unknown command: {} (type 'help' for commands)",
            quote(tokens[0])
        )),
    }
}

fn parse_nav(args: &[&str]) -> ParsedCommand {
    let usage = "usage: nav q <r> <c> (quadrant), nav s <r> <c> (sector), or nav <course> <warp>. Type 'help nav' for guide";
    if args.len() < 2 {
        return err(usage);
    }
    if args.len() == 3 || args.len() == 4 {
        let prefix = args[0].to_lowercase();
        if prefix == "q" || prefix == "quad" || prefix == "quadrant" {
            let (r, c) = (atoi32(args[1]), atoi32(args[2]));
            let (Some(r), Some(c)) = (r, c) else {
                return err("invalid quadrant parameters: expected integer row and column");
            };
            if !(1..=8).contains(&r) || !(1..=8).contains(&c) {
                return err("quadrant coordinates must be between 1 and 8");
            }
            let mut warp = 1.0;
            if args.len() == 4 {
                match parse_float(args[3]) {
                    Ok(w) if w > 0.0 => warp = w,
                    _ => return err("warp factor must be positive"),
                }
            }
            return act(ActionMove {
                dest_quad: [r, c],
                warp,
                ..Default::default()
            });
        }
        if args.len() == 3 {
            if prefix == "s" || prefix == "sec" || prefix == "sector" {
                let (Some(r), Some(c)) = (atoi32(args[1]), atoi32(args[2])) else {
                    return err("invalid sector parameters: expected integer row and column");
                };
                if !(1..=8).contains(&r) || !(1..=8).contains(&c) {
                    return err("sector coordinates must be between 1 and 8");
                }
                return act(ActionMove {
                    dest_sector: [r, c],
                    warp: 1.0,
                    ..Default::default()
                });
            }
            if prefix == "c" || prefix == "course" {
                let (Ok(course), Ok(warp)) = (parse_float(args[1]), parse_float(args[2])) else {
                    return err("invalid course/warp parameters: expected numbers");
                };
                if warp <= 0.0 {
                    return err("warp factor must be positive");
                }
                return act(ActionMove {
                    course,
                    warp,
                    ..Default::default()
                });
            }
        }
        return err(usage);
    }

    let has_dot = args[0].contains('.') || args[1].contains('.');
    if !has_dot {
        if let (Some(r), Some(c)) = (atoi32(args[0]), atoi32(args[1])) {
            if (1..=8).contains(&r) && (1..=8).contains(&c) {
                return act(ActionMove {
                    dest_sector: [r, c],
                    warp: 1.0,
                    ..Default::default()
                });
            }
        }
    }
    let (Ok(course), Ok(warp)) = (parse_float(args[0]), parse_float(args[1])) else {
        return err("invalid navigation parameters: expected numbers");
    };
    if warp <= 0.0 {
        return err("warp factor must be positive");
    }
    act(ActionMove {
        course,
        warp,
        ..Default::default()
    })
}

fn parse_help(args: &[&str]) -> ParsedCommand {
    if args.is_empty() {
        return special("help");
    }
    let topic = args[0].to_lowercase();
    let s = match topic.as_str() {
        "nav" | "move" | "warp" => "help nav".to_string(),
        "tor" | "torpedo" | "torpedoes" | "target" | "reticle" => "help tor".into(),
        "pha" | "phaser" | "phasers" => "help pha".into(),
        "she" | "shield" | "shields" | "def" => "help she".into(),
        "doc" | "dock" => "help doc".into(),
        "chart" | "map" => "help chart".into(),
        "saves" | "thaw" | "freeze" => "help saves".into(),
        "opts" | "options" | "settings" => "help options".into(),
        _ => format!("help {}", topic),
    };
    special(s)
}

fn parse_torpedo(args: &[&str]) -> ParsedCommand {
    if args.len() == 1 {
        return match parse_float(args[0]) {
            Ok(angle) => act(ActionFireTorpedo {
                angle,
                ..Default::default()
            }),
            Err(_) => err(format!("invalid torpedo course: {}", args[0])),
        };
    }
    if args.len() == 2 {
        let prefix = args[0].to_lowercase();
        if prefix == "c" || prefix == "course" {
            return match parse_float(args[1]) {
                Ok(dir) if !(dir < 1.0 || dir > 9.0) => act(ActionFireTorpedo {
                    direction: dir,
                    ..Default::default()
                }),
                _ => err("torpedo course must be between 1.0 and 9.0 (1=E, 3=N, 5=W, 7=S)"),
            };
        }
        let (Some(r), Some(c)) = (atoi32(args[0]), atoi32(args[1])) else {
            return err(format!("invalid torpedo target: {} {}", args[0], args[1]));
        };
        if !(1..=8).contains(&r) || !(1..=8).contains(&c) {
            return err("torpedo target sector coordinates must be between 1 and 8");
        }
        let target: Coord = [r, c];
        return act(ActionFireTorpedo {
            target,
            ..Default::default()
        });
    }
    err("usage: tor <course> or tor <row> <col>")
}

fn parse_energy(s: &str) -> Result<f64, ParsedCommand> {
    match parse_float(s) {
        Err(_) => Err(err(format!("invalid phaser energy: {}", s))),
        Ok(e) if e <= 0.0 => Err(err("phaser energy must be positive")),
        Ok(e) => Ok(e),
    }
}

fn parse_phasers(args: &[&str]) -> ParsedCommand {
    let coord_err = "phaser target sector coordinates must be between 1 and 8";
    let usage = "usage: pha <energy> or pha <energy> <row> <col>";
    match args.len() {
        1 => match parse_energy(args[0]) {
            Ok(energy) => act(ActionFirePhasers {
                energy,
                ..Default::default()
            }),
            Err(e) => e,
        },
        2 => {
            let energy = match parse_energy(args[0]) {
                Ok(e) => e,
                Err(e) => return e,
            };
            let parts: Vec<&str> = args[1].split(',').collect();
            if parts.len() == 2 {
                let r = atoi32(trim_space(parts[0]));
                let c = atoi32(trim_space(parts[1]));
                match (r, c) {
                    (Some(r), Some(c)) if (1..=8).contains(&r) && (1..=8).contains(&c) => {
                        return act(ActionPhaserDirect {
                            energy,
                            target_sector: [r, c],
                        })
                    }
                    _ => return err(coord_err),
                }
            }
            err(usage)
        }
        3 => {
            let energy = match parse_energy(args[0]) {
                Ok(e) => e,
                Err(e) => return e,
            };
            match (atoi32(args[1]), atoi32(args[2])) {
                (Some(r), Some(c)) if (1..=8).contains(&r) && (1..=8).contains(&c) => {
                    act(ActionPhaserDirect {
                        energy,
                        target_sector: [r, c],
                    })
                }
                _ => err(coord_err),
            }
        }
        _ => err(usage),
    }
}

fn parse_shields(args: &[&str]) -> ParsedCommand {
    if args.len() != 1 {
        return err("usage: she <amount>");
    }
    match parse_float(args[0]) {
        Ok(amount) => act(ActionShields { amount }),
        Err(_) => err(format!("invalid shield amount: {}", args[0])),
    }
}

impl ParsedCommand {
    /// `{"Action":...,"Special":"...","Error":null|"..."}` (parity transcript format).
    pub fn to_json(&self) -> String {
        let mut e = crate::json::Encoder::default();
        e.raw("{\"Action\":");
        match &self.action {
            Some(a) => e.raw(&a.to_json()),
            None => e.null(),
        }
        e.raw(",\"Special\":");
        e.string(&self.special);
        e.raw(",\"Error\":");
        match &self.error {
            Some(er) => e.string(&er.0),
            None => e.null(),
        }
        e.raw("}");
        String::from_utf8(e.buf).expect("utf-8")
    }
}
