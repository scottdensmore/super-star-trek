# Authentic Scoring Engine, Starfleet Ranks & Hall of Fame Design

## 1. Overview & Goals

Super Star Trek calculates battle outcomes and game-over states, but historically lacked a full-featured in-game scoring breakdown and persistent local leaderboard in the Go modern codebase.

This design introduces:
1. **Authentic 15-Rule Scoring Engine (`pkg/engine/score.go`)**: Direct mathematical parity with classic `rules.c` (`score_compute`) awarding points for Klingon, Commander, and Super-Commander kills, Romulan engagements, kill-rate multipliers, difficulty win bonuses, and penalties for starbase loss, star/planet destruction, help calls, and crew casualties.
2. **Starfleet Officer Rank Tiers**: Real-time evaluation of captain ratings from *Cadet* to *Fleet Admiral*.
3. **Persistent Local Leaderboard (`pkg/engine/leaderboard.go`)**: Cross-platform, zero-dependency JSON storage adhering to XDG standards (`$XDG_CONFIG_HOME/super-star-trek/highscores.json`) with atomic writes and fallback recovery.
4. **Interactive Dual-Tab TUI Modal (`pkg/tui/components/halloffame`)**:
   - Tab 1: Live mission telemetry and score sheet itemization.
   - Tab 2: Top 10 All-Time Starfleet Hall of Fame table.
   - Interactive Captain Callsign entry when a game finishes with a qualifying score.
   - Strict adherence to the 66×18 overlay and 80×24 terminal dimension budget.

---

## 2. Engine Metric Tracking & State Model

### 2.1 Metric Tracking (`pkg/engine/state.go`)
`GameState` tracks mission event counters needed for scoring:

```go
package engine

// GameMetrics records cumulative mission counters for scoring.
type GameMetrics struct {
    KlingonsKilled        int `json:"klingons_killed"`        // Regular Klingons destroyed (+10 pts)
    CommandersKilled      int `json:"commanders_killed"`      // Commanders destroyed (+50 pts)
    SuperCommandersKilled int `json:"super_commanders_killed"`// Super-Commanders destroyed (+200 pts)
    RomulansKilled        int `json:"romulans_killed"`        // Romulans destroyed (+20 pts)
    RomulansSurrendered   int `json:"romulans_surrendered"`   // Romulans surrendered (+1 pt)
    StarbasesDestroyed    int `json:"starbases_destroyed"`    // Friendly starbases lost (-100 pts)
    StarsDestroyed        int `json:"stars_destroyed"`        // Stars destroyed by torpedoes (-5 pts)
    PlanetsDestroyed      int `json:"planets_destroyed"`      // Planets destroyed (-10 pts)
    Casualties            int `json:"casualties"`             // Crew casualties suffered (-1 pt)
    HelpCalls             int `json:"help_calls"`             // Distress calls to starbase (-45 pts)
    StarshipsLost         int `json:"starships_lost"`         // Starships lost (-100 pts each)
}
```

`GameState` embeds `Metrics GameMetrics` and tracks `GameWon bool`.

### 2.2 Action Dispatch Integration (`pkg/engine/actions.go` & `combat.go`)
- **Torpedo / Phaser Hits**: When a Klingon is destroyed, increments `Metrics.KlingonsKilled`, or `Metrics.CommandersKilled` if `k.IsCommander`.
- **Friendly Base / Star Destruction**: When torpedoes impact starbases or stars, increments `Metrics.StarbasesDestroyed` and `Metrics.StarsDestroyed`.
- **Enemy Damage**: Incoming enemy fire damaging life support or hull accumulates `Metrics.Casualties`.
- **ActionCallHelp**: Increments `Metrics.HelpCalls`.

---

## 3. Classic 15-Rule Scoring Engine (`pkg/engine/score.go`)

### 3.1 Scoring Math Formula
Direct port of `score_compute` in `rules.c`:
1. **Elapsed Time**:
   `elapsed = g.Stardate - g.InitialStardate`
   If `elapsed < 5.0` or `g.RemainingKlingons > 0` (game not yet won), minimum elapsed duration is clamped to `5.0` to prevent inflated kill rates.
2. **Kill Rate**:
   $$\text{totalKills} = \text{Klingons} + \text{Commanders} + \text{SuperCommanders}$$
   $$\text{killRate} = \frac{\text{totalKills}}{\text{elapsed}}$$
   $$\text{killRateBonus} = \text{round}(500 \times \text{killRate})$$
3. **Win Bonus**:
   If `g.GameWon`, $\text{winBonus} = 100 \times \text{SkillLevel}$ (Novice = 100, Fair = 200, Good = 300, Expert = 400, Emeritus = 500).
4. **Itemized Score Calculation**:
   $$\begin{aligned}
   \text{Total} = &+ 10 \times \text{Klingons} + 50 \times \text{Commanders} + 200 \times \text{SuperCommanders} \\
                  &+ 20 \times \text{Romulans} + 1 \times \text{Surrendered} + \text{killRateBonus} + \text{winBonus} \\
                  &- 100 \times \text{StarbasesLost} - 100 \times \text{StarshipsLost} - 45 \times \text{HelpCalls} \\
                  &- 10 \times \text{PlanetsLost} - 5 \times \text{StarsLost} - 1 \times \text{Casualties}
   \end{aligned}$$

### 3.2 Score Breakdown Structure
```go
type ScoreBreakdown struct {
    KlingonsKilled        int
    KlingonPoints         int
    CommandersKilled      int
    CommanderPoints       int
    SuperCommandersKilled int
    SuperCommanderPoints  int
    RomulansKilled        int
    RomulanPoints         int
    RomulansSurrendered   int
    SurrenderedPoints     int
    ElapsedStardates      float64
    KillRate              float64
    KillRatePoints        int
    GameWon               bool
    WinBonus              int
    StarbasesLost         int
    StarbasePenalty       int
    StarshipsLost         int
    StarshipPenalty       int
    HelpCalls             int
    HelpPenalty           int
    PlanetsDestroyed      int
    PlanetPenalty         int
    StarsDestroyed        int
    StarPenalty           int
    Casualties            int
    CasualtyPenalty       int
    TotalScore            int
    RankTitle             string
    RankBadge             string
}
```

### 3.3 Starfleet Ranks
| Net Score | Rank Title | Badge |
|---|---|---|
| $< 0$ | Dishonorable Discharge | `[DISHONOR]` |
| $0 - 99$ | Starfleet Cadet | `[CADET]` |
| $100 - 199$ | Lieutenant | `[LT]` |
| $200 - 349$ | Commander | `[CDR]` |
| $350 - 499$ | Captain | `[CAPT]` |
| $500 - 749$ | Commodore | `[COMM]` |
| $750 - 999$ | Rear Admiral | `[RADM]` |
| $\ge 1000$ | Fleet Admiral | `[FADM]` |

---

## 4. Leaderboard Persistence (`pkg/engine/leaderboard.go`)

### 4.1 Schema
```go
type ScoreEntry struct {
    CaptainName string    `json:"captain_name"`
    Score       int       `json:"score"`
    Rank        string    `json:"rank"`
    Skill       string    `json:"skill"`
    Difficulty  string    `json:"difficulty"`
    Stardate    float64   `json:"stardate"`
    Date        time.Time `json:"date"`
    GameWon     bool      `json:"game_won"`
}

type Leaderboard struct {
    Entries []ScoreEntry `json:"entries"`
}
```

### 4.2 File Resolution & Safety
1. **Path Resolution**:
   - Check `$XDG_CONFIG_HOME/super-star-trek/highscores.json`.
   - Fallback to `os.UserConfigDir()/super-star-trek/highscores.json`.
   - Local fallback `./.sst-scores.json` if configuration directory is not writable.
2. **Default Seed**:
   If file does not exist, pre-populates default Starfleet legends:
   - James T. Kirk (1180 pts, FADM)
   - Spock (980 pts, RADM)
   - Christopher Pike (840 pts, COMM)
   - Hikaru Sulu (720 pts, COMM)
   - Nyota Uhura (560 pts, CAPT)
3. **Atomic Writes**: Writes to temporary file in the target directory, flushes, syncs, and atomically renames to ensure zero corruption.
4. **Corrupted File Recovery**: Safely logs error, copies corrupted file to `.corrupt`, and loads default leaderboard.

---

## 5. Dual-Tab TUI Modal (`pkg/tui/components/halloffame`)

### 5.1 Dimension Budget
- Overlay dimensions: Exact **66 columns wide × 18 rows high**.
- Leaves at least 7 columns of border margin and 3 lines of top/bottom margin within an 80×24 terminal.

### 5.2 Views
- **Tab 1: Mission Telemetry**:
  - Two-column breakdown of combat points vs. penalties.
  - Net score readout and projected rank badge.
- **Tab 2: Top 10 Hall of Fame**:
  - Formatted leaderboard table: `#`, `SCORE`, `CAPTAIN`, `RANK`, `DIFFICULTY`, `DATE`.
- **Name Entry Prompt**:
  - Activated when game over score qualifies for the leaderboard.
  - Integrated `textinput.Model` with cursor blinking.
  - `Enter` commits the score to disk and displays the player in the table.

### 5.3 Navigation & Controls
- `Tab` / `1` / `2` / Left / Right arrows: Toggle between Tab 1 and Tab 2.
- `Esc` / `Enter` / `q` / `h`: Dismiss modal and return to dashboard.

---

## 6. Root TUI Integration

### 6.1 Modal Enum & Model
- Add `ModalHallOfFame` to `ModalType` in `pkg/tui/model.go`.
- Add `HallOfFame halloffame.Model` field to `pkg/tui.Model`.
- Initialize in `NewModel` via `halloffame.New(th, 66, 18, leaderboardPath)`.

### 6.2 Command & Hotkey Triggers
- Commands: `score`, `scores`, `halloffame`, `hof` open `ModalHallOfFame`.
- Hotkeys: `h` / `H` / `ctrl+h` when command line buffer is empty.
- Game Over Trigger: When game ends (win/lose), automatically displays the modal and activates name entry if qualified.

---

## 7. Verification & Test Strategy

1. **Engine Scoring Unit Tests (`pkg/engine/score_test.go`)**:
   - Test itemized score arithmetic matching classic SST rules.
   - Test minimum 5-stardate clamping for kill rate.
   - Test win bonus scaling across all 5 skill levels.
   - Test rank badge mapping across all score ranges.
2. **Leaderboard Storage Unit Tests (`pkg/engine/leaderboard_test.go`)**:
   - Test default pre-seeding when file absent.
   - Test top-10 insertion, sort order, and truncation.
   - Test atomic save and load.
   - Test corrupted JSON recovery.
3. **Component Unit Tests (`pkg/tui/components/halloffame/halloffame_test.go`)**:
   - Verify strict 66 cols × 18 rows output across both tabs.
   - Verify tab switching (`Tab`, `1`, `2`).
   - Verify callsign text input, validation, and submission.
4. **Integration & Snapshot Tests**:
   - `pkg/tui/model_test.go`: Test hotkey `h`, commands `score`/`scores`, and game-over auto-display.
   - `tests/tui_golden_test.go`: Golden snapshots for Tab 1 and Tab 2.
5. **Full Regression Suites**:
   - `go test -v -race ./...`
   - `ctest --preset debug`
   - `bash tests/golden.sh`
