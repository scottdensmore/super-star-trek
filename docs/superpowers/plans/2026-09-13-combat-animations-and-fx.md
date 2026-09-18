# Combat Visual FX & Animation Engine Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement an interactive, non-blocking combat visual FX and animation engine in Charm Bubble Tea with torpedo flight trajectories, directional phaser ray beams, explosion shockwaves, instant key-skip, and configurable speed controls.

**Architecture:** Create a decoupled visual animation package `pkg/tui/anim` containing frame generators and vector raycasting math. Hook animation cell overrides into `pkg/tui/components/sectorgrid` without mutating underlying `engine.GameState`. Drive animations via asynchronous `tea.Tick` in root `pkg/tui`, with instant key-skip on any player input and user-configurable animation speed in `optionsmodal`.

**Tech Stack:** Go 1.26+, Charmbracelet Bubble Tea (`tea.Model`, `tea.Cmd`, `tea.Tick`), Lip Gloss, standard library `math` and `time`.

**Spec:** `docs/superpowers/specs/2026-09-13-combat-animations-and-fx-design.md`

## Global Constraints

- Target Go version: Go 1.26+ standard library and Charmbracelet packages (`bubbletea`, `lipgloss`)
- Terminal layout constraint: Strict 80 columns x 24 lines dimension budget for full TUI view
- Sector grid layout constraint: Exact 33 columns wide x 19 rows high dimension budget (3-character cell glyphs `[···]` plus padding)
- Zero compiled binaries or temporary files committed to git
- Maintain 100% passing tests for Go (`go test -v -race ./...`) and C (`ctest --preset debug`, `tests/golden.sh`)
- Work committed on feature branch `scottdensmore/feat/combat-animations-and-fx`

---

### Task 1: Animation Data Structures, Bresenham Vector Math & Frame Generators (pkg/tui/anim)

**Files:**
- Create: `pkg/tui/anim/anim.go`
- Create: `pkg/tui/anim/line.go`
- Create: `pkg/tui/anim/torpedo.go`
- Create: `pkg/tui/anim/phaser.go`
- Create: `pkg/tui/anim/pulse.go`
- Test: `pkg/tui/anim/anim_test.go`

**Interfaces:**
- Produces:
  - `type CellOverride struct { Glyph string; Style lipgloss.Style }`
  - `type Frame struct { Overrides map[engine.Coord]CellOverride; Duration time.Duration }`
  - `type Animation interface { TotalDuration() time.Duration; Frames() []Frame; IsFinished() bool; Step() Frame; Skip() Frame }`
  - `type TickMsg struct { AnimID int; Step int }`
  - `func TickCmd(animID, step int, d time.Duration) tea.Cmd`
  - `func BresenhamLine(start, end engine.Coord) []engine.Coord`
  - `func NewTorpedoAnimation(start, end engine.Coord, hit bool, speed int) Animation`
  - `func NewPhaserAnimation(start, target engine.Coord, hit bool, speed int) Animation`
  - `func RedAlertBadgeStyle(cycle int) lipgloss.Style`

- [ ] **Step 1: Write the failing tests**

In `pkg/tui/anim/anim_test.go`:
```go
package anim

import (
	"testing"
	"time"

	"github.com/charmbracelet/lipgloss"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
)

func TestBresenhamLine(t *testing.T) {
	// Cardinal horizontal East
	pEast := BresenhamLine(engine.Coord{Row: 2, Col: 2}, engine.Coord{Row: 2, Col: 5})
	if len(pEast) != 4 {
		t.Fatalf("expected 4 points for horizontal line, got %d", len(pEast))
	}
	if pEast[0] != (engine.Coord{Row: 2, Col: 2}) || pEast[3] != (engine.Coord{Row: 2, Col: 5}) {
		t.Errorf("line endpoints mismatch: %v", pEast)
	}

	// Diagonal South-East
	pDiag := BresenhamLine(engine.Coord{Row: 1, Col: 1}, engine.Coord{Row: 4, Col: 4})
	if len(pDiag) != 4 {
		t.Fatalf("expected 4 points for diagonal line, got %d", len(pDiag))
	}
	for i, pt := range pDiag {
		if pt.Row != i+1 || pt.Col != i+1 {
			t.Errorf("unexpected point %d in diagonal line: %v", i, pt)
		}
	}
}

func TestTorpedoAnimation_StepAndSkip(t *testing.T) {
	start := engine.Coord{Row: 1, Col: 1}
	target := engine.Coord{Row: 1, Col: 4}
	anim := NewTorpedoAnimation(start, target, true, 2) // Normal speed

	if anim.IsFinished() {
		t.Fatalf("expected animation to start unfinished")
	}

	// Step 1: In-flight trajectory
	f1 := anim.Step()
	if len(f1.Overrides) == 0 {
		t.Errorf("expected overrides in step 1")
	}

	// Skip to terminal
	finalFrame := anim.Skip()
	if !anim.IsFinished() {
		t.Errorf("expected animation to be finished after Skip()")
	}
	_ = finalFrame
}

func TestPhaserAnimation_DirectionalGlyphs(t *testing.T) {
	start := engine.Coord{Row: 3, Col: 1}
	target := engine.Coord{Row: 3, Col: 5}
	anim := NewPhaserAnimation(start, target, true, 2)

	f := anim.Step()
	intermediate := engine.Coord{Row: 3, Col: 3}
	ov, ok := f.Overrides[intermediate]
	if !ok {
		t.Fatalf("expected override along beam path at %v", intermediate)
	}
	if ov.Glyph != "---" {
		t.Errorf("expected horizontal beam '---', got %q", ov.Glyph)
	}
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `go test -v ./pkg/tui/anim`
Expected: FAIL (package does not exist)

- [ ] **Step 3: Implement minimal code**

1. Create `pkg/tui/anim/anim.go` with core structs, interfaces, and `TickCmd`:
```go
package anim

import (
	"time"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
)

type CellOverride struct {
	Glyph string
	Style lipgloss.Style
}

type Frame struct {
	Overrides map[engine.Coord]CellOverride
	Duration  time.Duration
}

type Animation interface {
	TotalDuration() time.Duration
	Frames() []Frame
	IsFinished() bool
	Step() Frame
	Skip() Frame
}

type TickMsg struct {
	AnimID int
	Step   int
}

func TickCmd(animID, step int, d time.Duration) tea.Cmd {
	return tea.Tick(d, func(t time.Time) tea.Msg {
		return TickMsg{AnimID: animID, Step: step}
	})
}
```

2. Create `pkg/tui/anim/line.go` with `BresenhamLine`:
```go
package anim

import "github.com/scottdensmore/super-star-trek/pkg/engine"

func BresenhamLine(start, end engine.Coord) []engine.Coord {
	var points []engine.Coord
	x0, y0 := start.Col, start.Row
	x1, y1 := end.Col, end.Row

	dx := abs(x1 - x0)
	dy := -abs(y1 - y0)
	sx := 1
	if x0 >= x1 {
		sx = -1
	}
	sy := 1
	if y0 >= y1 {
		sy = -1
	}
	err := dx + dy

	for {
		points = append(points, engine.Coord{Row: y0, Col: x0})
		if x0 == x1 && y0 == y1 {
			break
		}
		e2 := 2 * err
		if e2 >= dy {
			err += dy
			x0 += sx
		}
		if e2 <= dx {
			err += dx
			y0 += sy
		}
	}
	return points
}

func abs(v int) int {
	if v < 0 {
		return -v
	}
	return v
}
```

3. Create `pkg/tui/anim/torpedo.go` with `TorpedoAnimation`:
- Stepping trajectory points along `BresenhamLine` with glyphs ` · `, ` o `, ` O `.
- Impact bursts on destination coordinate: ` * `, `***`, `#*#`.
- `Skip()` returns empty overrides and marks finished.

4. Create `pkg/tui/anim/phaser.go` with `PhaserAnimation`:
- Calculates angle/direction between `start` and `target` to choose `---`, ` | `, ` \ `, ` / `.
- Highlights target cell with shield brackets `(E)` or `<K>`.

5. Create `pkg/tui/anim/pulse.go` with `RedAlertBadgeStyle(cycle int) lipgloss.Style`.

- [ ] **Step 4: Run test to verify it passes**

Run: `go test -v ./pkg/tui/anim`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add pkg/tui/anim/
git commit -m "feat(anim): create combat visual animation engine, vector raycasting, and frame generators"
```

---

### Task 2: Sector Grid Animation Overlay Hook & Cell Override Rendering (pkg/tui/components/sectorgrid)

**Files:**
- Modify: `pkg/tui/components/sectorgrid/grid.go`
- Modify: `pkg/tui/components/sectorgrid/grid_test.go`

**Interfaces:**
- Consumes:
  - `anim.CellOverride`
- Produces:
  - `func (m *Model) SetAnimOverrides(overrides map[engine.Coord]anim.CellOverride)`
  - `func (m *Model) ClearAnimOverrides()`

- [ ] **Step 1: Write the failing test**

In `pkg/tui/components/sectorgrid/grid_test.go`:
```go
func TestSectorGrid_AnimOverrides(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th)
	q := &engine.Quadrant{
		Enterprise: engine.Coord{Row: 1, Col: 1},
	}

	// 1. Initial view has no overrides
	viewInit := m.View(q)
	if strings.Contains(viewInit, "***") {
		t.Fatalf("unexpected explosion glyph in initial grid")
	}

	// 2. Set animation override at (2, 2)
	overrides := map[engine.Coord]anim.CellOverride{
		{Row: 2, Col: 2}: {
			Glyph: "***",
			Style: lipgloss.NewStyle().Foreground(lipgloss.Color("9")),
		},
	}
	m.SetAnimOverrides(overrides)
	viewAnim := m.View(q)

	if !strings.Contains(viewAnim, "***") {
		t.Errorf("expected override glyph '***' in animated view")
	}

	// 3. Verify exact 33x19 dimensions preserved
	lines := strings.Split(viewAnim, "\n")
	if len(lines) != 19 {
		t.Errorf("expected 19 lines, got %d", len(lines))
	}
	for i, l := range lines {
		if w := ansi.StringWidth(l); w != 33 {
			t.Errorf("line %d width = %d, expected 33", i, w)
		}
	}

	// 4. Clear overrides restores grid
	m.ClearAnimOverrides()
	viewClean := m.View(q)
	if strings.Contains(viewClean, "***") {
		t.Errorf("expected override removed after ClearAnimOverrides")
	}
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `go test -v ./pkg/tui/components/sectorgrid -run TestSectorGrid_AnimOverrides`
Expected: FAIL (`SetAnimOverrides` undefined)

- [ ] **Step 3: Implement minimal code**

In `pkg/tui/components/sectorgrid/grid.go`:
- Add field `animOverrides map[engine.Coord]anim.CellOverride` to `Model`.
- Implement `SetAnimOverrides(overrides map[engine.Coord]anim.CellOverride)`:
  ```go
  func (m *Model) SetAnimOverrides(overrides map[engine.Coord]anim.CellOverride) {
      m.animOverrides = overrides
  }
  func (m *Model) ClearAnimOverrides() {
      m.animOverrides = nil
  }
  ```
- In `renderRow(r int, q *engine.Quadrant) string`:
  - Before rendering entity or empty cell at `col := c`:
    ```go
    coord := engine.Coord{Row: r, Col: c}
    if ov, ok := m.animOverrides[coord]; ok {
        glyph := ov.Glyph
        if len(glyph) != 3 {
            glyph = padCell(glyph)
        }
        rowStr += ov.Style.Render(glyph) + " "
        continue
    }
    ```

- [ ] **Step 4: Run test to verify it passes**

Run: `go test -v ./pkg/tui/components/sectorgrid`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add pkg/tui/components/sectorgrid/grid.go pkg/tui/components/sectorgrid/grid_test.go
git commit -m "feat(sectorgrid): implement cell animation overrides and layout budget preservation"
```

---

### Task 3: Root TUI Animation Event Loop, Key-Skip & Options Modal Integration (pkg/tui, pkg/tui/components/optionsmodal)

**Files:**
- Modify: `pkg/engine/rules.go`
- Modify: `pkg/tui/components/optionsmodal/optionsmodal.go`
- Modify: `pkg/tui/model.go`
- Modify: `pkg/tui/update.go`
- Test: `pkg/tui/model_test.go`

**Interfaces:**
- Consumes:
  - `anim.Animation`, `anim.TickMsg`, `anim.TickCmd`
  - `m.Grid.SetAnimOverrides`
- Produces:
  - `Rules.AnimSpeed int`
  - `m.startCombatAnimation(anim anim.Animation) tea.Cmd`
  - Keyboard skip on any `tea.KeyMsg`

- [ ] **Step 1: Write the failing test**

In `pkg/tui/model_test.go`:
```go
func TestModel_CombatAnimation_KeySkip(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	mod := NewModel(g, theme.DefaultTheme())

	// Fire torpedo initiates animation
	updated, cmd := mod.handleCommand("tor 1 0")
	m := updated.(Model)
	if m.activeAnim == nil {
		t.Fatalf("expected activeAnim to be populated on torpedo fire")
	}
	if cmd == nil {
		t.Fatalf("expected TickCmd on active animation")
	}

	// Pressing any key immediately skips animation to completion
	keyMsg := tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune(" ")}
	updatedAfterKey, _ := m.Update(keyMsg)
	mSkipped := updatedAfterKey.(Model)

	if mSkipped.activeAnim != nil {
		t.Errorf("expected activeAnim cleared after keypress")
	}
}

func TestModel_CombatAnimation_SpeedOption(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	mod := NewModel(g, theme.DefaultTheme())

	// Disable animation via command: anim off
	updated, _ := mod.handleCommand("anim off")
	m := updated.(Model)
	if m.Game.Rules.AnimSpeed != 0 {
		t.Fatalf("expected AnimSpeed 0 on 'anim off', got %d", m.Game.Rules.AnimSpeed)
	}

	// Torpedo when speed is off does not start activeAnim
	updatedNoAnim, _ := m.handleCommand("tor 1 0")
	mNoAnim := updatedNoAnim.(Model)
	if mNoAnim.activeAnim != nil {
		t.Errorf("expected activeAnim to remain nil when AnimSpeed is off")
	}
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `go test -v ./pkg/tui -run TestModel_CombatAnimation_`
Expected: FAIL (`activeAnim` undefined)

- [ ] **Step 3: Implement minimal code**

1. In `pkg/engine/rules.go`:
   - Add `AnimSpeed int` to `Rules` (default `2` = Normal).
2. In `pkg/tui/model.go`:
   - Add `activeAnim anim.Animation` and `animID int` to `Model`.
3. In `pkg/tui/update.go`:
   - Implement `m.startCombatAnimation(anim anim.Animation) (Model, tea.Cmd)`.
   - In `ActionFireTorpedo` and `ActionFirePhaser` handlers: construct and start animation if `m.Game.Rules.AnimSpeed > 0`.
   - In `Update(msg)`:
     - Handle `anim.TickMsg`: advance frame with `m.activeAnim.Step()`, set grid overrides, schedule next tick if not finished.
     - In `tea.KeyMsg`: if `m.activeAnim != nil`, call `m.activeAnim.Skip()`, set `m.activeAnim = nil`, clear grid overrides, and continue processing key.
   - In `handleCommand`: support `"anim off"`, `"anim fast"`, `"anim normal"`, `"anim cinematic"`.
4. In `pkg/tui/components/optionsmodal/optionsmodal.go`:
   - Add Combat Animation row and toggle keys.

- [ ] **Step 4: Run test to verify it passes**

Run: `go test -v ./pkg/tui -run TestModel_CombatAnimation_`
Expected: PASS

Run all TUI tests:
`go test -v ./pkg/tui/...`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add pkg/engine/rules.go pkg/tui/components/optionsmodal/optionsmodal.go pkg/tui/model.go pkg/tui/update.go pkg/tui/model_test.go
git commit -m "feat(tui): integrate combat animation loop, options speed toggle, and instant key-skip"
```

---

### Task 4: Golden Snapshot Visual Regression & Full Verification (pkg/tui, test suites)

**Files:**
- Modify: `tests/tui_golden_test.go`
- Create: `tests/golden/tui/anim_torpedo_flight.golden`
- Create: `tests/golden/tui/anim_phaser_beam.golden`

**Interfaces:**
- Validates full 80×24 layout integrity during active projectile and beam overlays.

- [ ] **Step 1: Write the failing test**

In `tests/tui_golden_test.go`:
```go
func TestTUIGolden_CombatAnimations(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	m := tui.NewModel(g, theme.ModernTheme{})
	updated, _ := m.Update(tea.WindowSizeMsg{Width: 80, Height: 24})
	m = updated.(tui.Model)

	// Inject torpedo flight frame override at (3, 3)
	torpedoOverrides := map[engine.Coord]anim.CellOverride{
		{Row: 3, Col: 3}: {
			Glyph: " O ",
			Style: lipgloss.NewStyle().Foreground(lipgloss.Color("9")),
		},
	}
	m.Grid.SetAnimOverrides(torpedoOverrides)
	compareOrUpdate(t, "anim_torpedo_flight", m.View())

	// Inject phaser beam line override between (1, 1) and (1, 4)
	phaserOverrides := map[engine.Coord]anim.CellOverride{
		{Row: 1, Col: 2}: {Glyph: "---", Style: lipgloss.NewStyle().Foreground(lipgloss.Color("11"))},
		{Row: 1, Col: 3}: {Glyph: "---", Style: lipgloss.NewStyle().Foreground(lipgloss.Color("11"))},
	}
	m.Grid.SetAnimOverrides(phaserOverrides)
	compareOrUpdate(t, "anim_phaser_beam", m.View())
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `go test -v ./tests -run TestTUIGolden_CombatAnimations`
Expected: FAIL (golden snapshots missing)

- [ ] **Step 3: Generate snapshots and verify**

Run: `go test -v ./tests -run TestTUIGolden_CombatAnimations -update`
Expected: PASS

Run without `-update`:
`go test -v ./tests -run TestTUIGolden_CombatAnimations`
Expected: PASS

- [ ] **Step 4: Run full verification suite**

```bash
go test -v -race ./...
ctest --preset debug
bash tests/golden.sh ./build/debug/sst
```
Expected: All suites pass 100%.

- [ ] **Step 5: Commit**

```bash
git add tests/tui_golden_test.go tests/golden/tui/anim_*.golden
git commit -m "test(tui): add golden snapshot tests for combat animation overlays"
```
