# Design Specification: Configurable Difficulty & Realism Settings

**Date:** 2026-09-13  
**Status:** Approved  
**Author:** Pair Programming Agent & User  
**Issue:** [#228](https://github.com/scottdensmore/super-star-trek/issues/228)  
**Target Systems:** `pkg/engine`, `pkg/tui`, `pkg/tui/components/optionsmodal`, `cmd/sst`

---

## 1. Overview & Motivation

In Super Star Trek (SST), players face varying tactical and strategic challenges across galaxy exploration, starbase support, sensor intelligence, battle damage management, and enemy engagements. While recent milestones introduced procedurally generated galaxy charts, long-range reconnaissance (`lrscan`), starbase docking intelligence, and sensor degradation indicators, these systems currently operate under fixed, hardcoded defaults.

This specification details a comprehensive, configurable difficulty and realism system:
1. **Configurable Exploration & Starbase Surveillance (`pkg/engine`):** Four distinct surveillance modes (Full Survey, Classic Network, Local Perimeter, and Strict Blackout Fog-of-War).
2. **Tactical & Battle Realism Modifiers (`pkg/engine`):** A two-tier long-range sensor degradation curve (light damage vs heavy blackout), Klingon Commander tactical cloaking, subsystem repair time multipliers, and initial time margin scaling.
3. **Difficulty Profiles:** Preset difficulty bundles (`Casual`, `Normal`, `Hardcore`, `Nightmare`, `Custom`) providing instant setup while permitting granular overrides.
4. **Interactive TUI Options Modal (`pkg/tui/components/optionsmodal`):** An in-game overlay accessible via hotkey (`o`/`O`) or commands (`opts`/`options`/`settings`) to inspect and adjust configuration live.
5. **Launch Flags & Save Game Persistence (`cmd/sst`, `pkg/engine/save.go`):** Command-line launch configuration flags and backward-compatible JSON game state persistence.

---

## 2. Architecture & Data Structures

### 2.1 Engine Layer (`pkg/engine`)

#### 2.1.1 Types and Constants (`pkg/engine/rules.go` or `state.go`)

```go
package engine

// DifficultyProfile represents standard game difficulty presets.
type DifficultyProfile string

const (
	ProfileCasual    DifficultyProfile = "casual"
	ProfileNormal    DifficultyProfile = "normal"
	ProfileHardcore  DifficultyProfile = "hardcore"
	ProfileNightmare DifficultyProfile = "nightmare"
	ProfileCustom    DifficultyProfile = "custom"
)

// SurveillanceMode defines the extent of quadrant intelligence revealed upon starbase docking.
type SurveillanceMode string

const (
	SurveillanceFull     SurveillanceMode = "full"     // Reveals all 64 quadrants
	SurveillanceClassic  SurveillanceMode = "classic"  // Reveals 3x3 perimeters around all active starbases
	SurveillanceLocal    SurveillanceMode = "local"    // Reveals 3x3 perimeter around docked base only
	SurveillanceBlackout SurveillanceMode = "blackout" // Reveals no additional quadrants; starbases unmapped at start
)

// GameRules encapsulates difficulty and realism configuration parameters.
type GameRules struct {
	Profile           DifficultyProfile `json:"profile"`
	Surveillance      SurveillanceMode  `json:"surveillance"`
	SensorDegradation bool              `json:"sensor_degradation"` // 2-tier sensor damage degradation curve
	RepairMultiplier  float64           `json:"repair_multiplier"`  // Repair duration scale (0.5 to 2.0x)
	KlingonCloak      bool              `json:"klingon_cloak"`      // Commander cloaking capability
	TimeMargin        float64           `json:"time_margin"`        // Initial stardate allowance scale
}
```

#### 2.1.2 Difficulty Preset Matrix
`DefaultRulesForProfile(profile DifficultyProfile) GameRules` returns the pre-configured rules:

| Setting / Modifier | Casual | Normal / Classic | Hardcore | Nightmare |
| :--- | :--- | :--- | :--- | :--- |
| **Surveillance Extent** | `SurveillanceFull` | `SurveillanceClassic` | `SurveillanceLocal` | `SurveillanceBlackout` |
| **Starbases Charted at Start** | Yes (`ChartKnownBases`) | Yes (`ChartKnownBases`) | Yes (`ChartKnownBases`) | **No** (Unmapped) |
| **Sensor Degradation Curve** | `false` | `true` | `true` | `true` |
| **Repair Multiplier** | `0.75x` | `1.00x` | `1.50x` | `2.00x` |
| **Klingon Cloaking** | `false` | `false` | `true` | `true` |
| **Stardate Time Margin** | `1.25x` (37.5 units) | `1.00x` (30.0 units) | `0.80x` (24.0 units) | `0.60x` (18.0 units) |

#### 2.1.3 Game Initialization (`NewGameWithOptions`)
```go
func NewGameWithOptions(seed int64, skill SkillLevel, length GameLength, rules GameRules) *GameState {
    rng := NewPRNG(seed)
    g := &GameState{
        RNG:                rng,
        Skill:              skill,
        Length:             length,
        Rules:              rules,
        Stardate:           float64(2000 + rng.Intn(1000)),
        TimeRemaining:      30.0 * rules.TimeMargin,
        RemainingKlingons:  15,
        RemainingStarbases: 3,
        Enterprise: Enterprise{
            Quad:      Coord{rng.Intn(8) + 1, rng.Intn(8) + 1},
            Sector:    Coord{rng.Intn(8) + 1, rng.Intn(8) + 1},
            Energy:    5000,
            Shields:   0,
            Torpedoes: 10,
            Condition: ConditionGreen,
        },
    }
    g.InitialStardate = g.Stardate

    // Populate stars
    for r := 1; r <= 8; r++ {
        for c := 1; c <= 8; c++ {
            g.GalaxyChart[r][c] = rng.Intn(9) + 1
        }
    }

    // Place starbases
    placedBases := 0
    for placedBases < g.RemainingStarbases {
        r := rng.Intn(8) + 1
        c := rng.Intn(8) + 1
        if (g.GalaxyChart[r][c]%100)/10 == 0 {
            g.GalaxyChart[r][c] += 10
            if rules.Surveillance != SurveillanceBlackout {
                g.ChartKnownBases[r][c] = true
            }
            placedBases++
        }
    }

    // Distribute Klingons
    // ...
    // Starting quadrant discovered
    g.ChartDiscovered[g.Enterprise.Quad[0]][g.Enterprise.Quad[1]] = true

    return g
}

func NewGame(seed int64, skill SkillLevel, length GameLength) *GameState {
    return NewGameWithOptions(seed, skill, length, DefaultRulesForProfile(ProfileNormal))
}
```

---

## 3. Tactical & Simulation Mechanics

### 3.1 Starbase Docking Surveillance (`ActionDock`)
When executing `ActionDock.Execute(g)`:
1. **`SurveillanceFull`:**
   - Marks all active starbases in `ChartKnownBases`.
   - Sets `ChartDiscovered[r][c] = true` for all `r, c ∈ [1..8]`.
2. **`SurveillanceClassic`:**
   - Marks all active starbases in `ChartKnownBases`.
   - Discovers 3×3 perimeters around all active starbases.
3. **`SurveillanceLocal`:**
   - Marks only the currently docked starbase `sb` in `ChartKnownBases`.
   - Discovers 3×3 perimeter centered on `sb`.
4. **`SurveillanceBlackout`:**
   - Marks only the currently docked starbase `sb` in `ChartKnownBases`.
   - Discovers 0 additional quadrants beyond current quadrant.
5. Emits `EventStarbaseSurveillance{StarbaseCoord: sb, UpdatedQuads: count, Mode: g.Rules.Surveillance}`.

### 3.2 Two-Tier Sensor Degradation Curve
When `g.Rules.SensorDegradation == true`:
- **Operational (`DeviceLRSensors == 0`):** Full 3×3 radar display; standard `lrscan` execution.
- **Tier 1: Light Degradation (`0 < DeviceLRSensors < 2.0`):**
  - Radar header: `RADAR (QUAD ±1) [LRS DEGRADED]`. Adjacent quadrant markers display with noise or sensor ambiguity placeholder indicators (`?`).
  - `ActionLRScan`: Records quadrant data into `ChartDiscovered`, but emits a degradation advisory message: `"L.R. SENSORS DEGRADED: READINGS MAY CONTAIN AMBIGUITY"`.
- **Tier 2: Heavy Damage (`DeviceLRSensors >= 2.0`):**
  - Radar header: `RADAR (QUAD ±1) [LRS OFFLINE]` with all adjacent cells replaced with `???`.
  - `ActionLRScan`: Standalone scan blocked with `"LONG RANGE SENSORS DAMAGED"` unless docked with a Starbase.

If `g.Rules.SensorDegradation == false` (Casual mode):
- Only complete failure at heavy damage occurs, without Tier 1 noise or degradation restrictions.

### 3.3 Subsystem Repair Multipliers
- When advancing game time / stardates (`AdvanceTurn` or repair cycles):
  - `RepairStep = ElapsedStardate / g.Rules.RepairMultiplier`.
  - Accelerates repair under Casual (`0.75x`) and elongates repair under Hardcore (`1.5x`) and Nightmare (`2.0x`).

### 3.4 Klingon Commander Tactical Cloaking
- `Klingon` struct field: `IsCloaked bool`.
- When `g.Rules.KlingonCloak == true`:
  - Commander vessels (`IsCommander == true`) can cloak after maneuvering or when evading damage.
  - While `IsCloaked == true`:
    - The vessel does not render as `+K+` on the sector grid; instead, it is hidden or displayed as a sensor echo `?`.
    - Direct torpedo lock (`ActionTorpedoDirect` / `targetlock`) fails with `"TARGET LOCK FAILED: CLOAKED VESSEL"`.
    - The vessel decloaks upon discharging disruptors, firing torpedoes, or taking damage from a phaser sweep.
  - Emits `EventKlingonCloakState{KlingonID: id, Cloaked: bool}`.

---

## 4. User Interface & CLI Integration

### 4.1 CLI Launch Arguments (`cmd/sst/main.go`)
Added flags to `flag.FlagSet`:
- `--difficulty <casual|normal|hardcore|nightmare>` (default: `normal`)
- `--surveillance <full|classic|local|blackout>`
- `--sensor-degradation <true|false>`
- `--repair-multiplier <float64>`
- `--klingon-cloak <true|false>`

If any granular modifier is explicitly set via CLI flags while `--difficulty` is specified, the explicit modifier overrides the preset value, and the profile is tagged as `custom`.

### 4.2 TUI Options Modal (`pkg/tui/components/optionsmodal`)
A centered overlay rendered via `lipgloss.Place` over the main TUI view:
- **Title:** `⚙ STARFLEET CONFIGURATION & RULES`
- **Rows:**
  1. `Difficulty Profile:` `[ ◀ Casual | Normal | Hardcore | Nightmare | Custom ▶ ]`
  2. `Starbase Surveillance:` `[ ◀ Full | Classic | Local | Blackout ▶ ]`
  3. `Sensor Degradation Curve:` `[ ◀ Enabled | Disabled ▶ ]`
  4. `Subsystem Repair Multiplier:` `[ ◀ 0.75x | 1.00x | 1.50x | 2.00x ▶ ]`
  5. `Klingon Tactical Cloaking:` `[ ◀ Enabled | Disabled ▶ ]`
  6. `Stardate Time Margin:` `[ ◀ 125% | 100% | 80% | 60% ▶ ]`
  7. `[ Done / Resume Mission ]`
- **Keybindings & Interaction:**
  - `o` / `O` opens options from normal mode.
  - Commands: `opts`, `options`, `settings` open the modal.
  - `↑` / `↓` (`k` / `j`): Select item row.
  - `←` / `→` (`h` / `l`) or `Space`: Cycle values left/right.
  - Cycling the Profile automatically updates all subordinate options.
  - Altering an individual option updates Profile to `Custom`.
  - `Esc`, `q`, or `Enter` on `[ Done ]` closes the modal.
  - Live-updates `game.Rules` in the active game model.

---

## 5. Persistence & Backward Compatibility

### 5.1 Save/Load (`pkg/engine/save.go`)
- `GameState.Rules` is serialized directly into `.TRK` JSON files.
- In `LoadGame(path)`:
  - If `g.Rules.Profile == ""` (older save file generated before this feature), `g.Rules` is defaulted to `DefaultRulesForProfile(ProfileNormal)`.
  - Ensures existing `.TRK` saves continue to load cleanly without panics or broken states.

---

## 6. Testing Strategy

1. **Engine Preset & Factory Tests (`pkg/engine/rules_test.go`):**
   - Verify `DefaultRulesForProfile` matrices for all profiles.
   - Verify `NewGameWithOptions` with `SurveillanceBlackout` leaves `ChartKnownBases` empty.
   - Verify `NewGame` preserves identical behavior to `ProfileNormal`.
2. **Surveillance Action Tests (`pkg/engine/actions_test.go`):**
   - `SurveillanceFull`: All 64 quadrants revealed.
   - `SurveillanceClassic`: All starbase perimeters revealed.
   - `SurveillanceLocal`: Only docked starbase perimeter revealed; distant bases hidden.
   - `SurveillanceBlackout`: Zero additional quadrants revealed.
3. **Sensor Degradation Tests (`pkg/engine/engine_test.go`, `pkg/tui/components/statuspanel`):**
   - Light vs Heavy damage state strings and `lrscan` execution rules.
4. **Klingon Cloaking Tests (`pkg/engine/combat_test.go`):**
   - Direct lock prevention on cloaked ships; decloaking on firing/damage.
5. **Persistence Round-Trip Tests (`pkg/engine/save_test.go`):**
   - Save and reload custom rules; verify older JSON saves without rules upgrade gracefully.
6. **TUI Options Modal Tests (`pkg/tui/components/optionsmodal/options_test.go`):**
   - Navigation, value cycling, preset cascading, and theme rendering snapshots.
7. **CLI Flag Tests (`cmd/sst/main_test.go`):**
   - Flag validation and preset override resolution.
