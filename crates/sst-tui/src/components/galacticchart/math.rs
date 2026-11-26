//! Warp telemetry math (port of `pkg/tui/components/galacticchart/math.go`).

use sst_engine::gomath;
use sst_engine::Coord;

/// Navigation telemetry between two quadrants (Go `Telemetry`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Telemetry {
    pub from: Coord,
    pub to: Coord,
    pub delta_r: i32,
    pub delta_c: i32,
    pub distance: f64,
    pub course: f64,
    pub direction: String,
    pub recommended_warp: f64,
    pub is_current: bool,
}

/// `CalculateTelemetry`.
pub fn calculate_telemetry(from: Coord, to: Coord) -> Telemetry {
    if from == to {
        return Telemetry {
            from,
            to,
            direction: "Current".into(),
            is_current: true,
            recommended_warp: 0.0,
            ..Default::default()
        };
    }

    let dr = to[0] - from[0];
    let dc = to[1] - from[1];
    let dist = gomath::sqrt((dr * dr + dc * dc) as f64);

    let mut angle = gomath::atan2(-dr as f64, dc as f64);
    if angle < 0.0 {
        angle += 2.0 * gomath::PI;
    }

    let mut warp = gomath::round(dist * 10.0) / 10.0;
    if warp < 1.0 {
        warp = 1.0;
    }

    Telemetry {
        from,
        to,
        delta_r: dr,
        delta_c: dc,
        distance: dist,
        course: angle,
        direction: bearing_direction(angle).into(),
        recommended_warp: warp,
        is_current: false,
    }
}

fn bearing_direction(angle: f64) -> &'static str {
    let deg = angle * 180.0 / gomath::PI;
    let within = |lo: f64, hi: f64| deg >= lo && deg < hi;
    if deg >= 337.5 || deg < 22.5 {
        "East"
    } else if within(22.5, 67.5) {
        "North-East"
    } else if within(67.5, 112.5) {
        "North"
    } else if within(112.5, 157.5) {
        "North-West"
    } else if within(157.5, 202.5) {
        "West"
    } else if within(202.5, 247.5) {
        "South-West"
    } else if within(247.5, 292.5) {
        "South"
    } else {
        "South-East"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculate_telemetry_cardinal_directions() {
        let from = [4, 4];
        let pi = std::f64::consts::PI;
        let cases: [(&str, Coord, f64, f64, &str, f64); 9] = [
            ("Same Quadrant", [4, 4], 0.0, 0.0, "Current", 0.0),
            ("Due North", [2, 4], 2.0, pi / 2.0, "North", 2.0),
            ("Due East", [4, 7], 3.0, 0.0, "East", 3.0),
            ("Due South", [7, 4], 3.0, 3.0 * pi / 2.0, "South", 3.0),
            ("Due West", [4, 1], 3.0, pi, "West", 3.0),
            (
                "North-East",
                [2, 6],
                8f64.sqrt(),
                pi / 4.0,
                "North-East",
                2.8,
            ),
            (
                "North-West",
                [2, 2],
                8f64.sqrt(),
                3.0 * pi / 4.0,
                "North-West",
                2.8,
            ),
            (
                "South-West",
                [6, 2],
                8f64.sqrt(),
                5.0 * pi / 4.0,
                "South-West",
                2.8,
            ),
            (
                "South-East",
                [6, 6],
                8f64.sqrt(),
                7.0 * pi / 4.0,
                "South-East",
                2.8,
            ),
        ];
        for (name, to, dist, course, dir, warp) in cases {
            let res = calculate_telemetry(from, to);
            assert!(
                (res.distance - dist).abs() <= 0.05,
                "{name}: distance {}",
                res.distance
            );
            if to != from {
                assert!(
                    (res.course - course).abs() <= 0.05,
                    "{name}: course {}",
                    res.course
                );
            }
            assert_eq!(res.direction, dir, "{name}");
            assert!((res.recommended_warp - warp).abs() <= 0.05, "{name}: warp");
        }
    }

    #[test]
    fn calculate_telemetry_fields() {
        let coord = [3, 5];
        let res = calculate_telemetry(coord, coord);
        assert!(res.is_current);
        assert_eq!((res.from, res.to), (coord, coord));
        assert_eq!((res.delta_r, res.delta_c), (0, 0));
        assert_eq!(res.distance, 0.0);
        assert_eq!(res.recommended_warp, 0.0);
        assert_eq!(res.direction, "Current");

        let (from, to) = ([1, 1], [2, 3]);
        let res = calculate_telemetry(from, to);
        assert!(!res.is_current);
        assert_eq!((res.from, res.to), (from, to));
        assert_eq!((res.delta_r, res.delta_c), (1, 2));
        let expected = 5f64.sqrt();
        assert!((res.distance - expected).abs() <= 0.001);
        assert_eq!(res.recommended_warp, (expected * 10.0).round() / 10.0);

        let res = calculate_telemetry([3, 3], [3, 4]);
        assert!(res.recommended_warp >= 1.0);
    }
}
