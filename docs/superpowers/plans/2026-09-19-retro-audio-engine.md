# Authentic Retro Audio & Sound FX Engine Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement an authentic, procedural retro sound FX engine providing synthesized sci-fi audio effects for the WebAssembly browser build (via Web Audio API) and terminal Bubbletea TUI (via native OS commands, ASCII bells, and visual cues), driven by a decoupled engine event dispatcher.

**Architecture:** 
- `pkg/audio`: Core domain defining `SoundID` tokens, a procedural in-memory PCM RIFF/WAV synthesizer, a non-blocking `NativeOSPlayer` (spawning `afplay`/`paplay`/`aplay` in detached goroutines with `TerminalBellPlayer` fallback), and an event-driven `Dispatcher` mapping `engine.Event` slices to sound tokens.
- `web/audio.js`: Pure Web Audio API synthesizer (`OscillatorNode`, `GainNode`, noise `AudioBuffer`, and `BiquadFilterNode`) providing instant, self-contained browser audio with autoplay unlock.
- `cmd/wasm`: WASM bridge invoking JavaScript `window.sstPlaySound` when combat and alert events occur.
- `pkg/tui` & `cmd/sst`: `--sound`/`--no-sound` CLI flags with mutual exclusion, in-game hotkey (`Ctrl+S`/`m`), Options Modal `RowAudio` toggle, and status panel badge (`[SND: ON/OFF]`).

**Tech Stack:** Go 1.26.x, Web Audio API, JavaScript ES6, Bubbletea, Lipgloss, CMake/CTest, C17.

**Spec:** [docs/superpowers/specs/2026-09-19-retro-audio-design.md](file:///home/scottdensmore/Developer/scottdensmore/super-star-trek/docs/superpowers/specs/2026-09-19-retro-audio-design.md)

## Global Constraints

- Target branch: `scottdensmore/feat/retro-audio-engine`
- Target Go version: `1.26.x`
- 100% standard library for Go audio (zero external CGo or runtime audio libraries)
- Procedural audio synthesis (zero external audio file assets or network requests)
- WebAssembly compatibility: `GOOS=js GOARCH=wasm`
- Zero test failures across Go race detector (`go test -race ./...`), CTest, golden test, and WASM suite
- Pass `golangci-lint run ./...` with zero issues

---

### Task 1: Core Audio Domain, In-Memory WAV Synthesizer, & Event Dispatcher (`pkg/audio`)

**Files:**
- Create: `pkg/audio/sound.go`
- Create: `pkg/audio/wav.go`
- Create: `pkg/audio/player.go`
- Create: `pkg/audio/dispatcher.go`
- Test: `pkg/audio/wav_test.go`
- Test: `pkg/audio/dispatcher_test.go`
- Test: `pkg/audio/player_test.go`

**Interfaces:**
- Produces:
  - `SoundID` constants (`SoundPhaser`, `SoundTorpedoLaunch`, `SoundExplosion`, `SoundRedAlert`, `SoundDock`, `SoundWarp`, `SoundDamage`, `SoundShields`, `SoundVictory`, `SoundDefeat`)
  - `Player` interface (`Play(SoundID)`, `SetMuted(bool)`, `IsMuted() bool`)
  - `NullPlayer`, `TerminalBellPlayer`, `NativeOSPlayer`
  - `SynthesizeWav(SoundID) []byte`
  - `Dispatcher`: `NewDispatcher(Player)`, `DispatchEvents([]engine.Event)`

- [ ] **Step 1: Write tests for WAV synthesis and header generation**

Create `pkg/audio/wav_test.go`:
```go
package audio

import (
	"bytes"
	"encoding/binary"
	"testing"
)

func TestSynthesizeWav_HeaderValid(t *testing.T) {
	sounds := []SoundID{
		SoundPhaser,
		SoundTorpedoLaunch,
		SoundExplosion,
		SoundRedAlert,
		SoundDock,
		SoundWarp,
		SoundDamage,
		SoundShields,
		SoundVictory,
		SoundDefeat,
	}

	for _, s := range sounds {
		t.Run(string(s), func(t *testing.T) {
			wav := SynthesizeWav(s)
			if len(wav) < 44 {
				t.Fatalf("WAV buffer too small: %d bytes", len(wav))
			}
			if string(wav[0:4]) != "RIFF" {
				t.Errorf("expected RIFF header, got %s", string(wav[0:4]))
			}
			if string(wav[8:12]) != "WAVE" {
				t.Errorf("expected WAVE format, got %s", string(wav[8:12]))
			}
			if string(wav[12:16]) != "fmt " {
				t.Errorf("expected fmt subchunk, got %s", string(wav[12:16]))
			}
			audioFormat := binary.LittleEndian.Uint16(wav[20:22])
			if audioFormat != 1 {
				t.Errorf("expected PCM format (1), got %d", audioFormat)
			}
			numChannels := binary.LittleEndian.Uint16(wav[22:24])
			if numChannels != 1 {
				t.Errorf("expected mono (1), got %d", numChannels)
			}
			sampleRate := binary.LittleEndian.Uint32(wav[24:28])
			if sampleRate != 8000 {
				t.Errorf("expected 8000 Hz, got %d", sampleRate)
			}
			bitsPerSample := binary.LittleEndian.Uint16(wav[34:36])
			if bitsPerSample != 16 {
				t.Errorf("expected 16 bits per sample, got %d", bitsPerSample)
			}
			if string(wav[36:40]) != "data" {
				t.Errorf("expected data subchunk, got %s", string(wav[36:40]))
			}
			dataLen := binary.LittleEndian.Uint32(wav[40:44])
			if int(dataLen) != len(wav)-44 {
				t.Errorf("data length mismatch: header %d, actual %d", dataLen, len(wav)-44)
			}
		})
	}
}
```

- [ ] **Step 2: Write tests for Audio Dispatcher**

Create `pkg/audio/dispatcher_test.go`:
```go
package audio

import (
	"testing"

	"github.com/scottdensmore/super-star-trek/pkg/engine"
)

type mockPlayer struct {
	played []SoundID
	muted  bool
}

func (m *mockPlayer) Play(s SoundID) {
	if !m.muted {
		m.played = append(m.played, s)
	}
}
func (m *mockPlayer) SetMuted(muted bool) { m.muted = muted }
func (m *mockPlayer) IsMuted() bool       { return m.muted }

func TestDispatcher_EventMapping(t *testing.T) {
	p := &mockPlayer{}
	d := NewDispatcher(p)

	events := []engine.Event{
		engine.EventCombat{Weapon: engine.WeaponPhaser, TargetHit: true, TargetDestroyed: false},
		engine.EventShieldTransfer{Amount: 100},
		engine.EventDocked{Starbase: engine.Coord{Row: 4, Col: 4}},
		engine.EventGameOver{Won: true},
	}

	d.DispatchEvents(events)

	expected := []SoundID{
		SoundPhaser,
		SoundShields,
		SoundDock,
		SoundVictory,
	}

	if len(p.played) != len(expected) {
		t.Fatalf("expected %d sounds, got %d: %v", len(expected), len(p.played), p.played)
	}
	for i, exp := range expected {
		if p.played[i] != exp {
			t.Errorf("sound %d: expected %s, got %s", i, exp, p.played[i])
		}
	}
}
```

- [ ] **Step 3: Run tests to confirm failure (RED)**

Run: `go test ./pkg/audio/...`  
Expected: FAIL (packages not found / undefined types).

- [ ] **Step 4: Implement `pkg/audio/sound.go`, `pkg/audio/wav.go`, `pkg/audio/player.go`, and `pkg/audio/dispatcher.go`**

Write `pkg/audio/sound.go`:
```go
package audio

// SoundID identifies a distinct procedural retro sound effect.
type SoundID string

const (
	SoundPhaser        SoundID = "phaser"
	SoundTorpedoLaunch SoundID = "torpedo_launch"
	SoundExplosion     SoundID = "explosion"
	SoundRedAlert      SoundID = "red_alert"
	SoundDock          SoundID = "dock"
	SoundWarp          SoundID = "warp"
	SoundDamage        SoundID = "damage"
	SoundShields       SoundID = "shields"
	SoundVictory       SoundID = "victory"
	SoundDefeat        SoundID = "defeat"
)

// Player defines the playback interface for sound effects.
type Player interface {
	Play(sound SoundID)
	SetMuted(muted bool)
	IsMuted() bool
}
```

Write `pkg/audio/wav.go`:
Synthesize standard 8kHz 16-bit mono RIFF/WAVE byte buffers using pure Go math (`math.Sin`, pseudo-random noise, square/sawtooth sweeps).

Write `pkg/audio/player.go`:
Implement `NullPlayer`, `TerminalBellPlayer`, and `NativeOSPlayer` (using `exec.Command` with context timeouts for `afplay`/`paplay`/`aplay`, falling back to `TerminalBellPlayer`).

Write `pkg/audio/dispatcher.go`:
Implement `NewDispatcher(player Player)` and `DispatchEvents(events []engine.Event)`.

- [ ] **Step 5: Run tests to confirm passing (GREEN)**

Run: `go test -v -race ./pkg/audio/...`  
Expected: PASS with 100% test coverage and pristine output.

- [ ] **Step 6: Commit**

```bash
git add pkg/audio/
git commit -m "feat(audio): implement core sound domain, in-memory WAV synthesizer, and event dispatcher"
```

---

### Task 2: WebAssembly Audio Bridge & Procedural Web Audio Engine (`web/audio.js` & `cmd/wasm`)

**Files:**
- Create: `web/audio.js`
- Modify: `web/index.html:10-25`
- Modify: `web/app.js:20-60`
- Modify: `cmd/wasm/bridge_js.go:15-55`
- Modify: `cmd/wasm/bridge_other.go:1-15`
- Modify: `cmd/wasm/session.go:90-115`
- Test: `cmd/wasm/session_test.go`

**Interfaces:**
- Consumes: `pkg/audio` SoundID tokens, `engine.Event`
- Produces: `web/audio.js` Web Audio API procedural synthesizer, `window.sstPlaySound`, `window.sstSetMuted`, toolbar sound toggle button.

- [ ] **Step 1: Write test for WASM sound event triggering in session_test.go**

In `cmd/wasm/session_test.go`, add `TestSession_AudioEventDispatch`:
Verify that firing phasers, launching torpedoes, docking, or warping produces corresponding audio triggers or events without panicking.

- [ ] **Step 2: Run test to confirm failure or stub need (RED)**

Run: `go test -v ./cmd/wasm`

- [ ] **Step 3: Implement `web/audio.js`**

Implement procedural synthesizers using `AudioContext`, `OscillatorNode`, `GainNode`, and `BiquadFilterNode`:
- `playPhaser()`
- `playTorpedoLaunch()`
- `playExplosion()`
- `playRedAlert()`
- `playDock()`
- `playWarp()`
- `playDamage()`
- `playShields()`
- `playVictory()`
- `playDefeat()`
- Autoplay unlock on user interaction (`audioCtx.resume()`).
- Expose `window.sstPlaySound(soundID)` and `window.sstSetMuted(bool)`.

- [ ] **Step 4: Update `web/index.html` and `web/app.js`**

Add audio toggle button `<button id="sound-btn">🔊 Sound: ON</button>` in toolbar, wire click listener to `sstSetMuted`, and load `audio.js` before `app.js`.

- [ ] **Step 5: Update `cmd/wasm/bridge_js.go` and `cmd/wasm/session.go`**

In `bridge_js.go`, dispatch sound events to `window.sstPlaySound`.
In `bridge_other.go`, ensure clean compilation on non-JS targets.

- [ ] **Step 6: Run WASM tests and compilation checks (GREEN)**

Run:
```bash
GOOS=js GOARCH=wasm go build -o /dev/null ./cmd/wasm
GOOS=js GOARCH=wasm go test -v -exec="node $(go env GOROOT)/lib/wasm/wasm_exec_node.js" ./cmd/wasm
go test -v -race ./cmd/wasm
```
Expected: All compile and pass 100%.

- [ ] **Step 7: Commit**

```bash
git add web/ cmd/wasm/
git commit -m "feat(wasm,web): implement Web Audio API procedural synthesizer and WASM sound bridge"
```

---

### Task 3: Terminal Frontend Integration, CLI Flags, Hotkeys, & Options Modal (`pkg/tui`, `cmd/sst`)

**Files:**
- Modify: `cmd/sst/main.go:50-100`
- Modify: `cmd/sst/main_test.go:40-80`
- Modify: `pkg/tui/components/optionsmodal/modal.go:15-120`
- Modify: `pkg/tui/components/optionsmodal/modal_test.go`
- Modify: `pkg/tui/components/statuspanel/status.go:150-180`
- Modify: `pkg/tui/components/statuspanel/status_test.go`
- Modify: `pkg/tui/model.go:45-75`
- Modify: `pkg/tui/update.go:200-250`
- Modify: `pkg/tui/model_test.go`

**Interfaces:**
- Consumes: `pkg/audio.Player`, `pkg/audio.NewDispatcher`
- Produces: CLI `--sound`/`--no-sound`, hotkey `Ctrl+S` / `m` sound toggle, Options Modal `RowAudio`, status panel `[SND: ON/OFF]` badge.

- [ ] **Step 1: Write failing CLI tests in `cmd/sst/main_test.go`**

Test `--sound`, `--no-sound`, and mutual exclusion error when both are passed.

- [ ] **Step 2: Run CLI tests to verify failure (RED)**

Run: `go test -v ./cmd/sst`

- [ ] **Step 3: Implement CLI flags in `cmd/sst/main.go`**

Add `--sound` and `--no-sound` boolean flags with mutual exclusion check returning error and exit code 1. Pass sound preference to TUI.

- [ ] **Step 4: Write tests and implement `RowAudio` in `optionsmodal`**

In `pkg/tui/components/optionsmodal/modal.go`:
Add `RowAudio` to `Row` enum. Render `Sound FX: [ENABLED]` / `[DISABLED]`. Allow toggling with left/right/enter.
Verify with `modal_test.go`.

- [ ] **Step 5: Integrate `audio.Player` and hotkeys into `pkg/tui`**

In `pkg/tui/model.go`, embed `AudioPlayer audio.Player` and `AudioDispatcher *audio.Dispatcher`.
In `pkg/tui/update.go`, handle `ctrl+s` and `m` key presses to toggle mute state. Dispatch sound events when combat/actions occur.
In `pkg/tui/components/statuspanel/status.go`, render `[SND: ON]` / `[SND: OFF]`.

- [ ] **Step 6: Run full repository verification battery (GREEN)**

Run:
```bash
golangci-lint run ./...
go test -v -race ./...
PATH="$HOME/.local/share/mise/installs/go/1.26.6/bin:$PATH" tests/workflow.sh .
ctest --preset debug
tests/golden.sh build/debug/sst
```
Expected: All suites report 100% PASS with 0 errors.

- [ ] **Step 7: Commit**

```bash
git add cmd/sst/ pkg/tui/
git commit -m "feat(tui,cli): integrate audio engine, CLI flags, hotkeys, and Options Modal toggle"
```

---

### Task 4: Push Feature Branch, Open PR, and Verify GitHub Actions Execution

**Files:**
- GitHub Pull Request targeting `main`

- [ ] **Step 1: Push feature branch to origin**

```bash
git push -u origin scottdensmore/feat/retro-audio-engine
```

- [ ] **Step 2: Create Pull Request**

```bash
gh pr create --base main --head scottdensmore/feat/retro-audio-engine \
  --title "feat: authentic retro audio and sound FX engine" \
  --body "Closes #246 by introducing an authentic retro audio and sound FX engine..."
```

- [ ] **Step 3: Verify GitHub Actions CI Execution**

Run: `gh pr checks`  
Verify all 9 matrix jobs (`go-test`, `golangci-lint`, `wasm-verify`, `c-matrix`, `ci-success`) run and pass green.
