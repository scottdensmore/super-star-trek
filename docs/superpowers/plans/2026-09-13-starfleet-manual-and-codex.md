# Interactive Starfleet Technical Manual & Codex Modal Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement an interactive, LCARS-style two-pane Starfleet Technical Manual & Codex modal (`pkg/tui/components/manual`) with an 8-chapter curriculum, keyboard scrolling and direct numeric jumps, integrated into the root TUI with hotkeys (`F1`, `?`) and commands (`help`, `man`, `manual`, `doc`, `guide`).

**Architecture:** Create `pkg/tui/components/manual` with `Model` managing a 20-column chapter list on the left and a 43-column scrollable reader on the right within an exact 66×18 box. Integrate it into root `pkg/tui` with centered compositing, asynchronous `tea.Cmd` message passing, and topic-directed opening (`help tor`, `man nav`).

**Tech Stack:** Go 1.26+, Charmbracelet Bubble Tea, Lip Gloss, standard Go `testing`.

**Spec:** `docs/superpowers/specs/2026-09-13-starfleet-manual-and-codex-design.md`

## Global Constraints
- Target Go version: Go 1.26+ standard library and Charmbracelet packages (`bubbletea`, `lipgloss`)
- Terminal layout constraint: Strict 80 columns x 24 lines dimension budget for full TUI view
- Modal layout constraint: Exact 66 columns wide x 18 rows high dimension budget
- Zero compiled binaries or temporary files committed to git
- Maintain 100% passing tests for Go (`go test -v -race ./...`) and C (`ctest --preset debug`, `tests/golden.sh`)
- Work committed on feature branch `scottdensmore/feat/starfleet-manual-and-codex`

---

### Task 1: Technical Manual Component Data Structures, Curriculum Store & 66×18 ASCII Wireframe Layout (pkg/tui/components/manual)

**Files:**
- Create: `pkg/tui/components/manual/chapters.go`
- Create: `pkg/tui/components/manual/manual.go`
- Test: `pkg/tui/components/manual/manual_test.go`

**Interfaces:**
- Produces:
  - `type FocusPane int` (`FocusChapters`, `FocusContent`)
  - `type Chapter struct { ID string; Number int; Title string; ShortTag string; Lines []string }`
  - `type Model struct`
  - `func New(th theme.Theme, width, height int) Model`
  - `func (m *Model) SetTheme(th theme.Theme)`
  - `func (m *Model) SelectChapter(topicOrNum string)`
  - `func (m Model) View() string`

- [ ] **Step 1: Write the failing test**

Create `pkg/tui/components/manual/manual_test.go`:
```go
package manual

import (
	"strings"
	"testing"

	"github.com/charmbracelet/x/ansi"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

func TestManual_DimensionsAndLayoutBudget(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th, 66, 18)

	for i := 0; i < len(m.chapters); i++ {
		m.selectedIdx = i
		view := m.View()
		lines := strings.Split(view, "\n")

		if len(lines) != 18 {
			t.Fatalf("chapter %d: expected exactly 18 lines, got %d", i+1, len(lines))
		}

		for lineIdx, line := range lines {
			width := ansi.StringWidth(line)
			if width != 66 {
				t.Errorf("chapter %d line %d: expected width 66, got %d: %q", i+1, lineIdx, width, line)
			}
		}
	}
}

func TestManual_BorderIntegrity(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th, 66, 18)
	view := m.View()
	lines := strings.Split(view, "\n")

	if !strings.HasPrefix(lines[0], "┌") || !strings.HasSuffix(lines[0], "┐") {
		t.Errorf("top border corrupted: %q", lines[0])
	}
	if !strings.HasPrefix(lines[17], "└") || !strings.HasSuffix(lines[17], "┘") {
		t.Errorf("bottom border corrupted: %q", lines[17])
	}

	for i := 1; i <= 16; i++ {
		if !strings.HasPrefix(lines[i], "│") || !strings.HasSuffix(lines[i], "│") {
			t.Errorf("inner row %d missing side borders: %q", i, lines[i])
		}
	}
}

func TestManual_CurriculumChapters(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th, 66, 18)

	if len(m.chapters) != 8 {
		t.Fatalf("expected 8 core chapters, got %d", len(m.chapters))
	}

	expectedIDs := []string{
		"systems", "nav", "combat", "shields",
		"starbases", "tactics", "scoring", "commands",
	}

	for i, expectedID := range expectedIDs {
		if m.chapters[i].ID != expectedID {
			t.Errorf("chapter %d ID mismatch: expected %q, got %q", i+1, expectedID, m.chapters[i].ID)
		}
		if len(m.chapters[i].Lines) == 0 {
			t.Errorf("chapter %q has empty lines buffer", expectedID)
		}
		for lineIdx, line := range m.chapters[i].Lines {
			if w := ansi.StringWidth(line); w > 43 {
				t.Errorf("chapter %q line %d exceeds 43-column reader pane width (got %d): %q", expectedID, lineIdx, w, line)
			}
		}
	}
}

func TestManual_SelectChapter(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th, 66, 18)

	m.SelectChapter("combat")
	if m.selectedIdx != 2 {
		t.Errorf("expected chapter index 2 for 'combat', got %d", m.selectedIdx)
	}

	m.SelectChapter("7")
	if m.selectedIdx != 6 {
		t.Errorf("expected chapter index 6 for '7', got %d", m.selectedIdx)
	}

	m.SelectChapter("unknown")
	if m.selectedIdx != 6 {
		t.Errorf("expected unchanged index on unknown chapter, got %d", m.selectedIdx)
	}
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `go test -v ./pkg/tui/components/manual`
Expected: FAIL (package does not exist yet)

- [ ] **Step 3: Implement minimal code**

1. Create `pkg/tui/components/manual/chapters.go`:
   - Define `defaultChapters() []Chapter` returning all 8 chapters:
     1. Systems & Devices (`"systems"`)
     2. Flight Mechanics & Navigation (`"nav"`)
     3. Weapons & Combat Mathematics (`"combat"`)
     4. Deflector Shields & Damage Control (`"shields"`)
     5. Starbase Logistics & Surveillance (`"starbases"`)
     6. Tactical Threats & Enemy Doctrine (`"tactics"`)
     7. Scoring Engine & Starfleet Ranks (`"scoring"`)
     8. Command Reference & Keyboard Cheatsheet (`"commands"`)
   - Pre-wrap all lines to `ansi.StringWidth(l) <= 43`.

2. Create `pkg/tui/components/manual/manual.go`:
   - Implement `FocusPane`, `Chapter`, `Model`, `New`, `SetTheme`, `SelectChapter`, and `View`.
   - `View()` constructs:
     - Top border (66 cols): `┌─ [F1] STARFLEET TECHNICAL MANUAL & LIBRARY COMPUTER ───────────┐`
     - 16 inner content rows composed of:
       - Left border `│`
       - Left sidebar (20 cols): 8 chapters with pointer arrow `▶` on active, dimmed on others, plus bottom help text.
       - Vertical divider `│` (1 col)
       - Right reader pane (43 cols): chapter title banner + 14 content lines at `m.scrollOffsets[m.selectedIdx]` + scroll status hint.
       - Right border `│`
     - Bottom border (66 cols): `└─ [Tab: Pane]  [↑/↓: Move]  [1-8: Jump]  [Esc/Q/F1: Close] ─────┘`

- [ ] **Step 4: Run test to verify it passes**

Run: `go test -v ./pkg/tui/components/manual`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add pkg/tui/components/manual/chapters.go pkg/tui/components/manual/manual.go pkg/tui/components/manual/manual_test.go
git commit -m "feat(manual): create Starfleet Technical Manual component, curriculum, and 66x18 wireframe"
```

---

### Task 2: Dual-Pane Focus Management, Keyboard Scrolling Math & Dismissal (pkg/tui/components/manual)

**Files:**
- Modify: `pkg/tui/components/manual/manual.go`
- Test: `pkg/tui/components/manual/manual_test.go`

**Interfaces:**
- Produces:
  - `type CloseModalMsg struct{}`
  - `func (m Model) Update(msg tea.Msg) (Model, tea.Cmd)`

- [ ] **Step 1: Write the failing test**

Add to `pkg/tui/components/manual/manual_test.go`:
```go
func TestManual_DualPaneFocus(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th, 66, 18)

	if m.focus != FocusChapters {
		t.Fatalf("expected initial focus on FocusChapters")
	}

	// Tab toggles focus to FocusContent
	updated, _ := m.Update(tea.KeyMsg{Type: tea.KeyTab})
	m = updated.(Model)
	if m.focus != FocusContent {
		t.Errorf("expected focus on FocusContent after Tab, got %v", m.focus)
	}

	// Tab toggles back to FocusChapters
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyTab})
	m = updated.(Model)
	if m.focus != FocusChapters {
		t.Errorf("expected focus on FocusChapters after second Tab, got %v", m.focus)
	}

	// Enter or Right switches to FocusContent
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyRight})
	m = updated.(Model)
	if m.focus != FocusContent {
		t.Errorf("expected focus on FocusContent after KeyRight, got %v", m.focus)
	}

	// Left or Esc returns to FocusChapters
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyLeft})
	m = updated.(Model)
	if m.focus != FocusChapters {
		t.Errorf("expected focus on FocusChapters after KeyLeft, got %v", m.focus)
	}
}

func TestManual_KeyboardNavigationAndJumps(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th, 66, 18)

	// In FocusChapters, down wraps
	updated, _ := m.Update(tea.KeyMsg{Type: tea.KeyDown})
	m = updated.(Model)
	if m.selectedIdx != 1 {
		t.Errorf("expected selectedIdx 1 after down, got %d", m.selectedIdx)
	}

	// Direct numeric key '5'
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune("5")})
	m = updated.(Model)
	if m.selectedIdx != 4 {
		t.Errorf("expected selectedIdx 4 for key '5', got %d", m.selectedIdx)
	}
}

func TestManual_ScrollBoundsClamping(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th, 66, 18)
	m.focus = FocusContent

	// Scrolling up at offset 0 remains 0
	updated, _ := m.Update(tea.KeyMsg{Type: tea.KeyUp})
	m = updated.(Model)
	if m.scrollOffsets[m.selectedIdx] != 0 {
		t.Errorf("expected scroll offset clamped to 0, got %d", m.scrollOffsets[m.selectedIdx])
	}

	// Scrolling down advances offset
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyDown})
	m = updated.(Model)
	if m.scrollOffsets[m.selectedIdx] != 1 {
		t.Errorf("expected scroll offset 1, got %d", m.scrollOffsets[m.selectedIdx])
	}

	// End jumps to bottom
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyEnd})
	m = updated.(Model)
	maxOffset := len(m.chapters[m.selectedIdx].Lines) - 14
	if maxOffset < 0 {
		maxOffset = 0
	}
	if m.scrollOffsets[m.selectedIdx] != maxOffset {
		t.Errorf("expected scroll offset clamped to maxOffset %d, got %d", maxOffset, m.scrollOffsets[m.selectedIdx])
	}

	// Home jumps to top
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyHome})
	m = updated.(Model)
	if m.scrollOffsets[m.selectedIdx] != 0 {
		t.Errorf("expected scroll offset 0 after Home, got %d", m.scrollOffsets[m.selectedIdx])
	}
}

func TestManual_DismissalKeys(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th, 66, 18)

	for _, k := range []string{"esc", "q", "F1", "?"} {
		var keyMsg tea.KeyMsg
		switch k {
		case "esc":
			keyMsg = tea.KeyMsg{Type: tea.KeyEsc}
		case "F1":
			keyMsg = tea.KeyMsg{Type: tea.KeyF1}
		default:
			keyMsg = tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune(k)}
		}

		_, cmd := m.Update(keyMsg)
		if cmd == nil {
			t.Fatalf("expected dismissal command on key %s", k)
		}
		if _, ok := cmd().(CloseModalMsg); !ok {
			t.Errorf("expected CloseModalMsg on key %s, got %T", k, cmd())
		}
	}
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `go test -v ./pkg/tui/components/manual -run TestManual_DualPaneFocus`
Expected: FAIL (`Update` method undefined)

- [ ] **Step 3: Implement minimal code**

In `pkg/tui/components/manual/manual.go`:
- Define `CloseModalMsg struct{}`.
- Implement `Update(msg tea.Msg) (Model, tea.Cmd)`:
  - Handle `tea.KeyMsg`:
    - `esc`, `q`, `Q`, `F1`, `?`: when in `FocusChapters` (or if `q`/`Q`/`F1`/`?` in either pane), return `m, func() tea.Msg { return CloseModalMsg{} }`.
    - `Tab`, `ShiftTab`: toggle `m.focus`.
    - `1`..`8`: set `m.selectedIdx = int(r - '1')`, reset scroll offset.
    - If `m.focus == FocusChapters`:
      - `up`, `k`: `m.selectedIdx = (m.selectedIdx - 1 + len(m.chapters)) % len(m.chapters)`
      - `down`, `j`: `m.selectedIdx = (m.selectedIdx + 1) % len(m.chapters)`
      - `enter`, `right`, `l`: `m.focus = FocusContent`
    - If `m.focus == FocusContent`:
      - `up`, `k`: scroll up 1 line.
      - `down`, `j`: scroll down 1 line.
      - `pgup`, `b`, `ctrl+u`: scroll up 10 lines.
      - `pgdown`, `space`, `ctrl+d`: scroll down 10 lines.
      - `home`, `g`: scroll to line 0.
      - `end`, `G`: scroll to `maxOffset`.
      - `left`, `h`, `esc`: `m.focus = FocusChapters`.

- [ ] **Step 4: Run test to verify it passes**

Run: `go test -v ./pkg/tui/components/manual`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add pkg/tui/components/manual/manual.go pkg/tui/components/manual/manual_test.go
git commit -m "feat(manual): implement dual-pane keyboard navigation, scroll math, and dismissal"
```

---

### Task 3: Root TUI Integration, Commands, Hotkeys & Direct Topic Jumps (pkg/tui)

**Files:**
- Modify: `pkg/tui/model.go`
- Modify: `pkg/tui/update.go`
- Modify: `pkg/tui/view.go`
- Test: `pkg/tui/model_test.go`

**Interfaces:**
- Produces:
  - `ModalManual` in `ModalType`
  - `Manual manual.Model` field on `tui.Model`
  - `m.openManual(topic string) (Model, tea.Cmd)`

- [ ] **Step 1: Write the failing test**

Add to `pkg/tui/model_test.go`:
```go
func TestModel_Manual_OpenAndDismiss(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	mod := NewModel(g, theme.DefaultTheme())

	// 1. Open via command "help"
	updated, _ := mod.handleCommand("help")
	modHelp := updated.(Model)
	if modHelp.ActiveModal != ModalManual {
		t.Fatalf("expected ActiveModal = ModalManual on 'help', got %v", modHelp.ActiveModal)
	}

	// 2. Full-screen dimensions = 80x24
	view := modHelp.View()
	lines := strings.Split(view, "\n")
	if len(lines) != 24 {
		t.Errorf("expected 24 lines, got %d", len(lines))
	}
	for i, l := range lines {
		if w := ansi.StringWidth(l); w != 80 {
			t.Errorf("line %d width = %d, expected 80", i, w)
		}
	}

	// 3. Dismiss via 'esc' key
	escKey := tea.KeyMsg{Type: tea.KeyEsc}
	updatedAfterEsc, _ := modHelp.Update(escKey)
	modClosed := updatedAfterEsc.(Model)
	if modClosed.ActiveModal != ModalNone {
		t.Errorf("expected ActiveModal = ModalNone after 'esc' dismiss, got %v", modClosed.ActiveModal)
	}
}

func TestModel_Manual_TopicJumps(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	mod := NewModel(g, theme.DefaultTheme())

	// Open directly to combat via "help tor"
	updated, _ := mod.handleCommand("help tor")
	modTor := updated.(Model)
	if modTor.ActiveModal != ModalManual {
		t.Fatalf("expected ActiveModal = ModalManual on 'help tor', got %v", modTor.ActiveModal)
	}
	viewTor := modTor.View()
	if !strings.Contains(viewTor, "WEAPONS & COMBAT") {
		t.Errorf("expected Weapons & Combat chapter open for 'help tor'")
	}

	// Open directly to navigation via "man nav"
	updatedNav, _ := mod.handleCommand("man nav")
	modNav := updatedNav.(Model)
	viewNav := modNav.View()
	if !strings.Contains(viewNav, "NAVIGATION") {
		t.Errorf("expected Navigation chapter open for 'man nav'")
	}
}

func TestModel_Manual_Hotkeys(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	mod := NewModel(g, theme.DefaultTheme())

	// Hotkey '?' when command line empty
	qKey := tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune("?")}
	updated, _ := mod.Update(qKey)
	modQ := updated.(Model)
	if modQ.ActiveModal != ModalManual {
		t.Fatalf("expected ActiveModal = ModalManual on '?' hotkey, got %v", modQ.ActiveModal)
	}

	// Dismiss
	modQ.ActiveModal = ModalNone

	// Hotkey F1
	f1Key := tea.KeyMsg{Type: tea.KeyF1}
	updatedF1, _ := modQ.Update(f1Key)
	modF1 := updatedF1.(Model)
	if modF1.ActiveModal != ModalManual {
		t.Fatalf("expected ActiveModal = ModalManual on F1 hotkey, got %v", modF1.ActiveModal)
	}
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `go test -v ./pkg/tui -run TestModel_Manual_`
Expected: FAIL (`ModalManual` undefined)

- [ ] **Step 3: Implement minimal code**

1. In `pkg/tui/model.go`:
   - Add `ModalManual` to `ModalType` enum.
   - Add `Manual manual.Model` to `Model` struct.
   - Initialize in `NewModel`: `Manual: manual.New(th, 66, 18)`.

2. In `pkg/tui/update.go`:
   - In `applyTheme`: call `m.Manual.SetTheme(th)`.
   - Add `openManual(topic string) (Model, tea.Cmd)`:
     - If `topic != ""`, resolve to chapter (`nav`, `tor`, `pha`, `she`, `doc`, `chart`, `dam`, `scores`, `opts`, `tactics`, `scoring`, etc.) and call `m.Manual.SelectChapter(resolved)`.
     - Set `m.ActiveModal = ModalManual` and blur `CommandBar`.
   - In `handleCommand`:
     - Intercept `"help"`, `"man"`, `"manual"`, `"doc"`, `"codex"`, `"guide"` -> call `m.openManual("")`.
     - Intercept `"help <topic>"`, `"man <topic>"` -> call `m.openManual(topic)`.
   - In `Update(msg)`:
     - When `m.ActiveModal == ModalManual`:
       - Handle `manual.CloseModalMsg` -> set `m.ActiveModal = ModalNone` and focus `CommandBar`.
       - Delegate `msg` to `m.Manual.Update(msg)` returning `(m, cmd)`.
     - When command bar is empty and `m.ActiveModal == ModalNone`:
       - Key `'?'` or `"f1"` -> call `m.openManual("")`.

3. In `pkg/tui/view.go`:
   - In `renderDashboard` switch:
     ```go
     case ModalManual:
         modalView = m.Manual.View()
     ```

- [ ] **Step 4: Run test to verify it passes**

Run: `go test -v ./pkg/tui -run TestModel_Manual_`
Expected: PASS

Run full TUI tests:
`go test -v ./pkg/tui/...`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add pkg/tui/model.go pkg/tui/update.go pkg/tui/view.go pkg/tui/model_test.go
git commit -m "feat(tui): integrate Starfleet Technical Manual modal, commands, hotkeys, and topic jumps"
```

---

### Task 4: Golden Snapshot Visual Regression & Full Verification (pkg/tui, test suites)

**Files:**
- Modify: `tests/tui_golden_test.go`
- Create: `tests/golden/tui/modal_manual_systems.golden`
- Create: `tests/golden/tui/modal_manual_combat.golden`

**Interfaces:**
- Validates golden snapshot parity across Chapter 1 (Systems) and Chapter 3 (Combat) within the full 80×24 dashboard.

- [ ] **Step 1: Write the failing test**

In `tests/tui_golden_test.go`:
```go
func TestTUIGolden_ModalManual(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	m := tui.NewModel(g, theme.ModernTheme{})
	updated, _ := m.Update(tea.WindowSizeMsg{Width: 80, Height: 24})
	m = updated.(tui.Model)

	// Chapter 1: Systems (default)
	updatedModal, _ := m.Update(tea.KeyMsg{Type: tea.KeyF1})
	m = updatedModal.(tui.Model)
	compareOrUpdate(t, "modal_manual_systems", m.View())

	// Chapter 3: Combat (press '3')
	updatedCombat, _ := m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune("3")})
	m = updatedCombat.(tui.Model)
	compareOrUpdate(t, "modal_manual_combat", m.View())
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `go test -v ./tests -run TestTUIGolden_ModalManual`
Expected: FAIL (golden files not found)

- [ ] **Step 3: Generate snapshots and verify**

Run: `go test -v ./tests -run TestTUIGolden_ModalManual -update`
Expected: PASS

Run without `-update`:
`go test -v ./tests -run TestTUIGolden_ModalManual`
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
git add tests/tui_golden_test.go tests/golden/tui/modal_manual_*.golden
git commit -m "test(tui): add golden snapshot tests for Starfleet Technical Manual modal"
```

