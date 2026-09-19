package engine

import "testing"

func TestAnomaliesGenerationDisabledByDefault(t *testing.T) {
	g := NewGame(42, SkillGood, LengthMedium)
	if g.Rules.SpatialAnomalies {
		t.Errorf("expected SpatialAnomalies to be false by default in normal profile")
	}
	for r := 1; r <= 8; r++ {
		for c := 1; c <= 8; c++ {
			if g.QuadrantEnv[r][c] != EnvNormal {
				t.Errorf("expected EnvNormal at [%d,%d], got %v", r, c, g.QuadrantEnv[r][c])
			}
		}
	}
}

func TestAnomaliesGenerationEnabled(t *testing.T) {
	rules := DefaultRulesForProfile(ProfileHardcore)
	if !rules.SpatialAnomalies {
		t.Fatalf("expected Hardcore profile to have SpatialAnomalies enabled")
	}

	g := NewGameWithOptions(1337, SkillExpert, LengthMedium, rules)

	nebulaCount := 0
	ionStormCount := 0
	for r := 1; r <= 8; r++ {
		for c := 1; c <= 8; c++ {
			switch g.QuadrantEnv[r][c] {
			case EnvNebula:
				nebulaCount++
			case EnvIonStorm:
				ionStormCount++
			}
		}
	}

	if nebulaCount < 2 || nebulaCount > 4 {
		t.Errorf("expected between 2 and 4 nebulae, got %d", nebulaCount)
	}
	if ionStormCount < 2 || ionStormCount > 4 {
		t.Errorf("expected between 2 and 4 ion storms, got %d", ionStormCount)
	}

	// Starting quadrant must not be an anomaly
	startQ := g.Enterprise.Quad
	if g.QuadrantEnv[startQ[0]][startQ[1]] != EnvNormal {
		t.Errorf("starting quadrant [%d,%d] must be EnvNormal, got %v", startQ[0], startQ[1], g.QuadrantEnv[startQ[0]][startQ[1]])
	}
}
