# Save Game Browser, Command Ergonomics & Multi-Platform Distribution Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver an interactive Save Game Browser modal with rich metadata inspection, readline-style command history with Tab auto-completion, and multi-platform packaging via GoReleaser and CMake CPack.

**Architecture:** Extend `pkg/engine` with non-mutating `.TRK` metadata inspection (`InspectSave`). Enhance `pkg/tui/components/commandbar` with session-scoped history buffer and Tab keyword expansion. Implement `pkg/tui/components/savebrowser` as a themed 62x14 modal dialog with thaw/delete flows. Wire into the root TUI state machine under `ModalSaveBrowser` via `compositeOverlay`, `Ctrl+O`, and `saves`/`thaw` commands. Provide multi-platform distribution configs via `.goreleaser.yaml` (pure Go static binaries) and `CMakeLists.txt` (classic C CPack archives).

**Tech Stack:** Go 1.26+, Bubble Tea v1.3.4, Lip Gloss v1.0.0, Bubbles v0.20.0, CMake 3.21+, CPack, GoReleaser v2.

**Spec:** `docs/superpowers/specs/2026-09-11-save-browser-and-distribution-design.md`

## Global Constraints

- Target Go version: Go 1.26+ with standard library and Charmbracelet packages (`bubbletea`, `lipgloss`, `bubbles`).
- Zero changes to existing game simulation mechanics in `pkg/engine` (only add `InspectSave` and `SaveMetadata`).
- Classic teletype mode (`--classic`) and golden tests (`tests/golden_test.go`) must remain 100% green.
- Modals must be composited over the background dashboard using `compositeOverlay` without clearing or corrupting the view.
- Text input operations and queries must be rune-safe (`[]rune`).
- Every task must pass `go test -v -race ./...` with zero failures and zero race conditions.
- Preserves all C CI gates (`ci-debug` and `ci-release`).
- Work committed on feature branch `scottdensmore/feat/save-browser-and-distribution`.

---

### Task 1: Save Game Metadata Inspection (`pkg/engine`)

**Files:**
- Modify: `pkg/engine/save.go`
- Test: `pkg/engine/save_test.go`

**Interfaces:**
- Consumes: `engine.GameState`, `engine.SkillLevel`, `engine.ConditionType` from `pkg/engine/state.go`
- Produces:
  ```go
  type SaveMetadata struct {
      Path          string
      Filename      string
      ModTime       time.Time
      Skill         SkillLevel
      Stardate      float64
      TimeRemaining float64
      Condition     ConditionType
      KlingonsLeft  int
  }

  func InspectSave(path string) (*SaveMetadata, error)
  ```

- [ ] **Step 1: Write the failing test**

Add tests for `InspectSave` in `pkg/engine/save_test.go`:

```go
func TestInspectSave(t *testing.T) {
	tempDir := t.TempDir()
	savePath := filepath.Join(tempDir, "TESTSAVE.TRK")

	g := NewGame(12345, SkillGood, LengthMedium)
	g.Stardate = 3450.5
	g.TimeRemaining = 24.5
	g.Enterprise.Condition = ConditionYellow
	g.RemainingKlingons = 7

	if err := g.Save(savePath); err != nil {
		t.Fatalf("failed to create test save: %v", err)
	}

	meta, err := InspectSave(savePath)
	if err != nil {
		t.Fatalf("InspectSave returned unexpected error: %v", err)
	}

	if meta.Filename != "TESTSAVE.TRK" {
		t.Errorf("expected Filename 'TESTSAVE.TRK', got %q", meta.Filename)
	}
	if meta.Path != savePath {
		t.Errorf("expected Path %q, got %q", savePath, meta.Path)
	}
	if meta.Skill != SkillGood {
		t.Errorf("expected Skill %v, got %v", SkillGood, meta.Skill)
	}
	if meta.Stardate != 3450.5 {
		t.Errorf("expected Stardate 3450.5, got %f", meta.Stardate)
	}
	if meta.TimeRemaining != 24.5 {
		t.Errorf("expected TimeRemaining 24.5, got %f", meta.TimeRemaining)
	}
	if meta.Condition != ConditionYellow {
		t.Errorf("expected Condition %v, got %v", ConditionYellow, meta.Condition)
	}
	if meta.KlingonsLeft != 7 {
		t.Errorf("expected KlingonsLeft 7, got %d", meta.KlingonsLeft)
	}
	if meta.ModTime.IsZero() {
		t.Errorf("expected non-zero ModTime")
	}

	// Test non-existent file
	if _, err := InspectSave(filepath.Join(tempDir, "NONEXIST.TRK")); err == nil {
		t.Errorf("expected error for non-existent file, got nil")
	}

	// Test corrupted file
	corruptPath := filepath.Join(tempDir, "CORRUPT.TRK")
	if err := os.WriteFile(corruptPath, []byte("NOT_JSON"), 0644); err != nil {
		t.Fatalf("failed to write corrupt file: %v", err)
	}
	if _, err := InspectSave(corruptPath); err == nil {
		t.Errorf("expected error for corrupt JSON save, got nil")
	}
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `go test -v -race ./pkg/engine -run TestInspectSave`  
Expected: FAIL with `undefined: InspectSave`

- [ ] **Step 3: Write minimal implementation**

In `pkg/engine/save.go`, define `SaveMetadata` and implement `InspectSave`:

```go
// SaveMetadata holds summary attributes extracted from a saved game file.
type SaveMetadata struct {
	Path          string
	Filename      string
	ModTime       time.Time
	Skill         SkillLevel
	Stardate      float64
	TimeRemaining float64
	Condition     ConditionType
	KlingonsLeft  int
}

// InspectSave reads a .TRK file and extracts summary metadata without mutating any active game state.
func InspectSave(path string) (*SaveMetadata, error) {
	info, err := os.Stat(path)
	if err != nil {
		return nil, err
	}

	data, err := os.ReadFile(path)
	if err != nil {
		return nil, err
	}

	var state GameState
	if err := json.Unmarshal(data, &state); err != nil {
		return nil, fmt.Errorf("corrupted save file %s: %w", filepath.Base(path), err)
	}

	return &SaveMetadata{
		Path:          path,
		Filename:      filepath.Base(path),
		ModTime:       info.ModTime(),
		Skill:         state.Skill,
		Stardate:      state.Stardate,
		TimeRemaining: state.TimeRemaining,
		Condition:     state.Enterprise.Condition,
		KlingonsLeft:  state.RemainingKlingons,
	}, nil
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `go test -v -race ./pkg/engine`  
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add pkg/engine/save.go pkg/engine/save_test.go
git commit -m "feat(engine): add SaveMetadata and InspectSave for save file inspection"
```

---

### Task 2: Command Bar History & Tab Completion (`pkg/tui/components/commandbar`)

**Files:**
- Modify: `pkg/tui/components/commandbar/bar.go`
- Test: `pkg/tui/components/commandbar/bar_test.go`

**Interfaces:**
- Consumes: `theme.Theme`
- Produces:
  - Readline-style history recall on `Up`/`Down` with uncommitted draft text preservation
  - Canonical Tab keyword completion (`nav `, `tor `, `pha `, `she `, `theme `, `doc`, `srscan`, `lrscan`, `status`, `damage`, `chart`, `target`, `saves`, `thaw`, `help`, `quit`) with candidate cycling

- [ ] **Step 1: Write the failing test**

In `pkg/tui/components/commandbar/bar_test.go`, add tests for history navigation and Tab auto-completion:

```go
func TestCommandBarHistory(t *testing.T) {
	th := theme.DefaultTheme()
	cb := New(th)

	// Type and submit "nav 1 2"
	cb.SetValue("nav 1 2")
	cb, _ = cb.Update(tea.KeyMsg{Type: tea.KeyEnter})

	// Type and submit "tor 3 4"
	cb.SetValue("tor 3 4")
	cb, _ = cb.Update(tea.KeyMsg{Type: tea.KeyEnter})

	// Start typing draft "ph"
	cb.SetValue("ph")

	// Press Up -> recall "tor 3 4"
	cb, _ = cb.Update(tea.KeyMsg{Type: tea.KeyUp})
	if cb.Value() != "tor 3 4" {
		t.Errorf("expected recalled history 'tor 3 4', got %q", cb.Value())
	}

	// Press Up again -> recall "nav 1 2"
	cb, _ = cb.Update(tea.KeyMsg{Type: tea.KeyUp})
	if cb.Value() != "nav 1 2" {
		t.Errorf("expected recalled history 'nav 1 2', got %q", cb.Value())
	}

	// Press Down -> back to "tor 3 4"
	cb, _ = cb.Update(tea.KeyMsg{Type: tea.KeyDown})
	if cb.Value() != "tor 3 4" {
		t.Errorf("expected recalled history 'tor 3 4', got %q", cb.Value())
	}

	// Press Down again -> restore draft "ph"
	cb, _ = cb.Update(tea.KeyMsg{Type: tea.KeyDown})
	if cb.Value() != "ph" {
		t.Errorf("expected restored draft 'ph', got %q", cb.Value())
	}
}

func TestCommandBarTabCompletion(t *testing.T) {
	th := theme.DefaultTheme()
	cb := New(th)

	// Single match: "ph" -> "pha "
	cb.SetValue("ph")
	cb, _ = cb.Update(tea.KeyMsg{Type: tea.KeyTab})
	if cb.Value() != "pha " {
		t.Errorf("expected tab completion 'pha ', got %q", cb.Value())
	}

	// Multiple matches: "s" -> ["saves", "she ", "srscan", "status"]
	cb.SetValue("s")
	cb, _ = cb.Update(tea.KeyMsg{Type: tea.KeyTab})
	firstMatch := cb.Value()
	if firstMatch != "saves" {
		t.Errorf("expected first tab candidate 'saves', got %q", firstMatch)
	}

	// Tab cycle 2
	cb, _ = cb.Update(tea.KeyMsg{Type: tea.KeyTab})
	if cb.Value() != "she " {
		t.Errorf("expected second tab candidate 'she ', got %q", cb.Value())
	}

	// Typing key resets tab cycle
	cb, _ = cb.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'x'}})
	cb.SetValue("doc")
	cb, _ = cb.Update(tea.KeyMsg{Type: tea.KeyTab})
	if cb.Value() != "doc" {
		t.Errorf("expected instant match 'doc', got %q", cb.Value())
	}
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `go test -v -race ./pkg/tui/components/commandbar -run "TestCommandBarHistory|TestCommandBarTabCompletion"`  
Expected: FAIL (Key Up/Down and Tab do not alter input value).

- [ ] **Step 3: Write minimal implementation**

In `pkg/tui/components/commandbar/bar.go`:
1. Add history and tab completion fields to `Model`:
```go
type Model struct {
	textinput.Model
	theme       theme.Theme
	messages    []string
	maxMessages int
	width       int

	history     []string
	historyIdx  int
	draftInput  string
	tabMatches  []string
	tabMatchIdx int
}
```
2. Initialize them in `New(th theme.Theme)`:
```go
	return Model{
		Model:       ti,
		theme:       th,
		messages:    make([]string, 0, 4),
		maxMessages: 4,
		historyIdx:  -1,
	}
```
3. Define canonical keywords:
```go
var canonicalKeywords = []string{
	"chart",
	"damage",
	"doc",
	"help",
	"lrscan",
	"nav ",
	"pha ",
	"quit",
	"saves",
	"she ",
	"srscan",
	"status",
	"target",
	"thaw",
	"theme ",
	"tor ",
}
```
4. In `Update(msg tea.Msg)`:
```go
func (m Model) Update(msg tea.Msg) (Model, tea.Cmd) {
	if keyMsg, ok := msg.(tea.KeyMsg); ok {
		switch keyMsg.Type {
		case tea.KeyUp:
			if m.historyIdx == -1 {
				if len(m.history) > 0 {
					m.draftInput = m.Value()
					m.historyIdx = len(m.history) - 1
					m.SetValue(m.history[m.historyIdx])
					m.CursorEnd()
				}
			} else if m.historyIdx > 0 {
				m.historyIdx--
				m.SetValue(m.history[m.historyIdx])
				m.CursorEnd()
			}
			return m, nil

		case tea.KeyDown:
			if m.historyIdx != -1 {
				if m.historyIdx < len(m.history)-1 {
					m.historyIdx++
					m.SetValue(m.history[m.historyIdx])
					m.CursorEnd()
				} else if m.historyIdx == len(m.history)-1 {
					m.historyIdx = -1
					m.SetValue(m.draftInput)
					m.CursorEnd()
				}
			}
			return m, nil

		case tea.KeyTab:
			if len(m.tabMatches) == 0 {
				prefix := strings.ToLower(strings.TrimSpace(m.Value()))
				if prefix != "" {
					var matches []string
					for _, kw := range canonicalKeywords {
						trimmedKw := strings.TrimSpace(kw)
						if strings.HasPrefix(trimmedKw, prefix) {
							matches = append(matches, kw)
						}
					}
					if len(matches) > 0 {
						m.tabMatches = matches
						m.tabMatchIdx = 0
						m.SetValue(m.tabMatches[0])
						m.CursorEnd()
					}
				}
			} else {
				m.tabMatchIdx = (m.tabMatchIdx + 1) % len(m.tabMatches)
				m.SetValue(m.tabMatches[m.tabMatchIdx])
				m.CursorEnd()
			}
			return m, nil

		case tea.KeyEnter:
			val := strings.TrimSpace(m.Value())
			if val != "" {
				if len(m.history) == 0 || m.history[len(m.history)-1] != val {
					m.history = append(m.history, val)
				}
				m.historyIdx = -1
				m.draftInput = ""
				m.tabMatches = nil
				m.tabMatchIdx = 0
				m.Reset()
				return m, func() tea.Msg {
					return CommandSubmittedMsg{Text: val}
				}
			}
			return m, nil

		default:
			// Any key other than Tab resets active tab cycling
			m.tabMatches = nil
			m.tabMatchIdx = 0
		}
	}

	var cmd tea.Cmd
	m.Model, cmd = m.Model.Update(msg)
	return m, cmd
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `go test -v -race ./pkg/tui/components/commandbar`  
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add pkg/tui/components/commandbar/bar.go pkg/tui/components/commandbar/bar_test.go
git commit -m "feat(commandbar): add readline command history and tab completion"
```

---

### Task 3: Save Game Browser Component (`pkg/tui/components/savebrowser`)

**Files:**
- Create: `pkg/tui/components/savebrowser/browser.go`
- Test: `pkg/tui/components/savebrowser/browser_test.go`

**Interfaces:**
- Consumes: `engine.InspectSave`, `engine.SaveMetadata` from `pkg/engine`, `theme.Theme` from `pkg/tui/theme`
- Produces:
  ```go
  type LoadGameMsg struct { Path string }
  type CloseBrowserMsg struct{}

  type Model struct { ... }
  func New(th theme.Theme, dir string) Model
  func (m *Model) SetTheme(th theme.Theme)
  func (m *Model) SetSize(width, height int)
  func (m *Model) Refresh() error
  func (m *Model) Reset()
  func (m Model) Update(msg tea.Msg) (Model, tea.Cmd)
  func (m Model) View() string
  ```

- [ ] **Step 1: Write the failing test**

Create `pkg/tui/components/savebrowser/browser_test.go`:

```go
package savebrowser

import (
	"os"
	"path/filepath"
	"strings"
	"testing"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

func createTestSave(t *testing.T, dir, filename string, stardate float64, skill engine.SkillLevel) string {
	t.Helper()
	g := engine.NewGame(42, skill, engine.LengthShort)
	g.Stardate = stardate
	p := filepath.Join(dir, filename)
	if err := g.Save(p); err != nil {
		t.Fatalf("failed to create save %s: %v", filename, err)
	}
	return p
}

func TestSaveBrowserListingAndNavigation(t *testing.T) {
	tempDir := t.TempDir()
	createTestSave(t, tempDir, "GAME1.TRK", 3100.0, engine.SkillNovice)
	createTestSave(t, tempDir, "GAME2.TRK", 3200.0, engine.SkillGood)

	m := New(theme.DefaultTheme(), tempDir)
	if err := m.Refresh(); err != nil {
		t.Fatalf("Refresh failed: %v", err)
	}

	if len(m.saves) != 2 {
		t.Fatalf("expected 2 saves, got %d", len(m.saves))
	}

	if m.cursor != 0 {
		t.Errorf("expected cursor 0, got %d", m.cursor)
	}

	// Move down
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyDown})
	if m.cursor != 1 {
		t.Errorf("expected cursor 1 after KeyDown, got %d", m.cursor)
	}

	// Move down at bound (clamped)
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyDown})
	if m.cursor != 1 {
		t.Errorf("expected cursor clamped at 1, got %d", m.cursor)
	}

	// Move up
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyUp})
	if m.cursor != 0 {
		t.Errorf("expected cursor 0 after KeyUp, got %d", m.cursor)
	}
}

func TestSaveBrowserLoadGameMsg(t *testing.T) {
	tempDir := t.TempDir()
	p := createTestSave(t, tempDir, "LOADME.TRK", 3300.0, engine.SkillExpert)

	m := New(theme.DefaultTheme(), tempDir)
	_ = m.Refresh()

	var emitted tea.Msg
	m, cmd := m.Update(tea.KeyMsg{Type: tea.KeyEnter})
	if cmd != nil {
		emitted = cmd()
	}

	loadMsg, ok := emitted.(LoadGameMsg)
	if !ok {
		t.Fatalf("expected LoadGameMsg, got %T (%v)", emitted, emitted)
	}
	if filepath.Base(loadMsg.Path) != filepath.Base(p) {
		t.Errorf("expected LoadGameMsg.Path %s, got %s", p, loadMsg.Path)
	}
}

func TestSaveBrowserDeleteFlow(t *testing.T) {
	tempDir := t.TempDir()
	p := createTestSave(t, tempDir, "DELME.TRK", 3400.0, engine.SkillFair)

	m := New(theme.DefaultTheme(), tempDir)
	_ = m.Refresh()

	// Press 'd' -> enters deleting mode
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'d'}})
	if !m.deleting {
		t.Errorf("expected deleting mode true after 'd'")
	}
	if !strings.Contains(m.View(), "Delete 'DELME.TRK'?") {
		t.Errorf("expected delete prompt in View(), got %s", m.View())
	}

	// Press 'Esc' cancels delete mode
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyEsc})
	if m.deleting {
		t.Errorf("expected deleting mode false after Esc")
	}

	// Press 'd' again then Enter -> deletes file
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'d'}})
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyEnter})
	if m.deleting {
		t.Errorf("expected deleting mode false after confirmed delete")
	}
	if _, err := os.Stat(p); !os.IsNotExist(err) {
		t.Errorf("expected file %s to be deleted, stat err: %v", p, err)
	}
	if len(m.saves) != 0 {
		t.Errorf("expected 0 saves remaining, got %d", len(m.saves))
	}
}

func TestSaveBrowserCloseMsg(t *testing.T) {
	m := New(theme.DefaultTheme(), t.TempDir())
	_ = m.Refresh()

	m, cmd := m.Update(tea.KeyMsg{Type: tea.KeyEsc})
	if cmd == nil {
		t.Fatalf("expected command on Esc, got nil")
	}
	msg := cmd()
	if _, ok := msg.(CloseBrowserMsg); !ok {
		t.Errorf("expected CloseBrowserMsg on Esc, got %T", msg)
	}
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `go test -v -race ./pkg/tui/components/savebrowser`  
Expected: FAIL with package not found / undefined symbols.

- [ ] **Step 3: Write minimal implementation**

Create `pkg/tui/components/savebrowser/browser.go`:

```go
package savebrowser

import (
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strings"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

// LoadGameMsg is emitted when the user confirms thawing a mission.
type LoadGameMsg struct {
	Path string
}

// CloseBrowserMsg is emitted when the user dismisses the save browser without selection.
type CloseBrowserMsg struct{}

// Model represents the interactive save game browser modal.
type Model struct {
	theme        theme.Theme
	directory    string
	saves        []engine.SaveMetadata
	cursor       int
	deleting     bool
	errorMessage string
	width        int
	height       int
}

// New creates a new save game browser model.
func New(th theme.Theme, dir string) Model {
	if th == nil {
		th = theme.DefaultTheme()
	}
	if dir == "" {
		dir = "."
	}
	return Model{
		theme:     th,
		directory: dir,
		width:     62,
		height:    14,
	}
}

// SetTheme updates the component theme.
func (m *Model) SetTheme(th theme.Theme) {
	if th == nil {
		th = theme.DefaultTheme()
	}
	m.theme = th
}

// SetSize updates the dimensions for the modal dialog.
func (m *Model) SetSize(width, height int) {
	m.width = width
	m.height = height
}

// Reset resets cursor position, deletion state, and error message.
func (m *Model) Reset() {
	m.cursor = 0
	m.deleting = false
	m.errorMessage = ""
}

// Refresh scans the target directory for *.TRK files and parses their metadata.
func (m *Model) Refresh() error {
	m.errorMessage = ""
	entries, err := os.ReadDir(m.directory)
	if err != nil {
		m.errorMessage = fmt.Sprintf("Failed to read directory: %v", err)
		m.saves = nil
		m.cursor = 0
		return err
	}

	var loaded []engine.SaveMetadata
	for _, entry := range entries {
		if entry.IsDir() {
			continue
		}
		name := entry.Name()
		if strings.HasSuffix(strings.ToUpper(name), ".TRK") {
			fullPath := filepath.Join(m.directory, name)
			meta, err := engine.InspectSave(fullPath)
			if err == nil && meta != nil {
				loaded = append(loaded, *meta)
			}
		}
	}

	// Sort saves by modification time descending (most recent first)
	sort.Slice(loaded, func(i, j int) bool {
		return loaded[i].ModTime.After(loaded[j].ModTime)
	})

	m.saves = loaded
	if m.cursor >= len(m.saves) {
		m.cursor = len(m.saves) - 1
	}
	if m.cursor < 0 {
		m.cursor = 0
	}
	return nil
}

// Update processes keyboard navigation, selection, and deletion.
func (m Model) Update(msg tea.Msg) (Model, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.KeyMsg:
		switch msg.Type {
		case tea.KeyUp:
			if !m.deleting && m.cursor > 0 {
				m.cursor--
			}
			return m, nil

		case tea.KeyDown:
			if !m.deleting && m.cursor < len(m.saves)-1 {
				m.cursor++
			}
			return m, nil

		case tea.KeyEnter:
			if m.deleting {
				if len(m.saves) > 0 && m.cursor >= 0 && m.cursor < len(m.saves) {
					target := m.saves[m.cursor].Path
					if err := os.Remove(target); err != nil {
						m.errorMessage = fmt.Sprintf("Delete failed: %v", err)
					}
					m.deleting = false
					_ = m.Refresh()
				}
				return m, nil
			}

			if len(m.saves) > 0 && m.cursor >= 0 && m.cursor < len(m.saves) {
				selectedPath := m.saves[m.cursor].Path
				return m, func() tea.Msg {
					return LoadGameMsg{Path: selectedPath}
				}
			}
			return m, nil

		case tea.KeyEsc:
			if m.deleting {
				m.deleting = false
				return m, nil
			}
			return m, func() tea.Msg {
				return CloseBrowserMsg{}
			}

		case tea.KeyDelete:
			if len(m.saves) > 0 {
				m.deleting = true
			}
			return m, nil

		case tea.KeyRunes:
			s := msg.String()
			switch s {
			case "k", "K":
				if !m.deleting && m.cursor > 0 {
					m.cursor--
				}
				return m, nil
			case "j", "J":
				if !m.deleting && m.cursor < len(m.saves)-1 {
					m.cursor++
				}
				return m, nil
			case "d", "D":
				if len(m.saves) > 0 {
					m.deleting = !m.deleting
				}
				return m, nil
			}
		}
	}
	return m, nil
}

// View renders the 62x14 styled modal dialog.
func (m Model) View() string {
	th := m.theme
	if th == nil {
		th = theme.DefaultTheme()
	}
	styles := th.Styles()

	dialogWidth := 60
	if m.width > 0 {
		dialogWidth = m.width - 2
	}

	titleText := " SAVED MISSIONS (Ctrl+O) "
	header := styles.Title.Render(titleText)

	colHeader := styles.GaugeLabel.Render("  FILE       STARDATE  SKILL   COND    KLINGONS  MODIFIED")

	var rows []string
	const visibleRows = 7
	startIdx := 0
	if m.cursor >= visibleRows {
		startIdx = m.cursor - visibleRows + 1
	}
	endIdx := startIdx + visibleRows
	if endIdx > len(m.saves) {
		endIdx = len(m.saves)
	}

	if len(m.saves) == 0 {
		emptyMsg := styles.LogText.Render("  No saved missions (*.TRK) found.")
		rows = append(rows, emptyMsg)
		for len(rows) < visibleRows {
			rows = append(rows, "")
		}
	} else {
		for i := startIdx; i < endIdx; i++ {
			s := m.saves[i]
			cursorMark := "  "
			if i == m.cursor {
				cursorMark = "> "
			}

			skillStr := formatSkill(s.Skill)
			condStr := formatCondition(s.Condition)
			modStr := s.ModTime.Format("01-02 15:04")
			baseName := s.Filename
			if len(baseName) > 10 {
				baseName = baseName[:10]
			}

			line := fmt.Sprintf("%s%-10s %-9.1f %-7s %-7s %-9d %s",
				cursorMark, baseName, s.Stardate, skillStr, condStr, s.KlingonsLeft, modStr)

			if i == m.cursor {
				rows = append(rows, styles.CommandText.Render(line))
			} else {
				rows = append(rows, styles.LogText.Render(line))
			}
		}
		for len(rows) < visibleRows {
			rows = append(rows, "")
		}
	}

	divider := styles.Border.Render(strings.Repeat("─", dialogWidth))

	var footer string
	if m.deleting && len(m.saves) > 0 && m.cursor < len(m.saves) {
		delPrompt := fmt.Sprintf("Delete '%s'? [Enter: Confirm / Esc: Cancel]", m.saves[m.cursor].Filename)
		footer = styles.GaugeValue.Render(delPrompt)
	} else if m.errorMessage != "" {
		footer = styles.GaugeValue.Render(m.errorMessage)
	} else {
		footer = styles.LogText.Render("[Enter] Thaw  [D] Delete  [↑/↓] Select  [Esc] Close")
	}

	body := fmt.Sprintf("%s\n\n%s\n%s\n%s\n%s",
		header,
		colHeader,
		strings.Join(rows, "\n"),
		divider,
		footer,
	)

	return styles.Panel.Width(dialogWidth).Render(body)
}

func formatSkill(skill engine.SkillLevel) string {
	switch skill {
	case engine.SkillNovice:
		return "NOVICE"
	case engine.SkillFair:
		return "FAIR"
	case engine.SkillGood:
		return "GOOD"
	case engine.SkillExpert:
		return "EXPERT"
	case engine.SkillEmeritus:
		return "EMERITUS"
	default:
		return "UNKNOWN"
	}
}

func formatCondition(c engine.ConditionType) string {
	switch c {
	case engine.ConditionGreen:
		return "GREEN"
	case engine.ConditionYellow:
		return "YELLOW"
	case engine.ConditionRed:
		return "RED"
	case engine.ConditionDocked:
		return "DOCKED"
	default:
		return "UNKNOWN"
	}
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `go test -v -race ./pkg/tui/components/savebrowser`  
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add pkg/tui/components/savebrowser/
git commit -m "feat(savebrowser): implement interactive save game browser modal"
```

---

### Task 4: Root TUI Integration (`pkg/tui`)

**Files:**
- Modify: `pkg/tui/model.go`
- Modify: `pkg/tui/view.go`
- Modify: `pkg/tui/update.go`
- Modify: `pkg/tui/components/commandpalette/palette.go`
- Test: `pkg/tui/model_test.go`

**Interfaces:**
- Consumes: `savebrowser.Model`, `savebrowser.LoadGameMsg`, `savebrowser.CloseBrowserMsg` from `pkg/tui/components/savebrowser`
- Produces:
  - `ModalSaveBrowser` state in `ActiveModal`
  - Global `Ctrl+O` hotkey
  - `saves` and `thaw` commands
  - Seamless overlay compositing via `compositeOverlay`

- [ ] **Step 1: Write the failing test**

In `pkg/tui/model_test.go`, add integration tests for `ModalSaveBrowser`:

```go
func TestModalSaveBrowserIntegration(t *testing.T) {
	tempDir := t.TempDir()
	g := engine.NewGame(12345, engine.SkillNovice, engine.LengthShort)
	savePath := filepath.Join(tempDir, "TESTSAV.TRK")
	g.Stardate = 3150.0
	_ = g.Save(savePath)

	m := NewModel(g, theme.DefaultTheme())
	m.Width = 100
	m.Height = 30
	m.SaveBrowser = savebrowser.New(theme.DefaultTheme(), tempDir)

	// Test 1: Open via Ctrl+O
	m, _ = m.UpdateModel(tea.KeyMsg{Type: tea.KeyCtrlO})
	if m.ActiveModal != ModalSaveBrowser {
		t.Fatalf("expected ActiveModal == ModalSaveBrowser after Ctrl+O, got %v", m.ActiveModal)
	}

	// Test 2: Dismiss via CloseBrowserMsg
	m, _ = m.UpdateModel(savebrowser.CloseBrowserMsg{})
	if m.ActiveModal != ModalNone {
		t.Fatalf("expected ActiveModal == ModalNone after CloseBrowserMsg, got %v", m.ActiveModal)
	}

	// Test 3: Open via "saves" command
	m, _ = m.UpdateModel(commandbar.CommandSubmittedMsg{Text: "saves"})
	if m.ActiveModal != ModalSaveBrowser {
		t.Fatalf("expected ActiveModal == ModalSaveBrowser after 'saves' command, got %v", m.ActiveModal)
	}

	// Test 4: Thaw via LoadGameMsg
	m, _ = m.UpdateModel(savebrowser.LoadGameMsg{Path: savePath})
	if m.ActiveModal != ModalNone {
		t.Fatalf("expected ActiveModal == ModalNone after LoadGameMsg, got %v", m.ActiveModal)
	}
	if m.Game.Stardate != 3150.0 {
		t.Errorf("expected loaded Game.Stardate 3150.0, got %f", m.Game.Stardate)
	}

	// Test 5: Open via "thaw" command without args
	m, _ = m.UpdateModel(commandbar.CommandSubmittedMsg{Text: "thaw"})
	if m.ActiveModal != ModalSaveBrowser {
		t.Fatalf("expected ActiveModal == ModalSaveBrowser after bare 'thaw' command, got %v", m.ActiveModal)
	}

	// Test 6: Direct load via "thaw <filename>"
	m.ActiveModal = ModalNone
	m, _ = m.UpdateModel(commandbar.CommandSubmittedMsg{Text: "thaw " + savePath})
	if m.Game.Stardate != 3150.0 {
		t.Errorf("expected loaded Game.Stardate 3150.0 from thaw command, got %f", m.Game.Stardate)
	}
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `go test -v -race ./pkg/tui -run TestModalSaveBrowserIntegration`  
Expected: FAIL with `undefined: ModalSaveBrowser`

- [ ] **Step 3: Write minimal implementation**

1. In `pkg/tui/model.go`:
   - Add `ModalSaveBrowser` to `ModalType` enum:
     ```go
     const (
     	ModalNone ModalType = iota
     	ModalTargetLock
     	ModalCommandPalette
     	ModalSaveBrowser
     )
     ```
   - Add `SaveBrowser savebrowser.Model` to `Model` struct.
   - In `NewModel(g, th)`:
     ```go
     SaveBrowser: savebrowser.New(th, "."),
     ```

2. In `pkg/tui/view.go`:
   - In `renderDashboard()`:
     ```go
     	switch m.ActiveModal {
     	case ModalTargetLock:
     		modalView = m.TargetLock.View()
     	case ModalCommandPalette:
     		modalView = m.CommandPalette.View()
     	case ModalSaveBrowser:
     		modalView = m.SaveBrowser.View()
     	default:
     		return dashboard
     	}
     ```

3. In `pkg/tui/update.go`:
   - In `Update(msg tea.Msg)`:
     - Handle `savebrowser.LoadGameMsg`:
       ```go
       case savebrowser.LoadGameMsg:
       	loaded, err := engine.LoadGame(msg.Path)
       	if err != nil {
       		m.CommandBar.AddMessage(fmt.Sprintf("Failed to thaw %s: %v", filepath.Base(msg.Path), err))
       	} else {
       		m.Game = loaded
       		m.SelectedSector = engine.Coord{}
       		m.CommandBar.AddMessage(fmt.Sprintf("Mission thawed: %s (Stardate %.1f)", filepath.Base(msg.Path), loaded.Stardate))
       	}
       	m.ActiveModal = ModalNone
       	cmd := m.CommandBar.Focus()
       	return m, cmd
       ```
     - Handle `savebrowser.CloseBrowserMsg`:
       ```go
       case savebrowser.CloseBrowserMsg:
       	m.ActiveModal = ModalNone
       	cmd := m.CommandBar.Focus()
       	return m, cmd
       ```
     - When `m.ActiveModal == ModalSaveBrowser`:
       Delegate key events to `m.SaveBrowser.Update(msg)`:
       ```go
       case ModalSaveBrowser:
       	m.SaveBrowser, cmd = m.SaveBrowser.Update(msg)
       ```
     - Global hotkey `Ctrl+O`:
       ```go
       case msg.Type == tea.KeyCtrlO || msg.String() == "ctrl+o":
       	_ = m.SaveBrowser.Refresh()
       	m.ActiveModal = ModalSaveBrowser
       	m.CommandBar.Blur()
       	return m, nil
       ```
     - In `applyTheme`:
       ```go
       m.SaveBrowser.SetTheme(th)
       ```
     - In `handleCommand(text string)`:
       ```go
       case "saves", "thaw":
       	_ = m.SaveBrowser.Refresh()
       	m.ActiveModal = ModalSaveBrowser
       	m.CommandBar.Blur()
       	return m, nil
       ```
       And for `thaw <filename>`:
       ```go
       if strings.HasPrefix(trimmed, "thaw ") {
       	path := strings.TrimSpace(text[5:])
       	loaded, err := engine.LoadGame(path)
       	if err != nil {
       		m.CommandBar.AddMessage(fmt.Sprintf("Failed to thaw %s: %v", path, err))
       	} else {
       		m.Game = loaded
       		m.SelectedSector = engine.Coord{}
       		m.CommandBar.AddMessage(fmt.Sprintf("Mission thawed: %s (Stardate %.1f)", filepath.Base(path), loaded.Stardate))
       	}
       	return m, nil
       }
       ```

4. In `pkg/tui/components/commandpalette/palette.go`:
   - In `defaultCatalog()` add:
     ```go
     		{
     			title:         "SAVES",
     			desc:          "Browse, inspect, and load saved missions (Ctrl+O)",
     			prefix:        "saves",
     			parameterized: false,
     		},
     ```

- [ ] **Step 4: Run test to verify it passes**

Run: `go test -v -race ./pkg/tui/...`  
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add pkg/tui/model.go pkg/tui/view.go pkg/tui/update.go pkg/tui/components/commandpalette/palette.go pkg/tui/model_test.go
git commit -m "feat(tui): integrate save game browser modal, Ctrl+O hotkey, and thaw command"
```

---

### Task 5: Distribution Packaging & Cross-Compilation

**Files:**
- Create: `.goreleaser.yaml`
- Modify: `CMakeLists.txt`
- Test: Verification of cross-compilation & CPack targets

**Interfaces:**
- Consumes: Go project root `cmd/sst/main.go`, `sst.doc`, `README.md`, CMake C target `sst`
- Produces:
  - `.goreleaser.yaml` configuring GoReleaser v2 pure Go static binary cross-compilation across Linux (`amd64`, `arm64`), macOS (`amd64`, `arm64`), and Windows (`amd64`).
  - CMake CPack configuration in `CMakeLists.txt` for `TGZ` and `DEB` distributions.

- [ ] **Step 1: Write `.goreleaser.yaml`**

Create `.goreleaser.yaml` in the repo root:

```yaml
version: 2

project_name: sst

before:
  hooks:
    - go mod tidy

builds:
  - id: sst
    main: ./cmd/sst
    binary: sst
    env:
      - CGO_ENABLED=0
    goos:
      - linux
      - darwin
      - windows
    goarch:
      - amd64
      - arm64
    ignore:
      - goos: windows
        goarch: arm64
    ldflags:
      - -s -w

archives:
  - id: sst-archive
    builds:
      - sst
    name_template: "{{ .ProjectName }}_{{ .Version }}_{{ .Os }}_{{ .Arch }}"
    format: tar.gz
    format_overrides:
      - goos: windows
        format: zip
    files:
      - sst.doc
      - README.md
      - docs/**/*

checksum:
  name_template: "checksums.txt"
  algorithm: sha256

changelog:
  sort: asc
  filters:
    exclude:
      - "^docs:"
      - "^test:"
```

- [ ] **Step 2: Update `CMakeLists.txt` with CPack packaging**

Append installation targets and CPack configuration to `CMakeLists.txt`:

```cmake
install(TARGETS sst DESTINATION bin)
install(FILES sst.doc DESTINATION share/doc/super-star-trek)

set(CPACK_PACKAGE_NAME "super-star-trek")
set(CPACK_PACKAGE_VENDOR "Super Star Trek Authors")
set(CPACK_PACKAGE_DESCRIPTION_SUMMARY "Classic terminal space-strategy game written in C17")
set(CPACK_PACKAGE_VERSION "${PROJECT_VERSION}")
set(CPACK_GENERATOR "TGZ;DEB")
set(CPACK_DEBIAN_PACKAGE_MAINTAINER "Super Star Trek Authors")
set(CPACK_DEBIAN_PACKAGE_SECTION "games")
include(CPack)
```

- [ ] **Step 3: Run cross-compilation matrix verification**

Verify pure Go static compilation across all 5 target platforms:

```bash
GOOS=linux GOARCH=amd64 CGO_ENABLED=0 go build -o /dev/null ./cmd/sst
GOOS=linux GOARCH=arm64 CGO_ENABLED=0 go build -o /dev/null ./cmd/sst
GOOS=darwin GOARCH=amd64 CGO_ENABLED=0 go build -o /dev/null ./cmd/sst
GOOS=darwin GOARCH=arm64 CGO_ENABLED=0 go build -o /dev/null ./cmd/sst
GOOS=windows GOARCH=amd64 CGO_ENABLED=0 go build -o /dev/null ./cmd/sst
```
Expected: All 5 commands exit 0 without compilation errors.

- [ ] **Step 4: Run full test gates**

Run:
```bash
go test -v -race ./...
cmake --preset debug
cmake --build --preset debug
ctest --preset debug
```
Expected:
- All Go tests pass with 0 failures and 0 race conditions.
- All C tests pass.

- [ ] **Step 5: Commit**

```bash
git add .goreleaser.yaml CMakeLists.txt
git commit -m "build(dist): configure GoReleaser multi-platform builds and CMake CPack packaging"
```

---
