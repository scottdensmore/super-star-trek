# Galactic Star Chart & Quadrant Navigation Design Specification

## 1. Overview & Problem Statement

In the Super Star Trek TUI, the galaxy consists of an 8x8 grid of Quadrants (each containing an 8x8 grid of Sectors). Players currently face key visibility and ergonomic limitations when maneuvering across the galaxy:
1. **Lack of Location Situational Awareness**: The current quadrant coordinates (`[Row, Col]`) are not permanently shown anywhere on the main dashboard; once initial log messages scroll out of the 4-line event buffer, the player has no indication of where they are.
2. **Cryptic 3x3 Radar**: The 3x3 Radar in the status panel renders raw numbers (e.g. `105`) with neither coordinate labels nor explanation of the classic `K-B-S` digit encoding (Hundreds = Klingons, Tens = Starbases, Ones = Stars).
3. **No Visual Galactic Map**: The `chart` command currently only logs a static message without presenting the galaxy. Players cannot inspect explored quadrants or plan multi-quadrant voyages.
4. **Navigational Distance Calculation**: Players have to manually calculate quadrant distance and bearings for vector movement, or guess how far away a quadrant is.

This specification introduces:
- A permanent **Location Readout** and upgraded **3x3 Radar Guide** in the Status Panel.
- An interactive **8x8 Galactic Star Chart Modal** (`pkg/tui/components/galacticchart`) with arrow/vim cursor navigation, live distance/course/warp telemetry, and quick-warp on `Enter`.
- Global hotkey `Ctrl+M` (and `chart` command) integration into the root Bubble Tea model and Spock command palette.
- Enhanced in-game navigation guidance for `help nav` and `help chart`.

---

## 2. Architecture & Components

```
                      ┌─────────────────────────────────┐
                      │         tui.Model               │
                      │  (Root Bubble Tea State Machine)│
                      └────────────────┬────────────────┘
                                       │
        ┌──────────────────────────────┼──────────────────────────────┐
        ▼                              ▼                              ▼
┌──────────────┐             ┌──────────────────┐           ┌────────────────────┐
│ StatusPanel  │             │   GalacticChart  │           │     CommandBar     │
│  Component   │             │   Modal Overlay  │           │     Component      │
├──────────────┤             ├──────────────────┤           ├────────────────────┤
│• Location    │             │• 8x8 Galaxy Grid │           │• 'chart' command   │
│  [Quad, Sec] │             │• Cursor Nav      │           │• 'help nav'        │
│• 3x3 Radar   │             │• Dist/Vector HUD │           │• 'help chart'      │
│  w/ Coords   │             │• [Enter] to Warp │           │• History & Tab     │
└──────────────┘             └──────────────────┘           └────────────────────┘
```

### Component Boundaries
1. **`pkg/tui/components/statuspanel`**:
   - Displays `LOCATION: Quad [r, c]  Sec [r, c]` at the top of the panel.
   - Displays surrounding quadrant coordinates and `[K-B-S]` legend in the 3x3 Radar box.
2. **`pkg/tui/components/galacticchart`**:
   - Pure UI component adhering to Elm/Bubble Tea architecture (`Init`, `Update`, `View`).
   - Renders a 64x18 modal dialog containing the 8x8 Galaxy Star Chart and live navigation telemetry.
   - Dispatches `WarpToQuadrantMsg{DestQuad: engine.Coord}` on `Enter` and `CloseChartMsg{}` on `Esc`.
3. **`pkg/tui` (Root Model)**:
   - Maintains `ActiveModal` state machine (`ModalGalacticChart`).
   - Handles global hotkey `Ctrl+M` and commands (`chart`).
   - Uses `compositeOverlay` to render the star chart centered over the dashboard.
   - Intercepts `WarpToQuadrantMsg` to execute `engine.ActionMove{DestQuad: msg.DestQuad, Warp: 1.0}`.
4. **`pkg/tui/components/commandpalette`**:
   - Updates `CHART` entry to reflect interactive map functionality (`Ctrl+M`).

---

## 3. UI Layout & Visual Specifications

### 3.1 Status Panel Location & Radar Readout
In `pkg/tui/components/statuspanel/status.go`:
```text
LOCATION: Quad [3, 5]   Sec [2, 6]
CONDITION GREEN
Stardate: 2508.0   Time Remaining: 30.0
Energy:   [██████████] 5000/5000
Shields:  [░░░░░░░░░░]    0/2500
Torpedoes: [TORP: 10/10]
SUBSYSTEM REPAIR STATUS:
Warp:      OK  Tubes:             OK
SRS:       OK  Damage Control:    OK
LRS:       OK  Shields:           OK
Phasers:   OK  Computer:          OK
RADAR (QUADRANTS ±1)  [K-B-S]:
      4    5    6
  2  000  105  000
  3  000 <003> 012
  4  000  000  000
```
- **Location line**: Styled using `styles.GaugeLabel` for `"LOCATION: "` and `styles.Prompt` for `Quad [r, c]` and `Sec [r, c]`.
- **Radar Column & Row Numbers**: Adjacent coordinates derived from `Enterprise.Quad`. Out-of-bounds coordinates display `***`.

### 3.2 Galactic Star Chart Modal Layout (64x18)
In `pkg/tui/components/galacticchart`:
```text
┌───────────────── GALACTIC STAR CHART ─────────────────┐
│     1     2     3     4     5     6     7     8       │
│ 1  ···   ···   ···   ···   ···   ···   ···   ···      │
│ 2  ···   ···   105   ···   ···   ···   ···   ···      │
│ 3  ···   ···  [003]  012   ···   ···   ···   ···      │
│ 4  ···   ···   ···   ···   ···   ···   ···   ···      │
│ 5  ···   ···   ···   ···   ···   ···   ···   ···      │
│ 6  ···   ···   ···   ···   ···   ···   ···   ···      │
│ 7  ···   ···   ···   ···   ···   ···   ···   ···      │
│ 8  ···   ···   ···   ···   ···   ···   ···   ···      │
├───────────────────────────────────────────────────────┤
│ Target: Quad [2, 3]  •  Dist: 1.0 quads (ΔR: -1, ΔC: 0)│
│ Course: 1.57 rad (North)  •  Warp: 1.0                │
│ [Enter] Warp   [Arrows/HJKL] Move Cursor   [Esc] Close│
└───────────────────────────────────────────────────────┘
```
- **Border & Box**: Double or rounded panel styled with `theme.Styles().Panel`.
- **Current Position Indicator**: If cursor is on Enterprise's current quadrant, telemetry reads: `Target: Quad [3, 3]  •  Current Position`.
- **Target on Remote Quadrant**:
  - Distance: $\text{Dist} = \sqrt{\Delta r^2 + \Delta c^2}$ quadrants.
  - Course Angle: $\theta = \text{atan2}(-\Delta r, \Delta c)$ in radians $[0, 2\pi)$.
  - Cardinal Direction: Bearing name (e.g. North, North-East, East, South, etc.).
  - Recommended Warp: Rounded to 1 decimal place (minimum 1.0 for quadrant jump).

---

## 4. Keybindings & Modal Interactions

| Key | Modal Context | Action |
|---|---|---|
| `Ctrl+M` | Global / Dashboard | Opens Galactic Star Chart modal; blurs command bar. |
| `c` or `m` | Dashboard (Empty Input) | Opens Galactic Star Chart modal; blurs command bar. |
| `Up` / `k` | Galactic Chart Modal | Moves cursor one row North (clamped at row 1). |
| `Down` / `j` | Galactic Chart Modal | Moves cursor one row South (clamped at row 8). |
| `Left` / `h` | Galactic Chart Modal | Moves cursor one col West (clamped at col 1). |
| `Right` / `l` | Galactic Chart Modal | Moves cursor one col East (clamped at col 8). |
| `Enter` | Galactic Chart Modal | Emits `WarpToQuadrantMsg{DestQuad: cursor}`, executes warp movement, closes modal. |
| `Esc` | Galactic Chart Modal | Emits `CloseChartMsg{}`, closes modal, refocuses command bar. |

---

## 5. Mathematical Formulas

Given current quadrant $Q_1 = (r_1, c_1)$ and target quadrant $Q_2 = (r_2, c_2)$:
1. $\Delta r = r_2 - r_1$ (positive is South, negative is North)
2. $\Delta c = c_2 - c_1$ (positive is East, negative is West)
3. Euclidean Distance:
   $$D = \sqrt{\Delta r^2 + \Delta c^2}$$
4. Course Angle $\theta$ (consistent with `pkg/engine/geometry.go` and `actions.go` where $dr = -\sin(\theta)$ and $dc = \cos(\theta)$):
   $$\theta = \text{atan2}(-\Delta r, \Delta c)$$
   If $\theta < 0$, $\theta = \theta + 2\pi$.
5. Cardinal direction labels:
   - $\theta \approx 0.0$: East
   - $\theta \approx 0.79$: North-East
   - $\theta \approx 1.57$: North
   - $\theta \approx 2.36$: North-West
   - $\theta \approx 3.14$: West
   - $\theta \approx 3.93$: South-West
   - $\theta \approx 4.71$: South
   - $\theta \approx 5.50$: South-East

---

## 6. Testing & Verification Strategy

### 6.1 Unit Tests
- `pkg/tui/components/statuspanel/status_test.go`:
  - Verify `LOCATION: Quad [r, c]  Sec [r, c]` is rendered with valid coordinates.
  - Verify 3x3 Radar displays coordinates and `[K-B-S]` header.
- `pkg/tui/components/galacticchart/chart_test.go`:
  - Cursor navigation and clamping (1..8 in both axes).
  - Distance and bearing calculations for all cardinal and intercardinal directions.
  - Explored vs unexplored quadrant glyph rendering (`···` vs `KBS`).
  - View dimensions: strictly 64 columns wide x 18 rows high.
  - Theme switching and non-nil styles.
  - Message dispatch on `Enter` (`WarpToQuadrantMsg`) and `Esc` (`CloseChartMsg`).
- `pkg/tui/model_test.go`:
  - `Ctrl+M` hotkey activates `ModalGalacticChart`.
  - `chart` command activates `ModalGalacticChart`.
  - Selecting a target and pressing `Enter` closes modal and warps Enterprise to the target quadrant.
  - `CloseChartMsg` closes modal and refocuses command bar.
- `pkg/tui/components/commandpalette/palette_test.go`:
  - Assert `CHART` entry metadata and description.

### 6.2 Golden Tests
- `tests/tui_golden_test.go`:
  - Add `TestTUIGolden_ModalGalacticChart` snapshot test.
  - Update modern, lcars, and crt dashboard golden snapshots with new Status Panel location readout and radar header.
- Run `go test -v -race ./...` (100% passing, 0 race conditions).
- Run `ctest --preset debug` (100% passing).

---

## 7. Non-Functional Requirements & Constraints
- Pure Go 1.26+ standard library + `bubbletea` + `lipgloss`.
- Zero changes or regressions to existing `pkg/engine` simulation logic.
- ANSI-aware visual compositing ensures the modal floats cleanly centered over the background game view.
- 80x24 terminal guard preserved.
