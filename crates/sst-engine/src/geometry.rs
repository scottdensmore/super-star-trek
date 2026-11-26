//! Coordinate geometry (port of `pkg/engine/geometry.go`).

use crate::gomath;
use crate::types::Coord;

/// Go `Distance`: Euclidean distance between two coordinates.
pub fn distance(c1: Coord, c2: Coord) -> f64 {
    let dr = (c1[0] - c2[0]) as f64;
    let dc = (c1[1] - c2[1]) as f64;
    gomath::hypot(dr, dc)
}

/// Go `Bearing`: angle in radians from `from` to `to` (0 = east, pi/2 = north).
pub fn bearing(from: Coord, to: Coord) -> f64 {
    let dr = (to[0] - from[0]) as f64;
    let dc = (to[1] - from[1]) as f64;
    let mut angle = gomath::atan2(-dr, dc);
    if angle < 0.0 {
        angle += 2.0 * gomath::PI;
    }
    angle
}
