# Galactic Star Chart & Quadrant Navigation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the interactive 8x8 Galactic Star Chart modal (`pkg/tui/components/galacticchart`), HUD location and radar coordinate readouts (`pkg/tui/components/statuspanel`), root Bubble Tea modal routing and hotkey (`Ctrl+M`), and enhanced in-game navigation guidance.

**Architecture:** A dedicated, testable `galacticchart` component renders an interactive 64x18 modal over the dashboard using ANSI-aware line compositing. The status panel is updated with prominent `LOCATION: Quad [r, c]  Sec [r, c]` and coordinate-labeled 3x3 Radar. Keystrokes are routed via `ActiveModal == ModalGalacticChart`, emitting `WarpToQuadrantMsg` on `Enter` to directly maneuver the ship.

**Tech Stack:** Go 1.26+, Bubble Tea (`bubbletea`), Lip Gloss (`lipgloss`), standard library math/geometry.

**Spec:** [`docs/superpowers/specs/2026-09-12-galactic-chart-and-quadrant-navigation-design.md`](file:///home/scottdensmore/Developer/scottdensmore/super-star-trek/docs/superpowers/specs/2026-09-12-galactic-chart-and-quadrant-navigation-design.md)

## Global Constraints

- Go 1.26+ standard library and Charmbracelet packages (`bubbletea`, `lipgloss`).
- Zero changes or regressions to `pkg/engine` simulation logic.
- Coordinates are 1-indexed (1..8) for Quadrants and Sectors.
- Minimum terminal dimensions guard: strictly enforce 80x24 characters with centered alert dialog.
- Modals must be composited over the background dashboard using `compositeOverlay` without clearing or corrupting the view.
- Every task must pass `go test -v -race ./...` with zero failures and zero race conditions.
- Preserves all C CI gates (`ctest --preset debug`).
- All work committed on feature branch `scottdensmore/feat/nav-help-and-quadrant-movement`.

---

### Task 1: Status Panel Location Readout & Radar Enhancements

**Files:**
- Modify: `pkg/tui/components/statuspanel/status.go:74-165`
- Test: `pkg/tui/components/statuspanel/status_test.go`

**Interfaces:**
- Consumes: `g.Enterprise.Quad`, `g.Enterprise.Sector`, `g.GalaxyChart`
- Produces: Updated status panel rendering with location line and coordinate-labeled radar.

- [ ] **Step 1: Write failing unit test for Status Panel location line and radar coordinates**

In `pkg/tui/components/statuspanel/status_test.go`:
```go
func TestStatusPanel_LocationReadoutAndRadarHeaders(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	g.Enterprise.Quad = engine.Coord{3, 5}
	g.Enterprise.Sector = engine.Coord{2, 6}
	g.GalaxyChart[3][5] = 3
	g.GalaxyChart[2][5] = 105
	g.GalaxyChart[3][6] = 12

	m := New(theme.DefaultTheme())
	view := m.View(g)

	if !strings.Contains(view, "LOCATION: Quad [3, 5]   Sec [2, 6]") {
		t.Fatalf("expected location readout 'LOCATION: Quad [3, 5]   Sec [2, 6]', got:\n%s", view)
	}
	if !strings.Contains(view, "RADAR (QUADRANTS ±1)  [K-B-S]:") {
		t.Fatalf("expected radar header with K-B-S legend, got:\n%s", view)
	}
	// Verify coordinate headers for row 2, 3, 4 and col 4, 5, 6
	if !strings.Contains(view, "4    5    6") {
		t.Fatalf("expected radar column headers '4    5    6', got:\n%s", view)
	}
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `go test -v -run TestStatusPanel_LocationReadoutAndRadarHeaders ./pkg/tui/components/statuspanel`
Expected: FAIL with missing location readout / header text.

- [ ] **Step 3: Implement location line and radar headers in `status.go`**

In `pkg/tui/components/statuspanel/status.go`:
1. Render location line before `condBanner`:
   ```go
   locLabel := styles.GaugeLabel.Render("LOCATION: ")
   locVal := styles.Prompt.Render(fmt.Sprintf("Quad [%d, %d]   Sec [%d, %d]", g.Enterprise.Quad[0], g.Enterprise.Quad[1], g.Enterprise.Sector[0], g.Enterprise.Sector[1]))
   locationLine := locLabel + locVal
   ```
2. Update radar section:
   ```go
   radarHeader := styles.PanelTitle.Render("RADAR (QUADRANTS ±1)  [K-B-S]:")
   qr := g.Enterprise.Quad[0]
   qc := g.Enterprise.Quad[1]

   colHdr := fmt.Sprintf("      %-4s %-4s %-4s", radarColHeader(qc-1), radarColHeader(qc), radarColHeader(qc+1))
   ```
   For each row `dr := -1; dr <= 1`:
   Display row number `radarRowHeader(qr+dr)` followed by 3 cells.
   Surround current quad with `<...>` or `styles.Enterprise`.

- [ ] **Step 4: Run test to verify it passes**

Run: `go test -v -run TestStatusPanel_LocationReadoutAndRadarHeaders ./pkg/tui/components/statuspanel`
Expected: PASS

- [ ] **Step 5: Run existing status panel tests to ensure zero regressions**

Run: `go test -v ./pkg/tui/components/statuspanel/...`
Expected: PASS

- [ ] **Step 6: Commit changes**

```bash
git add pkg/tui/components/statuspanel/
git commit -m "feat(statuspanel): add current location readout and radar coordinate headers"
```

---

### Task 2: Galactic Chart Math & Telemetry Logic

**Files:**
- Create: `pkg/tui/components/galacticchart/math.go`
- Test: `pkg/tui/components/galacticchart/math_test.go`

**Interfaces:**
- Produces: `Telemetry` struct and `CalculateTelemetry(from, to engine.Coord) Telemetry` function for use by the Galactic Chart modal.

- [ ] **Step 1: Write failing tests for distance and telemetry calculations**

In `pkg/tui/components/galacticchart/math_test.go`:
```go
package galacticchart

import (
	"math"
	"testing"

	"github.com/scottdensmore/super-star-trek/pkg/engine"
)

func TestCalculateTelemetry_CardinalDirections(t *testing.T) {
	from := engine.Coord{4, 4}

	tests := []struct {
		name       string
		to         engine.Coord
		wantDist   float64
		wantCourse float64
		wantName   string
		wantWarp   float64
	}{
		{"Same Quadrant", engine.Coord{4, 4}, 0.0, 0.0, "Current", 0.0},
		{"Due North", engine.Coord{2, 4}, 2.0, math.Pi / 2, "North", 2.0},
		{"Due East", engine.Coord{4, 7}, 3.0, 0.0, "East", 3.0},
		{"Due South", engine.Coord{7, 4}, 3.0, 3 * math.Pi / 2, "South", 3.0},
		{"Due West", engine.Coord{4, 1}, 3.0, math.Pi, "West", 3.0},
		{"North-East", engine.Coord{2, 6}, math.Sqrt(8), math.Pi / 4, "North-East", 2.8},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			res := CalculateTelemetry(from, tc.to)
			if math.Abs(res.Distance-tc.wantDist) > 0.05 {
				t.Errorf("Distance = %v, want %v", res.Distance, tc.wantDist)
			}
			if tc.to != from && math.Abs(res.Course-tc.wantCourse) > 0.05 {
				t.Errorf("Course = %v, want %v", res.Course, tc.wantCourse)
			}
			if res.Direction != tc.wantName {
				t.Errorf("Direction = %q, want %q", res.Direction, tc.wantName)
			}
			if math.Abs(res.RecommendedWarp-tc.wantWarp) > 0.05 {
				t.Errorf("RecommendedWarp = %v, want %v", res.RecommendedWarp, tc.wantWarp)
			}
		})
	}
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `go test -v ./pkg/tui/components/galacticchart`
Expected: FAIL (package does not exist).

- [ ] **Step 3: Implement `CalculateTelemetry` in `math.go`**

In `pkg/tui/components/galacticchart/math.go`:
```go
package galacticchart

import (
	"math"

	"github.com/scottdensmore/super-star-trek/pkg/engine"
)

type Telemetry struct {
	From            engine.Coord
	To              engine.Coord
	DeltaR          int
	DeltaC          int
	Distance        float64
	Course          float64
	Direction       string
	RecommendedWarp float64
	IsCurrent       bool
}

func CalculateTelemetry(from, to engine.Coord) Telemetry {
	if from == to {
		return Telemetry{
			From:            from,
			To:              to,
			Direction:       "Current",
			IsCurrent:       true,
			RecommendedWarp: 0.0,
		}
	}

	dr := to[0] - from[0]
	dc := to[1] - from[1]
	dist := math.Sqrt(float64(dr*dr + dc*dc))

	// In Super Star Trek geometry, dr is negative North, positive South:
	// dr = -sin(theta), dc = cos(theta) => theta = atan2(-dr, dc)
	angle := math.Atan2(float64(-dr), float64(dc))
	if angle < 0 {
		angle += 2 * math.Pi
	}

	dir := bearingDirection(angle)
	warp := math.Round(dist*10) / 10
	if warp < 1.0 {
		warp = 1.0
	}

	return Telemetry{
		From:            from,
		To:              to,
		DeltaR:          dr,
		DeltaC:          dc,
		Distance:        dist,
		Course:          angle,
		Direction:       dir,
		RecommendedWarp: warp,
		IsCurrent:       false,
	}
}

func bearingDirection(angle float64) string {
	deg := angle * 180.0 / math.Pi
	switch {
	case deg >= 337.5 || deg < 22.5:
		return "East"
	case deg >= 22.5 && deg < 67.5:
		return "North-East"
	case deg >= 67.5 && deg < 112.5:
		return "North"
	case deg >= 112.5 && deg < 157.5:
		return "North-West"
	case deg >= 157.5 && deg < 202.5:
		return "West"
	case deg >= 202.5 && deg < 247.5:
		return "South-West"
	case deg >= 247.5 && deg < 292.5:
		return "South"
	default:
		return "South-East"
	}
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `go test -v ./pkg/tui/components/galacticchart`
Expected: PASS

- [ ] **Step 5: Commit changes**

```bash
git add pkg/tui/components/galacticchart/math.go pkg/tui/components/galacticchart/math_test.go
git commit -m "feat(galacticchart): implement quadrant distance, bearing, and telemetry math"
```

---

### Task 3: Interactive Galactic Star Chart Modal Component

**Files:**
- Create: `pkg/tui/components/galacticchart/chart.go`
- Create: `pkg/tui/components/galacticchart/chart_test.go`

**Interfaces:**
- Produces: `galacticchart.Model`, `WarpToQuadrantMsg`, `CloseChartMsg`, `New`, `SetTheme`, `SetState`, `Update`, `View`

- [ ] **Step 1: Write failing unit tests for Galactic Chart modal**

In `pkg/tui/components/galacticchart/chart_test.go`:
```go
package galacticchart

import (
	"strings"
	"testing"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

func TestGalacticChart_CursorNavigationAndBounds(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th, 64, 18)
	var chart [9][9]int
	var discovered [9][9]bool
	m.SetState(engine.Coord{3, 3}, chart, discovered)

	if m.Cursor() != (engine.Coord{3, 3}) {
		t.Fatalf("expected cursor initialized to enterprise quad [3,3], got %v", m.Cursor())
	}

	// Move Up (k)
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'k'}})
	if m.Cursor() != (engine.Coord{2, 3}) {
		t.Fatalf("expected cursor [2,3] after Up, got %v", m.Cursor())
	}

	// Move Left (h)
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'h'}})
	if m.Cursor() != (engine.Coord{2, 2}) {
		t.Fatalf("expected cursor [2,2] after Left, got %v", m.Cursor())
	}

	// Move beyond row 1 (clamp)
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'k'}})
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'k'}})
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'k'}})
	if m.Cursor()[0] != 1 {
		t.Fatalf("expected cursor row clamped to 1, got %d", m.Cursor()[0])
	}
}

func TestGalacticChart_EnterAndEscMessages(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th, 64, 18)
	var chart [9][9]int
	var discovered [9][9]bool
	m.SetState(engine.Coord{3, 3}, chart, discovered)

	// Move to [2, 3]
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyUp})

	// Enter emits WarpToQuadrantMsg
	m, cmd := m.Update(tea.KeyMsg{Type: tea.KeyEnter})
	if cmd == nil {
		t.Fatalf("expected command on Enter, got nil")
	}
	msg := cmd()
	warpMsg, ok := msg.(WarpToQuadrantMsg)
	if !ok || warpMsg.DestQuad != (engine.Coord{2, 3}) {
		t.Fatalf("expected WarpToQuadrantMsg with [2,3], got %v", msg)
	}

	// Esc emits CloseChartMsg
	m, cmd = m.Update(tea.KeyMsg{Type: tea.KeyEsc})
	if cmd == nil {
		t.Fatalf("expected command on Esc, got nil")
	}
	if _, ok := cmd().(CloseChartMsg); !ok {
		t.Fatalf("expected CloseChartMsg on Esc, got %T", cmd())
	}
}

func TestGalacticChart_ViewDimensionsAndLayout(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th, 64, 18)
	var chart [9][9]int
	var discovered [9][9]bool
	chart[3][3] = 3
	discovered[3][3] = true
	chart[2][5] = 105
	discovered[2][5] = true
	m.SetState(engine.Coord{3, 3}, chart, discovered)

	view := m.View()
	if !strings.Contains(view, "GALACTIC STAR CHART") {
		t.Fatalf("expected title in View, got:\n%s", view)
	}
	if !strings.Contains(view, "105") {
		t.Fatalf("expected discovered quad 105 in View, got:\n%s", view)
	}

	lines := strings.Split(strings.TrimRight(view, "\n"), "\n")
	if len(lines) != 18 {
		t.Fatalf("expected height 18 rows, got %d", len(lines))
	}
	for i, line := range lines {
		w := lipgloss.Width(line)
		if w != 64 {
			t.Fatalf("line %d width %d != 64: %q", i, w, line)
		}
	}
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `go test -v ./pkg/tui/components/galacticchart`
Expected: FAIL (`New`, `SetState`, `WarpToQuadrantMsg`, etc. undefined).

- [ ] **Step 3: Implement `chart.go`**

In `pkg/tui/components/galacticchart/chart.go`:
Implement:
- `Model` struct holding state, styles, dimensions (64x18), cursor, enterpriseQuad, chart data.
- `WarpToQuadrantMsg{DestQuad engine.Coord}` and `CloseChartMsg{}`.
- `Update(msg tea.Msg) (Model, tea.Cmd)` handling:
  - Arrow keys, `h`, `j`, `k`, `l` (clamping cursor `[1..8, 1..8]`).
  - `KeyEnter` -> returns `WarpToQuadrantMsg{DestQuad: m.cursor}`.
  - `KeyEsc` -> returns `CloseChartMsg{}`.
  - Safely ignore `WindowSizeMsg` to preserve fixed 64x18 dialog size.
- `View() string`:
  - 8x8 grid rendering with column numbers `1..8` and row numbers `1..8`.
  - Discovered cells rendered as `fmt.Sprintf("%03d", val)`.
  - Undiscovered cells rendered as `···`.
  - Enterprise position marked with `<>` or `styles.Enterprise`.
  - Cursor position marked with brackets `[...]` or accent background.
  - Telemetry bar at bottom using `CalculateTelemetry(m.enterpriseQuad, m.cursor)`.
  - Bounded Lip Gloss container of exact dimensions 64x18.

- [ ] **Step 4: Run test to verify it passes**

Run: `go test -v ./pkg/tui/components/galacticchart`
Expected: PASS

- [ ] **Step 5: Commit changes**

```bash
git add pkg/tui/components/galacticchart/chart.go pkg/tui/components/galacticchart/chart_test.go
git commit -m "feat(galacticchart): implement interactive galactic star chart component"
```

---

### Task 4: Root TUI Integration, Event Routing & Palette Updates

**Files:**
- Modify: `pkg/tui/model.go`
- Modify: `pkg/tui/view.go`
- Modify: `pkg/tui/update.go`
- Modify: `pkg/tui/components/commandpalette/palette.go`
- Modify: `pkg/tui/components/commandpalette/palette_test.go`
- Test: `pkg/tui/model_test.go`

**Interfaces:**
- Consumes: `galacticchart.Model`, `galacticchart.WarpToQuadrantMsg`, `galacticchart.CloseChartMsg`
- Produces: Integrated `ModalGalacticChart` activated via `chart`, `Ctrl+M`, and palette.

- [ ] **Step 1: Write failing integration tests in `pkg/tui/model_test.go`**

In `pkg/tui/model_test.go`:
```go
func TestModel_GalacticChart_HotkeyAndCommand(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	m := NewModel(g, theme.DefaultTheme())

	// Test 1: 'chart' command activates ModalGalacticChart
	m, _ = m.UpdateModel(commandbar.CommandSubmittedMsg{Text: "chart"})
	if m.ActiveModal != ModalGalacticChart {
		t.Fatalf("expected ActiveModal=ModalGalacticChart after 'chart', got %v", m.ActiveModal)
	}

	// Test 2: Esc closes modal
	m, _ = m.UpdateModel(tea.KeyMsg{Type: tea.KeyEsc})
	if m.ActiveModal != ModalNone {
		t.Fatalf("expected ActiveModal=ModalNone after Esc, got %v", m.ActiveModal)
	}

	// Test 3: Ctrl+M hotkey activates ModalGalacticChart
	m, _ = m.UpdateModel(tea.KeyMsg{Type: tea.KeyCtrlM})
	if m.ActiveModal != ModalGalacticChart {
		t.Fatalf("expected ActiveModal=ModalGalacticChart after Ctrl+M, got %v", m.ActiveModal)
	}

	// Test 4: View composites modal over dashboard
	view := m.View()
	if !strings.Contains(view, "GALACTIC STAR CHART") {
		t.Fatalf("expected View to contain star chart overlay, got:\n%s", view)
	}
}

func TestModel_GalacticChart_WarpSelection(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	g.Enterprise.Quad = engine.Coord{3, 3}
	m := NewModel(g, theme.DefaultTheme())

	// Open chart
	m, _ = m.UpdateModel(commandbar.CommandSubmittedMsg{Text: "chart"})

	// Move cursor to [4, 5] and press Enter
	m, _ = m.UpdateModel(tea.KeyMsg{Type: tea.KeyDown}) // row 4
	m, _ = m.UpdateModel(tea.KeyMsg{Type: tea.KeyRight}) // col 4
	m, _ = m.UpdateModel(tea.KeyMsg{Type: tea.KeyRight}) // col 5

	m, _ = m.UpdateModel(tea.KeyMsg{Type: tea.KeyEnter})

	// Verify modal closed and Enterprise moved to [4, 5]
	if m.ActiveModal != ModalNone {
		t.Fatalf("expected modal closed, got %v", m.ActiveModal)
	}
	if m.Game.Enterprise.Quad != (engine.Coord{4, 5}) {
		t.Fatalf("expected Enterprise at quad [4,5], got %v", m.Game.Enterprise.Quad)
	}
}

func TestModel_HelpChartAndNavDistance(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	m := NewModel(g, theme.DefaultTheme())

	m, _ = m.UpdateModel(commandbar.CommandSubmittedMsg{Text: "help chart"})
	msgs := m.CommandBar.Messages()
	joined := strings.Join(msgs, "\n")
	if !strings.Contains(joined, "CHART:") || !strings.Contains(joined, "Ctrl+M") {
		t.Fatalf("expected help chart with Ctrl+M, got:\n%s", joined)
	}

	m, _ = m.UpdateModel(commandbar.CommandSubmittedMsg{Text: "help nav"})
	msgs = m.CommandBar.Messages()
	joined = strings.Join(msgs, "\n")
	if !strings.Contains(joined, "Ctrl+M") {
		t.Fatalf("expected help nav to mention Ctrl+M map tool, got:\n%s", joined)
	}
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `go test -v -run "TestModel_GalacticChart|TestModel_HelpChartAndNavDistance" ./pkg/tui`
Expected: FAIL (`ModalGalacticChart` undefined).

- [ ] **Step 3: Update `pkg/tui/model.go`**

Add `ModalGalacticChart` to `ModalType` enum, add `GalacticChart galacticchart.Model` to `Model`, and initialize in `NewModel`:
```go
ModalGalacticChart ModalType = 4
```
```go
GalacticChart: galacticchart.New(th, 64, 18),
```

- [ ] **Step 4: Update `pkg/tui/view.go`**

In `renderDashboard()`:
```go
case ModalGalacticChart:
    modalView = m.GalacticChart.View()
```

- [ ] **Step 5: Update `pkg/tui/update.go`**

1. When `m.ActiveModal == ModalGalacticChart`:
   Route keys to `m.GalacticChart.Update(msg)`.
2. Handle `galacticchart.CloseChartMsg`:
   ```go
   case galacticchart.CloseChartMsg:
       m.ActiveModal = ModalNone
       return m, m.CommandBar.Focus()
   ```
3. Handle `galacticchart.WarpToQuadrantMsg`:
   ```go
   case galacticchart.WarpToQuadrantMsg:
       m.ActiveModal = ModalNone
       return m.executeAction(engine.ActionMove{DestQuad: msg.DestQuad, Warp: 1.0})
   ```
4. Hotkey `Ctrl+M` (and `c`/`m` when input buffer is empty):
   ```go
   if msg.Type == tea.KeyCtrlM || (m.CommandBar.Value() == "" && (msg.String() == "c" || msg.String() == "m")) {
       m.GalacticChart.SetState(m.Game.Enterprise.Quad, m.Game.GalaxyChart, m.Game.ChartDiscovered)
       m.ActiveModal = ModalGalacticChart
       m.CommandBar.Blur()
       return m, nil
   }
   ```
5. `case "chart":`:
   ```go
   m.GalacticChart.SetState(m.Game.Enterprise.Quad, m.Game.GalaxyChart, m.Game.ChartDiscovered)
   m.ActiveModal = ModalGalacticChart
   m.CommandBar.Blur()
   return m, nil
   ```
6. Update `help nav`, `help chart`, `help`, and `applyTheme`.

- [ ] **Step 6: Update `palette.go` and `palette_test.go`**

Update `CHART` entry description in `palette.go` to `"Interactive 8x8 galactic star chart and warp planner (Ctrl+M)"`.

- [ ] **Step 7: Run tests to verify they pass**

Run: `go test -v -run "TestModel_GalacticChart|TestModel_HelpChartAndNavDistance" ./pkg/tui`
Run: `go test -v ./pkg/tui/components/commandpalette/...`
Expected: PASS

- [ ] **Step 8: Commit changes**

```bash
git add pkg/tui/model.go pkg/tui/view.go pkg/tui/update.go pkg/tui/components/commandpalette/ pkg/tui/model_test.go
git commit -m "feat(tui): integrate interactive galactic chart modal, Ctrl+M hotkey, and warp routing"
```

---

### Task 5: End-to-End Test Suite Hardening & Golden Snapshot Updates

**Files:**
- Modify: `tests/tui_golden_test.go`
- Modify: `tests/golden/tui/*.golden`

**Interfaces:**
- Produces: Updated golden snapshots reflecting status panel location/radar enhancements and new `modal_galactic_chart.golden`.

- [ ] **Step 1: Add `TestTUIGolden_ModalGalacticChart` to `tests/tui_golden_test.go`**

In `tests/tui_golden_test.go`:
```go
func TestTUIGolden_ModalGalacticChart(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	g.Enterprise.Quad = engine.Coord{3, 3}
	g.GalaxyChart[3][3] = 3
	g.ChartDiscovered[3][3] = true
	g.GalaxyChart[2][5] = 105
	g.ChartDiscovered[2][5] = true
	g.GalaxyChart[3][4] = 12
	g.ChartDiscovered[3][4] = true

	m := tui.NewModel(g, theme.ModernTheme{})
	updated, _ := m.Update(tea.WindowSizeMsg{Width: 80, Height: 24})
	m = updated.(tui.Model)

	m.GalacticChart.SetState(g.Enterprise.Quad, g.GalaxyChart, g.ChartDiscovered)
	m.ActiveModal = tui.ModalGalacticChart
	compareOrUpdate(t, "modal_galactic_chart", m.View())
}
```

- [ ] **Step 2: Update golden files with `-update` flag**

Run: `go test ./tests -update`
Expected: Golden files updated cleanly.

- [ ] **Step 3: Run full Go test suite with race detector**

Run: `go test -v -race ./...`
Expected: PASS with 0 failures and 0 race conditions.

- [ ] **Step 4: Run C test suite gate**

Run: `ctest --preset debug`
Expected: 100% tests passed.

- [ ] **Step 5: Commit changes**

```bash
git add tests/ tests/golden/
git commit -m "test(tui): add galactic chart golden snapshot and update dashboard goldens"
```

---

## Self-Review Checklist

- [x] **Spec Coverage:**
  - Status panel location readout & radar headers covered in Task 1.
  - Quadrant distance, course angle, and telemetry math covered in Task 2.
  - 8x8 interactive galactic chart modal covered in Task 3.
  - Root model event routing, `Ctrl+M` hotkey, and `help chart` covered in Task 4.
  - Golden snapshots and CI gates covered in Task 5.
- [x] **No Placeholders:** All tasks include concrete files, code snippets, and commands.
- [x] **Type Consistency:** `WarpToQuadrantMsg`, `CloseChartMsg`, `ModalGalacticChart`, and `CalculateTelemetry` signatures match across tasks.
