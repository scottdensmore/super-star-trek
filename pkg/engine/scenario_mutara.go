package engine

// BuildMutaraNebula creates the initial GameState for the Mutara Nebula tactical duel.
func BuildMutaraNebula(seed int64) *GameState {
	rng := NewPRNG(seed)
	rules := DefaultRulesForProfile(ProfileNormal)
	rules.KlingonCloak = true
	g := NewGameWithOptions(seed, SkillExpert, LengthShort, rules)
	g.RNG = rng
	g.Scenario = ScenarioMutaraNebula
	g.Enterprise.Quad = Coord{5, 5}
	g.Enterprise.Sector = Coord{4, 4}
	g.Enterprise.Energy = 5000
	g.Enterprise.Shields = 0
	g.Enterprise.Torpedoes = 10
	g.Enterprise.Condition = ConditionRed
	g.QuadrantEnv[5][5] = EnvNebula

	// No starbases and no other Klingons outside [5, 5] in nebula duel
	g.RemainingStarbases = 0
	for r := 1; r <= 8; r++ {
		for c := 1; c <= 8; c++ {
			stars := g.GalaxyChart[r][c] % 10
			g.GalaxyChart[r][c] = stars
			g.ChartKnownBases[r][c] = false
		}
	}

	// Quadrant layout
	g.CurrentQuad = QuadrantState{}
	g.CurrentQuad.Grid[4][4] = EntityEnterprise

	findEmptySector := func() Coord {
		for {
			r := g.RNG.Intn(8) + 1
			c := g.RNG.Intn(8) + 1
			if g.CurrentQuad.Grid[r][c] == EntityEmpty {
				return Coord{r, c}
			}
		}
	}

	// Place 1 cloaked Super-Commander with high energy
	cmdSector := findEmptySector()
	k := &Klingon{
		ID:          1,
		Sector:      cmdSector,
		Energy:      1200,
		Shields:     0,
		IsCommander: true,
		IsCloaked:   true,
	}
	g.CurrentQuad.Grid[cmdSector[0]][cmdSector[1]] = EntitySuperCommander
	g.CurrentQuad.Klingons = []*Klingon{k}

	// Place 3 stars
	g.CurrentQuad.Stars = make([]Coord, 0, 3)
	for i := 0; i < 3; i++ {
		sec := findEmptySector()
		g.CurrentQuad.Grid[sec[0]][sec[1]] = EntityStar
		g.CurrentQuad.Stars = append(g.CurrentQuad.Stars, sec)
	}

	g.GalaxyChart[5][5] = 100 + len(g.CurrentQuad.Stars)
	g.ChartDiscovered[5][5] = true
	g.RemainingKlingons = 1

	return g
}

// EvaluateMutaraNebula checks for Super-Commander elimination, fleeing, or Enterprise destruction.
func EvaluateMutaraNebula(g *GameState) (done bool, won bool, reason GameOverReason) {
	if g == nil {
		return true, false, GameOverLost
	}

	// Victory condition: Super-Commander destroyed
	if g.Metrics.SuperCommandersKilled >= 1 || g.Metrics.CommandersKilled >= 1 || g.Metrics.KlingonsKilled >= 1 {
		return true, true, GameOverWon
	}

	// Abandonment condition: fleeing the Mutara Nebula quadrant
	if g.Enterprise.Quad != (Coord{5, 5}) {
		return true, false, GameOverLost
	}

	// Loss condition: Enterprise destroyed or energy/time depleted
	if g.Enterprise.Energy <= 0 || g.TimeRemaining <= 0 {
		return true, false, GameOverLost
	}

	return false, false, GameOverLost
}

// ComputeScoreMutaraNebula calculates the score breakdown for the Mutara Nebula duel.
func ComputeScoreMutaraNebula(g *GameState, won bool) ScoreBreakdown {
	return ComputeScore(g, won)
}
