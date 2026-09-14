# Interactive Damage Control Schematic Modal Design

## 1. Overview & Goals

Super Star Trek currently summarizes subsystem damage via a brief text message in the command bar when players issue `dam` or `damages`. While functionally accurate, it does not provide an authentic Starfleet bridge console experience or intuitive spatial awareness of where damage occurred across the vessel.

This design introduces a dedicated, interactive **Damage Control Schematic Modal** (`pkg/tui/components/damageschematic`) for the Charmbracelet Bubble Tea TUI. It displays a Constitution-class ASCII wireframe silhouette with status pins mapped to physical ship compartments, coupled with a detailed telemetry table comparing in-flight vs. docked repair times and tactical impacts.

### Goals
- **Immersive Spatial Visualization**: Display an ASCII wireframe cutaway of the USS Enterprise (NCC-1701) with live device status indicators mapped to the saucer, neck, secondary engineering hull, and warp nacelles.
- **Actionable Telemetry**: Clearly present in-flight vs. docked repair times (accounting for `GameRules.RepairMultiplier` and starbase docking repair acceleration).
- **Strict Budget Compliance**: Strictly maintain the 80x24 terminal constraint, rendering as a centered 66-column by 18-row overlay via `compositeOverlay`.
- **Keyboard & Command Integration**: Seamless invocation via `dam`, `damage`, `damages` commands and `d`/`D`/`Ctrl+D` hotkeys, with single-key dismissal (`Esc`, `Enter`, `q`, `Space`, `d`).
- **100% Test Parity**: Comprehensive component tests, root TUI integration tests, and golden snapshot visual regression tests.

---

## 2. Component Architecture

The damage schematic is encapsulated as a standalone Bubble Tea component in `pkg/tui/components/damageschematic`.

### 2.1 Model & Data Structures

```go
package damageschematic

import (
    "github.com/charmbracelet/bubbles/key"
    tea "github.com/charmbracelet/bubbletea"
    "github.com/scottdensmore/super-star-trek/pkg/engine"
    "github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

// Model represents the Damage Control Schematic overlay component.
type Model struct {
    width      int
    height     int
    theme      theme.Theme
    devices    [engine.NumDevices]float64
    condition  string // "GREEN", "YELLOW", "RED", "DOCKED"
    isDocked   bool
    repairMult float64
    active     bool
}
```

### 2.2 Public API & Interface

- `New(th theme.Theme, width, height int) Model`: Constructs a new model (default 66×18) initialized with theme styles.
- `SetTheme(th theme.Theme)`: Updates styles when cycling themes (e.g. F2).
- `SetState(enterprise engine.EnterpriseState, condition string, isDocked bool, repairMult float64)`: Updates live telemetry from the active game state.
- `Update(msg tea.Msg) (Model, tea.Cmd)`: Handles dismissal keystrokes (`esc`, `enter`, `q`, `space`, `d`).
- `View() string`: Renders the bordered ANSI wireframe schematic and telemetry breakdown.

---

## 3. Physical Layout & Subsystem Mapping

### 3.1 Terminal Dimension Budget
The modal is budgeted at **66 columns wide** by **18 rows high**, fitting comfortably inside an 80×24 terminal. This ensures at least 7 columns of border margin on the left/right and 3 rows on the top/bottom, preserving the background dashboard view.

### 3.2 Physical Subsystem Compartments
The 8 canonical subsystems (`engine.DeviceID`) map to specific ship sections:

| Device ID | Subsystem Name | Tag | Compartment Location | Tactical Impact Description |
|-----------|----------------|-----|----------------------|-----------------------------|
| 0 | Warp Engines | `WARP` | Port & Starboard Nacelles | Warp factor restricted (Max Warp 0.2) |
| 1 | Short-Range Sensors | `SRS` | Upper Saucer Sensor Array | Tactical grid scan offline |
| 2 | Long-Range Sensors | `LRS` | Forward Deflector Dish | Long-range sensor degraded / offline |
| 3 | Phaser Controls | `PHAS` | Saucer Ventral Emitters | Phaser targeting & discharge offline |
| 4 | Photon Tubes | `TUB` | Forward Neck Launcher Pod | Torpedo launcher locked |
| 5 | Damage Control | `DAM` | Midship Engineering Deck | Automated repair crew coordination slow |
| 6 | Shield Control | `SHL` | Saucer Shield Perimeter | Shield modulation & transfers locked |
| 7 | Library Computer | `COMP` | Primary Saucer Core Deck | Trajectory calc & galactic chart locked |

### 3.3 ASCII Wireframe & Schematic Composition

```text
┌────────────────── DAMAGE CONTROL SCHEMATIC ──────────────────┐
│ USS ENTERPRISE  NCC-1701                ALERT STATUS: YELLOW │
│                                                              │
│          .---[SRS: OK]---.              [LRS: OK]            │
│         /    [COMP: 2.1]  \             [PHAS: OK]           │
│        |   (=) Saucer      |===[TUB: 1.4]===.                │
│         \     Bridge      /                 |                │
│          '---[SHL: OK]---'                  |                │
│                   \                         |                │
│                    \===[DAM: OK]======[WARP: OK]             │
│                                                              │
│ SUBSYSTEM          STATUS    IN-FLIGHT  DOCKED  EFFECT       │
│ Library Computer   DAMAGED     2.1 SD   0.5 SD  No Chart/Nav │
│ Photon Tubes       DAMAGED     1.4 SD   0.4 SD  Tubes Locked │
│ All other primary and tactical systems operational.          │
│ [ESC / ENTER / Q / D] Dismiss Damage Schematic               │
└──────────────────────────────────────────────────────────────┘
```

### 3.4 Theme-Aware Styling & Alert States
- **Nominal (`damage == 0.0`)**: Styled with `theme.Styles.SubsystemNormal` (green/cyan) displaying `[OK]`.
- **Damaged (`0 < damage < 2.0`)**: Styled with `theme.Styles.SubsystemDamaged` (amber/orange) displaying `[x.x]` stardates.
- **Offline / Severe (`damage >= 2.0`)**: Styled with `theme.Styles.TextWarn` / highlight styling.
- **Repair Time Scaling**:
  - In-flight repair time: `turns * repairMult` stardates.
  - Docked repair time: `0.25 * turns * repairMult` stardates (or instant if currently docked at a starbase).

---

## 4. Modal Lifecycle & TUI Integration

### 4.1 Modal Type Registry
In `pkg/tui/model.go`:
```go
const (
    ModalNone ModalType = iota
    ModalTargetLock
    ModalCommandPalette
    ModalSaveBrowser
    ModalGalacticChart
    ModalDamageSchematic // New modal type
)
```

`pkg/tui.Model` holds `DamageSchematic damageschematic.Model`.

### 4.2 Invocation & Dismissal
1. **Invocation Triggers**:
   - Commands: `dam`, `damage`, `damages` entered into the command bar.
   - Hotkeys: `d` or `D` when the command line input buffer is empty, or `Ctrl+D`.
2. **State Sync**:
   `m.DamageSchematic.SetState(m.Game.Enterprise, m.Game.Condition, m.Game.Enterprise.Docked, m.Game.Rules.RepairMultiplier)`
3. **Display**:
   `m.ActiveModal = ModalDamageSchematic` and `m.CommandBar.Blur()`.
4. **Compositing**:
   In `pkg/tui/view.go`, `compositeOverlay` composites `m.DamageSchematic.View()` centered over the dashboard.
5. **Dismissal**:
   Keys `esc`, `enter`, `q`, `space`, `d` reset `m.ActiveModal = ModalNone` and refocus the command bar.

---

## 5. Testing & Verification

1. **Unit Tests (`pkg/tui/components/damageschematic/schematic_test.go`)**:
   - Test all devices nominal: verifies all pins display `[OK]` and summary table reports all systems operational.
   - Test individual and multiple damaged devices: validates pin label formatting and in-flight vs docked repair calculations.
   - Test repair multipliers: verifies `repairMult` correctly scales stardate displays.
   - Test keyboard interaction: verifies dismissal keys close the modal.
   - Dimension assertions: confirms rendered output is strictly 66 cols × 18 rows.
2. **Integration Tests (`pkg/tui/model_test.go`)**:
   - Test opening modal via `dam` command and `d` hotkey.
   - Test closing modal via `esc` and returning focus.
   - Verify overall terminal dimension budget (dashboard + overlay = exactly 80×24).
3. **Golden Snapshot Suite (`pkg/tui/golden_test.go`)**:
   - Capture golden snapshot of `ModalDamageSchematic` overlay across Modern, LCARS, and CRT themes.
4. **Regression Gates**:
   - `go test -v -race ./...`
   - `ctest --preset debug`
   - Shell test suites (`tests/tui.sh`, `tests/golden.sh`)
