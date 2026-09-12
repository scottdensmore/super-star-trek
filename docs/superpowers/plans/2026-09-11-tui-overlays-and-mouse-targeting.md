# Milestone 3: TUI Overlays & Mouse Targeting Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement floating tactical overlays (Target Lock HUD, Spock Command Palette modal) and Sector Grid mouse targeting in `pkg/tui`, giving Super Star Trek full hybrid keyboard/mouse control.

**Architecture:** Model-View-Update architecture using Charmbracelet Bubble Tea & Lip Gloss. State machine manages modal overlays (`ModalNone`, `ModalTargetLock`, `ModalCommandPalette`) composited over the split dashboard. Dedicated sub-components (`pkg/tui/components/targetlock`, `pkg/tui/components/commandpalette`) encapsulate targeting ballistics and fuzzy search lists (`bubbles/list`). Sector grid provides mouse hit-testing (`HitTest`) and reticle highlighting.

**Tech Stack:** Go 1.26+, Charmbracelet `bubbletea` v1.3.4, `lipgloss` v1.0.0, `bubbles` v0.20.0 (`bubbles/list`, `bubbles/textinput`).

**Spec:** [`docs/superpowers/specs/2026-09-11-tui-overlays-and-mouse-targeting-design.md`](file:///home/scottdensmore/Developer/scottdensmore/super-star-trek/docs/superpowers/specs/2026-09-11-tui-overlays-and-mouse-targeting-design.md)

## Global Constraints

- Target Go version: Go 1.26+ with standard library and Charmbracelet packages (`bubbletea`, `lipgloss`, `bubbles`).
- Zero changes or regressions to `pkg/engine` core game logic.
- Classic teletype mode (`--classic`) and golden tests (`tests/golden_test.go`) must remain 100% green.
- Minimum terminal dimensions guard: strictly enforce 80x24 characters with centered alert dialog.
- Coordinates are 1-indexed (`1..8`) for Quadrants and Sectors.
- Every task must pass `go test -v -race ./...` with zero failures and zero race conditions.
- Work committed on feature branch `scottdensmore/feat/tui-overlays-and-mouse`.

---

### Task 1: Sector Grid Hit-Testing & Reticle Selection

**Files:**
- Modify: `pkg/tui/components/sectorgrid/grid.go`
- Modify: `pkg/tui/components/sectorgrid/grid_test.go`
- Modify: `pkg/tui/view.go:58-60`

**Interfaces:**
- Consumes: `engine.Coord`, `engine.QuadrantState`, `theme.Theme`
- Produces:
  - `func (m Model) HitTest(relX, relY int) (engine.Coord, bool)`: translates relative grid coordinates into `(engine.Coord, true)` or `(engine.Coord{}, false)`.
  - `func (m Model) View(quad *engine.QuadrantState, entSector engine.Coord, selected engine.Coord) string`: renders grid with selected cell glyph reticle bracketed (e.g. `[.]`, `[E]`, `[K]`).

- [ ] **Step 1: Write failing unit tests for `HitTest` and reticle rendering**

Add tests in `pkg/tui/components/sectorgrid/grid_test.go`:
- `TestHitTest_ValidCells`: table-driven test verifying that coordinates `(relX, relY)` corresponding to each cell `(1,1)` through `(8,8)` return the exact expected `engine.Coord`.
- `TestHitTest_OutOfBoundsAndBorders`: verifying that clicks on headers (`relY == 0, 1`), separators, or margins return `false`.
- `TestViewWithReticle`: verifying that when `selected == engine.Coord{3, 4}`, the cell at row 3 col 4 renders with reticle brackets `[.]` or bracketed entity glyph.

- [ ] **Step 2: Run tests to verify failure**

Run: `go test -v -race ./pkg/tui/components/sectorgrid`  
Expected: Compilation failure or FAIL (undefined `HitTest` or mismatched `View` arity).

- [ ] **Step 3: Implement `HitTest` and updated `View` in `grid.go`**

Implement:
```go
// HitTest maps relative (x, y) coordinates within the grid's bounding box
// to a 1-indexed engine.Coord [Row, Col].
func (m Model) HitTest(relX, relY int) (engine.Coord, bool) {
	// relY: row 0 is header, row 1..8 are lines relY = r
	// In View: line 0 is col header.
	// Lines 1..8 are the 8 row lines (relY = 1..8).
	if relY < 1 || relY > 8 {
		return engine.Coord{}, false
	}
	r := relY

	// Each row starts with "r " (2 chars).
	// Then for c = 1..8:
	// c=1: chars 2..4 (width 3), char 5 space
	// c=2: chars 6..8, char 9 space
	// c=k: chars 2 + 4*(k-1) .. 4 + 4*(k-1)
	if relX < 2 {
		return engine.Coord{}, false
	}
	offset := relX - 2
	c := (offset / 4) + 1
	charWithinCell := offset % 4
	if c < 1 || c > 8 || charWithinCell >= 3 {
		return engine.Coord{}, false
	}
	return engine.Coord{r, c}, true
}
```
Update `View(quad *engine.QuadrantState, entSector engine.Coord, selected engine.Coord) string` to render reticle styling when `r == selected[0] && c == selected[1]`.
Update caller in `pkg/tui/view.go` to pass `m.SelectedSector`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `go test -v -race ./pkg/tui/components/sectorgrid ./pkg/tui`  
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add pkg/tui/components/sectorgrid/ pkg/tui/view.go
git commit -m "feat(tui): add hit-testing and reticle selection to sector grid"
```

---

### Task 2: Target Lock HUD Component

**Files:**
- Create: `pkg/tui/components/targetlock/targetlock.go`
- Create: `pkg/tui/components/targetlock/targetlock_test.go`

**Interfaces:**
- Consumes: `engine.Coord`, `engine.Klingon`, `theme.Theme`
- Produces:
  - `type TargetInfo struct { KlingonID int; Coord engine.Coord; Distance float64; Bearing float64; HitProbability float64; Power float64 }`
  - `type Model struct`
  - `func New(th theme.Theme) Model`
  - `func (m *Model) SetTheme(th theme.Theme)`
  - `func (m *Model) SetState(entSector engine.Coord, entEnergy float64, torpedoes int, klingons []*engine.Klingon, initialTarget engine.Coord)`
  - `func (m Model) CurrentTarget() *TargetInfo`
  - `func (m Model) Targets() []TargetInfo`
  - `func (m Model) Update(msg tea.Msg) (Model, tea.Cmd)`
  - `func (m Model) View() string`
  - `type FireTorpedoMsg struct { Target engine.Coord; Bearing float64 }`
  - `type FirePhasersMsg struct { Energy float64 }`
  - `type CloseHUDMsg struct{}`

- [ ] **Step 1: Write failing unit tests for Target Lock HUD**

In `pkg/tui/components/targetlock/targetlock_test.go`:
- `TestTargetLock_BallisticsMath`: tests distance, bearing, and hit probability against known sector geometry (e.g. Enterprise at `[4,4]`, Klingon at `[4,7]`: distance 3.0, bearing 1.0, hit prob = 0.80).
- `TestTargetLock_TargetCycling`: tests cycling with `Tab`, `Left`, `Right` across multiple Klingons.
- `TestTargetLock_FireTorpedo`: tests `Enter` emitting `FireTorpedoMsg`.
- `TestTargetLock_ZeroTorpedoesWarning`: tests `Enter` when torpedoes = 0 displays warning without emitting fire message.
- `TestTargetLock_FirePhasersPrompt`: tests `P` key activating phaser prompt and typing numbers + `Enter` emitting `FirePhasersMsg`.
- `TestTargetLock_Close`: tests `Esc` emitting `CloseHUDMsg`.

- [ ] **Step 2: Run test to verify failure**

Run: `go test -v -race ./pkg/tui/components/targetlock`  
Expected: FAIL with package not found or undefined symbols.

- [ ] **Step 3: Implement Target Lock HUD in `targetlock.go`**

Write `pkg/tui/components/targetlock/targetlock.go`:
- Implement Euclidean distance, direction angle / bearing calculation, and bounded hit probability ($0.10 \le P \le 0.95$).
- Implement state setter ordering Klingons by distance from Enterprise.
- Implement Bubble Tea `Update` handling keys: `Enter`, `P`, `Tab`, `Left`, `Right`, `Esc`, and numeric digits during phaser input.
- Implement Lip Gloss `View` with 52x12 panel layout, border, target statistics, ship readiness, and action hints.

- [ ] **Step 4: Run tests to verify they pass**

Run: `go test -v -race ./pkg/tui/components/targetlock`  
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add pkg/tui/components/targetlock/
git commit -m "feat(tui): implement Target Lock HUD component"
```

---

### Task 3: Spock Command Palette Component

**Files:**
- Create: `pkg/tui/components/commandpalette/palette.go`
- Create: `pkg/tui/components/commandpalette/palette_test.go`

**Interfaces:**
- Consumes: `theme.Theme`, `bubbles/list`
- Produces:
  - `type PaletteItem struct { title string; desc string; prefix string; parameterized bool }`
  - `type Model struct`
  - `func New(th theme.Theme, width, height int) Model`
  - `func (m *Model) SetTheme(th theme.Theme)`
  - `func (m *Model) SetSize(width, height int)`
  - `func (m *Model) Reset()`
  - `func (m Model) Update(msg tea.Msg) (Model, tea.Cmd)`
  - `func (m Model) View() string`
  - `type CommandSelectedMsg struct { CommandPrefix string; Parameterized bool }`
  - `type ClosePaletteMsg struct{}`

- [ ] **Step 1: Write failing unit tests for Spock Command Palette**

In `pkg/tui/components/commandpalette/palette_test.go`:
- `TestCommandPalette_Catalog`: verifies catalog contains required commands (`TOR`, `PHA`, `SHE`, `TARGET`, `NAV`, `DOC`, `SRSCAN`, `LRSCAN`, `STATUS`, `DAM`, `CHART`, `THEME: Modern`, `THEME: LCARS`, `THEME: CRT`, `HELP`, `QUIT`).
- `TestCommandPalette_FuzzyFilter`: verifies typing queries like `"pha"` or `"nav"` filters matching items.
- `TestCommandPalette_Selection`: verifies pressing `Enter` on a parameterized command (`"pha"`) emits `CommandSelectedMsg{CommandPrefix: "pha ", Parameterized: true}` and instant command (`"doc"`) emits `Parameterized: false`.
- `TestCommandPalette_Close`: verifies `Esc` emits `ClosePaletteMsg`.

- [ ] **Step 2: Run test to verify failure**

Run: `go test -v -race ./pkg/tui/components/commandpalette`  
Expected: FAIL with package not found.

- [ ] **Step 3: Implement Spock Command Palette in `palette.go`**

Write `pkg/tui/components/commandpalette/palette.go`:
- Configure `list.New` with items from the catalog.
- Customize list styling via active `theme.Theme.Styles()`.
- Implement `Update` handling list navigation, filter typing, `Enter` selection, and `Esc` dismissal.
- Implement `View` rendering a styled 56x16 modal container.

- [ ] **Step 4: Run tests to verify they pass**

Run: `go test -v -race ./pkg/tui/components/commandpalette`  
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add pkg/tui/components/commandpalette/
git commit -m "feat(tui): implement Spock Command Palette modal component"
```

---

### Task 4: Root Model Integration, Modal Compositing & Mouse Event Routing

**Files:**
- Modify: `pkg/tui/model.go`
- Modify: `pkg/tui/view.go`
- Modify: `pkg/tui/update.go`
- Modify: `pkg/tui/model_test.go`

**Interfaces:**
- Consumes: `targetlock.Model`, `commandpalette.Model`, `sectorgrid.Model`
- Produces:
  - Updated `pkg/tui.Model` with `ActiveModal`, `TargetLock`, `CommandPalette`, `SelectedSector`
  - Overlay compositing in `View()`
  - Mouse click handling and modal key intercept in `Update()`

- [ ] **Step 1: Write failing integration tests for modal routing and mouse clicks**

In `pkg/tui/model_test.go`:
- `TestModel_OpenTargetLockHotkey`: pressing `T` with Klingons in quadrant opens `ModalTargetLock`.
- `TestModel_OpenTargetLockNoEnemies`: pressing `T` with 0 Klingons stays on `ModalNone` and logs warning.
- `TestModel_OpenCommandPaletteHotkey`: pressing `Ctrl+P` or `/` opens `ModalCommandPalette`.
- `TestModel_ModalDismissalEsc`: pressing `Esc` when modal is open closes modal and restores `CommandBar`.
- `TestModel_MouseClickSelectSector`: left-click on cell `[3, 4]` updates `SelectedSector`.
- `TestModel_MouseClickKlingonOpensHUD`: left-click on a Klingon sector immediately opens `ModalTargetLock`.
- `TestModel_MouseDoubleClickImpulseMove`: two rapid clicks on empty cell `[5, 5]` dispatches `ActionMove`.
- `TestModel_ModalActionDispatch`: receiving `FireTorpedoMsg` or `FirePhasersMsg` executes action and logs to `CommandBar`.

- [ ] **Step 2: Run tests to verify failure**

Run: `go test -v -race ./pkg/tui`  
Expected: FAIL

- [ ] **Step 3: Implement root model changes**

1. In `pkg/tui/model.go`:
   - Add `ActiveModal`, `TargetLock`, `CommandPalette`, `SelectedSector`, `LastClickTime`, `LastClickCoord`.
   - Initialize `TargetLock` and `CommandPalette` in `NewModel`.
2. In `pkg/tui/view.go`:
   - Pass `m.SelectedSector` to `m.Grid.View(quad, entSector, m.SelectedSector)`.
   - If `m.ActiveModal == ModalTargetLock`, composite `m.TargetLock.View()` centered using `lipgloss.Place`.
   - If `m.ActiveModal == ModalCommandPalette`, composite `m.CommandPalette.View()` centered using `lipgloss.Place`.
3. In `pkg/tui/update.go`:
   - If `m.ActiveModal != ModalNone`: intercept keys and route exclusively to active modal.
   - Handle modal messages: `targetlock.FireTorpedoMsg`, `targetlock.FirePhasersMsg`, `targetlock.CloseHUDMsg`, `commandpalette.CommandSelectedMsg`, `commandpalette.ClosePaletteMsg`.
   - If `m.ActiveModal == ModalNone`:
     - Intercept `T` (Target Lock) and `Ctrl+P` / `/` (Command Palette).
     - Handle `tea.MouseMsg`: left-click hit-tests sector grid; single click updates `SelectedSector` (or opens HUD if Klingon); double-click within 400ms dispatches `ActionMove`.
   - Propagate theme switching (`F2`) to `TargetLock` and `CommandPalette`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `go test -v -race ./pkg/tui/...`  
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add pkg/tui/
git commit -m "feat(tui): wire overlays, modal event routing, and mouse targeting into root model"
```

---

### Task 5: End-to-End CLI Verification & Test Suite Hardening

**Files:**
- Modify: `cmd/sst/main.go`
- Modify: `cmd/sst/main_test.go`

**Interfaces:**
- Consumes: `tea.WithMouseCellMotion()`, `pkg/tui`
- Produces:
  - Enabled mouse input in interactive TUI program options.

- [ ] **Step 1: Write failing test for mouse option initialization**

In `cmd/sst/main_test.go`:
- Verify `cmd/sst` supports interactive runner with mouse cell motion enabled when launching Bubble Tea program.

- [ ] **Step 2: Run test to verify failure or need**

Run: `go test -v -race ./cmd/sst`

- [ ] **Step 3: Update `cmd/sst/main.go`**

Add `tea.WithMouseCellMotion()` to `tea.NewProgram(...)` in `cmd/sst/main.go`.

- [ ] **Step 4: Run full Go test suite with race detector**

Run: `go test -v -race ./...`  
Expected: 100% PASS across all packages.

- [ ] **Step 5: Run C test gate**

Run: `ctest --preset debug`  
Expected: 100% PASS (12 passed, 1 skipped).

- [ ] **Step 6: Commit**

```bash
git add cmd/sst/
git commit -m "feat(cli): enable mouse cell motion for interactive TUI mode"
```
