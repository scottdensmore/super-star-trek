//! Phaser beam animation (port of `pkg/tui/anim/phaser.go`).

use sst_engine::types::Coord;
use sst_tuikit::lipgloss;

use super::{
    bresenham_line, frame_duration, impl_animation, CellOverride, Frame, Overrides, Playback,
};

/// Go's unexported `phaserAnimation`.
#[derive(Clone)]
pub struct PhaserAnimation {
    pb: Playback,
}
impl_animation!(PhaserAnimation);

/// `beamGlyph`: directional ray glyph from the row/column deltas.
fn beam_glyph(dr: i32, dc: i32) -> &'static str {
    let abs_r = dr.abs();
    let abs_c = dc.abs();
    if abs_r == 0 {
        return "---";
    }
    if abs_c == 0 {
        return " | ";
    }
    if abs_c >= 2 * abs_r {
        return "---";
    }
    if abs_r >= 2 * abs_c {
        return " | ";
    }
    if (dr > 0 && dc > 0) || (dr < 0 && dc < 0) {
        return " \\ ";
    }
    " / "
}

/// `NewMultiPhaserAnimation`: multi-target beam raycast and shield flash
/// (always 4 frames).
pub fn new_multi_phaser_animation(
    start: Coord,
    targets: &[Coord],
    hits: &[bool],
    speed: i32,
) -> PhaserAnimation {
    let d = frame_duration(speed);
    let beam_style = lipgloss::new_style()
        .foreground(lipgloss::color("11"))
        .bold(true);
    let target_style = lipgloss::new_style()
        .foreground(lipgloss::color("14"))
        .bold(true);

    let mut overrides = Overrides::new();
    for (i, &target) in targets.iter().enumerate() {
        let hit = hits.get(i).copied().unwrap_or(false);
        let line = bresenham_line(start, target);
        let glyph = beam_glyph(target[0] - start[0], target[1] - start[1]);
        if line.len() > 2 {
            for &c in &line[1..line.len() - 1] {
                overrides.insert(
                    c,
                    CellOverride {
                        glyph: glyph.to_string(),
                        style: beam_style.clone(),
                    },
                );
            }
        }
        if hit {
            overrides.insert(
                target,
                CellOverride {
                    glyph: "<K>".to_string(),
                    style: target_style.clone(),
                },
            );
        }
    }

    let frames = (0..4)
        .map(|i| {
            let mut f_overrides = overrides.clone();
            if i == 1 || i == 2 {
                for target in targets {
                    if let Some(ov) = f_overrides.get_mut(target) {
                        if ov.glyph == "<K>" {
                            *ov = CellOverride {
                                glyph: "*K*".to_string(),
                                style: lipgloss::new_style()
                                    .foreground(lipgloss::color("11"))
                                    .bold(true),
                            };
                        }
                    }
                }
            }
            Frame {
                overrides: f_overrides,
                duration: d,
            }
        })
        .collect();

    PhaserAnimation {
        pb: Playback::new(frames),
    }
}

/// `NewPhaserAnimation`: single-target phaser.
pub fn new_phaser_animation(start: Coord, target: Coord, hit: bool, speed: i32) -> PhaserAnimation {
    new_multi_phaser_animation(start, &[target], &[hit], speed)
}
