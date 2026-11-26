//! 8x8 sector grid visualizer (port of `pkg/tui/components/sectorgrid/grid.go`).

use sst_engine::{Coord, EntityType, GameState, QuadrantState};

use crate::anim::Overrides;
use crate::theme::{self, Styles, Theme};

#[cfg(test)]
mod parity_tests;
#[cfg(test)]
mod tests;

/// The sector grid component (Go `sectorgrid.Model`).
#[derive(Clone, Default)]
pub struct Model {
    theme: Option<Theme>,
    anim_overrides: Overrides,
}

/// `New`: a grid using `th` (nil falls back to the default theme).
pub fn new(th: Option<Theme>) -> Model {
    Model {
        theme: Some(th.unwrap_or_else(theme::default_theme)),
        anim_overrides: Overrides::new(),
    }
}

impl Model {
    /// `SetTheme` (nil falls back to the default theme).
    pub fn set_theme(&mut self, th: Option<Theme>) {
        self.theme = Some(th.unwrap_or_else(theme::default_theme));
    }

    /// `SetAnimOverrides`: temporary cell overrides for animation rendering.
    pub fn set_anim_overrides(&mut self, overrides: Overrides) {
        self.anim_overrides = overrides;
    }

    /// `ClearAnimOverrides`.
    pub fn clear_anim_overrides(&mut self) {
        self.anim_overrides = Overrides::new();
    }

    /// `Theme`.
    pub fn theme(&self) -> Theme {
        self.theme.unwrap_or_else(theme::default_theme)
    }

    /// `HitTest`: maps coordinates relative to the grid's bounding box
    /// (including its border) to a 1-indexed sector. Go's `(Coord{}, false)`
    /// is `None`.
    pub fn hit_test(&self, rel_x: i32, rel_y: i32) -> Option<Coord> {
        if !(2..=9).contains(&rel_y) {
            return None;
        }
        let r = rel_y - 1;
        if rel_x < 3 {
            return None;
        }
        let offset = rel_x - 3;
        let c = offset / 4 + 1;
        let char_within_cell = offset % 4;
        if !(1..=8).contains(&c) || char_within_cell >= 3 {
            return None;
        }
        Some([r, c])
    }

    /// `ViewWithGame`.
    pub fn view_with_game(&self, g: Option<&GameState>) -> String {
        match g {
            Some(g) => self.view(Some(&g.current_quad), g.enterprise.sector, [0, 0]),
            None => self.view(None, [0, 0], [0, 0]),
        }
    }

    /// `View`: renders the grid for `quad`, the Enterprise position and the
    /// selected reticle cell.
    pub fn view(&self, quad: Option<&QuadrantState>, ent_sector: Coord, selected: Coord) -> String {
        let styles = self.theme().styles();

        let mut b = String::new();
        b.push_str(
            &styles
                .grid_header
                .render("  1   2   3   4   5   6   7   8  "),
        );

        for r in 1..=8 {
            b.push('\n');
            b.push_str(&styles.grid_header.render(&format!("{r} ")));
            for c in 1..=8 {
                let coord = [r, c];
                if let Some(ov) = self.anim_overrides.get(&coord) {
                    let glyph = pad_cell(&ov.glyph);
                    b.push_str(&ov.style.render(&glyph));
                    if c < 8 {
                        b.push(' ');
                    }
                    continue;
                }

                let mut ent = EntityType::Empty;
                if let Some(q) = quad {
                    ent = q.grid[r as usize][c as usize];
                }
                if ent_sector[0] == r && ent_sector[1] == c {
                    ent = EntityType::Enterprise;
                }
                let is_cloaked = match quad {
                    Some(q) if ent != EntityType::Enterprise => cloaked_at(q, r, c),
                    _ => false,
                };
                let is_selected = selected[0] == r && selected[1] == c;
                b.push_str(&entity_glyph(ent, &styles, is_selected, is_cloaked));
                if c < 8 {
                    b.push(' ');
                }
            }
        }

        styles.grid.render(&b)
    }
}

/// Whether a cloaked Klingon or adversary sits at `(r, c)`.
fn cloaked_at(q: &QuadrantState, r: i32, c: i32) -> bool {
    let at = |s: Coord| s[0] == r && s[1] == c;
    for k in q.klingons.iter().flatten() {
        let k = k.borrow();
        if at(k.sector) && k.is_cloaked {
            return true;
        }
    }
    for e in q.enemies.iter().flatten() {
        let e = e.borrow();
        if at(e.sector) && e.is_cloaked {
            return true;
        }
    }
    false
}

/// `entityGlyph`: the styled 3-character glyph, bracketed when selected.
fn entity_glyph(ent: EntityType, styles: &Styles, selected: bool, cloaked: bool) -> String {
    if cloaked {
        if selected {
            return styles.empty.clone().reverse(true).render("[?]");
        }
        return styles.empty.render(" ? ");
    }

    let (style, sel, plain) = match ent {
        EntityType::Enterprise => (&styles.enterprise, "[E]", "<E>"),
        EntityType::Klingon | EntityType::Commander | EntityType::SuperCommander => {
            (&styles.klingon, "[K]", "+K+")
        }
        EntityType::Romulan => (&styles.romulan, "[R]", "+R+"),
        EntityType::Tholian => (&styles.tholian, "[T]", "<T>"),
        EntityType::PlasmaTorpedo => (&styles.plasma_torpedo, "[P]", "*P*"),
        EntityType::TholianWeb => (&styles.tholian_web, "[:]", ":::"),
        EntityType::Starbase => (&styles.starbase, "[B]", ">B<"),
        EntityType::Star => (&styles.star, "[*]", " * "),
        EntityType::Planet => (&styles.planet, "[O]", " O "),
        EntityType::BlackHole => (&styles.black_hole, "[@]", " @ "),
        EntityType::Wormhole => (&styles.wormhole, "[W]", ">W<"),
        _ => (&styles.empty, "[.]", " . "),
    };
    if selected {
        style.clone().reverse(true).render(sel)
    } else {
        style.render(plain)
    }
}

/// `RenderCell`: one cell at `(r, c)` with default theme styling.
pub fn render_cell(r: i32, c: i32, quad: Option<&QuadrantState>, selected: bool) -> String {
    let styles = theme::default_theme().styles();
    let mut ent = EntityType::Empty;
    if let Some(q) = quad {
        if (1..=8).contains(&r) && (1..=8).contains(&c) {
            ent = q.grid[r as usize][c as usize];
        }
    }
    let is_cloaked = quad.is_some_and(|q| cloaked_at(q, r, c));
    entity_glyph(ent, &styles, selected, is_cloaked)
}

/// `padCell`: clamps/pads an override glyph to exactly 3 runes.
fn pad_cell(s: &str) -> String {
    let runes: Vec<char> = s.chars().collect();
    match runes.len() {
        0 => "   ".into(),
        1 => format!(" {s} "),
        2 => format!(" {s}"),
        3 => s.into(),
        _ => runes[..3].iter().collect(),
    }
}
