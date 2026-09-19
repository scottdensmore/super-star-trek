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
