# Interactive Starfleet Technical Manual & Codex Modal Design

## 1. Overview & Goals

Super Star Trek historically provided player documentation via a 75KB raw text file (`sst.doc`) and teletype command dumps. In the modernized Go/Charm TUI, the `help` command simply prints 1–2 lines into the scrolling event log, providing minimal tactical context for new and returning players.

This specification introduces the **Interactive Starfleet Technical Manual & Codex Modal (`pkg/tui/components/manual`)**:
1. **LCARS-Style Two-Pane Library Computer Layout**: An interactive 66-column by 18-row floating modal featuring a 20-column Chapter Index on the left and a 43-column scrollable Reader Pane on the right.
2. **8 Core Starfleet Curriculum Chapters**: Comprehensive guides covering Ship Systems & Subsystems, Flight Mechanics & Navigation, Weapons & Combat Physics, Deflector Shields & Damage Control, Starbase Logistics & Surveillance, Enemy Tactics & Tactical Cloaking, the Authentic 15-Rule Scoring Engine, and a full Command & Hotkey Cheatsheet.
3. **Dual-Pane Navigation & Focus Management**: Intuitive navigation with `Tab` pane toggling, `↑`/`↓` line scrolling, `PgUp`/`PgDn` paging, `1`–`8` numeric chapter jumps, and clean dismissal via `Esc`, `q`, `F1`, or `?`.
4. **Unified Command & Hotkey Triggers**: Seamless activation via `help`, `man`, `manual`, `doc`, `guide`, `codex`, `F1`, or `?` (when the command line is empty), with instant topic jump support (e.g. `help tor`, `man nav`).
5. **Strict 80×24 Dimension Compliance**: Perfect adherence to terminal layout budgets, ANSI-safe line formatting with intact borders, and visual golden regression snapshots.

---

## 2. Architecture & Data Structures

### Component Location
The component lives in `pkg/tui/components/manual/`:
- `manual.go`: Data model, chapter registry, view rendering, and keyboard updates.
- `manual_test.go`: Unit tests for dimensions, focus management, chapter navigation, and scroll bounds.

### Data Structures (`pkg/tui/components/manual/manual.go`)

```go
package manual

import (
	tea "github.com/charmbracelet/bubbletea"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

// FocusPane represents which pane currently receives directional navigation keys.
type FocusPane int

const (
	FocusChapters FocusPane = iota
	FocusContent
)

// Chapter represents an individual section of the Starfleet Technical Manual.
type Chapter struct {
	ID       string   // Unique identifier (e.g. "combat", "nav")
	Number   int      // 1-indexed chapter number (1..8)
	Title    string   // Full chapter title (e.g. "Weapons & Combat")
	ShortTag string   // Sidebar label (e.g. "[3] COMBAT")
	Lines    []string // Pre-formatted lines wrapped to <= 43 characters
}

// CloseModalMsg is emitted when the player requests modal dismissal.
type CloseModalMsg struct{}

// Model encapsulates the Technical Manual component state.
type Model struct {
	width         int
	height        int
	theme         theme.Theme
	chapters      []Chapter
	selectedIdx   int       // Currently selected chapter (0..7)
	scrollOffsets []int     // Per-chapter vertical scroll offsets
	focus         FocusPane // Active focus pane
}
```

---

## 3. UI Layout & Wireframe (66×18 Box)

The modal is constructed to fit the exact **66 columns wide by 18 rows high** dimension budget matching all existing modals (`ModalTargetLock`, `ModalGalacticChart`, `ModalDamageSchematic`, `ModalHallOfFame`).

### Dimension Breakdown
- **Outer Box**: Width 66, Height 18.
- **Top Border (Row 1)**: Width 66.
- **Inner Content Rows (Rows 2–17, 16 rows)**:
  - Left border glyph `│` (1 col).
  - Left Chapter Index: 20 cols.
  - Vertical Divider `│`: 1 col.
  - Right Content Reader: 43 cols.
  - Right border glyph `│` (1 col).
  - Total inner width: $1 + 20 + 1 + 43 + 1 = 66$ cols.
- **Bottom Border (Row 18)**: Width 66.

### ASCII Wireframe

```text
┌─ [F1] STARFLEET TECHNICAL MANUAL & LIBRARY COMPUTER ───────────┐
│ CHAPTERS           │ CHAPTER 3: WEAPONS & COMBAT       [01/42] │
│                    │ ───────────────────────────────────────── │
│   [1] SYSTEMS      │ 1. PHOTON TORPEDOES                       │
│   [2] NAVIGATION   │ Ballistic projectile with 40-step vector  │
│ ▶ [3] COMBAT       │ tracing. Trajectory stops upon collision  │
│   [4] SHIELDS      │ with any solid obstacle or vessel.        │
│   [5] STARBASES    │ Direct Sector:   tor <r> <c>              │
│   [6] TACTICS      │ Bearing Course:  tor <radians>           │
│   [7] SCORING      │ Auto Target-Lock: Press [T]               │
│   [8] COMMANDS     │                                           │
│                    │ 2. PHASER BANKS                           │
│ ────────────────── │ Beam weapons dividing fired energy across │
│ [Tab] Focus Reader │ all Klingons present in quadrant.         │
│ [1-8] Direct Jump  │ Damage attenuates inversely: D = E / dist │
│                    │ ▲ [↑/↓, J/K to scroll 14 lines] ▼         │
└─ [Tab: Pane]  [↑/↓: Move]  [1-8: Jump]  [Esc/Q/F1: Close] ─────┘
```

---

## 4. Chapter Curriculum & Content

The manual contains 8 pre-formatted chapters:

### Chapter 1: Ship Systems & Subsystem Operations (`systems`)
- **USS Enterprise Subsystems**: Warp Engines, Short-Range Sensors (SRS), Long-Range Sensors (LRS), Phaser Controls, Photon Torpedo Tubes, Damage Control, Deflector Shields, and Library Computer.
- **Subsystem Casualties**: Operating limitations when damaged (e.g. SRS offline, LRS blackout, Torpedo tubes jammed, phasers disabled, navigation without computer).
- **Life Support & Reserves**: Energy drains, alert states (Green, Yellow, Red, Docked), and casualty accumulation.

### Chapter 2: Flight Mechanics & Astrogation (`nav`)
- **Astrogation Grid**: 8×8 Quadrant galaxy coordinates ($1..8$) vs. 8×8 Sector local coordinates ($1..8$).
- **Warp vs. Impulse**: Vector courses (0.0=East, 1.57=North, 3.14=West, 4.71=South) and direct jumps (`nav q <r> <c>`, `nav s <r> <c>`).
- **Energy Equations**: Distance $D = \sqrt{\Delta R^2 + \Delta C^2}$; power consumption $E = 2 \times \text{dist} \times \text{warp}$, doubled when shields are raised.
- **Navigational Hazards**: Stars, planets, asteroid barriers, and black hole gravity wells.

### Chapter 3: Weapons & Combat Mathematics (`combat`)
- **Photon Torpedoes**: 40-step trajectory tracing, collision mechanics, line-of-sight physics, and ammunition limits.
- **Phaser Physics**: Discharged energy divided equally across all enemies in the quadrant; inverse distance attenuation formula ($D = E / \text{dist}$).
- **Target Lock HUD**: Target lock auto-aiming via key `T` or clicking enemy vessels; bearing angle calculations.

### Chapter 4: Deflector Shields & Damage Control (`shields`)
- **Shield Dynamics**: Bipolar energy transfer (`she <amount>`); shield absorption vs. direct hull damage.
- **Crew Casualties**: Crew mortality mechanics when shields buckle under disruptor or torpedo fire.
- **Repair Multipliers**: In-flight repair pacing vs. accelerated Starbase 4× drydock repairs (`docfac = 0.25`).

### Chapter 5: Starbase Logistics & Surveillance (`starbases`)
- **Docking Protocol**: Proximity rules (must be in adjacent sector), shields lowered automatically upon mooring.
- **Servicing**: Instant energy restoration to 5000 units, torpedo complement restocked to 10.
- **Surveillance Networks**: Full, Classic, Local, and Blackout surveillance modes and their galactic chart discovery effects.

### Chapter 6: Tactical Threats & Enemy Doctrine (`tactics`)
- **Klingon Vessels**: Standard raiders, weapon yields, and return-fire algorithms.
- **Commanders & Cloaking**: Tactical cloaking device, targeting lock lockout, disruptor ambush tactics, decloaking on weapon fire or shield hits.
- **Super-Commanders**: Fleet coordination, base invasions, and offensive strikes.
- **Romulan Neutral Zone**: Patrol parameters, plasma torpedoes, and prisoner surrender opportunities.

### Chapter 7: Scoring Engine & Starfleet Ranks (`scoring`)
- **The 15 Scoring Rules**: Point yields for enemy classes (+10 Klingon, +50 Commander, +200 Super-Commander, +20 Romulan, +1 surrender).
- **Kill-Rate Formula**: $500 \times \text{killRate}$ with minimum 5.0 stardates clamp on loss to prevent inflated scores.
- **Win Bonus & Penalties**: Win bonus ($100 \times \text{SkillLevel}$); penalties for lost starbases (-100), distress calls (-45), destroyed planets (-10), destroyed stars (-5), and casualties (-1).
- **8 Rank Tiers**: Rank thresholds from `[DISHONOR]` to `[FADM]` (Fleet Admiral).

### Chapter 8: Command Reference & Keyboard Cheatsheet (`commands`)
- **Command Bar Syntax**: `nav`, `tor`, `pha`, `she`, `doc`, `chart`, `dam`, `saves`, `score`, `opts`, `theme`, `call`, `quit`.
- **Direct Hotkeys**: `Ctrl+P` (Palette), `Ctrl+M` / `C` (Chart), `D` (Damage Schematic), `H` (Hall of Fame), `O` (Options), `F1` / `?` (Manual), `F2` (Theme Switch).
- **Mouse Controls**: Sector selection, enemy click targeting, starbase click docking.

---

## 5. Keyboard Navigation & Focus Matrix

The modal supports smooth dual-pane navigation:

| Key Input | When Left Pane (Chapters) Focused | When Right Pane (Reader) Focused |
|---|---|---|
| `Tab` / `Shift+Tab` | Switch focus to Reader Pane | Switch focus to Chapter List |
| `←` / `h` | *No-op* | Return focus to Chapter List |
| `→` / `l` / `Enter` | Switch focus to Reader Pane | *No-op* |
| `↑` / `k` | Move chapter selection up (wraps 1..8) | Scroll up 1 line (`offset--`) |
| `↓` / `j` | Move chapter selection down (wraps 1..8) | Scroll down 1 line (`offset++`) |
| `PgUp` / `b` / `Ctrl+U` | Previous chapter | Scroll up 10 lines |
| `PgDn` / `Space` / `Ctrl+D` | Next chapter | Scroll down 10 lines |
| `Home` / `g` | Jump to Chapter 1 | Scroll to line 0 (top of chapter) |
| `End` / `G` | Jump to Chapter 8 | Scroll to bottom of chapter |
| `1` – `8` | Jump directly to Chapter $N$ | Jump directly to Chapter $N$ |
| `Esc` / `q` / `F1` / `?` | Emit `CloseModalMsg` (dismiss modal) | Return focus to Chapter List (or dismiss on `Esc`/`Q`) |

---

## 6. Root TUI Integration

### 1. Model State (`pkg/tui/model.go`)
- Add `ModalManual` to `ModalType` enum.
- Add `Manual manual.Model` to `Model` struct.
- Initialize in `NewModel` via `manual.New(th, 66, 18)`.

### 2. Update Dispatch (`pkg/tui/update.go`)
- In `Update(msg)`:
  - When `m.ActiveModal == ModalManual`: delegate to `m.Manual.Update(msg)` and return `(m, cmd)`.
  - Handle `manual.CloseModalMsg`: set `m.ActiveModal = ModalNone` and focus `m.CommandBar`.
  - When `m.ActiveModal == ModalNone` and command line is empty:
    - Key `'?'` or `"f1"` calls `m.openManual("")`.
- In `handleCommand(text)`:
  - Commands `"help"`, `"man"`, `"manual"`, `"doc"`, `"codex"`, `"guide"` calls `m.openManual("")`.
  - Topic-directed commands `"help <topic>"` / `"man <topic>"` calls `m.openManual(topic)`.
- Helper `openManual(topic string) (Model, tea.Cmd)`:
  - If `topic` maps to an existing chapter, calls `m.Manual.SelectChapter(topic)`.
  - Sets `m.ActiveModal = ModalManual` and blurs `CommandBar`.
- In `applyTheme`: calls `m.Manual.SetTheme(th)`.

### 3. View Compositing (`pkg/tui/view.go`)
- In `renderDashboard` switch:
  ```go
  case ModalManual:
      modalView = m.Manual.View()
  ```
- Composited centered over the background dashboard using `compositeOverlay(dashboard, modalView, m.Width, m.Height)`.

---

## 7. Testing & Verification Strategy

1. **Unit Tests (`pkg/tui/components/manual/manual_test.go`)**:
   - `TestManual_DimensionsAndLayoutBudget`: Asserts exact 66×18 dimension budget across all chapters and scroll offsets.
   - `TestManual_BorderIntegrity`: Asserts every row starts and ends with `│` without character truncation.
   - `TestManual_ChapterNavigationAndJumps`: Exercises `1`–`8` numeric keys, `↑`/`↓` selection wrapping, and `SelectChapter(topic)`.
   - `TestManual_DualPaneFocus`: Tests `Tab`, `Enter`, and arrow keys transitioning focus between `FocusChapters` and `FocusContent`.
   - `TestManual_ScrollBoundsClamping`: Tests that vertical scrolling clamps at line 0 and `maxOffset`.
   - `TestManual_DismissalKeys`: Asserts `CloseModalMsg` emitted on `Esc`, `Q`, `F1`, and `?`.
   - `TestManual_ThemePropagation`: Tests style updates when `SetTheme(th)` is called.
2. **Root TUI Integration Tests (`pkg/tui/model_test.go`)**:
   - `TestModel_Manual_OpenAndDismiss`: Verifies command triggers (`help`, `man`, `doc`), hotkey triggers (`F1`, `?`), 80×24 dimension budget, and clean dismissal.
   - `TestModel_Manual_TopicJumps`: Verifies `help tor` and `man nav` open directly to the targeted chapter.
3. **Golden Snapshot Tests (`tests/tui_golden_test.go`)**:
   - `TestTUIGolden_ModalManual`: Generates and verifies golden snapshots:
     - `tests/golden/tui/modal_manual_systems.golden` (Chapter 1)
     - `tests/golden/tui/modal_manual_combat.golden` (Chapter 3)
4. **Full Test Verification**:
   - `go test -v -race ./...`
   - `ctest --preset debug`
   - `bash tests/golden.sh ./build/debug/sst`

