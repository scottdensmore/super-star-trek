//! Port of `pkg/tui/components/sectorgrid/grid_test.go`.

use sst_engine::{ptr, EntityType, Klingon, QuadrantState};
use sst_tuikit::ansi;
use sst_tuikit::lipgloss;

use super::*;
use crate::anim::{CellOverride, Overrides};
use crate::theme::{detect, LcarsTheme, ModernTheme};

fn setup() {
    detect::set_fixed_detection(true);
}

fn dt() -> Option<Theme> {
    Some(theme::default_theme())
}

#[test]
fn grid_rendering() {
    setup();
    let m = new(dt());
    let mut quad = QuadrantState::default();
    quad.grid[2][3] = EntityType::Enterprise;
    quad.grid[4][5] = EntityType::Klingon;
    quad.grid[1][1] = EntityType::Starbase;
    quad.grid[8][8] = EntityType::Star;
    let view = m.view(Some(&quad), [2, 3], [0, 0]);
    assert!(view.contains('1') && view.contains('8'), "{view}");
    for g in ["<E>", "+K+", ">B<", " * "] {
        assert!(view.contains(g), "missing {g}:\n{view}");
    }
}

#[test]
fn all_entity_glyphs() {
    setup();
    let m = new(dt());
    let mut quad = QuadrantState::default();
    let row1 = [
        EntityType::Enterprise,
        EntityType::Klingon,
        EntityType::Commander,
        EntityType::SuperCommander,
        EntityType::Starbase,
        EntityType::Star,
        EntityType::Planet,
        EntityType::BlackHole,
    ];
    for (i, e) in row1.iter().enumerate() {
        quad.grid[1][i + 1] = *e;
    }
    quad.grid[2][1] = EntityType::Wormhole;
    let view = m.view(Some(&quad), [1, 1], [0, 0]);
    for g in ["<E>", "+K+", ">B<", " * ", " O ", " @ ", ">W<", " . "] {
        assert!(view.contains(g), "missing {g:?}:\n{view}");
    }
}

#[test]
fn grid_render_entity_wormhole_and_black_hole() {
    setup();
    let mut quad = QuadrantState::default();
    quad.grid[3][3] = EntityType::Wormhole;
    assert!(render_cell(3, 3, Some(&quad), false).contains(">W<"));
    quad.grid[3][3] = EntityType::BlackHole;
    assert!(render_cell(3, 3, Some(&quad), false).contains('@'));
}

#[test]
fn grid_axes() {
    setup();
    let m = new(dt());
    let quad = QuadrantState::default();
    let view = m.view(Some(&quad), [1, 1], [0, 0]);
    for i in 1..=8 {
        assert!(view.contains(&i.to_string()), "missing axis {i}");
    }
}

#[test]
fn set_theme() {
    setup();
    let mut m = new(Some(ModernTheme::default().into()));
    assert_eq!(m.theme().name(), "modern");
    let mut quad = QuadrantState::default();
    quad.grid[1][1] = EntityType::Enterprise;
    let view_modern = m.view(Some(&quad), [1, 1], [0, 0]);

    m.set_theme(Some(LcarsTheme::default().into()));
    assert_eq!(m.theme().name(), "lcars");
    let view_lcars = m.view(Some(&quad), [1, 1], [0, 0]);
    assert!(!view_modern.is_empty() && !view_lcars.is_empty());
    assert_ne!(view_modern, view_lcars);

    m.set_theme(None);
    assert_eq!(m.theme().name(), "modern");
    assert_eq!(m.view(Some(&quad), [1, 1], [0, 0]), view_modern);
}

#[test]
fn nil_quadrant_and_defaults() {
    setup();
    let m = new(None);
    assert_eq!(m.theme().name(), "modern");
    let view = m.view(None, [3, 4], [0, 0]);
    assert!(view.contains("<E>"));
    assert!(view.contains(" . "));
    let view_no_ent = m.view(None, [0, 0], [0, 0]);
    assert!(!view_no_ent.contains("<E>"));
}

#[test]
fn hit_test_valid_cells() {
    let m = new(None);
    for r in 1..=8 {
        for c in 1..=8 {
            let start_x = 3 + 4 * (c - 1);
            for dx in 0..3 {
                assert_eq!(m.hit_test(start_x + dx, r + 1), Some([r, c]));
            }
        }
    }
}

#[test]
fn hit_test_out_of_bounds_and_borders() {
    let m = new(None);
    let cases = [
        (4, 0),
        (4, 1),
        (4, -1),
        (4, 10),
        (4, 11),
        (5, 20),
        (0, 2),
        (1, 2),
        (2, 2),
        (-1, 2),
        (6, 2),
        (10, 3),
        (14, 4),
        (18, 5),
        (22, 6),
        (26, 7),
        (30, 8),
        (34, 2),
        (35, 2),
        (50, 4),
    ];
    for (x, y) in cases {
        assert_eq!(m.hit_test(x, y), None, "({x}, {y})");
    }
}

#[test]
fn view_with_reticle() {
    setup();
    let m = new(dt());
    let mut quad = QuadrantState::default();
    quad.grid[2][3] = EntityType::Enterprise;
    quad.grid[4][5] = EntityType::Klingon;
    quad.grid[1][1] = EntityType::Starbase;
    quad.grid[6][7] = EntityType::Star;
    quad.grid[7][8] = EntityType::Planet;
    quad.grid[8][1] = EntityType::BlackHole;
    let v = |sel| m.view(Some(&quad), [2, 3], sel);

    assert!(v([3, 4]).contains("[.]"));
    let ve = v([2, 3]);
    assert!(ve.contains("[E]") && !ve.contains("<E>"));
    let vk = v([4, 5]);
    assert!(vk.contains("[K]") && !vk.contains("+K+"));
    assert!(v([1, 1]).contains("[B]"));
    assert!(v([6, 7]).contains("[*]"));
    assert!(v([7, 8]).contains("[O]"));
    assert!(v([8, 1]).contains("[@]"));
    let none = v([0, 0]);
    for b in ["[.]", "[E]", "[K]", "[B]", "[*]", "[O]", "[@]"] {
        assert!(!none.contains(b), "unexpected {b}");
    }
}

#[test]
fn cloaked_klingon_rendering_sensor_anomaly() {
    setup();
    let m = new(dt());
    let mut quad = QuadrantState::default();
    quad.grid[4][5] = EntityType::Commander;
    let cloaked = ptr(Klingon {
        id: 1,
        sector: [4, 5],
        is_commander: true,
        is_cloaked: true,
        ..Default::default()
    });
    quad.klingons = vec![cloaked.clone()].into();

    let unsel = m.view(Some(&quad), [1, 1], [0, 0]);
    assert!(unsel.contains(" ? ") && !unsel.contains("+K+"));
    let sel = m.view(Some(&quad), [1, 1], [4, 5]);
    assert!(sel.contains("[?]") && !sel.contains("[K]"));

    cloaked.as_ref().unwrap().borrow_mut().is_cloaked = false;
    let dec = m.view(Some(&quad), [1, 1], [0, 0]);
    assert!(dec.contains("+K+") && !dec.contains(" ? "));
    assert!(m.view(Some(&quad), [1, 1], [4, 5]).contains("[K]"));

    cloaked.as_ref().unwrap().borrow_mut().is_cloaked = true;
    let uncloaked = ptr(Klingon {
        id: 2,
        sector: [6, 6],
        ..Default::default()
    });
    quad.grid[6][6] = EntityType::Klingon;
    quad.klingons = vec![cloaked, uncloaked].into();
    let both = m.view(Some(&quad), [1, 1], [0, 0]);
    assert!(both.contains(" ? ") && both.contains("+K+"));
}

fn widths(v: &str) -> Vec<i32> {
    v.split('\n').map(ansi::string_width).collect()
}

#[test]
fn sector_grid_anim_overrides() {
    setup();
    let mut m = new(dt());
    let quad = QuadrantState::default();
    let view_init = m.view(Some(&quad), [1, 1], [0, 0]);
    assert!(!view_init.contains("***"));

    let mut ov = Overrides::new();
    ov.insert(
        [2, 2],
        CellOverride {
            glyph: "***".into(),
            style: lipgloss::new_style().foreground(lipgloss::color("9")),
        },
    );
    m.set_anim_overrides(ov);
    let view_anim = m.view(Some(&quad), [1, 1], [0, 0]);
    assert!(view_anim.contains("***"));
    assert_eq!(widths(&view_anim), widths(&view_init));

    m.clear_anim_overrides();
    let clean = m.view(Some(&quad), [1, 1], [0, 0]);
    assert!(!clean.contains("***"));
    assert_eq!(clean, view_init);
}

#[test]
fn sector_grid_anim_overrides_padding() {
    setup();
    let mut m = new(dt());
    let quad = QuadrantState::default();
    let view_init = m.view(Some(&quad), [1, 1], [0, 0]);
    let mut ov = Overrides::new();
    ov.insert(
        [3, 3],
        CellOverride {
            glyph: "*".into(),
            style: lipgloss::new_style(),
        },
    );
    ov.insert(
        [4, 4],
        CellOverride {
            glyph: "OVERLONG".into(),
            style: lipgloss::new_style(),
        },
    );
    m.set_anim_overrides(ov);
    let view_anim = m.view(Some(&quad), [1, 1], [0, 0]);
    assert_eq!(widths(&view_anim), widths(&view_init));
}
