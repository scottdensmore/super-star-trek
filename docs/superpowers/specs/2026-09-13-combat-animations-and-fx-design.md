# Design Specification: Dynamic Combat Visual FX & Animation Engine

**Document:** `docs/superpowers/specs/2026-09-13-combat-animations-and-fx-design.md`  
**Date:** 2026-09-13  
**Status:** Approved  
**Target Systems:** `pkg/engine`, `pkg/tui`, `pkg/tui/anim`, `pkg/tui/components/sectorgrid`, `pkg/tui/components/optionsmodal`

---

## 1. Overview & Objectives

Super Star Trek's modern Charm Bubbletea interface provides an authentic 80×24 tactical bridge dashboard. While torpedoes and phasers resolve deterministically and immediately in the core engine (`pkg/engine`), the visual experience in the TUI is currently instantaneous: entities disappear or damage counters update without visual feedback of the projectile flight or energy discharge.

This specification defines a lightweight, non-blocking **Combat Visual FX & Animation Engine** in `pkg/tui/anim`:
- **Real-Time Torpedo Trajectory Animation:** Steps a projectile glyph (`·` -> `o` -> `O`) along the Bresenham vector path across sector grid cells.
- **Explosion & Impact Shockwaves:** Multi-frame visual bursts (`*` -> `***` -> `#*#`) upon entity destruction or obstacle collision.
- **Directional Phaser Beam Vectors:** Instantaneous visual raycast lines (`---`, `\\\`, `///`, `|||`) rendered between Enterprise and hostile Klingon/Romulan targets with shield flash brackets (`(E)`, `<K>`).
- **Condition Red Klaxon Accent:** Subtle, periodic pulse oscillation on the `RED` condition badge.
- **Player Input Interactivity & Instant Key-Skip:** Any player keypress immediately cancels/skips active animations to the final resolved state without lag or dropped keystrokes.
- **Configurable Animation Speeds:** Pacing toggle in the Options modal (`Off`, `Fast (~150ms)`, `Normal (~300ms)`, `Cinematic (~600ms)`).

---

## 2. Architecture & Data Structures (`pkg/tui/anim`)

### 2.1 Core Types
Located in `pkg/tui/anim/anim.go`:

```go
package anim

import (
	"time"

	"github.com/charmbracelet/lipgloss"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
)

// CellOverride represents a temporary visual glyph and style replacing a grid cell.
type CellOverride struct {
	Glyph string         // Exact 3-rune cell glyph (e.g. " · ", " o ", " * ", "***", "---", " \\ ")
	Style lipgloss.Style // Foreground/background styling
}

// Frame represents the visual state of all overridden cells at a single animation step.
type Frame struct {
	Overrides map[engine.Coord]CellOverride
	Duration  time.Duration
}

// Animation defines an interactive visual sequence.
type Animation interface {
	TotalDuration() time.Duration
	Frames() []Frame
	IsFinished() bool
	Step() Frame
	Skip() Frame
}

// TickMsg is dispatched via tea.Tick for frame advancement.
type TickMsg struct {
	AnimID int
	Step   int
}
```

### 2.2 Concrete Animation Implementations
1. **`TorpedoAnimation` (`pkg/tui/anim/torpedo.go`):**
   - Calculates the Bresenham line of coordinates between `start` and `target`.
   - Generates trajectory frames advancing the projectile glyph (`·` -> `o` -> `O`).
   - Generates 3 impact frames on the destination cell:
     - Frame 1: ` * ` (amber / bright yellow)
     - Frame 2: `***` (vivid red / orange)
     - Frame 3: `#*#` (dark orange / smoke settling)
2. **`PhaserAnimation` (`pkg/tui/anim/phaser.go`):**
   - Calculates the vector line between firing entity and target entity.
   - Computes appropriate directional ray glyphs along intermediate cells:
     - Horizontal: `---`
     - Vertical: ` | `
     - Diagonal down-right / up-left: ` \ `
     - Diagonal down-left / up-right: ` / `
   - Flashes intermediate cells with amber beam style for 2 frames.
   - Flashes target with shield impact brackets (`(E)`, `<K>`) in electric cyan/blue.
3. **`RedAlertPulse` (`pkg/tui/anim/pulse.go`):**
   - Mathematical cosine oscillator returning alternating high-intensity crimson and dimmed red lipgloss styles for the Condition badge on a 1.0s cycle.

### 2.3 Speed Configurations
Defined in `engine.Rules.AnimSpeed`:
- `AnimSpeedOff = 0`: 0ms duration (animations skipped instantly; no ticks dispatched).
- `AnimSpeedFast = 1`: ~40ms per frame (~150ms total).
- `AnimSpeedNormal = 2`: ~80ms per frame (~320ms total, default).
- `AnimSpeedCinematic = 3`: ~160ms per frame (~640ms total).

---

## 3. Sector Grid & Root TUI Integration

### 3.1 Sector Grid Component (`pkg/tui/components/sectorgrid`)
In `pkg/tui/components/sectorgrid/grid.go`:
- Add `animOverrides map[engine.Coord]anim.CellOverride` to `Model`.
- Add `SetAnimOverrides(overrides map[engine.Coord]anim.CellOverride)`.
- During `renderRow()`, check if coordinate `(r, c)` exists in `m.animOverrides`.
  - If present, render the override glyph and style, strictly clamped to 3 runes width.
  - If absent, render the standard entity glyph or background dot.
  - Preserves exact 33×19 sector grid dimension budget.

### 3.2 Root TUI Event Loop (`pkg/tui/model.go`, `pkg/tui/update.go`)
- **State Fields:**
  - `activeAnim anim.Animation`
  - `animID int` (counter to invalidate stale ticks)
- **Starting Animation:**
  - When `ActionFireTorpedo` or `ActionFirePhaser` produces combat events:
    - If `m.Game.Rules.AnimSpeed != AnimSpeedOff`, construct `m.activeAnim` matching target/trajectory.
    - Dispatch `tea.Tick(frame.Duration, ...)` to step through frames.
- **Handling `TickMsg`:**
  - If `msg.AnimID == m.animID` and `m.activeAnim != nil`:
    - Advance with `frame := m.activeAnim.Step()`.
    - Apply `m.Grid.SetAnimOverrides(frame.Overrides)`.
    - If finished, set `m.activeAnim = nil`, clear grid overrides, and flush deferred combat game-over events.
    - Otherwise, schedule next tick.
- **Instant Key-Skip:**
  - When any `tea.KeyMsg` is received while `m.activeAnim != nil`:
    - Call `m.activeAnim.Skip()`, set `m.activeAnim = nil`, and clear grid overrides.
    - Immediately proceed to process the keystroke (e.g. command input, hotkey, dismissal).

---

## 4. Options Modal Integration (`pkg/tui/components/optionsmodal`)

In `pkg/tui/components/optionsmodal/optionsmodal.go`:
- Add `Combat Animation Speed` row.
- Toggle between `[Off]`, `[Fast]`, `[Normal]`, `[Cinematic]`.
- Add command shortcuts in command bar: `anim off`, `anim fast`, `anim normal`, `anim cinematic`.

---

## 5. Testing & Verification Strategy

1. **Unit Tests (`pkg/tui/anim`):**
   - Bresenham line algorithm correctness across all 8 cardinal/diagonal directions.
   - Deterministic frame sequences for `TorpedoAnimation` and `PhaserAnimation`.
   - `Skip()` returns terminal state immediately.
2. **Component Tests (`pkg/tui/components/sectorgrid`):**
   - Verifying cell overrides preserve exact 33×19 layout budget without line distortion.
   - Verifying clearing overrides restores underlying entity layout cleanly.
3. **Root TUI Integration Tests (`pkg/tui/model_test.go`):**
   - Firing torpedo initiates `m.activeAnim`.
   - Subsequent `tea.KeyMsg` cancels animation immediately and evaluates key without delay.
4. **Golden Snapshot Visual Regressions (`tests/tui_golden_test.go`):**
   - `anim_torpedo_flight.golden`: Full 80×24 view with in-flight projectile overlay.
   - `anim_phaser_beam.golden`: Full 80×24 view with phaser vector line overlay.
   - `anim_explosion.golden`: Full 80×24 view with 3-frame explosion overlay.
5. **Full Suite Verification:**
   - `go test -v -race ./...` (100% pass)
   - `ctest --preset debug` (13/13 pass)
   - `bash tests/golden.sh ./build/debug/sst` (golden OK)
