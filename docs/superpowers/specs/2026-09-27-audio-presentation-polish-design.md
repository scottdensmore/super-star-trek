# Sub-Project 3: Audio & Presentation Polish — Design Specification

**Date:** 2026-09-27  
**Status:** Approved  
**Scope:** Sub-Project 3 of the Super Star Trek 3-Part Roadmap (Starfleet Career -> Expanded Adversaries -> Audio & Presentation Polish)

---

## 1. Executive Summary & Goals

This specification defines the architecture, components, and integration requirements for **Sub-Project 3: Audio & Presentation Polish** in Super Star Trek.

### Primary Objectives
1. **Procedural Retro Audio Engine (`pkg/audio`):**
   - Pure standard library Go (`CGO_ENABLED=0`) procedural sound generation (16-bit 8kHz/16kHz mono PCM).
   - Dedicated sound effects for expanded adversary factions and tactical events: Romulan cloaking/decloaking, tracking plasma torpedoes, Tholian web weaving/fracturing, point defense phaser bursts, and bridge communications.
   - Dynamic linear volume attenuation with amplitude clamping.
2. **Persistent User Configuration (`pkg/config`):**
   - User configuration saved to `~/.super-star-trek/config.json` managing master volume, audio mode, active visual theme, and animation speed.
   - CLI flags in `cmd/sst/main.go` for overriding volume and mute settings at startup.
3. **Interactive TUI Controls & Options Modal (`pkg/tui/components/optionsmodal`):**
   - Dynamic volume slider with visual gauge and 10% step increments.
   - Interactive sound test row enabling players to audition retro sound effects on demand.
   - Audio driver selection (`Auto`, `Native OS`, `Terminal Bell`, `Off`).
4. **Tactical Combat Animations & Theme Polish (`pkg/tui/anim`, `pkg/tui/theme`):**
   - In-flight plasma torpedo projectile tracking animations on the sector grid.
   - Romulan 4-stage phase shimmer cloaking/decloaking animations.
   - Tholian web weaving lattice pulses and filament breach shattering animations.
   - LCARS curved styling polish with audio telemetry badge (`🔊 80%` / `🔇 MUTED`).
   - CRT phosphor scanline enhancements and warning banner glow.
5. **Strict Automated Test Audio Silencing:**
   - Universal `testing.Testing()` protection guaranteeing that unit, package, integration, and headless journey tests run with 100% audio silence (zero OS audio processes spawned, zero terminal bell escapes).

---

## 2. Architecture & Data Structures

```
┌──────────────────────────────────────────────────────────────┐
│                        TUI Layer                             │
│  ┌───────────────────────┐        ┌───────────────────────┐  │
│  │     Options Modal     │        │     Status Panel      │  │
│  │ (Volume, Test, Mode)  │        │ (Audio Telemetry HUD) │  │
│  └───────────┬───────────┘        └───────────────────────┘  │
│              │                                               │
│  ┌───────────▼───────────┐        ┌───────────────────────┐  │
│  │      TUI Model        │◄───────┤     Tactical Anim     │  │
│  │  (Event Dispatching)  │        │ (Plasma, Cloak, Web)  │  │
│  └───────────┬───────────┘        └───────────────────────┘  │
└──────────────┼───────────────────────────────────────────────┘
               │
┌──────────────▼───────────────────────────────────────────────┐
│                      Audio Subsystem                         │
│  ┌────────────────────────────────────────────────────────┐  │
│  │                     Player Interface                   │  │
│  │     Play(), SetVolume(), Volume(), SetMuted()          │  │
│  └───────────────────────────┬────────────────────────────┘  │
│                              │                               │
│              ┌───────────────┴───────────────┐               │
│              ▼                               ▼               │
│   ┌─────────────────────┐         ┌─────────────────────┐    │
│   │   NativeOSPlayer    │         │ TerminalBellPlayer  │    │
│   │  (afplay/paplay)    │         │       (\a)          │    │
│   └──────────┬──────────┘         └─────────────────────┘    │
│              ▼                                               │
│   ┌─────────────────────┐                                    │
│   │    SynthesizeWav    │ (Procedural 16-bit PCM Mono)       │
│   │   Volume Scaler     │                                    │
│   └─────────────────────┘                                    │
└──────────────────────────────────────────────────────────────┘
               │
┌──────────────▼───────────────────────────────────────────────┐
│                    Persistence Layer                         │
│  ┌────────────────────────────────────────────────────────┐  │
│  │            pkg/config (config.json)                    │  │
│  │      Volume, AudioMode, Theme, AnimSpeed               │  │
│  └────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
```

### 2.1 Extended Sound Palette (`pkg/audio/sound.go`)
```go
type SoundID string

const (
    // Existing Sounds
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

    // Sub-Project 3 Additions
    SoundCloak         SoundID = "cloak"
    SoundDecloak       SoundID = "decloak"
    SoundPlasmaLaunch  SoundID = "plasma_launch"
    SoundPlasmaImpact  SoundID = "plasma_impact"
    SoundTholianWeb    SoundID = "tholian_web"
    SoundWebBreached   SoundID = "web_breached"
    SoundPointDefense  SoundID = "point_defense"
    SoundCommChime     SoundID = "comm_chime"
    SoundComputerBeep  SoundID = "computer_beep"
)
```

### 2.2 Player Interface (`pkg/audio/sound.go`)
```go
type Player interface {
    Play(sound SoundID)
    SetMuted(muted bool)
    IsMuted() bool
    SetVolume(vol int) // 0 to 100
    Volume() int
}
```

### 2.3 Volume Scaling & Sample Synthesis (`pkg/audio/wav.go`)
Linear amplitude attenuation is applied to synthesized samples prior to WAV encoding:
$$\text{Sample}_{\text{attenuated}} = \text{int16}\left(\text{float64}(\text{Sample}_{\text{orig}}) \times \frac{\text{volume}}{100.0}\right)$$
Clamped strictly to $[-32768, 32767]$.
Caching is partitioned by `(SoundID, VolumeBucket)` to prevent cache thrashing while retaining rapid playback.

### 2.4 User Configuration Schema (`pkg/config/config.go`)
```go
type AudioMode string

const (
    AudioModeAuto   AudioMode = "auto"
    AudioModeNative AudioMode = "native"
    AudioModeBell   AudioMode = "bell"
    AudioModeOff    AudioMode = "off"
)

type Config struct {
    Volume     int       `json:"volume"`      // 0 to 100, default 80
    Muted      bool      `json:"muted"`       // default false
    AudioMode  AudioMode `json:"audio_mode"`  // default "auto"
    Theme      string    `json:"theme"`       // "modern", "lcars", "crt" (default "modern")
    AnimSpeed  string    `json:"anim_speed"`  // "fast", "normal", "slow" (default "normal")
}

func DefaultConfig() Config {
    return Config{
        Volume:    80,
        Muted:     false,
        AudioMode: AudioModeAuto,
        Theme:     "modern",
        AnimSpeed: "normal",
    }
}
```

---

## 3. Component Details & Interactions

### 3.1 Procedural Waveform Synthesis Algorithms
- **`SoundCloak`:** Resonant exponential sine sweep decaying from 1200 Hz down to 220 Hz over 450ms, with 12 Hz amplitude modulation simulating phased cloaking fields.
- **`SoundDecloak`:** Ascending pitch sweep (200 Hz up to 1400 Hz over 400ms) with a metallic square-wave harmonic burst upon phase lock.
- **`SoundPlasmaLaunch`:** Low-frequency pulsing drone (85 Hz modulated at 12 Hz) with a heavy resonant hiss simulating superheated plasma ejection.
- **`SoundPlasmaImpact`:** Dual-phase impact: 100ms high-energy electrical sizzle (white noise modulated by 400 Hz sine) followed by 500ms sub-bass exponential hull reverberation (55 Hz).
- **`SoundTholianWeb`:** High-frequency crystalline arpeggio (harmonic chirps at 2400 Hz, 3200 Hz, 4800 Hz over 300ms).
- **`SoundWebBreached`:** Crystalline fracture sound (dissonant cluster 3500 Hz + 3820 Hz decaying into white noise burst).
- **`SoundPointDefense`:** High-cadence micro-burst phaser (80ms rapid pulses at 1800 Hz).
- **`SoundCommChime`:** Classic Starfleet bridge chime: 120ms at 987 Hz (B5) followed by 250ms at 1318 Hz (E6).

### 3.2 Tactical Animations (`pkg/tui/anim`)
- **`PlasmaAnimation`:** Step-wise projectile tracking animation across sector coordinates (`*P*`).
- **`CloakAnimation`:** 4-stage shimmer transition (`+R+` $\to$ `~R~` $\to$ `·?·` $\to$ ` . `).
- **`WebAnimation`:** Lattice pulse when filaments are laid (`:::`) and particle fracture when breached.

### 3.3 Options Modal (`pkg/tui/components/optionsmodal`)
- Navigable rows: `RowAudioMode`, `RowVolume`, `RowSoundTest`.
- Volume slider visual display: `[■■■■■■■■··] 80%`.
- Sound test preview: Pressing Enter or Space triggers `player.Play(selectedSound)` at the current volume.

### 3.4 Theme Polish (`pkg/tui/theme`)
- **LCARS:** Curved pill caps (`◖`, `◗`), styled color headers, audio telemetry badge `🔊 80%`.
- **CRT:** Phosphor scanline text effects and warning badge `[SND: 80%]`.
- **Modern:** Clean UTF-8 borders with minimalist volume badge.

---

## 4. Verification & Testing Strategy

1. **Automated Test Silencing:**
   - Verified that `testing.Testing()` suppresses all terminal bell writes and OS audio binary execution across all unit tests and journey tests.
2. **Audio Unit Tests (`pkg/audio`):**
   - Validate WAV RIFF header encoding, 8kHz 16-bit mono sample integrity, volume scaling, and amplitude bounds $[-32768, 32767]$.
3. **Config Unit Tests (`pkg/config`):**
   - Test default generation, atomic save/load roundtrips, invalid JSON recovery, and boundary clamping.
4. **Options Modal Tests (`pkg/tui/components/optionsmodal`):**
   - Test keyboard navigation across volume and sound test rows, volume slider increments/decrements, and audio mode toggling.
5. **Animation Tests (`pkg/tui/anim`):**
   - Test `PlasmaAnimation`, `CloakAnimation`, and `WebAnimation` frame advancement and completion.
6. **Full Test Suite:**
   - `go test -count=1 -race ./...` (100% pass across all packages).
   - `go vet ./...` (0 issues).
