# Super Star Trek: Next-Gen Go + Charmbracelet Architecture RFC

## Overview & Motivation

This specification defines the architecture, design, and roadmap for transitioning **Super Star Trek** to a modern **Go** implementation powered by the **Charmbracelet** ecosystem ([Bubble Tea](https://github.com/charmbracelet/bubbletea), [Lip Gloss](https://github.com/charmbracelet/lipgloss), [Bubbles](https://github.com/charmbracelet/bubbles), inspired by [Crush](https://github.com/charmbracelet/crush)).

This document serves as the design specification for RFC issue [#219](https://github.com/scottdensmore/super-star-trek/issues/219) and coordinates features [#215](https://github.com/scottdensmore/super-star-trek/issues/215) (Interactive Save Browser), [#216](https://github.com/scottdensmore/super-star-trek/issues/216) (Readline History & Autocomplete), [#217](https://github.com/scottdensmore/super-star-trek/issues/217) (TrueColor & Themes), and [#218](https://github.com/scottdensmore/super-star-trek/issues/218) (Cross-Platform Distribution).

### Core Goals
1. **Single Zero-Dependency Static Binary**: Compiled via Go (`go build`), cross-compiling effortlessly across Linux, macOS (Intel & Apple Silicon), and Windows with no C toolchain prerequisites.
2. **Mathematical & Gameplay Fidelity**: 100% adherence to original combat mathematics, sensor formulas, and stardate progression, validated against recorded golden fixtures.
3. **Declarative Elm Architecture**: Eliminate curses resizing race conditions, imperative cursor placement bugs, and terminal-driver edge cases by adopting Bubble Tea's functional `Model` -> `Update` -> `View` loop.
4. **Rich Interactive Overlays**: Introduce floating HUD modals, targeting reticles, Spock command palettes, and visual damage schematics without sacrificing terminal responsiveness.
5. **Dual-Mode Operation**: Default to the modern Charmbracelet TUI while retaining a dedicated `--classic` / `--plain` mode for piping, headless scripted play, and byte-for-byte golden verification.
6. **Preservation of the C Baseline**: The modernized C version remains permanently tagged at `v1.0.0` and maintained on branch `c-main`.

---

## 1. Architecture & Repository Layout

### Development Branch Strategy
- The C codebase baseline is tagged at `v1.0.0` and preserved on branch `c-main`.
- Go modernization work takes place on feature branch `scottdensmore/feat/go-modernization`.
- Once full parity and verification gates pass, this branch merges to `main`.

### Repository Structure
```
super-star-trek/
├── cmd/
│   └── sst/
│       └── main.go              # CLI entry point, flag parsing (--classic, --seed, --theme)
├── pkg/
│   ├── engine/                  # Pure headless game engine (Zero terminal/UI imports)
│   │   ├── state.go             # GameState, Quadrant, Sector, Enterprise, Klingon structs
│   │   ├── prng.go              # Deterministic PRNG matching original sequence
│   │   ├── actions.go           # Action interface and typed command actions
│   │   ├── events.go            # Typed event definitions emitted by actions
│   │   ├── combat.go            # Phaser dropoff math, torpedo ballistics, shield absorption
│   │   ├── nav.go               # Warp courses, impulse maneuvering, quadrant transitions
│   │   ├── damage.go            # Subsystem casualty distribution and repair math
│   │   └── save.go              # .TRK binary and JSON save/thaw serialization
│   ├── classic/                 # Teletype classic frontend & golden test runner
│   │   ├── subscriber.go        # Translates engine events to authentic 1970s mainframe text
│   │   └── cli.go               # Plain line-by-line REPL for scripting and pipes
│   └── tui/                     # Modern Charmbracelet TUI (Bubble Tea)
│       ├── model.go             # Root tea.Model state container
│       ├── view.go              # Split Dashboard declarative view layout
│       ├── update.go            # Message dispatch (KeyMsg, MouseMsg, WindowSizeMsg)
│       ├── theme/               # Theme engine & color definitions
│       │   └── theme.go         # TrueColor 24-bit Lip Gloss styles (LCARS, CRT, Starfleet)
│       └── components/          # Reusable Bubble Tea UI components
│           ├── sectorgrid/      # Interactive 8x8 Short-Range Scan with mouse click/hover
│           ├── statuspanel/     # Energy/Shield progress meters & ship condition
│           ├── overlays/        # Floating modals: Targeting HUD, Spock palette, Saves
│           └── commandbar/      # Autocomplete textinput with history buffer
├── tests/
│   ├── golden/                  # Recorded teletype test fixtures (preserved from C baseline)
│   └── golden_test.go           # Parity test runner validating classic mode output
├── go.mod
└── go.sum
```

---

## 2. Engine Core Specification (`pkg/engine`)

The engine is completely isolated from terminal libraries, rendering code, and standard I/O.

### State Representation (`pkg/engine/state.go`)
- **`Enterprise`**:
  - `Quadrant [2]int`, `Sector [2]int` (1-indexed coordinates: 1..8)
  - `Energy float64`, `Shields float64`, `Torpedoes int`
  - `Condition ConditionType` (`ConditionGreen`, `ConditionYellow`, `ConditionRed`, `ConditionDocked`)
  - `Devices [NumDevices]DeviceState`: status (`Operational`, `Damaged`) and remaining repair time in stardates
  - `LifeSupport float64`
- **`Galaxy`**:
  - `Chart [8][8]QuadrantSummary`: Klingon count, Starbase presence, Stars, Explored bitmask
  - `CurrentQuadrant QuadrantState`: 8x8 sector array containing entity IDs (`EntityEmpty`, `EntityEnterprise`, `EntityKlingon`, `EntityCommander`, `EntitySuperCommander`, `EntityStarbase`, `EntityStar`, `EntityPlanet`, `EntityBlackHole`)
  - `RemainingKlingons int`, `RemainingStarbases int`
  - `Stardate float64`, `TimeRemaining float64`
- **`Config`**:
  - `Skill SkillLevel` (`Novice`, `Fair`, `Good`, `Expert`, `Emeritus`)
  - `Seed int64`: PRNG seed for reproducible tournaments and testing

### Action / Event Contract (`pkg/engine/actions.go` & `events.go`)
State mutation is strictly governed by the Command Pattern:

```go
type Action interface {
    Validate(state *GameState) error
    Execute(state *GameState) ([]Event, error)
}
```

#### Standard Actions:
- `ActionMove{Course float64, Warp float64}`: Computes vector, sector bounds, obstacles, and time/energy consumption.
- `ActionFireTorpedo{Target [2]int, Angle float64}`: Computes ballistic trajectory through the 8x8 sector grid.
- `ActionFirePhasers{Energy float64, ManualAllocation map[int]float64}`: Computes distance attenuation ($1/d$) and shield impacts.
- `ActionTransferShields{Amount float64}`: Transfers power between main energy banks and defensive shields.
- `ActionDock{}`: Verifies adjacency to a Starbase and replenishes torpedoes, energy, and shields.
- `ActionFreeze{Path string}` / `ActionThaw{Path string}`: Persists and restores game state.

#### Typed Event Stream:
Executing an Action emits events for consumption by subscribers:
- `EventTorpedoLaunched{Origin [2]int, Angle float64}`
- `EventTorpedoHit{Target [2]int, TargetType EntityType, Damage float64, Destroyed bool}`
- `EventPhaserFired{TotalEnergy float64}`
- `EventKlingonCounterAttack{EnemyID int, Damage float64}`
- `EventConditionChanged{From ConditionType, To ConditionType}`
- `EventSubsystemDamaged{Device DeviceID, RepairTime float64}`
- `EventSubsystemRepaired{Device DeviceID}`
- `EventGameOver{Reason GameOverReason, Score float64}`

---

## 3. Classic Mode & Golden Parity (`pkg/classic`)

To ensure mathematical and behavioral continuity with 50 years of Super Star Trek history:
- `pkg/classic/subscriber.go` subscribes to the engine's event stream.
- Formats each event into exact, byte-for-byte strings matching the C `proutf` / `proutfn` output.
- `cmd/sst --classic` executes the game in standard teletype mode:
  - Consumes stdin line-by-line.
  - Emits pure ASCII text without ANSI escape sequences.
  - Compatible with pipes, redirection, and terminal emulators of any size.
- `tests/golden_test.go` automatically runs all recorded fixtures in `tests/golden/` against the Go engine in classic mode, asserting zero regressions.

---

## 4. Modern Charmbracelet TUI (`pkg/tui`)

The interactive TUI is constructed with Charmbracelet's Elm architecture (`tea.Model`):

### Model State (`pkg/tui/model.go`)
```go
type Model struct {
    engine      *engine.GameState
    theme       theme.Theme
    width       int
    height      int
    
    // Sub-components
    commandBar  textinput.Model
    history     []string
    historyIdx  int
    
    // Active Modals / Overlays
    activeModal ModalType // None, Targeting, SpockPalette, SaveBrowser, DamageSchematic
    modalState  any
    
    // Tactical HUD state
    selectedSector [2]int
    targetLock     *TargetLockInfo
    recentMessages []string
}
```

### Layout: Split Dashboard (`pkg/tui/view.go`)
Rendered declaratively via Lip Gloss:
- **Left Pane (50% Width)**:
  - **8x8 Sector Grid**:
    - Glyphs: `<E>` (Enterprise, bold Cyan), `+K+` (Klingon, Alert Red), `>B<` (Starbase, Green), ` * ` (Star, Yellow).
    - Mouse Support: Click cell to select coordinate; double-click or enter to target.
    - Targeting Reticle: Highlights trajectory line when aiming torpedoes.
- **Right Pane (50% Width)**:
  - **Ship Telemetry**:
    - Animated progress bars (`bubbles/progress`) for Energy (`[████████░░] 3800/4000`) and Shields (`[██████░░░░] 1200/2000`).
    - Alert Banner (`CONDITION GREEN`, `YELLOW`, or pulsating `RED`).
    - Device health indicators with repair time bars.
  - **Mini Long-Range Sensor Box**: 3x3 surrounding quadrant sensor data (`: 003 : 105 : 002 :`).
- **Bottom Pane (Full Width)**:
  - **Command Bar**: Textinput supporting inline command autocomplete (`PHA`, `TOR`, `NAV`), command history recall (Up/Down arrows), and hotkey hints.
  - **Recent Log**: 4–6 lines of scrolling combat messages and Spock tactical advice.

### Overlays & Modals (Crush Inspired)
Rendered as floating Lip Gloss containers composited over the dashboard:
1. **Target Lock HUD**:
   - Opens when clicking an enemy or pressing `T`.
   - Displays computed range, required bearing, hit probability, and quick-fire buttons (`[Enter: Fire Torpedo] [P: Phasers] [Esc: Cancel]`).
2. **Spock Command Palette (`Ctrl+P` or `/`)**:
   - Fuzzy-searchable modal list (`bubbles/list`) indexing all game commands, manual help topics, galaxy reports, and settings.
3. **Interactive Save Game Browser Modal ([#215](https://github.com/scottdensmore/super-star-trek/issues/215))**:
   - Lists `.TRK` files with timestamps, skill levels, and stardates, allowing 1-click loading.
4. **Damage Control Schematic**:
   - Wireframe Enterprise layout showing subsystem placement and engineering repair queues.

### Theming System (`pkg/tui/theme/theme.go`)
Full 24-bit TrueColor support with instant hotkey toggling (`F2` or theme flag):
- **LCARS**: Authentic Star Trek 24th-century palette (`#FF9900` amber, `#CC6699` purple, `#FFCC66` peach, `#3399CC` teal) with rounded pill borders.
- **Phosphor CRT**: Nostalgic monochrome terminal glow (`#33FF33` phosphor green or amber on `#0A0A0A` black).
- **Starfleet Modern**: High-contrast modern dark mode (`#0A192F` navy, `#00E5FF` electric cyan, `#FF3344` alert red, `#FFD700` gold).

---

## 5. Verification & Testing Strategy

1. **Unit Tests (`go test -v -race ./...`)**:
   - Core combat attenuation, shield penetration, and casualty formulas.
   - Torpedo collision bounds and quadrant boundary wrapping.
   - Save file serialization and deserialization compatibility with `.TRK` files.
2. **Golden Output Parity (`tests/golden_test.go`)**:
   - Automated replay of golden fixtures through `pkg/classic`.
   - Asserts exact string equivalence against recorded C baseline fixtures.
3. **Headless TUI Testing (`teatest`)**:
   - Synthetic keypress and mouse event feeding to `pkg/tui/model.go`.
   - Window resize tests asserting clean re-rendering at terminal dimensions from 80x24 up to 200x60 without panics or clipping.
4. **Static Analysis & Linting**:
   - `golangci-lint run`: Enforces standard Go conventions, staticcheck, and formatting.

---

## 6. Implementation Milestones

| Milestone | Scope | Deliverables |
|---|---|---|
| **M1: Engine Core & Classic CLI** | Port C game logic to `pkg/engine`; implement `pkg/classic` teletype subscriber | 100% test pass on `tests/golden_test.go` |
| **M2: Charm TUI Dashboard & Themes** | Build `pkg/tui` Split Dashboard with Bubble Tea, Lip Gloss layout, and TrueColor themes | Functional interactive terminal play with theme support ([#217](https://github.com/scottdensmore/super-star-trek/issues/217)) |
| **M3: Overlays & Mouse Targeting** | Implement floating Target Lock HUD, Spock Command Palette, and mouse click handling | Full hybrid control support ([#219](https://github.com/scottdensmore/super-star-trek/issues/219)) |
| **M4: Save Browser & Distribution** | Add Save Browser modal ([#215](https://github.com/scottdensmore/super-star-trek/issues/215)), command history ([#216](https://github.com/scottdensmore/super-star-trek/issues/216)), and `goreleaser` packaging ([#218](https://github.com/scottdensmore/super-star-trek/issues/218)) | Production cross-platform binaries for Linux, macOS, and Windows |
