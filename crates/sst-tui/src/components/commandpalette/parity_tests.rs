//! Byte-for-byte parity with `testdata/parity/components/commandpalette.txt`
//! (recorded by `pkg/tui/components/commandpalette/zz_paritygen_test.go`).

#[path = "../modal_parity_util.rs"]
#[allow(clippy::duplicate_mod)]
mod util;

use sst_tuikit::termenv::Profile;
use sst_tuikit::{lipgloss, tea::Msg};
use util::*;

use super::*;
use crate::theme::{get_theme, ColorMode};

const DATA: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testdata/parity/components/commandpalette.txt"
));

const SEQUENCES: &[&[&str]] = &[
    &[
        "down", "down", "down", "up", "pgdown", "pgdown", "pgup", "end", "home", "enter", "esc",
    ],
    &[
        "p",
        "h",
        "a",
        "enter",
        "backspace",
        "backspace",
        "backspace",
        "backspace",
    ],
    &[
        "n",
        "a",
        "v",
        "space",
        "q",
        "backspace",
        "delete",
        "down",
        "enter",
    ],
    &["t", "h", "e", "m", "e", "end", "up", "enter"],
    &["z", "z", "z", "z", "enter", "down", "end", "home"],
    &["🚀", "é", "漢", "backspace", "backspace", "backspace"],
    &[
        "tab",
        "ctrl+x",
        "f1",
        "alt+a",
        "winsize",
        "shift+tab",
        "~",
        "/",
        "?",
    ],
    &["s", "c", "a", "n", "pgdown", "pgup"],
];

const SIZES: &[(i32, i32)] = &[
    (56, 16),
    (60, 20),
    (40, 10),
    (0, 0),
    (80, 30),
    (20, 5),
    (12, 3),
];

fn desc(m: &Msg) -> Option<String> {
    if let Some(v) = m.downcast_ref::<CommandSelectedMsg>() {
        return Some(format!(
            "CommandSelectedMsg prefix={} param={}",
            sst_engine::gofmt::quote(&v.command_prefix),
            v.parameterized
        ));
    }
    if m.downcast_ref::<ClosePaletteMsg>().is_some() {
        return Some("ClosePaletteMsg".into());
    }
    None
}

fn titles(items: &[PaletteItem]) -> String {
    items
        .iter()
        .map(|i| i.title())
        .collect::<Vec<_>>()
        .join("|")
}

fn state(o: &mut Out, p: &str, m: &Model) {
    o.w(&format!("{p}view"), &m.view());
    o.w(&format!("{p}filter"), &m.filter_value());
    o.w(&format!("{p}visible"), &titles(&m.visible_items()));
    let sel = m
        .selected_item()
        .map(|s| s.title())
        .unwrap_or_else(|| "nil".into());
    o.w(&format!("{p}selected"), &sel);
}

fn generate() -> Vec<String> {
    let mut o = Out::default();
    lipgloss::set_has_dark_background(true);
    lipgloss::set_color_profile(Profile::Ascii);
    {
        let m = new(get_theme("modern").with_color_mode(ColorMode::Dark), 56, 16);
        for (i, it) in m.items().iter().enumerate() {
            o.w(
                &format!("catalog/{i}"),
                &format!(
                    "{}|{}|{}|{}|{}",
                    it.title(),
                    it.description(),
                    it.filter_value(),
                    it.prefix(),
                    it.parameterized()
                ),
            );
        }
    }
    for c in combos() {
        let (th, pre) = c.apply();
        let full = c.full();
        for (zi, (w, h)) in SIZES.iter().enumerate() {
            if !full && zi > 1 {
                break;
            }
            let m = new(th, *w, *h);
            o.w(&format!("{pre}size/{w}x{h}"), &m.view());
        }
        for (si, seq) in SEQUENCES.iter().enumerate() {
            let mut m = new(th, 56, 16);
            for (ki, k) in seq.iter().enumerate() {
                let cmd = m.update(&key(k));
                let p = format!("{pre}seq{si}/{ki}/{k}/");
                o.w(&format!("{p}msg"), &describe_cmd(cmd, &desc));
                if full || (ki == seq.len() - 1 && si % 2 == 0) {
                    state(&mut o, &p, &m);
                }
            }
        }
        let mut m = new(th, 56, 16);
        for k in ["d", "o", "c"] {
            let _ = m.update(&key(k));
        }
        m.set_size(70, 12);
        state(&mut o, &format!("{pre}resized/"), &m);
        m.set_theme(get_theme("crt").with_color_mode(ColorMode::Light));
        state(&mut o, &format!("{pre}rethemed/"), &m);
        o.w(
            &format!("{pre}rethemed/theme"),
            &format!("{} {}", m.theme().name(), m.theme().color_mode()),
        );
        m.reset();
        state(&mut o, &format!("{pre}reset/"), &m);
        m.set_size(0, -1);
        state(&mut o, &format!("{pre}zero/"), &m);
    }
    lipgloss::set_color_profile(Profile::Ascii);
    o.0
}

#[test]
fn commandpalette_parity() {
    let (lines, bytes) = compare(&generate(), DATA);
    eprintln!("commandpalette parity: {lines} lines, {bytes} bytes");
}
