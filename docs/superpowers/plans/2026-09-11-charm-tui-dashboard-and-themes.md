# Charmbracelet TUI Dashboard & Themes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the interactive, declarative Charmbracelet TUI (`pkg/tui`) for Super Star Trek featuring a Split Dashboard layout (8x8 sector grid, telemetry & status meters, command bar) and dynamic TrueColor theme switching (Starfleet Modern, LCARS, CRT).

**Architecture:** Built on Charmbracelet's Elm-inspired architecture (`tea.Model` -> `Update` -> `View`). The root TUI model coordinates three focused sub-components: `sectorgrid`, `statuspanel`, and `commandbar`. Player commands submitted through the command bar are parsed into strongly typed `engine.Action` calls dispatched to `engine.GameState`. Views are styled using Lip Gloss palettes defined in `pkg/tui/theme`.

**Tech Stack:** Go 1.26+, `github.com/charmbracelet/bubbletea`, `github.com/charmbracelet/lipgloss`, `github.com/charmbracelet/bubbles`.

**Spec:** [`docs/superpowers/specs/2026-09-11-charm-tui-dashboard-and-themes-design.md`](docs/superpowers/specs/2026-09-11-charm-tui-dashboard-and-themes-design.md)

## Global Constraints
- Target Go version: Go 1.26+ with standard library and Charmbracelet packages (`bubbletea`, `lipgloss`, `bubbles`).
- Zero changes or regressions to `pkg/engine` combat formulas, coordinates, or stardate math.
- Classic teletype mode (`--classic`) must remain fully functional with 100% test pass on `tests/golden_test.go`.
- Terminal dimensions < 80x24 must display the resize notice cleanly without panic or clipping.
- Every task must pass `go test -v -race ./...` with zero failures and zero race conditions.
- Preserves all C CI gates (`cmake --preset ci-debug` and `cmake --preset ci-release`).
- Work committed on feature branch `scottdensmore/feat/charm-tui`.

---

### Task 1: Module Dependencies & Theme Engine

**Files:**
- Modify: `go.mod`
- Create: `pkg/tui/theme/theme.go`
- Create: `pkg/tui/theme/modern.go`
- Create: `pkg/tui/theme/lcars.go`
- Create: `pkg/tui/theme/crt.go`
- Create: `pkg/tui/theme/theme_test.go`

**Interfaces:**
- Produces:
  - `type Theme interface { Name() string; Styles() Styles; Next() Theme }`
  - `type Styles struct { ... }`
  - `func DefaultTheme() Theme`
  - `func GetTheme(name string) Theme`

- [ ] **Step 1: Add Charmbracelet dependencies to go.mod**

```bash
go get github.com/charmbracelet/bubbletea@v1.3.4
go get github.com/charmbracelet/lipgloss@v1.0.0
go get github.com/charmbracelet/bubbles@v0.20.0
```

- [ ] **Step 2: Write failing theme unit tests**

```go
// pkg/tui/theme/theme_test.go
package theme

import "testing"

func TestThemeCyclingAndLookup(t *testing.T) {
	def := DefaultTheme()
	if def.Name() != "modern" {
		t.Fatalf("expected modern as default, got %s", def.Name())
	}

	next := def.Next()
	if next.Name() != "lcars" {
		t.Fatalf("expected lcars next, got %s", next.Name())
	}

	third := next.Next()
	if third.Name() != "crt" {
		t.Fatalf("expected crt next, got %s", third.Name())
	}

	backToDef := third.Next()
	if backToDef.Name() != "modern" {
		t.Fatalf("expected cycle back to modern, got %s", backToDef.Name())
	}

	if GetTheme("lcars").Name() != "lcars" {
		t.Fatalf("failed to retrieve lcars by name")
	}
	if GetTheme("unknown").Name() != "modern" {
		t.Fatalf("unknown theme should fallback to modern")
	}
}

func TestStylesNonNull(t *testing.T) {
	themes := []Theme{ModernTheme{}, LcarsTheme{}, CrtTheme{}}
	for _, th := range themes {
		s := th.Styles()
		if s.Title.Render("test") == "" {
			t.Errorf("theme %s failed to render title", th.Name())
		}
	}
}
```

- [ ] **Step 3: Run test to verify failure**

Run: `go test -v ./pkg/tui/theme`
Expected: FAIL (packages and types not defined)

- [ ] **Step 4: Implement Theme interface and palettes**

Create `pkg/tui/theme/theme.go`, `modern.go`, `lcars.go`, and `crt.go` with Lip Gloss styles for panels, grids, progress bars, condition badges, and title banners.

- [ ] **Step 5: Verify tests pass and commit**

Run: `go test -v -race ./pkg/tui/theme`
Commit: `git add go.mod go.sum pkg/tui/theme/ && git commit -m "feat(tui): add theme engine with Modern, LCARS, and CRT palettes"`

---

### Task 2: 8x8 Sector Grid Component

**Files:**
- Create: `pkg/tui/components/sectorgrid/grid.go`
- Create: `pkg/tui/components/sectorgrid/grid_test.go`

**Interfaces:**
- Consumes: `engine.QuadrantState`, `engine.Coord`, `engine.EntityType`, `theme.Theme`
- Produces:
  - `type Model struct { ... }`
  - `func New(th theme.Theme) Model`
  - `func (m Model) View(quad *engine.QuadrantState, entSector engine.Coord) string`
  - `func (m *Model) SetTheme(th theme.Theme)`

- [ ] **Step 1: Write failing grid tests**

```go
// pkg/tui/components/sectorgrid/grid_test.go
package sectorgrid

import (
	"strings"
	"testing"

	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

func TestGridRendering(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th)

	var quad engine.QuadrantState
	quad.Grid[2][3] = engine.EntityEnterprise
	quad.Grid[4][5] = engine.EntityKlingon
	quad.Grid[1][1] = engine.EntityStarbase
	quad.Grid[8][8] = engine.EntityStar

	view := m.View(&quad, engine.Coord{2, 3})

	// Check header contains columns 1..8
	if !strings.Contains(view, "1") || !strings.Contains(view, "8") {
		t.Fatalf("grid view missing column headers:\n%s", view)
	}

	// Check entity glyphs
	if !strings.Contains(view, "<E>") {
		t.Fatalf("grid view missing Enterprise glyph <E>:\n%s", view)
	}
	if !strings.Contains(view, "+K+") {
		t.Fatalf("grid view missing Klingon glyph +K+:\n%s", view)
	}
	if !strings.Contains(view, ">B<") {
		t.Fatalf("grid view missing Starbase glyph >B<:\n%s", view)
	}
	if !strings.Contains(view, " * ") {
		t.Fatalf("grid view missing Star glyph *:\n%s", view)
	}
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `go test -v ./pkg/tui/components/sectorgrid`
Expected: FAIL

- [ ] **Step 3: Implement Sector Grid Component**

Implement `pkg/tui/components/sectorgrid/grid.go` formatting the 8x8 sector layout with bordered frame, axes numbers, and styled glyphs.

- [ ] **Step 4: Verify tests pass and commit**

Run: `go test -v -race ./pkg/tui/components/sectorgrid`
Commit: `git add pkg/tui/components/sectorgrid/ && git commit -m "feat(tui): implement 8x8 sector grid visualizer component"`

---

### Task 3: Telemetry & Status Panel Component

**Files:**
- Create: `pkg/tui/components/statuspanel/status.go`
- Create: `pkg/tui/components/statuspanel/status_test.go`

**Interfaces:**
- Consumes: `engine.Enterprise`, `engine.GameState`, `theme.Theme`
- Produces:
  - `type Model struct { ... }`
  - `func New(th theme.Theme) Model`
  - `func (m Model) View(g *engine.GameState) string`
  - `func (m *Model) SetTheme(th theme.Theme)`

- [ ] **Step 1: Write failing status panel tests**

```go
// pkg/tui/components/statuspanel/status_test.go
package statuspanel

import (
	"strings"
	"testing"

	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

func TestStatusPanelRendering(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th)

	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	g.Enterprise.Energy = 4500
	g.Enterprise.Shields = 1500
	g.Enterprise.Condition = engine.ConditionYellow

	view := m.View(g)

	if !strings.Contains(view, "YELLOW") {
		t.Fatalf("status view missing condition alert:\n%s", view)
	}
	if !strings.Contains(view, "4500") {
		t.Fatalf("status view missing energy readout:\n%s", view)
	}
	if !strings.Contains(view, "1500") {
		t.Fatalf("status view missing shields readout:\n%s", view)
	}
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `go test -v ./pkg/tui/components/statuspanel`
Expected: FAIL

- [ ] **Step 3: Implement Status Panel Component**

Implement `pkg/tui/components/statuspanel/status.go` rendering condition banner, energy/shield progress bars, torpedo counts, device status, and mini 3x3 surrounding quadrant radar box.

- [ ] **Step 4: Verify tests pass and commit**

Run: `go test -v -race ./pkg/tui/components/statuspanel`
Commit: `git add pkg/tui/components/statuspanel/ && git commit -m "feat(tui): implement telemetry and status panel component"`

---

### Task 4: Command Bar Component & Input Parser

**Files:**
- Create: `pkg/tui/components/commandbar/bar.go`
- Create: `pkg/tui/components/commandbar/bar_test.go`
- Create: `pkg/tui/parser.go`
- Create: `pkg/tui/parser_test.go`

**Interfaces:**
- Produces:
  - `type ParsedCommand struct { Action engine.Action; Special string; Error error }`
  - `func ParseCommand(input string) ParsedCommand`
  - `type CommandBarModel struct { ... }`
  - `func NewCommandBar(th theme.Theme) CommandBarModel`

- [ ] **Step 1: Write failing parser and command bar tests**

```go
// pkg/tui/parser_test.go
package tui

import (
	"testing"

	"github.com/scottdensmore/super-star-trek/pkg/engine"
)

func TestParseCommands(t *testing.T) {
	// NAV course warp
	res := ParseCommand("nav 1.5 2")
	if res.Error != nil {
		t.Fatalf("unexpected error for nav: %v", res.Error)
	}
	mv, ok := res.Action.(engine.ActionMove)
	if !ok || mv.Course != 1.5 || mv.Warp != 2.0 {
		t.Fatalf("mismatch in parsed nav action: %+v", res.Action)
	}

	// PHA energy
	res = ParseCommand("pha 500")
	if res.Error != nil {
		t.Fatalf("unexpected error for pha: %v", res.Error)
	}
	pha, ok := res.Action.(engine.ActionFirePhasers)
	if !ok || pha.Energy != 500 {
		t.Fatalf("mismatch in parsed pha action: %+v", res.Action)
	}

	// Special commands: theme, help, quit
	if ParseCommand("theme lcars").Special != "theme lcars" {
		t.Fatalf("expected theme special command")
	}
	if ParseCommand("quit").Special != "quit" {
		t.Fatalf("expected quit special command")
	}
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `go test -v ./pkg/tui`
Expected: FAIL

- [ ] **Step 3: Implement Parser and Command Bar**

Implement `pkg/tui/parser.go` and `pkg/tui/components/commandbar/bar.go` supporting text input buffer and 4-line event history log.

- [ ] **Step 4: Verify tests pass and commit**

Run: `go test -v -race ./pkg/tui/...`
Commit: `git add pkg/tui/parser.go pkg/tui/parser_test.go pkg/tui/components/commandbar/ && git commit -m "feat(tui): add command parser and interactive command bar component"`

---

### Task 5: Split Dashboard Composition & Model Lifecycle

**Files:**
- Create: `pkg/tui/model.go`
- Create: `pkg/tui/view.go`
- Create: `pkg/tui/update.go`
- Create: `pkg/tui/model_test.go`

**Interfaces:**
- Consumes: `engine.GameState`, `theme.Theme`, sub-components
- Produces:
  - `func NewModel(g *engine.GameState, th theme.Theme) Model`
  - `(m Model) Init() tea.Cmd`
  - `(m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd)`
  - `(m Model) View() string`

- [ ] **Step 1: Write failing Model lifecycle tests**

```go
// pkg/tui/model_test.go
package tui

import (
	"strings"
	"testing"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

func TestModelSizeGuard(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	m := NewModel(g, theme.DefaultTheme())

	// Small size triggers warning
	m, _ = m.Update(tea.WindowSizeMsg{Width: 70, Height: 20})
	view := m.View()
	if !strings.Contains(view, "TERMINAL WINDOW TOO SMALL") {
		t.Fatalf("expected size warning view, got:\n%s", view)
	}

	// Standard size renders dashboard
	m, _ = m.Update(tea.WindowSizeMsg{Width: 80, Height: 24})
	view = m.View()
	if strings.Contains(view, "TOO SMALL") || !strings.Contains(view, "<E>") {
		t.Fatalf("expected dashboard view, got:\n%s", view)
	}
}

func TestModelThemeToggle(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	m := NewModel(g, theme.DefaultTheme())

	if m.Theme.Name() != "modern" {
		t.Fatalf("expected initial modern theme")
	}

	// Press F2 to cycle theme
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyF2})
	if m.Theme.Name() != "lcars" {
		t.Fatalf("expected theme cycled to lcars, got: %s", m.Theme.Name())
	}
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `go test -v ./pkg/tui`
Expected: FAIL

- [ ] **Step 3: Implement Root Model, View, and Update**

Implement `pkg/tui/model.go`, `view.go`, and `update.go` coordinating the horizontal split (left: grid, right: status) and bottom command bar, enforcing minimum 80x24 bounds.

- [ ] **Step 4: Verify tests pass and commit**

Run: `go test -v -race ./pkg/tui/...`
Commit: `git add pkg/tui/ && git commit -m "feat(tui): assemble split dashboard root model and lifecycle handling"`

---

### Task 6: CLI Integration & End-to-End Verification

**Files:**
- Modify: `cmd/sst/main.go`
- Modify: `tests/golden_test.go` (if needed)

**Interfaces:**
- Produces:
  - `cmd/sst/main.go`: Default TUI execution, `--classic` execution, and `--theme` flag support.

- [ ] **Step 1: Update cmd/sst/main.go to launch Bubble Tea TUI**

Connect `cmd/sst/main.go` to launch `tea.NewProgram(tui.NewModel(game, selectedTheme), tea.WithAltScreen()).Run()` by default, keeping `--classic` flag calling `classic.RunClassicCLI`.

- [ ] **Step 2: Run full Go test suite**

Run: `go test -v -race ./...`
Expected: PASS across all packages.

- [ ] **Step 3: Run CI Gates**

Run: `cmake --preset ci-debug && cmake --build --preset ci-debug && ctest --preset ci-debug`
Run: `cmake --preset ci-release && cmake --build --preset ci-release && ctest --preset ci-release`
Expected: PASS.

- [ ] **Step 4: Commit**

Commit: `git add cmd/sst/main.go && git commit -m "feat(cli): integrate Charmbracelet TUI as default frontend with classic fallback"`
