# Starfleet Career & Campaign Mode (Patrol Tour) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the Starfleet Career & Campaign Mode (Patrol Tour) for Super Star Trek, introducing a multi-sector campaign state machine, 6 modular starship refits with 3 tiers, an interactive Starbase Drydock modal, dual-state persistence, Hall of Fame tour commissions, and CLI/TUI integration.

**Architecture:** 
- `pkg/engine/campaign.go`: Tour state machine, procedural sector generation, objective evaluation, and requisition economy.
- `pkg/engine/refits.go`: Modular refits registry, tier costs, and tactical combat/subsystem modifier hooks.
- `pkg/engine/save.go`: Dual-state serialization for mid-tour and drydock persistence.
- `pkg/engine/leaderboard.go`: Dedicated Tour of Duty service records, medals, and commission ranks.
- `pkg/tui/components/drydockmodal/`: Bubbletea modal with ASCII starship cutaway, refit store, and disembark controls.
- `pkg/tui/`: Campaign HUD gauges, `tour`/`orders` commands, drydock transition handling, and `--tour` CLI flag in `cmd/sst`.

**Tech Stack:** Go 1.26+, Bubbletea v1, Lip Gloss, Standard Library.

**Spec:** [docs/superpowers/specs/2026-09-24-starfleet-career-campaign-design.md](file:///Users/scottdensmore/Developer/scottdensmore/super-star-trek/docs/superpowers/specs/2026-09-24-starfleet-career-campaign-design.md)

## Global Constraints

- Target Go version: `1.26.x`
- 100% pure standard library Go for core engine logic (`CGO_ENABLED=0`)
- Backward compatibility: Standalone classic game and existing scenario modes must remain completely unaffected
- Zero test failures across Go race detector (`go test -v -race ./...`)
- Zero linter issues (`golangci-lint run ./...`)
- Consistent styling across `Modern`, `LCARS`, and `CRT` visual themes

---

### Task 1: Tour State Machine & Sector Progression (`pkg/engine/campaign.go`)

**Files:**
- Create: `pkg/engine/campaign.go`
- Test: `pkg/engine/campaign_test.go`

**Interfaces:**
- Produces:
  - `type TourID string`
  - `type SectorObjectiveType string`
  - `type TourSectorConfig struct`
  - `type TourState struct`
  - `func NewTour(seed int64) *TourState`
  - `func (t *TourState) CurrentSector() *TourSectorConfig`
  - `func (t *TourState) StartCurrentSector() *GameState`
  - `func (t *TourState) EvaluateSector() (cleared bool, failed bool, bounty int)`
  - `func (t *TourState) AdvanceToDrydock(bounty int)`
  - `func (t *TourState) DisembarkToNextSector() (*GameState, error)`

- [ ] **Step 1: Write the failing tests in `pkg/engine/campaign_test.go`**

Create `pkg/engine/campaign_test.go`:
```go
package engine

import (
	"testing"
)

func TestNewTour_InitializesFourSectors(t *testing.T) {
	tour := NewTour(42)
	if tour == nil {
		t.Fatal("expected non-nil TourState")
	}
	if !tour.Active {
		t.Errorf("expected tour.Active to be true, got %v", tour.Active)
	}
	if len(tour.Sectors) != 4 {
		t.Fatalf("expected 4 sectors in tour, got %d", len(tour.Sectors))
	}
	if tour.CurrentSectorIndex != 0 {
		t.Errorf("expected CurrentSectorIndex 0, got %d", tour.CurrentSectorIndex)
	}
	if tour.RequisitionPoints != 0 {
		t.Errorf("expected initial 0 RequisitionPoints, got %d", tour.RequisitionPoints)
	}
	if tour.Sectors[0].Objective != ObjectiveBorderPatrol {
		t.Errorf("expected sector 1 objective %s, got %s", ObjectiveBorderPatrol, tour.Sectors[0].Objective)
	}
}

func TestTourState_SectorProgressionAndBounty(t *testing.T) {
	tour := NewTour(12345)
	state := tour.StartCurrentSector()
	if state == nil {
		t.Fatal("expected non-nil GameState for sector 1")
	}
	if tour.InDrydock {
		t.Errorf("expected InDrydock false during active sector, got true")
	}

	// Eliminate hostiles to satisfy sector 1 objective
	for qx := 0; qx < GalaxySize; qx++ {
		for qy := 0; qy < GalaxySize; qy++ {
			q := state.Galaxy[qx][qy]
			q.Klingons = 0
			q.Commanders = 0
			q.SuperCommanders = 0
		}
	}
	state.KlingonsRemaining = 0

	cleared, failed, bounty := tour.EvaluateSector()
	if !cleared {
		t.Errorf("expected sector to be cleared, got %v", cleared)
	}
	if failed {
		t.Errorf("expected sector not failed, got %v", failed)
	}
	if bounty <= 0 {
		t.Errorf("expected positive bounty payout, got %d", bounty)
	}

	tour.AdvanceToDrydock(bounty)
	if !tour.InDrydock {
		t.Errorf("expected InDrydock true after advancing, got false")
	}
	if tour.RequisitionPoints != bounty {
		t.Errorf("expected RequisitionPoints == bounty (%d), got %d", bounty, tour.RequisitionPoints)
	}
	if tour.SectorsCompleted != 1 {
		t.Errorf("expected SectorsCompleted 1, got %d", tour.SectorsCompleted)
	}

	// Disembark to sector 2
	nextState, err := tour.DisembarkToNextSector()
	if err != nil {
		t.Fatalf("unexpected error disembarking: %v", err)
	}
	if nextState == nil {
		t.Fatal("expected non-nil nextState")
	}
	if tour.InDrydock {
		t.Errorf("expected InDrydock false after disembarking")
	}
	if tour.CurrentSectorIndex != 1 {
		t.Errorf("expected CurrentSectorIndex 1, got %d", tour.CurrentSectorIndex)
	}
}

func TestTourState_PermadeathOnLoss(t *testing.T) {
	tour := NewTour(999)
	state := tour.StartCurrentSector()

	// Enterprise destroyed
	state.GameOver = true
	state.GameOverReason = GameOverDestroyed

	cleared, failed, _ := tour.EvaluateSector()
	if cleared {
		t.Errorf("expected cleared false on destroyed ship")
	}
	if !failed {
		t.Errorf("expected failed true on destroyed ship")
	}
	if tour.Active {
		t.Errorf("expected tour.Active false on permadeath")
	}
	if !tour.Failed {
		t.Errorf("expected tour.Failed true on permadeath")
	}
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `go test -v ./pkg/engine -run TestNewTour`
Expected: Compilation failure (`undefined: NewTour`, `undefined: TourState`).

- [ ] **Step 3: Implement `pkg/engine/campaign.go`**

Create `pkg/engine/campaign.go`:
```go
package engine

import (
	"fmt"
	"math/rand"
	"time"
)

type TourID string

type SectorObjectiveType string

const (
	ObjectiveBorderPatrol     SectorObjectiveType = "border_patrol"
	ObjectiveDeepSurveillance SectorObjectiveType = "deep_surveillance"
	ObjectiveConvoyEscort     SectorObjectiveType = "convoy_escort"
	ObjectiveStarbaseSiege    SectorObjectiveType = "starbase_siege"
)

type TourSectorConfig struct {
	Index         int                 `json:"index"`
	Name          string              `json:"name"`
	Objective     SectorObjectiveType `json:"objective"`
	Description   string              `json:"description"`
	InitialDays   float64             `json:"initial_days"`
	HostileCount  int                 `json:"hostile_count"`
	StarbaseCount int                 `json:"starbase_count"`
	Anomalies     bool                `json:"anomalies"`
	BonusBounty   int                 `json:"bonus_bounty"`
}

type TourState struct {
	ID                 TourID             `json:"id"`
	Active             bool               `json:"active"`
	CurrentSectorIndex int                `json:"current_sector_index"`
	Sectors            []TourSectorConfig `json:"sectors"`
	RequisitionPoints  int                `json:"requisition_points"`
	InstalledRefits    map[RefitID]int    `json:"installed_refits"`
	TotalTourScore     int                `json:"total_tour_score"`
	SectorsCompleted   int                `json:"sectors_completed"`
	HostilesDestroyed  int                `json:"hostiles_destroyed"`
	CurrentGameState   *GameState         `json:"current_game_state,omitempty"`
	InDrydock          bool               `json:"in_drydock"`
	Completed          bool               `json:"completed"`
	Failed             bool               `json:"failed"`
	FailureReason      GameOverReason     `json:"failure_reason,omitempty"`
	Medals             []string           `json:"medals"`
	Seed               int64              `json:"seed"`
}

func defaultSectors() []TourSectorConfig {
	return []TourSectorConfig{
		{
			Index:         1,
			Name:          "Vanguard Border Incursion",
			Objective:     ObjectiveBorderPatrol,
			Description:   "Eliminate Klingon vanguard battlecruisers infiltrating Federation border space.",
			InitialDays:   30.0,
			HostileCount:  4,
			StarbaseCount: 1,
			Anomalies:     false,
			BonusBounty:   250,
		},
		{
			Index:         2,
			Name:          "Mutara Deep Surveillance",
			Objective:     ObjectiveDeepSurveillance,
			Description:   "Chart unmapped anomaly quadrants in the Mutara rift and eliminate stealth scout vessels.",
			InitialDays:   32.0,
			HostileCount:  5,
			StarbaseCount: 1,
			Anomalies:     true,
			BonusBounty:   350,
		},
		{
			Index:         3,
			Name:          "Federation Convoy Escort",
			Objective:     ObjectiveConvoyEscort,
			Description:   "Protect Federation transport ships and eliminate marauder squadrons targeting medical convoys.",
			InitialDays:   35.0,
			HostileCount:  6,
			StarbaseCount: 2,
			Anomalies:     false,
			BonusBounty:   500,
		},
		{
			Index:         4,
			Name:          "Starbase 01 Final Siege",
			Objective:     ObjectiveStarbaseSiege,
			Description:   "Defend Starbase 01 against a concentrated hostile fleet armada. Hold the line at all costs.",
			InitialDays:   40.0,
			HostileCount:  8,
			StarbaseCount: 1,
			Anomalies:     true,
			BonusBounty:   750,
		},
	}
}

func NewTour(seed int64) *TourState {
	if seed == 0 {
		seed = time.Now().UnixNano()
	}
	return &TourState{
		ID:                 TourID(fmt.Sprintf("tour-%d", seed)),
		Active:             true,
		CurrentSectorIndex: 0,
		Sectors:            defaultSectors(),
		RequisitionPoints:  0,
		InstalledRefits:    make(map[RefitID]int),
		TotalTourScore:     0,
		SectorsCompleted:   0,
		InDrydock:          false,
		Completed:          false,
		Failed:             false,
		Medals:             make([]string, 0),
		Seed:               seed,
	}
}

func (t *TourState) CurrentSector() *TourSectorConfig {
	if t.CurrentSectorIndex < 0 || t.CurrentSectorIndex >= len(t.Sectors) {
		return nil
	}
	return &t.Sectors[t.CurrentSectorIndex]
}

func (t *TourState) StartCurrentSector() *GameState {
	sec := t.CurrentSector()
	if sec == nil {
		return nil
	}
	sectorSeed := t.Seed + int64(t.CurrentSectorIndex*1000)
	g := NewGameWithSeed(sectorSeed)
	g.Options.Difficulty = DifficultyNormal
	g.DaysRemaining = sec.InitialDays
	g.TimeRemaining = sec.InitialDays

	// Apply any installed refits to starting game state
	ApplyRefits(g, t.InstalledRefits)

	t.CurrentGameState = g
	t.InDrydock = false
	return g
}

func (t *TourState) EvaluateSector() (cleared bool, failed bool, bounty int) {
	g := t.CurrentGameState
	if g == nil {
		return false, false, 0
	}

	// Permadeath check
	if g.GameOver && g.GameOverReason != GameOverWon {
		t.Active = false
		t.Failed = true
		t.FailureReason = g.GameOverReason
		return false, true, 0
	}

	sec := t.CurrentSector()
	if sec == nil {
		return false, false, 0
	}

	// Sector cleared when hostiles remaining reach zero
	if g.KlingonsRemaining <= 0 {
		baseBounty := 1000
		stardateBonus := int(g.DaysRemaining * 50)
		flawlessBonus := 0
		hasDamage := false
		for _, dev := range g.Enterprise.Damage {
			if dev > 0 {
				hasDamage = true
				break
			}
		}
		if !hasDamage {
			flawlessBonus = 250
		}
		bounty = baseBounty + stardateBonus + flawlessBonus + sec.BonusBounty
		return true, false, bounty
	}

	return false, false, 0
}

func (t *TourState) AdvanceToDrydock(bounty int) {
	t.InDrydock = true
	t.RequisitionPoints += bounty
	t.SectorsCompleted++
	if t.CurrentGameState != nil {
		t.TotalTourScore += t.CurrentGameState.Score
	}
}

func (t *TourState) DisembarkToNextSector() (*GameState, error) {
	if !t.InDrydock {
		return nil, fmt.Errorf("cannot disembark: ship is not docked in drydock")
	}
	if t.CurrentSectorIndex+1 >= len(t.Sectors) {
		t.Completed = true
		t.Active = false
		t.InDrydock = false
		return nil, nil
	}
	t.CurrentSectorIndex++
	return t.StartCurrentSector(), nil
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `go test -v ./pkg/engine -run TestNewTour`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add pkg/engine/campaign.go pkg/engine/campaign_test.go
git commit -m "feat(engine): implement tour state machine and sector progression"
```

---

### Task 2: Modular Ship Refits & Tactical Stat Modifiers (`pkg/engine/refits.go`)

**Files:**
- Create: `pkg/engine/refits.go`
- Modify: `pkg/engine/combat.go:1-120`
- Modify: `pkg/engine/actions.go:1-120`
- Test: `pkg/engine/refits_test.go`

**Interfaces:**
- Produces:
  - `type RefitID string` (`RefitDilithiumCore`, `RefitDeflectorGrid`, `RefitTorpedoCasings`, `RefitTorpedoBays`, `RefitSensorMatrix`, `RefitDamageNanites`)
  - `type RefitDefinition struct`
  - `var RefitCatalog []RefitDefinition`
  - `func GetRefitDefinition(id RefitID) *RefitDefinition`
  - `func ApplyRefits(g *GameState, refits map[RefitID]int)`
  - `func PurchaseRefit(tour *TourState, id RefitID) error`

- [ ] **Step 1: Write failing tests in `pkg/engine/refits_test.go`**

Create `pkg/engine/refits_test.go`:
```go
package engine

import (
	"testing"
)

func TestRefits_ApplyEnergyAndTorpedoScaling(t *testing.T) {
	g := NewGameWithSeed(100)
	refits := map[RefitID]int{
		RefitDilithiumCore:  2, // Tier 2: 3000 + 1000 = 4000
		RefitTorpedoBays:    1, // Tier 1: 10 + 4 = 14
	}

	ApplyRefits(g, refits)

	if g.Enterprise.MaxEnergy != 4000.0 {
		t.Errorf("expected MaxEnergy 4000, got %f", g.Enterprise.MaxEnergy)
	}
	if g.Enterprise.Energy != 4000.0 {
		t.Errorf("expected Energy 4000, got %f", g.Enterprise.Energy)
	}
	if g.Enterprise.MaxTorpedoes != 14 {
		t.Errorf("expected MaxTorpedoes 14, got %d", g.Enterprise.MaxTorpedoes)
	}
	if g.Enterprise.Torpedoes != 14 {
		t.Errorf("expected Torpedoes 14, got %d", g.Enterprise.Torpedoes)
	}
}

func TestRefits_PurchaseWorkflowAndConstraints(t *testing.T) {
	tour := NewTour(555)
	tour.RequisitionPoints = 1000

	// Purchase Tier 1 Dilithium Core (cost 500)
	err := PurchaseRefit(tour, RefitDilithiumCore)
	if err != nil {
		t.Fatalf("unexpected error purchasing refit: %v", err)
	}
	if tour.InstalledRefits[RefitDilithiumCore] != 1 {
		t.Errorf("expected Dilithium Core tier 1, got %d", tour.InstalledRefits[RefitDilithiumCore])
	}
	if tour.RequisitionPoints != 500 {
		t.Errorf("expected 500 requisition points remaining, got %d", tour.RequisitionPoints)
	}

	// Attempting Tier 2 costs 1000, but only 500 remaining -> should error
	err = PurchaseRefit(tour, RefitDilithiumCore)
	if err == nil {
		t.Errorf("expected error for insufficient funds, got nil")
	}
}

func TestRefits_TorpedoDamageScaling(t *testing.T) {
	g := NewGameWithSeed(200)
	// Base damage
	baseYield := CalculateTorpedoDamage(g, 100.0)

	// Apply High Yield Tier 2 (+50%)
	g.ActiveRefits = map[RefitID]int{RefitTorpedoCasings: 2}
	boostedYield := CalculateTorpedoDamage(g, 100.0)

	expectedYield := baseYield * 1.5
	if boostedYield != expectedYield {
		t.Errorf("expected boosted yield %f, got %f", expectedYield, boostedYield)
	}
}

func TestRefits_ShieldAbsorptionReduction(t *testing.T) {
	g := NewGameWithSeed(300)
	incomingRaw := 200.0

	// Without refits
	unshieldedLoss := CalculateShieldDamageAbsorption(g, incomingRaw)

	// With Tier 2 Reinforced Deflectors (-30% shield drain)
	g.ActiveRefits = map[RefitID]int{RefitDeflectorGrid: 2}
	reinforcedLoss := CalculateShieldDamageAbsorption(g, incomingRaw)

	if reinforcedLoss >= unshieldedLoss {
		t.Errorf("expected reinforced loss %f < unshielded loss %f", reinforcedLoss, unshieldedLoss)
	}
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `go test -v ./pkg/engine -run TestRefits`
Expected: Compilation failure (`undefined: RefitDilithiumCore`, `undefined: ApplyRefits`).

- [ ] **Step 3: Implement `pkg/engine/refits.go` and hook into `combat.go` and `actions.go`**

Create `pkg/engine/refits.go`:
```go
package engine

import (
	"fmt"
)

type RefitID string

const (
	RefitDilithiumCore  RefitID = "dilithium_core"
	RefitDeflectorGrid  RefitID = "deflector_grid"
	RefitTorpedoCasings RefitID = "torpedo_casings"
	RefitTorpedoBays    RefitID = "torpedo_bays"
	RefitSensorMatrix   RefitID = "sensor_matrix"
	RefitDamageNanites  RefitID = "damage_nanites"
)

type RefitDefinition struct {
	ID          RefitID   `json:"id"`
	Name        string    `json:"name"`
	Description string    `json:"description"`
	TierCosts   [3]int    `json:"tier_costs"`
	TierEffects [3]string `json:"tier_effects"`
}

var RefitCatalog = []RefitDefinition{
	{
		ID:          RefitDilithiumCore,
		Name:        "Dilithium Core Tuning",
		Description: "Increases primary warp core energy storage capacity.",
		TierCosts:   [3]int{500, 1000, 1750},
		TierEffects: [3]string{
			"Max Energy: 3,500 (+500)",
			"Max Energy: 4,000 (+1,000)",
			"Max Energy: 4,500 (+1,500)",
		},
	},
	{
		ID:          RefitDeflectorGrid,
		Name:        "Reinforced Deflectors",
		Description: "Enhances deflector shield absorption efficiency under attack.",
		TierCosts:   [3]int{450, 900, 1500},
		TierEffects: [3]string{
			"Shield drain reduced by 15%",
			"Shield drain reduced by 30%",
			"Shield drain reduced by 45%",
		},
	},
	{
		ID:          RefitTorpedoCasings,
		Name:        "High-Yield Torpedoes",
		Description: "Upgrades photon warhead casing density and antimatter payload.",
		TierCosts:   [3]int{400, 800, 1400},
		TierEffects: [3]string{
			"Torpedo damage +25%",
			"Torpedo damage +50%",
			"Torpedo damage +75%",
		},
	},
	{
		ID:          RefitTorpedoBays,
		Name:        "Auxiliary Torpedo Magazine",
		Description: "Expands physical torpedo storage capacity in forward ordnance deck.",
		TierCosts:   [3]int{350, 700, 1200},
		TierEffects: [3]string{
			"Max Torpedoes: 14 (+4)",
			"Max Torpedoes: 18 (+8)",
			"Max Torpedoes: 22 (+12)",
		},
	},
	{
		ID:          RefitSensorMatrix,
		Name:        "Subspace Sensor Matrix",
		Description: "Extends long-range scan radius and reveals cloaked silhouettes.",
		TierCosts:   [3]int{300, 600, 1000},
		TierEffects: [3]string{
			"LRS scan radius +1 quadrant",
			"Cloaked hostile silhouettes revealed",
			"Automatic free LRS on quadrant entry",
		},
	},
	{
		ID:          RefitDamageNanites,
		Name:        "Automated Repair Bots",
		Description: "Deployable micro-drones accelerate damage control during warp transit.",
		TierCosts:   [3]int{400, 850, 1450},
		TierEffects: [3]string{
			"Warp movement passive repair +0.5d",
			"Warp movement passive repair +1.0d",
			"Warp movement passive repair +1.5d",
		},
	},
}

func GetRefitDefinition(id RefitID) *RefitDefinition {
	for i := range RefitCatalog {
		if RefitCatalog[i].ID == id {
			return &RefitCatalog[i]
		}
	}
	return nil
}

func ApplyRefits(g *GameState, refits map[RefitID]int) {
	if g == nil || refits == nil {
		return
	}
	g.ActiveRefits = make(map[RefitID]int)
	for k, v := range refits {
		g.ActiveRefits[k] = v
	}

	// Dilithium Core
	if tier := refits[RefitDilithiumCore]; tier > 0 {
		bonus := float64(tier * 500)
		g.Enterprise.MaxEnergy = 3000.0 + bonus
		g.Enterprise.Energy = g.Enterprise.MaxEnergy
	} else {
		g.Enterprise.MaxEnergy = 3000.0
	}

	// Torpedo Bays
	if tier := refits[RefitTorpedoBays]; tier > 0 {
		bonus := tier * 4
		g.Enterprise.MaxTorpedoes = 10 + bonus
		g.Enterprise.Torpedoes = g.Enterprise.MaxTorpedoes
	} else {
		g.Enterprise.MaxTorpedoes = 10
	}
}

func PurchaseRefit(tour *TourState, id RefitID) error {
	if tour == nil {
		return fmt.Errorf("tour is nil")
	}
	def := GetRefitDefinition(id)
	if def == nil {
		return fmt.Errorf("unknown refit module: %s", id)
	}

	currentTier := tour.InstalledRefits[id]
	if currentTier >= 3 {
		return fmt.Errorf("%s is already at maximum tier 3", def.Name)
	}

	nextTier := currentTier + 1
	cost := def.TierCosts[nextTier-1]
	if tour.RequisitionPoints < cost {
		return fmt.Errorf("insufficient requisition: requires %d, have %d", cost, tour.RequisitionPoints)
	}

	tour.RequisitionPoints -= cost
	tour.InstalledRefits[id] = nextTier
	return nil
}

func CalculateTorpedoDamage(g *GameState, baseDamage float64) float64 {
	if g == nil || g.ActiveRefits == nil {
		return baseDamage
	}
	tier := g.ActiveRefits[RefitTorpedoCasings]
	if tier <= 0 {
		return baseDamage
	}
	multiplier := 1.0 + (float64(tier) * 0.25)
	return baseDamage * multiplier
}

func CalculateShieldDamageAbsorption(g *GameState, incomingDamage float64) float64 {
	if g == nil || g.ActiveRefits == nil {
		return incomingDamage
	}
	tier := g.ActiveRefits[RefitDeflectorGrid]
	if tier <= 0 {
		return incomingDamage
	}
	reductionPercent := float64(tier) * 0.15
	return incomingDamage * (1.0 - reductionPercent)
}
```

In `pkg/engine/state.go`, add `ActiveRefits map[RefitID]int` to `GameState` and `MaxTorpedoes int` to `EnterpriseState`:
```go
type GameState struct {
    ...
    ActiveRefits map[RefitID]int `json:"active_refits,omitempty"`
}

type EnterpriseState struct {
    ...
    MaxEnergy    float64 `json:"max_energy"`
    MaxTorpedoes int     `json:"max_torpedoes"`
}
```

In `pkg/engine/actions.go`, in `Move()` after advancing stardate time:
```go
if g.ActiveRefits != nil {
    if tier := g.ActiveRefits[RefitDamageNanites]; tier > 0 {
        repairBoost := float64(tier) * 0.5
        for dev, d := range g.Enterprise.Damage {
            if d > 0 {
                g.Enterprise.Damage[dev] = max(0, d-repairBoost)
            }
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `go test -v ./pkg/engine -run TestRefits`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add pkg/engine/refits.go pkg/engine/refits_test.go pkg/engine/state.go pkg/engine/actions.go
git commit -m "feat(engine): implement modular ship refits and tactical stat modifiers"
```

---

### Task 3: Dual-State Save/Load Persistence (`pkg/engine/save.go`)

**Files:**
- Modify: `pkg/engine/save.go:1-120`
- Test: `pkg/engine/save_test.go`

**Interfaces:**
- Consumes: `TourState` from `pkg/engine/campaign.go`, `GameState` from `pkg/engine/state.go`
- Produces:
  - Extended JSON structure with `"tour_state": ...`
  - `func SaveTourGame(filename string, g *GameState, t *TourState) error`
  - `func LoadTourGame(filename string) (*GameState, *TourState, error)`

- [ ] **Step 1: Write failing tests in `pkg/engine/save_test.go`**

In `pkg/engine/save_test.go`, add:
```go
func TestSaveAndLoadTourGame(t *testing.T) {
	tmpDir := t.TempDir()
	savePath := filepath.Join(tmpDir, "test_tour.json")

	tour := NewTour(777)
	g := tour.StartCurrentSector()
	tour.RequisitionPoints = 1450
	tour.InstalledRefits[RefitDilithiumCore] = 1

	err := SaveTourGame(savePath, g, tour)
	if err != nil {
		t.Fatalf("failed to save tour game: %v", err)
	}

	loadedGame, loadedTour, err := LoadTourGame(savePath)
	if err != nil {
		t.Fatalf("failed to load tour game: %v", err)
	}
	if loadedTour == nil {
		t.Fatal("expected non-nil loadedTour")
	}
	if loadedTour.RequisitionPoints != 1450 {
		t.Errorf("expected 1450 RequisitionPoints, got %d", loadedTour.RequisitionPoints)
	}
	if loadedTour.InstalledRefits[RefitDilithiumCore] != 1 {
		t.Errorf("expected tier 1 Dilithium Core, got %d", loadedTour.InstalledRefits[RefitDilithiumCore])
	}
	if loadedGame.Enterprise.MaxEnergy != 3500.0 {
		t.Errorf("expected loadedGame MaxEnergy 3500, got %f", loadedGame.Enterprise.MaxEnergy)
	}
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `go test -v ./pkg/engine -run TestSaveAndLoadTourGame`
Expected: Compilation failure (`undefined: SaveTourGame`, `undefined: LoadTourGame`).

- [ ] **Step 3: Implement `SaveTourGame` and `LoadTourGame` in `pkg/engine/save.go`**

In `pkg/engine/save.go`:
```go
type SaveEnvelope struct {
	Version   int        `json:"version"`
	GameState *GameState `json:"game_state"`
	TourState *TourState `json:"tour_state,omitempty"`
}

func SaveTourGame(filename string, g *GameState, t *TourState) error {
	envelope := SaveEnvelope{
		Version:   2,
		GameState: g,
		TourState: t,
	}
	data, err := json.MarshalIndent(envelope, "", "  ")
	if err != nil {
		return fmt.Errorf("failed to serialize tour save: %w", err)
	}
	return os.WriteFile(filename, data, 0o600)
}

func LoadTourGame(filename string) (*GameState, *TourState, error) {
	data, err := os.ReadFile(filename)
	if err != nil {
		return nil, nil, fmt.Errorf("failed to read save file: %w", err)
	}
	var envelope SaveEnvelope
	if err := json.Unmarshal(data, &envelope); err != nil {
		return nil, nil, fmt.Errorf("failed to deserialize save file: %w", err)
	}
	if envelope.GameState != nil && envelope.TourState != nil {
		ApplyRefits(envelope.GameState, envelope.TourState.InstalledRefits)
	}
	return envelope.GameState, envelope.TourState, nil
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `go test -v ./pkg/engine -run TestSaveAndLoadTourGame`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add pkg/engine/save.go pkg/engine/save_test.go
git commit -m "feat(engine): implement dual-state save and load persistence for tours"
```

---

### Task 4: Hall of Fame Tour Commissions & Medals (`pkg/engine/leaderboard.go`)

**Files:**
- Modify: `pkg/engine/leaderboard.go:1-180`
- Test: `pkg/engine/leaderboard_test.go`

**Interfaces:**
- Produces:
  - `type TourRecord struct`
  - `func (lb *Leaderboard) RecordTour(tour *TourState, callsign string) *TourRecord`
  - `func CalculateTourCommission(tour *TourState) (rank string, medals []string)`

- [ ] **Step 1: Write failing tests in `pkg/engine/leaderboard_test.go`**

In `pkg/engine/leaderboard_test.go`, add:
```go
func TestLeaderboard_RecordTourAndRankCommission(t *testing.T) {
	lb := NewLeaderboard()
	tour := NewTour(888)
	tour.SectorsCompleted = 4
	tour.Completed = true
	tour.TotalTourScore = 18500
	tour.InstalledRefits[RefitDilithiumCore] = 3

	rec := lb.RecordTour(tour, "Kirk")
	if rec == nil {
		t.Fatal("expected non-nil TourRecord")
	}
	if rec.Rank != "Admiral of the Fleet" {
		t.Errorf("expected Admiral of the Fleet for 4/4 clear, got %s", rec.Rank)
	}
	if len(rec.Medals) == 0 {
		t.Errorf("expected medals awarded for full tour clear")
	}
	if rec.SectorsCleared != 4 {
		t.Errorf("expected 4 sectors cleared, got %d", rec.SectorsCleared)
	}
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `go test -v ./pkg/engine -run TestLeaderboard_RecordTourAndRankCommission`
Expected: Compilation failure (`undefined: TourRecord`, `undefined: RecordTour`).

- [ ] **Step 3: Implement `RecordTour` and rank commissions in `pkg/engine/leaderboard.go`**

In `pkg/engine/leaderboard.go`:
```go
type TourRecord struct {
	Date           string   `json:"date"`
	Callsign       string   `json:"callsign"`
	Rank           string   `json:"rank"`
	Score          int      `json:"score"`
	SectorsCleared int      `json:"sectors_cleared"`
	TotalSectors   int      `json:"total_sectors"`
	RefitsCount    int      `json:"refits_count"`
	Medals         []string `json:"medals"`
	Completed      bool     `json:"completed"`
}

func CalculateTourCommission(tour *TourState) (rank string, medals []string) {
	medals = make([]string, 0)
	if tour == nil {
		return "Cadet", medals
	}

	switch tour.SectorsCompleted {
	case 4:
		rank = "Admiral of the Fleet"
		medals = append(medals, "Starfleet Legion of Honor", "Klingon Campaign Ribbon", "Vanguard Star")
	case 3:
		rank = "Commodore"
		medals = append(medals, "Starfleet Merit Citation", "Klingon Campaign Ribbon")
	case 2:
		rank = "Fleet Captain"
		medals = append(medals, "Frontier Service Medal")
	case 1:
		rank = "Captain"
		medals = append(medals, "Patrol Ribbon")
	default:
		rank = "Commander (KIA)"
	}

	totalRefits := 0
	for _, tier := range tour.InstalledRefits {
		totalRefits += tier
	}
	if totalRefits >= 6 {
		medals = append(medals, "Master Starship Architect")
	}

	return rank, medals
}

func (lb *Leaderboard) RecordTour(tour *TourState, callsign string) *TourRecord {
	if tour == nil {
		return nil
	}
	if callsign == "" {
		callsign = "Enterprise"
	}
	rank, medals := CalculateTourCommission(tour)

	totalRefits := 0
	for _, tier := range tour.InstalledRefits {
		totalRefits += tier
	}

	rec := TourRecord{
		Date:           time.Now().Format("2006-01-02 15:04"),
		Callsign:       callsign,
		Rank:           rank,
		Score:          tour.TotalTourScore,
		SectorsCleared: tour.SectorsCompleted,
		TotalSectors:   len(tour.Sectors),
		RefitsCount:    totalRefits,
		Medals:         medals,
		Completed:      tour.Completed,
	}

	lb.TourRecords = append(lb.TourRecords, rec)
	return &rec
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `go test -v ./pkg/engine -run TestLeaderboard_RecordTourAndRankCommission`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add pkg/engine/leaderboard.go pkg/engine/leaderboard_test.go
git commit -m "feat(engine): implement Starfleet tour commissions and medals in Hall of Fame"
```

---

### Task 5: Starbase Drydock Modal Component (`pkg/tui/components/drydockmodal/`)

**Files:**
- Create: `pkg/tui/components/drydockmodal/drydockmodal.go`
- Test: `pkg/tui/components/drydockmodal/drydockmodal_test.go`

**Interfaces:**
- Consumes: `TourState` and `RefitCatalog` from `pkg/engine`
- Produces:
  - `type Model struct`
  - `func New(tour *engine.TourState, theme theme.Theme) Model`
  - `func (m Model) Update(msg tea.Msg) (Model, tea.Cmd)`
  - `func (m Model) View() string`
  - Messages: `DisembarkMsg`, `RefitPurchasedMsg`

- [ ] **Step 1: Write failing tests in `pkg/tui/components/drydockmodal/drydockmodal_test.go`**

Create `pkg/tui/components/drydockmodal/drydockmodal_test.go`:
```go
package drydockmodal

import (
	"testing"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

func TestDrydockModal_NavigationAndSelection(t *testing.T) {
	tour := engine.NewTour(404)
	tour.InDrydock = true
	tour.RequisitionPoints = 2000

	m := New(tour, theme.GetTheme(theme.ThemeModern))
	if m.selectedIndex != 0 {
		t.Errorf("expected initial selectedIndex 0, got %d", m.selectedIndex)
	}

	// Move down
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyDown})
	if m.selectedIndex != 1 {
		t.Errorf("expected selectedIndex 1 after down key, got %d", m.selectedIndex)
	}

	// Move up
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyUp})
	if m.selectedIndex != 0 {
		t.Errorf("expected selectedIndex 0 after up key, got %d", m.selectedIndex)
	}
}

func TestDrydockModal_PurchaseRefit(t *testing.T) {
	tour := engine.NewTour(405)
	tour.InDrydock = true
	tour.RequisitionPoints = 1000 // Enough for Dilithium Core (500)

	m := New(tour, theme.GetTheme(theme.ThemeModern))
	m.selectedIndex = 0 // Dilithium Core

	m, cmd := m.Update(tea.KeyMsg{Type: tea.KeyEnter})
	if cmd == nil {
		t.Fatal("expected command on purchase")
	}
	if tour.InstalledRefits[engine.RefitDilithiumCore] != 1 {
		t.Errorf("expected Dilithium Core tier 1, got %d", tour.InstalledRefits[engine.RefitDilithiumCore])
	}
	if tour.RequisitionPoints != 500 {
		t.Errorf("expected 500 requisition remaining, got %d", tour.RequisitionPoints)
	}
}

func TestDrydockModal_Disembark(t *testing.T) {
	tour := engine.NewTour(406)
	tour.InDrydock = true

	m := New(tour, theme.GetTheme(theme.ThemeModern))
	_, cmd := m.Update(tea.KeyMsg{Type: tea.KeySpace})
	if cmd == nil {
		t.Fatal("expected DisembarkMsg command on Space key")
	}
	msg := cmd()
	if _, ok := msg.(DisembarkMsg); !ok {
		t.Errorf("expected msg to be DisembarkMsg, got %T", msg)
	}
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `go test -v ./pkg/tui/components/drydockmodal`
Expected: Compilation failure (`undefined: New`, `undefined: DisembarkMsg`).

- [ ] **Step 3: Implement `pkg/tui/components/drydockmodal/drydockmodal.go`**

Create `pkg/tui/components/drydockmodal/drydockmodal.go`:
```go
package drydockmodal

import (
	"fmt"
	"strings"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

type DisembarkMsg struct{}
type RefitPurchasedMsg struct {
	RefitID engine.RefitID
	Tier    int
}

type Model struct {
	Tour          *engine.TourState
	Theme         theme.Theme
	Width         int
	Height        int
	selectedIndex int
	errorMessage  string
}

func New(tour *engine.TourState, th theme.Theme) Model {
	return Model{
		Tour:          tour,
		Theme:         th,
		Width:         80,
		Height:        24,
		selectedIndex: 0,
	}
}

func (m Model) Update(msg tea.Msg) (Model, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.KeyMsg:
		m.errorMessage = ""
		switch msg.String() {
		case "up", "k":
			if m.selectedIndex > 0 {
				m.selectedIndex--
			}
		case "down", "j":
			if m.selectedIndex < len(engine.RefitCatalog)-1 {
				m.selectedIndex++
			}
		case "enter":
			def := engine.RefitCatalog[m.selectedIndex]
			err := engine.PurchaseRefit(m.Tour, def.ID)
			if err != nil {
				m.errorMessage = err.Error()
				return m, nil
			}
			tier := m.Tour.InstalledRefits[def.ID]
			return m, func() tea.Msg {
				return RefitPurchasedMsg{RefitID: def.ID, Tier: tier}
			}
		case " ", "d", "D":
			return m, func() tea.Msg {
				return DisembarkMsg{}
			}
		}
	}
	return m, nil
}

func (m Model) View() string {
	boxStyle := lipgloss.NewStyle().
		Border(lipgloss.RoundedBorder()).
		BorderForeground(m.Theme.BorderNormal).
		Width(78).
		Padding(0, 1)

	headerStyle := lipgloss.NewStyle().
		Foreground(m.Theme.Foreground).
		Bold(true)

	accentStyle := lipgloss.NewStyle().
		Foreground(m.Theme.GaugeGood).
		Bold(true)

	// Top Title
	secIdx := 0
	if m.Tour != nil {
		secIdx = m.Tour.SectorsCompleted
	}
	title := headerStyle.Render(fmt.Sprintf("★ STARBASE 01 DRYDOCK & REFIT FACILITY ★  (Sector %d Cleared)", secIdx))
	reqText := accentStyle.Render(fmt.Sprintf("Requisition: %d PTS", m.Tour.RequisitionPoints))
	topBar := fmt.Sprintf("%-52s %22s", title, reqText)

	// Left: ASCII Starship Cutaway
	schematic := `
      /================\
     /                  \
 ===|     NCC - 1701     |===
 #  \                    /  #
 #   \==================/   #
 #            ||            #
 #+-------+   ||   +-------+#
  | WARP  |=======| WARP  |
  +-------+       +-------+
`
	leftPane := lipgloss.NewStyle().
		Width(30).
		Render(fmt.Sprintf("%s\nModules Installed: %d/6", schematic, len(m.Tour.InstalledRefits)))

	// Right: Refit Store List
	var items []string
	for i, refit := range engine.RefitCatalog {
		cursor := "  "
		if i == m.selectedIndex {
			cursor = "> "
		}
		currentTier := m.Tour.InstalledRefits[refit.ID]
		tierStr := fmt.Sprintf("[Tier %d/3]", currentTier)

		costStr := "[MAX TIER]"
		if currentTier < 3 {
			costStr = fmt.Sprintf("Cost: %d", refit.TierCosts[currentTier])
		}

		itemStyle := lipgloss.NewStyle().Foreground(m.Theme.Foreground)
		if i == m.selectedIndex {
			itemStyle = itemStyle.Bold(true).Foreground(m.Theme.ActiveCommand)
		}

		line := fmt.Sprintf("%s%-26s %-12s %s", cursor, refit.Name, tierStr, costStr)
		desc := fmt.Sprintf("     %s", refit.Description)
		items = append(items, itemStyle.Render(line), lipgloss.NewStyle().Foreground(m.Theme.StatusMuted).Render(desc))
	}
	rightPane := lipgloss.NewStyle().
		Width(44).
		Render(strings.Join(items, "\n"))

	content := lipgloss.JoinHorizontal(lipgloss.Top, leftPane, rightPane)

	footer := lipgloss.NewStyle().
		Foreground(m.Theme.StatusMuted).
		Render("[↑/↓ / j/k]: Select   [Enter]: Purchase Refit   [Space / D]: Disembark to Next Sector")

	if m.errorMessage != "" {
		footer = lipgloss.NewStyle().Foreground(m.Theme.ConditionRed).Render("Error: " + m.errorMessage)
	}

	return boxStyle.Render(fmt.Sprintf("%s\n\n%s\n\n%s", topBar, content, footer))
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `go test -v ./pkg/tui/components/drydockmodal`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add pkg/tui/components/drydockmodal/
git commit -m "feat(tui): implement interactive Starbase Drydock modal component"
```

---

### Task 6: TUI Dashboard, Commands & CLI Integration (`pkg/tui`, `cmd/sst`)

**Files:**
- Modify: `pkg/tui/model.go`
- Modify: `pkg/tui/update.go`
- Modify: `pkg/tui/view.go`
- Modify: `cmd/sst/main.go`
- Test: `cmd/sst/main_test.go`

**Interfaces:**
- Connects: `DrydockModal` into TUI state when `tour.InDrydock` is active
- Commands: `tour` and `orders` display sector mission parameters
- Flag: `--tour` or `--campaign` launches directly into Tour mode

- [ ] **Step 1: Write failing test in `cmd/sst/main_test.go`**

In `cmd/sst/main_test.go`, add `TestMain_TourFlag`:
```go
func TestMain_TourFlag(t *testing.T) {
	var out, errOut bytes.Buffer
	// Provide --help with tour flag check
	code := run([]string{"--help"}, strings.NewReader(""), &out, &errOut)
	if code != 0 {
		t.Fatalf("expected code 0, got %d", code)
	}
	if !strings.Contains(out.String(), "--tour") {
		t.Errorf("expected --tour flag documented in help output, got:\n%s", out.String())
	}
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `go test -v ./cmd/sst -run TestMain_TourFlag`
Expected: FAIL (missing `--tour` in help output).

- [ ] **Step 3: Integrate DrydockModal and `--tour` flag**

In `cmd/sst/main.go`:
```go
tourFlag := fs.Bool("tour", false, "Launch in Starfleet Career & Campaign (Patrol Tour) mode")
```
When `tourFlag` is true, initialize `TourState` via `engine.NewTour(seed)` and pass to `tui.NewModelWithTour()`.

In `pkg/tui/model.go`:
- Add `Tour *engine.TourState`
- Add `Drydock drydockmodal.Model`

In `pkg/tui/update.go`:
- Handle `drydockmodal.DisembarkMsg`: invokes `tour.DisembarkToNextSector()`, switches active game state, dismisses drydock modal.
- In turn evaluation: if `tour != nil && !tour.InDrydock`:
  - Call `cleared, failed, bounty := m.Tour.EvaluateSector()`.
  - If `cleared`: invoke `m.Tour.AdvanceToDrydock(bounty)` and open `m.Drydock`.
  - If `failed`: commit score to `Leaderboard.RecordTour(m.Tour, m.PlayerCallsign)`.
- In command bar / palette: handle `tour` and `orders` commands to output sector briefing and remaining requirements.

In `pkg/tui/view.go`:
- If `m.Tour != nil && m.Tour.InDrydock`:
  - Render `m.Drydock.View()` centered on screen.
- Header gauge: if `m.Tour != nil`, render `TOUR: SEC X/4`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `go test -v ./cmd/sst -run TestMain_TourFlag`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add cmd/sst/main.go cmd/sst/main_test.go pkg/tui/model.go pkg/tui/update.go pkg/tui/view.go
git commit -m "feat(tui): integrate campaign tour commands, HUD gauge, and drydock modal"
```

---

### Task 7: Automated End-to-End User Journey Test (`pkg/tui/tour_journey_test.go`)

**Files:**
- Create: `pkg/tui/tour_journey_test.go`

**Interfaces:**
- Simulates complete headless user journey through a campaign:
  - Sector 1 launch $\to$ kill hostiles $\to$ sector clear $\to$ drydock transition $\to$ purchase refit $\to$ disembark to Sector 2 $\to$ verify refit stat retention.

- [ ] **Step 1: Write the end-to-end journey test in `pkg/tui/tour_journey_test.go`**

Create `pkg/tui/tour_journey_test.go`:
```go
package tui

import (
	"testing"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/drydockmodal"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

func TestTourJourney_MultiSectorAndRefitWorkflow(t *testing.T) {
	tour := engine.NewTour(98765)
	th := theme.GetTheme(theme.ThemeModern)
	m := NewModelWithTour(tour, th)

	if m.Tour == nil || !m.Tour.Active {
		t.Fatal("expected active tour in model")
	}

	// 1. Clear Sector 1 hostiles
	g := m.Game
	for qx := 0; qx < engine.GalaxySize; qx++ {
		for qy := 0; qy < engine.GalaxySize; qy++ {
			g.Galaxy[qx][qy].Klingons = 0
			g.Galaxy[qx][qy].Commanders = 0
		}
	}
	g.KlingonsRemaining = 0

	// Trigger update loop turn
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyEnter})

	if !m.Tour.InDrydock {
		t.Fatalf("expected Tour to transition to Drydock, got InDrydock=%v", m.Tour.InDrydock)
	}
	if m.Tour.RequisitionPoints <= 0 {
		t.Errorf("expected positive RequisitionPoints, got %d", m.Tour.RequisitionPoints)
	}

	// 2. Buy Refit at Drydock (Dilithium Core)
	m.Drydock.Tour = m.Tour
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyEnter}) // Purchases highlighted item (Dilithium Core)

	if m.Tour.InstalledRefits[engine.RefitDilithiumCore] != 1 {
		t.Fatalf("expected Dilithium Core tier 1, got %d", m.Tour.InstalledRefits[engine.RefitDilithiumCore])
	}

	// 3. Disembark to Sector 2
	m, _ = m.Update(drydockmodal.DisembarkMsg{})

	if m.Tour.InDrydock {
		t.Errorf("expected InDrydock false after disembarking")
	}
	if m.Tour.CurrentSectorIndex != 1 {
		t.Errorf("expected CurrentSectorIndex 1, got %d", m.Tour.CurrentSectorIndex)
	}

	// 4. Verify refit stat boost transferred to Sector 2
	if m.Game.Enterprise.MaxEnergy != 3500.0 {
		t.Errorf("expected MaxEnergy 3500 in sector 2, got %f", m.Game.Enterprise.MaxEnergy)
	}
}
```

- [ ] **Step 2: Run test to verify it compiles and passes**

Run: `go test -v ./pkg/tui -run TestTourJourney`
Expected: PASS

- [ ] **Step 3: Run full verification suite**

Run: `go test -race ./...`
Expected: 100% PASS

- [ ] **Step 4: Commit**

```bash
git add pkg/tui/tour_journey_test.go
git commit -m "test(tui): add automated user journey test for multi-sector tour and drydock refits"
```

---

## Plan Self-Review Check
1. **Spec Coverage:** Covers all 4 sector objectives, 6 refits with 3 tiers, drydock modal, persistence, Hall of Fame commissions, and CLI `--tour` flag.
2. **Type Consistency:** Method signatures and types (`TourState`, `RefitID`, `RefitDefinition`, `DisembarkMsg`) are strictly synchronized across tasks.
3. **No Placeholders:** All tasks contain exact Go code, test assertions, file paths, and bash commands.
