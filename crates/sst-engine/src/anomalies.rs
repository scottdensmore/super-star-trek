//! Spatial anomalies (port of `pkg/engine/anomalies.go`).

use crate::state::GameState;
use crate::types::EnvironmentType;

/// Go `SeedAnomalies`: 2-4 nebulae and 2-4 ion storms away from the start quadrant.
pub fn seed_anomalies(g: &mut GameState) {
    if !g.rules.spatial_anomalies || g.rng.is_none() {
        return;
    }
    let start = g.enterprise.quad;
    let num_nebulae = 2 + g.rng().intn(3);
    let mut placed = 0;
    while placed < num_nebulae {
        let r = g.rng().intn(8) + 1;
        let c = g.rng().intn(8) + 1;
        if [r, c] != start && g.quadrant_env[r as usize][c as usize] == EnvironmentType::Normal {
            g.quadrant_env[r as usize][c as usize] = EnvironmentType::Nebula;
            placed += 1;
        }
    }
    let num_storms = 2 + g.rng().intn(3);
    let mut placed = 0;
    while placed < num_storms {
        let r = g.rng().intn(8) + 1;
        let c = g.rng().intn(8) + 1;
        if [r, c] != start && g.quadrant_env[r as usize][c as usize] == EnvironmentType::Normal {
            g.quadrant_env[r as usize][c as usize] = EnvironmentType::IonStorm;
            placed += 1;
        }
    }
}
