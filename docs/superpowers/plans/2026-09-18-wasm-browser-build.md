# WebAssembly Browser Build & Retro Terminal Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build an authentic, zero-install WebAssembly edition of Super Star Trek runnable in any modern web browser with an interactive retro xterm.js terminal emulator, ANSI telemetry formatting, and `localStorage` persistence.

**Architecture:** A standalone Go WebAssembly module in `cmd/wasm` exposes an asynchronous `syscall/js` interface (`sstInit`, `sstCommand`, `sstSave`, `sstLoad`, `sstReset`) driving the headless `pkg/engine` simulation. The browser frontend in `web/` connects `xterm.js` to the WASM runtime with line buffering, command history, and retro phosphor styling.

**Tech Stack:** Go 1.26+ (`GOOS=js GOARCH=wasm`), `syscall/js`, xterm.js 5.x, HTML5/CSS3, JavaScript (ES6+).

**Spec:** [`docs/superpowers/specs/2026-09-18-wasm-browser-build-design.md`](file:///home/scottdensmore/Developer/scottdensmore/super-star-trek/docs/superpowers/specs/2026-09-18-wasm-browser-build-design.md)

## Global Constraints

- Target Go version: Go 1.26+ standard library.
- WebAssembly compilation target: `GOOS=js GOARCH=wasm`.
- Zero compiled binaries (e.g. `sst.wasm`) committed to git tracking (`web/*.wasm` git-ignored).
- Maintain 100% passing tests across Go (`go test -v -race ./...`) and C (`ctest --preset debug`, `tests/golden.sh`).
- Classic teletype CLI mode (`--classic`), Bubbletea TUI, and tournament test suites remain untouched and passing.
- Work committed on feature branch `scottdensmore/feat/wasm-browser-build`.

---

### Task 1: Core WebAssembly Session & Command Dispatcher (`cmd/wasm/session.go`)

**Files:**
- Create: `cmd/wasm/session.go`
- Test: `cmd/wasm/session_test.go`

**Interfaces:**
- Consumes: `pkg/engine` (`NewGameWithOptions`, `GameState`, `Action*`, `DifficultyProfile`), `pkg/tui` (`ParseCommand`)
- Produces:
  - `type Session struct`
  - `func NewSession(seed int64, difficulty engine.DifficultyProfile) *Session`
  - `func (s *Session) Execute(input string) string`
  - `func (s *Session) Save() (string, error)`
  - `func (s *Session) Load(data string) error`

- [ ] **Step 1: Write the failing test**

In `cmd/wasm/session_test.go`:
```go
package main

import (
	"strings"
	"testing"

	"github.com/scottdensmore/super-star-trek/pkg/engine"
)

func TestSession_NewAndExecute(t *testing.T) {
	s := NewSession(12345, engine.ProfileNormal)
	if s == nil || s.Game() == nil {
		t.Fatalf("expected non-nil session and game")
	}

	// Test help command
	helpOut := s.Execute("help")
	if !strings.Contains(helpOut, "COMMANDS:") {
		t.Errorf("expected help output to list commands, got: %s", helpOut)
	}

	// Test status command
	statusOut := s.Execute("srs")
	if !strings.Contains(statusOut, "CONDITION") || !strings.Contains(statusOut, "<E>") {
		t.Errorf("expected short-range scan output, got: %s", statusOut)
	}

	// Test shields command
	shieldOut := s.Execute("she 500")
	if !strings.Contains(shieldOut, "Shields") {
		t.Errorf("expected shield transfer output, got: %s", shieldOut)
	}
}

func TestSession_SaveAndLoad(t *testing.T) {
	s1 := NewSession(12345, engine.ProfileNormal)
	s1.Execute("she 500")

	savedData, err := s1.Save()
	if err != nil {
		t.Fatalf("Save() failed: %v", err)
	}

	s2 := NewSession(99999, engine.ProfileHardcore)
	if err := s2.Load(savedData); err != nil {
		t.Fatalf("Load() failed: %v", err)
	}

	if s2.Game().Enterprise.Shields != s1.Game().Enterprise.Shields {
		t.Errorf("loaded shields = %v, want %v", s2.Game().Enterprise.Shields, s1.Game().Enterprise.Shields)
	}
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `go test -v ./cmd/wasm -run TestSession`
Expected: FAIL (`NewSession` undefined)

- [ ] **Step 3: Write minimal implementation**

In `cmd/wasm/session.go`:
```go
package main

import (
	"encoding/json"
	"fmt"
	"strings"

	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui"
)

// Session manages a single player's game session.
type Session struct {
	game    *engine.GameState
	history []string
}

// NewSession creates and initializes a new Session with the specified seed and difficulty.
func NewSession(seed int64, difficulty engine.DifficultyProfile) *Session {
	if seed == 0 {
		seed = 12345
	}
	rules := engine.DefaultRulesForProfile(difficulty)
	game := engine.NewGameWithOptions(seed, engine.SkillGood, engine.LengthMedium, rules)
	game.PopulateQuadrant(game.Enterprise.Quad, game.Enterprise.Sector)
	return &Session{
		game:    game,
		history: make([]string, 0),
	}
}

// Game returns the underlying GameState.
func (s *Session) Game() *engine.GameState {
	return s.game
}

// Execute processes a user input string and returns formatted teletype output.
func (s *Session) Execute(input string) string {
	trimmed := strings.TrimSpace(input)
	if trimmed == "" {
		return ""
	}
	s.history = append(s.history, trimmed)

	parsed := tui.ParseCommand(trimmed)
	if parsed.Error != nil {
		return fmt.Sprintf("Error: %v\r\n", parsed.Error)
	}

	if parsed.Special != "" {
		switch strings.ToLower(parsed.Special) {
		case "help", "?":
			return "COMMANDS: nav, srs, lrs, pha, tor, she, dam, chart, com, help, quit\r\n"
		case "srs", "srscan":
			return FormatSRS(s.game)
		case "lrs", "lrscan":
			return FormatLRS(s.game)
		case "chart":
			return FormatChart(s.game)
		case "dam", "damages":
			return FormatDamages(s.game)
		case "status":
			return FormatSRS(s.game)
		default:
			return fmt.Sprintf("Unknown command: %s\r\nType 'help' for command reference.\r\n", parsed.Special)
		}
	}

	if parsed.Action != nil {
		events, err := s.game.Dispatch(parsed.Action)
		if err != nil {
			return fmt.Sprintf("Cannot execute: %v\r\n", err)
		}
		var out strings.Builder
		out.WriteString(FormatCombatEvents(events))
		if _, ok := parsed.Action.(engine.ActionMove); ok {
			out.WriteString(FormatSRS(s.game))
		}
		return out.String()
	}

	return "Invalid command.\r\n"
}

// Save serializes the game state to a JSON string.
func (s *Session) Save() (string, error) {
	data, err := json.Marshal(s.game)
	if err != nil {
		return "", err
	}
	return string(data), nil
}

// Load restores the game state from a JSON string.
func (s *Session) Load(data string) error {
	var g engine.GameState
	if err := json.Unmarshal([]byte(data), &g); err != nil {
		return err
	}
	s.game = &g
	return nil
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `go test -v ./cmd/wasm -run TestSession`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add cmd/wasm/session.go cmd/wasm/session_test.go
git commit -m "feat(wasm): implement Session struct, command parsing, and save/load"
```

---

### Task 2: Teletype ANSI Output Formatter (`cmd/wasm/formatter.go`)

**Files:**
- Create: `cmd/wasm/formatter.go`
- Modify: `cmd/wasm/session.go`
- Test: `cmd/wasm/formatter_test.go`

**Interfaces:**
- Consumes: `pkg/engine` (`GameState`, `Event*`, `Coord`)
- Produces:
  - `func FormatSRS(g *engine.GameState) string`
  - `func FormatLRS(g *engine.GameState) string`
  - `func FormatChart(g *engine.GameState) string`
  - `func FormatDamages(g *engine.GameState) string`
  - `func FormatCombatEvents(events []engine.Event) string`

- [ ] **Step 1: Write the failing test**

In `cmd/wasm/formatter_test.go`:
```go
package main

import (
	"strings"
	"testing"

	"github.com/scottdensmore/super-star-trek/pkg/engine"
)

func TestFormatSRS(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	g.PopulateQuadrant(g.Enterprise.Quad, g.Enterprise.Sector)

	out := FormatSRS(g)
	if !strings.Contains(out, "<E>") {
		t.Errorf("expected Enterprise glyph <E> in SRS, got:\n%s", out)
	}
	if !strings.Contains(out, "Stardate:") || !strings.Contains(out, "Energy:") {
		t.Errorf("expected telemetry metrics in SRS, got:\n%s", out)
	}
	if !strings.Contains(out, "\r\n") {
		t.Errorf("expected CR LF line endings for xterm.js compatibility")
	}
}

func TestFormatLRS(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	out := FormatLRS(g)
	if !strings.Contains(out, "LONG RANGE SENSOR SCAN") {
		t.Errorf("expected LRS header, got:\n%s", out)
	}
}

func TestFormatCombatEvents(t *testing.T) {
	events := []engine.Event{
		engine.EventTorpedoFired{Origin: engine.Coord{3, 3}, Target: engine.Coord{3, 7}, Angle: 0.0},
		engine.EventTorpedoHit{Target: engine.Coord{3, 7}, Damage: 350, Destroyed: true},
	}
	out := FormatCombatEvents(events)
	if !strings.Contains(out, "TORPEDO TRACK") || !strings.Contains(out, "DESTROYED") {
		t.Errorf("expected torpedo tracking and destruction message, got:\n%s", out)
	}
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `go test -v ./cmd/wasm -run "TestFormatSRS|TestFormatLRS|TestFormatCombatEvents"`
Expected: FAIL (`FormatSRS` undefined)

- [ ] **Step 3: Write minimal implementation**

In `cmd/wasm/formatter.go`:
```go
package main

import (
	"fmt"
	"strings"

	"github.com/scottdensmore/super-star-trek/pkg/engine"
)

const (
	ansiReset  = "\x1b[0m"
	ansiGreen  = "\x1b[32m"
	ansiYellow = "\x1b[33m"
	ansiRed    = "\x1b[31;1m"
	ansiCyan   = "\x1b[36m"
	ansiBold   = "\x1b[1m"
	ansiDim    = "\x1b[2m"
)

// FormatSRS renders an 8x8 sector grid paired with a telemetry status panel.
func FormatSRS(g *engine.GameState) string {
	if g == nil {
		return ""
	}
	var b strings.Builder
	b.WriteString(fmt.Sprintf("%s=== SHORT RANGE SENSOR SCAN [%d,%d] ===%s\r\n", ansiCyan, g.Enterprise.Quad.Row(), g.Enterprise.Quad.Col(), ansiReset))

	for r := 1; r <= 8; r++ {
		b.WriteString(fmt.Sprintf("%s%d%s ", ansiDim, r, ansiReset))
		for c := 1; c <= 8; c++ {
			coord := engine.Coord{r, c}
			if coord == g.Enterprise.Sector {
				b.WriteString(fmt.Sprintf("%s<E>%s ", ansiCyan, ansiReset))
			} else if g.HasKlingonAt(coord) {
				b.WriteString(fmt.Sprintf("%s+K+%s ", ansiRed, ansiReset))
			} else if g.HasStarbaseAt(coord) {
				b.WriteString(fmt.Sprintf("%s>B<%s ", ansiYellow, ansiReset))
			} else if g.HasStarAt(coord) {
				b.WriteString(fmt.Sprintf("%s * %s ", ansiGreen, ansiReset))
			} else {
				b.WriteString(" .  ")
			}
		}

		// Append telemetry sidebar
		switch r {
		case 1:
			b.WriteString(fmt.Sprintf("  Stardate:   %.1f", g.Stardate))
		case 2:
			condColor := ansiGreen
			switch g.Enterprise.Condition {
			case engine.ConditionYellow:
				condColor = ansiYellow
			case engine.ConditionRed:
				condColor = ansiRed
			case engine.ConditionDocked:
				condColor = ansiCyan
			}
			b.WriteString(fmt.Sprintf("  Condition:  %s%s%s", condColor, g.Enterprise.Condition.String(), ansiReset))
		case 3:
			b.WriteString(fmt.Sprintf("  Sector:     [%d,%d]", g.Enterprise.Sector.Row(), g.Enterprise.Sector.Col()))
		case 4:
			b.WriteString(fmt.Sprintf("  Energy:     %.0f / 5000", g.Enterprise.Energy))
		case 5:
			b.WriteString(fmt.Sprintf("  Shields:    %.0f / 2500", g.Enterprise.Shields))
		case 6:
			b.WriteString(fmt.Sprintf("  Torpedoes:  %d", g.Enterprise.Torpedoes))
		case 7:
			b.WriteString(fmt.Sprintf("  Klingons:   %d", g.KlingonsLeft))
		case 8:
			b.WriteString(fmt.Sprintf("  Starbases:  %d", g.StarbasesLeft))
		}
		b.WriteString("\r\n")
	}
	return b.String()
}

// FormatLRS renders a 3x3 surrounding quadrant radar scan.
func FormatLRS(g *engine.GameState) string {
	if g == nil {
		return ""
	}
	var b strings.Builder
	b.WriteString(fmt.Sprintf("%s=== LONG RANGE SENSOR SCAN ===%s\r\n", ansiCyan, ansiReset))
	b.WriteString("-------------------\r\n")
	eq := g.Enterprise.Quad
	for dr := -1; dr <= 1; dr++ {
		b.WriteString(": ")
		for dc := -1; dc <= 1; dc++ {
			qr := eq.Row() + dr
			qc := eq.Col() + dc
			if qr < 1 || qr > 8 || qc < 1 || qc > 8 {
				b.WriteString("*** : ")
			} else {
				val := g.GalaxyChart[qr][qc]
				b.WriteString(fmt.Sprintf("%03d : ", val))
			}
		}
		b.WriteString("\r\n-------------------\r\n")
	}
	return b.String()
}

// FormatChart renders the 8x8 galactic star chart.
func FormatChart(g *engine.GameState) string {
	if g == nil {
		return ""
	}
	var b strings.Builder
	b.WriteString(fmt.Sprintf("%s=== GALACTIC STAR CHART ===%s\r\n", ansiCyan, ansiReset))
	b.WriteString("    1   2   3   4   5   6   7   8\r\n")
	b.WriteString("  ---------------------------------\r\n")
	for r := 1; r <= 8; r++ {
		b.WriteString(fmt.Sprintf("%d |", r))
		for c := 1; c <= 8; c++ {
			if r == g.Enterprise.Quad.Row() && c == g.Enterprise.Quad.Col() {
				b.WriteString(fmt.Sprintf("%s%03d%s|", ansiCyan, g.GalaxyChart[r][c], ansiReset))
			} else if g.ChartDiscovered[r][c] {
				b.WriteString(fmt.Sprintf("%03d|", g.GalaxyChart[r][c]))
			} else if g.ChartKnownBases[r][c] {
				b.WriteString(fmt.Sprintf("%s.B.%s|", ansiYellow, ansiReset))
			} else {
				b.WriteString("...|")
			}
		}
		b.WriteString("\r\n  ---------------------------------\r\n")
	}
	return b.String()
}

// FormatDamages renders damaged subsystem statuses.
func FormatDamages(g *engine.GameState) string {
	if g == nil {
		return ""
	}
	var b strings.Builder
	b.WriteString(fmt.Sprintf("%s=== DAMAGE CONTROL REPORT ===%s\r\n", ansiCyan, ansiReset))
	for dev := engine.DeviceWarpEngines; dev <= engine.DeviceSubspaceRadio; dev++ {
		status := fmt.Sprintf("%sOPERATIONAL%s", ansiGreen, ansiReset)
		dmg := g.Enterprise.Damage[dev]
		if dmg > 0 {
			status = fmt.Sprintf("%sDAMAGED (Repair in %.1f stardates)%s", ansiRed, dmg, ansiReset)
		}
		b.WriteString(fmt.Sprintf("%-22s : %s\r\n", dev.String(), status))
	}
	return b.String()
}

// FormatCombatEvents formats combat events into teletype output.
func FormatCombatEvents(events []engine.Event) string {
	var b strings.Builder
	for _, ev := range events {
		switch e := ev.(type) {
		case engine.EventTorpedoFired:
			b.WriteString(fmt.Sprintf("%s[TORPEDO TRACK]%s Fired on course %.2f toward [%d,%d]\r\n", ansiYellow, ansiReset, e.Angle, e.Target.Row(), e.Target.Col()))
		case engine.EventTorpedoHit:
			if e.Destroyed {
				b.WriteString(fmt.Sprintf("%s*** KLINGON WARSHIP DESTROYED AT [%d,%d] ***%s\r\n", ansiRed, e.Target.Row(), e.Target.Col(), ansiReset))
			} else {
				b.WriteString(fmt.Sprintf("Torpedo hit Klingon at [%d,%d]: %.0f units damage\r\n", e.Target.Row(), e.Target.Col(), e.Damage))
			}
		case engine.EventPhaserFired:
			b.WriteString(fmt.Sprintf("%s[PHASERS FIRED]%s Allocated %.0f units\r\n", ansiYellow, ansiReset, e.Energy))
		case engine.EventPhaserHit:
			if e.Destroyed {
				b.WriteString(fmt.Sprintf("%s*** KLINGON DESTROYED BY PHASER FIRE ***%s\r\n", ansiRed, ansiReset))
			} else {
				b.WriteString(fmt.Sprintf("Phaser beam hit target at [%d,%d]: %.0f units\r\n", e.Target.Row(), e.Target.Col(), e.Damage))
			}
		case engine.EventShieldTransfer:
			b.WriteString(fmt.Sprintf("Deflector shields: %.0f  |  Total energy: %.0f\r\n", e.NewShields, e.NewEnergy))
		case engine.EventWarpCompleted:
			b.WriteString(fmt.Sprintf("%s[WARP ENGINES ENGAGED]%s Arrived at sector [%d,%d]\r\n", ansiGreen, ansiReset, e.Sector.Row(), e.Sector.Col()))
		}
	}
	return b.String()
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `go test -v ./cmd/wasm -run "TestFormatSRS|TestFormatLRS|TestFormatCombatEvents"`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add cmd/wasm/formatter.go cmd/wasm/formatter_test.go
git commit -m "feat(wasm): add ANSI teletype output formatting for SRS, LRS, chart, and combat"
```

---

### Task 3: WebAssembly JavaScript Syscall Bridge & Main Entrypoint (`cmd/wasm/main.go`, `cmd/wasm/bridge_js.go`, `cmd/wasm/bridge_other.go`)

**Files:**
- Create: `cmd/wasm/main.go`
- Create: `cmd/wasm/bridge_js.go` (Build tag: `//go:build js && wasm`)
- Create: `cmd/wasm/bridge_other.go` (Build tag: `//go:build !(js && wasm)`)
- Test: `cmd/wasm/main_test.go`

**Interfaces:**
- Consumes: `Session`, `syscall/js` (in WASM)
- Produces:
  - `func registerBridge()` in `bridge_js.go` (hooks `window.sstInit`, `window.sstCommand`, `window.sstSave`, `window.sstLoad`, `window.sstReset`)
  - `func registerBridge()` in `bridge_other.go` (no-op for non-wasm builds)

- [ ] **Step 1: Write the failing test**

In `cmd/wasm/main_test.go`:
```go
package main

import (
	"testing"
)

func TestBridgeCompilationAndMain(t *testing.T) {
	// Verify that registerBridge compiles and runs without panic on native arch
	registerBridge()
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `go test -v ./cmd/wasm -run TestBridgeCompilationAndMain`
Expected: FAIL (`registerBridge` undefined)

- [ ] **Step 3: Write minimal implementation**

In `cmd/wasm/bridge_other.go`:
```go
//go:build !(js && wasm)

package main

// registerBridge is a no-op when compiled for non-js/wasm architectures.
func registerBridge() {}
```

In `cmd/wasm/bridge_js.go`:
```go
//go:build js && wasm

package main

import (
	"syscall/js"

	"github.com/scottdensmore/super-star-trek/pkg/engine"
)

var activeSession *Session

func registerBridge() {
	activeSession = NewSession(12345, engine.ProfileNormal)

	js.Global().Set("sstInit", js.FuncOf(func(this js.Value, args []js.Value) any {
		seed := int64(0)
		diff := engine.ProfileNormal
		if len(args) > 0 && !args[0].IsNull() && !args[0].IsUndefined() {
			seed = int64(args[0].Int())
		}
		if len(args) > 1 && !args[1].IsNull() && !args[1].IsUndefined() {
			diff = engine.DifficultyProfile(args[1].String())
		}
		activeSession = NewSession(seed, diff)
		return FormatSRS(activeSession.Game())
	}))

	js.Global().Set("sstCommand", js.FuncOf(func(this js.Value, args []js.Value) any {
		if len(args) == 0 {
			return ""
		}
		return activeSession.Execute(args[0].String())
	}))

	js.Global().Set("sstSave", js.FuncOf(func(this js.Value, args []js.Value) any {
		s, err := activeSession.Save()
		if err != nil {
			return ""
		}
		return s
	}))

	js.Global().Set("sstLoad", js.FuncOf(func(this js.Value, args []js.Value) any {
		if len(args) == 0 {
			return false
		}
		err := activeSession.Load(args[0].String())
		return err == nil
	}))

	js.Global().Set("sstReset", js.FuncOf(func(this js.Value, args []js.Value) any {
		activeSession = NewSession(0, engine.ProfileNormal)
		return FormatSRS(activeSession.Game())
	}))
}
```

In `cmd/wasm/main.go`:
```go
package main

func main() {
	registerBridge()

	// Keep runtime active in WebAssembly
	select {}
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `go test -v ./cmd/wasm -run TestBridgeCompilationAndMain`
Expected: PASS

Run WASM build check:
`GOOS=js GOARCH=wasm go build -o /dev/null ./cmd/wasm`
Expected: PASS (exit code 0)

- [ ] **Step 5: Commit**

```bash
git add cmd/wasm/main.go cmd/wasm/bridge_js.go cmd/wasm/bridge_other.go cmd/wasm/main_test.go
git commit -m "feat(wasm): add syscall/js browser bridge and WASM main entrypoint"
```

---

### Task 4: Web Assets, Retro xterm.js Terminal & Build Pipeline (`web/`, `scripts/build-wasm.sh`)

**Files:**
- Create: `web/index.html`
- Create: `web/app.js`
- Create: `web/style.css`
- Create: `scripts/build-wasm.sh`
- Modify: `.gitignore`

**Interfaces:**
- Produces:
  - `scripts/build-wasm.sh`: Builds `web/sst.wasm` and installs `web/wasm_exec.js`.
  - `web/`: Complete standalone playable web application with xterm.js and CRT theme styling.

- [ ] **Step 1: Write `scripts/build-wasm.sh` and update `.gitignore`**

In `.gitignore`, add:
```
web/sst.wasm
web/wasm_exec.js
```

In `scripts/build-wasm.sh`:
```bash
#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

GOROOT="$(go env GOROOT)"
mkdir -p web

# Locate wasm_exec.js in Go toolchain
WASM_EXEC=""
if [ -f "$GOROOT/misc/wasm/wasm_exec.js" ]; then
    WASM_EXEC="$GOROOT/misc/wasm/wasm_exec.js"
elif [ -f "$GOROOT/lib/wasm/wasm_exec.js" ]; then
    WASM_EXEC="$GOROOT/lib/wasm/wasm_exec.js"
fi

if [ -n "$WASM_EXEC" ]; then
    cp "$WASM_EXEC" web/wasm_exec.js
    echo "Copied wasm_exec.js from $WASM_EXEC"
else
    echo "Warning: wasm_exec.js not found in GOROOT ($GOROOT)"
fi

echo "Compiling WebAssembly binary: web/sst.wasm..."
GOOS=js GOARCH=wasm go build -trimpath -ldflags="-s -w" -o web/sst.wasm ./cmd/wasm
echo "Build complete: $(ls -lh web/sst.wasm | awk '{print $5}')"
```

Make it executable: `chmod +x scripts/build-wasm.sh`

- [ ] **Step 2: Create `web/index.html`, `web/style.css`, and `web/app.js`**

In `web/style.css`:
```css
* {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
}

body {
    background-color: #0b0c10;
    color: #c5c6c7;
    font-family: monospace;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 100vh;
    overflow: hidden;
}

header {
    width: 100%;
    max-width: 900px;
    padding: 12px 20px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    background: #1f2833;
    border-radius: 8px 8px 0 0;
    border: 1px solid #45a29e;
    border-bottom: none;
}

h1 {
    font-size: 1.1rem;
    color: #66fcf1;
    letter-spacing: 2px;
}

.controls {
    display: flex;
    gap: 10px;
}

select, button {
    background: #0b0c10;
    color: #66fcf1;
    border: 1px solid #45a29e;
    padding: 4px 10px;
    border-radius: 4px;
    font-family: inherit;
    font-size: 0.85rem;
    cursor: pointer;
}

select:hover, button:hover {
    background: #45a29e;
    color: #0b0c10;
}

#terminal-container {
    width: 100%;
    max-width: 900px;
    height: 520px;
    background: #000;
    border: 1px solid #45a29e;
    border-radius: 0 0 8px 8px;
    padding: 8px;
    box-shadow: 0 0 20px rgba(102, 252, 241, 0.15);
    position: relative;
}

/* CRT scanlines effect */
.crt::after {
    content: " ";
    display: block;
    position: absolute;
    top: 0; left: 0; bottom: 0; right: 0;
    background: linear-gradient(rgba(18, 16, 16, 0) 50%, rgba(0, 0, 0, 0.25) 50%), linear-gradient(90deg, rgba(255, 0, 0, 0.03), rgba(0, 255, 0, 0.01), rgba(0, 0, 255, 0.03));
    background-size: 100% 2px, 3px 100%;
    pointer-events: none;
}
```

In `web/index.html`:
```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Super Star Trek - WebAssembly Edition</title>
    <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/@xterm/xterm@5.5.0/css/xterm.css" />
    <link rel="stylesheet" href="style.css" />
    <script src="https://cdn.jsdelivr.net/npm/@xterm/xterm@5.5.0/lib/xterm.js"></script>
    <script src="https://cdn.jsdelivr.net/npm/@xterm/addon-fit@0.10.0/lib/addon-fit.js"></script>
    <script src="wasm_exec.js"></script>
</head>
<body>
    <header>
        <h1>★ SUPER STAR TREK ★ (WASM)</h1>
        <div class="controls">
            <select id="palette-select">
                <option value="color">Full ANSI Color</option>
                <option value="green">Classic Green Phosphor</option>
                <option value="amber">Amber Phosphor</option>
            </select>
            <button id="restart-btn">Restart Game</button>
        </div>
    </header>
    <div id="terminal-container" class="crt"></div>
    <script src="app.js"></script>
</body>
</html>
```

In `web/app.js`:
```javascript
const term = new Terminal({
    cursorBlink: true,
    fontFamily: '"Courier New", Courier, monospace',
    fontSize: 14,
    theme: {
        background: '#05070a',
        foreground: '#66fcf1',
        cursor: '#66fcf1'
    }
});

const fitAddon = new FitAddon.FitAddon();
term.loadAddon(fitAddon);

const container = document.getElementById('terminal-container');
term.open(container);
fitAddon.fit();
window.addEventListener('resize', () => fitAddon.fit());

let lineBuffer = '';
const history = [];
let historyIndex = -1;

function prompt() {
    term.write('\x1b[36mCommand?\x1b[0m ');
}

term.onData(e => {
    switch (e) {
        case '\r': // Enter
            term.write('\r\n');
            if (lineBuffer.trim().length > 0) {
                history.push(lineBuffer);
                historyIndex = history.length;
                if (typeof window.sstCommand === 'function') {
                    const output = window.sstCommand(lineBuffer);
                    term.write(output);
                }
            }
            lineBuffer = '';
            prompt();
            break;
        case '\u007F': // Backspace
            if (lineBuffer.length > 0) {
                lineBuffer = lineBuffer.slice(0, -1);
                term.write('\b \b');
            }
            break;
        case '\u0003': // Ctrl+C
            term.write('^C\r\n');
            lineBuffer = '';
            prompt();
            break;
        case '\u001b[A': // Up arrow
            if (historyIndex > 0) {
                historyIndex--;
                while (lineBuffer.length > 0) {
                    term.write('\b \b');
                    lineBuffer = lineBuffer.slice(0, -1);
                }
                lineBuffer = history[historyIndex];
                term.write(lineBuffer);
            }
            break;
        case '\u001b[B': // Down arrow
            if (historyIndex < history.length - 1) {
                historyIndex++;
                while (lineBuffer.length > 0) {
                    term.write('\b \b');
                    lineBuffer = lineBuffer.slice(0, -1);
                }
                lineBuffer = history[historyIndex];
                term.write(lineBuffer);
            } else if (historyIndex === history.length - 1) {
                historyIndex = history.length;
                while (lineBuffer.length > 0) {
                    term.write('\b \b');
                    lineBuffer = lineBuffer.slice(0, -1);
                }
            }
            break;
        default:
            if (e >= ' ' && e <= '~') {
                lineBuffer += e;
                term.write(e);
            }
            break;
    }
});

// Color palettes
document.getElementById('palette-select').addEventListener('change', (e) => {
    const val = e.target.value;
    if (val === 'green') {
        term.options.theme = { background: '#020d02', foreground: '#33ff33', cursor: '#33ff33' };
    } else if (val === 'amber') {
        term.options.theme = { background: '#0e0802', foreground: '#ffb000', cursor: '#ffb000' };
    } else {
        term.options.theme = { background: '#05070a', foreground: '#66fcf1', cursor: '#66fcf1' };
    }
});

document.getElementById('restart-btn').addEventListener('click', () => {
    if (typeof window.sstReset === 'function') {
        term.clear();
        term.write(window.sstReset());
        prompt();
    }
});

// Initialize Go WASM runtime
async function initWasm() {
    term.write('\x1b[33mInitializing Starfleet Computer Interface (WASM)...\x1b[0m\r\n');
    const go = new Go();
    try {
        const result = await WebAssembly.instantiateStreaming(fetch('sst.wasm'), go.importObject);
        go.run(result.instance);
        term.write('\x1b[32mInterface ready. Subspace radio link established.\x1b[0m\r\n\r\n');
        if (typeof window.sstInit === 'function') {
            term.write(window.sstInit());
        }
        prompt();
    } catch (err) {
        term.write('\x1b[31;1mError loading WebAssembly interface: ' + err.message + '\x1b[0m\r\n');
    }
}

initWasm();
```

- [ ] **Step 3: Run the build script to verify WASM compilation**

Run: `bash scripts/build-wasm.sh`
Expected: `Build complete` (produces `web/sst.wasm` and `web/wasm_exec.js`)

- [ ] **Step 4: Run full verification suite**

```bash
go test -v -race ./...
ctest --preset debug
bash tests/golden.sh ./build/debug/sst
```
Expected: All suites pass 100%.

- [ ] **Step 5: Commit**

```bash
git add .gitignore scripts/build-wasm.sh web/index.html web/style.css web/app.js
git commit -m "feat(wasm): add retro xterm.js browser terminal interface and build pipeline"
```
