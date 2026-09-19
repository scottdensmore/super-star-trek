# Design Specification: Authentic Retro Audio & Sound FX Engine

**Issue:** [#246: feat: authentic retro audio and sound FX engine](https://github.com/scottdensmore/super-star-trek/issues/246)  
**Date:** 2026-09-19  
**Status:** Approved by User  
**Target Branch:** `scottdensmore/feat/retro-audio-engine`  

---

## 1. Overview & Goals

Audio effects dramatically enhance player immersion, tactical tension, and retro authenticity. Super Star Trek operates across two primary modern presentation frontends:
1. **Interactive Terminal TUI:** Terminal dashboard rendered via Bubbletea and Lipgloss.
2. **WebAssembly Browser Build:** Terminal teletype terminal hosted in modern web browsers via xterm.js and WebAssembly.

This feature introduces a comprehensive, decoupled retro sound FX engine providing procedural vintage sci-fi sound effects across both environments without requiring external CGo audio dependencies or bulky pre-recorded audio assets.

### Key Objectives
1. **Procedural Web Audio API Engine (`web/audio.js`):** Synthesize authentic vintage sci-fi sound effects in the browser using oscillators (square/sawtooth), white noise buffers, and resonant filter sweeps (phasers, photon torpedoes, explosions, Red Alert klaxon, docking chimes, warp hum).
2. **Terminal Audio & Fallback Engine (`pkg/audio`):** Pure Go in-memory PCM WAV generator coupled with non-blocking native OS audio commands (`afplay` on macOS, `paplay`/`aplay` on Linux), with an automatic fallback to standard ANSI/ASCII `\a` terminal bells and visual pulses.
3. **Decoupled Event Dispatcher:** Map `engine.Event` domain events emitted during game turns into typed sound tokens (`SoundID`).
4. **Player Controls & Ergonomics:** CLI flags (`--sound` / `--no-sound`), in-game hotkeys (`Ctrl+S`, `m`), status telemetry badge (`[SND: ON/OFF]`), and interactive toggle in the Options Modal.
5. **Zero External Dependencies:** 100% standard library Go, C17, and pure Web Audio API without third-party audio packages.

---

## 2. Architecture & Component Decomposition

```
                         ┌─────────────────────────────┐
                         │   Game Engine Actions       │
                         │   (pkg/engine/actions.go)   │
                         └──────────────┬──────────────┘
                                        │ emits []engine.Event
                                        ▼
                         ┌─────────────────────────────┐
                         │      Audio Dispatcher       │
                         │     (pkg/audio/dispatch)    │
                         └──────────────┬──────────────┘
                                        │ dispatches SoundID
                 ┌──────────────────────┴──────────────────────┐
                 ▼                                             ▼
     ┌────────────────────────┐                   ┌────────────────────────┐
     │   TUI Terminal Player  │                   │     WASM JS Bridge     │
     │  (pkg/audio/player.go) │                   │  (cmd/wasm/bridge_js)  │
     └───────────┬────────────┘                   └────────────┬───────────┘
                 │                                             │
        ┌────────┴────────┐                                    ▼
        ▼                 ▼                       ┌────────────────────────┐
┌───────────────┐ ┌───────────────┐               │   Web Audio Engine     │
│ Native OS Cmd │ │ Terminal Bell │               │     (web/audio.js)     │
│(afplay/aplay) │ │     (\a)      │               │(Oscillators & Envelopes│
└───────────────┘ └───────────────┘               └────────────────────────┘
```

---

## 3. Detailed Component Specifications

### 3.1 Core Audio Domain & Dispatcher (`pkg/audio`)

#### Sound Identifiers (`SoundID`)
```go
package audio

type SoundID string

const (
    SoundPhaser        SoundID = "phaser"          // Rapid downward frequency sweep
    SoundTorpedoLaunch SoundID = "torpedo_launch"  // Chirp + whistling ballistic launch
    SoundExplosion     SoundID = "explosion"       // White noise burst with low-pass decay
    SoundRedAlert      SoundID = "red_alert"       // Two-tone pulsing warble / siren
    SoundDock          SoundID = "dock"            // Harmonized major triad chime
    SoundWarp          SoundID = "warp"            // Low frequency accelerating drone
    SoundDamage        SoundID = "damage"          // Harsh crunchy impact noise
    SoundShields       SoundID = "shields"         // Resonant shield flare shimmer
    SoundVictory       SoundID = "victory"         // Ascending fanfare sequence
    SoundDefeat        SoundID = "defeat"          // Descending mournful tone
)
```

#### Player Interface
```go
type Player interface {
    Play(sound SoundID)
    SetMuted(muted bool)
    IsMuted() bool
}
```

#### Implementations:
1. **`TerminalBellPlayer`:**
   Writes `\a` to terminal output on alert/combat sounds (`SoundRedAlert`, `SoundDamage`, `SoundExplosion`, `SoundPhaser`, `SoundTorpedoLaunch`). Invokes an optional `VisualBellCallback` function.
2. **`NativeOSPlayer`:**
   Generates an 8 kHz 16-bit mono RIFF/WAV byte buffer in memory using mathematical formulas for sine/square/noise waveforms. Writes to a temporary file or pipes to OS player (`afplay` on Darwin, `paplay`/`aplay` on Linux) in a background goroutine. If no OS player executable is available on `PATH`, seamlessly delegates to `TerminalBellPlayer`.
3. **`NullPlayer`:**
   No-op implementation used when audio is disabled or in unit test mocks.

#### Event Mapping (`Dispatcher`)
`Dispatcher.DispatchEvents(events []engine.Event)` maps engine domain events:
- `EventCombat`:
  - Phaser attack $\rightarrow$ `SoundPhaser`
  - Torpedo attack $\rightarrow$ `SoundTorpedoLaunch`
  - Target damaged / destroyed $\rightarrow$ `SoundExplosion`
  - Enterprise hit $\rightarrow$ `SoundDamage`
- `EventShieldTransfer` / `EventShieldDischarge` $\rightarrow$ `SoundShields`
- `EventDocked` $\rightarrow$ `SoundDock`
- `EventAnomalyDiscovered` $\rightarrow$ `SoundRedAlert`
- `EventWormholeJump` $\rightarrow$ `SoundWarp`
- `EventGameOver`:
  - `Won == true` $\rightarrow$ `SoundVictory`
  - `Won == false` $\rightarrow$ `SoundDefeat`

---

### 3.2 Web Audio API Synthesizer (`web/audio.js`)

`web/audio.js` implements procedural vintage sci-fi synthesis using the browser's native `AudioContext`:

1. **User Gesture Unlock:** Initializes `AudioContext` lazily or in a suspended state; automatically invokes `audioCtx.resume()` on the first terminal click, key press, or control bar button interaction.
2. **Procedural Generators:**
   - **Phaser:** Sawtooth `OscillatorNode` sweeping exponentially from 1200 Hz down to 200 Hz over 250 ms, shaped by a resonant lowpass filter.
   - **Torpedo Launch:** Sine/triangle oscillator whistling from 600 Hz up to 1100 Hz with rapid decay.
   - **Explosion:** Procedural white noise buffer generated in memory, shaped with a falling 800 Hz $\rightarrow$ 80 Hz low-pass filter and an exponential decay gain envelope over 700 ms.
   - **Red Alert:** Two-tone pulsing siren oscillating between 550 Hz and 850 Hz with a dual-oscillator warble.
   - **Docking Chime:** Harmonious ascending triad (C5-E5-G5: 523 Hz, 659 Hz, 784 Hz) with bell-like exponential decay.
   - **Warp Drive:** Low sub-bass frequency ramp (55 Hz $\rightarrow$ 165 Hz) over 800 ms.
3. **Global Functions:**
   - `window.sstPlaySound(soundID)`: Triggered directly from WebAssembly.
   - `window.sstSetMuted(bool)`: Toggles audio synthesis and persists to `localStorage.getItem('sst_sound_muted')`.
4. **UI Button:** Toolbar toggle button in `web/index.html` displaying `🔊 Sound: ON` / `🔇 Sound: OFF`.

---

### 3.3 Terminal Frontend & TUI Integration (`pkg/tui`, `cmd/sst`)

1. **CLI Flags (`cmd/sst/main.go`):**
   - `--sound`: Enable sound effects (default: true).
   - `--no-sound`: Disable sound effects.
   - Mutual exclusion check: Reject passing both `--sound` and `--no-sound` with code 1.
2. **TUI Model & Keybindings (`pkg/tui`):**
   - Hotkeys: Pressing `Ctrl+S` or `m` (when the input line is empty) toggles sound on and off, displaying a brief status message: `*** Audio: Muted ***` or `*** Audio: Enabled ***`.
   - Visual Bell: Emits a brief visual accent pulse (150 ms) on the command bar border when critical combat events occur.
3. **Status Panel Telemetry (`pkg/tui/components/statuspanel`):**
   - Renders `[SND: ON]` (dim cyan) or `[SND: OFF]` (dim gray) in the telemetry footer.
4. **Options Modal (`pkg/tui/components/optionsmodal`):**
   - Adds `RowAudio` to allow players to toggle `Sound FX: [ENABLED] / [DISABLED]` from the in-game options menu (`F2` or Command Palette).

---

## 4. Testing & Verification Strategy

1. **Unit Tests (`pkg/audio`):**
   - Test event-to-sound mapping in `dispatcher_test.go`.
   - Test PCM WAV header generation and buffer synthesis in `wav_test.go` (validating standard RIFF/WAVE header fields: format, sample rate, bit depth, data chunk size).
   - Test `NullPlayer`, `TerminalBellPlayer`, and mock player interactions.
2. **TUI Tests (`pkg/tui`):**
   - Test hotkey `Ctrl+S` and `m` toggling audio state in `pkg/tui/update_test.go`.
   - Test Options Modal audio row navigation and toggle in `pkg/tui/components/optionsmodal/modal_test.go`.
   - Test status panel badge rendering in `pkg/tui/components/statuspanel/status_test.go`.
3. **CLI Tests (`cmd/sst/main_test.go`):**
   - Verify `--sound` and `--no-sound` parsing.
   - Verify mutual exclusion error when both `--sound` and `--no-sound` are supplied.
4. **WASM Compilation & Verification:**
   - Confirm compilation: `GOOS=js GOARCH=wasm go build -o /dev/null ./cmd/wasm`.
   - Run Node WASM tests: `GOOS=js GOARCH=wasm go test -v -exec="node $(go env GOROOT)/lib/wasm/wasm_exec_node.js" ./cmd/wasm`.
5. **Full Repository & CI Verification:**
   - `golangci-lint run ./...`
   - `go test -v -race ./...`
   - `tests/workflow.sh .` and `ctest --preset debug`
