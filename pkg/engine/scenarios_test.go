package engine

import "testing"

func TestScenarioRegistry_LookupAndAliases(t *testing.T) {
	cases := []struct {
		input    string
		expected ScenarioID
		found    bool
	}{
		{"kobayashi-maru", ScenarioKobayashiMaru, true},
		{"kobayashi", ScenarioKobayashiMaru, true},
		{"km", ScenarioKobayashiMaru, true},
		{"Kobayashi-Maru", ScenarioKobayashiMaru, true},
		{"KM", ScenarioKobayashiMaru, true},
		{"mutara-nebula", ScenarioMutaraNebula, true},
		{"mutara", ScenarioMutaraNebula, true},
		{"nebula", ScenarioMutaraNebula, true},
		{"MUTARA", ScenarioMutaraNebula, true},
		{"starbase-siege", ScenarioStarbaseSiege, true},
		{"siege", ScenarioStarbaseSiege, true},
		{"starbase", ScenarioStarbaseSiege, true},
		{"Starbase-Siege", ScenarioStarbaseSiege, true},
		{"unknown-scenario", "", false},
		{"", "", false},
	}

	for _, tc := range cases {
		s, ok := GetScenario(ScenarioID(tc.input))
		if ok != tc.found {
			t.Errorf("GetScenario(%q): expected found=%v, got %v", tc.input, tc.found, ok)
		}
		if ok && s.ID != tc.expected {
			t.Errorf("GetScenario(%q): expected ID=%v, got %v", tc.input, tc.expected, s.ID)
		}
	}
}

func TestListScenarios_Ordering(t *testing.T) {
	list := ListScenarios()
	if len(list) < 3 {
		t.Fatalf("expected at least 3 registered scenarios, got %d", len(list))
	}
	expected := []ScenarioID{ScenarioKobayashiMaru, ScenarioMutaraNebula, ScenarioStarbaseSiege}
	for i, expID := range expected {
		if list[i].ID != expID {
			t.Errorf("scenario index %d: expected %v, got %v", i, expID, list[i].ID)
		}
	}
}

func TestRegisterScenario_Custom(t *testing.T) {
	scenarioMu.Lock()
	origMap := make(map[ScenarioID]*Scenario)
	for k, v := range scenarioMap {
		origMap[k] = v
	}
	origAlias := make(map[string]ScenarioID)
	for k, v := range scenarioAlias {
		origAlias[k] = v
	}
	origOrder := append([]ScenarioID(nil), scenarioOrder...)
	scenarioMu.Unlock()

	t.Cleanup(func() {
		scenarioMu.Lock()
		scenarioMap = origMap
		scenarioAlias = origAlias
		scenarioOrder = origOrder
		scenarioMu.Unlock()
	})

	customID := ScenarioID("custom-challenge")
	custom := &Scenario{
		ID:          customID,
		Name:        "Custom Challenge",
		Subtitle:    "Testing Registry",
		Difficulty:  "Test",
		Description: "A test scenario",
		Aliases:     []string{"custom", "cc"},
	}

	RegisterScenario(custom)

	s, ok := GetScenario(customID)
	if !ok || s == nil {
		t.Fatalf("expected custom scenario to be found")
	}
	if s.Name != "Custom Challenge" {
		t.Errorf("expected Name %q, got %q", "Custom Challenge", s.Name)
	}

	// Test alias lookup
	sAlias, ok := GetScenario("cc")
	if !ok || sAlias == nil {
		t.Fatalf("expected custom scenario via alias 'cc' to be found")
	}
	if sAlias.ID != customID {
		t.Errorf("expected ID %v, got %v", customID, sAlias.ID)
	}
}

func TestScenario_KobayashiMaru_Mechanics(t *testing.T) {
	s, ok := GetScenario(ScenarioKobayashiMaru)
	if !ok {
		t.Fatalf("failed to find Kobayashi Maru scenario")
	}

	game := s.Build(42)
	if game.Scenario != ScenarioKobayashiMaru {
		t.Errorf("expected scenario ID %v, got %v", ScenarioKobayashiMaru, game.Scenario)
	}
	if game.Enterprise.Quad != (Coord{4, 4}) {
		t.Errorf("expected Enterprise in Quad [4, 4], got %v", game.Enterprise.Quad)
	}
	if len(game.CurrentQuad.Klingons) != 3 {
		t.Errorf("expected 3 Klingons initially, got %d", len(game.CurrentQuad.Klingons))
	}

	// Test Commendation scoring on loss
	game.Metrics.KlingonsKilled = 4
	game.Stardate = game.InitialStardate + 3.0
	score := s.ComputeScore(game, false)
	if score.RankTitle != "[COMM-2] Commendation for Tactical Excellence" {
		t.Errorf("expected [COMM-2], got %s", score.RankTitle)
	}
}

func TestScenario_KobayashiMaru_WaveReinforcement(t *testing.T) {
	s, ok := GetScenario(ScenarioKobayashiMaru)
	if !ok {
		t.Fatalf("failed to find Kobayashi Maru scenario")
	}

	game := s.Build(42)
	if len(game.CurrentQuad.Klingons) != 3 {
		t.Fatalf("expected 3 Klingons initially, got %d", len(game.CurrentQuad.Klingons))
	}

	// Simulate eliminating two Klingons
	k1 := game.CurrentQuad.Klingons[0]
	game.CurrentQuad.Klingons = []*Klingon{k1}
	for r := 1; r <= 8; r++ {
		for c := 1; c <= 8; c++ {
			if game.CurrentQuad.Grid[r][c] == EntityKlingon && (Coord{r, c}) != k1.Sector {
				game.CurrentQuad.Grid[r][c] = EntityEmpty
			}
		}
	}

	// Evaluate should trigger wave reinforcement and spawn back up to 3 Klingons
	done, won, _ := s.Evaluate(game)
	if done || won {
		t.Errorf("expected game to continue, got done=%v won=%v", done, won)
	}
	if len(game.CurrentQuad.Klingons) != 3 {
		t.Errorf("expected 3 Klingons after reinforcement, got %d", len(game.CurrentQuad.Klingons))
	}
}

func TestScenario_KobayashiMaru_CommendationTiers(t *testing.T) {
	s, ok := GetScenario(ScenarioKobayashiMaru)
	if !ok {
		t.Fatalf("failed to find Kobayashi Maru scenario")
	}

	cases := []struct {
		kills    int
		elapsed  float64
		expected string
		badge    string
	}{
		{1, 1.0, "[COMM-1] Cadet Commendation for Valor", "[COMM-1]"},
		{4, 3.0, "[COMM-2] Commendation for Tactical Excellence", "[COMM-2]"},
		{7, 5.0, "[COMM-3] Starfleet Cross of Honor", "[COMM-3]"},
		{11, 2.0, "[COMM-4] Admiral's Citation for Gallantry", "[COMM-4]"},
		{2, 9.0, "[COMM-4] Admiral's Citation for Gallantry", "[COMM-4]"},
	}

	for _, tc := range cases {
		game := s.Build(42)
		game.Metrics.KlingonsKilled = tc.kills
		game.Stardate = game.InitialStardate + tc.elapsed
		score := s.ComputeScore(game, false)
		if score.RankTitle != tc.expected {
			t.Errorf("kills=%d elapsed=%f: expected %s, got %s", tc.kills, tc.elapsed, tc.expected, score.RankTitle)
		}
		if score.RankBadge != tc.badge {
			t.Errorf("kills=%d elapsed=%f: expected badge %s, got %s", tc.kills, tc.elapsed, tc.badge, score.RankBadge)
		}
	}
}

func TestScenario_MutaraNebula_Mechanics(t *testing.T) {
	s, ok := GetScenario(ScenarioMutaraNebula)
	if !ok {
		t.Fatalf("failed to find Mutara Nebula scenario")
	}

	game := s.Build(42)
	if game.Enterprise.Quad != (Coord{5, 5}) {
		t.Errorf("expected Enterprise at [5, 5], got %v", game.Enterprise.Quad)
	}
	if game.QuadrantEnv[5][5] != EnvNebula {
		t.Errorf("expected EnvNebula at [5, 5], got %v", game.QuadrantEnv[5][5])
	}
	if game.Enterprise.Shields != 0 {
		t.Errorf("expected 0 shields in Mutara Nebula, got %f", game.Enterprise.Shields)
	}

	// Kill the Super-Commander -> Evaluate should return won
	game.Metrics.SuperCommandersKilled = 1
	done, won, _ := s.Evaluate(game)
	if !done || !won {
		t.Errorf("expected victory when SuperCommander is killed, got done=%v, won=%v", done, won)
	}
}

func TestScenario_StarbaseSiege_Mechanics(t *testing.T) {
	s, ok := GetScenario(ScenarioStarbaseSiege)
	if !ok {
		t.Fatalf("failed to find Starbase Siege scenario")
	}

	game := s.Build(42)
	if game.Enterprise.Quad != (Coord{2, 2}) {
		t.Errorf("expected Enterprise at [2, 2], got %v", game.Enterprise.Quad)
	}
	if game.TimeRemaining != 6.0 {
		t.Errorf("expected 6.0 stardates time limit, got %f", game.TimeRemaining)
	}

	// If Starbase 12 in [4, 4] is destroyed, Evaluate must return loss
	game.RemainingStarbases = 0
	done, won, reason := s.Evaluate(game)
	if !done || won || reason != GameOverLost {
		t.Errorf("expected immediate loss when starbase is destroyed, got done=%v won=%v reason=%v", done, won, reason)
	}
}

func TestScenario_StarbaseSiege_Victory(t *testing.T) {
	s, ok := GetScenario(ScenarioStarbaseSiege)
	if !ok {
		t.Fatalf("failed to find Starbase Siege scenario")
	}

	game := s.Build(42)
	game.Metrics.KlingonsKilled = 3
	game.RemainingKlingons = 0
	done, won, reason := s.Evaluate(game)
	if !done || !won || reason != GameOverWon {
		t.Errorf("expected victory when siege cruisers eliminated, got done=%v won=%v reason=%v", done, won, reason)
	}
}

func TestScenario_ActionMove_TurnHook(t *testing.T) {
	s, ok := GetScenario(ScenarioMutaraNebula)
	if !ok {
		t.Fatalf("failed to find Mutara Nebula scenario")
	}

	game := s.Build(42)
	// Trigger victory condition
	game.Metrics.SuperCommandersKilled = 1

	// Destination in current quadrant
	dest := Coord{game.Enterprise.Sector[0], game.Enterprise.Sector[1] + 1}
	if dest[1] > 8 {
		dest[1] = game.Enterprise.Sector[1] - 1
	}
	game.CurrentQuad.Grid[dest[0]][dest[1]] = EntityEmpty

	act := ActionMove{
		DestSector: dest,
		Warp:       0.1,
	}

	events, err := act.Execute(game)
	if err != nil {
		t.Fatalf("unexpected error executing ActionMove: %v", err)
	}

	foundGameOver := false
	for _, ev := range events {
		if goEv, ok := ev.(EventGameOver); ok {
			foundGameOver = true
			if goEv.Reason != GameOverWon {
				t.Errorf("expected GameOverWon reason, got %v", goEv.Reason)
			}
		}
	}
	if !foundGameOver {
		t.Errorf("expected EventGameOver in events after scenario victory")
	}
	if !game.GameWon {
		t.Errorf("expected game.GameWon to be true")
	}
}
