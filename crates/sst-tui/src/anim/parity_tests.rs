//! Byte-for-byte parity with `testdata/parity/anim/anim.txt`, recorded by
//! `pkg/tui/anim/zz_paritygen_test.go`.

use std::time::Duration;

use sst_engine::gofmt::quote;
use sst_engine::types::{Coord, EntityType};
use sst_tuikit::lipgloss::{self, Style};
use sst_tuikit::termenv::Profile;

use super::*;

const DATA: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testdata/parity/anim/anim.txt"
));

const PROFILES: [Profile; 4] = [
    Profile::Ascii,
    Profile::ANSI,
    Profile::ANSI256,
    Profile::TrueColor,
];

fn render(s: &Style, text: &str) -> String {
    PROFILES
        .iter()
        .map(|&p| {
            lipgloss::set_color_profile(p);
            s.render(text)
        })
        .collect::<Vec<_>>()
        .join("|")
}

fn ns(d: Duration) -> i64 {
    d.as_nanos() as i64
}

fn frame(f: &Frame) -> String {
    let mut b = format!("dur={} n={}", ns(f.duration), f.overrides.len());
    for (k, ov) in &f.overrides {
        b.push_str(&format!(
            " [{},{}]{}={}",
            k[0],
            k[1],
            quote(&ov.glyph),
            render(&ov.style, &ov.glyph)
        ));
    }
    b
}

/// Go `%v` of a `Coord`.
fn cv(c: Coord) -> String {
    format!("[{} {}]", c[0], c[1])
}

/// Go `%v` of a `[]Coord`.
fn csv(cs: &[Coord]) -> String {
    format!(
        "[{}]",
        cs.iter().map(|&c| cv(c)).collect::<Vec<_>>().join(" ")
    )
}

struct Out(Vec<String>);

impl Out {
    fn w(&mut self, k: &str, v: &str) {
        self.0.push(format!("{k}\t{}", quote(v)));
    }

    fn anim<A: Animation>(&mut self, key: &str, a: &mut A, extra: Option<&dyn Fn(&A) -> String>) {
        let n = a.frames().len();
        self.w(
            &format!("{key}.total"),
            &format!(
                "{} frames={} finished={}",
                ns(a.total_duration()),
                n,
                a.is_finished()
            ),
        );
        for (i, f) in a.frames().iter().enumerate() {
            self.w(&format!("{key}.frame{i}"), &frame(f));
        }
        for i in 0..=n + 1 {
            let pre = extra.map(|e| e(a)).unwrap_or_default();
            let before = a.is_finished();
            let f = a.step();
            self.w(
                &format!("{key}.step{i}"),
                &format!(
                    "before={before} after={} {pre} {}",
                    a.is_finished(),
                    frame(&f)
                ),
            );
        }
    }

    fn skip<A: Animation>(
        &mut self,
        key: &str,
        a: &mut A,
        steps: usize,
        extra: Option<&dyn Fn(&A) -> String>,
    ) {
        for _ in 0..steps {
            a.step();
        }
        let f = a.skip();
        let post = extra.map(|e| e(a)).unwrap_or_default();
        self.w(
            &format!("{key}.skip{steps}"),
            &format!("finished={} {post} {}", a.is_finished(), frame(&f)),
        );
        let f = a.step();
        self.w(
            &format!("{key}.skip{steps}.step"),
            &format!("finished={} {}", a.is_finished(), frame(&f)),
        );
    }
}

fn generate() -> Vec<String> {
    let mut o = Out(Vec::new());
    lipgloss::set_has_dark_background(true);

    let speeds = [-1, 0, 1, 2, 3, 4];
    for s in [-5, -1, 0, 1, 2, 3, 4, 99] {
        o.w(
            &format!("frameduration.{s}"),
            &ns(frame_duration(s)).to_string(),
        );
    }

    let mut pts = Vec::new();
    for r in 1..=8 {
        for c in 1..=8 {
            pts.push([r, c]);
        }
    }
    for &a in &pts {
        let mut b = String::new();
        for &e in &pts {
            for p in bresenham_line(a, e) {
                b.push_str(&format!("{}{}", p[0], p[1]));
            }
            b.push(' ');
        }
        o.w(&format!("bresenham.{}.{}", a[0], a[1]), &b);
    }
    let odd: [(Coord, Coord); 4] = [
        ([0, 0], [10, 3]),
        ([-2, 5], [7, -4]),
        ([12, 12], [1, 1]),
        ([0, 0], [0, 0]),
    ];
    for (i, (a, b)) in odd.iter().enumerate() {
        o.w(&format!("bresenham.odd{i}"), &csv(&bresenham_line(*a, *b)));
    }

    for c in -1..=4 {
        let s = red_alert_badge_style(c);
        o.w(
            &format!("redalert.{c}"),
            &format!("bold={} {}", s.get_bold(), render(&s, "RED")),
        );
    }

    // Phaser.
    #[allow(clippy::type_complexity)]
    let phaser_cases: Vec<(Coord, Vec<Coord>, Vec<bool>)> = vec![
        ([3, 1], vec![[3, 5]], vec![true]),
        ([1, 4], vec![[5, 4]], vec![true]),
        ([1, 1], vec![[4, 4]], vec![false]),
        ([1, 4], vec![[4, 1]], vec![true]),
        (
            [4, 4],
            vec![[4, 7], [1, 4], [7, 7]],
            vec![true, false, true],
        ),
        (
            [4, 4],
            vec![[4, 5], [5, 5], [8, 1], [2, 8], [1, 2]],
            vec![true, true],
        ),
        ([5, 5], vec![[5, 5]], vec![true]),
        (
            [2, 2],
            vec![[8, 5], [5, 8], [8, 3]],
            vec![false, true, true, true],
        ),
        ([4, 4], vec![], vec![]),
        ([4, 4], vec![[4, 8], [4, 8]], vec![false, true]),
    ];
    for (i, (start, targets, hits)) in phaser_cases.iter().enumerate() {
        for sp in speeds {
            let key = format!("phaser.{i}.s{sp}");
            o.anim(
                &key,
                &mut new_multi_phaser_animation(*start, targets, hits, sp),
                None,
            );
            o.skip(
                &key,
                &mut new_multi_phaser_animation(*start, targets, hits, sp),
                1,
                None,
            );
        }
        if !targets.is_empty() {
            let hit = !hits.is_empty() && hits[0];
            o.anim(
                &format!("phaser1.{i}"),
                &mut new_phaser_animation(*start, targets[0], hit, 2),
                None,
            );
        }
    }

    // Torpedo.
    let torp_cases: [(Coord, Coord); 6] = [
        ([1, 1], [1, 4]),
        ([1, 1], [1, 2]),
        ([3, 3], [3, 3]),
        ([8, 8], [1, 1]),
        ([2, 7], [6, 1]),
        ([4, 4], [5, 6]),
    ];
    let entities = [
        EntityType::Klingon,
        EntityType::Romulan,
        EntityType::Tholian,
        EntityType::Starbase,
        EntityType::Star,
        EntityType::Planet,
        EntityType::BlackHole,
        EntityType::PlasmaTorpedo,
        EntityType::TholianWeb,
        EntityType::Empty,
        EntityType::Commander,
        EntityType::Wormhole,
    ];
    for (i, (s, e)) in torp_cases.iter().enumerate() {
        for hit in [true, false] {
            for sp in speeds {
                let key = format!("torpedo.{i}.{hit}.s{sp}");
                o.anim(&key, &mut new_torpedo_animation(*s, *e, hit, sp), None);
                o.skip(&key, &mut new_torpedo_animation(*s, *e, hit, sp), 2, None);
            }
            for ent in entities {
                let key = format!("torpedoE.{i}.{hit}.e{}", ent.as_i32());
                o.anim(
                    &key,
                    &mut new_torpedo_animation_with_entity(*s, *e, hit, ent, 2),
                    None,
                );
            }
        }
    }

    // Cloak.
    let ext_speeds = [-3, 7, -1, 0, 1, 2, 3, 4];
    for cloaking in [true, false] {
        let coord = [3, 6];
        for sp in ext_speeds {
            let key = format!("cloak.{cloaking}.s{sp}");
            let ex = |c: &CloakAnimation| {
                format!(
                    "glyph={} done={} coord={} cloaking={}",
                    quote(c.current_glyph()),
                    c.done(),
                    cv(c.coord()),
                    c.is_cloaking()
                )
            };
            o.anim(
                &key,
                &mut new_cloak_animation_with_speed(coord, cloaking, sp),
                Some(&ex),
            );
            let ex2 = |c: &CloakAnimation| {
                format!("glyph={} done={}", quote(c.current_glyph()), c.done())
            };
            o.skip(
                &key,
                &mut new_cloak_animation_with_speed(coord, cloaking, sp),
                1,
                Some(&ex2),
            );
        }
        let ex = |c: &CloakAnimation| format!("glyph={}", quote(c.current_glyph()));
        o.anim(
            &format!("cloakdef.{cloaking}"),
            &mut new_cloak_animation([1, 8], cloaking),
            Some(&ex),
        );
        let mut ct = new_cloak_animation([2, 2], cloaking);
        for i in 0..6 {
            let fr = ct.tick();
            o.w(
                &format!("cloaktick.{cloaking}.{i}"),
                &format!("{} {} {}", quote(ct.current_glyph()), ct.done(), frame(&fr)),
            );
        }
    }

    // Web.
    for action in [WebAction::Weave, WebAction::Breach, WebAction(7)] {
        o.w(&format!("webaction.{}", action.0), &action.to_string());
        let coord = [6, 2];
        for sp in ext_speeds {
            let key = format!("web.{}.s{sp}", action.0);
            let ex = |w: &WebAnimation| {
                format!(
                    "glyph={} done={} coord={} action={}",
                    quote(w.current_glyph()),
                    w.done(),
                    cv(w.coord()),
                    w.action()
                )
            };
            o.anim(
                &key,
                &mut new_web_animation_with_speed(coord, action, sp),
                Some(&ex),
            );
            let ex2 =
                |w: &WebAnimation| format!("glyph={} done={}", quote(w.current_glyph()), w.done());
            o.skip(
                &key,
                &mut new_web_animation_with_speed(coord, action, sp),
                2,
                Some(&ex2),
            );
        }
        let ex = |w: &WebAnimation| format!("glyph={}", quote(w.current_glyph()));
        o.anim(
            &format!("webdef.{}", action.0),
            &mut new_web_animation([8, 8], action),
            Some(&ex),
        );
    }
    o.w("webaction.-1", &WebAction(-1).to_string());

    // Plasma. Go clamps negative durations to 100ms; Rust durations are
    // unsigned, so the negative Go cases are constructed with the clamp.
    let plasma_cases: [(Coord, Coord); 5] = [
        ([2, 2], [4, 4]),
        ([1, 1], [1, 3]),
        ([5, 5], [5, 5]),
        ([8, 1], [1, 7]),
        ([3, 7], [6, 2]),
    ];
    let durs = [
        Duration::from_millis(100), // -1s
        Duration::from_millis(100), // -1ns
        Duration::ZERO,
        Duration::from_nanos(1),
        Duration::from_millis(50),
        Duration::from_millis(100),
        Duration::from_millis(240),
    ];
    for (i, (from, to)) in plasma_cases.iter().enumerate() {
        for (di, d) in durs.iter().enumerate() {
            let key = format!("plasma.{i}.d{di}");
            let ex = |p: &PlasmaAnimation| {
                format!(
                    "cur={} glyph={} done={} from={} to={} path={}",
                    cv(p.current_coord()),
                    quote(p.current_glyph()),
                    p.done(),
                    cv(p.from()),
                    cv(p.to()),
                    csv(p.path())
                )
            };
            o.anim(&key, &mut new_plasma_animation(*from, *to, *d), Some(&ex));
            let ex2 =
                |p: &PlasmaAnimation| format!("cur={} done={}", cv(p.current_coord()), p.done());
            o.skip(
                &key,
                &mut new_plasma_animation(*from, *to, *d),
                1,
                Some(&ex2),
            );
        }
    }
    o.0
}

#[test]
fn anim_parity() {
    let got = generate();
    let want: Vec<&str> = DATA.lines().collect();
    let mut mismatches = 0;
    for (i, (g, w)) in got.iter().zip(want.iter()).enumerate() {
        if g != w {
            mismatches += 1;
            if mismatches <= 5 {
                eprintln!("line {}:\n  go:   {w}\n  rust: {g}", i + 1);
            }
        }
    }
    assert_eq!(got.len(), want.len(), "line count");
    assert_eq!(mismatches, 0, "{mismatches} mismatching lines");
    let frames = want.iter().filter(|l| l.contains(".frame")).count();
    let steps = want.iter().filter(|l| l.contains(".step")).count();
    eprintln!(
        "anim parity: {} lines ({frames} frames, {steps} steps), {} bytes compared",
        want.len(),
        DATA.len()
    );
}
