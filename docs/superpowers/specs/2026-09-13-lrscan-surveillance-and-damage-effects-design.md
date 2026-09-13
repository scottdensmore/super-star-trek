# Design Specification: LRSCAN Discovery, Starbase Surveillance & Subsystem Damage Degradation

**Date:** 2026-09-13  
**Status:** Approved  
**Author:** Pair Programming Agent & User  
**Target Systems:** `pkg/engine`, `pkg/tui`, `pkg/tui/components/statuspanel`, `pkg/tui/components/galacticchart`

---

## 1. Overview & Motivation

In Super Star Trek (SST), galaxy exploration, sensor reconnaissance, and battle damage management are core strategic pillars. While Milestone 4 introduced the interactive 8x8 Galactic Star Chart modal and quadrant movement, the discovery map currently requires manual inspection setup, `lrscan` only logs static text without recording data, starbase docking does not download surveillance telemetry, and subsystem damage does not visually degrade sensors or star chart operations.

This specification details three integrated systems:
1. **Procedural Galaxy Generation & LRSCAN Discovery (`pkg/engine`):**
   - Seed-based deterministic galaxy population (`GalaxyChart[9][9]int`) with stars, starbases, and Klingon battle forces.
   - Long-range scan action (`ActionLRScan`) recording 3x3 surrounding quadrants into `ChartDiscovered[9][9]bool`.
2. **Starbase & Galaxy Surveillance (`pkg/engine` & `pkg/tui`):**
   - Initial Starfleet Command starbase charting in `ChartKnownBases[9][9]bool`.
   - Starbase docking intelligence download (`ActionDock`), revealing 3x3 quadrant surveillance perimeters around all active starbases.
   - Distinct chart rendering for unvisited starbase locations (`.1.`).
3. **Subsystem Damage Degradation (`pkg/tui/components`):**
   - **Long Range Sensors (`DeviceLRSensors > 0`):** Blocks standalone `lrscan` (permits starbase relay when docked), replaces Status Panel 3x3 radar with `RADAR (QUAD ±1) [LRS OFFLINE]` and `???` in adjacent cells.
   - **Library Computer (`DeviceComputer > 0`):** Degrades Galactic Star Chart modal footer to `Course: [CALC OFFLINE] • Warp: [CALC OFFLINE]` and locks out quick-warp on `Enter` with `"COMPUTER DAMAGED, USE A POCKET CALCULATOR"`.
   - Real dynamic device damage reporting for `dam` / `damages` commands.

---

## 2. Architecture & Data Structures

### 2.1 Engine Layer (`pkg/engine`)

#### 2.1.1 Procedural Galaxy Generation in `NewGame`
In `NewGame(seed int64, skill SkillLevel, length GameLength) *GameState`:
- Deterministically populate `GalaxyChart[9][9]int` using `rng := NewPRNG(seed)`:
  - **Stars:** 1 to 9 stars per quadrant: `stars := rng.Intn(9) + 1`. Quadrant value initialized to `stars`.
  - **Starbases:** Placed across `RemainingStarbases` distinct quadrants (avoiding existing base coordinates): `GalaxyChart[r][c] += 10`.
  - **Klingons:** Distribute `RemainingKlingons` in clusters across quadrants (clamped to max 9 Klingons per quadrant): `GalaxyChart[r][c] += kCount * 100`.
- **Initial Discovery State:**
  - `Enterprise.Quad` is discovered: `ChartDiscovered[entQuad[0]][entQuad[1]] = true`.
  - All starbase coordinates are marked in `ChartKnownBases`: for each quadrant with starbase, `ChartKnownBases[r][c] = true`.

#### 2.1.2 New Actions & Events
```go
// ActionLRScan triggers a long-range reconnaissance scan of surrounding quadrants.
type ActionLRScan struct{}

// EventLRScanCompleted is emitted upon successful execution of ActionLRScan.
type EventLRScanCompleted struct {
    CenterQuad    Coord
    ScannedQuads  []Coord
    RelayedByBase bool
}

// EventStarbaseSurveillance is emitted when Enterprise docks and downloads starbase network intelligence.
type EventStarbaseSurveillance struct {
    StarbaseCoord Coord
    UpdatedQuads  int
}
```

#### 2.1.3 Action Logic
- **`ActionLRScan.Execute(g *GameState)`:**
  - If `g.Enterprise.Devices[DeviceLRSensors] > 0` and `g.Enterprise.Condition != ConditionDocked`:
    - Returns `errors.New("long-range sensors damaged")`.
  - Relayed by base flag: `relayed := (g.Enterprise.Condition == ConditionDocked && g.Enterprise.Devices[DeviceLRSensors] > 0)`.
  - Identifies 3x3 surrounding quadrants `[r-1..r+1, c-1..c+1]`:
    - For each in-bounds quadrant `(1 <= r <= 8 && 1 <= c <= 8)`:
      - Sets `g.ChartDiscovered[r][c] = true`.
      - Appends quadrant to `ScannedQuads`.
  - Consumes 0 stardates and 0 turns (free informational action matching SST).
  - Returns `[]Event{EventLRScanCompleted{CenterQuad: g.Enterprise.Quad, ScannedQuads: scanned, RelayedByBase: relayed}}`.

- **`ActionDock.Execute(g *GameState)`:**
  - Refuels energy (5000), restocks torpedoes (10), resets all devices to 0 (`operational`), and sets `ConditionDocked`.
  - Iterates over all quadrants in `g.GalaxyChart`:
    - If quadrant contains an active starbase `(val % 100) / 10 > 0`:
      - Sets `g.ChartKnownBases[r][c] = true`.
      - For each quadrant in 3x3 perimeter around that starbase `(1 <= nr <= 8 && 1 <= nc <= 8)`:
        - If not previously discovered, marks `g.ChartDiscovered[nr][nc] = true` and increments `updatedQuads`.
  - Returns `[]Event{EventDocked{Starbase: sb}, EventStarbaseSurveillance{StarbaseCoord: sb, UpdatedQuads: updatedQuads}}`.

#### 2.1.4 Save Game Persistence (`pkg/engine/save.go`)
- `GameState` struct updated to include:
  ```go
  ChartKnownBases [9][9]bool `json:"chart_known_bases"`
  ```
- Because `GalaxyChart`, `ChartDiscovered`, and `ChartKnownBases` are exported fields on `GameState`, they are preserved across JSON serialization in `.TRK` files without breaking backwards compatibility.

---

## 3. Presentation Layer & Subsystem Degradation

### 3.1 Status Panel Radar Degradation (`pkg/tui/components/statuspanel`)

- **State Interface:**
  - `SetState(ent engine.Enterprise, timeRemaining float64, klingonsLeft int, starbasesLeft int, isDocked bool, chart [9][9]int)`
  - Evaluates: `lrsDamaged := ent.Devices[engine.DeviceLRSensors] > 0 && !isDocked`.
- **Header:**
  - Operational: `RADAR (QUADRANTS ±1)  [K-B-S]:`
  - Damaged: `RADAR (QUAD ±1) [LRS OFFLINE]:` (styled with warning color).
- **Quadrant Cells:**
  - Out of bounds: `***`
  - Current Quadrant (`dr == 0 && dc == 0`): rendered normally via `%03d` with `<>` or Enterprise highlight (short-range sensors operate).
  - Surrounding cells (`dr != 0 || dc != 0`):
    - Operational: `%03d` (e.g., `005`, `104`).
    - Damaged: `???` (styled with dimmed/warning style).

### 3.2 Galactic Star Chart Modal (`pkg/tui/components/galacticchart`)

- **State Interface:**
  - `SetState(entQuad engine.Coord, chart [9][9]int, discovered [9][9]bool, knownBases [9][9]bool, computerDamaged bool)`
- **Cell Rendering:**
  1. `chartDiscovered[r][c] == true`: Full telemetry reading `fmt.Sprintf("%03d", val)`.
  2. `!chartDiscovered[r][c] && knownBases[r][c] == true`: Known starbase reading `".1."`.
  3. `!chartDiscovered[r][c] && !knownBases[r][c]`: Undiscovered `···`.
  4. Enterprise position highlighted with `<>` or theme styles.
  5. Cursor highlighted with brackets `[...]`.
- **Telemetry Footer Under Computer Damage:**
  - **Operational (`computerDamaged == false`):**
    - `Target:   Quadrant [r, c]`
    - `Distance: X.X quads (ΔR: +Y, ΔC: +Z)`
    - `Vector:   Course: X.XX rad (Direction) • Warp: X.X`
    - `Actions:  [Enter] Warp  [Arrows/HJKL] Move  [Esc] Close`
  - **Damaged (`computerDamaged == true`):**
    - `Target:   Quadrant [r, c]`
    - `Distance: [CALC OFFLINE]`
    - `Vector:   Course: [CALC OFFLINE] • Warp: [CALC OFFLINE]`
    - `Actions:  [Enter] Disabled (Comp Offline)  [Arrows/HJKL] Move  [Esc] Close`
- **Key Event Handling:**
  - Pressing `Enter` when `computerDamaged == true`: emits `WarpBlockedMsg{Reason: "COMPUTER DAMAGED, USE A POCKET CALCULATOR."}`.
  - Pressing `Enter` when operational: emits `WarpToQuadrantMsg{DestQuad: m.cursor, Warp: telemetry.RecommendedWarp}`.

### 3.3 TUI Root Model Integration (`pkg/tui`)

- **`update.go` Command Routing:**
  - `case "lrscan":`
    - Dispatches `m.Game.Dispatch(engine.ActionLRScan{})`.
    - On error: logs `"LONG-RANGE SENSORS DAMAGED. Long-range scan unavailable."` to command bar.
    - On success: logs `"Long-range scan complete. Star chart updated for 3x3 surrounding quadrants."` (or starbase relay message if relayed).
  - `case "dock":`
    - Dispatches `m.Game.Dispatch(engine.ActionDock{})`.
    - Logs arrival and surveillance download: `"Docked at Starbase [%d,%d]. Systems refueled & repaired. Starbase surveillance records downloaded."`.
  - `case "dam", "damages":`
    - Inspects `m.Game.Enterprise.Devices`.
    - If all devices `0`: `"Damage report: all systems operational."`.
    - If any device `> 0`: lists non-zero devices and stardates remaining (e.g., `"Damage report: LRS: 3.2 stardates, Computer: 1.5 stardates"`).
  - `case WarpBlockedMsg:`
    - Closes modal (`m.ActiveModal = ModalNone`), logs Spock's advice: `"COMPUTER DAMAGED, USE A POCKET CALCULATOR. Manual navigation required: nav q <r> <c> [warp]"`.
- **`openGalacticChart()`:**
  - Invokes `m.GalacticChart.SetState(m.Game.Enterprise.Quad, m.Game.GalaxyChart, m.Game.ChartDiscovered, m.Game.ChartKnownBases, m.Game.Enterprise.Devices[engine.DeviceComputer] > 0)`.

---

## 4. Testing & Verification Strategy

### 4.1 Unit Tests
- **`pkg/engine/engine_test.go`:**
  - `TestProceduralGalaxyGeneration`: Verifies star counts in [1..9], exactly 3 starbases placed, remaining Klingons match `RemainingKlingons`.
  - `TestActionLRScan_Operational`: Verifies 3x3 surrounding quadrant discovery in `ChartDiscovered`.
  - `TestActionLRScan_DamagedUndocked`: Verifies error returned when `DeviceLRSensors > 0`.
  - `TestActionLRScan_DamagedDocked`: Verifies successful execution using starbase relay.
  - `TestStarbaseSurveillanceOnDock`: Verifies 3x3 discovery perimeter around starbases and `ChartKnownBases` population.
  - `TestSaveRoundtripDiscovery`: Verifies save and load round-trips `GalaxyChart`, `ChartDiscovered`, and `ChartKnownBases`.
- **`pkg/tui/components/statuspanel/status_test.go`:**
  - `TestStatusPanel_RadarLrsDamaged`: Verifies header displays `[LRS OFFLINE]` and surrounding cells display `???`.
  - `TestStatusPanel_RadarLrsDamagedDocked`: Verifies normal numbers displayed when docked.
- **`pkg/tui/components/galacticchart/chart_test.go`:**
  - `TestGalacticChart_KnownBaseRendering`: Verifies unvisited known bases render as `.1.`.
  - `TestGalacticChart_ComputerDamagedTelemetry`: Verifies `[CALC OFFLINE]` rendering in footer.
  - `TestGalacticChart_ComputerDamagedEnterLockout`: Verifies `WarpBlockedMsg` emitted on `Enter`.
- **`pkg/tui/model_test.go`:**
  - `TestModel_LRScanCommand`: Verifies `lrscan` execution and command bar feedback.
  - `TestModel_DockSurveillance`: Verifies surveillance download feedback.
  - `TestModel_DynamicDamageReport`: Verifies `dam` command lists actual damaged devices.

### 4.2 Golden Snapshot Tests (`tests/tui_golden_test.go`)
- Maintain 100% snapshot integrity at strict 80x24 dimensions.
- Add `TestTUIGolden_LrsDamagedDashboard` to lock in visual rendering of the degraded radar view.
- Update `modal_galactic_chart.golden` if cell styling updates for known bases.

### 4.3 CI Gates
- `go test -v -race ./...` (100% pass, 0 data races).
- `ctest --preset debug` (100% pass).
- No compiled binary artifacts or generated files committed.
