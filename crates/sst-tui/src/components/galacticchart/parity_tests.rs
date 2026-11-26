//! Byte-for-byte parity with `testdata/parity/components/galacticchart.txt`,
//! recorded by `pkg/tui/components/galacticchart/zz_paritygen_test.go`.

#![allow(clippy::needless_range_loop, clippy::type_complexity)]

#[allow(clippy::duplicate_mod)]
#[path = "../parity_support.rs"]
mod pg;

use sst_engine::gofmt::{fmt_v, quote};
use sst_engine::DeviceID;
use sst_tuikit::tea::{msg, MouseMsg, WindowSizeMsg, MOUSE_BUTTON_LEFT};

use super::*;

const DATA: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testdata/parity/components/galacticchart.txt"
));

fn cv(c: Coord) -> String {
    format!("[{} {}]", c[0], c[1])
}

fn telem(t: &Telemetry) -> String {
    format!(
        "{{{} {} {} {} {} {} {} {} {}}}",
        cv(t.from),
        cv(t.to),
        t.delta_r,
        t.delta_c,
        fmt_v(t.distance),
        fmt_v(t.course),
        t.direction,
        fmt_v(t.recommended_warp),
        t.is_current
    )
}

fn cmd_desc(c: Option<Cmd>) -> String {
    let Some(c) = c else { return "nil".into() };
    let r = c.call().expect("message");
    if let Some(v) = r.downcast_ref::<WarpToQuadrantMsg>() {
        return format!("warp {} {}", cv(v.dest_quad), fmt_v(v.warp));
    }
    if let Some(v) = r.downcast_ref::<WarpBlockedMsg>() {
        return format!("blocked {}", quote(&v.reason));
    }
    if r.downcast_ref::<CloseChartMsg>().is_some() {
        return "close".into();
    }
    "?".into()
}

fn mstate(m: &Model) -> String {
    format!(
        "cursor={} help={} size={}x{} comp={} ent={}",
        cv(m.cursor()),
        m.showing_help(),
        m.width(),
        m.height(),
        m.computer_damaged(),
        cv(m.enterprise_quad)
    )
}

fn generate() -> Vec<String> {
    let mut o = pg::Out::new();
    let th0 = Some(pg::variants()[0].1);

    for from in [[1, 1], [4, 4], [8, 8], [3, 6], [0, 0], [9, -1]] {
        let mut b = String::new();
        for r in 0..=9 {
            for c in 0..=9 {
                b.push_str(&telem(&calculate_telemetry(from, [r, c])));
                b.push('|');
            }
        }
        o.w(&format!("telemetry/{},{}", from[0], from[1]), &b);
    }

    for (i, (name, g)) in pg::games().into_iter().enumerate() {
        let comp = g.enterprise.devices[DeviceID::Computer.as_i32() as usize] > 0.0;
        let mut m = new(th0, 64, 18);
        m.set_state(
            g.enterprise.quad,
            g.galaxy_chart,
            g.chart_discovered,
            g.chart_known_bases,
            comp,
        );
        if i % 3 == 1 {
            m.set_cursor([1 + (i % 8) as i32, 8 - (i % 8) as i32]);
        }
        o.w(&format!("game/{name}/state"), &mstate(&m));
        let r = |th| {
            m.set_theme(Some(th));
            m.view()
        };
        if name == "hazards" {
            o.each(&format!("game/{name}"), r);
        } else {
            o.rot(&format!("game/{name}"), r);
        }
    }

    let mut chart = [[0i32; 9]; 9];
    let mut disc = [[false; 9]; 9];
    let mut bases = [[false; 9]; 9];
    for r in 0..9 {
        for c in 0..9 {
            chart[r][c] = ((r * 131 + c * 17) % 1200) as i32;
            disc[r][c] = (r + c) % 3 != 0;
            bases[r][c] = (r * c) % 4 == 1;
        }
    }
    chart[2][2] = -7;
    let seq = [
        "?", "k", "?", "esc", "?", "q", "?", "Q", "?", "x", "?", "j", "up", "up", "up", "up", "up",
        "left", "left", "left", "left", "h", "H", "K", "enter", "l", "L", "right", "J", "down",
        "j", "?", "enter", "esc", "f2", "tab", "space", "?", "?", "l", "enter",
    ];
    for comp in [false, true] {
        let mut m = new(th0, 64, 18);
        m.set_state([3, 3], chart, disc, bases, comp);
        for (i, k) in seq.iter().enumerate() {
            let cmd = m.update(&pg::key_msg(k));
            let key = format!("seq/comp={comp}/{i:02}");
            o.w(
                &key,
                &format!("key={} cmd={} {}", quote(k), cmd_desc(cmd), mstate(&m)),
            );
            if i % 8 == 0 {
                o.rot(&format!("{key}/view"), |th| {
                    m.set_theme(Some(th));
                    m.view()
                });
            }
        }
        let cmd = m.update(&msg(WindowSizeMsg {
            width: 200,
            height: 100,
        }));
        o.w(
            &format!("seq/comp={comp}/winsize"),
            &format!("{} {}", cmd_desc(cmd), mstate(&m)),
        );
        let cmd = m.update(&msg(MouseMsg {
            x: 10,
            y: 5,
            button: MOUSE_BUTTON_LEFT,
            ..Default::default()
        }));
        o.w(
            &format!("seq/comp={comp}/mouse"),
            &format!("{} {}", cmd_desc(cmd), mstate(&m)),
        );
    }
    o.each("synthetic", |th| {
        let mut m = new(Some(th), 64, 18);
        m.set_state([5, 2], chart, disc, bases, false);
        m.set_cursor([2, 7]);
        m.view()
    });
    o.lite("help", |th| {
        let mut m = new(Some(th), 64, 18);
        m.set_showing_help(true);
        m.view()
    });

    for (w, h) in [(0, 0), (80, 24), (40, 10), (12, 5), (3, 3), (-1, 30)] {
        o.lite(&format!("new/{w}x{h}"), |th| {
            let m = new(Some(th), w, h);
            format!("{}\n{}", mstate(&m), m.view())
        });
        o.rot(&format!("setsize/{w}x{h}"), |th| {
            let mut m = new(Some(th), 64, 18);
            m.set_size(w, h);
            m.set_showing_help(w == 80);
            format!("{}\n{}", mstate(&m), m.view())
        });
    }
    let mut m = new(th0, 64, 18);
    for c in [[0, 10], [-3, 4], [9, 9], [5, 5]] {
        m.set_state(c, chart, disc, bases, false);
        o.w(&format!("setstate/{},{}", c[0], c[1]), &mstate(&m));
        m.set_cursor([c[1], c[0]]);
        o.w(&format!("setcursor/{},{}", c[1], c[0]), &mstate(&m));
    }
    o.w("init", &m.init().is_none().to_string());
    for help in [false, true] {
        m.set_showing_help(help);
        for y in -1..=13 {
            let mut b = String::new();
            for x in -1..=65 {
                match m.hit_test(x, y) {
                    Some(c) => b.push_str(&format!("{},{} ", c[0], c[1])),
                    None => b.push_str("- "),
                }
            }
            o.w(&format!("hit/help={help}/{y}"), &b);
        }
    }
    for (k, th) in pg::variants() {
        o.w(
            &format!("theme/{k}"),
            new(Some(th), 0, 0).theme().unwrap().name(),
        );
    }
    o.lines
}

#[test]
fn parity_galacticchart() {
    pg::check(DATA, &generate());
}
