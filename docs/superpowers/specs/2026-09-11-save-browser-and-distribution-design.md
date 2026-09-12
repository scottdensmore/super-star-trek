# Milestone 4 Design Specification: Save Game Browser, Command Ergonomics & Multi-Platform Distribution

**Date:** 2026-09-11  
**Status:** Approved  
**Target:** `pkg/engine`, `pkg/tui/components/savebrowser`, `pkg/tui/components/commandbar`, `pkg/tui`, `.goreleaser.yaml`, `CMakeLists.txt`  
**Parent RFC:** [#219](https://github.com/scottdensmore/super-star-trek/issues/219)  
**Parent Design Spec:** `docs/superpowers/specs/2026-09-10-go-charm-modernization-design.md`  
**Target Issues:** [#215](https://github.com/scottdensmore/super-star-trek/issues/215), [#216](https://github.com/scottdensmore/super-star-trek/issues/216), [#218](https://github.com/scottdensmore/super-star-trek/issues/218)  

---

## 1. Executive Summary

Milestone 4 completes the modernization of Super Star Trek by delivering:
1. **Interactive Save Game Browser Modal (`pkg/tui/components/savebrowser`) ([#215](https://github.com/scottdensmore/super-star-trek/issues/215))**: A full-featured modal dialog that scans the filesystem for `*.TRK` save games, extracts rich game metadata (skill, stardates remaining, alert condition, Klingon count, timestamp), and allows 1-click thawing (`Enter`) or file deletion (`D`/`Delete` with confirmation).
2. **Readline-Style Command History & Tab Completion (`pkg/tui/components/commandbar`) ([#216](https://github.com/scottdensmore/super-star-trek/issues/216))**: Session-scoped command history recalled via `Up`/`Down` arrow keys with draft input preservation, and smart Tab auto-completion matching canonical command keywords (`p` $\to$ `pha `, `to` $\to$ `tor `, `na` $\to$ `nav `, etc.) with multi-match candidate cycling.
3. **Multi-Platform Distribution Packaging ([#218](https://github.com/scottdensmore/super-star-trek/issues/218))**:
   - `.goreleaser.yaml`: Cross-compilation for pure Go static binaries across Linux (`amd64`, `arm64`), macOS (`amd64`, `arm64`), and Windows (`amd64`), packaged with documentation (`sst.doc`, `README.md`) and SHA-256 checksums.
   - `CMakeLists.txt`: CPack configuration for classic C release packaging (`.tar.gz` and `.deb`) with standard installation targets.

---

## 2. Save Game Metadata Inspection (`pkg/engine`)

### 2.1 Engine Metadata Inspector
To preview saved games without modifying active gameplay state, `pkg/engine/save.go` introduces `InspectSave`:

```go
package engine

type SaveMetadata struct {
    Path          string
    Filename      string
    ModTime       time.Time
    Skill         SkillLevel
    Stardate      float64
    TimeRemaining float64
    Condition     ConditionType
    KlingonsLeft  int
}

// InspectSave reads a .TRK file and extracts summary metadata without mutating game state.
func InspectSave(path string) (*SaveMetadata, error)
```

`InspectSave` reads the file, parses the JSON payload into an unexported temporary struct or `GameState`, and populates `SaveMetadata`. If the file is missing or corrupted, a structured error is returned.

---

## 3. Save Game Browser Component (`pkg/tui/components/savebrowser`)

### 3.1 Responsibilities
Scans a directory for `*.TRK` files, parses their metadata, and displays an interactive modal table.

### 3.2 Model & Messages
```go
package savebrowser

type Model struct {
    theme        theme.Theme
    directory    string
    saves        []engine.SaveMetadata
    cursor       int
    deleting     bool // Confirmation mode when user presses 'D'
    errorMessage string
}

// Emitted messages:
type LoadGameMsg struct {
    Path string
}

type CloseBrowserMsg struct{}
```

### 3.3 Keybindings & Workflow
- **`Up` / `Down` / `k` / `j`**: Moves cursor through save entries.
- **`Enter`**:
  - In normal mode: emits `LoadGameMsg{Path: selected.Path}` to thaw the mission.
  - In deleting mode: deletes the selected file from disk, refreshes the listing, and exits deleting mode.
- **`D` / `Delete`**: Toggles deletion confirmation prompt on the footer line: `Delete '<filename>'? [Enter: Confirm / Esc: Cancel]`.
- **`Esc`**:
  - In deleting mode: cancels deletion mode.
  - In normal mode: emits `CloseBrowserMsg{}` to close the dialog.

### 3.4 Visual Layout
Dimensions: 62 columns x 14 rows, centered and styled via Lip Gloss themes:
```
┌──────────────── SAVED MISSIONS (Ctrl+O) ─────────────────┐
│  FILE       STARDATE  SKILL   COND    KLINGONS  MODIFIED │
│> GAME1.TRK  3421.5    GOOD    YELLOW  4         09-11    │
│  TEST.TRK   3100.0    NOVICE  GREEN   8         09-10    │
│  HARD.TRK   3540.2    EXPERT  RED     12        09-08    │
│                                                          │
│                                                          │
├──────────────────────────────────────────────────────────┤
│ [Enter] Thaw Mission  [D] Delete  [↑/↓] Select  [Esc]    │
└──────────────────────────────────────────────────────────┘
```

---

## 4. Command Bar History & Tab Completion (`pkg/tui/components/commandbar`)

### 4.1 History Buffer State
`pkg/tui/components/commandbar/bar.go` maintains session command history:
- `history []string`: Chronological list of non-empty submitted commands.
- `historyIdx int`: Position in history (-1 = editing draft input line; 0..len-1 = browsing past commands).
- `draftInput string`: Preserves uncommitted text typed by the user before pressing `Up`.

### 4.2 History Navigation Mechanics
1. **`Up`**:
   - If `historyIdx == -1` and `len(history) > 0`:
     - Stashes current input: `draftInput = textInput.Value()`.
     - Sets `historyIdx = len(history) - 1`.
     - Updates input: `textInput.SetValue(history[historyIdx])`.
   - If `historyIdx > 0`:
     - Decrements `historyIdx--`.
     - Updates input: `textInput.SetValue(history[historyIdx])`.
2. **`Down`**:
   - If `historyIdx != -1`:
     - If `historyIdx < len(history) - 1`: increments `historyIdx++`, updates input to `history[historyIdx]`.
     - If `historyIdx == len(history) - 1`: sets `historyIdx = -1`, restores `textInput.SetValue(draftInput)`.
3. **`Enter`**:
   - Appends text to `history` if non-empty and different from `history[len-1]`.
   - Resets `historyIdx = -1` and `draftInput = ""`.

### 4.3 Tab Auto-Completion Mechanics
Indexed Command Verbs:
- **Parameterized** (completes with trailing space):
  - `"nav "`, `"tor "`, `"pha "`, `"she "`, `"theme "`
- **Instant** (completes without trailing space):
  - `"doc"`, `"srscan"`, `"lrscan"`, `"status"`, `"damage"`, `"chart"`, `"target"`, `"saves"`, `"thaw"`, `"help"`, `"quit"`

**Behavior on `Tab`**:
- Prefix = `strings.ToLower(strings.TrimSpace(textInput.Value()))`.
- Filters all keywords matching prefix.
- **Single Match**: Completes input directly (e.g. `"p"` $\to$ `"pha "`).
- **Multiple Matches**:
  - Sets `tabMatches` and starts at `tabMatchIdx = 0`.
  - Replaces input with `tabMatches[0]`.
  - Consecutive `Tab` presses cycle sequentially: `tabMatchIdx = (tabMatchIdx + 1) % len(tabMatches)`.
- Typing any character, backspace, or non-Tab key resets `tabMatches`.

---

## 5. Root TUI Model Integration (`pkg/tui`)

### 5.1 Modal State Machine Expansion
In `pkg/tui/model.go`:
```go
const (
    ModalNone ModalType = iota
    ModalTargetLock
    ModalCommandPalette
    ModalSaveBrowser // Milestone 4
)

type Model struct {
    // ...
    ActiveModal    ModalType
    TargetLock     targetlock.Model
    CommandPalette commandpalette.Model
    SaveBrowser    savebrowser.Model // Milestone 4
    // ...
}
```

### 5.2 Triggers & Invocation
1. **Global Hotkey (`Ctrl+O`)**: Opens `ModalSaveBrowser`, scanning directory `.`.
2. **Commands**:
   - `saves` or bare `thaw`: Opens `ModalSaveBrowser`.
   - `thaw <filename>`: Loads specified `.TRK` file directly via `engine.Load`.
3. **Command Palette (`Ctrl+P`)**:
   - Catalog item: `SAVES — Mission Save Browser: "Browse, inspect, and load saved missions (Ctrl+O)"` *(Instant)*.

### 5.3 View Compositing & Event Routing
- In `pkg/tui/view.go`: When `ActiveModal == ModalSaveBrowser`, `compositeOverlay` composites `m.SaveBrowser.View()` centered over the dashboard.
- In `pkg/tui/update.go`:
  - When `ActiveModal == ModalSaveBrowser`, routes keys to `SaveBrowser`.
  - Handles `savebrowser.LoadGameMsg`: loads `engine.Load(msg.Path)`, updates `m.Game`, resets reticle, logs confirmation to command bar, and closes modal.
  - Handles `savebrowser.CloseBrowserMsg`: closes modal and refocuses command bar.
  - Theme switching (`F2`) propagates to `m.SaveBrowser.SetTheme(th)`.

---

## 6. Distribution Packaging

### 6.1 GoReleaser (`.goreleaser.yaml`)
Automated pure Go static builds (`CGO_ENABLED=0`):
- **Builds**:
  - `main: ./cmd/sst`, binary `sst` (or `sst.exe` on Windows).
  - Target matrix:
    - `linux/amd64`, `linux/arm64`
    - `darwin/amd64`, `darwin/arm64`
    - `windows/amd64`
- **Archives**:
  - Format: `tar.gz` for Linux and macOS; `zip` for Windows.
  - Files included: `sst.doc`, `README.md`, `LICENSE` (if present).
- **Checksums**: SHA256 in `checksums.txt`.

### 6.2 CMake CPack (`CMakeLists.txt`)
Enables classic C binary package generation for Unix/Linux:
- Installs `sst` to `bin/` and `sst.doc` to `share/doc/super-star-trek/`.
- Configures `CPack` generators: `TGZ` and `DEB`.
- Sets maintainer metadata and package descriptions.

---

## 7. Verification & Testing Strategy

1. **Unit Testing (`pkg/engine/save_test.go`)**:
   - Verify `InspectSave` parses valid metadata without altering game state.
   - Verify error handling on missing/corrupted files.
2. **Unit Testing (`pkg/tui/components/savebrowser/browser_test.go`)**:
   - Verify directory filtering for `*.TRK`.
   - Verify cursor navigation and wrapping.
   - Verify `LoadGameMsg` emission on `Enter`.
   - Verify delete confirmation flow (`D` $\to$ `Enter` deletes file; `Esc` cancels).
   - Verify empty state message.
3. **Unit Testing (`pkg/tui/components/commandbar/bar_test.go`)**:
   - Verify `Up`/`Down` history recall and draft input restoration.
   - Verify single and multi-match Tab auto-completion.
4. **Integration Testing (`pkg/tui/model_test.go`)**:
   - Verify `Ctrl+O` and `saves` command trigger `ModalSaveBrowser`.
   - Verify `LoadGameMsg` restores game state and logs status.
   - Verify theme toggling propagates to `SaveBrowser`.
5. **Cross-Compilation Verification**:
   - `GOOS=linux GOARCH=amd64 CGO_ENABLED=0 go build -o /dev/null ./cmd/sst`
   - `GOOS=darwin GOARCH=arm64 CGO_ENABLED=0 go build -o /dev/null ./cmd/sst`
   - `GOOS=windows GOARCH=amd64 CGO_ENABLED=0 go build -o /dev/null ./cmd/sst`
6. **Full Gate Verification**:
   - `go test -v -race ./...`: 100% PASS across all Go packages.
   - `ctest --preset debug`: 100% PASS across all C tests.
   - `ctest --preset debug -R '^lineendings$'`: 100% PASS.
