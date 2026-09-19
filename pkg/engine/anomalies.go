package engine

// SeedAnomalies proceduralizes nebulae, ion storms, black holes, and wormholes across the galaxy.
func SeedAnomalies(g *GameState) {
	if !g.Rules.SpatialAnomalies || g.RNG == nil {
		return
	}

	startQuad := g.Enterprise.Quad

	// 1. Seed 2-4 Nebulae
	numNebulae := 2 + g.RNG.Intn(3) // 2..4
	placedNebulae := 0
	for placedNebulae < numNebulae {
		r := g.RNG.Intn(8) + 1
		c := g.RNG.Intn(8) + 1
		if (Coord{r, c}) != startQuad && g.QuadrantEnv[r][c] == EnvNormal {
			g.QuadrantEnv[r][c] = EnvNebula
			placedNebulae++
		}
	}

	// 2. Seed 2-4 Ion Storms
	numStorms := 2 + g.RNG.Intn(3) // 2..4
	placedStorms := 0
	for placedStorms < numStorms {
		r := g.RNG.Intn(8) + 1
		c := g.RNG.Intn(8) + 1
		if (Coord{r, c}) != startQuad && g.QuadrantEnv[r][c] == EnvNormal {
			g.QuadrantEnv[r][c] = EnvIonStorm
			placedStorms++
		}
	}
}
