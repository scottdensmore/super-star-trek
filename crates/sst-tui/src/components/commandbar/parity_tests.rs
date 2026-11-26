//! Byte-for-byte parity with `testdata/parity/components/commandbar.txt`,
//! recorded by `pkg/tui/components/commandbar/zz_paritygen_test.go`.

#![allow(clippy::needless_range_loop, clippy::type_complexity)]

#[allow(clippy::duplicate_mod)]
#[path = "../parity_support.rs"]
mod pg;

use sst_engine::gofmt::quote;
use sst_tuikit::tea::{
    msg, MouseMsg, WindowSizeMsg, KEY_RUNES, MOUSE_ACTION_PRESS, MOUSE_BUTTON_LEFT,
};

use super::*;

const DATA: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testdata/parity/components/commandbar.txt"
));

fn quote_list(v: &[String]) -> String {
    format!(
        "[{}]",
        v.iter().map(|s| quote(s)).collect::<Vec<_>>().join(" ")
    )
}

fn state(m: &Model) -> String {
    format!(
        "val={} pos={} focused={} width={}/{} bell={} msgs={}",
        quote(&m.value()),
        m.position(),
        m.focused(),
        m.width(),
        m.model.width,
        m.visual_bell(),
        quote_list(&m.messages())
    )
}

fn cmd_desc(c: &Option<Cmd>) -> String {
    if c.is_some() { "cmd" } else { "nil" }.into()
}

fn step(m: &mut Model, act: &str) -> String {
    if let Some(t) = act.strip_prefix("@add:") {
        m.add_message(t);
        return "-".into();
    }
    if let Some(n) = act.strip_prefix("@width:") {
        m.set_width(n.parse().unwrap());
        return "-".into();
    }
    if let Some(t) = act.strip_prefix("@set:") {
        m.set_value(t);
        return "-".into();
    }
    if let Some(t) = act.strip_prefix("@paste:") {
        let k = KeyMsg {
            typ: KEY_RUNES,
            runes: t.chars().collect(),
            paste: true,
            ..Default::default()
        };
        return cmd_desc(&m.update(&msg(k)));
    }
    match act {
        "@clear" => {
            m.clear_messages();
            "-".into()
        }
        "@bell:1" => {
            m.set_visual_bell(true);
            "-".into()
        }
        "@bell:0" => {
            m.set_visual_bell(false);
            "-".into()
        }
        "@blur" => {
            m.blur();
            "-".into()
        }
        "@focus" => cmd_desc(&m.focus()),
        "@reset" => {
            m.reset();
            "-".into()
        }
        "@winsize" => cmd_desc(&m.update(&msg(WindowSizeMsg {
            width: 100,
            height: 40,
        }))),
        "@mouse" => cmd_desc(&m.update(&msg(MouseMsg {
            x: 3,
            y: 4,
            button: MOUSE_BUTTON_LEFT,
            action: MOUSE_ACTION_PRESS,
            ..Default::default()
        }))),
        _ => {
            let c = m.update(&pg::key_msg(act));
            match c {
                None => "nil".into(),
                Some(c) if act == "enter" => match c.call() {
                    Some(r) => match r.downcast_ref::<CommandSubmittedMsg>() {
                        Some(sub) => format!("submit {}", quote(&sub.text)),
                        None => "?".into(),
                    },
                    None => "?".into(),
                },
                Some(_) => "cmd".into(),
            }
        }
    }
}

fn generate() -> Vec<String> {
    let mut o = pg::Out::new();

    let long = format!("@add:{}", "long message ".repeat(12));
    let sessions: Vec<Vec<&str>> = vec![
        vec![
            "n",
            "a",
            "v",
            " ",
            "1",
            " ",
            "2",
            "enter",
            "t",
            "o",
            "r",
            "enter",
            "s",
            "t",
            "a",
            "t",
            "u",
            "s",
            "enter",
            "@set:status",
            "enter",
            "@set:  status  ",
            "enter",
            "p",
            "h",
            "up",
            "up",
            "up",
            "up",
            "down",
            "down",
            "down",
            "down",
            "up",
            "x",
            "down",
            "enter",
            "enter",
            "@set:   ",
            "enter",
            "down",
            "up",
            "up",
        ],
        vec![
            "@set:s",
            "tab",
            "tab",
            "tab",
            "tab",
            "tab",
            "x",
            "@set:doc",
            "tab",
            "tab",
            "@set:xyz",
            "tab",
            "@set:",
            "tab",
            "@set:  T ",
            "tab",
            "tab",
            "tab",
            "tab",
            "tab",
            "left",
            "tab",
            "@set:pha ",
            "tab",
            "tab",
            "@set:N",
            "tab",
            "up",
            "tab",
            "@set:she",
            "tab",
            "enter",
            "@set:s",
            "tab",
            "down",
            "tab",
        ],
        vec![
            "@width:30",
            "a",
            "b",
            "c",
            "d",
            "e",
            "f",
            "g",
            "h",
            "i",
            "j",
            "k",
            "l",
            "m",
            "n",
            "o",
            "p",
            "q",
            "r",
            "s",
            "t",
            "u",
            "v",
            "w",
            "x",
            "y",
            "z",
            "0",
            "1",
            "2",
            "3",
            "home",
            "right",
            "right",
            "delete",
            "end",
            "left",
            "left",
            "backspace",
            "ctrl+a",
            "ctrl+k",
            "@set:one two three",
            "ctrl+w",
            "alt+b",
            "ctrl+u",
            "★",
            "é",
            "@paste:pasted text",
            "ctrl+e",
            "space",
            "f2",
            "@winsize",
            "@mouse",
            "enter",
        ],
        vec![
            "@add:Enterprise systems online.",
            "@add:Sensors detect 3 Klingons.\nShields up.",
            "@add:\r\n",
            "@add:CRLF line\r\n",
            "@add:fifth\nsixth\nseventh",
            "@width:80",
            "@bell:1",
            "@width:0",
            "@width:3",
            "@width:4",
            "@width:5",
            "@width:-2",
            "@width:120",
            "@bell:0",
            "@blur",
            "a",
            "@focus",
            "b",
            &long,
            "@width:40",
            "@clear",
            "@add:after clear",
            "@reset",
            "@blur",
            "@bell:1",
        ],
    ];
    for (si, sess) in sessions.iter().enumerate() {
        let mut m = new(Some(pg::variants()[0].1));
        for (i, act) in sess.iter().enumerate() {
            let res = step(&mut m, act);
            let key = format!("s{si}/{i:02}");
            o.w(
                &format!("{key}/state"),
                &format!("act={} cmd={res} {}", quote(act), state(&m)),
            );
            o.rot(&format!("{key}/view"), |th| {
                m.set_theme(Some(th));
                m.view()
            });
        }
        o.each(&format!("s{si}/final"), |th| {
            m.set_theme(Some(th));
            m.view()
        });
    }

    for (k, th) in pg::variants() {
        let m = new(Some(th));
        let a = new_command_bar(Some(th));
        o.w(
            &format!("new/{k}"),
            &format!(
                "{} {} prompt={} alias={}",
                m.theme().name(),
                state(&m),
                quote(&m.prompt),
                quote(&a.prompt)
            ),
        );
    }
    o.each("new/view", |th| {
        let mut m = new(Some(th));
        m.add_message("Super Star Trek — Tactical Display Online");
        m.add_message("Type 'help' for commands, 'quit' to exit, F2 to cycle themes");
        m.set_width(80);
        m.view()
    });
    o.lines
}

#[test]
fn parity_commandbar() {
    pg::check(DATA, &generate());
}
