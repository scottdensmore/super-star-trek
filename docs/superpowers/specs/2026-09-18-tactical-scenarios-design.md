# Curated Tactical Scenarios & Challenge Modes Design Specification

- **Date:** 2026-09-18
- **Status:** Approved
- **Target Branch:** `scottdensmore/feat/tactical-scenarios`
- **Issue Reference:** Closes Subsystem 2 of [#247](https://github.com/scottdensmore/super-star-trek/issues/247)

---

## 1. Executive Summary & Goals

Following the implementation of the Spatial Anomalies & Environmental Hazards Engine (Subsystem 1), this specification defines **Subsystem 2: Curated Tactical Scenarios & Challenge Modes** for Super Star Trek.

Tactical Scenarios provide handcrafted, lore-rich challenge missions that depart from the standard procedural sandbox campaign. Players can test their command capabilities in focused tactical engagements featuring custom fleet compositions, environmental hazards, time constraints, and tailored victory/defeat criteria.

### Key Goals
1. **Three Flagship Scenarios:** Deliver authentic, highly differentiated missions:
   - *Kobayashi Maru*: The infamous unwinnable endurance trial set in the Neutral Zone, awarding Starfleet Commendations based on survival time and kill count.
   - *Mutara Nebula*: A high-stakes blind duel in electrostatic gas clouds with defensive shields completely disabled and long-range sensors blinded.
   - *Starbase Under Siege*: A time-critical rescue mission requiring the captain to intercept an assault fleet and save Starbase 12 before it falls.
2. **Modular Scenario Architecture:** Introduce a declarative `Scenario` specification and registry in `pkg/engine/scenarios.go` allowing clean custom initializations, post-turn condition checks, and specialized scoring without altering classic campaign logic.
3. **Dedicated Scenario Leaderboards:** Persist separate top-10 Hall of Fame rankings for each scenario with thematic historical legends and commendation ratings.
4. **Seamless Player Access:** Support direct launch via CLI flags (`--scenario <id>`, `--list-scenarios`), WebAssembly terminal commands (`scenario <id>`), and an interactive Scenario Browser modal in the Bubbletea TUI accessible via the Command Palette (`Ctrl+P` / `F1`).
5. **Zero Classic Regressions:** Standard campaigns remain 100% true to 1978 baseline rules with zero modifications.

---

## 2. Architecture & Domain Models

```
┌─────────────────────────────────────────────────────────────┐
│                    Scenario Registry                        │
│                                                             │
│   GetScenario(id ScenarioID) (*Scenario, bool)              │
│   ListScenarios() []*Scenario                               │
│                                                             │
│   ┌──────────────────────┐  ┌────────────────────────────┐  │
│   │   kobayashi-maru     │  │       mutara-nebula        │  │
│   ├──────────────────────┤  ├────────────────────────────┤  │
│   │   starbase-siege     │  │       (future scenarios)   │  │
│   └──────────────────────┘  └────────────────────────────┘  │
└──────────────┬───────────────────────────────┬──────────────┘
               │ Build()                       │ Evaluate()
               ▼                               ▼
┌─────────────────────────────────────────────────────────────┐
│                         GameState                           │
│                                                             │
│   Scenario: ScenarioID ("kobayashi-maru", etc.)             │
│   Rules: GameRules (Customized per scenario)                │
│   CurrentQuad: Custom quadrant layout                       │
│   Metrics: GameMetrics                                      │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                  Leaderboard Subsystem                      │
│                                                             │
│  - Campaign: leaderboard.json                               │
│  - Scenarios: leaderboard_<scenario_id>.json                │
│  - Commendations: [COMM-1] .. [COMM-4] / [KIRK-AWARD]       │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌──────────────────────────────┴──────────────────────────────┐
│                    Player Entry Points                      │
│                                                             │
│  ┌───────────────────────┐      ┌────────────────────────┐  │
│  │   CLI: --scenario     │      │   TUI: Scenario Modal  │  │
│  │   --list-scenarios    │      │   (Command Palette)    │  │
│  └───────────────────────┘      └────────────────────────┘  │
│  ┌───────────────────────┐                                  │
│  │   WASM: scenario <id> │                                  │
│  └───────────────────────┘                                  │
└─────────────────────────────────────────────────────────────┘
```

### 2.1 Scenario Domain Model (`pkg/engine/scenarios.go`)

```go
// ScenarioID uniquely identifies a curated challenge scenario.
type ScenarioID string

const (
    ScenarioNone          ScenarioID = ""
    ScenarioKobayashiMaru ScenarioID = "kobayashi-maru"
    ScenarioMutaraNebula  ScenarioID = "mutara-nebula"
    ScenarioStarbaseSiege ScenarioID = "starbase-siege"
)

// Scenario encapsulates the configuration, factory, and custom logic of a challenge mode.
type Scenario struct {
    ID          ScenarioID `json:"id"`
    Name        string     `json:"name"`
    Subtitle    string     `json:"subtitle"`
    Difficulty  string     `json:"difficulty"` // "Extreme", "Hard", "Challenge"
    Description string     `json:"description"`
    Briefing    []string   `json:"briefing"`

    // Factory creates and configures the deterministic initial game state
    Build func(seed int64) *GameState `json:"-"`

    // Evaluate inspects GameState after each turn for custom win/loss triggers
    Evaluate func(g *GameState) (done bool, won bool, reason GameOverReason) `json:"-"`

    // ComputeScore calculates the scenario's specialized commendation or score breakdown
    ComputeScore func(g *GameState, won bool) ScoreBreakdown `json:"-"`
}
```

### 2.2 GameState Extension (`pkg/engine/state.go`)

```go
type GameState struct {
    // ... existing fields ...
    Scenario ScenarioID `json:"scenario,omitempty"`
    // ...
}
```

When `g.Scenario != ScenarioNone`, the game engine calls `scenario.Evaluate(g)` during turn resolution. If `done == true`, the engine emits the resulting `EventGameOver` immediately.

---

## 3. The Three Curated Scenarios

### 3.1 Scenario 1: *Kobayashi Maru* (`ScenarioKobayashiMaru`)
- **Theme**: Starfleet Academy's legendary no-win tactical simulation.
- **Aliases**: `kobayashi`, `km`, `kobayashi-maru`.
- **Difficulty**: `Extreme`.
- **Briefing**:
  ```text
  STATION LOG: GAMMA HYDRA SECTOR 10 (KLINGON NEUTRAL ZONE)
  Third-class neutronic fuel carrier KOBAYASHI MARU has struck a gravitic mine.
  Hull breached, 81 crew aboard, 300 passengers. Power systems failing.
  You are entering the Neutral Zone in violation of the Organian Peace Treaty.
  ```
- **Initial Setup**:
  - `Enterprise.Quad = Coord{4, 4}` (Neutral Zone).
  - `Enterprise.Sector = Coord{4, 4}`.
  - `Enterprise.Energy = 5000`, `Shields = 2500`, `Torpedoes = 10`.
  - Starting Quadrant `[4, 4]` contains 3 Klingon Battlecruisers.
  - Neighboring quadrants seeded with Klingon patrol fleets.
- **Wave & Reinforcement Mechanics**:
  - After any Klingon in the quadrant is eliminated or every 2.0 stardates elapsed, if fewer than 3 Klingons occupy the quadrant, reinforcements enter from edge sectors.
- **Evaluation & Commendations**:
  - The simulation cannot be won conventionally.
  - When the Enterprise is destroyed or energy is depleted, `Evaluate` returns `done = true, won = false, reason = GameOverLost`.
  - `ComputeScore` assigns a Starfleet Tactical Commendation based on survival:
    - **`< 3` Kills, `< 2.0` Stardates**: `[COMM-1]` *Cadet Commendation for Valor*
    - **`3–5` Kills, `2.0–4.0` Stardates**: `[COMM-2]` *Commendation for Tactical Excellence*
    - **`6–9` Kills, `4.0–8.0` Stardates**: `[COMM-3]` *Starfleet Cross of Honor*
    - **`10+` Kills or `8.0+` Stardates**: `[COMM-4]` *Admiral's Citation for Gallantry*

### 3.2 Scenario 2: *Mutara Nebula* (`ScenarioMutaraNebula`)
- **Theme**: Khan-style tactical duel inside a dense electrostatic nebula.
- **Aliases**: `mutara`, `nebula`, `mutara-nebula`.
- **Difficulty**: `Hard`.
- **Briefing**:
  ```text
  TACTICAL ENGAGEMENT: MUTARA NEBULA (SECTOR 5-5)
  USS Enterprise has pursued an advanced Klingon Super-Commander into the Mutara Nebula.
  High ionization renders deflector shields completely inoperative.
  Long-range sensors blinded. Target vessel possesses tactical cloaking.
  Rely on manual ballistic vectors and short-range scans to locate and destroy the enemy.
  ```
- **Initial Setup**:
  - `Enterprise.Quad = Coord{5, 5}`.
  - `QuadrantEnv[5][5] = EnvNebula`.
  - `Enterprise.Shields = 0`, `Enterprise.Energy = 5000`, `Torpedoes = 10`.
  - `Quadrant[5][5]` contains 1 Klingon Super-Commander with cloaking enabled (`IsCloaked = true`, `IsCommander = true`, `Energy = 1200`).
  - LRS returns `-1` (`***`), shields cannot be raised.
- **Victory & Defeat**:
  - **Victory**: Destroy the Super-Commander (`KlingonsKilled + SuperCommandersKilled >= 1`).
  - **Defeat**: Enterprise destroyed or energy depleted.

### 3.3 Scenario 3: *Starbase Under Siege* (`ScenarioStarbaseSiege`)
- **Theme**: Emergency defense of a besieged Federation starbase under strict time pressure.
- **Aliases**: `siege`, `starbase-siege`, `starbase`.
- **Difficulty**: `Challenge`.
- **Briefing**:
  ```text
  PRIORITY ONE DISTRESS CALL: STARBASE 12 (QUADRANT 4-4)
  Starbase 12 is under heavy bombardment by a coordinated Klingon strike wing.
  Enterprise position: Quadrant 2-2. Distance: 2 quadrants.
  Estimated time until starbase defensive perimeter collapses: 6.0 Stardates.
  Intercept immediately, eliminate the siege fleet, and protect Starbase 12.
  ```
- **Initial Setup**:
  - Enterprise starts at Quadrant `[2, 2]`.
  - Starbase 12 is located in Quadrant `[4, 4]` at Sector `[4, 4]`, surrounded by 3 Klingon Battlecruisers.
  - `TimeRemaining = 6.0` stardates.
- **Siege Mechanics**:
  - Each stardate that elapses while Klingons occupy Quadrant `[4, 4]` damages Starbase 12.
  - If Starbase 12 is destroyed, `Evaluate` immediately returns:
    `done = true, won = false, reason = GameOverLost` (*"Starbase 12 was destroyed by enemy bombardment."*).
- **Victory Condition**:
  - Enter Quadrant `[4, 4]` and eliminate all 3 attacking cruisers before Starbase 12 is destroyed and before time expires.

---

## 4. Dedicated Scenario Leaderboards & Persistence

### 4.1 Schema Extension (`pkg/engine/leaderboard.go`)
```go
type ScoreEntry struct {
    CaptainName  string    `json:"captain_name"`
    Score        int       `json:"score"`
    Rank         string    `json:"rank"`
    Skill        string    `json:"skill,omitempty"`
    Difficulty   string    `json:"difficulty,omitempty"`
    Stardate     float64   `json:"stardate,omitempty"`
    Date         time.Time `json:"date"`
    GameWon      bool      `json:"game_won,omitempty"`
    Scenario     string    `json:"scenario,omitempty"`
    Commendation string    `json:"commendation,omitempty"`
}
```

### 4.2 Storage Isolation
- Standard campaign: `leaderboard.json`
- Scenarios: `leaderboard_<scenario_id>.json`
  - Function `ScenarioLeaderboardPath(id ScenarioID) string`
  - Function `LoadScenarioLeaderboard(id ScenarioID) (*Leaderboard, error)`
  - Function `SaveScenarioLeaderboard(id ScenarioID, lb *Leaderboard) error`
  - Function `AddScenarioScore(id ScenarioID, entry ScoreEntry) (int, bool, error)`

### 4.3 Default Historical Legends
- **Kobayashi Maru**:
  - `Cadet James T. Kirk` (Score: 2400, Rank: `[KIRK-AWARD]`, Commendation: "Commendation for Original Thinking")
  - `Cadet Spock` (Score: 950, Rank: `[COMM-3]`, Commendation: "Starfleet Cross of Honor")
  - `Lieutenant Saavik` (Score: 620, Rank: `[COMM-2]`, Commendation: "Tactical Excellence")
- **Mutara Nebula**:
  - `Admiral James T. Kirk` (Score: 1500, Rank: `[ADM]`, Won: true)
  - `Captain Clark Terrell` (Score: 820, Rank: `[CAPT]`, Won: false)
- **Starbase Under Siege**:
  - `Captain Hikaru Sulu` (Score: 1350, Rank: `[CAPT]`, Won: true)
  - `Commander Montgomery Scott` (Score: 1100, Rank: `[COMM]`, Won: true)

---

## 5. UI & Presentation Specifications

### 5.1 CLI Flags (`cmd/sst/main.go`)
- `--scenario <id>` (or `-s <id>`): Launches the named scenario.
- `--list-scenarios`: Displays formatted ASCII list of available scenarios and exits with 0.

### 5.2 TUI Scenario Browser Modal (`pkg/tui/components/scenariomodal`)
- Triggered from Command Palette (`Ctrl+P` / `F1` $\to$ *"Launch Tactical Scenario"*).
- Displays two-column overlay:
  - Left: Selectable scenario list with difficulty badges (`[EXTREME]`, `[HARD]`, `[CHALLENGE]`).
  - Right: Mission briefing, special operational rules, and current high score record.
- Keyboard bindings: `↑`/`k` / `↓`/`j` navigate, `Enter` confirms and launches, `Esc`/`q` cancels.

### 5.3 WebAssembly Terminal (`cmd/wasm`)
- Adds `scenario` command to parser:
  - `scenario list`: Formats and displays available scenarios.
  - `scenario <id>`: Initializes and restarts session in the requested scenario.

---

## 6. Testing Strategy & Verification Plan

### 6.1 Core Scenario Engine Tests (`pkg/engine/scenarios_test.go`)
1. **Registry Verification**:
   - `TestScenarioRegistry`: Verify lookup by ID and all aliases (`kobayashi`, `km`, `mutara`, `siege`).
   - `TestListScenarios`: Verify all 3 scenarios are returned in order.
2. **Scenario Setup & Mechanics**:
   - `TestKobayashiMaru_InitAndWaves`: Verify Enterprise at `[4, 4]`, 3 cruisers present, wave reinforcement triggers when cruisers fall below threshold.
   - `TestKobayashiMaru_CommendationEvaluation`: Verify that loss with 1 kill awards `[COMM-1]`, 4 kills awards `[COMM-2]`, 7 kills awards `[COMM-3]`, and 11 kills awards `[COMM-4]`.
   - `TestMutaraNebula_InitAndVictory`: Verify quadrant `[5, 5]` is `EnvNebula`, shields are 0, cloaked commander present, victory triggers when commander is killed.
   - `TestStarbaseSiege_InitAndLossOnBaseDestroyed`: Verify Starbase 12 in `[4, 4]`, Enterprise in `[2, 2]`, loss triggers if Starbase 12 is eliminated.

### 6.2 Scenario Leaderboards (`pkg/engine/leaderboard_test.go`)
1. `TestScenarioLeaderboard_Isolation`: Assert writing scenario score modifies only `leaderboard_<id>.json` and leaves standard `leaderboard.json` intact.
2. `TestScenarioLeaderboard_Commendations`: Assert commendation strings serialize and deserialize without data loss.

### 6.3 UI & CLI Tests
1. `cmd/sst/main_test.go`:
   - `TestCLI_ListScenarios`: Verify `--list-scenarios` outputs catalog and exits 0.
   - `TestCLI_ScenarioLaunch`: Verify `--scenario mutara-nebula` initializes scenario game state.
   - `TestCLI_ScenarioInvalid`: Verify unknown scenario exits 1 with helpful error message.
2. `pkg/tui/components/scenariomodal/modal_test.go`:
   - Verify modal key navigation, rendering, and launch selection.
3. `cmd/wasm/session_test.go`:
   - Verify `scenario list` and `scenario mutara-nebula` commands in WASM teletype mode.

### 6.4 Regression Suite
- `tests/golden.sh build/debug/sst` (100% golden master pass).
- `ctest --preset debug` (100% C parity pass).
- `go test -v -race ./...` (zero race conditions, zero failures).
- `GOOS=js GOARCH=wasm go build -o /dev/null ./cmd/wasm` (clean WASM compilation).
