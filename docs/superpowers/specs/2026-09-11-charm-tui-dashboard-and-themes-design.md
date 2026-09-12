# Super Star Trek: Milestone 2 Charmbracelet TUI Dashboard & Themes Design Spec

## Overview
This specification details the architecture, component hierarchy, command pipeline, and TrueColor theming system for Milestone 2 of Super Star Trek's Go modernization.

Milestone 2 delivers an interactive, declarative terminal user interface using the **Charmbracelet** ecosystem ([Bubble Tea](https://github.com/charmbracelet/bubbletea), [Lip Gloss](https://github.com/charmbracelet/lipgloss), [Bubbles](https://github.com/charmbracelet/bubbles)), implementing a Split Dashboard layout, live command execution bar, and dynamic theme switching.

---

## Architecture & Package Structure

```
pkg/tui/
├── model.go                      # Root tea.Model lifecycle (Init, Update, View)
├── view.go                       # Top-level layout composition & terminal resize guard
├── update.go                     # Message handling (KeyMsg, WindowSizeMsg, CommandSubmittedMsg)
├── parser.go                     # Player command tokenizer & engine.Action mapper
├── parser_test.go                # Comprehensive command parser test table
├── model_test.go                 # Bubble Tea lifecycle & resize tests
├── theme/
│   ├── theme.go                  # Theme interface, Palette struct, Next() cycle logic
│   ├── modern.go                 # Starfleet Modern (default high-contrast dark theme)
│   ├── lcars.go                  # 24th century LCARS palette & rounded styles
│   ├── crt.go                    # 1970s monochrome phosphor CRT
│   └── theme_test.go             # Theme integrity and cycling unit tests
└── components/
    ├── sectorgrid/
    │   ├── grid.go               # 8x8 Short-Range scan visualizer with 1-8 row/col axes & glyphs
    │   └── grid_test.go          # Grid rendering and entity glyph tests
    ├── statuspanel/
    │   ├── status.go             # Energy/shield bars, condition banner, 3x3 sensor box, devices
    │   └── status_test.go        # Telemetry calculations and status formatting tests
    └── commandbar/
        ├── bar.go                # Embedded bubbles/textinput + scrolling event log
        └── bar_test.go           # Input buffering and message queue tests
```

---

## Dependencies & Module Setup
The following packages are added to `go.mod`:
- `github.com/charmbracelet/bubbletea` (v1.3.4 or latest stable)
- `github.com/charmbracelet/lipgloss` (v1.0.0 or latest stable)
- `github.com/charmbracelet/bubbles` (v0.20.0 or latest stable)

All imports strictly conform to Go standard library + Charmbracelet libraries. Zero CGO dependencies.

---

## Component Specifications

### 1. Root Model (`pkg/tui/model.go`)
- **State Container:**
  ```go
  type Model struct {
      Game       *engine.GameState
      Theme      theme.Theme
      Width      int
      Height     int
      Grid       sectorgrid.Model
      Status     statuspanel.Model
      CommandBar commandbar.Model
      Err        error
  }
  ```
- **Lifecycle:**
  - `Init() tea.Cmd`: Initializes cursor blink and initial sub-component commands.
  - `Update(msg tea.Msg) (tea.Model, tea.Cmd)`: Handles `tea.WindowSizeMsg`, `tea.KeyMsg` (`F2` theme cycling, `Ctrl+C`/`Esc` exit), and internal command execution.
  - `View() string`: Renders size guard if `Width < 80 || Height < 24`; otherwise calls `renderDashboard()`.

### 2. Sector Grid (`pkg/tui/components/sectorgrid`)
- Displays current quadrant 8x8 sector grid with coordinates:
  - Top header: column numbers `  1   2   3   4   5   6   7   8  `
  - Left gutter: row numbers `1 ` through `8 `
- Entity Glyphs (3 characters wide):
  - `<E>`: USS Enterprise
  - `+K+`: Klingon
  - `>B<`: Starbase
  - ` * `: Star
  - ` O `: Planet
  - ` @ `: Black hole
  - ` . `: Empty space

### 3. Status Panel (`pkg/tui/components/statuspanel`)
- Condition Banner: `CONDITION GREEN`, `CONDITION YELLOW`, `CONDITION RED`, `DOCKED`.
- Animated Progress Bars (`bubbles/progress`):
  - Energy: `[████████░░] 4250/5000`
  - Shields: `[██████░░░░] 1500/2500`
- Torpedo Inventory: `[TORP: 10/10]`
- Subsystem device repair countdown table (Warp, SRS, LRS, Phasers, Tubes, Damage Control, Shields, Computer).
- 3x3 surrounding quadrant sensor radar box displaying summary densities.

### 4. Command Bar (`pkg/tui/components/commandbar`)
- Embeds `bubbles/textinput.Model` configured with prompt `COMMAND> `.
- Maintains a 4-line scrolling buffer for recent combat feedback and tactical notices.
- Dispatches `CommandSubmittedMsg{Text: string}` on Enter.

---

## Command Parser (`pkg/tui/parser.go`)

Translates text strings into `engine.Action`:
- `nav <course> <warp>`: Translates to `engine.ActionMove{Course: c, Warp: w}`
- `nav <sector_r> <sector_c>`: Translates to `engine.ActionMove{DestSector: Coord{r, c}, Warp: 1.0}`
- `tor <course>`: Translates to `engine.ActionFireTorpedo{Angle: course}`
- `tor <target_r> <target_c>`: Translates to `engine.ActionFireTorpedo{Target: Coord{r, c}}`
- `pha <energy>`: Translates to `engine.ActionFirePhasers{Energy: energy}`
- `she <amount>`: Translates to `engine.ActionShields{Amount: amount}`
- `doc`: Translates to `engine.ActionDock{}`
- `theme [name]`: Switches theme directly (`modern`, `lcars`, `crt`)
- `help`: Appends command reference to message log
- `quit` / `exit`: Dispatches `tea.Quit`

---

## Theming Engine (`pkg/tui/theme`)

### Interface & Palette
```go
type Theme interface {
    Name() string
    Styles() Styles
    Next() Theme
}
```

### Palettes
1. **Starfleet Modern (Default):**
   - High-contrast 24-bit TrueColor
   - Base dark: `#0B0F19`, Panel borders: `#1E293B`, Cyan accent: `#00E5FF`, Red alert: `#FF3344`, Gold: `#FFD700`.
2. **LCARS:**
   - 24th-century LCARS aesthetic
   - Amber: `#FF9900`, Purple: `#CC6699`, Peach: `#FFCC66`, Teal: `#3399CC`, rounded borders.
3. **Phosphor CRT:**
   - Nostalgic 1970s mainframe green glow
   - Primary: `#33FF33`, Dim: `#116611`, Deep background: `#050B05`.

---

## Verification & Acceptance Criteria
1. `go test -v -race ./...` runs all engine, classic, and TUI tests with 100% pass and zero race conditions.
2. `sst` launched without flags displays the interactive Split Dashboard in Starfleet Modern theme.
3. `sst --classic` launches classic line-by-line mode unchanged.
4. `sst --theme lcars` launches with LCARS theme.
5. Pressing `F2` dynamically cycles themes without disrupting game state.
6. Submitting valid commands updates the game state, grid, telemetry meters, and log output.
7. Terminal windows < 80x24 cleanly display the resize notice without clipping or panic.
8. C CI presets (`ci-debug`, `ci-release`) remain 100% green.
