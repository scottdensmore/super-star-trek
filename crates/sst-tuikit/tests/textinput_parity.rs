//! bubbles/textinput parity (`testdata/parity/tuikit/textinput.txt`): views
//! (Ascii and TrueColor), value, cursor position, blink state and
//! suggestions after every step of key sequences, for the configurations the
//! game uses (command bar, hall-of-fame callsign) and variations.

mod common;

use std::time::Duration;

use common::{get, load, Checker};
use sst_tuikit::bubbles::{cursor, textinput};
use sst_tuikit::lipgloss;
use sst_tuikit::tea::{
    self, BatchMsg, BlurMsg, Cmd, FocusMsg, KeyMsg, KeyType, Msg, KEY_RUNES, KEY_SPACE,
};
use sst_tuikit::termenv::Profile;

fn ti_config(name: &str) -> textinput::Model {
    let mut ti = textinput::new();
    ti.cursor.blink_speed = Duration::from_millis(1);
    let bold_cyan = || {
        lipgloss::new_style()
            .bold(true)
            .foreground(lipgloss::color("#00E5FF"))
    };
    let text = || lipgloss::new_style().foreground(lipgloss::color("#E2E8F0"));
    match name {
        "default" => {}
        "commandbar" => {
            ti.prompt = "COMMAND> ".into();
            ti.prompt_style = bold_cyan();
            ti.text_style = text();
        }
        "commandbar_w40" => {
            ti.prompt = "COMMAND> ".into();
            ti.prompt_style = bold_cyan();
            ti.text_style = text();
            ti.width = 40;
        }
        "narrow" => {
            ti.prompt = "> ".into();
            ti.width = 8;
        }
        "halloffame" => {
            ti.prompt = String::new();
            ti.placeholder = "Enter callsign...".into();
            ti.char_limit = 20;
            ti.width = 20;
            ti.prompt_style = lipgloss::new_style()
                .bold(true)
                .foreground(lipgloss::color("#FFD700"));
            ti.text_style = text();
        }
        "placeholder_nowidth" => ti.placeholder = "Type a command".into(),
        "placeholder_short" => ti.placeholder = "x".into(),
        "password" => {
            ti.echo_mode = textinput::EchoMode::EchoPassword;
            ti.width = 12;
        }
        "echonone" => ti.echo_mode = textinput::EchoMode::EchoNone,
        "limit5" => ti.char_limit = 5,
        "suggest" => {
            ti.show_suggestions = true;
            ti.set_suggestions(&["navigate", "nav", "phasers", "photon", "status"]);
        }
        "static" => {
            let _ = ti.set_cursor_mode(cursor::Mode::CursorStatic);
        }
        "hidden" => {
            let _ = ti.set_cursor_mode(cursor::Mode::CursorHide);
        }
        "wide" => {
            ti.width = 6;
            ti.prompt = "★ ".into();
        }
        other => panic!("config {other}"),
    }
    ti
}

fn run_cmds(cmds: Vec<Option<Cmd>>) -> Vec<Msg> {
    let mut out = Vec::new();
    for c in cmds.into_iter().flatten() {
        let Some(m) = c.call() else { continue };
        match m.downcast::<BatchMsg>() {
            Ok(b) => out.extend(run_cmds(b.0.into_iter().map(Some).collect())),
            Err(m) => out.push(m),
        }
    }
    out
}

fn step_msg(step: &str) -> Msg {
    let (kind, arg) = step.split_once(':').unwrap_or((step, ""));
    match kind {
        "r" => tea::msg(KeyMsg::runes(arg)),
        "ra" => tea::msg(KeyMsg {
            alt: true,
            ..KeyMsg::runes(arg)
        }),
        "p" => tea::msg(KeyMsg {
            paste: true,
            ..KeyMsg::runes(arg)
        }),
        "k" | "ka" => tea::msg(KeyMsg {
            typ: KeyType(arg.parse().unwrap()),
            alt: kind == "ka",
            ..Default::default()
        }),
        "sp" => tea::msg(KeyMsg {
            typ: KEY_SPACE,
            runes: vec![' '],
            ..Default::default()
        }),
        "focusmsg" => tea::msg(FocusMsg),
        "blurmsg" => tea::msg(BlurMsg),
        _ => panic!("step {step}"),
    }
}

fn views(ti: &textinput::Model) -> (String, String) {
    lipgloss::set_color_profile(Profile::Ascii);
    let a = ti.view();
    lipgloss::set_color_profile(Profile::TrueColor);
    let t = ti.view();
    lipgloss::set_color_profile(Profile::Ascii);
    (a, t)
}

#[test]
fn textinput_parity() {
    lipgloss::set_has_dark_background(true);
    let recs = load("textinput.txt");
    let mut c = Checker::default();
    let _ = KEY_RUNES;
    for r in &recs {
        let cfg = get(r, "config");
        let steps = get(r, "steps");
        let mut ti = ti_config(cfg);
        let mut pending: Vec<Option<Cmd>> = Vec::new();
        let (a, t) = views(&ti);
        c.check(|| format!("{cfg} init"), &a, get(r, "init_a"));
        c.check(|| format!("{cfg} init tc"), &t, get(r, "init_t"));
        for (i, step) in steps.split('|').enumerate() {
            let (kind, arg) = step.split_once(':').unwrap_or((step, ""));
            match kind {
                "focus" => pending.push(ti.focus()),
                "blur" => ti.blur(),
                "blink" => {
                    let msgs = run_cmds(std::mem::take(&mut pending));
                    for m in msgs {
                        pending.push(ti.update(&m));
                    }
                }
                "setvalue" => ti.set_value(arg),
                "cursorstart" => ti.cursor_start(),
                "setcursor" => ti.set_cursor(arg.parse().unwrap()),
                "reset" => ti.reset(),
                _ => {
                    let m = step_msg(step);
                    pending.push(ti.update(&m));
                }
            }
            let (a, t) = views(&ti);
            let p = i.to_string();
            let what = || format!("{cfg} steps={steps:?} step {i} ({step:?})");
            c.check(what, &a, get(r, &format!("a{p}")));
            c.check(what, &t, get(r, &format!("t{p}")));
            c.check(what, &ti.value(), get(r, &format!("val{p}")));
            c.check(what, &ti.position().to_string(), get(r, &format!("pos{p}")));
            c.check(
                what,
                &ti.cursor.blink.to_string(),
                get(r, &format!("blink{p}")),
            );
            c.check(
                what,
                &ti.focused().to_string(),
                get(r, &format!("focused{p}")),
            );
            let sug = format!(
                "{}#{}",
                ti.matched_suggestions().join(","),
                ti.current_suggestion_index()
            );
            c.check(what, &sug, get(r, &format!("sug{p}")));
        }
    }
    c.finish("textinput");
}
