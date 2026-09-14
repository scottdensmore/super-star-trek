# Authentic Scoring Engine, Starfleet Ranks & Hall of Fame Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the authentic 15-rule scoring engine matching classic `rules.c:score_compute`, Starfleet rank tiers, persistent XDG JSON leaderboard storage, and an interactive dual-tab TUI Hall of Fame modal (`pkg/tui/components/halloffame`).

**Architecture:** Add `GameMetrics` and `ComputeScore` in `pkg/engine/score.go`, persistent leaderboard management in `pkg/engine/leaderboard.go`, a 66x18 dual-tab Bubble Tea modal in `pkg/tui/components/halloffame`, and integrate it into root `pkg/tui` with hotkey `H`, commands `score`/`scores`/`halloffame`, and automatic game-over display with callsign entry.

**Tech Stack:** Go 1.26+, Charmbracelet Bubble Tea, Lip Gloss, Bubbles (`textinput`), standard Go `testing`.

**Spec:** `docs/superpowers/specs/2026-09-13-scoring-and-hall-of-fame-design.md`

## Global Constraints
- Target Go version: Go 1.26+ standard library and Charmbracelet packages (`bubbletea`, `lipgloss`, `bubbles`)
- Terminal layout constraint: Strict 80 columns x 24 lines dimension budget for full TUI view
- Modal layout constraint: Exact 66 columns wide x 18 rows high dimension budget
- Zero compiled binaries or temporary files committed to git
- Maintain 100% passing tests for Go (`go test -v -race ./...`) and C (`ctest --preset debug`, `tests/golden.sh`)
- Work committed on feature branch `scottdensmore/feat/scoring-and-hall-of-fame`

---

### Task 1: Engine Metric Tracking & 15-Rule Scoring Math (pkg/engine)

**Files:**
- Modify: `pkg/engine/state.go`
- Modify: `pkg/engine/combat.go`
- Modify: `pkg/engine/actions.go`
- Create: `pkg/engine/score.go`
- Test: `pkg/engine/score_test.go`

**Interfaces:**
- Produces:
  - `type GameMetrics struct`
  - `type ScoreBreakdown struct`
  - `func ComputeScore(g *GameState, won bool) ScoreBreakdown`
  - `func RankForScore(score int) (title string, badge string)`

- [ ] **Step 1: Write the failing test**

Create `pkg/engine/score_test.go`:
```go
package engine

import (
	"testing"
)

func TestScore_CalculationMatchingClassicRules(t *testing.T) {
	g := NewGame(12345, SkillGood, LengthMedium)
	g.InitialStardate = 2000.0
	g.Stardate = 2010.0 // 10 stardates elapsed

	g.Metrics.KlingonsKilled = 8        // 8 * 10 = 80
	g.Metrics.CommandersKilled = 2      // 2 * 50 = 100
	g.Metrics.SuperCommandersKilled = 1 // 1 * 200 = 200
	g.Metrics.RomulansKilled = 1        // 1 * 20 = 20
	g.Metrics.RomulansSurrendered = 2   // 2 * 1 = 2
	g.Metrics.StarbasesDestroyed = 1    // -100
	g.Metrics.StarsDestroyed = 1        // -5
	g.Metrics.PlanetsDestroyed = 1      // -10
	g.Metrics.Casualties = 15           // -15
	g.Metrics.HelpCalls = 1             // -45
	g.Metrics.StarshipsLost = 0         // 0

	// 11 total kills / 10 stardates = 1.1 kill rate
	// 500 * 1.1 = 550 kill rate points
	// Won game on SkillGood (3) -> 100 * 3 = 300 win bonus
	// Total expected:
	// + 80 + 100 + 200 + 20 + 2 + 550 + 300 - 100 - 5 - 10 - 15 - 45 = 1077
	score := ComputeScore(g, true)

	if score.KlingonPoints != 80 {
		t.Errorf("expected 80 Klingon points, got %d", score.KlingonPoints)
	}
	if score.CommanderPoints != 100 {
		t.Errorf("expected 100 Commander points, got %d", score.CommanderPoints)
	}
	if score.SuperCommanderPoints != 200 {
		t.Errorf("expected 200 Super-Commander points, got %d", score.SuperCommanderPoints)
	}
	if score.KillRatePoints != 550 {
		t.Errorf("expected 550 kill rate points, got %d (kill rate: %.2f)", score.KillRatePoints, score.KillRate)
	}
	if score.WinBonus != 300 {
		t.Errorf("expected 300 win bonus, got %d", score.WinBonus)
	}
	if score.TotalScore != 1077 {
		t.Errorf("expected 1077 total score, got %d", score.TotalScore)
	}
	if score.RankBadge != "[FADM]" {
		t.Errorf("expected [FADM] rank badge for score 1077, got %s", score.RankBadge)
	}
}

func TestScore_MinimumFiveStardatesClampingWhenLost(t *testing.T) {
	g := NewGame(12345, SkillGood, LengthMedium)
	g.InitialStardate = 2000.0
	g.Stardate = 2001.0 // Only 1 stardate elapsed, but game lost!

	g.Metrics.KlingonsKilled = 2 // 2 kills
	// Elapsed clamped to 5.0 -> kill rate = 2 / 5.0 = 0.4
	// 500 * 0.4 = 200 kill rate points
	// Lost -> 0 win bonus
	// Total: 20 + 200 = 220
	score := ComputeScore(g, false)

	if score.ElapsedStardates != 5.0 {
		t.Errorf("expected elapsed clamped to 5.0, got %.1f", score.ElapsedStardates)
	}
	if score.KillRatePoints != 200 {
		t.Errorf("expected 200 kill rate points, got %d", score.KillRatePoints)
	}
	if score.WinBonus != 0 {
		t.Errorf("expected 0 win bonus when game lost, got %d", score.WinBonus)
	}
	if score.TotalScore != 220 {
		t.Errorf("expected 220 total score, got %d", score.TotalScore)
	}
	if score.RankBadge != "[CDR]" {
		t.Errorf("expected [CDR] for score 220, got %s", score.RankBadge)
	}
}

func TestScore_Ranks(t *testing.T) {
	tests := []struct {
		score int
		badge string
		title string
	}{
		{-10, "[DISHONOR]", "Dishonorable Discharge"},
		{50, "[CADET]", "Starfleet Cadet"},
		{150, "[LT]", "Lieutenant"},
		{250, "[CDR]", "Commander"},
		{400, "[CAPT]", "Captain"},
		{600, "[COMM]", "Commodore"},
		{800, "[RADM]", "Rear Admiral"},
		{1200, "[FADM]", "Fleet Admiral"},
	}

	for _, tt := range tests {
		title, badge := RankForScore(tt.score)
		if badge != tt.badge || title != tt.title {
			t.Errorf("score %d: expected (%s, %s), got (%s, %s)", tt.score, tt.title, tt.badge, title, badge)
		}
	}
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `go test -v ./pkg/engine -run TestScore_`
Expected: FAIL (`ComputeScore` and `RankForScore` undefined)

- [ ] **Step 3: Implement minimal code**

1. In `pkg/engine/state.go`:
   - Add `GameMetrics` struct.
   - Add `Metrics GameMetrics` and `GameWon bool` to `GameState`.
2. In `pkg/engine/score.go`:
   Implement `ScoreBreakdown`, `ComputeScore`, and `RankForScore`.
3. In `pkg/engine/combat.go`:
   When Klingons are killed in torpedo or phaser resolution, increment `g.Metrics.KlingonsKilled` (or `CommandersKilled`).
4. In `pkg/engine/actions.go`:
   Increment `g.Metrics.HelpCalls` in `ActionCallHelp` or starbase/star destruction counters.

- [ ] **Step 4: Run test to verify it passes**

Run: `go test -v ./pkg/engine -run TestScore_`
Expected: PASS

Run full engine tests:
`go test -v ./pkg/engine`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add pkg/engine/state.go pkg/engine/combat.go pkg/engine/actions.go pkg/engine/score.go pkg/engine/score_test.go
git commit -m "feat(engine): implement metric tracking, 15-rule scoring math, and Starfleet ranks"
```

---

### Task 2: Leaderboard Persistence, XDG Resolution & Default Legends (pkg/engine)

**Files:**
- Create: `pkg/engine/leaderboard.go`
- Test: `pkg/engine/leaderboard_test.go`

**Interfaces:**
- Produces:
  - `type ScoreEntry struct`
  - `type Leaderboard struct`
  - `func DefaultLeaderboard() *Leaderboard`
  - `func DefaultLeaderboardPath() string`
  - `func LoadLeaderboard(path string) (*Leaderboard, error)`
  - `func (lb *Leaderboard) Qualifies(score int) bool`
  - `func (lb *Leaderboard) Add(entry ScoreEntry)`
  - `func (lb *Leaderboard) Save(path string) error`

- [ ] **Step 1: Write the failing test**

Create `pkg/engine/leaderboard_test.go`:
```go
package engine

import (
	"os"
	"path/filepath"
	"testing"
	"time"
)

func TestLeaderboard_DefaultSeedAndQualifies(t *testing.T) {
	lb := DefaultLeaderboard()
	if len(lb.Entries) == 0 {
		t.Fatalf("expected non-empty default leaderboard")
	}
	if len(lb.Entries) > 10 {
		t.Fatalf("expected at most 10 default entries, got %d", len(lb.Entries))
	}

	lowestScore := lb.Entries[len(lb.Entries)-1].Score
	if !lb.Qualifies(lowestScore + 10) {
		t.Errorf("expected score %d to qualify against lowest %d", lowestScore+10, lowestScore)
	}
	if lb.Qualifies(lowestScore - 50) && len(lb.Entries) >= 10 {
		t.Errorf("expected score %d not to qualify when table is full", lowestScore-50)
	}
}

func TestLeaderboard_SaveAndLoadAtomic(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "highscores.json")

	lb := &Leaderboard{
		Entries: []ScoreEntry{
			{CaptainName: "Kirk", Score: 1180, Rank: "[FADM]", Date: time.Now()},
			{CaptainName: "Spock", Score: 980, Rank: "[RADM]", Date: time.Now()},
		},
	}

	if err := lb.Save(path); err != nil {
		t.Fatalf("failed to save leaderboard: %v", err)
	}

	loaded, err := LoadLeaderboard(path)
	if err != nil {
		t.Fatalf("failed to load leaderboard: %v", err)
	}

	if len(loaded.Entries) != 2 {
		t.Fatalf("expected 2 entries, got %d", len(loaded.Entries))
	}
	if loaded.Entries[0].CaptainName != "Kirk" || loaded.Entries[0].Score != 1180 {
		t.Errorf("unexpected top entry: %+v", loaded.Entries[0])
	}
}

func TestLeaderboard_AddAndTruncateTop10(t *testing.T) {
	lb := DefaultLeaderboard()

	entry := ScoreEntry{
		CaptainName: "Picard",
		Score:       2000,
		Rank:        "[FADM]",
		Date:        time.Now(),
	}

	lb.Add(entry)

	if len(lb.Entries) > 10 {
		t.Errorf("expected max 10 entries after add, got %d", len(lb.Entries))
	}
	if lb.Entries[0].CaptainName != "Picard" || lb.Entries[0].Score != 2000 {
		t.Errorf("expected Picard to be #1, got %+v", lb.Entries[0])
	}
}

func TestLeaderboard_CorruptedFileRecovery(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "corrupt.json")

	if err := os.WriteFile(path, []byte("{ invalid json "), 0644); err != nil {
		t.Fatalf("failed to write corrupt file: %v", err)
	}

	lb, err := LoadLeaderboard(path)
	if err != nil {
		t.Fatalf("expected clean fallback on corrupt file, got err: %v", err)
	}
	if len(lb.Entries) == 0 {
		t.Errorf("expected default entries on corrupt fallback")
	}

	// Corrupted file should be backed up
	if _, err := os.Stat(path + ".corrupt"); err != nil {
		t.Errorf("expected .corrupt backup file to exist")
	}
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `go test -v ./pkg/engine -run TestLeaderboard_`
Expected: FAIL (`DefaultLeaderboard` and methods undefined)

- [ ] **Step 3: Implement minimal code**

Create `pkg/engine/leaderboard.go`:
- Implement `ScoreEntry` and `Leaderboard`.
- Implement `DefaultLeaderboard()` with classic Starfleet legends.
- Implement `DefaultLeaderboardPath()` using `os.UserConfigDir()` with fallback.
- Implement `LoadLeaderboard()`, `Qualifies()`, `Add()`, and `Save()` with atomic write pattern.

- [ ] **Step 4: Run test to verify it passes**

Run: `go test -v ./pkg/engine -run TestLeaderboard_`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add pkg/engine/leaderboard.go pkg/engine/leaderboard_test.go
git commit -m "feat(engine): implement leaderboard persistence, atomic storage, and default legends"
```

---

### Task 3: Dual-Tab Hall of Fame TUI Component (pkg/tui/components/halloffame)

**Files:**
- Create: `pkg/tui/components/halloffame/halloffame.go`
- Test: `pkg/tui/components/halloffame/halloffame_test.go`

**Interfaces:**
- Consumes: `engine.ScoreBreakdown`, `engine.Leaderboard`, `engine.ScoreEntry`, `theme.Theme`
- Produces:
  - `type Model struct`
  - `type CloseModalMsg struct{}`
  - `type ScoreRecordedMsg struct{ Entry engine.ScoreEntry }`
  - `func New(th theme.Theme, width, height int, storagePath string) Model`
  - `func (m *Model) SetState(score engine.ScoreBreakdown, lb *engine.Leaderboard, promptName bool)`
  - `func (m Model) Update(msg tea.Msg) (Model, tea.Cmd)`
  - `func (m Model) View() string`

- [ ] **Step 1: Write the failing test**

Create `pkg/tui/components/halloffame/halloffame_test.go`:
```go
package halloffame

import (
	"strings"
	"testing"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/x/ansi"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

func TestHallOfFame_DimensionsAndTabSwitching(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th, 66, 18, "")

	score := engine.ScoreBreakdown{
		KlingonsKilled:   5,
		KlingonPoints:    50,
		KillRate:         1.0,
		KillRatePoints:   500,
		TotalScore:       550,
		RankBadge:        "[COMM]",
		RankTitle:        "Commodore",
		ElapsedStardates: 5.0,
	}
	lb := engine.DefaultLeaderboard()

	m.SetState(score, lb, false)

	// Tab 1: Mission Telemetry
	view1 := m.View()
	lines1 := strings.Split(view1, "\n")
	if len(lines1) != 18 {
		t.Fatalf("expected 18 lines on Tab 1, got %d", len(lines1))
	}
	for i, l := range lines1 {
		if w := ansi.StringWidth(l); w != 66 {
			t.Errorf("Tab 1 line %d width = %d, expected 66", i, w)
		}
	}
	if !strings.Contains(view1, "MISSION DEBRIEF") || !strings.Contains(view1, "CURRENT NET SCORE") {
		t.Errorf("expected mission telemetry contents in Tab 1")
	}

	// Switch to Tab 2 via Tab key
	tabMsg := tea.KeyMsg{Type: tea.KeyTab}
	updated, _ := m.Update(tabMsg)
	mTab2 := updated

	view2 := mTab2.View()
	lines2 := strings.Split(view2, "\n")
	if len(lines2) != 18 {
		t.Fatalf("expected 18 lines on Tab 2, got %d", len(lines2))
	}
	for i, l := range lines2 {
		if w := ansi.StringWidth(l); w != 66 {
			t.Errorf("Tab 2 line %d width = %d, expected 66", i, w)
		}
	}
	if !strings.Contains(view2, "HALL OF FAME") || !strings.Contains(view2, "James T. Kirk") {
		t.Errorf("expected leaderboard table in Tab 2")
	}
}

func TestHallOfFame_DismissalKeys(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th, 66, 18, "")

	for _, k := range []string{"esc", "q", "h"} {
		var keyMsg tea.KeyMsg
		if k == "esc" {
			keyMsg = tea.KeyMsg{Type: tea.KeyEsc}
		} else {
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

Run: `go test -v ./pkg/tui/components/halloffame`
Expected: FAIL (package does not exist)

- [ ] **Step 3: Implement minimal code**

Create `pkg/tui/components/halloffame/halloffame.go`:
- Construct `Model` with width (66), height (18), active tab (`tabTelemetry` = 0, `tabLeaderboard` = 1), textinput for name entry.
- Implement `SetState`, `Update`, `View` formatting Tab 1 and Tab 2 within exact 66x18 borders.

- [ ] **Step 4: Run test to verify it passes**

Run: `go test -v ./pkg/tui/components/halloffame`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add pkg/tui/components/halloffame/halloffame.go pkg/tui/components/halloffame/halloffame_test.go
git commit -m "feat(halloffame): create dual-tab Hall of Fame and telemetry modal component"
```

---

### Task 4: Root TUI Integration, Commands, Hotkeys & Auto Game-Over Flow (pkg/tui)

**Files:**
- Modify: `pkg/tui/model.go`
- Modify: `pkg/tui/update.go`
- Modify: `pkg/tui/view.go`
- Test: `pkg/tui/model_test.go`

**Interfaces:**
- Produces:
  - `ModalHallOfFame` in `ModalType`
  - `HallOfFame halloffame.Model` on `tui.Model`
  - `m.openHallOfFame(promptName bool) (Model, tea.Cmd)`

- [ ] **Step 1: Write the failing test**

Add to `pkg/tui/model_test.go`:
```go
func TestModel_HallOfFame_OpenAndDismiss(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	mod := NewModel(g, theme.DefaultTheme())

	// 1. Open via command "score"
	updated, _ := mod.handleCommand("score")
	modScore := updated.(Model)
	if modScore.ActiveModal != ModalHallOfFame {
		t.Fatalf("expected ActiveModal = ModalHallOfFame on 'score', got %v", modScore.ActiveModal)
	}

	// 2. Dimensions = 80x24
	view := modScore.View()
	lines := strings.Split(view, "\n")
	if len(lines) != 24 {
		t.Errorf("expected 24 lines, got %d", len(lines))
	}
	for i, l := range lines {
		if w := ansi.StringWidth(l); w != 80 {
			t.Errorf("line %d width = %d, expected 80", i, w)
		}
	}

	// 3. Dismiss via 'h' key
	hKey := tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune("h")}
	updatedAfterH, _ := modScore.Update(hKey)
	modClosed := updatedAfterH.(Model)
	if modClosed.ActiveModal != ModalNone {
		t.Errorf("expected ActiveModal = ModalNone after 'h' dismiss, got %v", modClosed.ActiveModal)
	}
}

func TestModel_HallOfFame_Hotkeys(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	mod := NewModel(g, theme.DefaultTheme())

	hKey := tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune("h")}
	updated, _ := mod.Update(hKey)
	modH := updated.(Model)
	if modH.ActiveModal != ModalHallOfFame {
		t.Fatalf("expected ActiveModal = ModalHallOfFame on 'h' hotkey, got %v", modH.ActiveModal)
	}
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `go test -v ./pkg/tui -run TestModel_HallOfFame_`
Expected: FAIL (`ModalHallOfFame` undefined)

- [ ] **Step 3: Implement minimal code**

1. In `pkg/tui/model.go`:
   - Add `ModalHallOfFame` to `ModalType`.
   - Add `HallOfFame halloffame.Model` to `Model` struct.
   - Initialize in `NewModel` via `halloffame.New(th, 66, 18, engine.DefaultLeaderboardPath())`.
2. In `pkg/tui/update.go`:
   - In `applyTheme`: update `m.HallOfFame.SetTheme(th)`.
   - Add `openHallOfFame(promptName bool) (Model, tea.Cmd)`.
   - In `handleCommand`: map `"score", "scores", "halloffame", "hof"` to `m.openHallOfFame(false)`.
   - In `Update(msg)`:
     - Handle `halloffame.CloseModalMsg` -> `m.ActiveModal = ModalNone` and focus `CommandBar`.
     - When `m.ActiveModal == ModalHallOfFame`: delegate to `m.HallOfFame.Update(msg)`.
     - Hotkey: when command line is empty, `'h'`, `'H'`, `'ctrl+h'` calls `m.openHallOfFame(false)`.
     - On game over event: trigger `m.openHallOfFame(qualifies)`.
3. In `pkg/tui/view.go`:
   - In `renderDashboard`: map `case ModalHallOfFame: modalView = m.HallOfFame.View()`.

- [ ] **Step 4: Run test to verify it passes**

Run: `go test -v ./pkg/tui -run TestModel_HallOfFame_`
Expected: PASS

Run full TUI tests:
`go test -v ./pkg/tui/...`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add pkg/tui/model.go pkg/tui/update.go pkg/tui/view.go pkg/tui/model_test.go
git commit -m "feat(tui): integrate Hall of Fame modal, score commands, hotkeys, and game-over flow"
```

---

### Task 5: Golden Snapshot Visual Regression & Full Verification (pkg/tui, test suites)

**Files:**
- Modify: `tests/tui_golden_test.go`
- Create: `tests/golden/tui/modal_hall_of_fame_telemetry.golden`
- Create: `tests/golden/tui/modal_hall_of_fame_leaderboard.golden`

**Interfaces:**
- Validates golden snapshot parity across both tabs of `ModalHallOfFame`.

- [ ] **Step 1: Write the failing test**

In `tests/tui_golden_test.go`:
```go
func TestTUIGolden_ModalHallOfFame(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	g.Metrics.KlingonsKilled = 8
	g.Metrics.CommandersKilled = 2
	g.Metrics.Casualties = 12

	m := tui.NewModel(g, theme.ModernTheme{})
	updated, _ := m.Update(tea.WindowSizeMsg{Width: 80, Height: 24})
	m = updated.(tui.Model)

	// Tab 1: Telemetry
	updatedModal, _ := m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune("h")})
	m = updatedModal.(tui.Model)
	compareOrUpdate(t, "modal_hall_of_fame_telemetry", m.View())

	// Tab 2: Leaderboard
	updatedTab2, _ := m.Update(tea.KeyMsg{Type: tea.KeyTab})
	m = updatedTab2.(tui.Model)
	compareOrUpdate(t, "modal_hall_of_fame_leaderboard", m.View())
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `go test -v ./tests -run TestTUIGolden_ModalHallOfFame`
Expected: FAIL (golden file not found)

- [ ] **Step 3: Generate snapshots and verify**

Run: `go test -v ./tests -run TestTUIGolden_ModalHallOfFame -update`
Expected: PASS

Run without `-update`:
`go test -v ./tests -run TestTUIGolden_ModalHallOfFame`
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
git add tests/tui_golden_test.go tests/golden/tui/modal_hall_of_fame_*.golden
git commit -m "test(tui): add golden snapshot tests for Hall of Fame modal tabs"
```
