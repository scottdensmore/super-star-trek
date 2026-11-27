//! Byte-for-byte parity with `testdata/parity/components/damageschematic.txt`,
//! recorded by `pkg/tui/components/damageschematic/zz_paritygen_test.go`.

#![allow(clippy::needless_range_loop, clippy::type_complexity)]

#[allow(clippy::duplicate_mod)]
#[path = "../parity_support.rs"]
mod pg;

use sst_tuikit::tea::{msg, MouseMsg, WindowSizeMsg, MOUSE_BUTTON_LEFT};

use super::*;

const DATA: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testdata/parity/components/damageschematic.txt"
));

fn render(
    th: Theme,
    w: i32,
    // The recorded Go cases vary the (unused) height too.
    _h: i32,
    devs: [f64; 8],
    cond: &str,
    docked: bool,
    mult: f64,
) -> String {
    let mut m = new(Some(th), w);
    let ent = EnterpriseState {
        devices: devs,
        ..Default::default()
    };
    m.set_state(&ent, cond, docked, mult);
    m.view()
}

fn generate() -> Vec<String> {
    let mut o = pg::Out::new();

    let cases: [(&str, [f64; 8], &str, bool, f64); 13] = [
        ("nominal", [0.0; 8], "GREEN", false, 1.0),
        ("empty-cond", [0.0; 8], "", false, 1.0),
        ("docked-flag", [0.0; 8], "RED", true, 1.0),
        (
            "two",
            [0.0, 0.0, 0.0, 0.0, 1.4, 0.0, 0.0, 2.1],
            "YELLOW",
            false,
            1.0,
        ),
        (
            "warp-mult",
            [2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            "RED",
            false,
            1.5,
        ),
        (
            "three",
            [0.5, 0.0, 3.0, 0.0, 0.0, 1.99, 0.0, 0.0],
            "YELLOW",
            false,
            0.25,
        ),
        (
            "four",
            [1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0],
            "GREEN",
            true,
            1.0,
        ),
        (
            "five",
            [1.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0],
            "GREEN",
            true,
            1.0,
        ),
        (
            "all",
            [0.1, 0.2, 2.3, 4.4, 5.5, 6.6, 7.7, 8.8],
            "RED",
            false,
            2.0,
        ),
        (
            "neg-mult",
            [0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            "GREEN",
            false,
            -1.0,
        ),
        (
            "zero-mult",
            [0.0, 3.25, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            "DOCKED",
            false,
            0.0,
        ),
        (
            "negative-dev",
            [-1.0, 0.0, -0.5, 0.0, 0.0, 0.0, 0.0, 0.05],
            "GREEN",
            false,
            1.0,
        ),
        (
            "huge",
            [0.0, 0.0, 0.0, 1234.56, 0.0, 0.0, 99999.9, 0.0],
            "A VERY LONG ALERT CONDITION STRING THAT OVERFLOWS",
            false,
            3.0,
        ),
    ];
    for (i, (name, devs, cond, docked, mult)) in cases.iter().enumerate() {
        let r = |th| render(th, 66, 18, *devs, cond, *docked, *mult);
        if i == 0 || i == 3 || i == 12 {
            o.each(&format!("case/{name}"), r);
        } else {
            o.rot(&format!("case/{name}"), r);
        }
    }
    for (w, h) in [
        (0, 0),
        (80, 20),
        (50, 10),
        (30, 5),
        (2, 2),
        (1, 1),
        (-5, -5),
    ] {
        o.lite(&format!("size/{w}x{h}"), |th| {
            render(th, w, h, cases[8].1, "YELLOW", false, 1.0)
        });
    }
    o.lite("settheme-nil", |th| {
        let mut m = new(Some(th), 66);
        m.set_theme(None);
        m.view()
    });
    o.lite("settheme", |th| {
        let mut m = new(Some(pg::variants()[5].1), 66);
        m.set_theme(Some(th));
        m.view()
    });
    o.lite("unset", |th| new(Some(th), 66).view());

    let mut m = new(Some(pg::variants()[0].1), 66);
    for k in [
        "esc", "enter", "q", "Q", "d", "D", " ", "space", "x", "tab", "up", "ctrl+a", "alt+b",
        "f2", "dd",
    ] {
        let res = match m.update(&pg::key_msg(k)) {
            None => "nil",
            Some(c) => match c.call() {
                Some(r) if r.downcast_ref::<CloseModalMsg>().is_some() => "close",
                _ => "?",
            },
        };
        o.w(&format!("key/{k}"), res);
    }
    let msgs: [(&str, Msg); 3] = [
        (
            "tea.WindowSizeMsg",
            msg(WindowSizeMsg {
                width: 3,
                height: 3,
            }),
        ),
        (
            "tea.MouseMsg",
            msg(MouseMsg {
                button: MOUSE_BUTTON_LEFT,
                ..Default::default()
            }),
        ),
        ("struct {}", msg(())),
    ];
    for (name, mm) in msgs {
        o.w(&format!("msg/{name}"), &m.update(&mm).is_none().to_string());
    }
    o.lines
}

#[test]
fn parity_damageschematic() {
    pg::check(DATA, &generate());
}
