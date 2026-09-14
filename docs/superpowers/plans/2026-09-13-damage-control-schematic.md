# Interactive Damage Control Schematic Modal Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement a dedicated, interactive Damage Control Schematic Modal (`pkg/tui/components/damageschematic`) displaying a Constitution-class ASCII wireframe silhouette with live subsystem pins, in-flight vs. docked repair telemetry, and full keyboard integration.

**Architecture:** A self-contained Bubble Tea model in `pkg/tui/components/damageschematic` that renders a 66x18 bordered overlay box. Integrated into the root `pkg/tui.Model` as `ModalDamageSchematic`, rendered centered via `compositeOverlay`, and triggered by `dam`/`damage`/`damages` commands and `d`/`D`/`Ctrl+D` hotkeys.

**Tech Stack:** Go 1.26+, Charmbracelet Bubble Tea & Lip Gloss, standard Go `testing`.

**Spec:** `docs/superpowers/specs/2026-09-13-damage-control-schematic-design.md`

## Global Constraints
- Target Go version: Go 1.26+ standard library and Charmbracelet packages (`bubbletea`, `lipgloss`)
- Terminal layout constraint: Strict 80 columns x 24 lines dimension budget for full TUI view
- Modal layout constraint: Exact 66 columns wide x 18 rows high dimension budget
- Zero compiled binaries or temporary files committed to git
- Maintain 100% passing tests for Go (`go test -v -race ./...`) and C (`ctest --preset debug`, `tests/tui.sh`, `tests/golden.sh`)
- Work committed on feature branch `scottdensmore/feat/damage-control-schematic`

---

### Task 1: Schematic Component Data Structures, Layout & ASCII Wireframe (pkg/tui/components/damageschematic)

**Files:**
- Create: `pkg/tui/components/damageschematic/schematic.go`
- Test: `pkg/tui/components/damageschematic/schematic_test.go`

**Interfaces:**
- Consumes: `engine.EnterpriseState`, `engine.DeviceID`, `engine.NumDevices`, `theme.Theme`, `theme.Styles`
- Produces:
  - `type Model struct`
  - `func New(th theme.Theme, width, height int) Model`
  - `func (m *Model) SetTheme(th theme.Theme)`
  - `func (m *Model) SetState(enterprise engine.EnterpriseState, condition string, isDocked bool, repairMult float64)`
  - `func (m Model) View() string`

- [ ] **Step 1: Write the failing test**

Create `pkg/tui/components/damageschematic/schematic_test.go`:
```go
package damageschematic

import (
	"strings"
	"testing"

	"github.com/charmbracelet/x/ansi"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

func TestSchematic_NominalDimensionsAndContent(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th, 66, 18)

	var ent engine.EnterpriseState
	ent.Energy = 5000
	ent.Shields = 2500

	m.SetState(ent, "GREEN", false, 1.0)
	view := m.View()

	lines := strings.Split(view, "\n")
	if len(lines) != 18 {
		t.Fatalf("expected 18 lines, got %d", len(lines))
	}

	for i, line := range lines {
		w := ansi.StringWidth(line)
		if w != 66 {
			t.Errorf("line %d width = %d, expected 66 (content: %q)", i, w, line)
		}
	}

	if !strings.Contains(view, "DAMAGE CONTROL SCHEMATIC") {
		t.Errorf("expected header title in view")
	}
	if !strings.Contains(view, "NCC-1701") {
		t.Errorf("expected ship registry in view")
	}
	if !strings.Contains(view, "SRS: OK") {
		t.Errorf("expected nominal SRS status")
	}
	if !strings.Contains(view, "All other primary and tactical systems operational.") {
		t.Errorf("expected nominal summary message when no devices damaged")
	}
}

func TestSchematic_DamagedSubsystems(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th, 66, 18)

	var ent engine.EnterpriseState
	ent.Devices[engine.DeviceComputer] = 2.1
	ent.Devices[engine.DevicePhotonTubes] = 1.4

	m.SetState(ent, "YELLOW", false, 1.0)
	view := m.View()

	lines := strings.Split(view, "\n")
	if len(lines) != 18 {
		t.Fatalf("expected 18 lines, got %d", len(lines))
	}

	if !strings.Contains(view, "COMP: 2.1") {
		t.Errorf("expected damaged computer countdown in wireframe pin")
	}
	if !strings.Contains(view, "TUB: 1.4") {
		t.Errorf("expected damaged photon tubes countdown in wireframe pin")
	}
	if !strings.Contains(view, "Library Computer") || !strings.Contains(view, "No Chart/Nav") {
		t.Errorf("expected library computer tactical impact breakdown")
	}
	if !strings.Contains(view, "Photon Tubes") || !strings.Contains(view, "Tubes Locked") {
		t.Errorf("expected photon tubes tactical impact breakdown")
	}
}

func TestSchematic_RepairMultiplierAndDocked(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th, 66, 18)

	var ent engine.EnterpriseState
	ent.Devices[engine.DeviceWarp] = 2.0

	// 1.5x repair multiplier
	m.SetState(ent, "RED", false, 1.5)
	view := m.View()

	// In-flight: 2.0 * 1.5 = 3.0 SD; Docked: 3.0 * 0.25 = 0.8 SD
	if !strings.Contains(view, "3.0 SD") {
		t.Errorf("expected scaled in-flight repair time 3.0 SD")
	}
	if !strings.Contains(view, "0.8 SD") {
		t.Errorf("expected scaled docked repair time 0.8 SD")
	}
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `go test -v ./pkg/tui/components/damageschematic`
Expected: FAIL (package does not exist)

- [ ] **Step 3: Write minimal implementation**

Create `pkg/tui/components/damageschematic/schematic.go`:
```go
package damageschematic

import (
	"fmt"
	"strings"

	"github.com/charmbracelet/lipgloss"
	"github.com/charmbracelet/x/ansi"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

type deviceMeta struct {
	id     engine.DeviceID
	tag    string
	name   string
	impact string
}

var devices = []deviceMeta{
	{engine.DeviceWarp, "WARP", "Warp Engines", "Max Warp 0.2"},
	{engine.DeviceSRSensors, "SRS", "Short-Range Sens", "SRS Offline"},
	{engine.DeviceLRSensors, "LRS", "Long-Range Sens", "LRS Offline"},
	{engine.DevicePhasers, "PHAS", "Phaser Controls", "Phasers Locked"},
	{engine.DevicePhotonTubes, "TUB", "Photon Tubes", "Tubes Locked"},
	{engine.DeviceDamageControl, "DAM", "Damage Control", "Repairs Delayed"},
	{engine.DeviceShields, "SHL", "Shield System", "Shields Locked"},
	{engine.DeviceComputer, "COMP", "Library Computer", "No Chart/Nav"},
}

// Model represents the Damage Control Schematic modal component.
type Model struct {
	width      int
	height     int
	theme      theme.Theme
	devices    [engine.NumDevices]float64
	condition  string
	isDocked   bool
	repairMult float64
}

// New constructs a new schematic Model with the specified dimensions.
func New(th theme.Theme, width, height int) Model {
	if th == nil {
		th = theme.DefaultTheme()
	}
	if width <= 0 {
		width = 66
	}
	if height <= 0 {
		height = 18
	}
	return Model{
		width:      width,
		height:     height,
		theme:      th,
		repairMult: 1.0,
	}
}

// SetTheme updates the active theme.
func (m *Model) SetTheme(th theme.Theme) {
	if th != nil {
		m.theme = th
	}
}

// SetState updates device damage, condition, docking status, and repair scaling.
func (m *Model) SetState(enterprise engine.EnterpriseState, condition string, isDocked bool, repairMult float64) {
	m.devices = enterprise.Devices
	m.condition = condition
	m.isDocked = isDocked
	if repairMult <= 0 {
		repairMult = 1.0
	}
	m.repairMult = repairMult
}

func (m Model) formatPin(dev engine.DeviceID, tag string, styles theme.Styles) string {
	dmg := m.devices[dev]
	if dmg <= 0 {
		return styles.SubsystemNormal.Render(fmt.Sprintf("[%s: OK]", tag))
	}
	scaled := dmg * m.repairMult
	if dmg >= 2.0 {
		return styles.TextWarn.Render(fmt.Sprintf("[%s: %.1f]", tag, scaled))
	}
	return styles.SubsystemDamaged.Render(fmt.Sprintf("[%s: %.1f]", tag, scaled))
}

// View renders the bordered 66x18 damage schematic overlay.
func (m Model) View() string {
	styles := m.theme.Styles()

	srsPin := m.formatPin(engine.DeviceSRSensors, "SRS", styles)
	compPin := m.formatPin(engine.DeviceComputer, "COMP", styles)
	lrsPin := m.formatPin(engine.DeviceLRSensors, "LRS", styles)
	phasPin := m.formatPin(engine.DevicePhasers, "PHAS", styles)
	tubPin := m.formatPin(engine.DevicePhotonTubes, "TUB", styles)
	shlPin := m.formatPin(engine.DeviceShields, "SHL", styles)
	damPin := m.formatPin(engine.DeviceDamageControl, "DAM", styles)
	warpPin := m.formatPin(engine.DeviceWarp, "WARP", styles)

	condText := m.condition
	if m.isDocked {
		condText = "DOCKED"
	}
	if condText == "" {
		condText = "GREEN"
	}

	innerW := m.width - 2 // 64 chars inside borders

	headerTitle := styles.PanelTitle.Render("DAMAGE CONTROL SCHEMATIC")
	borderTop := styles.Border.Render("┌") + styles.Border.Render("─") + headerTitle + styles.Border.Render(strings.Repeat("─", innerW-ansi.StringWidth(headerTitle)-1)) + styles.Border.Render("┐")

	formatRow := func(content string) string {
		w := ansi.StringWidth(content)
		pad := innerW - w
		if pad < 0 {
			content = ansi.Truncate(content, innerW, "")
			pad = 0
		}
		return styles.Border.Render("│") + content + strings.Repeat(" ", pad) + styles.Border.Render("│")
	}

	row1 := fmt.Sprintf(" USS ENTERPRISE  NCC-1701                ALERT STATUS: %s", condText)
	row2 := ""
	row3 := fmt.Sprintf("          .---%s---.              %s", srsPin, lrsPin)
	row4 := fmt.Sprintf("         /    %s  \\             %s", compPin, phasPin)
	row5 := fmt.Sprintf("        |   (=) Saucer      |===%s===.", tubPin)
	row6 := "         \\     Bridge      /                 |"
	row7 := fmt.Sprintf("          '---%s---'                  |", shlPin)
	row8 := "                   \\                         |"
	row9 := fmt.Sprintf("                    \\===%s======%s", damPin, warpPin)
	row10 := ""

	var damagedRows []string
	for _, d := range devices {
		dmg := m.devices[d.id]
		if dmg > 0 {
			inFlight := dmg * m.repairMult
			docked := inFlight * 0.25
			statusStr := "DAMAGED"
			if dmg >= 2.0 {
				statusStr = "OFFLINE"
			}
			damagedRows = append(damagedRows, fmt.Sprintf(" %-18s %-9s %5.1f SD  %5.1f SD  %-13s", d.name, statusStr, inFlight, docked, d.impact))
		}
	}

	row11 := " SUBSYSTEM          STATUS    IN-FLIGHT  DOCKED  EFFECT"
	var row12, row13, row14, row15 string

	if len(damagedRows) == 0 {
		row12 = " All primary and auxiliary subsystems operational."
		row13 = " All other primary and tactical systems operational."
		row14 = ""
		row15 = " Scotty- \"All systems purring like kittens, Captain!\""
	} else {
		if len(damagedRows) > 0 {
			row12 = damagedRows[0]
		}
		if len(damagedRows) > 1 {
			row13 = damagedRows[1]
		} else {
			row13 = " All other primary and tactical systems operational."
		}
		if len(damagedRows) > 2 {
			row14 = damagedRows[2]
		} else if len(damagedRows) == 2 {
			row14 = " All other primary and tactical systems operational."
		} else {
			row14 = ""
		}
		if len(damagedRows) > 3 {
			row15 = fmt.Sprintf(" (+%d more damaged subsystems: view in status panel)", len(damagedRows)-3)
		} else {
			row15 = " Dock at starbase for accelerated 4x damage repair."
		}
	}

	row16 := " [ESC / ENTER / Q / D] Dismiss Damage Schematic"
	borderBottom := styles.Border.Render("└") + styles.Border.Render(strings.Repeat("─", innerW)) + styles.Border.Render("┘")

	rows := []string{
		borderTop,
		formatRow(row1),
		formatRow(row2),
		formatRow(row3),
		formatRow(row4),
		formatRow(row5),
		formatRow(row6),
		formatRow(row7),
		formatRow(row8),
		formatRow(row9),
		formatRow(row10),
		formatRow(row11),
		formatRow(row12),
		formatRow(row13),
		formatRow(row14),
		formatRow(row15),
		formatRow(row16),
		borderBottom,
	}

	return strings.Join(rows, "\n")
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `go test -v ./pkg/tui/components/damageschematic`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add pkg/tui/components/damageschematic/schematic.go pkg/tui/components/damageschematic/schematic_test.go
git commit -m "feat(damageschematic): create schematic model and ASCII wireframe rendering"
```

---

### Task 2: Schematic Component Key Handling, Lifecycle & Dismissal (pkg/tui/components/damageschematic)

**Files:**
- Modify: `pkg/tui/components/damageschematic/schematic.go`
- Test: `pkg/tui/components/damageschematic/schematic_test.go`

**Interfaces:**
- Produces:
  - `type CloseModalMsg struct{}`
  - `func (m Model) Update(msg tea.Msg) (Model, tea.Cmd)`

- [ ] **Step 1: Write the failing test**

Add to `pkg/tui/components/damageschematic/schematic_test.go`:
```go
func TestSchematic_UpdateDismissKeys(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th, 66, 18)

	dismissKeys := []string{"esc", "enter", "q", "d", " "}
	for _, k := range dismissKeys {
		var keyMsg tea.KeyMsg
		switch k {
		case "esc":
			keyMsg = tea.KeyMsg{Type: tea.KeyEsc}
		case "enter":
			keyMsg = tea.KeyMsg{Type: tea.KeyEnter}
		case "space", " ":
			keyMsg = tea.KeyMsg{Type: tea.KeySpace}
		default:
			keyMsg = tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune(k)}
		}

		updated, cmd := m.Update(keyMsg)
		_ = updated
		if cmd == nil {
			t.Errorf("expected CloseModalMsg command on key %q, got nil", k)
			continue
		}
		msg := cmd()
		if _, ok := msg.(CloseModalMsg); !ok {
			t.Errorf("expected CloseModalMsg on key %q, got %T", k, msg)
		}
	}

	// Non-dismissal key should not emit CloseModalMsg
	otherKey := tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune("x")}
	_, cmd := m.Update(otherKey)
	if cmd != nil {
		t.Errorf("expected nil cmd on non-dismiss key 'x', got %v", cmd())
	}
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `go test -v ./pkg/tui/components/damageschematic`
Expected: FAIL (`Update` not declared or `CloseModalMsg` not defined)

- [ ] **Step 3: Write minimal implementation**

Add to `pkg/tui/components/damageschematic/schematic.go`:
```go
// CloseModalMsg is emitted when the player presses a dismissal key in the schematic modal.
type CloseModalMsg struct{}

// Update handles keyboard messages for the damage schematic modal.
func (m Model) Update(msg tea.Msg) (Model, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.KeyMsg:
		switch msg.String() {
		case "esc", "enter", "q", "Q", "d", "D", " ":
			return m, func() tea.Msg {
				return CloseModalMsg{}
			}
		}
	}
	return m, nil
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `go test -v ./pkg/tui/components/damageschematic`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add pkg/tui/components/damageschematic/schematic.go pkg/tui/components/damageschematic/schematic_test.go
git commit -m "feat(damageschematic): implement keyboard update routing and CloseModalMsg"
```

---

### Task 3: Root TUI Integration, Commands & Hotkeys (pkg/tui)

**Files:**
- Modify: `pkg/tui/model.go`
- Modify: `pkg/tui/update.go`
- Modify: `pkg/tui/view.go`
- Test: `pkg/tui/model_test.go`

**Interfaces:**
- Consumes: `damageschematic.Model`, `damageschematic.CloseModalMsg`
- Produces:
  - `ModalDamageSchematic` in `ModalType`
  - `DamageSchematic damageschematic.Model` field on `tui.Model`
  - `m.openDamageSchematic() (Model, tea.Cmd)`

- [ ] **Step 1: Write the failing test**

Add tests to `pkg/tui/model_test.go`:
```go
func TestModel_DamageSchematicModal_OpenAndDismiss(t *testing.T) {
	g := engine.NewGame()
	mod := NewModel(g, theme.DefaultTheme())

	// 1. Open via command "dam"
	updated, _ := mod.handleCommand("dam")
	modDam := updated.(Model)
	if modDam.ActiveModal != ModalDamageSchematic {
		t.Fatalf("expected ActiveModal = ModalDamageSchematic on 'dam', got %v", modDam.ActiveModal)
	}

	// 2. Overlay rendered within 80x24 budget
	view := modDam.View()
	lines := strings.Split(view, "\n")
	if len(lines) != 24 {
		t.Errorf("expected exactly 24 lines, got %d", len(lines))
	}
	for i, line := range lines {
		w := ansi.StringWidth(line)
		if w != 80 {
			t.Errorf("line %d width = %d, expected 80", i, w)
		}
	}
	if !strings.Contains(view, "DAMAGE CONTROL SCHEMATIC") {
		t.Errorf("expected schematic title in overlay view")
	}

	// 3. Dismiss via Escape key
	escMsg := tea.KeyMsg{Type: tea.KeyEsc}
	updatedAfterEsc, _ := modDam.Update(escMsg)
	modClosed := updatedAfterEsc.(Model)
	if modClosed.ActiveModal != ModalNone {
		t.Errorf("expected ActiveModal = ModalNone after Esc, got %v", modClosed.ActiveModal)
	}
}

func TestModel_DamageSchematicModal_Hotkey(t *testing.T) {
	g := engine.NewGame()
	mod := NewModel(g, theme.DefaultTheme())

	// Press 'd' when command buffer is empty
	dKey := tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune("d")}
	updated, _ := mod.Update(dKey)
	modD := updated.(Model)
	if modD.ActiveModal != ModalDamageSchematic {
		t.Fatalf("expected ActiveModal = ModalDamageSchematic on 'd' hotkey, got %v", modD.ActiveModal)
	}

	// Press 'd' to close
	updatedClose, _ := modD.Update(dKey)
	modClosed := updatedClose.(Model)
	if modClosed.ActiveModal != ModalNone {
		t.Errorf("expected ActiveModal = ModalNone after 'd' dismiss, got %v", modClosed.ActiveModal)
	}
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `go test -v ./pkg/tui -run TestModel_DamageSchematicModal`
Expected: FAIL (`ModalDamageSchematic` undefined or does not open)

- [ ] **Step 3: Implement minimal code**

1. In `pkg/tui/model.go`:
   - Import `"github.com/scottdensmore/super-star-trek/pkg/tui/components/damageschematic"`
   - Add `ModalDamageSchematic` to `ModalType`
   - Add `DamageSchematic damageschematic.Model` field to `Model` struct
   - In `NewModel`: initialize `DamageSchematic: damageschematic.New(th, 66, 18)`

2. In `pkg/tui/update.go`:
   - In `applyTheme`: add `m.DamageSchematic.SetTheme(th)`
   - Add method:
     ```go
     func (m Model) openDamageSchematic() (Model, tea.Cmd) {
         cond := "GREEN"
         isDocked := false
         repairMult := 1.0
         if m.Game != nil {
             cond = m.Game.Condition
             isDocked = m.Game.Enterprise.Docked
             if m.Game.Rules.RepairMultiplier > 0 {
                 repairMult = m.Game.Rules.RepairMultiplier
             }
         }
         m.DamageSchematic.SetState(m.Game.Enterprise, cond, isDocked, repairMult)
         m.ActiveModal = ModalDamageSchematic
         m.CommandBar.Blur()
         return m, nil
     }
     ```
   - In `handleCommand`:
     Replace case `"dam", "damages":` with:
     ```go
     case "dam", "damage", "damages":
         return m.openDamageSchematic()
     ```
   - In `Update(msg tea.Msg)`:
     When `m.ActiveModal == ModalDamageSchematic`:
     Handle `damageschematic.CloseModalMsg`:
     ```go
     case damageschematic.CloseModalMsg:
         m.ActiveModal = ModalNone
         m.CommandBar.Focus()
         return m, nil
     ```
     Route key events to `m.DamageSchematic.Update(msg)`. If it returns `damageschematic.CloseModalMsg` or on `esc`/`enter`/`q`/`d`:
     ```go
     if m.ActiveModal == ModalDamageSchematic {
         switch msg := msg.(type) {
         case tea.KeyMsg:
             switch msg.String() {
             case "esc", "enter", "q", "Q", "d", "D", " ":
                 m.ActiveModal = ModalNone
                 m.CommandBar.Focus()
                 return m, nil
             }
         }
         var cmd tea.Cmd
         m.DamageSchematic, cmd = m.DamageSchematic.Update(msg)
         return m, cmd
     }
     ```
   - In hotkey routing (when command buffer is empty):
     Add `d` / `D` or `ctrl+d`:
     ```go
     case "d", "D", "ctrl+d":
         if m.CommandBar.Value() == "" {
             return m.openDamageSchematic()
         }
     ```

3. In `pkg/tui/view.go`:
   - In `renderDashboard`:
     Add case to `switch m.ActiveModal`:
     ```go
     case ModalDamageSchematic:
         modalView = m.DamageSchematic.View()
     ```

- [ ] **Step 4: Run test to verify it passes**

Run: `go test -v ./pkg/tui -run TestModel_DamageSchematicModal`
Expected: PASS

Run all TUI tests:
`go test -v ./pkg/tui/...`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add pkg/tui/model.go pkg/tui/update.go pkg/tui/view.go pkg/tui/model_test.go
git commit -m "feat(tui): integrate damage schematic modal, commands, and keyboard triggers"
```

---

### Task 4: Golden Snapshot Visual Regression & Verification (pkg/tui, test suites)

**Files:**
- Modify: `pkg/tui/golden_test.go`
- Create/Update: `pkg/tui/testdata/golden/damage_schematic_modal.txt`

**Interfaces:**
- Verifies full modal rendering across themes and compares with golden snapshots.

- [ ] **Step 1: Write the failing test**

In `pkg/tui/golden_test.go`:
```go
func TestGolden_DamageSchematicModal(t *testing.T) {
	g := createGoldenTestGame()
	g.Enterprise.Devices[engine.DeviceComputer] = 2.1
	g.Enterprise.Devices[engine.DevicePhotonTubes] = 1.4

	mod := NewModel(g, theme.DefaultTheme())
	updated, _ := mod.openDamageSchematic()
	modDam := updated.(Model)

	actual := normalizeForGolden(modDam.View())
	goldenPath := "testdata/golden/damage_schematic_modal.txt"

	if *updateGoldens {
		os.MkdirAll(filepath.Dir(goldenPath), 0755)
		if err := os.WriteFile(goldenPath, []byte(actual), 0644); err != nil {
			t.Fatalf("failed to update golden file: %v", err)
		}
		t.Logf("updated golden file: %s", goldenPath)
		return
	}

	expectedBytes, err := os.ReadFile(goldenPath)
	if err != nil {
		t.Fatalf("golden file not found: %v (run with -update to generate)", err)
	}
	expected := string(expectedBytes)

	if actual != expected {
		t.Errorf("golden mismatch for damage schematic modal:\n%s", diffStrings(expected, actual))
	}
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `go test -v ./pkg/tui -run TestGolden_DamageSchematicModal`
Expected: FAIL (golden file not found)

- [ ] **Step 3: Generate snapshot and verify**

Run: `go test -v ./pkg/tui -run TestGolden_DamageSchematicModal -update`
Expected: PASS (generates `testdata/golden/damage_schematic_modal.txt`)

Run again without `-update`:
`go test -v ./pkg/tui -run TestGolden_DamageSchematicModal`
Expected: PASS

- [ ] **Step 4: Run full verification suite**

```bash
go test -v -race ./...
ctest --preset debug
bash tests/tui.sh
bash tests/golden.sh
```
Expected: All suites PASS 100%.

- [ ] **Step 5: Commit**

```bash
git add pkg/tui/golden_test.go pkg/tui/testdata/golden/damage_schematic_modal.txt
git commit -m "test(tui): add golden snapshot test for damage control schematic modal"
```
