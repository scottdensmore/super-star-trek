# Spatial Anomalies & Environmental Hazards Engine Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the Spatial Anomalies & Environmental Hazards Engine in Super Star Trek, modeling macroscopic quadrant phenomena (Nebulae, Ion Storms) and tactical sector objects (Black Holes, Wormholes) with symmetrical physics across player and Klingon ships, fully integrated with TUI and WebAssembly frontends.

**Architecture:** Extend `pkg/engine/state.go` with `EnvironmentType` and `EntityWormhole`, configure via `GameRules.SpatialAnomalies` with 100% classic parity by default, intercept movement, combat, and sensor pipelines in `pkg/engine/actions.go` and `combat.go`, emit typed `engine.Event` instances, and render rich visual feedback in `pkg/tui` and `cmd/wasm`.

**Tech Stack:** Go 1.26+, Bubbletea, Lipgloss, xterm.js (WASM), standard library (`math`, `encoding/json`, `errors`).

**Spec:** `docs/superpowers/specs/2026-09-18-spatial-anomalies-design.md`

## Global Constraints

- Target branch: `scottdensmore/feat/spatial-anomalies`
- Target Go version: Go 1.26+ standard library
- WebAssembly compilation target: `GOOS=js GOARCH=wasm`
- Zero compiled binaries (e.g. `sst.wasm`, `sst`) committed to git
- Maintain 100% passing tests across Go (`go test -v -race ./...`) and C (`ctest --preset debug`, `tests/golden.sh`)
- 100% backward compatibility when `rules.SpatialAnomalies == false`

---

### Task 1: Core Anomaly Data Models, GameRules, & Procedural Generation

**Files:**
- Create: `pkg/engine/anomalies.go`
- Create: `pkg/engine/anomalies_test.go`
- Modify: `pkg/engine/state.go:1-25,110-160`
- Modify: `pkg/engine/rules.go:30-89`
- Modify: `pkg/engine/save.go:1-60`
- Modify: `pkg/engine/save_test.go:1-80`

**Interfaces:**
- Produces:
  ```go
  type EnvironmentType int
  const (
      EnvNormal EnvironmentType = iota
      EnvNebula
      EnvIonStorm
  )
  const EntityWormhole EntityType = 9
  ```
  In `GameRules`: `SpatialAnomalies bool json:"spatial_anomalies"`
  In `GameState`: `QuadrantEnv [9][9]EnvironmentType json:"quadrant_env"`
  Function `SeedAnomalies(g *GameState)` in `pkg/engine/anomalies.go`

- [ ] **Step 1: Write failing tests in `pkg/engine/anomalies_test.go`**

```go
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
```

- [ ] **Step 2: Run tests to verify failure**

Run: `go test -v ./pkg/engine -run TestAnomalies`
Expected: FAIL with `undefined: EnvNormal`, `undefined: EnvNebula`, `SpatialAnomalies undefined`

- [ ] **Step 3: Implement data models, rules, generation, and serialization**

In `pkg/engine/state.go`:
```go
// Add EntityWormhole to EntityType enum
const (
	EntityEmpty EntityType = iota
	EntityEnterprise
	EntityKlingon
	EntityCommander
	EntitySuperCommander
	EntityStarbase
	EntityStar
	EntityPlanet
	EntityBlackHole
	EntityWormhole
)

// EnvironmentType defines the ambient phenomenon within a galactic quadrant.
type EnvironmentType int

const (
	EnvNormal EnvironmentType = iota
	EnvNebula
	EnvIonStorm
)

// In GameState struct:
type GameState struct {
	RNG                *PRNG
	Rules              GameRules `json:"rules"`
	Skill              SkillLevel
	Length             GameLength
	Enterprise         Enterprise
	CurrentQuad        QuadrantState
	GalaxyChart        [9][9]int
	ChartDiscovered    [9][9]bool
	ChartKnownBases    [9][9]bool
	QuadrantEnv        [9][9]EnvironmentType `json:"quadrant_env"`
	RemainingKlingons  int
	RemainingStarbases int
	Stardate           float64
	InitialStardate    float64
	TimeRemaining      float64
	Metrics            GameMetrics `json:"metrics"`
	GameWon            bool        `json:"game_won"`
}
```

In `pkg/engine/rules.go`:
```go
type GameRules struct {
	Profile           DifficultyProfile `json:"profile"`
	Surveillance      SurveillanceMode  `json:"surveillance"`
	SensorDegradation bool              `json:"sensor_degradation"`
	RepairMultiplier  float64           `json:"repair_multiplier"`
	KlingonCloak      bool              `json:"klingon_cloak"`
	SpatialAnomalies  bool              `json:"spatial_anomalies"`
	TimeMargin        float64           `json:"time_margin"`
	AnimSpeed         int               `json:"anim_speed"`
}

// In DefaultRulesForProfile:
// ProfileCasual: SpatialAnomalies: false
// ProfileNormal: SpatialAnomalies: false
// ProfileHardcore: SpatialAnomalies: true
// ProfileNightmare: SpatialAnomalies: true
```

In `pkg/engine/anomalies.go`:
```go
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
```

Call `SeedAnomalies(g)` at the end of `NewGameWithOptions` in `pkg/engine/state.go`.

- [ ] **Step 4: Run tests to verify pass**

Run: `go test -v ./pkg/engine -run TestAnomalies`
Expected: PASS

- [ ] **Step 5: Verify Save/Load roundtrip with anomalies in `pkg/engine/save_test.go`**

Add `TestSaveLoadWithAnomalies` ensuring `QuadrantEnv` and `SpatialAnomalies` round-trip through JSON correctly.
Run: `go test -v ./pkg/engine -run "TestAnomalies|TestSave"`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add pkg/engine/anomalies.go pkg/engine/anomalies_test.go pkg/engine/state.go pkg/engine/rules.go pkg/engine/save.go pkg/engine/save_test.go
git commit -m "feat(engine): add anomaly data models, rules configuration, and procedural generation"
```

---

### Task 2: Environmental Movement & Navigation Physics (Ion Drift, Gravity Well, Wormhole Jump)

**Files:**
- Modify: `pkg/engine/events.go:1-60`
- Modify: `pkg/engine/actions.go:430-580`
- Modify: `pkg/engine/actions_test.go:400-550`

**Interfaces:**
- Consumes: `EnvironmentType`, `EntityBlackHole`, `EntityWormhole` from Task 1
- Produces:
  ```go
  type EventHazardTriggered struct {
      HazardType  string
      Description string
      EnergyDrain float64
  }
  type EventWormholeJump struct {
      FromQuad   Coord
      FromSector Coord
      ToQuad     Coord
      ToSector   Coord
      TimeDelta  float64
  }
  type EventSingularityAbsorption struct {
      Sector Coord
      Target EntityType
      Weapon string
  }
  ```
  `ActionMove` handling course drift in `EnvIonStorm`, energy multiplier adjacent to `EntityBlackHole`, lethal collision with `EntityBlackHole`, and jump on `EntityWormhole`.

- [ ] **Step 1: Write failing tests in `pkg/engine/actions_test.go`**

```go
func TestActionMove_BlackHoleFatalCollision(t *testing.T) {
	g := NewGame(10, SkillGood, LengthMedium)
	g.Enterprise.Sector = Coord{4, 4}
	g.CurrentQuad.Grid[4][4] = EntityEnterprise
	g.CurrentQuad.Grid[4][5] = EntityBlackHole

	// Move directly east into black hole
	act := ActionMove{Course: 1.0, Warp: 0.125}
	events, err := act.Execute(g)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	foundGameOver := false
	for _, e := range events {
		if goe, ok := e.(EventGameOver); ok {
			foundGameOver = true
			if goe.Reason != GameOverLost {
				t.Errorf("expected GameOverLost, got %v", goe.Reason)
			}
		}
	}
	if !foundGameOver {
		t.Errorf("expected EventGameOver when colliding with Black Hole")
	}
}

func TestActionMove_BlackHoleGravityWellEnergyCost(t *testing.T) {
	g := NewGame(10, SkillGood, LengthMedium)
	g.Enterprise.Sector = Coord{4, 4}
	g.CurrentQuad.Grid[4][4] = EntityEnterprise
	g.CurrentQuad.Grid[5][5] = EntityBlackHole // Adjacent diagonal gravity well

	initialEnergy := g.Enterprise.Energy
	// Move away to Coord{3, 4}
	act := ActionMove{DestSector: Coord{3, 4}, Warp: 0.1}
	_, err := act.Execute(g)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	energyUsed := initialEnergy - g.Enterprise.Energy
	// In gravity well, cost is doubled
	expectedCost := 2.0 * (0.1 * 1.0 * 1.0 * 1.0 * 1.0)
	if math.Abs(energyUsed-expectedCost) > 0.001 {
		t.Errorf("expected energy cost %f, got %f", expectedCost, energyUsed)
	}
}

func TestActionMove_WormholeJump(t *testing.T) {
	g := NewGame(10, SkillGood, LengthMedium)
	g.Enterprise.Quad = Coord{2, 2}
	g.Enterprise.Sector = Coord{4, 4}
	g.CurrentQuad.Grid[4][4] = EntityEnterprise
	g.CurrentQuad.Grid[4][5] = EntityWormhole

	act := ActionMove{DestSector: Coord{4, 5}, Warp: 0.1}
	events, err := act.Execute(g)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	foundJump := false
	for _, e := range events {
		if j, ok := e.(EventWormholeJump); ok {
			foundJump = true
			if j.FromQuad != (Coord{2, 2}) || j.FromSector != (Coord{4, 4}) {
				t.Errorf("unexpected jump origin: %+v", j)
			}
		}
	}
	if !foundJump {
		t.Errorf("expected EventWormholeJump when entering wormhole")
	}
	if g.Enterprise.Quad == (Coord{2, 2}) && g.Enterprise.Sector == (Coord{4, 5}) {
		t.Errorf("Enterprise should have teleported away from wormhole cell")
	}
}
```

- [ ] **Step 2: Run tests to verify failure**

Run: `go test -v ./pkg/engine -run "TestActionMove_BlackHole|TestActionMove_Wormhole"`
Expected: FAIL (missing event definitions and action handling)

- [ ] **Step 3: Implement movement anomaly physics and events**

In `pkg/engine/events.go`, add:
- `EventHazardTriggered`
- `EventWormholeJump`
- `EventSingularityAbsorption`

In `pkg/engine/actions.go` (`ActionMove.Execute`):
1. **Gravity Well Check**: Check if starting `fromSector` or current sector is within Chebyshev distance 1 of any `EntityBlackHole` in `g.CurrentQuad.Grid`. If yes, apply `2.0` multiplier to movement energy.
2. **Ion Storm Drift Check**: If `g.QuadrantEnv[g.Enterprise.Quad[0]][g.Enterprise.Quad[1]] == EnvIonStorm` and moving via vector steps:
   - On each step, if `g.RNG.Float64() < 0.25`, apply $\pm 1$ lateral deflection to `nextR` or `nextC`.
   - Emit `EventHazardTriggered{HazardType: "ion_storm_drift", Description: "Ion storm turbulence deflected course"}`.
3. **Collision / Cell Evaluation**:
   - If destination cell is `EntityBlackHole`: emit `EventSingularityAbsorption{Sector: nextSector, Target: EntityEnterprise, Weapon: "ship"}` and emit `EventGameOver{Reason: GameOverLost}`.
   - If destination cell is `EntityWormhole`:
     - Select random target quadrant `Coord{g.RNG.Intn(8)+1, g.RNG.Intn(8)+1}` and sector `Coord{g.RNG.Intn(8)+1, g.RNG.Intn(8)+1}`.
     - Advance stardate by `0.2`.
     - Emit `EventWormholeJump`.
     - Update Enterprise coordinates.

- [ ] **Step 4: Run tests to verify pass**

Run: `go test -v ./pkg/engine -run "TestActionMove_BlackHole|TestActionMove_Wormhole"`
Expected: PASS

- [ ] **Step 5: Run full engine test suite**

Run: `go test -v ./pkg/engine`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add pkg/engine/events.go pkg/engine/actions.go pkg/engine/actions_test.go
git commit -m "feat(engine): implement navigation physics for black holes, wormholes, and ion drift"
```

---

### Task 3: Combat, Sensor, & Shield Interference (Nebula Shields, LRS Masking, Singularity Ballistics, Klingon Symmetry)

**Files:**
- Modify: `pkg/engine/actions.go:13-43,147-240,597-630`
- Modify: `pkg/engine/combat.go:1-120`
- Modify: `pkg/engine/actions_test.go:1-100,551-650`
- Modify: `pkg/engine/combat_test.go:1-100`

**Interfaces:**
- Consumes: `EnvNebula`, `EntityBlackHole`, `EventAnomalyDiscovered`, `EventSingularityAbsorption`
- Produces:
  `ActionShields.Execute`: prevents shield raising in `EnvNebula`
  `ActionLRScan.Execute`: returns static masks for nebula quadrants
  `ActionFireTorpedo.Execute`: intercepts line-of-fire with `EntityBlackHole`
  `KlingonTurn`: applies zero shields in `EnvNebula` and fatal absorption if pushed into `EntityBlackHole`

- [ ] **Step 1: Write failing tests in `pkg/engine/actions_test.go` and `combat_test.go`**

```go
func TestActionShields_BlockedInNebula(t *testing.T) {
	g := NewGame(10, SkillGood, LengthMedium)
	g.QuadrantEnv[g.Enterprise.Quad[0]][g.Enterprise.Quad[1]] = EnvNebula

	act := ActionShields{Amount: 500}
	_, err := act.Execute(g)
	if err == nil {
		t.Fatalf("expected error when raising shields in a nebula, got nil")
	}
	if g.Enterprise.Shields != 0 {
		t.Errorf("shields should remain 0 in nebula, got %f", g.Enterprise.Shields)
	}
}

func TestActionFireTorpedo_AbsorbedByBlackHole(t *testing.T) {
	g := NewGame(10, SkillGood, LengthMedium)
	g.Enterprise.Sector = Coord{4, 1}
	g.CurrentQuad.Grid[4][1] = EntityEnterprise
	g.CurrentQuad.Grid[4][4] = EntityBlackHole
	// Place Klingon behind black hole
	klingon := &Klingon{ID: 1, Sector: Coord{4, 7}, Energy: 500}
	g.CurrentQuad.Klingons = []*Klingon{klingon}
	g.CurrentQuad.Grid[4][7] = EntityKlingon

	// Fire torpedo east along row 4 (direction 1.0)
	act := ActionFireTorpedo{Direction: 1.0}
	events, err := act.Execute(g)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	foundAbsorption := false
	for _, e := range events {
		if sa, ok := e.(EventSingularityAbsorption); ok {
			foundAbsorption = true
			if sa.Weapon != "torpedo" {
				t.Errorf("expected weapon 'torpedo', got %s", sa.Weapon)
			}
		}
	}
	if !foundAbsorption {
		t.Errorf("expected EventSingularityAbsorption when torpedo hits Black Hole")
	}
	// Klingon behind black hole must not be hit
	if klingon.Energy < 500 {
		t.Errorf("Klingon behind black hole took damage, energy is %f", klingon.Energy)
	}
}

func TestKlingonCombat_ZeroShieldsInNebula(t *testing.T) {
	g := NewGame(10, SkillGood, LengthMedium)
	g.QuadrantEnv[g.Enterprise.Quad[0]][g.Enterprise.Quad[1]] = EnvNebula
	klingon := &Klingon{ID: 1, Sector: Coord{4, 5}, Energy: 400}
	g.CurrentQuad.Klingons = []*Klingon{klingon}
	g.CurrentQuad.Grid[4][5] = EntityKlingon

	// Symmetrical damage: in nebula, 100 energy phaser hit does full direct damage with no shield mitigation
	act := ActionFirePhasers{Energy: 200}
	_, err := act.Execute(g)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if klingon.Energy >= 400 {
		t.Errorf("expected Klingon to take full damage in nebula, remaining: %f", klingon.Energy)
	}
}
```

- [ ] **Step 2: Run tests to verify failure**

Run: `go test -v ./pkg/engine -run "TestNebula|TestBlackHole|TestKlingonCombat_ZeroShields"`
Expected: FAIL

- [ ] **Step 3: Implement nebula shield and sensor blocking, black hole torpedo absorption, and Klingon symmetry**

1. In `pkg/engine/actions.go` (`ActionShields.Execute`):
   ```go
   if a.Amount > 0 && g.QuadrantEnv[g.Enterprise.Quad[0]][g.Enterprise.Quad[1]] == EnvNebula {
       return nil, errors.New("sensors indicate extreme particle ionization: shields cannot hold cohesive geometry in nebula")
   }
   ```
2. When moving into a new quadrant in `ActionMove`:
   If entering `EnvNebula`, collapse `Enterprise.Shields` to 0, return energy to `Enterprise.Energy` (clamped to 5000), and emit `EventAnomalyDiscovered{Quad: toQuad, Env: EnvNebula}`.
3. In `ActionFireTorpedo.Execute`:
   During raycast trajectory, if `g.CurrentQuad.Grid[curR][curC] == EntityBlackHole`:
   Emit `EventSingularityAbsorption{Sector: Coord{curR, curC}, Target: EntityBlackHole, Weapon: "torpedo"}` and break out of trajectory without hitting entities beyond.
4. In `ActionLRScan.Execute`:
   Add masked scan indicators for nebulae: when reading surrounding quadrants, if `g.QuadrantEnv[qr][qc] == EnvNebula`, return `-1` (representing `***`).

- [ ] **Step 4: Run tests to verify pass**

Run: `go test -v ./pkg/engine -run "TestNebula|TestBlackHole|TestKlingonCombat_ZeroShields"`
Expected: PASS

- [ ] **Step 5: Run full engine test suite with race detection**

Run: `go test -v -race ./pkg/engine/...`
Expected: PASS with 0 race warnings

- [ ] **Step 6: Commit**

```bash
git add pkg/engine/actions.go pkg/engine/combat.go pkg/engine/actions_test.go pkg/engine/combat_test.go
git commit -m "feat(engine): add combat and sensor anomaly rules with Klingon symmetry"
```

---

### Task 4: TUI Presentation, Sector Grid Symbols, & WebAssembly CRT Formatter

**Files:**
- Modify: `pkg/tui/components/sectorgrid/grid.go:155-185`
- Modify: `pkg/tui/components/sectorgrid/grid_test.go:50-100,240-270`
- Modify: `cmd/wasm/formatter.go:50-120,200-260`
- Modify: `cmd/wasm/formatter_test.go:50-150`
- Modify: `cmd/sst/main.go:30-80`

**Interfaces:**
- Consumes: `EntityWormhole`, `EventAnomalyDiscovered`, `EventHazardTriggered`, `EventWormholeJump`, `EventSingularityAbsorption`
- Produces:
  `sectorgrid.RenderCell`: `EntityWormhole` rendered as `>W<` (cyan)
  `cmd/wasm/formatter.FormatSRS`: `EntityWormhole` teletype symbol `>W<`
  `cmd/wasm/formatter.FormatCombatEvents`: teletype ANSI strings for all anomaly events
  `cmd/sst/main.go`: `--anomalies` / `--no-anomalies` CLI flag support

- [ ] **Step 1: Write failing tests in `sectorgrid_test.go` and `cmd/wasm/formatter_test.go`**

In `pkg/tui/components/sectorgrid/grid_test.go`:
```go
func TestGridRender_EntityWormhole(t *testing.T) {
	var quad engine.QuadrantState
	quad.Grid[3][3] = engine.EntityWormhole
	rendered := RenderCell(3, 3, &quad, false)
	if !strings.Contains(rendered, ">W<") {
		t.Errorf("expected wormhole symbol '>W<', got %q", rendered)
	}
}
```

In `cmd/wasm/formatter_test.go`:
```go
func TestFormatCombatEvents_AnomalyEvents(t *testing.T) {
	events := []engine.Event{
		engine.EventHazardTriggered{
			HazardType:  "ion_storm_drift",
			Description: "Ion storm turbulence deflected course",
		},
		engine.EventSingularityAbsorption{
			Sector: engine.Coord{4, 4},
			Weapon: "torpedo",
		},
		engine.EventWormholeJump{
			FromQuad: engine.Coord{1, 1},
			ToQuad:   engine.Coord{5, 6},
			ToSector: engine.Coord{3, 3},
		},
	}
	output := FormatCombatEvents(events)
	if !strings.Contains(output, "turbulence deflected course") {
		t.Errorf("expected ion storm message, got: %s", output)
	}
	if !strings.Contains(output, "absorbed into event horizon") {
		t.Errorf("expected singularity absorption message, got: %s", output)
	}
	if !strings.Contains(output, "Wormhole transit completed") {
		t.Errorf("expected wormhole transit message, got: %s", output)
	}
}
```

- [ ] **Step 2: Run tests to verify failure**

Run: `go test -v ./pkg/tui/components/sectorgrid -run TestGridRender_EntityWormhole`
Run: `go test -v ./cmd/wasm -run TestFormatCombatEvents_AnomalyEvents`
Expected: FAIL

- [ ] **Step 3: Implement TUI sector grid rendering, WASM formatting, and CLI flag**

In `pkg/tui/components/sectorgrid/grid.go`:
```go
case engine.EntityWormhole:
	return styles.Wormhole.Render(">W<") // Add Wormhole style with lipgloss cyan
```

In `cmd/wasm/formatter.go`:
- Handle `engine.EntityWormhole` in `FormatSRS`: return `">W<"`
- Handle `EventHazardTriggered`, `EventSingularityAbsorption`, `EventWormholeJump`, and `EventAnomalyDiscovered` in `FormatCombatEvents`:
  ```go
  case engine.EventHazardTriggered:
      sb.WriteString(fmt.Sprintf("\x1b[33m*** HAZARD: %s ***\x1b[0m\r\n", e.Description))
  case engine.EventSingularityAbsorption:
      sb.WriteString("\x1b[35m*** GRAVITATIONAL SINGULARITY: Photon torpedo absorbed into event horizon! ***\x1b[0m\r\n")
  case engine.EventWormholeJump:
      sb.WriteString(fmt.Sprintf("\x1b[36m*** SUBSPACE RIFT: Wormhole transit completed to Quadrant [%d, %d] Sector [%d, %d]! ***\x1b[0m\r\n", e.ToQuad[0], e.ToQuad[1], e.ToSector[0], e.ToSector[1]))
  ```

In `cmd/sst/main.go`:
Add `--anomalies` and `--no-anomalies` command-line flags modifying `rules.SpatialAnomalies`.

- [ ] **Step 4: Run tests to verify pass**

Run: `go test -v ./pkg/tui/components/sectorgrid -run TestGridRender_EntityWormhole`
Run: `go test -v ./cmd/wasm -run TestFormatCombatEvents_AnomalyEvents`
Expected: PASS

- [ ] **Step 5: Verify WASM compilation and Golden suite**

Run: `GOOS=js GOARCH=wasm go build -o /dev/null ./cmd/wasm`
Run: `tests/golden.sh`
Run: `go test -v -race ./...`
Expected: All suites PASS with 100% clean output

- [ ] **Step 6: Commit**

```bash
git add pkg/tui/components/sectorgrid/ cmd/wasm/ cmd/sst/main.go
git commit -m "feat(tui,wasm): add anomaly sector rendering, CRT teletype logs, and CLI flags"
```
