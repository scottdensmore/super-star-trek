//! Byte-for-byte parity with `testdata/parity/components/sectorgrid.txt`,
//! recorded by `pkg/tui/components/sectorgrid/zz_paritygen_test.go`.

#![allow(clippy::needless_range_loop, clippy::type_complexity)]

#[allow(clippy::duplicate_mod)]
#[path = "../parity_support.rs"]
mod pg;

use sst_engine::{
    ptr, EnemyVessel, EntityType, FactionType, Klingon, QuadrantState, ANIM_SPEED_FAST,
};
use sst_tuikit::lipgloss;

use super::*;
use crate::anim::{new_torpedo_animation, Animation, CellOverride, Overrides};

const DATA: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testdata/parity/components/sectorgrid.txt"
));

fn first_sel(q: &QuadrantState) -> Coord {
    for r in 1..=8 {
        for c in 1..=8 {
            let e = q.grid[r][c];
            if e != EntityType::Empty && e != EntityType::Enterprise {
                return [r as i32, c as i32];
            }
        }
    }
    [4, 4]
}

fn generate() -> Vec<String> {
    let mut o = pg::Out::new();

    let full = ["hot42", "hazards"];
    for (gi, (name, g)) in pg::games().into_iter().enumerate() {
        let r = |th: Theme| new(Some(th)).view(Some(&g.current_quad), g.enterprise.sector, [0, 0]);
        if full.contains(&name.as_str()) {
            o.each(&format!("game/{name}/sel0"), r);
        } else {
            o.rot(&format!("game/{name}/sel0"), r);
        }
        let sel = first_sel(&g.current_quad);
        o.rot(&format!("game/{name}/sel{}{}", sel[0], sel[1]), |th| {
            new(Some(th)).view(Some(&g.current_quad), g.enterprise.sector, sel)
        });
        if gi < 6 {
            o.rot(&format!("game/{name}/withgame"), |th| {
                new(Some(th)).view_with_game(Some(&g))
            });
        }
    }

    let mut all = QuadrantState::default();
    let ents = [
        EntityType::Empty,
        EntityType::Enterprise,
        EntityType::Klingon,
        EntityType::Commander,
        EntityType::SuperCommander,
        EntityType::Starbase,
        EntityType::Star,
        EntityType::Planet,
        EntityType::BlackHole,
        EntityType::Wormhole,
        EntityType::Romulan,
        EntityType::Tholian,
        EntityType::PlasmaTorpedo,
        EntityType::TholianWeb,
        EntityType(42),
    ];
    for (i, e) in ents.iter().enumerate() {
        all.grid[1 + i / 8][1 + i % 8] = *e;
    }
    all.grid[4][1] = EntityType::Commander;
    all.grid[4][2] = EntityType::Romulan;
    all.klingons = vec![
        None,
        ptr(Klingon {
            id: 1,
            sector: [4, 1],
            is_cloaked: true,
            ..Default::default()
        }),
    ]
    .into();
    all.enemies = vec![
        None,
        ptr(EnemyVessel {
            id: 2,
            faction: FactionType::Romulan,
            sector: [4, 2],
            is_cloaked: true,
            ..Default::default()
        }),
        ptr(EnemyVessel {
            id: 3,
            sector: [1, 2],
            ..Default::default()
        }),
    ]
    .into();
    o.each("all/sel0", |th| {
        new(Some(th)).view(Some(&all), [6, 6], [0, 0])
    });
    for i in 0..ents.len() {
        let sel = [1 + i as i32 / 8, 1 + i as i32 % 8];
        o.rot(&format!("all/sel{}{}", sel[0], sel[1]), |th| {
            new(Some(th)).view(Some(&all), [6, 6], sel)
        });
    }
    for sel in [[4, 1], [4, 2], [6, 6], [0, 3], [9, 9]] {
        o.rot(&format!("all/sel{}{}", sel[0], sel[1]), |th| {
            new(Some(th)).view(Some(&all), [6, 6], sel)
        });
    }
    o.lite("all/ent-on-cloak", |th| {
        new(Some(th)).view(Some(&all), [4, 1], [4, 1])
    });
    o.lite("all/ent-offgrid", |th| {
        new(Some(th)).view(Some(&all), [0, 9], [0, 0])
    });
    o.each("nilquad", |th| new(Some(th)).view(None, [3, 4], [5, 5]));
    o.lite("nilgame", |th| new(Some(th)).view_with_game(None));

    let mut ov = Overrides::new();
    let cell = |g: &str, s: lipgloss::Style| CellOverride {
        glyph: g.into(),
        style: s,
    };
    ov.insert([1, 1], cell("", lipgloss::new_style()));
    ov.insert(
        [2, 2],
        cell("*", lipgloss::new_style().foreground(lipgloss::color("9"))),
    );
    ov.insert([3, 3], cell("**", lipgloss::new_style().bold(true)));
    ov.insert(
        [4, 4],
        cell(
            "***",
            lipgloss::new_style()
                .foreground(lipgloss::color("#FF8800"))
                .background(lipgloss::color("#001122")),
        ),
    );
    ov.insert(
        [5, 5],
        cell("OVERLONG", lipgloss::new_style().reverse(true)),
    );
    ov.insert(
        [6, 6],
        cell(
            "★☆✦✧",
            lipgloss::new_style().foreground(lipgloss::color("201")),
        ),
    );
    ov.insert(
        [8, 8],
        cell(
            " \\ ",
            lipgloss::new_style().foreground(lipgloss::color("#00FF00")),
        ),
    );
    let g0 = pg::games().swap_remove(1).1;
    o.each("anim/overrides", |th| {
        let mut m = new(None);
        m.set_theme(Some(th));
        m.set_anim_overrides(ov.clone());
        m.view(Some(&g0.current_quad), g0.enterprise.sector, [2, 2])
    });
    o.lite("anim/cleared", |th| {
        let mut m = new(Some(th));
        m.set_anim_overrides(ov.clone());
        m.clear_anim_overrides();
        m.view(Some(&g0.current_quad), g0.enterprise.sector, [2, 2])
    });
    let tor = new_torpedo_animation([1, 1], [8, 7], true, ANIM_SPEED_FAST);
    for (i, fr) in tor.frames().iter().enumerate().take(4) {
        o.rot(&format!("anim/torpedo{i}"), |th| {
            let mut m = new(Some(th));
            m.set_anim_overrides(fr.overrides.clone());
            m.view(Some(&g0.current_quad), g0.enterprise.sector, [0, 0])
        });
    }

    for (k, th) in pg::variants() {
        o.w(&format!("theme/{k}"), new(Some(th)).theme().name());
    }
    for y in -2..=12 {
        let mut b = String::new();
        for x in -2..=40 {
            match new(None).hit_test(x, y) {
                Some(c) => b.push_str(&format!("{},{} ", c[0], c[1])),
                None => b.push_str("- "),
            }
        }
        o.w(&format!("hit/{y}"), &b);
    }
    o.lines
}

#[test]
fn parity_sectorgrid() {
    pg::check(DATA, &generate());
}
