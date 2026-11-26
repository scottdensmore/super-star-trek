//! Bresenham line rasterisation (port of `pkg/tui/anim/line.go`).

use sst_engine::types::Coord;

/// `BresenhamLine`: grid coordinates from `start` to `end` inclusive.
pub fn bresenham_line(start: Coord, end: Coord) -> Vec<Coord> {
    let mut points = Vec::new();
    let (mut x0, mut y0) = (start[1], start[0]);
    let (x1, y1) = (end[1], end[0]);

    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 > x1 { -1 } else { 1 };
    let sy = if y0 > y1 { -1 } else { 1 };
    let mut err = dx + dy;

    loop {
        points.push([y0, x0]);
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
    points
}
