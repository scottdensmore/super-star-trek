package engine

import (
	"strings"
	"sync"
)

// ScenarioID uniquely identifies a curated challenge scenario.
type ScenarioID string

const (
	ScenarioNone          ScenarioID = ""
	ScenarioKobayashiMaru ScenarioID = "kobayashi-maru"
	ScenarioMutaraNebula  ScenarioID = "mutara-nebula"
	ScenarioStarbaseSiege ScenarioID = "starbase-siege"
)

// Scenario encapsulates configuration, initial state factory, and custom evaluation logic
// for a tactical scenario or challenge mode.
type Scenario struct {
	ID           ScenarioID                                                     `json:"id"`
	Name         string                                                         `json:"name"`
	Subtitle     string                                                         `json:"subtitle"`
	Difficulty   string                                                         `json:"difficulty"`
	Description  string                                                         `json:"description"`
	Briefing     []string                                                       `json:"briefing"`
	Aliases      []string                                                       `json:"aliases,omitempty"`
	Build        func(seed int64) *GameState                                    `json:"-"`
	Evaluate     func(g *GameState) (done bool, won bool, reason GameOverReason) `json:"-"`
	ComputeScore func(g *GameState, won bool) ScoreBreakdown                    `json:"-"`
}

var (
	scenarioMu    sync.RWMutex
	scenarioMap   = make(map[ScenarioID]*Scenario)
	scenarioAlias = make(map[string]ScenarioID)
	scenarioOrder = []ScenarioID{}
)

// RegisterScenario registers or updates a scenario in the global registry.
// Aliases and ID are normalized to lowercase for case-insensitive lookup.
func RegisterScenario(s *Scenario) {
	if s == nil || s.ID == ScenarioNone {
		return
	}

	scenarioMu.Lock()
	defer scenarioMu.Unlock()

	normID := strings.ToLower(strings.TrimSpace(string(s.ID)))

	found := false
	for _, id := range scenarioOrder {
		if id == s.ID {
			found = true
			break
		}
	}
	if !found {
		scenarioOrder = append(scenarioOrder, s.ID)
	}

	scenarioMap[s.ID] = s
	scenarioAlias[normID] = s.ID

	for _, alias := range s.Aliases {
		normAlias := strings.ToLower(strings.TrimSpace(alias))
		if normAlias != "" {
			scenarioAlias[normAlias] = s.ID
		}
	}
}

// GetScenario looks up a scenario by ID or alias (case-insensitive).
func GetScenario(id ScenarioID) (*Scenario, bool) {
	scenarioMu.RLock()
	defer scenarioMu.RUnlock()

	norm := strings.ToLower(strings.TrimSpace(string(id)))
	if norm == "" {
		return nil, false
	}

	targetID, ok := scenarioAlias[norm]
	if !ok {
		return nil, false
	}

	s, ok := scenarioMap[targetID]
	return s, ok
}

// ListScenarios returns all registered scenarios in registration order.
func ListScenarios() []*Scenario {
	scenarioMu.RLock()
	defer scenarioMu.RUnlock()

	result := make([]*Scenario, 0, len(scenarioOrder))
	for _, id := range scenarioOrder {
		if s, ok := scenarioMap[id]; ok {
			result = append(result, s)
		}
	}
	return result
}

func init() {
	RegisterScenario(&Scenario{
		ID:          ScenarioKobayashiMaru,
		Name:        "Kobayashi Maru",
		Subtitle:    "The Unwinnable Simulation",
		Difficulty:  "Extreme",
		Description: "Starfleet Academy's infamous no-win tactical scenario in the Klingon Neutral Zone.",
		Briefing: []string{
			"STATION LOG: GAMMA HYDRA SECTOR 10 (KLINGON NEUTRAL ZONE)",
			"Third-class neutronic fuel carrier KOBAYASHI MARU has struck a gravitic mine.",
			"Hull breached, 81 crew aboard, 300 passengers. Power systems failing.",
			"You are entering the Neutral Zone in violation of the Organian Peace Treaty.",
		},
		Aliases:      []string{"kobayashi", "km"},
		Build:        BuildKobayashiMaru,
		Evaluate:     EvaluateKobayashiMaru,
		ComputeScore: ComputeScoreKobayashiMaru,
	})

	RegisterScenario(&Scenario{
		ID:          ScenarioMutaraNebula,
		Name:        "Mutara Nebula",
		Subtitle:    "Blind Tactical Duel",
		Difficulty:  "Hard",
		Description: "A tactical duel against a cloaked Klingon commander inside an ionized gas cloud with shields disabled.",
		Briefing: []string{
			"TACTICAL BRIEFING: MUTARA SECTOR (IONIZED GAS CLOUD)",
			"A rogue Klingon Super-Commander has lured the Enterprise into the Mutara Nebula.",
			"High electrostatic discharge has disabled defensive shields and blinded long-range sensors.",
			"Locate and destroy the enemy commander before your ship is compromised.",
		},
		Aliases:      []string{"mutara", "nebula"},
		Build:        BuildMutaraNebula,
		Evaluate:     EvaluateMutaraNebula,
		ComputeScore: ComputeScoreMutaraNebula,
	})

	RegisterScenario(&Scenario{
		ID:          ScenarioStarbaseSiege,
		Name:        "Starbase Under Siege",
		Subtitle:    "Defend Starbase 12",
		Difficulty:  "Challenge",
		Description: "Klingon strike fleet is assaulting Starbase 12. Intercept and eliminate the attackers before the base falls.",
		Briefing: []string{
			"RED ALERT: STARBASE 12 UNDER ATTACK",
			"A coordinated Klingon strike fleet is bombarding Starbase 12 in Sector [4, 4].",
			"The starbase shields are degrading under heavy orbital bombardment.",
			"Rush to their defense and eliminate all hostile warships before the base is destroyed.",
		},
		Aliases:      []string{"siege", "starbase"},
		Build:        BuildStarbaseSiege,
		Evaluate:     EvaluateStarbaseSiege,
		ComputeScore: ComputeScoreStarbaseSiege,
	})
}
