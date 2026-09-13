# LRSCAN Discovery, Starbase Surveillance & Subsystem Damage Degradation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement procedural galaxy generation, long-range scan discovery (`lrscan`), starbase network surveillance upon docking, subsystem damage effects (LRS offline on radar, computer offline on star chart), and golden snapshot testing.

**Architecture:** Procedural generation and discovery state (`GalaxyChart`, `ChartDiscovered`, `ChartKnownBases`) live in `pkg/engine` and persist via JSON serialization. New actions `ActionLRScan` and enhanced `ActionDock` manage discovery and surveillance events. Presentation components `statuspanel` and `galacticchart` reactively degrade their layouts and lock out automated vectors when `DeviceLRSensors` or `DeviceComputer` are damaged.

**Tech Stack:** Go 1.26+, Bubble Tea, Lip Gloss, Bubbles, C99 (for classic verification gates).

**Spec:** [`docs/superpowers/specs/2026-09-13-lrscan-surveillance-and-damage-effects-design.md`](file:///home/scottdensmore/Developer/scottdensmore/super-star-trek/docs/superpowers/specs/2026-09-13-lrscan-surveillance-and-damage-effects-design.md)

## Global Constraints

- Target Go version: Go 1.26+ standard library and Charmbracelet packages (`bubbletea`, `lipgloss`, `bubbles`).
- Zero compiled binaries (e.g., `sst`, `sst.exe`) or temporary files committed to git.
- Maintain 100% passing tests for both Go (`go test -v -race ./...`) and C (`ctest --preset debug`).
- Classic teletype mode (`--classic`) and tournament tests remain untouched.
- Dashboard layout must strictly maintain the 80x24 terminal dimension budget (exactly 24 lines).
- Work committed on feature branch `scottdensmore/feat/lrscan-surveillance-and-damage-effects`.

---

### Task 1: Procedural Galaxy Generation & Save Persistence (`pkg/engine`)

**Files:**
- Modify: `pkg/engine/state.go:90-135`
- Modify: `pkg/engine/save.go:35-70`
- Test: `pkg/engine/geometry_test.go`

**Interfaces:**
- Consumes: `PRNG` deterministic random generation.
- Produces:
  - `GameState.ChartKnownBases [9][9]bool`
  - `NewGame(seed int64, skill SkillLevel, length GameLength) *GameState` with populated `GalaxyChart` and initial `ChartDiscovered` + `ChartKnownBases`.

- [ ] **Step 1: Write failing tests in `pkg/engine/geometry_test.go`**

```go
func TestProceduralGalaxyGeneration(t *testing.T) {
	seed := int64(12345)
	g := NewGame(seed, SkillGood, LengthMedium)

	totalStars := 0
	totalStarbases := 0
	totalKlingons := 0

	for r := 1; r <= 8; r++ {
		for c := 1; c <= 8; c++ {
			val := g.GalaxyChart[r][c]
			k := val / 100
			b := (val % 100) / 10
			s := val % 10

			if s < 1 || s > 9 {
				t.Errorf("quadrant [%d,%d] invalid star count: %d", r, c, s)
			}
			totalStars += s
			totalStarbases += b
			totalKlingons += k

			if b > 0 && !g.ChartKnownBases[r][c] {
				t.Errorf("quadrant [%d,%d] has starbase but ChartKnownBases is false", r, c)
			}
		}
	}

	if totalStarbases != g.RemainingStarbases {
		t.Errorf("expected total starbases %d, got %d", g.RemainingStarbases, totalStarbases)
	}
	if totalKlingons != g.RemainingKlingons {
		t.Errorf("expected total klingons %d, got %d", g.RemainingKlingons, totalKlingons)
	}
	if !g.ChartDiscovered[g.Enterprise.Quad[0]][g.Enterprise.Quad[1]] {
		t.Errorf("starting quadrant %v was not marked discovered", g.Enterprise.Quad)
	}
}

func TestSaveRoundtripDiscoveryAndBases(t *testing.T) {
	tempDir := t.TempDir()
	savePath := filepath.Join(tempDir, "TESTDISC.TRK")

	g := NewGame(12345, SkillGood, LengthMedium)
	g.ChartDiscovered[2][3] = true
	g.ChartKnownBases[4][5] = true

	if err := g.Save(savePath); err != nil {
		t.Fatalf("failed to save: %v", err)
	}

	loaded, err := LoadGame(savePath)
	if err != nil {
		t.Fatalf("failed to load: %v", err)
	}

	if loaded.GalaxyChart != g.GalaxyChart {
		t.Errorf("GalaxyChart mismatch after save/load")
	}
	if loaded.ChartDiscovered != g.ChartDiscovered {
		t.Errorf("ChartDiscovered mismatch after save/load")
	}
	if loaded.ChartKnownBases != g.ChartKnownBases {
		t.Errorf("ChartKnownBases mismatch after save/load")
	}
}
```

- [ ] **Step 2: Run tests to verify failure**

Run: `go test -v -race ./pkg/engine -run "TestProceduralGalaxyGeneration|TestSaveRoundtripDiscoveryAndBases"`
Expected: FAIL due to zeroed `GalaxyChart` and missing `ChartKnownBases`.

- [ ] **Step 3: Implement procedural galaxy generation in `pkg/engine/state.go`**

Add `ChartKnownBases` to `GameState`:
```go
type GameState struct {
	RNG                *PRNG
	Skill              SkillLevel
	Length             GameLength
	Enterprise         Enterprise
	CurrentQuad        QuadrantState
	GalaxyChart        [9][9]int // Klingons*100 + Starbases*10 + Stars
	ChartDiscovered    [9][9]bool
	ChartKnownBases    [9][9]bool
	RemainingKlingons  int
	RemainingStarbases int
	Stardate           float64
	InitialStardate    float64
	TimeRemaining      float64
}
```

In `NewGame`:
```go
	// Procedural generation:
	// 1. Stars (1..9 in each quadrant)
	for r := 1; r <= 8; r++ {
		for c := 1; c <= 8; c++ {
			g.GalaxyChart[r][c] = rng.Intn(9) + 1
		}
	}

	// 2. Starbases across RemainingStarbases distinct quadrants
	placedBases := 0
	for placedBases < g.RemainingStarbases {
		r := rng.Intn(8) + 1
		c := rng.Intn(8) + 1
		if (g.GalaxyChart[r][c] % 100) / 10 == 0 {
			g.GalaxyChart[r][c] += 10
			g.ChartKnownBases[r][c] = true
			placedBases++
		}
	}

	// 3. Klingons distributed in clusters of 1..3
	klingonsToPlace := g.RemainingKlingons
	for klingonsToPlace > 0 {
		r := rng.Intn(8) + 1
		c := rng.Intn(8) + 1
		currentK := g.GalaxyChart[r][c] / 100
		if currentK < 9 {
			cluster := rng.Intn(3) + 1
			if cluster > klingonsToPlace {
				cluster = klingonsToPlace
			}
			if currentK + cluster > 9 {
				cluster = 9 - currentK
			}
			g.GalaxyChart[r][c] += cluster * 100
			klingonsToPlace -= cluster
		}
	}

	// 4. Starting quadrant discovered
	g.ChartDiscovered[g.Enterprise.Quad[0]][g.Enterprise.Quad[1]] = true
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `go test -v -race ./pkg/engine -run "TestProceduralGalaxyGeneration|TestSaveRoundtripDiscoveryAndBases"`
Expected: PASS.

- [ ] **Step 5: Commit changes**

```bash
git add pkg/engine/
git commit -m "feat(engine): procedurally generate galaxy chart and persist known bases"
```

---

### Task 2: ActionLRScan & Starbase Docking Surveillance (`pkg/engine`)

**Files:**
- Modify: `pkg/engine/actions.go`
- Modify: `pkg/engine/events.go`
- Test: `pkg/engine/engine_test.go`

**Interfaces:**
- Consumes: `GameState.GalaxyChart`, `GameState.ChartDiscovered`, `GameState.ChartKnownBases`, `Enterprise.Devices[DeviceLRSensors]`, `Enterprise.Condition`.
- Produces:
  - `ActionLRScan{}`
  - `EventLRScanCompleted{CenterQuad Coord, ScannedQuads []Coord, RelayedByBase bool}`
  - `EventStarbaseSurveillance{StarbaseCoord Coord, UpdatedQuads int}`

- [ ] **Step 1: Write failing tests in `pkg/engine/engine_test.go`**

```go
func TestActionLRScan_Operational(t *testing.T) {
	g := NewGame(12345, SkillGood, LengthMedium)
	g.Enterprise.Quad = Coord{3, 3}
	g.Enterprise.Devices[DeviceLRSensors] = 0

	events, err := g.Dispatch(ActionLRScan{})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(events) != 1 {
		t.Fatalf("expected 1 event, got %d", len(events))
	}
	scanEvt, ok := events[0].(EventLRScanCompleted)
	if !ok {
		t.Fatalf("expected EventLRScanCompleted, got %T", events[0])
	}
	if scanEvt.RelayedByBase {
		t.Errorf("expected RelayedByBase false")
	}
	if len(scanEvt.ScannedQuads) != 9 {
		t.Errorf("expected 9 scanned quads, got %d", len(scanEvt.ScannedQuads))
	}
	for dr := -1; dr <= 1; dr++ {
		for dc := -1; dc <= 1; dc++ {
			if !g.ChartDiscovered[3+dr][3+dc] {
				t.Errorf("quad [%d,%d] not marked discovered", 3+dr, 3+dc)
			}
		}
	}
}

func TestActionLRScan_DamagedUndocked(t *testing.T) {
	g := NewGame(12345, SkillGood, LengthMedium)
	g.Enterprise.Devices[DeviceLRSensors] = 2.5
	g.Enterprise.Condition = ConditionGreen

	_, err := g.Dispatch(ActionLRScan{})
	if err == nil {
		t.Fatalf("expected error when LRS damaged and undocked, got nil")
	}
}

func TestActionLRScan_DamagedDocked(t *testing.T) {
	g := NewGame(12345, SkillGood, LengthMedium)
	g.Enterprise.Devices[DeviceLRSensors] = 2.5
	g.Enterprise.Condition = ConditionDocked

	events, err := g.Dispatch(ActionLRScan{})
	if err != nil {
		t.Fatalf("unexpected error when docked with damaged LRS: %v", err)
	}
	scanEvt := events[0].(EventLRScanCompleted)
	if !scanEvt.RelayedByBase {
		t.Errorf("expected RelayedByBase true")
	}
}

func TestActionDock_Surveillance(t *testing.T) {
	g := NewGame(12345, SkillGood, LengthMedium)
	sbCoord := Coord{4, 4}
	g.CurrentQuad.Starbase = &sbCoord
	g.CurrentQuad.Grid[4][4] = EntityStarbase
	g.Enterprise.Sector = Coord{4, 5}
	g.Enterprise.Condition = ConditionGreen

	// Set a starbase at quad [5, 5]
	g.GalaxyChart[5][5] = 15 // 1 starbase, 5 stars

	events, err := g.Dispatch(ActionDock{})
	if err != nil {
		t.Fatalf("unexpected error docking: %v", err)
	}

	foundSurveillance := false
	for _, ev := range events {
		if surv, ok := ev.(EventStarbaseSurveillance); ok {
			foundSurveillance = true
			if surv.StarbaseCoord != sbCoord {
				t.Errorf("expected starbase coord %v, got %v", sbCoord, surv.StarbaseCoord)
			}
		}
	}
	if !foundSurveillance {
		t.Errorf("EventStarbaseSurveillance was not emitted")
	}
	// Verify 3x3 perimeter around starbase [5, 5] was discovered
	for dr := -1; dr <= 1; dr++ {
		for dc := -1; dc <= 1; dc++ {
			if !g.ChartDiscovered[5+dr][5+dc] {
				t.Errorf("perimeter quad [%d,%d] of starbase [5,5] not marked discovered", 5+dr, 5+dc)
			}
		}
	}
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `go test -v -race ./pkg/engine -run "TestActionLRScan|TestActionDock_Surveillance"`
Expected: FAIL due to missing `ActionLRScan` and `EventStarbaseSurveillance`.

- [ ] **Step 3: Implement `ActionLRScan` and `ActionDock` surveillance in `pkg/engine/actions.go` and `events.go`**

In `pkg/engine/events.go`:
```go
// EventLRScanCompleted is emitted when a long-range scan completes.
type EventLRScanCompleted struct {
	CenterQuad    Coord
	ScannedQuads  []Coord
	RelayedByBase bool
}

func (e EventLRScanCompleted) EventType() string { return "LRScanCompleted" }

// EventStarbaseSurveillance is emitted when starbase records update the galactic star chart.
type EventStarbaseSurveillance struct {
	StarbaseCoord Coord
	UpdatedQuads  int
}

func (e EventStarbaseSurveillance) EventType() string { return "StarbaseSurveillance" }
```

In `pkg/engine/actions.go`:
```go
type ActionLRScan struct{}

func (a ActionLRScan) Execute(g *GameState) ([]Event, error) {
	if g.Enterprise.Devices[DeviceLRSensors] > 0 && g.Enterprise.Condition != ConditionDocked {
		return nil, errors.New("long-range sensors damaged")
	}

	relayed := (g.Enterprise.Condition == ConditionDocked && g.Enterprise.Devices[DeviceLRSensors] > 0)
	center := g.Enterprise.Quad
	var scanned []Coord

	for dr := -1; dr <= 1; dr++ {
		for dc := -1; dc <= 1; dc++ {
			r := center[0] + dr
			c := center[1] + dc
			if r >= 1 && r <= 8 && c >= 1 && c <= 8 {
				g.ChartDiscovered[r][c] = true
				scanned = append(scanned, Coord{r, c})
			}
		}
	}

	return []Event{
		EventLRScanCompleted{
			CenterQuad:    center,
			ScannedQuads:  scanned,
			RelayedByBase: relayed,
		},
	}, nil
}
```

In `ActionDock.Execute`:
```go
	// Starbase surveillance download
	updatedQuads := 0
	for r := 1; r <= 8; r++ {
		for c := 1; c <= 8; c++ {
			if (g.GalaxyChart[r][c] % 100) / 10 > 0 {
				g.ChartKnownBases[r][c] = true
				for dr := -1; dr <= 1; dr++ {
					for dc := -1; dc <= 1; dc++ {
						nr := r + dr
						nc := c + dc
						if nr >= 1 && nr <= 8 && nc >= 1 && nc <= 8 {
							if !g.ChartDiscovered[nr][nc] {
								g.ChartDiscovered[nr][nc] = true
								updatedQuads++
							}
						}
					}
				}
			}
		}
	}

	return []Event{
		EventDocked{
			Starbase: sb,
		},
		EventStarbaseSurveillance{
			StarbaseCoord: sb,
			UpdatedQuads:  updatedQuads,
		},
	}, nil
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `go test -v -race ./pkg/engine -run "TestActionLRScan|TestActionDock_Surveillance"`
Expected: PASS.

- [ ] **Step 5: Commit changes**

```bash
git add pkg/engine/
git commit -m "feat(engine): implement ActionLRScan and starbase docking surveillance"
```

---

### Task 3: Status Panel Radar Subsystem Degradation (`pkg/tui/components/statuspanel`)

**Files:**
- Modify: `pkg/tui/components/statuspanel/status.go:120-185`
- Test: `pkg/tui/components/statuspanel/status_test.go`

**Interfaces:**
- Consumes: `Enterprise.Devices[engine.DeviceLRSensors]`, `isDocked bool`.
- Produces:
  - Header: `RADAR (QUAD ±1) [LRS OFFLINE]:` when `DeviceLRSensors > 0 && !isDocked`.
  - Surrounding cells rendered as `???` in warning/dimmed style.
  - Current quadrant cell remains visible.

- [ ] **Step 1: Write failing tests in `pkg/tui/components/statuspanel/status_test.go`**

```go
func TestStatusPanel_RadarLrsDamaged(t *testing.T) {
	th := theme.ModernTheme{}
	m := New(th, 30, 16)
	ent := engine.Enterprise{
		Quad:      engine.Coord{4, 4},
		Sector:    engine.Coord{2, 3},
		Energy:    4500,
		Shields:   1000,
		Torpedoes: 8,
		Condition: engine.ConditionGreen,
	}
	ent.Devices[engine.DeviceLRSensors] = 2.5 // Damaged!

	var chart [9][9]int
	chart[4][4] = 105
	chart[3][4] = 203

	m.SetState(ent, 25.0, 10, 3, false, chart)
	view := m.View()

	if !strings.Contains(view, "[LRS OFFLINE]") {
		t.Errorf("expected view to contain '[LRS OFFLINE]', got:\n%s", view)
	}
	if !strings.Contains(view, "???") {
		t.Errorf("expected surrounding cells to display '???', got:\n%s", view)
	}
	// Current cell 105 should still be visible
	if !strings.Contains(view, "105") {
		t.Errorf("expected current quadrant cell '105' to remain visible, got:\n%s", view)
	}
}

func TestStatusPanel_RadarLrsDamagedDocked(t *testing.T) {
	th := theme.ModernTheme{}
	m := New(th, 30, 16)
	ent := engine.Enterprise{
		Quad:      engine.Coord{4, 4},
		Sector:    engine.Coord{2, 3},
		Energy:    4500,
		Shields:   1000,
		Torpedoes: 8,
		Condition: engine.ConditionDocked,
	}
	ent.Devices[engine.DeviceLRSensors] = 2.5 // Damaged, but docked!

	var chart [9][9]int
	chart[4][4] = 105
	chart[3][4] = 203

	m.SetState(ent, 25.0, 10, 3, true, chart)
	view := m.View()

	if strings.Contains(view, "[LRS OFFLINE]") {
		t.Errorf("did not expect '[LRS OFFLINE]' when docked")
	}
	if !strings.Contains(view, "203") {
		t.Errorf("expected surrounding cell '203' to be visible using starbase relay, got:\n%s", view)
	}
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `go test -v -race ./pkg/tui/components/statuspanel -run "TestStatusPanel_RadarLrsDamaged"`
Expected: FAIL.

- [ ] **Step 3: Implement radar degradation in `pkg/tui/components/statuspanel/status.go`**

In `renderRadar`:
```go
	lrsDamaged := m.enterprise.Devices[engine.DeviceLRSensors] > 0 && !m.isDocked
	radarTitle := styles.TextMuted.Render("RADAR (QUADRANTS ±1)  [K-B-S]:")
	if lrsDamaged {
		radarTitle = styles.TextWarn.Render("RADAR (QUAD ±1) [LRS OFFLINE]:")
	}
```
In cell loop:
```go
				if dr == 0 && dc == 0 {
					// Enterprise quadrant remains visible via short range sensors
					content = styles.Enterprise.Render(fmt.Sprintf("<%03d>", val))
				} else if lrsDamaged {
					content = styles.TextWarn.Render(" ??? ")
				} else {
					content = styles.Normal.Render(fmt.Sprintf(" %03d ", val))
				}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `go test -v -race ./pkg/tui/components/statuspanel -run "TestStatusPanel_RadarLrsDamaged"`
Expected: PASS.

- [ ] **Step 5: Commit changes**

```bash
git add pkg/tui/components/statuspanel/
git commit -m "feat(statuspanel): degrade radar display when long-range sensors damaged"
```

---

### Task 4: Galactic Star Chart Modal Known Bases & Computer Damage Degradation (`pkg/tui/components/galacticchart`)

**Files:**
- Modify: `pkg/tui/components/galacticchart/chart.go`
- Test: `pkg/tui/components/galacticchart/chart_test.go`

**Interfaces:**
- Produces:
  - `WarpBlockedMsg{Reason string}`
  - `SetState(entQuad engine.Coord, chart [9][9]int, discovered [9][9]bool, knownBases [9][9]bool, computerDamaged bool)`
  - `.1.` rendering for unvisited known starbases.
  - `[CALC OFFLINE]` rendering and `Enter` lockout when `computerDamaged == true`.

- [ ] **Step 1: Write failing tests in `pkg/tui/components/galacticchart/chart_test.go`**

```go
func TestGalacticChart_KnownBaseRendering(t *testing.T) {
	th := theme.ModernTheme{}
	m := New(th, 64, 18)
	var chart [9][9]int
	var disc [9][9]bool
	var knownBases [9][9]bool

	chart[2][3] = 15 // base at [2,3]
	knownBases[2][3] = true
	// Not yet discovered!

	m.SetState(engine.Coord{4, 4}, chart, disc, knownBases, false)
	view := m.View()

	if !strings.Contains(view, ".1.") {
		t.Errorf("expected view to contain '.1.' for known base quadrant, got:\n%s", view)
	}
}

func TestGalacticChart_ComputerDamagedTelemetryAndLockout(t *testing.T) {
	th := theme.ModernTheme{}
	m := New(th, 64, 18)
	var chart [9][9]int
	var disc [9][9]bool
	var knownBases [9][9]bool

	m.SetState(engine.Coord{4, 4}, chart, disc, knownBases, true) // Computer damaged!
	view := m.View()

	if !strings.Contains(view, "[CALC OFFLINE]") {
		t.Errorf("expected '[CALC OFFLINE]' in footer, got:\n%s", view)
	}
	if !strings.Contains(view, "Disabled (Comp Offline)") {
		t.Errorf("expected action hint to indicate Enter disabled, got:\n%s", view)
	}

	// Test Enter lockout
	m.cursor = engine.Coord{6, 6}
	_, cmd := m.Update(tea.KeyMsg{Type: tea.KeyEnter})
	if cmd == nil {
		t.Fatalf("expected command emitting WarpBlockedMsg, got nil")
	}
	msg := cmd()
	blocked, ok := msg.(WarpBlockedMsg)
	if !ok {
		t.Fatalf("expected WarpBlockedMsg, got %T", msg)
	}
	if !strings.Contains(blocked.Reason, "COMPUTER DAMAGED") {
		t.Errorf("expected reason to contain 'COMPUTER DAMAGED', got: %s", blocked.Reason)
	}
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `go test -v -race ./pkg/tui/components/galacticchart -run "TestGalacticChart_KnownBase|TestGalacticChart_ComputerDamaged"`
Expected: FAIL.

- [ ] **Step 3: Implement `knownBases`, `computerDamaged`, and `WarpBlockedMsg` in `chart.go`**

In `pkg/tui/components/galacticchart/chart.go`:
```go
// WarpBlockedMsg is emitted when quick-warp is attempted with a damaged library computer.
type WarpBlockedMsg struct {
	Reason string
}

type Model struct {
	theme           theme.Theme
	cursor          engine.Coord
	enterpriseQuad  engine.Coord
	galaxyChart     [9][9]int
	chartDiscovered [9][9]bool
	knownBases      [9][9]bool
	computerDamaged bool
	width           int
	height          int
}

func (m *Model) SetState(entQuad engine.Coord, chart [9][9]int, discovered [9][9]bool, knownBases [9][9]bool, computerDamaged bool) {
	m.enterpriseQuad = entQuad
	m.enterpriseQuad[0] = clampCoord(m.enterpriseQuad[0], 1, 8)
	m.enterpriseQuad[1] = clampCoord(m.enterpriseQuad[1], 1, 8)
	m.galaxyChart = chart
	m.chartDiscovered = discovered
	m.knownBases = knownBases
	m.computerDamaged = computerDamaged
	m.cursor = m.enterpriseQuad
	m.cursor[0] = clampCoord(m.cursor[0], 1, 8)
	m.cursor[1] = clampCoord(m.cursor[1], 1, 8)
}
```

In `renderGrid`:
```go
			if isEnt {
				if m.chartDiscovered[r][c] {
					cellText = fmt.Sprintf("<%03d>", m.galaxyChart[r][c])
				} else if m.knownBases[r][c] {
					cellText = "<.1.>"
				} else {
					cellText = "<···>"
				}
			} else if isCur {
				if m.chartDiscovered[r][c] {
					cellText = fmt.Sprintf("[%03d]", m.galaxyChart[r][c])
				} else if m.knownBases[r][c] {
					cellText = "[.1.]"
				} else {
					cellText = "[···]"
				}
			} else {
				if m.chartDiscovered[r][c] {
					cellText = fmt.Sprintf(" %03d ", m.galaxyChart[r][c])
				} else if m.knownBases[r][c] {
					cellText = " .1. "
				} else {
					cellText = " ··· "
				}
			}
```

In `renderFooter`:
```go
	var distanceStr, vectorStr, actionStr string
	if m.computerDamaged {
		distanceStr = styles.TextWarn.Render("Distance: [CALC OFFLINE]")
		vectorStr = styles.TextWarn.Render("Course: [CALC OFFLINE] • Warp: [CALC OFFLINE]")
		actionStr = styles.TextMuted.Render("[Enter] Disabled (Comp Offline)  [Arrows/HJKL] Move  [Esc] Close")
	} else {
		// existing telemetry calculation
```

In `Update` on `KeyEnter`:
```go
		case msg.Type == tea.KeyEnter:
			if m.computerDamaged {
				return m, func() tea.Msg {
					return WarpBlockedMsg{
						Reason: "COMPUTER DAMAGED, USE A POCKET CALCULATOR. Manual navigation required (nav q <r> <c> [warp]).",
					}
				}
			}
			telemetry := CalculateTelemetry(m.enterpriseQuad, m.cursor)
			return m, func() tea.Msg {
				return WarpToQuadrantMsg{
					DestQuad: m.cursor,
					Warp:     telemetry.RecommendedWarp,
				}
			}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `go test -v -race ./pkg/tui/components/galacticchart -run "TestGalacticChart_KnownBase|TestGalacticChart_ComputerDamaged"`
Expected: PASS.

- [ ] **Step 5: Commit changes**

```bash
git add pkg/tui/components/galacticchart/
git commit -m "feat(galacticchart): add known base rendering and computer damage degradation"
```

---

### Task 5: Root TUI Integration, Dynamic Damage Report & Golden Snapshots (`pkg/tui`, `tests`)

**Files:**
- Modify: `pkg/tui/update.go`
- Modify: `pkg/tui/view.go`
- Modify: `pkg/tui/model_test.go`
- Modify: `tests/tui_golden_test.go`
- Golden snapshots: `tests/golden/tui/*.golden`

**Interfaces:**
- Consumes: `engine.ActionLRScan`, `engine.EventLRScanCompleted`, `engine.EventStarbaseSurveillance`, `galacticchart.WarpBlockedMsg`.
- Produces:
  - Dynamic `dam` / `damages` reporting
  - Connected `lrscan` and `dock` event log feedback
  - Golden snapshot `lrs_damaged_dashboard_80x24.golden`

- [ ] **Step 1: Write failing tests in `pkg/tui/model_test.go`**

```go
func TestModel_LRScanCommand(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	g.Enterprise.Devices[engine.DeviceLRSensors] = 0
	m := NewModel(g, theme.ModernTheme{})

	updated, _ := m.handleCommand("lrscan")
	mod := updated.(Model)

	messages := mod.CommandBar.Messages()
	if len(messages) == 0 || !strings.Contains(messages[len(messages)-1], "Long-range scan complete") {
		t.Errorf("expected completion message for lrscan, got: %v", messages)
	}
}

func TestModel_DockSurveillance(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	sbCoord := engine.Coord{4, 4}
	g.CurrentQuad.Starbase = &sbCoord
	g.CurrentQuad.Grid[4][4] = engine.EntityStarbase
	g.Enterprise.Sector = engine.Coord{4, 5}
	g.Enterprise.Condition = engine.ConditionGreen

	m := NewModel(g, theme.ModernTheme{})
	updated, _ := m.handleCommand("dock")
	mod := updated.(Model)

	messages := mod.CommandBar.Messages()
	foundSurveillance := false
	for _, msg := range messages {
		if strings.Contains(msg, "surveillance") {
			foundSurveillance = true
			break
		}
	}
	if !foundSurveillance {
		t.Errorf("expected docking to log starbase surveillance message, got: %v", messages)
	}
}

func TestModel_DynamicDamageReport(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	g.Enterprise.Devices[engine.DeviceLRSensors] = 3.2
	g.Enterprise.Devices[engine.DeviceComputer] = 1.5

	m := NewModel(g, theme.ModernTheme{})
	updated, _ := m.handleCommand("dam")
	mod := updated.(Model)

	messages := mod.CommandBar.Messages()
	lastMsg := messages[len(messages)-1]
	if !strings.Contains(lastMsg, "LRS") || !strings.Contains(lastMsg, "Computer") {
		t.Errorf("expected damage report to list damaged devices, got: %s", lastMsg)
	}
}

func TestModel_GalacticChart_WarpBlocked(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	g.Enterprise.Devices[engine.DeviceComputer] = 2.0 // Computer damaged!
	m := NewModel(g, theme.ModernTheme{})

	// Open chart
	m.openGalacticChart()
	if m.ActiveModal != ModalGalacticChart {
		t.Fatalf("expected ModalGalacticChart")
	}

	// Send WarpBlockedMsg
	updated, _ := m.Update(galacticchart.WarpBlockedMsg{
		Reason: "COMPUTER DAMAGED, USE A POCKET CALCULATOR.",
	})
	mod := updated.(Model)

	if mod.ActiveModal != ModalNone {
		t.Errorf("expected modal to close on WarpBlockedMsg")
	}
	messages := mod.CommandBar.Messages()
	if len(messages) == 0 || !strings.Contains(messages[len(messages)-1], "COMPUTER DAMAGED") {
		t.Errorf("expected pocket calculator warning message, got: %v", messages)
	}
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `go test -v -race ./pkg/tui -run "TestModel_LRScanCommand|TestModel_DockSurveillance|TestModel_DynamicDamageReport|TestModel_GalacticChart_WarpBlocked"`
Expected: FAIL.

- [ ] **Step 3: Implement event routing and damage reporting in `pkg/tui/update.go`**

In `openGalacticChart()`:
```go
func (m Model) openGalacticChart() (Model, tea.Cmd) {
	var entQuad engine.Coord = engine.Coord{1, 1}
	var chart [9][9]int
	var discovered [9][9]bool
	var knownBases [9][9]bool
	var compDamaged bool
	if m.Game != nil {
		entQuad = m.Game.Enterprise.Quad
		chart = m.Game.GalaxyChart
		discovered = m.Game.ChartDiscovered
		knownBases = m.Game.ChartKnownBases
		compDamaged = m.Game.Enterprise.Devices[engine.DeviceComputer] > 0
	}
	m.GalacticChart.SetState(entQuad, chart, discovered, knownBases, compDamaged)
	m.ActiveModal = ModalGalacticChart
	m.CommandBar.Blur()
	return m, nil
}
```

In `handleCommand`:
- For `case "lrscan":`:
  ```go
	case "lrscan":
		if m.Game == nil {
			m.CommandBar.AddMessage("No active game.")
			return m, nil
		}
		events, err := m.Game.Dispatch(engine.ActionLRScan{})
		if err != nil {
			m.CommandBar.AddMessage(fmt.Sprintf("LONG-RANGE SENSORS DAMAGED. %v", err))
			return m, nil
		}
		for _, ev := range events {
			if scanEvt, ok := ev.(engine.EventLRScanCompleted); ok {
				if scanEvt.RelayedByBase {
					m.CommandBar.AddMessage("Starbase relay: Long-range scan complete. Star chart updated.")
				} else {
					m.CommandBar.AddMessage("Long-range scan complete. Star chart updated for 3x3 surrounding quadrants.")
				}
			}
		}
		return m, nil
  ```
- For `case "dam", "damages":`:
  ```go
	case "dam", "damages":
		if m.Game == nil {
			m.CommandBar.AddMessage("No active game.")
			return m, nil
		}
		var damagedList []string
		for dev := engine.DeviceID(0); dev < engine.NumDevices; dev++ {
			turns := m.Game.Enterprise.Devices[dev]
			if turns > 0 {
				damagedList = append(damagedList, fmt.Sprintf("%s: %.1f stardates", deviceShortString(dev), turns))
			}
		}
		if len(damagedList) == 0 {
			m.CommandBar.AddMessage("Damage report: all systems nominal.")
		} else {
			m.CommandBar.AddMessage("Damage report: " + strings.Join(damagedList, ", "))
		}
		return m, nil
  ```
- Handle `case galacticchart.WarpBlockedMsg`:
  ```go
	case galacticchart.WarpBlockedMsg:
		m.ActiveModal = ModalNone
		m.CommandBar.Focus()
		m.CommandBar.AddMessage(msg.Reason)
		return m, nil
  ```
- In `formatEvent`:
  ```go
	case engine.EventStarbaseSurveillance:
		return fmt.Sprintf("Starbase at [%d,%d] downloaded surveillance. Updated %d quadrants.", e.StarbaseCoord[0], e.StarbaseCoord[1], e.UpdatedQuads)
  ```

- [ ] **Step 4: Run tests to verify they pass**

Run: `go test -v -race ./pkg/tui -run "TestModel_LRScanCommand|TestModel_DockSurveillance|TestModel_DynamicDamageReport|TestModel_GalacticChart_WarpBlocked"`
Expected: PASS.

- [ ] **Step 5: Add golden snapshot test `TestTUIGolden_LrsDamagedDashboard` in `tests/tui_golden_test.go`**

```go
func TestTUIGolden_LrsDamagedDashboard(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	g.Enterprise.Devices[engine.DeviceLRSensors] = 3.5 // Damaged!
	m := tui.NewModel(g, theme.ModernTheme{})
	updated, _ := m.Update(tea.WindowSizeMsg{Width: 80, Height: 24})
	m = updated.(tui.Model)
	compareOrUpdate(t, "lrs_damaged_dashboard_80x24", m.View())
}
```

Generate golden snapshot:
```bash
go test ./tests -run "TestTUIGolden_LrsDamagedDashboard" -update
```

- [ ] **Step 6: Verify full test suites**

Run Go tests:
```bash
go test -v -race ./...
```
Run C tests:
```bash
ctest --preset debug
```

- [ ] **Step 7: Commit changes**

```bash
git add pkg/tui/ tests/
git commit -m "feat(tui): route LRScan, starbase surveillance, dynamic damage reports, and snapshot testing"
```
