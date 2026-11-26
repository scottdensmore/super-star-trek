//! Byte-for-byte parity with `testdata/parity/components/manual.txt`
//! (recorded by `pkg/tui/components/manual/zz_paritygen_test.go`).

#[path = "../modal_parity_util.rs"]
#[allow(clippy::duplicate_mod)]
mod util;

use sst_engine::gofmt::quote;
use sst_tuikit::tea::Msg;
use util::*;

use super::*;

const DATA: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testdata/parity/components/manual.txt"
));

const SEQUENCES: &[&[&str]] = &[
    &[
        "down", "j", "k", "up", "up", "down", "5", "1", "8", "9", "0", "enter",
    ],
    &[
        "tab", "down", "j", "end", "down", "pgup", "b", "home", "pgdown", "space", "ctrl+d", "G",
        "g", "ctrl+u", "x", "esc", "esc",
    ],
    &[
        "3",
        "right",
        "pgdown",
        "pgdown",
        "pgdown",
        "pgdown",
        "left",
        "down",
        "l",
        "up",
        "h",
        "shift+tab",
        "k",
        "tab",
    ],
    &[
        "q", "Q", "?", "f1", "winsize", "alt+a", "6", "l", "end", "7", "end", "4", "end", "2",
        "end",
    ],
];

const TOPICS: &[&str] = &[
    "", "  ", "combat", "COMBAT", "7", "9", "0", "unknown", "nav", " Nav ", "systems", "warp",
    "tor", "doc", "cloak", "points", "guide", "help nav", "shield", "commands", "Scoring", "12",
];

fn desc(m: &Msg) -> Option<String> {
    m.downcast_ref::<CloseModalMsg>()
        .map(|_| "CloseModalMsg".to_string())
}

fn state_str(m: &Model) -> String {
    let offs: Vec<String> = m.scroll_offsets.iter().map(|o| o.to_string()).collect();
    format!(
        "sel={} focus={} off=[{}]",
        m.selected_idx,
        m.focus as i32,
        offs.join(" ")
    )
}

fn state(o: &mut Out, p: &str, m: &Model) {
    o.w(&format!("{p}view"), &m.view());
    o.w(&format!("{p}state"), &state_str(m));
}

fn generate() -> Vec<String> {
    let mut o = Out::default();
    for c in combos() {
        let (th, pre) = c.apply();
        let full = c.full();
        for i in 0..8usize {
            if !full && i != 2 {
                continue;
            }
            let mut m = new(th, 66, 18);
            m.selected_idx = i as i32;
            o.w(&format!("{pre}chapter/{i}"), &m.view());
            m.focus = FocusPane::FocusContent;
            m.scroll_offsets[i] = 1000;
            o.w(&format!("{pre}chapter/{i}/bottom"), &m.view());
        }
        for (si, seq) in SEQUENCES.iter().enumerate() {
            let mut m = new(th, 80, 24);
            for (ki, k) in seq.iter().enumerate() {
                let cmd = m.update(&key(k));
                let p = format!("{pre}seq{si}/{ki}/{k}/");
                o.w(&format!("{p}msg"), &describe_cmd(cmd, &desc));
                if full || ki == seq.len() - 1 {
                    state(&mut o, &p, &m);
                } else {
                    o.w(&format!("{p}state"), &state_str(&m));
                }
            }
        }
        if full {
            let mut m = new(th, 0, 0);
            for tp in TOPICS {
                m.select_chapter(tp);
                o.w(
                    &format!("{pre}topic/{}", quote(tp)),
                    &m.selected_idx.to_string(),
                );
            }
            m.selected_idx = 42;
            o.w(&format!("{pre}oob/view"), &m.view());
            let _ = m.update(&key("down"));
            state(&mut o, &format!("{pre}oob/"), &m);
            m.selected_idx = -3;
            let _ = m.update(&key("x"));
            state(&mut o, &format!("{pre}neg/"), &m);
            m.set_theme(None);
            o.w(
                &format!("{pre}niltheme"),
                &format!("{} {}", m.theme.name(), m.theme.color_mode()),
            );
            for (i, ch) in new(th, 66, 18).chapters.iter().enumerate() {
                o.w(
                    &format!("{pre}meta/{i}"),
                    &format!(
                        "{}|{}|{}|{}|{}",
                        ch.id,
                        ch.number,
                        ch.title,
                        ch.short_tag,
                        ch.lines.len()
                    ),
                );
            }
        }
    }
    o.0
}

#[test]
fn manual_parity() {
    let (lines, bytes) = compare(&generate(), DATA);
    eprintln!("manual parity: {lines} lines, {bytes} bytes");
}
