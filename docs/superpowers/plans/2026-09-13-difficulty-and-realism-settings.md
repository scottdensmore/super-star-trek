# Configurable Difficulty & Realism Settings Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement configurable game options and difficulty presets for starbase surveillance, two-tier sensor degradation, Klingon commander cloaking, repair multipliers, launch CLI flags, and an interactive in-game TUI options modal.

**Architecture:** A centralized `GameRules` struct in `pkg/engine` encapsulates exploration and realism parameters, initialized via preset profiles (`Casual`, `Normal`, `Hardcore`, `Nightmare`, `Custom`). `ActionDock`, sensor rendering, combat targeting, and repair cycles respect these rules. A new TUI overlay `pkg/tui/components/optionsmodal` enables live adjustments in game, and CLI flags configure initial rules on launch.

**Tech Stack:** Go 1.26+, Bubble Tea, Lip Gloss, Bubbles, C99 (for classic verification gates).

**Spec:** [`docs/superpowers/specs/2026-09-13-difficulty-and-realism-settings-design.md`](file:///home/scottdensmore/Developer/scottdensmore/super-star-trek/docs/superpowers/specs/2026-09-13-difficulty-and-realism-settings-design.md)

## Global Constraints

- Target Go version: Go 1.26+ standard library and Charmbracelet packages (`bubbletea`, `lipgloss`, `bubbles`).
- Zero compiled binaries (e.g., `sst`, `sst.exe`) or temporary files committed to git.
- Maintain 100% passing tests for both Go (`go test -v -race ./...`) and C (`ctest --preset debug`).
- Classic teletype mode (`--classic`) and tournament tests remain untouched.
- Dashboard layout must strictly maintain the 80x24 terminal dimension budget (exactly 24 lines).
- Work committed on feature branch `scottdensmore/feat/difficulty-and-realism-settings`.

---

### Task 1: `GameRules` Data Structures, Difficulty Presets, and Game Initialization (`pkg/engine`)

**Files:**
- Create: `pkg/engine/rules.go`
- Create: `pkg/engine/rules_test.go`
- Modify: `pkg/engine/state.go:90-174`
- Modify: `pkg/engine/save.go:90-117`

**Interfaces:**
- Consumes: `PRNG`, `SkillLevel`, `GameLength`, `Coord` from `pkg/engine`.
- Produces:
  - Types: `DifficultyProfile`, `SurveillanceMode`, `GameRules`.
  - Functions: `DefaultRulesForProfile(profile DifficultyProfile) GameRules`, `NewGameWithOptions(seed int64, skill SkillLevel, length GameLength, rules GameRules) *GameState`.
  - `GameState.Rules GameRules`.
  - `NewGame(seed, skill, length)` initializing `DefaultRulesForProfile(ProfileNormal)`.

- [ ] **Step 1: Write failing tests in `pkg/engine/rules_test.go`**

```go
package engine

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func TestDifficultyPresets(t *testing.T) {
	casual := DefaultRulesForProfile(ProfileCasual)
	if casual.Surveillance != SurveillanceFull || casual.SensorDegradation || casual.RepairMultiplier != 0.75 || casual.KlingonCloak || casual.TimeMargin != 1.25 {
		t.Errorf("unexpected Casual preset: %+v", casual)
	}

	normal := DefaultRulesForProfile(ProfileNormal)
	if normal.Surveillance != SurveillanceClassic || !normal.SensorDegradation || normal.RepairMultiplier != 1.00 || normal.KlingonCloak || normal.TimeMargin != 1.00 {
		t.Errorf("unexpected Normal preset: %+v", normal)
	}

	hardcore := DefaultRulesForProfile(ProfileHardcore)
	if hardcore.Surveillance != SurveillanceLocal || !hardcore.SensorDegradation || hardcore.RepairMultiplier != 1.50 || !hardcore.KlingonCloak || hardcore.TimeMargin != 0.80 {
		t.Errorf("unexpected Hardcore preset: %+v", hardcore)
	}

	nightmare := DefaultRulesForProfile(ProfileNightmare)
	if nightmare.Surveillance != SurveillanceBlackout || !nightmare.SensorDegradation || nightmare.RepairMultiplier != 2.00 || !nightmare.KlingonCloak || nightmare.TimeMargin != 0.60 {
		t.Errorf("unexpected Nightmare preset: %+v", nightmare)
	}
}

func TestNewGameWithOptions_BlackoutHidesStarbases(t *testing.T) {
	rules := DefaultRulesForProfile(ProfileNightmare)
	g := NewGameWithOptions(12345, SkillGood, LengthMedium, rules)

	if g.Rules.Surveillance != SurveillanceBlackout {
		t.Fatalf("expected Blackout surveillance, got %v", g.Rules.Surveillance)
	}
	if g.TimeRemaining != 30.0*0.60 {
		t.Errorf("expected TimeRemaining 18.0, got %f", g.TimeRemaining)
	}

	knownBasesCount := 0
	for r := 1; r <= 8; r++ {
		for c := 1; c <= 8; c++ {
			if g.ChartKnownBases[r][c] {
				knownBasesCount++
			}
		}
	}
	if knownBasesCount != 0 {
		t.Errorf("expected 0 known bases in blackout mode at start, found %d", knownBasesCount)
	}
}

func TestNewGameWithOptions_NormalChartsStarbases(t *testing.T) {
	rules := DefaultRulesForProfile(ProfileNormal)
	g := NewGameWithOptions(12345, SkillGood, LengthMedium, rules)

	knownBasesCount := 0
	for r := 1; r <= 8; r++ {
		for c := 1; c <= 8; c++ {
			if g.ChartKnownBases[r][c] {
				knownBasesCount++
			}
		}
	}
	if knownBasesCount != g.RemainingStarbases {
		t.Errorf("expected %d known bases in normal mode at start, found %d", g.RemainingStarbases, knownBasesCount)
	}
}

func TestSaveLoad_GameRulesPersistenceAndBackwardCompatibility(t *testing.T) {
	tempDir := t.TempDir()
	savePath := filepath.Join(tempDir, "TESTSAVE.TRK")

	rules := GameRules{
		Profile:           ProfileCustom,
		Surveillance:      SurveillanceLocal,
		SensorDegradation: true,
		RepairMultiplier:  1.75,
		KlingonCloak:      true,
		TimeMargin:        0.9,
	}
	g := NewGameWithOptions(999, SkillExpert, LengthLong, rules)
	if err := g.Save(savePath); err != nil {
		t.Fatalf("Save failed: %v", err)
	}

	loaded, err := LoadGame(savePath)
	if err != nil {
		t.Fatalf("Load failed: %v", err)
	}
	if loaded.Rules != rules {
		t.Errorf("Rules mismatch after load: got %+v, want %+v", loaded.Rules, rules)
	}

	// Test backward compatibility: old save with no "rules" key
	rawOld := map[string]interface{}{
		"Skill":              SkillGood,
		"Length":             LengthMedium,
		"RemainingKlingons":  15,
		"RemainingStarbases": 3,
	}
	data, _ := json.Marshal(rawOld)
	oldPath := filepath.Join(tempDir, "OLDSAVE.TRK")
	_ = os.WriteFile(oldPath, data, 0644)

	loadedOld, err := LoadGame(oldPath)
	if err != nil {
		t.Fatalf("Load old save failed: %v", err)
	}
	if loadedOld.Rules.Profile != ProfileNormal || loadedOld.Rules.Surveillance != SurveillanceClassic {
		t.Errorf("expected default Normal rules for legacy save, got %+v", loadedOld.Rules)
	}
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `go test -v ./pkg/engine -run "TestDifficultyPresets|TestNewGameWithOptions|TestSaveLoad_GameRules"`
Expected: FAIL due to undefined types/functions (`GameRules`, `DefaultRulesForProfile`, `NewGameWithOptions`).

- [ ] **Step 3: Write minimal implementation in `pkg/engine/rules.go`, `state.go`, and `save.go`**

Create `pkg/engine/rules.go`:
```go
package engine

// DifficultyProfile represents standard game difficulty presets.
type DifficultyProfile string

const (
	ProfileCasual    DifficultyProfile = "casual"
	ProfileNormal    DifficultyProfile = "normal"
	ProfileHardcore  DifficultyProfile = "hardcore"
	ProfileNightmare DifficultyProfile = "nightmare"
	ProfileCustom    DifficultyProfile = "custom"
)

// SurveillanceMode defines the extent of quadrant intelligence revealed upon starbase docking.
type SurveillanceMode string

const (
	SurveillanceFull     SurveillanceMode = "full"     // Reveals all 64 quadrants
	SurveillanceClassic  SurveillanceMode = "classic"  // Reveals 3x3 perimeters around all active starbases
	SurveillanceLocal    SurveillanceMode = "local"    // Reveals 3x3 perimeter around docked base only
	SurveillanceBlackout SurveillanceMode = "blackout" // Reveals no additional quadrants; starbases unmapped at start
)

// GameRules encapsulates difficulty and realism configuration parameters.
type GameRules struct {
	Profile           DifficultyProfile `json:"profile"`
	Surveillance      SurveillanceMode  `json:"surveillance"`
	SensorDegradation bool              `json:"sensor_degradation"`
	RepairMultiplier  float64           `json:"repair_multiplier"`
	KlingonCloak      bool              `json:"klingon_cloak"`
	TimeMargin        float64           `json:"time_margin"`
}

// DefaultRulesForProfile returns standard game rules for the specified difficulty profile.
func DefaultRulesForProfile(profile DifficultyProfile) GameRules {
	switch profile {
	case ProfileCasual:
		return GameRules{
			Profile:           ProfileCasual,
			Surveillance:      SurveillanceFull,
			SensorDegradation: false,
			RepairMultiplier:  0.75,
			KlingonCloak:      false,
			TimeMargin:        1.25,
		}
	case ProfileHardcore:
		return GameRules{
			Profile:           ProfileHardcore,
			Surveillance:      SurveillanceLocal,
			SensorDegradation: true,
			RepairMultiplier:  1.50,
			KlingonCloak:      true,
			TimeMargin:        0.80,
		}
	case ProfileNightmare:
		return GameRules{
			Profile:           ProfileNightmare,
			Surveillance:      SurveillanceBlackout,
			SensorDegradation: true,
			RepairMultiplier:  2.00,
			KlingonCloak:      true,
			TimeMargin:        0.60,
		}
	case ProfileNormal:
		fallthrough
	default:
		return GameRules{
			Profile:           ProfileNormal,
			Surveillance:      SurveillanceClassic,
			SensorDegradation: true,
			RepairMultiplier:  1.00,
			KlingonCloak:      false,
			TimeMargin:        1.00,
		}
	}
}
```

In `pkg/engine/state.go`, add `Rules GameRules `json:"rules"` to `GameState`.
Implement `NewGameWithOptions(seed int64, skill SkillLevel, length GameLength, rules GameRules) *GameState` and refactor `NewGame` to call `NewGameWithOptions(seed, skill, length, DefaultRulesForProfile(ProfileNormal))`.
In `NewGameWithOptions`, respect `rules.TimeMargin` when initializing `TimeRemaining = 30.0 * rules.TimeMargin` and only set `ChartKnownBases[r][c] = true` if `rules.Surveillance != SurveillanceBlackout`.

In `pkg/engine/save.go`, in `LoadGame`, after `json.Unmarshal`:
```go
if g.Rules.Profile == "" {
    g.Rules = DefaultRulesForProfile(ProfileNormal)
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `go test -v ./pkg/engine -run "TestDifficultyPresets|TestNewGameWithOptions|TestSaveLoad_GameRules"`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add pkg/engine/rules.go pkg/engine/rules_test.go pkg/engine/state.go pkg/engine/save.go
git commit -m "feat(engine): add GameRules, difficulty presets, and customizable game initialization"
```

---

### Task 2: Surveillance Modes in `ActionDock` & Two-Tier Sensor Degradation (`pkg/engine`, `pkg/tui/components/statuspanel`)

**Files:**
- Modify: `pkg/engine/actions.go:70-105`
- Modify: `pkg/engine/actions.go:470-506`
- Modify: `pkg/engine/events.go:120-140`
- Modify: `pkg/tui/components/statuspanel/status.go:90-130`
- Test: `pkg/engine/actions_test.go`
- Test: `pkg/tui/components/statuspanel/status_test.go`

**Interfaces:**
- Consumes: `g.Rules.Surveillance`, `g.Rules.SensorDegradation` from `GameState`.
- Produces:
  - `ActionDock.Execute` reveals quadrants strictly according to `g.Rules.Surveillance`.
  - `EventStarbaseSurveillance` includes `Mode SurveillanceMode`.
  - Status panel radar shows `RADAR (QUAD ±1) [LRS DEGRADED]` with noisy `?` markers when `0 < DeviceLRSensors < 2.0 && g.Rules.SensorDegradation`, and `[LRS OFFLINE]` with `???` when `>= 2.0`.
  - `ActionLRScan` appends degraded advisory event when `0 < DeviceLRSensors < 2.0`.

- [ ] **Step 1: Write failing tests in `pkg/engine/actions_test.go` & `status_test.go`**

In `pkg/engine/actions_test.go`:
```go
func TestActionDock_SurveillanceModes(t *testing.T) {
	// 1. Full surveillance reveals all 64
	gFull := NewGameWithOptions(100, SkillGood, LengthMedium, GameRules{Surveillance: SurveillanceFull})
	gFull.CurrentQuad.Starbase = &Coord{4, 4}
	gFull.Enterprise.Sector = Coord{4, 5}
	_, err := ActionDock{}.Execute(gFull)
	if err != nil {
		t.Fatalf("ActionDock failed: %v", err)
	}
	discoveredCount := 0
	for r := 1; r <= 8; r++ {
		for c := 1; c <= 8; c++ {
			if gFull.ChartDiscovered[r][c] {
				discoveredCount++
			}
		}
	}
	if discoveredCount != 64 {
		t.Errorf("SurveillanceFull: expected 64 discovered quads, got %d", discoveredCount)
	}

	// 2. Local surveillance reveals only docked base 3x3
	gLocal := NewGameWithOptions(200, SkillGood, LengthMedium, GameRules{Surveillance: SurveillanceLocal})
	// Reset discovered except starting
	for r := 1; r <= 8; r++ {
		for c := 1; c <= 8; c++ {
			gLocal.ChartDiscovered[r][c] = false
		}
	}
	sbQuad := Coord{2, 2}
	gLocal.Enterprise.Quad = sbQuad
	gLocal.CurrentQuad.Starbase = &Coord{5, 5}
	gLocal.Enterprise.Sector = Coord{5, 6}
	_, err = ActionDock{}.Execute(gLocal)
	if err != nil {
		t.Fatalf("ActionDock failed: %v", err)
	}
	for r := 1; r <= 8; r++ {
		for c := 1; c <= 8; c++ {
			inLocal := r >= 1 && r <= 3 && c >= 1 && c <= 3
			if gLocal.ChartDiscovered[r][c] != inLocal {
				t.Errorf("SurveillanceLocal quad [%d,%d]: got discovered=%v, want %v", r, c, gLocal.ChartDiscovered[r][c], inLocal)
			}
		}
	}

	// 3. Blackout surveillance reveals 0 additional quadrants
	gBlack := NewGameWithOptions(300, SkillGood, LengthMedium, GameRules{Surveillance: SurveillanceBlackout})
	for r := 1; r <= 8; r++ {
		for c := 1; c <= 8; c++ {
			gBlack.ChartDiscovered[r][c] = false
		}
	}
	gBlack.Enterprise.Quad = Coord{3, 3}
	gBlack.CurrentQuad.Starbase = &Coord{1, 1}
	gBlack.Enterprise.Sector = Coord{1, 2}
	events, err := ActionDock{}.Execute(gBlack)
	if err != nil {
		t.Fatalf("ActionDock failed: %v", err)
	}
	survEvent := events[1].(EventStarbaseSurveillance)
	if survEvent.UpdatedQuads != 0 {
		t.Errorf("SurveillanceBlackout: expected 0 updated quads, got %d", survEvent.UpdatedQuads)
	}
}

func TestActionLRScan_DegradedWarning(t *testing.T) {
	g := NewGameWithOptions(100, SkillGood, LengthMedium, DefaultRulesForProfile(ProfileNormal))
	g.Enterprise.Devices[DeviceLRSensors] = 1.2 // Light damage
	events, err := ActionLRScan{}.Execute(g)
	if err != nil {
		t.Fatalf("ActionLRScan should succeed with degraded warning: %v", err)
	}
	foundWarning := false
	for _, ev := range events {
		if lrs, ok := ev.(EventLRScanCompleted); ok && lrs.Degraded {
			foundWarning = true
		}
	}
	if !foundWarning {
		t.Errorf("expected EventLRScanCompleted with Degraded=true for light sensor damage")
	}
}
```

In `pkg/tui/components/statuspanel/status_test.go`:
```go
func TestRadarDegradation_TwoTier(t *testing.T) {
	th := theme.ModernTheme()

	// Light damage: [LRS DEGRADED]
	panelLight := New(th)
	dataLight := SamplePanelData()
	dataLight.Rules = engine.DefaultRulesForProfile(engine.ProfileNormal)
	dataLight.Devices[engine.DeviceLRSensors] = 1.0
	outLight := panelLight.Render(dataLight)
	if !strings.Contains(outLight, "[LRS DEGRADED]") {
		t.Errorf("expected [LRS DEGRADED] in light damage, got:\n%s", outLight)
	}

	// Heavy damage: [LRS OFFLINE]
	panelHeavy := New(th)
	dataHeavy := SamplePanelData()
	dataHeavy.Rules = engine.DefaultRulesForProfile(engine.ProfileNormal)
	dataHeavy.Devices[engine.DeviceLRSensors] = 2.5
	outHeavy := panelHeavy.Render(dataHeavy)
	if !strings.Contains(outHeavy, "[LRS OFFLINE]") {
		t.Errorf("expected [LRS OFFLINE] in heavy damage, got:\n%s", outHeavy)
	}
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `go test -v ./pkg/engine -run "TestActionDock_SurveillanceModes|TestActionLRScan_DegradedWarning"`
Expected: FAIL due to missing `Mode` / `Degraded` fields and unconditioned surveillance logic.

- [ ] **Step 3: Write minimal implementation in `pkg/engine/actions.go`, `pkg/engine/events.go`, and `pkg/tui/components/statuspanel/status.go`**

In `pkg/engine/events.go`:
Add `Mode SurveillanceMode` to `EventStarbaseSurveillance` and `Degraded bool` to `EventLRScanCompleted`.

In `pkg/engine/actions.go`:
Update `ActionDock.Execute`:
```go
switch g.Rules.Surveillance {
case SurveillanceFull:
    for r := 1; r <= 8; r++ {
        for c := 1; c <= 8; c++ {
            if (g.GalaxyChart[r][c]%100)/10 > 0 {
                g.ChartKnownBases[r][c] = true
            }
            if !g.ChartDiscovered[r][c] {
                g.ChartDiscovered[r][c] = true
                updatedQuads++
            }
        }
    }
case SurveillanceLocal:
    eq := g.Enterprise.Quad
    g.ChartKnownBases[eq[0]][eq[1]] = true
    for dr := -1; dr <= 1; dr++ {
        for dc := -1; dc <= 1; dc++ {
            nr, nc := eq[0]+dr, eq[1]+dc
            if nr >= 1 && nr <= 8 && nc >= 1 && nc <= 8 {
                if !g.ChartDiscovered[nr][nc] {
                    g.ChartDiscovered[nr][nc] = true
                    updatedQuads++
                }
            }
        }
    }
case SurveillanceBlackout:
    eq := g.Enterprise.Quad
    g.ChartKnownBases[eq[0]][eq[1]] = true
    // Discovers no extra quadrants
case SurveillanceClassic:
    fallthrough
default:
    // Existing classic loop over all starbases
}
```
Update `ActionLRScan.Execute`:
If `g.Rules.SensorDegradation && g.Enterprise.Devices[DeviceLRSensors] >= 2.0`: fail with `"LONG RANGE SENSORS DAMAGED"`.
If `g.Rules.SensorDegradation && g.Enterprise.Devices[DeviceLRSensors] > 0`: execute scan but emit `EventLRScanCompleted{Degraded: true, ...}`.

In `pkg/tui/components/statuspanel/status.go`:
Add `Rules engine.GameRules` to `PanelData`.
Update radar title rendering:
```go
if data.Rules.SensorDegradation && data.Devices[engine.DeviceLRSensors] >= 2.0 {
    radarTitle = "[LRS OFFLINE]"
} else if data.Rules.SensorDegradation && data.Devices[engine.DeviceLRSensors] > 0 {
    radarTitle = "[LRS DEGRADED]"
}
```
If degraded, render adjacent quadrants that are not yet scanned with noisy `?` instead of clean blanks.

- [ ] **Step 4: Run test to verify it passes**

Run: `go test -v ./pkg/engine -run "TestActionDock_SurveillanceModes|TestActionLRScan_DegradedWarning"`
Run: `go test -v ./pkg/tui/components/statuspanel -run "TestRadarDegradation_TwoTier"`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add pkg/engine/actions.go pkg/engine/events.go pkg/engine/actions_test.go pkg/tui/components/statuspanel/
git commit -m "feat(engine,tui): support surveillance modes and two-tier sensor degradation"
```

---

### Task 3: Tactical Realism: Klingon Commander Cloaking & Repair Multipliers (`pkg/engine`)

**Files:**
- Modify: `pkg/engine/state.go:75-83`
- Modify: `pkg/engine/combat.go:1-79`
- Modify: `pkg/engine/events.go:140-160`
- Test: `pkg/engine/combat_test.go`

**Interfaces:**
- Consumes: `g.Rules.KlingonCloak`, `g.Rules.RepairMultiplier`.
- Produces:
  - `Klingon.IsCloaked bool`.
  - `EventKlingonCloakState{KlingonID int, Cloaked bool}`.
  - `ActionTorpedoDirect` rejects target lock on cloaked vessels.
  - `ResolveShieldHit` or phaser hits decloak Klingon commanders.
  - Subsystem repair scaling: `AdvanceTurn(elapsed float64)`.

- [ ] **Step 1: Write failing tests in `pkg/engine/combat_test.go`**

```go
func TestKlingonCloaking_LockLockoutAndDecloak(t *testing.T) {
	rules := DefaultRulesForProfile(ProfileHardcore)
	g := NewGameWithOptions(100, SkillGood, LengthMedium, rules)
	klingon := &Klingon{
		ID:          1,
		Sector:      Coord{4, 4},
		Energy:      400,
		IsCommander: true,
		IsCloaked:   true,
	}
	g.CurrentQuad.Klingons = []*Klingon{klingon}
	g.CurrentQuad.Grid[4][4] = EntityCommander

	// Firing direct torpedo at cloaked commander must fail target lock
	action := ActionTorpedoDirect{TargetSector: Coord{4, 4}}
	_, err := action.Execute(g)
	if err == nil || !strings.Contains(err.Error(), "CLOAKED") {
		t.Errorf("expected error containing 'CLOAKED', got %v", err)
	}

	// Phaser sweep or direct hit decloaks commander
	events := DecloakKlingon(g, klingon)
	if klingon.IsCloaked {
		t.Errorf("klingon should be decloaked")
	}
	if len(events) == 0 {
		t.Errorf("expected EventKlingonCloakState event")
	}
}

func TestRepairMultiplier(t *testing.T) {
	rules := GameRules{RepairMultiplier: 2.0}
	g := NewGameWithOptions(100, SkillGood, LengthMedium, rules)
	g.Enterprise.Devices[DeviceWarp] = 4.0

	// Advance turn by 1.0 stardate unit
	// With 2.0x multiplier, repaired amount is 1.0 / 2.0 = 0.5 units
	AdvanceRepairs(g, 1.0)
	expected := 3.5
	if g.Enterprise.Devices[DeviceWarp] != expected {
		t.Errorf("expected device warp damage %f, got %f", expected, g.Enterprise.Devices[DeviceWarp])
	}
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `go test -v ./pkg/engine -run "TestKlingonCloaking|TestRepairMultiplier"`
Expected: FAIL due to missing fields and functions (`IsCloaked`, `DecloakKlingon`, `AdvanceRepairs`).

- [ ] **Step 3: Write minimal implementation in `pkg/engine/state.go`, `pkg/engine/combat.go`, `pkg/engine/events.go`, and `pkg/engine/engine.go`**

In `pkg/engine/state.go`:
Add `IsCloaked bool` to `Klingon`.

In `pkg/engine/events.go`:
```go
type EventKlingonCloakState struct {
	KlingonID int
	Cloaked   bool
}
```

In `pkg/engine/combat.go`:
Add check in `ActionTorpedoDirect.Execute`:
```go
for _, k := range g.CurrentQuad.Klingons {
    if k.Sector == a.TargetSector && k.IsCloaked {
        return nil, errors.New("TARGET LOCK FAILED: CLOAKED VESSEL")
    }
}
```
Add `DecloakKlingon(g *GameState, k *Klingon) []Event`:
```go
func DecloakKlingon(g *GameState, k *Klingon) []Event {
    if !k.IsCloaked {
        return nil
    }
    k.IsCloaked = false
    return []Event{
        EventKlingonCloakState{
            KlingonID: k.ID,
            Cloaked:   false,
        },
    }
}
```
In `pkg/engine/engine.go`:
```go
func AdvanceRepairs(g *GameState, elapsed float64) {
    if elapsed <= 0 {
        return
    }
    mult := g.Rules.RepairMultiplier
    if mult <= 0 {
        mult = 1.0
    }
    repairStep := elapsed / mult
    for i := range g.Enterprise.Devices {
        if g.Enterprise.Devices[i] > 0 {
            g.Enterprise.Devices[i] -= repairStep
            if g.Enterprise.Devices[i] < 0 {
                g.Enterprise.Devices[i] = 0
            }
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `go test -v ./pkg/engine -run "TestKlingonCloaking|TestRepairMultiplier"`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add pkg/engine/state.go pkg/engine/combat.go pkg/engine/events.go pkg/engine/engine.go pkg/engine/combat_test.go
git commit -m "feat(engine): implement Klingon commander cloaking and repair multiplier scaling"
```

---

### Task 4: Interactive TUI Options Modal (`pkg/tui/components/optionsmodal`)

**Files:**
- Create: `pkg/tui/components/optionsmodal/modal.go`
- Create: `pkg/tui/components/optionsmodal/modal_test.go`

**Interfaces:**
- Consumes: `theme.Theme` from `pkg/tui/theme`, `engine.GameRules` from `pkg/engine`.
- Produces:
  - `optionsmodal.Model`: Bubble Tea component managing navigation and settings cycling.
  - `New(th theme.Theme, rules engine.GameRules) Model`.
  - `Update(msg tea.Msg) (Model, tea.Cmd)`.
  - `View() string`.
  - `Rules() engine.GameRules`.
  - `Active() bool`.

- [ ] **Step 1: Write failing tests in `pkg/tui/components/optionsmodal/modal_test.go`**

```go
package optionsmodal

import (
	"testing"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

func TestOptionsModal_NavigationAndCycle(t *testing.T) {
	th := theme.ModernTheme()
	rules := engine.DefaultRulesForProfile(engine.ProfileNormal)
	m := New(th, rules)

	// Default row is 0 (Difficulty Preset)
	if m.SelectedRow != 0 {
		t.Fatalf("expected initial SelectedRow 0, got %d", m.SelectedRow)
	}

	// Press right arrow to cycle preset to Hardcore
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRight})
	if m.Rules().Profile != engine.ProfileHardcore {
		t.Errorf("expected ProfileHardcore after cycling right, got %s", m.Rules().Profile)
	}
	if m.Rules().Surveillance != engine.SurveillanceLocal {
		t.Errorf("expected SurveillanceLocal from preset cascade, got %s", m.Rules().Surveillance)
	}

	// Navigate down to Surveillance row (row 1)
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyDown})
	if m.SelectedRow != 1 {
		t.Fatalf("expected SelectedRow 1, got %d", m.SelectedRow)
	}

	// Cycle surveillance to Blackout
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRight})
	if m.Rules().Surveillance != engine.SurveillanceBlackout {
		t.Errorf("expected SurveillanceBlackout, got %s", m.Rules().Surveillance)
	}
	// Manual adjustment should tag profile as Custom
	if m.Rules().Profile != engine.ProfileCustom {
		t.Errorf("expected ProfileCustom after manual setting change, got %s", m.Rules().Profile)
	}
}

func TestOptionsModal_RenderLayout(t *testing.T) {
	th := theme.ModernTheme()
	rules := engine.DefaultRulesForProfile(engine.ProfileNormal)
	m := New(th, rules)

	view := m.View()
	expectedStrings := []string{
		"STARFLEET CONFIGURATION & RULES",
		"Difficulty Profile",
		"Starbase Surveillance",
		"Sensor Degradation",
		"Repair Multiplier",
		"Klingon Cloaking",
	}
	for _, exp := range expectedStrings {
		if !strings.Contains(view, exp) {
			t.Errorf("expected modal view to contain %q, view:\n%s", exp, view)
		}
	}
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `go test -v ./pkg/tui/components/optionsmodal`
Expected: FAIL due to package not existing.

- [ ] **Step 3: Write minimal implementation in `pkg/tui/components/optionsmodal/modal.go`**

```go
package optionsmodal

import (
	"fmt"
	"strings"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

type Row int

const (
	RowProfile Row = iota
	RowSurveillance
	RowSensorDegradation
	RowRepairMultiplier
	RowKlingonCloak
	RowTimeMargin
	RowDone
	NumRows
)

type Model struct {
	Theme       theme.Theme
	rules       engine.GameRules
	SelectedRow Row
	Closed      bool
}

func New(th theme.Theme, rules engine.GameRules) Model {
	return Model{
		Theme:       th,
		rules:       rules,
		SelectedRow: RowProfile,
		Closed:      false,
	}
}

func (m Model) Rules() engine.GameRules {
	return m.rules
}

func (m *Model) SetRules(r engine.GameRules) {
	m.rules = r
}

func (m Model) Update(msg tea.Msg) (Model, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.KeyMsg:
		switch msg.String() {
		case "up", "k":
			if m.SelectedRow > 0 {
				m.SelectedRow--
			} else {
				m.SelectedRow = NumRows - 1
			}
		case "down", "j":
			if m.SelectedRow < NumRows-1 {
				m.SelectedRow++
			} else {
				m.SelectedRow = 0
			}
		case "left", "h":
			m.cycleOption(-1)
		case "right", "l", " ":
			m.cycleOption(1)
		case "enter":
			if m.SelectedRow == RowDone {
				m.Closed = true
			} else {
				m.cycleOption(1)
			}
		case "esc", "q":
			m.Closed = true
		}
	}
	return m, nil
}

func (m *Model) cycleOption(dir int) {
	profiles := []engine.DifficultyProfile{engine.ProfileCasual, engine.ProfileNormal, engine.ProfileHardcore, engine.ProfileNightmare, engine.ProfileCustom}
	survModes := []engine.SurveillanceMode{engine.SurveillanceFull, engine.SurveillanceClassic, engine.SurveillanceLocal, engine.SurveillanceBlackout}
	repMults := []float64{0.75, 1.00, 1.50, 2.00}
	timeMargins := []float64{1.25, 1.00, 0.80, 0.60}

	switch m.SelectedRow {
	case RowProfile:
		idx := 0
		for i, p := range profiles {
			if p == m.rules.Profile {
				idx = i
				break
			}
		}
		newIdx := (idx + dir + len(profiles)) % len(profiles)
		if profiles[newIdx] != engine.ProfileCustom {
			m.rules = engine.DefaultRulesForProfile(profiles[newIdx])
		} else {
			m.rules.Profile = engine.ProfileCustom
		}
	case RowSurveillance:
		idx := 0
		for i, s := range survModes {
			if s == m.rules.Surveillance {
				idx = i
				break
			}
		}
		m.rules.Surveillance = survModes[(idx+dir+len(survModes))%len(survModes)]
		m.rules.Profile = engine.ProfileCustom
	case RowSensorDegradation:
		m.rules.SensorDegradation = !m.rules.SensorDegradation
		m.rules.Profile = engine.ProfileCustom
	case RowRepairMultiplier:
		idx := 0
		for i, rm := range repMults {
			if rm == m.rules.RepairMultiplier {
				idx = i
				break
			}
		}
		m.rules.RepairMultiplier = repMults[(idx+dir+len(repMults))%len(repMults)]
		m.rules.Profile = engine.ProfileCustom
	case RowKlingonCloak:
		m.rules.KlingonCloak = !m.rules.KlingonCloak
		m.rules.Profile = engine.ProfileCustom
	case RowTimeMargin:
		idx := 0
		for i, tm := range timeMargins {
			if tm == m.rules.TimeMargin {
				idx = i
				break
			}
		}
		m.rules.TimeMargin = timeMargins[(idx+dir+len(timeMargins))%len(timeMargins)]
		m.rules.Profile = engine.ProfileCustom
	}
}

func (m Model) View() string {
	boxStyle := lipgloss.NewStyle().
		Border(lipgloss.RoundedBorder()).
		BorderForeground(lipgloss.Color(m.Theme.BorderColor)).
		Padding(1, 2).
		Width(66)

	titleStyle := lipgloss.NewStyle().
		Bold(true).
		Foreground(lipgloss.Color(m.Theme.PrimaryColor)).
		Align(lipgloss.Center)

	var rows []string
	rows = append(rows, titleStyle.Render("⚙ STARFLEET CONFIGURATION & RULES"), "")

	renderRow := func(r Row, label, val string) string {
		prefix := "  "
		style := lipgloss.NewStyle().Foreground(lipgloss.Color(m.Theme.DimColor))
		if m.SelectedRow == r {
			prefix = "▶ "
			style = lipgloss.NewStyle().Bold(true).Foreground(lipgloss.Color(m.Theme.HighlightColor))
		}
		return prefix + style.Render(fmt.Sprintf("%-30s ◀ %s ▶", label, val))
	}

	rows = append(rows, renderRow(RowProfile, "Difficulty Profile", strings.ToUpper(string(m.rules.Profile))))
	rows = append(rows, renderRow(RowSurveillance, "Starbase Surveillance", strings.ToUpper(string(m.rules.Surveillance))))
	degStr := "DISABLED"
	if m.rules.SensorDegradation {
		degStr = "ENABLED"
	}
	rows = append(rows, renderRow(RowSensorDegradation, "Sensor Degradation", degStr))
	rows = append(rows, renderRow(RowRepairMultiplier, "Repair Multiplier", fmt.Sprintf("%.2fx", m.rules.RepairMultiplier)))
	cloakStr := "DISABLED"
	if m.rules.KlingonCloak {
		cloakStr = "ENABLED"
	}
	rows = append(rows, renderRow(RowKlingonCloak, "Klingon Cloaking", cloakStr))
	rows = append(rows, renderRow(RowTimeMargin, "Stardate Time Margin", fmt.Sprintf("%.0f%%", m.rules.TimeMargin*100)))

	doneStyle := lipgloss.NewStyle().Foreground(lipgloss.Color(m.Theme.DimColor))
	prefix := "  "
	if m.SelectedRow == RowDone {
		prefix = "▶ "
		doneStyle = lipgloss.NewStyle().Bold(true).Foreground(lipgloss.Color(m.Theme.HighlightColor))
	}
	rows = append(rows, "", prefix+doneStyle.Render("[ Done / Resume Mission ]"), "")

	footerStyle := lipgloss.NewStyle().Foreground(lipgloss.Color(m.Theme.DimColor)).Italic(true)
	rows = append(rows, footerStyle.Render("↑/↓: Navigate • ←/→/Space: Change • Esc/q: Close"))

	return boxStyle.Render(strings.Join(rows, "\n"))
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `go test -v ./pkg/tui/components/optionsmodal`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add pkg/tui/components/optionsmodal/
git commit -m "feat(tui): add interactive options modal component for game configuration"
```

---

### Task 5: TUI & CLI Integration (`pkg/tui`, `cmd/sst`)

**Files:**
- Modify: `pkg/tui/model.go`
- Modify: `pkg/tui/update.go`
- Modify: `pkg/tui/view.go`
- Modify: `pkg/tui/parser.go`
- Modify: `cmd/sst/main.go`
- Test: `pkg/tui/parser_test.go`
- Test: `pkg/tui/model_test.go`
- Test: `cmd/sst/main_test.go`

**Interfaces:**
- Consumes: `optionsmodal.Model`, `engine.GameRules`.
- Produces:
  - Key `o` / `O` and command `opts` / `options` / `settings` open `OptionsModal`.
  - Closing `OptionsModal` applies `Rules()` back to `m.game.Rules`.
  - Flags `--difficulty`, `--surveillance`, `--sensor-degradation`, `--repair-multiplier`, `--klingon-cloak` parsed in `cmd/sst/main.go`.

- [ ] **Step 1: Write failing tests in `pkg/tui/parser_test.go` & `cmd/sst/main_test.go`**

In `pkg/tui/parser_test.go`:
```go
func TestParseOptionsCommand(t *testing.T) {
	for _, cmd := range []string{"opts", "options", "settings"} {
		parsed := ParseCommand(cmd)
		if parsed.Special != "options" {
			t.Errorf("command %q: expected Special 'options', got %q", cmd, parsed.Special)
		}
	}
}
```

In `cmd/sst/main_test.go`:
```go
func TestCLIFlags_DifficultyAndSurveillance(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := run([]string{"--difficulty=nightmare", "--seed=12345"}, strings.NewReader(""), &stdout, &stderr)
	// Even in mock run, runProgram is invoked with GameState configured with Nightmare rules
	if code != 0 {
		t.Fatalf("run failed with code %d: %s", code, stderr.String())
	}
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `go test -v ./pkg/tui -run "TestParseOptionsCommand"`
Expected: FAIL due to unrecognized command.

- [ ] **Step 3: Write minimal implementation**

In `pkg/tui/parser.go`:
Add `"opts"`, `"options"`, `"settings"` cases returning `ParsedCommand{Special: "options"}`.

In `pkg/tui/model.go`:
Add `optionsModal optionsmodal.Model` and `showOptions bool` to `Model`. Initialize `optionsModal` in `NewModel`.

In `pkg/tui/update.go`:
If `m.showOptions`:
- Route keys to `m.optionsModal.Update(msg)`.
- If `m.optionsModal.Closed`:
  - `m.showOptions = false`
  - `m.game.Rules = m.optionsModal.Rules()`
  - `m.optionsModal.Closed = false`
In normal mode:
- If key is `"o"` or `"O"`, or `Special == "options"`:
  - `m.optionsModal.SetRules(m.game.Rules)`
  - `m.showOptions = true`

In `pkg/tui/view.go`:
If `m.showOptions`:
```go
overlay := m.optionsModal.View()
return lipgloss.Place(m.width, m.height, lipgloss.Center, lipgloss.Center, overlay)
```

In `cmd/sst/main.go`:
Define flags:
```go
difficulty := fs.String("difficulty", "normal", "difficulty profile (casual, normal, hardcore, nightmare)")
surveillance := fs.String("surveillance", "", "surveillance extent (full, classic, local, blackout)")
sensorDegradation := fs.Bool("sensor-degradation", true, "enable two-tier sensor degradation curve")
repairMult := fs.Float64("repair-multiplier", 1.0, "subsystem repair duration multiplier")
klingonCloak := fs.Bool("klingon-cloak", false, "enable Klingon commander tactical cloaking")
```
Resolve `rules`:
Start with `rules := engine.DefaultRulesForProfile(engine.DifficultyProfile(*difficulty))`.
Apply flag overrides if passed.
Call `game := engine.NewGameWithOptions(s, engine.SkillGood, engine.LengthMedium, rules)`.

- [ ] **Step 4: Run test to verify it passes**

Run: `go test -v ./pkg/tui -run "TestParseOptionsCommand"`
Run: `go test -v ./cmd/sst`
Run: `go test -v ./pkg/tui`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add pkg/tui/parser.go pkg/tui/parser_test.go pkg/tui/model.go pkg/tui/update.go pkg/tui/view.go cmd/sst/main.go cmd/sst/main_test.go
git commit -m "feat(tui,cli): wire options modal and CLI difficulty flags into game loop"
```

---

### Task 6: Full System Verification, Golden Snapshot Suite & Issue Closure

**Files:**
- Test: `tests/golden_test.go`
- Test: `tests/golden.sh`
- Test: `tests/tui.sh`

**Interfaces:**
- Consumes: All updated components and CLI options.
- Produces: 100% clean verification across Go and C test pipelines.

- [ ] **Step 1: Run all Go unit and race tests**

Run: `go test -v -race ./...`
Expected: All packages PASS with 0 race conditions.

- [ ] **Step 2: Run C classic test suite**

Run: `ctest --preset debug`
Expected: 100% tests pass.

- [ ] **Step 3: Run TUI shell verification suite**

Run: `bash tests/tui.sh`
Expected: PASS.

- [ ] **Step 4: Commit and finalize**

```bash
git add docs/superpowers/plans/2026-09-13-difficulty-and-realism-settings.md
git commit -m "docs(plan): add implementation plan for configurable difficulty and realism settings (#228)"
```
