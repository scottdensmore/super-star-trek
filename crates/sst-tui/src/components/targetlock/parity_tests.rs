//! Byte-for-byte parity with `testdata/parity/components/targetlock.txt`,
//! recorded by `pkg/tui/components/targetlock/zz_paritygen_test.go`.

#![allow(clippy::needless_range_loop, clippy::type_complexity)]

#[allow(clippy::duplicate_mod)]
#[path = "../parity_support.rs"]
mod pg;

use sst_engine::gofmt::{fmt_v, quote};
use sst_engine::{ptr, GameLength, PlasmaTorpedo, SkillLevel, TholianWebSegment};
use sst_tuikit::tea::{msg, WindowSizeMsg};

use super::*;

const DATA: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testdata/parity/components/targetlock.txt"
));

fn cv(c: Coord) -> String {
    format!("[{} {}]", c[0], c[1])
}

fn targets(m: &Model) -> String {
    let mut b = String::new();
    for t in m.targets() {
        b.push_str(&format!(
            "{} {} {} {} {} {} {} {}|",
            t.klingon_id,
            cv(t.coord),
            fmt_v(t.distance),
            fmt_v(t.bearing),
            fmt_v(t.hit_probability),
            fmt_v(t.power),
            quote(&t.name),
            t.typ.as_i32()
        ));
    }
    b
}

fn mstate(m: &Model) -> String {
    let cur = match m.current_target() {
        Some(c) => format!("{}@{}", c.klingon_id, cv(c.coord)),
        None => "nil".into(),
    };
    format!(
        "idx={} cur={cur} phaser={} input={} warn={} n={}",
        m.target_index(),
        m.inputting_phaser(),
        quote(m.phaser_input()),
        quote(m.warning_message()),
        m.targets().len()
    )
}

fn cmd_desc(c: Option<Cmd>) -> String {
    let Some(c) = c else { return "nil".into() };
    let r = c.call().expect("message");
    if let Some(v) = r.downcast_ref::<FireTorpedoMsg>() {
        return format!("torpedo {} {}", cv(v.target), fmt_v(v.bearing));
    }
    if let Some(v) = r.downcast_ref::<FirePhasersMsg>() {
        return format!("phasers {}", fmt_v(v.energy));
    }
    if r.downcast_ref::<CloseHUDMsg>().is_some() {
        return "close".into();
    }
    "?".into()
}

fn kl(id: i32, sector: Coord, energy: f64) -> Ptr<Klingon> {
    ptr(Klingon {
        id,
        sector,
        energy,
        ..Default::default()
    })
}

fn generate() -> Vec<String> {
    let mut o = pg::Out::new();
    let th0 = Some(pg::variants()[0].1);

    let games = pg::games();
    for (name, g) in &games {
        let mut m = new(th0);
        m.set_state_from_game(Some(g), [0, 0]);
        o.w(&format!("game/{name}/targets"), &targets(&m));
        o.w(&format!("game/{name}/state"), &mstate(&m));
        o.rot(&format!("game/{name}/view"), |th| {
            m.set_theme(Some(th));
            m.view()
        });
        let ts = m.targets();
        if let Some(last) = ts.last() {
            m.set_state_from_game(Some(g), last.coord);
            o.w(&format!("game/{name}/initial"), &mstate(&m));
        }
    }
    let mut nm = new(th0);
    nm.set_state_from_game(None, [1, 1]);
    o.w("nilgame", &mstate(&nm));

    let seq = [
        "tab",
        "tab",
        "right",
        "shift+tab",
        "left",
        "enter",
        "x",
        "p",
        "3",
        "5",
        "0",
        ".",
        "5",
        "a",
        "tab",
        "enter",
        "p",
        "enter",
        "backspace",
        "x",
        "1",
        "backspace",
        "backspace",
        "P",
        "p",
        ".",
        "enter",
        "9",
        "9",
        "backspace",
        "enter",
        "P",
        "esc",
        "p",
        "esc",
        "left",
        "left",
        "left",
        "enter",
        "space",
        "f2",
    ];
    for name in ["hot42", "hazards", "damaged", "seed1"] {
        let g = &games.iter().find(|(n, _)| n == name).unwrap().1;
        let mut m = new(th0);
        m.set_state_from_game(Some(g), [0, 0]);
        for (i, k) in seq.iter().enumerate() {
            let cmd = m.update(&pg::key_msg(k));
            let key = format!("seq/{name}/{i:02}");
            o.w(
                &key,
                &format!("key={} cmd={} {}", quote(k), cmd_desc(cmd), mstate(&m)),
            );
            if name == "hot42" || i % 8 == 0 {
                o.rot(&format!("{key}/view"), |th| {
                    m.set_theme(Some(th));
                    m.view()
                });
            }
        }
        let cmd = m.update(&msg(WindowSizeMsg {
            width: 10,
            height: 10,
        }));
        o.w(
            &format!("seq/{name}/winsize"),
            &format!("{} {}", cmd_desc(cmd), mstate(&m)),
        );
    }

    let mut ties = Vec::new();
    for i in 0..16usize {
        ties.push(kl(
            5 - (i % 4) as i32,
            [4 + [-2, 0, 2, 0][i % 4], 4 + [0, 2, 0, -2][i % 4]],
            (100 + i) as f64,
        ));
    }
    ties.push(None);
    let mut all = Vec::new();
    for r in 1..=8 {
        for c in 1..=8 {
            all.push(kl((r * 7 + c * 3) % 11, [r, c], (r * c) as f64 * 12.5));
        }
    }
    let far = vec![kl(4, [16, 16], 400.0), kl(3, [1, 1], -1.0)];
    let lists: Vec<(&str, Coord, &[Ptr<Klingon>], Coord, i32)> = vec![
        ("empty", [4, 4], &[], [0, 0], 10),
        ("ties", [4, 4], &ties, [4, 6], 3),
        ("all-center", [4, 4], &all, [8, 8], 0),
        ("all-corner", [1, 1], &all, [9, 9], 10),
        ("all-edge", [8, 5], &all[10..40], [0, 0], 1),
        ("far", [1, 1], &far, [0, 0], 10),
    ];
    for (name, ent, ks, init, torps) in lists {
        let mut m = new(th0);
        m.set_state(ent, 3450.5, torps, ks, init);
        o.w(&format!("list/{name}/targets"), &targets(&m));
        o.w(&format!("list/{name}/state"), &mstate(&m));
        o.rot(&format!("list/{name}/view"), |th| {
            m.set_theme(Some(th));
            m.view()
        });
        let cmd = m.update(&pg::key_msg("enter"));
        o.w(
            &format!("list/{name}/enter"),
            &format!("{} {}", cmd_desc(cmd), mstate(&m)),
        );
    }

    let mut wg = GameState::new_game(5, SkillLevel::Good, GameLength::Medium);
    wg.enterprise.sector = [4, 4];
    for r in 1..=8 {
        for c in (1..=8).step_by(3) {
            wg.current_quad.web_segments.push(ptr(TholianWebSegment {
                coord: [r, c],
                strength: (r + c) as f64,
            }));
        }
    }
    wg.current_quad.web_segments.push(None);
    wg.current_quad.plasma_torpedoes = vec![
        None,
        ptr(PlasmaTorpedo {
            id: 10005,
            sector: [4, 7],
            energy: 9.0,
            ..Default::default()
        }),
    ]
    .into();
    wg.current_quad.klingons = vec![
        kl(10001, [1, 4], 50.0),
        kl(2, [2, 2], 0.0),
        ptr(Klingon {
            id: 3,
            sector: [3, 3],
            energy: 70.0,
            is_cloaked: true,
            ..Default::default()
        }),
        ptr(Klingon {
            id: 4,
            sector: [7, 7],
            energy: 80.0,
            is_commander: true,
            ..Default::default()
        }),
    ]
    .into();
    let mut m = new(th0);
    m.set_state_from_game(Some(&wg), [4, 7]);
    o.w("web/targets", &targets(&m));
    o.w("web/state", &mstate(&m));
    o.each("web/view", |th| {
        m.set_theme(Some(th));
        m.view()
    });
    let _ = m.update(&pg::key_msg("p"));
    let _ = m.update(&pg::key_msg("7"));
    o.each("web/phaser", |th| {
        m.set_theme(Some(th));
        m.view()
    });
    o.each("new", |th| new(Some(th)).view());
    for (k, th) in pg::variants() {
        o.w(
            &format!("theme/{k}"),
            &format!(
                "{} {}",
                new(Some(th)).theme().name(),
                mstate(&new(Some(th)))
            ),
        );
    }
    o.lines
}

#[test]
fn parity_targetlock() {
    pg::check(DATA, &generate());
}
