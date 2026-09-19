package engine

// BuildStarbaseSiege creates the initial GameState for the Starbase Under Siege scenario.
func BuildStarbaseSiege(seed int64) *GameState {
	rng := NewPRNG(seed)
	rules := DefaultRulesForProfile(ProfileNormal)
	g := NewGameWithOptions(seed, SkillExpert, LengthShort, rules)
	g.RNG = rng
	g.Scenario = ScenarioStarbaseSiege
	g.Enterprise.Quad = Coord{2, 2}
	g.Enterprise.Sector = Coord{4, 4}
	g.Enterprise.Energy = 5000
	g.Enterprise.Shields = 1500
	g.Enterprise.Torpedoes = 10
	g.TimeRemaining = 6.0
	g.RemainingStarbases = 1
	g.RemainingKlingons = 3

	// Clear all other starbases and Klingons from the galaxy
	for r := 1; r <= 8; r++ {
		for c := 1; c <= 8; c++ {
			stars := g.GalaxyChart[r][c] % 10
			g.GalaxyChart[r][c] = stars
			g.ChartKnownBases[r][c] = false
		}
	}

	// Quadrant [4, 4] contains Starbase 12 and 3 attacking Klingons
	stars := g.GalaxyChart[4][4] % 10
	if stars == 0 {
		stars = 3
	}
	g.GalaxyChart[4][4] = 310 + stars
	g.ChartKnownBases[4][4] = true

	// Populate starting quadrant [2, 2]
	g.PopulateQuadrant(Coord{2, 2}, Coord{4, 4})
	g.Enterprise.Condition = ConditionGreen

	return g
}

// EvaluateStarbaseSiege checks whether Starbase 12 survived or the siege fleet was eliminated.
func EvaluateStarbaseSiege(g *GameState) (done bool, won bool, reason GameOverReason) {
	if g == nil {
		return true, false, GameOverLost
	}

	// Immediate loss if Starbase 12 is destroyed
	if g.RemainingStarbases == 0 || g.Metrics.StarbasesDestroyed > 0 || (g.GalaxyChart[4][4]%100)/10 == 0 {
		return true, false, GameOverLost
	}

	// Time expired or Enterprise destroyed
	if g.TimeRemaining <= 0 || g.Enterprise.Energy <= 0 {
		return true, false, GameOverLost
	}

	// Victory if all 3 siege cruisers eliminated
	totalKills := g.Metrics.KlingonsKilled + g.Metrics.CommandersKilled + g.Metrics.SuperCommandersKilled
	if totalKills >= 3 || (g.RemainingKlingons == 0 && (g.Enterprise.Quad != (Coord{4, 4}) || len(g.CurrentQuad.Klingons) == 0)) {
		return true, true, GameOverWon
	}

	return false, false, GameOverLost
}

// ComputeScoreStarbaseSiege calculates the final score for the Starbase Siege defense.
func ComputeScoreStarbaseSiege(g *GameState, won bool) ScoreBreakdown {
	return ComputeScore(g, won)
}
