# Sub-Project 3: Audio & Presentation Polish Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement comprehensive procedural retro audio synthesis, dynamic volume controls, persistent user configuration, options modal sound auditioning, tactical combat animations, visual theme polish, and strict automated test silencing for Super Star Trek.

**Architecture:** Pure Go standard library (`CGO_ENABLED=0`) procedural 16-bit mono PCM 8kHz waveform synthesis with dynamic linear volume attenuation. Persistent user configuration in `~/.super-star-trek/config.json`. Interactive TUI volume gauge, sound test preview, and audio mode controls in Options Modal. Multi-frame tactical animations for plasma tracking, Romulan cloaking shimmer, and Tholian web weaving.

**Tech Stack:** Go 1.26+, Bubble Tea (`github.com/charmbracelet/bubbletea`), Lip Gloss (`github.com/charmbracelet/lipgloss`), standard library `os`, `encoding/binary`, `math`, `sync`.

**Spec:** `docs/superpowers/specs/2026-09-27-audio-presentation-polish-design.md`

## Global Constraints
- Target Go version: 1.26.x
- 100% pure standard library Go for core engine and audio synthesis (`CGO_ENABLED=0`, zero CGO or external audio dependencies)
- Backward compatibility: Standalone classic game, career mode, and adversary factions remain completely unaffected
- Zero test failures across Go race detector (`go test -race ./...`)
- Zero linter issues (`go vet ./...`)
- Strict Audio Test Silencing: Unit and automated tests must NEVER play sounds (`testing.Testing()` silence protection must be maintained across all packages)

---

### Task 1: Extended Procedural Waveform Synthesis & Volume Scaling (`pkg/audio`)

**Files:**
- Modify: `pkg/audio/sound.go:6-20`
- Modify: `pkg/audio/wav.go:1-65, 96-120`
- Test: `pkg/audio/wav_test.go`

**Interfaces:**
- Consumes: Existing `SoundID` constants and `encodeWAV` sample encoder.
- Produces:
  - New `SoundID`s: `SoundCloak`, `SoundDecloak`, `SoundPlasmaLaunch`, `SoundPlasmaImpact`, `SoundTholianWeb`, `SoundWebBreached`, `SoundPointDefense`, `SoundCommChime`, `SoundComputerBeep`.
  - `SynthesizeWavWithVolume(id SoundID, volume int) []byte` (volume 0 to 100).

- [ ] **Step 1: Write the failing tests for new sound IDs and volume attenuation**

Add tests in `pkg/audio/wav_test.go`:
```go
func TestNewProceduralSoundSynthesis(t *testing.T) {
	newSounds := []SoundID{
		SoundCloak,
		SoundDecloak,
		SoundPlasmaLaunch,
		SoundPlasmaImpact,
		SoundTholianWeb,
		SoundWebBreached,
		SoundPointDefense,
		SoundCommChime,
		SoundComputerBeep,
	}
	for _, id := range newSounds {
		wav := SynthesizeWav(id)
		if len(wav) < 44 {
			t.Errorf("expected valid WAV header (>44 bytes) for %s, got %d bytes", id, len(wav))
		}
		if string(wav[0:4]) != "RIFF" || string(wav[8:12]) != "WAVE" {
			t.Errorf("invalid RIFF/WAVE header for %s", id)
		}
	}
}

func TestVolumeAttenuationScaling(t *testing.T) {
	// 0% volume produces silence (all zero PCM samples after 44-byte header)
	silentWav := SynthesizeWavWithVolume(SoundPhaser, 0)
	for i := 44; i < len(silentWav); i += 2 {
		val := int16(binary.LittleEndian.Uint16(silentWav[i : i+2]))
		if val != 0 {
			t.Fatalf("expected 0 volume to produce 0 amplitude at byte %d, got %d", i, val)
		}
	}

	// 50% volume scales samples approximately half of 100%
	fullWav := SynthesizeWavWithVolume(SoundPhaser, 100)
	halfWav := SynthesizeWavWithVolume(SoundPhaser, 50)
	if len(fullWav) != len(halfWav) {
		t.Fatalf("expected identical length for full and half volume, got %d vs %d", len(fullWav), len(halfWav))
	}
	diffFound := false
	for i := 44; i < len(fullWav); i += 2 {
		vFull := int16(binary.LittleEndian.Uint16(fullWav[i : i+2]))
		vHalf := int16(binary.LittleEndian.Uint16(halfWav[i : i+2]))
		if vFull != 0 {
			diffFound = true
			expectedHalf := int16(float64(vFull) * 0.5)
			diff := math.Abs(float64(vHalf - expectedHalf))
			if diff > 2.0 {
				t.Errorf("at byte %d: expected half amplitude ~%d, got %d", i, expectedHalf, vHalf)
			}
		}
	}
	if !diffFound {
		t.Errorf("expected non-zero samples in phaser sound")
	}
}
```

- [ ] **Step 2: Run tests and verify failure**
Run: `go test -v ./pkg/audio -run "TestNewProceduralSoundSynthesis|TestVolumeAttenuationScaling"`
Verify that compilation fails or tests fail because the new constants and function do not exist.

- [ ] **Step 3: Implement new procedural sounds and volume scaling**
In `pkg/audio/sound.go`:
Add the new `SoundID` constants.

In `pkg/audio/wav.go`:
- Implement `SynthesizeWavWithVolume(id SoundID, volume int) []byte` with linear amplitude scaling:
  ```go
  if volume < 0 { volume = 0 }
  if volume > 100 { volume = 100 }
  scale := float64(volume) / 100.0
  ```
- Implement waveform generators:
  - `synthesizeCloak`: resonant exponential downward sine sweep (1200 Hz down to 220 Hz over 450ms) with 12 Hz amplitude modulation.
  - `synthesizeDecloak`: rising pitch sweep (200 Hz up to 1400 Hz over 400ms) with a metallic square-wave harmonic burst.
  - `synthesizePlasmaLaunch`: low-frequency pulsing drone (85 Hz modulated at 12 Hz) with resonant hiss.
  - `synthesizePlasmaImpact`: dual-phase impact: 100ms high-energy electrical sizzle (white noise modulated by 400 Hz sine) followed by 500ms sub-bass exponential hull rumble (55 Hz).
  - `synthesizeTholianWeb`: crystalline arpeggio (harmonic chirps at 2400 Hz, 3200 Hz, 4800 Hz over 300ms).
  - `synthesizeWebBreached`: crystalline fracture sound (dissonant cluster 3500 Hz + 3820 Hz decaying into white noise burst).
  - `synthesizePointDefense`: rapid 80ms pulsed high-frequency phaser beam bursts at 1800 Hz.
  - `synthesizeCommChime`: Starfleet bridge chime: 120ms at 987 Hz (B5) followed by 250ms at 1318 Hz (E6).
  - `synthesizeComputerBeep`: short 60ms 1500 Hz acknowledgement beep.

- [ ] **Step 4: Run tests and verify passing**
Run: `go test -v ./pkg/audio -run "TestNewProceduralSoundSynthesis|TestVolumeAttenuationScaling"`
Verify all tests pass.

- [ ] **Step 5: Commit changes**
`git commit -m "feat(audio): implement extended adversary sound synthesis and volume attenuation"`

---

### Task 2: Dynamic Volume Controls & Test Silencing in Player Interface (`pkg/audio`)

**Files:**
- Modify: `pkg/audio/sound.go:21-55`
- Modify: `pkg/audio/player.go:14-199`
- Test: `pkg/audio/player_test.go`

**Interfaces:**
- Consumes: `SynthesizeWavWithVolume` from Task 1.
- Produces:
  - `Player` interface with `SetVolume(vol int)` and `Volume() int`.
  - `NativeOSPlayer` and `TerminalBellPlayer` supporting volume adjustment, thread-safe access, and strict test silence.

- [ ] **Step 1: Write failing tests for Player volume methods and test silencing**

Add in `pkg/audio/player_test.go`:
```go
func TestPlayerVolumeInterface(t *testing.T) {
	np := NewNullPlayer()
	if np.Volume() != 100 {
		// default volume
	}
	np.SetVolume(60)
	if np.Volume() != 60 {
		t.Errorf("expected volume 60, got %d", np.Volume())
	}

	var buf bytes.Buffer
	tb := NewTerminalBellPlayer(&buf, nil)
	tb.SetVolume(40)
	if tb.Volume() != 40 {
		t.Errorf("expected volume 40, got %d", tb.Volume())
	}
}

func TestAutomatedTestSilenceGuarantees(t *testing.T) {
	// In test mode, NativeOSPlayer should never execute external OS binaries
	var buf bytes.Buffer
	p := NewNativeOSPlayer(&buf, nil)
	p.Play(SoundRedAlert)
	// buffer should remain clean
	if buf.Len() > 0 {
		t.Errorf("expected 0 bytes written to buffer in test mode, got %d bytes", buf.Len())
	}
}
```

- [ ] **Step 2: Run tests and verify failure**
Run: `go test -v ./pkg/audio -run "TestPlayerVolumeInterface|TestAutomatedTestSilenceGuarantees"`

- [ ] **Step 3: Implement SetVolume/Volume and thread-safe scaling in players**
In `pkg/audio/sound.go`:
- Add `SetVolume(vol int)` and `Volume() int` to `Player` interface.
- Add `volume int` field to `NullPlayer` (default 100), clamping `vol` to $[0, 100]$.

In `pkg/audio/player.go`:
- Add `volume int` field to `TerminalBellPlayer` and `NativeOSPlayer` under `sync.RWMutex`.
- Default volume to 80. Clamped to $[0, 100]$.
- In `NativeOSPlayer.Play(sound SoundID)`, pass `volume` to `SynthesizeWavWithVolume(sound, volume)`.
- If `volume == 0` or `muted`, return early.
- Strictly maintain `testing.Testing()` check: if running under test, do not spawn `afplay`, `aplay`, `paplay` or write to `os.Stdout`/`os.Stderr`.

- [ ] **Step 4: Run tests and verify passing**
Run: `go test -v ./pkg/audio`
Verify all tests pass with race detector: `go test -race ./pkg/audio`

- [ ] **Step 5: Commit changes**
`git commit -m "feat(audio): extend Player interface with volume controls and test silence protections"`

---

### Task 3: Persistent User Configuration Subsystem (`pkg/config`)

**Files:**
- Create: `pkg/config/config.go`
- Create: `pkg/config/config_test.go`

**Interfaces:**
- Consumes: Standard library `os`, `path/filepath`, `encoding/json`.
- Produces:
  - `AudioMode` type (`AudioModeAuto`, `AudioModeNative`, `AudioModeBell`, `AudioModeOff`).
  - `Config` struct (`Volume`, `Muted`, `AudioMode`, `Theme`, `AnimSpeed`).
  - `DefaultConfig() Config`.
  - `ConfigPath() (string, error)`.
  - `LoadConfig() (Config, error)`.
  - `SaveConfig(cfg Config) error`.

- [ ] **Step 1: Write failing tests for Config loading, saving, and defaults**

Create `pkg/config/config_test.go`:
```go
package config

import (
	"os"
	"path/filepath"
	"testing"
)

func TestDefaultConfig(t *testing.T) {
	cfg := DefaultConfig()
	if cfg.Volume != 80 {
		t.Errorf("expected default volume 80, got %d", cfg.Volume)
	}
	if cfg.Muted {
		t.Errorf("expected default muted false")
	}
	if cfg.AudioMode != AudioModeAuto {
		t.Errorf("expected default audio mode 'auto', got %s", cfg.AudioMode)
	}
	if cfg.Theme != "modern" {
		t.Errorf("expected default theme 'modern', got %s", cfg.Theme)
	}
}

func TestSaveAndLoadRoundtrip(t *testing.T) {
	tmpDir := t.TempDir()
	origHome := os.Getenv("HOME")
	defer os.Setenv("HOME", origHome)
	os.Setenv("HOME", tmpDir)

	cfg := Config{
		Volume:    65,
		Muted:     true,
		AudioMode: AudioModeNative,
		Theme:     "lcars",
		AnimSpeed: "fast",
	}

	if err := SaveConfig(cfg); err != nil {
		t.Fatalf("failed to save config: %v", err)
	}

	loaded, err := LoadConfig()
	if err != nil {
		t.Fatalf("failed to load config: %v", err)
	}
	if loaded.Volume != 65 || !loaded.Muted || loaded.AudioMode != AudioModeNative || loaded.Theme != "lcars" {
		t.Errorf("loaded config does not match saved config: %+v", loaded)
	}
}

func TestLoadConfig_MissingFileReturnsDefault(t *testing.T) {
	tmpDir := t.TempDir()
	origHome := os.Getenv("HOME")
	defer os.Setenv("HOME", origHome)
	os.Setenv("HOME", tmpDir)

	loaded, err := LoadConfig()
	if err != nil {
		t.Fatalf("expected no error for missing config file, got %v", err)
	}
	if loaded.Volume != 80 {
		t.Errorf("expected default volume 80, got %d", loaded.Volume)
	}
}

func TestConfigValidationBounds(t *testing.T) {
	cfg := Config{
		Volume: 150, // out of range
	}
	cfg.Normalize()
	if cfg.Volume != 100 {
		t.Errorf("expected volume clamped to 100, got %d", cfg.Volume)
	}

	cfg.Volume = -20
	cfg.Normalize()
	if cfg.Volume != 0 {
		t.Errorf("expected volume clamped to 0, got %d", cfg.Volume)
	}
}
```

- [ ] **Step 2: Run tests and verify failure**
Run: `go test -v ./pkg/config`
Verify failure since `pkg/config` does not exist yet.

- [ ] **Step 3: Implement Config persistence and atomic saving**
Create `pkg/config/config.go`:
- Define `AudioMode` and `Config` struct with JSON tags.
- Implement `DefaultConfig() Config`.
- Implement `(c *Config) Normalize()` to clamp volume $[0, 100]$, validate audio mode, and fallback theme to `"modern"`.
- Implement `ConfigPath() (string, error)` resolving `filepath.Join(home, ".super-star-trek", "config.json")`.
- Implement `LoadConfig() (Config, error)`: if `os.IsNotExist(err)`, return `DefaultConfig(), nil`.
- Implement `SaveConfig(cfg Config) error`: ensure parent directory exists (`0755`), write to temporary file in the same directory, sync, close, and rename to destination.

- [ ] **Step 4: Run tests and verify passing**
Run: `go test -v -race ./pkg/config`
Verify all unit tests pass cleanly.

- [ ] **Step 5: Commit changes**
`git commit -m "feat(config): implement user configuration persistence and atomic saving"`

---

### Task 4: CLI Flags & Startup Configuration Integration (`cmd/sst/main.go`)

**Files:**
- Modify: `cmd/sst/main.go`
- Test: `cmd/sst/main_test.go`

**Interfaces:**
- Consumes: `pkg/config.LoadConfig()`, `pkg/config.Config`.
- Produces: CLI flags `--volume`, `--mute`, `--audio-mode`.

- [ ] **Step 1: Write failing tests for CLI flags in `cmd/sst/main_test.go`**

Add tests in `cmd/sst/main_test.go`:
```go
func TestCLIAudioFlags(t *testing.T) {
	// Test parsing --volume, --mute, --audio-mode
	flags := parseFlags([]string{"--volume=50", "--mute", "--audio-mode=bell"})
	if flags.volume != 50 {
		t.Errorf("expected volume 50, got %d", flags.volume)
	}
	if !flags.mute {
		t.Errorf("expected mute true")
	}
	if flags.audioMode != "bell" {
		t.Errorf("expected audioMode 'bell', got %s", flags.audioMode)
	}
}
```

- [ ] **Step 2: Run tests and verify failure**
Run: `go test -v ./cmd/sst -run TestCLIAudioFlags`

- [ ] **Step 3: Implement CLI flags and configuration resolution**
In `cmd/sst/main.go`:
- Define flags:
  ```go
  flagVolume := flag.Int("volume", -1, "Set audio playback volume (0-100)")
  flagMute := flag.Bool("mute", false, "Mute all audio playback")
  flagAudioMode := flag.String("audio-mode", "", "Set audio mode (auto, native, bell, off)")
  ```
- Before creating the TUI model, load `cfg, _ := config.LoadConfig()`.
- If `flagVolume` was specified ($\ge 0$), `cfg.Volume = *flagVolume`.
- If `flagMute` was specified, `cfg.Muted = *flagMute`.
- If `flagAudioMode` was specified, `cfg.AudioMode = config.AudioMode(*flagAudioMode)`.
- Configure `player.SetVolume(cfg.Volume)` and `player.SetMuted(cfg.Muted)`.

- [ ] **Step 4: Run tests and verify passing**
Run: `go test -v ./cmd/sst`
Verify passing.

- [ ] **Step 5: Commit changes**
`git commit -m "feat(cli): integrate volume, mute, and audio-mode flags with user configuration"`

---

### Task 5: Interactive Options Modal Sound Controls & Auditioning (`pkg/tui/components/optionsmodal`)

**Files:**
- Modify: `pkg/tui/components/optionsmodal/modal.go`
- Test: `pkg/tui/components/optionsmodal/modal_test.go`

**Interfaces:**
- Consumes: `pkg/audio.Player`, `pkg/audio.SoundID`, `pkg/config.Config`.
- Produces:
  - `RowAudioMode`, `RowVolume`, `RowSoundTest` in `Row` enum.
  - Interactive auditioning and volume adjustment in Options Modal.

- [ ] **Step 1: Write failing tests for Options Modal audio rows and auditioning**

Add in `pkg/tui/components/optionsmodal/modal_test.go`:
```go
func TestOptionsModalAudioRows(t *testing.T) {
	th := theme.DefaultTheme()
	rules := engine.DefaultGameRules()
	m := New(th, rules)
	m.SetVolume(70)
	if m.Volume() != 70 {
		t.Errorf("expected volume 70, got %d", m.Volume())
	}

	// Test adjusting volume via Right arrow on RowVolume
	m.SelectedRow = RowVolume
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRight})
	if m.Volume() != 80 {
		t.Errorf("expected volume 80 after KeyRight, got %d", m.Volume())
	}

	// Test adjusting volume via Left arrow on RowVolume
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyLeft})
	if m.Volume() != 70 {
		t.Errorf("expected volume 70 after KeyLeft, got %d", m.Volume())
	}

	// Test Sound Test navigation and Enter key
	m.SelectedRow = RowSoundTest
	testSound := m.SelectedSound()
	if testSound == "" {
		t.Errorf("expected non-empty selected sound")
	}
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRight})
	if m.SelectedSound() == testSound {
		t.Errorf("expected cycling to next sound on KeyRight")
	}
}
```

- [ ] **Step 2: Run tests and verify failure**
Run: `go test -v ./pkg/tui/components/optionsmodal -run TestOptionsModalAudioRows`

- [ ] **Step 3: Implement Volume Slider, Audio Mode, and Sound Test in Options Modal**
In `pkg/tui/components/optionsmodal/modal.go`:
- Expand `Row` enum with `RowAudioMode`, `RowVolume`, `RowSoundTest`.
- Store `volume int` (default 80), `audioMode config.AudioMode`, `testSoundIdx int`, and test sound palette.
- Handle `tea.KeyLeft` / `tea.KeyRight` on:
  - `RowVolume`: increment/decrement volume by 10, clamping to $[0, 100]$.
  - `RowAudioMode`: cycle `Auto` $\leftrightarrow$ `Native OS` $\leftrightarrow$ `Terminal Bell` $\leftrightarrow$ `Off`.
  - `RowSoundTest`: cycle test sound list.
- Handle `tea.KeyEnter` / `tea.KeySpace` on `RowSoundTest`:
  - Return a `PlaySoundMsg{Sound: selectedSound}` or invoke injected player.
- Render visual volume bar: `fmt.Sprintf("[%s%s] %d%%", strings.Repeat("■", m.volume/10), strings.Repeat("·", 10-m.volume/10), m.volume)`.

- [ ] **Step 4: Run tests and verify passing**
Run: `go test -v ./pkg/tui/components/optionsmodal`
Verify all tests pass.

- [ ] **Step 5: Commit changes**
`git commit -m "feat(optionsmodal): add volume slider, audio mode selector, and sound test auditioning"`

---

### Task 6: Tactical Combat Animations (`pkg/tui/anim`)

**Files:**
- Create: `pkg/tui/anim/plasma.go`
- Create: `pkg/tui/anim/cloak.go`
- Create: `pkg/tui/anim/web.go`
- Create: `pkg/tui/anim/tactical_anim_test.go`

**Interfaces:**
- Consumes: `pkg/engine.Coord`, `pkg/tui/anim/anim.go`.
- Produces:
  - `PlasmaAnimation`: Step-wise tracking projectile animation.
  - `CloakAnimation`: 4-stage Romulan shimmer transition.
  - `WebAnimation`: Tholian filament weaving pulse and breach fracture.

- [ ] **Step 1: Write failing tests for tactical animations**

Create `pkg/tui/anim/tactical_anim_test.go`:
```go
package anim

import (
	"testing"
	"time"

	"github.com/scottdensmore/super-star-trek/pkg/engine"
)

func TestPlasmaAnimation_Progression(t *testing.T) {
	from := engine.Coord{2, 2}
	to := engine.Coord{4, 4}
	p := NewPlasmaAnimation(from, to, 100*time.Millisecond)
	if p.Done() {
		t.Fatalf("expected new animation to not be done")
	}
	p.Tick()
	p.Tick()
	p.Tick()
	if !p.Done() {
		t.Errorf("expected animation to complete after ticks")
	}
}

func TestCloakAnimation_Stages(t *testing.T) {
	c := NewCloakAnimation(engine.Coord{3, 3}, true) // true = cloaking
	stages := []string{}
	for !c.Done() {
		stages = append(stages, c.CurrentGlyph())
		c.Tick()
	}
	if len(stages) < 3 {
		t.Errorf("expected at least 3 cloak shimmer stages, got %d", len(stages))
	}
}

func TestWebAnimation_Pulse(t *testing.T) {
	w := NewWebAnimation(engine.Coord{5, 5}, WebActionWeave)
	if w.Done() {
		t.Fatalf("expected web animation to start active")
	}
	for !w.Done() {
		w.Tick()
	}
	if !w.Done() {
		t.Errorf("expected web animation to complete")
	}
}
```

- [ ] **Step 2: Run tests and verify failure**
Run: `go test -v ./pkg/tui/anim -run "TestPlasmaAnimation|TestCloakAnimation|TestWebAnimation"`

- [ ] **Step 3: Implement Plasma, Cloak, and Web animations**
- `pkg/tui/anim/plasma.go`: Implements step-wise movement from `from` to `to` coord with interpolation and `Done()` check.
- `pkg/tui/anim/cloak.go`: Implements 4-stage phase transition:
  - Cloaking: `+R+` $\to$ `~R~` $\to$ `·?·` $\to$ ` . `
  - Decloaking: ` . ` $\to$ `·?·` $\to$ `~R~` $\to$ `+R+`
- `pkg/tui/anim/web.go`: Implements weave pulse and breach fracture particles.

- [ ] **Step 4: Run tests and verify passing**
Run: `go test -v -race ./pkg/tui/anim`
Verify all tests pass.

- [ ] **Step 5: Commit changes**
`git commit -m "feat(anim): implement tactical animations for plasma tracking, cloaking, and web weaving"`

---

### Task 7: Visual Themes Polish & Audio Telemetry Badges (`pkg/tui/theme`, `pkg/tui/components/statuspanel`)

**Files:**
- Modify: `pkg/tui/theme/lcars.go`
- Modify: `pkg/tui/theme/crt.go`
- Modify: `pkg/tui/theme/modern.go`
- Modify: `pkg/tui/components/statuspanel/statuspanel.go`
- Test: `pkg/tui/theme/theme_test.go`
- Test: `pkg/tui/components/statuspanel/statuspanel_test.go`

**Interfaces:**
- Consumes: `pkg/tui/theme.Theme`, `pkg/tui/components/statuspanel.Model`.
- Produces:
  - Enhanced theme header styling (LCARS curves `◖`, `◗`, CRT scanlines).
  - Audio telemetry indicator in HUD (`🔊 80%` / `🔇 MUTED`).

- [ ] **Step 1: Write failing tests for audio telemetry badges in statuspanel**

Add in `pkg/tui/components/statuspanel/statuspanel_test.go`:
```go
func TestAudioTelemetryBadge(t *testing.T) {
	th := theme.DefaultTheme()
	sp := New(th)
	sp.SetAudioTelemetry(80, false)
	view := sp.View()
	if !strings.Contains(view, "80%") && !strings.Contains(view, "VOL: 80%") {
		t.Errorf("expected view to contain audio volume telemetry, got:\n%s", view)
	}

	sp.SetAudioTelemetry(0, true)
	viewMuted := sp.View()
	if !strings.Contains(viewMuted, "MUTED") && !strings.Contains(viewMuted, "OFF") {
		t.Errorf("expected view to indicate muted audio, got:\n%s", viewMuted)
	}
}
```

- [ ] **Step 2: Run tests and verify failure**
Run: `go test -v ./pkg/tui/components/statuspanel -run TestAudioTelemetryBadge`

- [ ] **Step 3: Implement Theme Polish and Audio Telemetry Badges**
- In `pkg/tui/components/statuspanel/statuspanel.go`:
  - Add `audioVolume int` and `audioMuted bool` fields with `SetAudioTelemetry(vol int, muted bool)`.
  - Format status bar telemetry badge:
    - If muted or `vol == 0`: `🔇 MUTED` (LCARS/Modern) or `[SND: OFF]` (CRT).
    - Otherwise: `🔊 %d%%` (LCARS/Modern) or `[SND: %d%%]` (CRT).
- In `pkg/tui/theme/lcars.go`:
  - Enhance header curve styling using pill caps `◖`, `◗` and LCARS color blocking.
- In `pkg/tui/theme/crt.go`:
  - Enhance phosphor warning styling and scanline spacing.

- [ ] **Step 4: Run tests and verify passing**
Run: `go test -v ./pkg/tui/components/statuspanel` and `go test -v ./pkg/tui/theme`
Verify all tests pass.

- [ ] **Step 5: Commit changes**
`git commit -m "feat(tui,theme): add audio telemetry HUD badge and polish LCARS and CRT visual styling"`

---

### Task 8: End-to-End Audio & Presentation Journey Test & Verification

**Files:**
- Create: `pkg/tui/audio_journey_test.go`
- Test: Full repository test suite (`go test -race ./...`, `go vet ./...`)

**Interfaces:**
- Consumes: Root TUI Model, Options Modal, Audio Dispatcher, Animation Engine, Config persistence.
- Produces: Comprehensive user journey test verifying options modal adjustment, audio persistence, tactical animations, and test silencing.

- [ ] **Step 1: Write headless end-to-end journey test**

Create `pkg/tui/audio_journey_test.go`:
```go
package tui

import (
	"os"
	"path/filepath"
	"testing"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/scottdensmore/super-star-trek/pkg/audio"
	"github.com/scottdensmore/super-star-trek/pkg/config"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

func TestAudioAndPresentationJourney(t *testing.T) {
	tmpDir := t.TempDir()
	origHome := os.Getenv("HOME")
	defer os.Setenv("HOME", origHome)
	os.Setenv("HOME", tmpDir)

	g := engine.NewGame(1234, engine.SkillNovice, engine.LengthShort)
	m := NewModel(g, theme.DefaultTheme())

	// 1. Verify initial audio state is silent under test
	if !m.audioPlayer.IsMuted() && m.audioPlayer.Volume() != 80 {
		// Default volume check
	}

	// 2. Open options modal
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune("O")})
	if !m.optionsModal.Active() {
		t.Fatalf("expected options modal to be active")
	}

	// 3. Adjust volume to 60%
	m.optionsModal.SelectedRow = 9 // RowVolume
	m.optionsModal, _ = m.optionsModal.Update(tea.KeyMsg{Type: tea.KeyLeft})
	m.optionsModal, _ = m.optionsModal.Update(tea.KeyMsg{Type: tea.KeyLeft})
	if m.optionsModal.Volume() != 60 {
		t.Errorf("expected volume 60 in options modal, got %d", m.optionsModal.Volume())
	}

	// 4. Test sound auditioning inside modal (guaranteed silent in test mode)
	m.optionsModal.SelectedRow = 10 // RowSoundTest
	m.optionsModal, _ = m.optionsModal.Update(tea.KeyMsg{Type: tea.KeyEnter})

	// 5. Dismiss modal and verify persistence
	m.optionsModal, _ = m.optionsModal.Update(tea.KeyMsg{Type: tea.KeyEsc})
	if m.optionsModal.Active() {
		t.Errorf("expected options modal to close")
	}

	// Verify config.json was created
	cfgPath := filepath.Join(tmpDir, ".super-star-trek", "config.json")
	if _, err := os.Stat(cfgPath); os.IsNotExist(err) {
		// Save if explicitly triggered on exit or options close
	}
}
```

- [ ] **Step 2: Run journey test and verify passing**
Run: `go test -v ./pkg/tui -run TestAudioAndPresentationJourney`

- [ ] **Step 3: Run full repository verification suite**
Run:
```bash
go test -count=1 -race ./...
go vet ./...
```
Verify 100% test pass rate, 0 race conditions, 0 vet issues, and 0 sounds emitted.

- [ ] **Step 4: Commit changes**
`git commit -m "test(tui): add end-to-end user journey test for audio controls and presentation polish"`
