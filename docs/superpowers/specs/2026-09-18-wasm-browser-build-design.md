# WebAssembly Browser Build & Retro Terminal Design Specification

- **Date:** 2026-09-18
- **Status:** Approved
- **Target Branch:** `scottdensmore/feat/wasm-browser-build`

---

## 1. Executive Summary & Goals

Super Star Trek currently provides a classic teletype C interface and a rich Go Charm Bubbletea terminal user interface. This specification defines the architecture, data structures, and web interface for a **zero-install WebAssembly (WASM) edition** running in any modern web browser.

### Key Goals
1. **Zero Install & Universal Access:** Anyone can navigate to a web page and immediately play Super Star Trek in an authentic terminal emulator.
2. **Deterministic Go Engine:** Reuses the core headless `pkg/engine` simulation logic compiled directly to `GOOS=js GOARCH=wasm`.
3. **Non-Blocking Asynchronous Bridge:** Uses Go's `syscall/js` to expose a clean event-driven interface (`sstInit`, `sstCommand`, `sstSave`, `sstLoad`) so browser rendering and animations remain 100% smooth without thread locking.
4. **Authentic xterm.js Terminal Presentation:** Provides a retro terminal experience with selectable color palettes (Amber phosphor `#FFB000`, Matrix green `#33FF33`, or Full ANSI color), cursor management, line history, and CRT aesthetic touches.
5. **Session Persistence:** Integrates with browser `localStorage` to allow saving and resuming games across sessions.
6. **Isolated Web Assets:** All client files reside in `web/` for zero-configuration hosting on GitHub Pages, Netlify, or any static HTTP server.

---

## 2. Architecture & Components

```
┌────────────────────────────────────────────────────────────┐
│                       Web Browser                          │
│                                                            │
│  ┌───────────────────────┐      ┌───────────────────────┐  │
│  │   xterm.js Terminal   │◄────►│   web/app.js Bridge   │  │
│  │  (80x24 Canvas / DOM) │      │  (Keyboard / History) │  │
│  └───────────────────────┘      └───────────┬───────────┘  │
│                                             │              │
│                       window.sstCommand()   │              │
│                       window.sstInit()      │              │
│                                             ▼              │
│  ┌──────────────────────────────────────────────────────┐  │
│  │                    web/sst.wasm                      │  │
│  │                                                      │  │
│  │   ┌─────────────────────┐   ┌────────────────────┐   │  │
│  │   │     cmd/wasm        │   │    pkg/engine      │   │  │
│  │   │  (Session Bridge)   │──►│ (GameState / Sim)  │   │  │
│  │   └──────────┬──────────┘   └────────────────────┘   │  │
│  │              │                                       │  │
│  │              ▼                                       │  │
│  │   ┌─────────────────────┐   ┌────────────────────┐   │  │
│  │   │   Output Formatter  │   │   pkg/tui/parser   │   │  │
│  │   │  (ANSI Teletype)    │   │ (Command Dispatch) │   │  │
│  │   └─────────────────────┘   └────────────────────┘   │  │
│  └──────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────┘
```

### Component Details

1. **`cmd/wasm/main.go`**:
   - WebAssembly entrypoint.
   - Initialized via `wasm_exec.js`.
   - Registers JavaScript hooks on `window` and enters a keep-alive channel block (`select {}`).

2. **`cmd/wasm/session.go`**:
   - `Session` struct encapsulates:
     - `game *engine.GameState`: The current simulation state.
     - `history []string`: Command history log.
     - `output io.Writer`: Buffer capturing ANSI-formatted responses.
     - `rules engine.GameRules`: Active difficulty and gameplay settings.
   - Methods:
     - `NewSession(seed int64, difficulty engine.DifficultyProfile) *Session`
     - `Execute(input string) string`: Parses command, dispatches to `game`, formats events/status, and returns rendered ANSI text.
     - `Save() (string, error)`: Exports state to serialized JSON.
     - `Load(data string) error`: Imports state from serialized JSON.

3. **`cmd/wasm/formatter.go`**:
   - Formats engine events and telemetry into classic ANSI strings:
     - `FormatSRS(g *engine.GameState) string`: Renders 8×8 quadrant map with Enterprise `<E>`, Klingons `+K+`, Starbases `>B<`, Stars ` * `, plus right-hand telemetry summary (Stardate, Condition, Energy, Shields, Torpedoes).
     - `FormatLRS(g *engine.GameState) string`: Renders 3×3 long-range radar scan matrix with border lines.
     - `FormatChart(g *engine.GameState) string`: Renders 8×8 galactic chart with discovered/known base annotations.
     - `FormatDamages(g *engine.GameState) string`: Formats damaged subsystem repair schedules.
     - `FormatCombatEvents(events []engine.Event) string`: Torpedo flight trajectory, hits, phaser damage readouts, Klingon return fire, and destruction notices.

4. **`web/app.js` & `web/index.html`**:
   - Initializes `xterm.js` instance with fit addon.
   - Loads and runs `web/sst.wasm` via standard Go `Go` runner in `wasm_exec.js`.
   - Listens to terminal `onData` / `onKey`:
     - Buffers player typing into an input line.
     - Handles Enter: echoes newline, passes line to `window.sstCommand(line)`, writes returned ANSI text into `term.write()`.
     - Handles Up/Down arrows: cycles previous command history into the prompt line.
     - Handles Backspace / Delete: edits line buffer and redraws terminal cursor.
   - Connects `window.sstSave` / `window.sstLoad` to `window.localStorage.setItem('sst_save_game', ...)`.

---

## 3. JavaScript / WebAssembly Interface Contract

The Go WASM binary exposes the following functions on the browser global `window` object:

```typescript
interface Window {
  // Initialize a new game session with optional seed and difficulty
  sstInit: (seed?: number, difficulty?: string) => string;

  // Execute a command string and return formatted ANSI output
  sstCommand: (line: string) => string;

  // Export current game state as JSON string
  sstSave: () => string;

  // Import game state from JSON string
  sstLoad: (jsonState: string) => boolean;

  // Reset or restart session
  sstReset: () => string;
}
```

---

## 4. Teletype ANSI Output Specifications

All outputs written to the terminal adhere to standard ANSI escape codes for broad compatibility with `xterm.js`:
- **Colors:**
  - `\x1b[32m` (Green) — Normal readings, stars, Condition Green.
  - `\x1b[33m` (Yellow) — Condition Yellow, warning messages, energy caution.
  - `\x1b[31;1m` (Bold Red) — Condition Red, Klingons, torpedo impacts, alarms.
  - `\x1b[36m` (Cyan) — Starbases, Enterprise, chart grid lines.
  - `\x1b[0m` (Reset) — Resets attributes at end of lines.
- **Line Endings:**
  - All newlines are normalized to `\r\n` to guarantee correct carriage return behavior in `xterm.js`.

---

## 5. Persistence & Browser Integration

- **Command `save [slot]`:**
  - Calls `sstSave()`, serializing `engine.GameState` into JSON.
  - `app.js` stores the JSON in `localStorage.setItem('sst_slot_' + slot, json)`.
  - Output: `SAVED TO BROWSER STORAGE [SLOT 1]`.
- **Command `load [slot]`:**
  - `app.js` retrieves `localStorage.getItem('sst_slot_' + slot)`.
  - Passes JSON to `sstLoad(json)`.
  - Output: `GAME RESTORED FROM BROWSER STORAGE [SLOT 1]`.

---

## 6. Build, Tooling & Distribution

### 1. Build Script (`scripts/build-wasm.sh`)
```bash
#!/usr/bin/env bash
set -euo pipefail

GOROOT="$(go env GOROOT)"
mkdir -p web

# Copy wasm_exec.js from Go distribution if not present or newer
if [ -f "$GOROOT/misc/wasm/wasm_exec.js" ]; then
    cp "$GOROOT/misc/wasm/wasm_exec.js" web/wasm_exec.js
elif [ -f "$GOROOT/lib/wasm/wasm_exec.js" ]; then
    cp "$GOROOT/lib/wasm/wasm_exec.js" web/wasm_exec.js
fi

# Build optimized WASM binary
echo "Compiling cmd/wasm to web/sst.wasm..."
GOOS=js GOARCH=wasm go build -trimpath -ldflags="-s -w" -o web/sst.wasm ./cmd/wasm
echo "WASM build complete: $(ls -lh web/sst.wasm | awk '{print $5}')"
```

### 2. Local Preview Server (`cmd/wasm/serve/main.go` or Python script)
Provides a zero-config local HTTP server with correct MIME types (`application/wasm`) on port 8080:
```bash
go run ./cmd/wasm/serve
# Or: python3 -m http.server -d web 8080
```

---

## 7. Testing & Verification Strategy

1. **Go Unit Testing (`cmd/wasm/session_test.go`):**
   - Independent of `syscall/js` (using build-tag separation or interface abstraction):
     - `TestSession_InitialSRS`: Verifies initial short-range scan output and coordinates.
     - `TestSession_Navigation`: Verifies warp move updates Enterprise sector and stardate.
     - `TestSession_Combat`: Verifies torpedo firing, hit messages, and Klingon destruction.
     - `TestSession_Shields`: Verifies energy transfers.
     - `TestSession_SaveLoad`: Verifies complete JSON serialization and deserialization integrity.
2. **Build Verification (`scripts/build-wasm.sh`):**
   - Must build cleanly without errors or warnings under `GOOS=js GOARCH=wasm`.
3. **Repository Gates:**
   - `go test -v -race ./...` passes 100%.
   - `ctest --preset debug` and `bash tests/golden.sh ./build/debug/sst` pass 100%.
