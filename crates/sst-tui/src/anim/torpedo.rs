//! Photon torpedo animation (port of `pkg/tui/anim/torpedo.go`).

use sst_engine::types::{Coord, EntityType};
use sst_tuikit::lipgloss;

use super::{
    bresenham_line, frame_duration, impl_animation, single, CellOverride, Frame, Overrides,
    Playback,
};

/// Go's unexported `torpedoAnimation`.
#[derive(Clone)]
pub struct TorpedoAnimation {
    pb: Playback,
}
impl_animation!(TorpedoAnimation);

/// `NewTorpedoAnimation`: flight and impact against a Klingon.
pub fn new_torpedo_animation(start: Coord, end: Coord, hit: bool, speed: i32) -> TorpedoAnimation {
    new_torpedo_animation_with_entity(start, end, hit, EntityType::Klingon, speed)
}

fn bold_fg(c: &str) -> lipgloss::Style {
    lipgloss::new_style()
        .foreground(lipgloss::color(c))
        .bold(true)
}

/// `NewTorpedoAnimationWithEntity`: keeps the target entity glyph visible at
/// the impact sector while the torpedo is in flight.
pub fn new_torpedo_animation_with_entity(
    start: Coord,
    end: Coord,
    hit: bool,
    entity: EntityType,
    speed: i32,
) -> TorpedoAnimation {
    let d = frame_duration(speed);
    let pts = bresenham_line(start, end);

    let flight_points: &[Coord] = if hit {
        if pts.len() > 2 {
            &pts[1..pts.len() - 1]
        } else {
            &[]
        }
    } else if pts.len() > 1 {
        &pts[1..]
    } else {
        &pts
    };

    const PROJECTILE_GLYPHS: [&str; 3] = [" · ", " o ", " O "];
    let torp_style = bold_fg("9");
    let target_override = target_cell_override(entity);

    let n = flight_points.len();
    let mut frames = Vec::new();
    for (i, &pt) in flight_points.iter().enumerate() {
        let idx = if n > 1 {
            ((i * PROJECTILE_GLYPHS.len()) / n).min(PROJECTILE_GLYPHS.len() - 1)
        } else {
            PROJECTILE_GLYPHS.len() - 1
        };
        let mut overrides = Overrides::new();
        overrides.insert(
            pt,
            CellOverride {
                glyph: PROJECTILE_GLYPHS[idx].to_string(),
                style: torp_style.clone(),
            },
        );
        if hit && pt != end {
            overrides.insert(end, target_override.clone());
        }
        frames.push(Frame {
            overrides,
            duration: d,
        });
    }

    if hit {
        frames.push(single(end, " * ", bold_fg("11"), d));
        frames.push(single(end, "***", bold_fg("9"), d));
        frames.push(single(end, "#*#", bold_fg("208"), d));
    } else {
        frames.push(single(end, "   ", lipgloss::new_style(), d));
    }

    TorpedoAnimation {
        pb: Playback::new(frames),
    }
}

/// `targetCellOverride`: in-flight glyph for the impact sector's entity.
fn target_cell_override(entity: EntityType) -> CellOverride {
    let (glyph, color) = match entity {
        EntityType::Romulan => ("+R+", "10"),
        EntityType::Tholian => ("<T>", "11"),
        EntityType::Starbase => (">B<", "14"),
        EntityType::Star => (" * ", "11"),
        EntityType::Planet => (" O ", "12"),
        EntityType::BlackHole => (" @ ", "13"),
        EntityType::PlasmaTorpedo => ("*P*", "9"),
        EntityType::TholianWeb => (":::", "11"),
        _ => ("+K+", "9"),
    };
    CellOverride {
        glyph: glyph.to_string(),
        style: bold_fg(color),
    }
}
