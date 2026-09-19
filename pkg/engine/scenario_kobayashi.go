package engine

// BuildKobayashiMaru creates the initial GameState for the Kobayashi Maru simulation.
func BuildKobayashiMaru(seed int64) *GameState {
	rng := NewPRNG(seed)
	rules := DefaultRulesForProfile(ProfileNormal)
	g := NewGameWithOptions(seed, SkillExpert, LengthMedium, rules)
	g.RNG = rng
	g.Scenario = ScenarioKobayashiMaru
	g.Enterprise.Quad = Coord{4, 4}
	g.Enterprise.Sector = Coord{4, 4}
	g.Enterprise.Energy = 5000
	g.Enterprise.Shields = 2500
	g.Enterprise.Torpedoes = 10
	g.Enterprise.Condition = ConditionRed

	// Strip starbases across galaxy in Neutral Zone scenario
	g.RemainingStarbases = 0
	for r := 1; r <= 8; r++ {
		for c := 1; c <= 8; c++ {
			g.GalaxyChart[r][c] = (g.GalaxyChart[r][c] / 100) * 100 + (g.GalaxyChart[r][c] % 10)
			g.ChartKnownBases[r][c] = false
		}
	}

	// Quadrant [4, 4] layout
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

	// Place 3 initial Klingon Battlecruisers
	g.CurrentQuad.Klingons = make([]*Klingon, 0, 3)
	for i := 0; i < 3; i++ {
		sec := findEmptySector()
		k := &Klingon{
			ID:          i + 1,
			Sector:      sec,
			Energy:      400.0 + g.RNG.Float64()*100.0,
			IsCommander: false,
			IsCloaked:   false,
		}
		g.CurrentQuad.Grid[sec[0]][sec[1]] = EntityKlingon
		g.CurrentQuad.Klingons = append(g.CurrentQuad.Klingons, k)
	}

	// Place 3 stars
	g.CurrentQuad.Stars = make([]Coord, 0, 3)
	for i := 0; i < 3; i++ {
		sec := findEmptySector()
		g.CurrentQuad.Grid[sec[0]][sec[1]] = EntityStar
		g.CurrentQuad.Stars = append(g.CurrentQuad.Stars, sec)
	}

	g.GalaxyChart[4][4] = 300 + len(g.CurrentQuad.Stars)
	g.ChartDiscovered[4][4] = true
	g.RemainingKlingons = 100

	return g
}

// EvaluateKobayashiMaru executes wave reinforcement and checks for ship destruction.
func EvaluateKobayashiMaru(g *GameState) (done bool, won bool, reason GameOverReason) {
	if g == nil {
		return true, false, GameOverLost
	}

	// Loss condition: Enterprise destroyed or out of energy/time
	if g.Enterprise.Energy <= 0 || g.TimeRemaining <= 0 {
		return true, false, GameOverLost
	}

	// Wave reinforcement in quadrant [4, 4]
	if g.Enterprise.Quad == (Coord{4, 4}) {
		var activeKlingons []*Klingon
		for _, k := range g.CurrentQuad.Klingons {
			if k != nil && k.Energy > 0 && g.CurrentQuad.Grid[k.Sector[0]][k.Sector[1]] == EntityKlingon {
				activeKlingons = append(activeKlingons, k)
			}
		}
		g.CurrentQuad.Klingons = activeKlingons

		for len(g.CurrentQuad.Klingons) < 3 {
			// Find empty edge sectors (row 1, 8 or col 1, 8)
			var edgeSectors []Coord
			for r := 1; r <= 8; r++ {
				for c := 1; c <= 8; c++ {
					if (r == 1 || r == 8 || c == 1 || c == 8) && g.CurrentQuad.Grid[r][c] == EntityEmpty {
						edgeSectors = append(edgeSectors, Coord{r, c})
					}
				}
			}

			var spawnCoord Coord
			if len(edgeSectors) > 0 {
				idx := 0
				if g.RNG != nil {
					idx = g.RNG.Intn(len(edgeSectors))
				}
				spawnCoord = edgeSectors[idx]
			} else {
				var anyEmpty []Coord
				for r := 1; r <= 8; r++ {
					for c := 1; c <= 8; c++ {
						if g.CurrentQuad.Grid[r][c] == EntityEmpty {
							anyEmpty = append(anyEmpty, Coord{r, c})
						}
					}
				}
				if len(anyEmpty) == 0 {
					break
				}
				idx := 0
				if g.RNG != nil {
					idx = g.RNG.Intn(len(anyEmpty))
				}
				spawnCoord = anyEmpty[idx]
			}

			maxID := 0
			for _, k := range g.CurrentQuad.Klingons {
				if k.ID > maxID {
					maxID = k.ID
				}
			}

			energy := 400.0
			if g.RNG != nil {
				energy += g.RNG.Float64() * 100.0
			}

			newK := &Klingon{
				ID:          maxID + 1,
				Sector:      spawnCoord,
				Energy:      energy,
				IsCommander: false,
				IsCloaked:   false,
			}
			g.CurrentQuad.Grid[spawnCoord[0]][spawnCoord[1]] = EntityKlingon
			g.CurrentQuad.Klingons = append(g.CurrentQuad.Klingons, newK)
			g.GalaxyChart[4][4] += 100
		}
	}

	return false, false, GameOverLost
}

// ComputeScoreKobayashiMaru calculates the Starfleet Tactical Commendation score.
func ComputeScoreKobayashiMaru(g *GameState, won bool) ScoreBreakdown {
	sb := ComputeScore(g, false)
	if g == nil {
		return sb
	}

	elapsed := g.Stardate - g.InitialStardate
	kills := g.Metrics.KlingonsKilled + g.Metrics.CommandersKilled + g.Metrics.SuperCommandersKilled

	var title, badge string
	if kills >= 10 || elapsed >= 8.0 {
		title = "[COMM-4] Admiral's Citation for Gallantry"
		badge = "[COMM-4]"
	} else if kills >= 6 || elapsed >= 4.0 {
		title = "[COMM-3] Starfleet Cross of Honor"
		badge = "[COMM-3]"
	} else if kills >= 3 || elapsed >= 2.0 {
		title = "[COMM-2] Commendation for Tactical Excellence"
		badge = "[COMM-2]"
	} else {
		title = "[COMM-1] Cadet Commendation for Valor"
		badge = "[COMM-1]"
	}

	sb.RankTitle = title
	sb.RankBadge = badge
	return sb
}
