# Curated Tactical Scenarios & Challenge Modes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement Subsystem 2 of Issue #247: Curated Tactical Scenarios & Challenge Modes in Super Star Trek, delivering *Kobayashi Maru*, *Mutara Nebula*, and *Starbase Under Siege* with custom rules, wave mechanics, dedicated leaderboards, CLI/WASM commands, and an interactive TUI Scenario Browser modal.

**Architecture:** A modular `Scenario` registry in `pkg/engine/scenarios.go` encapsulates deterministic state factories (`Build`), turn condition evaluators (`Evaluate`), and custom scoring (`ComputeScore`). The engine turn pipeline hooks into `Evaluate` when `g.Scenario != ScenarioNone`. Isolated scenario leaderboards persist to `leaderboard_<scenario>.json`. Players can launch scenarios via CLI flags (`--scenario`), WASM commands (`scenario <id>`), or the TUI Scenario Browser modal (`Ctrl+P`).

**Tech Stack:** Go 1.26+, Bubbletea, Lipgloss, xterm.js (WASM), standard library (`math`, `encoding/json`, `errors`).

**Spec:** `docs/superpowers/specs/2026-09-18-tactical-scenarios-design.md`

## Global Constraints

- Target branch: `scottdensmore/feat/tactical-scenarios`
- Target Go version: Go 1.26+ standard library
- WebAssembly compilation target: `GOOS=js GOARCH=wasm`
- Zero compiled binaries (e.g. `sst.wasm`, `sst`) committed to git
- Maintain 100% passing tests across Go (`go test -v -race ./...`) and C (`ctest --preset debug`, `tests/golden.sh`)
- 100% backward compatibility when `g.Scenario == ScenarioNone`

---

### Task 1: Scenario Domain Models, Registry, & GameState Integration

**Files:**
- Create: `pkg/engine/scenarios.go`
- Create: `pkg/engine/scenarios_test.go`
- Modify: `pkg/engine/state.go:110-130`
- Modify: `pkg/engine/save_test.go:1-80`

**Interfaces:**
- Produces:
  ```go
  type ScenarioID string
  const (
      ScenarioNone          ScenarioID = ""
      ScenarioKobayashiMaru ScenarioID = "kobayashi-maru"
      ScenarioMutaraNebula  ScenarioID = "mutara-nebula"
      ScenarioStarbaseSiege ScenarioID = "starbase-siege"
  )
  type Scenario struct {
      ID          ScenarioID `json:"id"`
      Name        string     `json:"name"`
      Subtitle    string     `json:"subtitle"`
      Difficulty  string     `json:"difficulty"`
      Description string     `json:"description"`
      Briefing    []string   `json:"briefing"`
      Build       func(seed int64) *GameState `json:"-"`
      Evaluate    func(g *GameState) (done bool, won bool, reason GameOverReason) `json:"-"`
      ComputeScore func(g *GameState, won bool) ScoreBreakdown `json:"-"`
  }
  func GetScenario(id ScenarioID) (*Scenario, bool)
  func ListScenarios() []*Scenario
  func RegisterScenario(s *Scenario)
  ```
  In `GameState`: `Scenario ScenarioID json:"scenario,omitempty"`

- [ ] **Step 1: Write failing tests in `pkg/engine/scenarios_test.go`**

```go
package engine

import "testing"

func TestScenarioRegistry_LookupAndAliases(t *testing.T) {
	cases := []struct {
		input    string
		expected ScenarioID
		found    bool
	}{
		{"kobayashi-maru", ScenarioKobayashiMaru, true},
		{"kobayashi", ScenarioKobayashiMaru, true},
		{"km", ScenarioKobayashiMaru, true},
		{"mutara-nebula", ScenarioMutaraNebula, true},
		{"mutara", ScenarioMutaraNebula, true},
		{"nebula", ScenarioMutaraNebula, true},
		{"starbase-siege", ScenarioStarbaseSiege, true},
		{"siege", ScenarioStarbaseSiege, true},
		{"starbase", ScenarioStarbaseSiege, true},
		{"unknown-scenario", "", false},
	}

	for _, tc := range cases {
		s, ok := GetScenario(ScenarioID(tc.input))
		if ok != tc.found {
			t.Errorf("GetScenario(%q): expected found=%v, got %v", tc.input, tc.found, ok)
		}
		if ok && s.ID != tc.expected {
			t.Errorf("GetScenario(%q): expected ID=%v, got %v", tc.input, tc.expected, s.ID)
		}
	}
}

func TestListScenarios_Ordering(t *testing.T) {
	list := ListScenarios()
	if len(list) < 3 {
		t.Fatalf("expected at least 3 registered scenarios, got %d", len(list))
	}
	expected := []ScenarioID{ScenarioKobayashiMaru, ScenarioMutaraNebula, ScenarioStarbaseSiege}
	for i, expID := range expected {
		if list[i].ID != expID {
			t.Errorf("scenario index %d: expected %v, got %v", i, expID, list[i].ID)
		}
	}
}
```

- [ ] **Step 2: Run tests to verify failure**

Run: `go test -v ./pkg/engine -run "TestScenarioRegistry|TestListScenarios"`
Expected: FAIL (types and functions not defined)

- [ ] **Step 3: Implement domain models and registry in `pkg/engine/scenarios.go` and `pkg/engine/state.go`**

In `pkg/engine/state.go`:
```go
type GameState struct {
	// ... existing fields ...
	Scenario           ScenarioID `json:"scenario,omitempty"`
	// ...
}
```

In `pkg/engine/scenarios.go`:
Implement `ScenarioID`, `Scenario`, registry map with alias normalization, `GetScenario`, `ListScenarios`, and placeholder stub registrations for the 3 scenarios.

- [ ] **Step 4: Run tests to verify pass**

Run: `go test -v ./pkg/engine -run "TestScenarioRegistry|TestListScenarios"`
Expected: PASS

- [ ] **Step 5: Verify Save/Load persistence of Scenario field in `pkg/engine/save_test.go`**

Add test verifying `g.Scenario = ScenarioMutaraNebula` survives save/load roundtrip.
Run: `go test -v ./pkg/engine -run "TestScenario|TestSave"`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add pkg/engine/scenarios.go pkg/engine/scenarios_test.go pkg/engine/state.go pkg/engine/save_test.go
git commit -m "feat(engine): add scenario domain models, registry, and state integration"
```

---

### Task 2: Flagship Scenario Implementations (Kobayashi Maru, Mutara Nebula, Starbase Siege)

**Files:**
- Create: `pkg/engine/scenario_kobayashi.go`
- Create: `pkg/engine/scenario_mutara.go`
- Create: `pkg/engine/scenario_siege.go`
- Modify: `pkg/engine/scenarios.go`
- Modify: `pkg/engine/scenarios_test.go`
- Modify: `pkg/engine/actions.go:430-550,750-800`

**Interfaces:**
- Consumes: `Scenario`, `ScenarioID`, `GameState`, `QuadrantEnv`, `GameOverReason`
- Produces:
  Complete `Build`, `Evaluate`, and `ComputeScore` implementations for:
  - `ScenarioKobayashiMaru`: Enterprise at `[4, 4]`, 3 battlecruisers, wave reinforcement spawns, Starfleet Commendations `[COMM-1]` to `[COMM-4]`.
  - `ScenarioMutaraNebula`: Enterprise at `[5, 5]` (`EnvNebula`), 0 shields, cloaked Super-Commander, victory on kill.
  - `ScenarioStarbaseSiege`: Starbase 12 in `[4, 4]`, Enterprise in `[2, 2]`, 6.0 stardate limit, loss if Starbase 12 is destroyed.
  Hook in `ActionMove.Execute` calling `scenario.Evaluate(g)` after turn advancement.

- [ ] **Step 1: Write failing tests in `pkg/engine/scenarios_test.go`**

```go
func TestScenario_KobayashiMaru_Mechanics(t *testing.T) {
	s, ok := GetScenario(ScenarioKobayashiMaru)
	if !ok {
		t.Fatalf("failed to find Kobayashi Maru scenario")
	}

	game := s.Build(42)
	if game.Scenario != ScenarioKobayashiMaru {
		t.Errorf("expected scenario ID %v, got %v", ScenarioKobayashiMaru, game.Scenario)
	}
	if game.Enterprise.Quad != (Coord{4, 4}) {
		t.Errorf("expected Enterprise in Quad [4, 4], got %v", game.Enterprise.Quad)
	}
	if len(game.CurrentQuad.Klingons) != 3 {
		t.Errorf("expected 3 Klingons initially, got %d", len(game.CurrentQuad.Klingons))
	}

	// Test Commendation scoring on loss
	game.Metrics.KlingonsKilled = 4
	game.Stardate = game.InitialStardate + 3.0
	score := s.ComputeScore(game, false)
	if score.RankTitle != "[COMM-2] Commendation for Tactical Excellence" {
		t.Errorf("expected [COMM-2], got %s", score.RankTitle)
	}
}

func TestScenario_MutaraNebula_Mechanics(t *testing.T) {
	s, ok := GetScenario(ScenarioMutaraNebula)
	if !ok {
		t.Fatalf("failed to find Mutara Nebula scenario")
	}

	game := s.Build(42)
	if game.Enterprise.Quad != (Coord{5, 5}) {
		t.Errorf("expected Enterprise at [5, 5], got %v", game.Enterprise.Quad)
	}
	if game.QuadrantEnv[5][5] != EnvNebula {
		t.Errorf("expected EnvNebula at [5, 5], got %v", game.QuadrantEnv[5][5])
	}
	if game.Enterprise.Shields != 0 {
		t.Errorf("expected 0 shields in Mutara Nebula, got %f", game.Enterprise.Shields)
	}

	// Kill the Super-Commander -> Evaluate should return won
	game.Metrics.SuperCommandersKilled = 1
	done, won, _ := s.Evaluate(game)
	if !done || !won {
		t.Errorf("expected victory when SuperCommander is killed, got done=%v, won=%v", done, won)
	}
}

func TestScenario_StarbaseSiege_Mechanics(t *testing.T) {
	s, ok := GetScenario(ScenarioStarbaseSiege)
	if !ok {
		t.Fatalf("failed to find Starbase Siege scenario")
	}

	game := s.Build(42)
	if game.Enterprise.Quad != (Coord{2, 2}) {
		t.Errorf("expected Enterprise at [2, 2], got %v", game.Enterprise.Quad)
	}
	if game.TimeRemaining != 6.0 {
		t.Errorf("expected 6.0 stardates time limit, got %f", game.TimeRemaining)
	}

	// If Starbase 12 in [4, 4] is destroyed, Evaluate must return loss
	game.RemainingStarbases = 0
	done, won, reason := s.Evaluate(game)
	if !done || won || reason != GameOverLost {
		t.Errorf("expected immediate loss when starbase is destroyed, got done=%v won=%v reason=%v", done, won, reason)
	}
}
```

- [ ] **Step 2: Run tests to verify failure**

Run: `go test -v ./pkg/engine -run "TestScenario_KobayashiMaru|TestScenario_MutaraNebula|TestScenario_StarbaseSiege"`
Expected: FAIL

- [ ] **Step 3: Implement the three scenarios and hook into turn evaluation**

1. Create `pkg/engine/scenario_kobayashi.go` with `Build`, `Evaluate` (wave reinforcement logic), and `ComputeScore` (assigning `[COMM-1]` to `[COMM-4]`).
2. Create `pkg/engine/scenario_mutara.go` with `Build`, `Evaluate` (victory on commander kill), and `ComputeScore`.
3. Create `pkg/engine/scenario_siege.go` with `Build`, `Evaluate` (bombardment and defeat on starbase loss), and `ComputeScore`.
4. In `pkg/engine/actions.go` (`ActionMove.Execute`):
   After moving and advancing stardates, if `g.Scenario != ScenarioNone`, invoke registered scenario's `Evaluate(g)`. If `done == true`, append `EventGameOver{Reason: reason, FinalScore: score}` and set `g.GameWon = won`.

- [ ] **Step 4: Run tests to verify pass**

Run: `go test -v ./pkg/engine -run "TestScenario_"`
Expected: PASS

- [ ] **Step 5: Run full engine test suite with race detection**

Run: `go test -v -race ./pkg/engine/...`
Expected: PASS with 0 race warnings

- [ ] **Step 6: Commit**

```bash
git add pkg/engine/scenario_*.go pkg/engine/scenarios.go pkg/engine/scenarios_test.go pkg/engine/actions.go
git commit -m "feat(engine): implement Kobayashi Maru, Mutara Nebula, and Starbase Siege scenarios"
```

---

### Task 3: Dedicated Scenario Leaderboards & Persistence

**Files:**
- Modify: `pkg/engine/leaderboard.go:10-90,130-220`
- Modify: `pkg/engine/leaderboard_test.go:1-120`

**Interfaces:**
- Consumes: `ScenarioID`, `ScoreEntry`, `Leaderboard`
- Produces:
  ```go
  func ScenarioLeaderboardPath(id ScenarioID) string
  func DefaultScenarioLeaderboard(id ScenarioID) *Leaderboard
  func LoadScenarioLeaderboard(id ScenarioID) (*Leaderboard, error)
  func SaveScenarioLeaderboard(id ScenarioID, lb *Leaderboard) error
  func AddScenarioScore(id ScenarioID, entry ScoreEntry) (int, bool, error)
  ```
  `ScoreEntry` fields `Scenario string json:"scenario,omitempty"` and `Commendation string json:"commendation,omitempty"`.

- [ ] **Step 1: Write failing tests in `pkg/engine/leaderboard_test.go`**

```go
func TestScenarioLeaderboards_Isolation(t *testing.T) {
	tempDir := t.TempDir()
	origHome := os.Getenv("HOME")
	os.Setenv("HOME", tempDir)
	defer os.Setenv("HOME", origHome)

	// Save score for Kobayashi Maru
	entry := ScoreEntry{
		CaptainName:  "Test Cadet",
		Score:        1200,
		Rank:         "[COMM-3]",
		Commendation: "Starfleet Cross of Honor",
		Date:         time.Now(),
	}
	rank, added, err := AddScenarioScore(ScenarioKobayashiMaru, entry)
	if err != nil || !added {
		t.Fatalf("failed to add scenario score: %v, added=%v", err, added)
	}
	if rank < 1 {
		t.Errorf("expected positive rank, got %d", rank)
	}

	// Verify standard leaderboard was NOT touched
	stdLB, err := LoadLeaderboard()
	if err != nil {
		t.Fatalf("unexpected error loading standard leaderboard: %v", err)
	}
	for _, e := range stdLB.Entries {
		if e.CaptainName == "Test Cadet" {
			t.Errorf("standard leaderboard should not contain scenario entry")
		}
	}

	// Verify scenario leaderboard contains entry
	scenLB, err := LoadScenarioLeaderboard(ScenarioKobayashiMaru)
	if err != nil {
		t.Fatalf("failed to load scenario leaderboard: %v", err)
	}
	found := false
	for _, e := range scenLB.Entries {
		if e.CaptainName == "Test Cadet" && e.Commendation == "Starfleet Cross of Honor" {
			found = true
			break
		}
	}
	if !found {
		t.Errorf("entry with commendation not found in scenario leaderboard")
	}
}
```

- [ ] **Step 2: Run tests to verify failure**

Run: `go test -v ./pkg/engine -run TestScenarioLeaderboards_Isolation`
Expected: FAIL (functions not defined)

- [ ] **Step 3: Implement dedicated scenario leaderboard persistence**

In `pkg/engine/leaderboard.go`:
1. Add `Scenario string json:"scenario,omitempty"` and `Commendation string json:"commendation,omitempty"` to `ScoreEntry`.
2. Implement `ScenarioLeaderboardPath(id ScenarioID) string`: writes to `filepath.Join(dir, fmt.Sprintf("leaderboard_%s.json", id))`.
3. Implement `DefaultScenarioLeaderboard(id ScenarioID)` pre-seeded with historical records (Kirk, Spock, Saavik for Kobayashi Maru; Kirk for Mutara; Sulu for Starbase Siege).
4. Implement `LoadScenarioLeaderboard`, `SaveScenarioLeaderboard`, and `AddScenarioScore`.

- [ ] **Step 4: Run tests to verify pass**

Run: `go test -v ./pkg/engine -run "TestScenarioLeaderboard|TestLeaderboard"`
Expected: PASS

- [ ] **Step 5: Run full engine test suite**

Run: `go test -v -race ./pkg/engine/...`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add pkg/engine/leaderboard.go pkg/engine/leaderboard_test.go
git commit -m "feat(engine): add dedicated scenario leaderboards with commendations"
```

---

### Task 4: CLI Flags & WebAssembly Terminal Scenario Commands

**Files:**
- Modify: `cmd/sst/main.go:35-125`
- Modify: `cmd/sst/main_test.go:300-400`
- Modify: `cmd/wasm/session.go:40-150`
- Modify: `cmd/wasm/session_test.go:100-180`

**Interfaces:**
- Consumes: `ScenarioID`, `GetScenario`, `ListScenarios`, `Session`
- Produces:
  CLI `--scenario <id>` (or `-s <id>`) and `--list-scenarios`
  WASM commands `scenario list` and `scenario <id>`

- [ ] **Step 1: Write failing tests in `cmd/sst/main_test.go` and `cmd/wasm/session_test.go`**

In `cmd/sst/main_test.go`:
```go
func TestCLI_ListScenarios(t *testing.T) {
	out := &bytes.Buffer{}
	errOut := &bytes.Buffer{}
	code := run([]string{"--list-scenarios"}, strings.NewReader(""), out, errOut)
	if code != 0 {
		t.Errorf("expected exit code 0, got %d", code)
	}
	output := out.String()
	if !strings.Contains(output, "kobayashi-maru") || !strings.Contains(output, "mutara-nebula") {
		t.Errorf("expected scenario list in output, got: %s", output)
	}
}

func TestCLI_ScenarioLaunch_Unknown(t *testing.T) {
	out := &bytes.Buffer{}
	errOut := &bytes.Buffer{}
	code := run([]string{"--scenario=nonexistent"}, strings.NewReader(""), out, errOut)
	if code != 1 {
		t.Errorf("expected exit code 1 for invalid scenario, got %d", code)
	}
	if !strings.Contains(errOut.String(), "unknown scenario") {
		t.Errorf("expected 'unknown scenario' error, got: %s", errOut.String())
	}
}
```

In `cmd/wasm/session_test.go`:
```go
func TestSession_ScenarioCommands(t *testing.T) {
	s := NewSession(42)
	listOutput := s.Execute("scenario list")
	if !strings.Contains(listOutput, "kobayashi-maru") {
		t.Errorf("expected scenario list in WASM output, got: %s", listOutput)
	}

	loadOutput := s.Execute("scenario mutara")
	if !strings.Contains(loadOutput, "MUTARA NEBULA") {
		t.Errorf("expected Mutara briefing or header, got: %s", loadOutput)
	}
	if s.game.Scenario != engine.ScenarioMutaraNebula {
		t.Errorf("expected game scenario to be %v, got %v", engine.ScenarioMutaraNebula, s.game.Scenario)
	}
}
```

- [ ] **Step 2: Run tests to verify failure**

Run: `go test -v ./cmd/sst -run "TestCLI_ListScenarios|TestCLI_Scenario"`
Run: `go test -v ./cmd/wasm -run TestSession_ScenarioCommands`
Expected: FAIL

- [ ] **Step 3: Implement CLI flags in `cmd/sst/main.go` and WASM commands in `cmd/wasm/session.go`**

1. In `cmd/sst/main.go`:
   - Add `scenarioFlag := fs.String("scenario", "", "launch specific tactical scenario")`
   - Add `fs.StringVar(scenarioFlag, "s", "", "shorthand for --scenario")`
   - Add `listScenariosFlag := fs.Bool("list-scenarios", false, "display available tactical scenarios")`
   - If `*listScenariosFlag`: print formatted list of scenarios and return 0.
   - If `*scenarioFlag != ""`: look up scenario via `engine.GetScenario`. If not found, print error and return 1. Build game via `scenario.Build(seed)`.
2. In `cmd/wasm/session.go`:
   - Handle `"scenario"` command: if args are `"list"`, format available scenarios. If args match a scenario ID/alias, rebuild session with `scenario.Build(seed)` and format the briefing.

- [ ] **Step 4: Run tests to verify pass**

Run: `go test -v ./cmd/sst -run "TestCLI_ListScenarios|TestCLI_Scenario"`
Run: `go test -v ./cmd/wasm -run TestSession_ScenarioCommands`
Expected: PASS

- [ ] **Step 5: Verify WASM compilation**

Run: `GOOS=js GOARCH=wasm go build -o /dev/null ./cmd/wasm`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add cmd/sst/main.go cmd/sst/main_test.go cmd/wasm/session.go cmd/wasm/session_test.go
git commit -m "feat(cli,wasm): add scenario CLI flags and WebAssembly session commands"
```

---

### Task 5: TUI Scenario Browser Modal & Command Palette Integration

**Files:**
- Create: `pkg/tui/components/scenariomodal/modal.go`
- Create: `pkg/tui/components/scenariomodal/modal_test.go`
- Modify: `pkg/tui/app.go:50-200`
- Modify: `pkg/tui/commandpalette.go:1-60`

**Interfaces:**
- Consumes: `Scenario`, `ListScenarios`, `GetScenario`, `Theme`
- Produces:
  `scenariomodal.Model`: interactive Bubbletea component rendering 2-column scenario catalog.
  Message `MsgLaunchScenario{ScenarioID: id}` handled in `pkg/tui/app.go`.
  Command Palette command: `"Launch Tactical Scenario"`.

- [ ] **Step 1: Write failing tests in `pkg/tui/components/scenariomodal/modal_test.go`**

```go
package scenariomodal

import (
	"testing"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

func TestScenarioModal_NavigationAndSelection(t *testing.T) {
	th := theme.GetTheme("modern")
	m := NewModel(th)
	m.SetDimensions(80, 24)

	// Initial selection is index 0 (Kobayashi Maru)
	if m.SelectedScenario().ID != engine.ScenarioKobayashiMaru {
		t.Errorf("expected first selection %v, got %v", engine.ScenarioKobayashiMaru, m.SelectedScenario().ID)
	}

	// Move down
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyDown})
	if m.SelectedScenario().ID != engine.ScenarioMutaraNebula {
		t.Errorf("expected second selection %v, got %v", engine.ScenarioMutaraNebula, m.SelectedScenario().ID)
	}

	// Press Enter to trigger launch message
	_, cmd := m.Update(tea.KeyMsg{Type: tea.KeyEnter})
	if cmd == nil {
		t.Fatalf("expected launch command on Enter, got nil")
	}
	msg := cmd()
	launchMsg, ok := msg.(MsgLaunchScenario)
	if !ok || launchMsg.ScenarioID != engine.ScenarioMutaraNebula {
		t.Errorf("expected MsgLaunchScenario for %v, got %+v", engine.ScenarioMutaraNebula, msg)
	}
}
```

- [ ] **Step 2: Run tests to verify failure**

Run: `go test -v ./pkg/tui/components/scenariomodal`
Expected: FAIL

- [ ] **Step 3: Implement Scenario Modal and App Integration**

1. In `pkg/tui/components/scenariomodal/modal.go`:
   - Implement `Model` with scenario cursor navigation (`Up`/`Down`/`k`/`j`), dismiss on `Esc`/`q`, confirm on `Enter`.
   - Implement `View`: 2-column layout styled with Lipgloss. Left pane lists scenario names and difficulty badges. Right pane displays full briefing, rules, and best score.
2. In `pkg/tui/commandpalette.go`:
   - Add `"Launch Tactical Scenario"` command opening the scenario modal.
3. In `pkg/tui/app.go`:
   - Handle opening and closing the scenario modal overlay.
   - On `MsgLaunchScenario`: initialize new game with `scenario.Build(seed)`, update active game model, and reset dashboard view.

- [ ] **Step 4: Run tests to verify pass**

Run: `go test -v ./pkg/tui/components/scenariomodal`
Run: `go test -v ./pkg/tui/...`
Expected: PASS

- [ ] **Step 5: Run full project verification**

Run: `go test -v -race ./...`
Run: `tests/golden.sh build/debug/sst`
Run: `ctest --preset debug`
Run: `GOOS=js GOARCH=wasm go build -o /dev/null ./cmd/wasm`
Expected: All tests pass with 100% clean output.

- [ ] **Step 6: Commit**

```bash
git add pkg/tui/components/scenariomodal/ pkg/tui/app.go pkg/tui/commandpalette.go
git commit -m "feat(tui): add interactive Scenario Browser modal and Command Palette integration"
```
