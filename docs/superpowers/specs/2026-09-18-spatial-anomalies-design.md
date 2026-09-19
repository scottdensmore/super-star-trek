# Spatial Anomalies & Environmental Hazards Engine Design Specification

- **Date:** 2026-09-18
- **Status:** Approved
- **Target Branch:** `scottdensmore/feat/spatial-anomalies`
- **Issue Reference:** Closes part of [#247](https://github.com/scottdensmore/super-star-trek/issues/247)

---

## 1. Executive Summary & Goals

Super Star Trek combines strategic galaxy navigation with tactical sector combat. While classic gameplay features static obstacles (stars, starbases, planets, and enemy Klingon warships), adding dynamic environmental phenomena enriches tactical depth and replayability.

This specification details **Subsystem 1: Spatial Anomalies & Environmental Hazards Engine**, establishing the foundational physical hazards and environmental modifiers across both quadrant navigation and sector tactics. Curated tactical challenge scenarios (e.g. *Kobayashi Maru*, *Mutara Nebula* duel, *Starbase Under Siege*) will build directly upon this foundation in Subsystem 2.

### Key Goals
1. **Hybrid Scale Hazards:** Seamlessly model macro quadrant environments (dense nebulae, volatile ion storms) alongside micro tactical sector objects (gravitational black holes, subspace wormholes).
2. **Symmetrical Tactical Physics:** Environmental effects govern both the Enterprise and Klingon warships equally, unlocking emergent tactical strategies (e.g. luring Klingons into a nebula to disable their shields or baiting them near a singularity).
3. **100% Classic Parity by Default:** Standard gameplay modes (`ProfileCasual`, `ProfileNormal`) remain 100% true to 1978 rules with zero anomalies unless explicitly activated via `GameRules` or the `--anomalies` CLI flag. Hardcore and Nightmare profiles feature anomalies by default.
4. **Rich Event-Driven Visuals:** All anomalies emit strongly typed `engine.Event` instances, powering responsive Bubbletea TUI terminal rendering and WebAssembly CRT teletype logs.
5. **Seamless Save Compatibility:** Maintain full backward- and forward-compatibility with JSON save states in `pkg/engine/save.go`.

---

## 2. Architecture & Data Structures

```
┌─────────────────────────────────────────────────────────────┐
│                         GameState                           │
│                                                             │
│   QuadrantEnv: [9][9]EnvironmentType                        │
│   Rules.SpatialAnomalies: bool                              │
│                                                             │
│   ┌────────────────────────┐       ┌─────────────────────┐  │
│   │   CurrentQuadrant      │       │     GalaxyChart     │  │
│   │                        │       │                     │  │
│   │   Grid: [9][9]Entity   │       │   [9][9]int         │  │
│   │   (Stars, Bases,       │       │   (Klingons/Bases/  │  │
│   │    BlackHoles,         │       │    Stars)           │  │
│   │    Wormholes)          │       │                     │  │
│   └───────────┬────────────┘       └───────────┬─────────┘  │
└───────────────┼────────────────────────────────┼────────────┘
                │                                │
                ▼                                ▼
┌─────────────────────────────────────────────────────────────┐
│                    Engine Action Pipeline                   │
│                                                             │
│  - ActionMove: Course drift (Ion Storm), gravity well (BH), │
│                instant transit (Wormhole)                   │
│  - ActionShields: Shield formation check (Nebula)           │
│  - ActionLRScan: EM sensor masking (Nebula)                 │
│  - ActionFireTorpedo: Ballistic singularity absorption (BH) │
│  - KlingonTurn: Symmetrical damage, shield & drift physics  │
└──────────────────────────────┬──────────────────────────────┘
                               │ Emits
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                     Typed Event Stream                      │
│                                                             │
│  EventAnomalyDiscovered, EventHazardTriggered,              │
│  EventWormholeJump, EventSingularityAbsorption              │
└──────────────┬───────────────────────────────┬──────────────┘
               │                               │
               ▼                               ▼
┌─────────────────────────────┐ ┌─────────────────────────────┐
│     Bubbletea TUI (TUI)     │ │   WebAssembly (cmd/wasm)    │
│  - Sector Grid symbols      │ │  - ANSI Teletype format     │
│  - Status & Hazard banners  │ │  - CRT Log stream           │
│  - Condition bar indicators │ │  - LocalStorage persistence │
└─────────────────────────────┘ └─────────────────────────────┘
```

### 2.1 Types & Enums (`pkg/engine/state.go`)

```go
// EnvironmentType defines the ambient phenomenon within a galactic quadrant.
type EnvironmentType int

const (
    EnvNormal EnvironmentType = iota
    EnvNebula                 // High ionization: shields offline, LRS blinded
    EnvIonStorm               // Subspace turbulence: navigation course drift, energy surges
)

// EntityType additions (expanding existing enum in pkg/engine/state.go)
const (
    EntityEmpty EntityType = iota
    EntityEnterprise
    EntityKlingon
    EntityCommander
    EntitySuperCommander
    EntityStarbase
    EntityStar
    EntityPlanet
    EntityBlackHole // Value 8 (existing in engine)
    EntityWormhole  // Value 9 (new tactical sector entity)
)
```

### 2.2 GameState Extensions (`pkg/engine/state.go`)

```go
type GameState struct {
    // ... existing fields ...
    QuadrantEnv [9][9]EnvironmentType `json:"quadrant_env"`
    // ...
}
```

### 2.3 Rules Extensions (`pkg/engine/rules.go`)

```go
type GameRules struct {
    // ... existing fields ...
    SpatialAnomalies bool `json:"spatial_anomalies"`
}
```

- **`ProfileCasual`**: `SpatialAnomalies: false`
- **`ProfileNormal`**: `SpatialAnomalies: false`
- **`ProfileHardcore`**: `SpatialAnomalies: true`
- **`ProfileNightmare`**: `SpatialAnomalies: true`
- **CLI Flag**: `sst --anomalies` or `sst --no-anomalies` overrides profile default.

---

## 3. Environmental Mechanics & Physics

### 3.1 Nebulae (`EnvNebula`)
- **Shield Interference:**
  - Upon entering an `EnvNebula` quadrant, the Enterprise's defensive shields collapse to 0. Remaining shield energy is instantly channeled back into `Enterprise.Energy` reserves (clamped to max capacity 5000).
  - Calling `she <amount>` while in a nebula returns error: `"Sensors indicate extreme particle ionization: shields cannot hold cohesive geometry."`
  - **Klingon Symmetry:** All Klingon vessels present in a nebula have their shield efficiency reduced to 0. Direct hits penetrate straight to their energy banks.
- **Sensor Blindness:**
  - `ActionLRScan` scanning an `EnvNebula` quadrant returns `***` (Sensor Blind / Static) instead of the standard 3-digit numeric reading.
  - Short-range sensors operate with ambient haze indicators in the TUI.

### 3.2 Ion Storms (`EnvIonStorm`)
- **Navigation Drift:**
  - When calculating vector movement (`ActionMove`) within or through an `EnvIonStorm` quadrant, each step has a 25% chance (determined by `g.RNG.Float64() < 0.25`) of deflecting the trajectory by $\pm 1$ sector laterally.
  - If a drift causes collision with an occupied cell (e.g. Star or Klingon), standard collision mechanics apply.
- **Electrostatic Surges:**
  - At turn end / stardate advance inside an ion storm, the Enterprise has a 30% chance of suffering a power surge, deducting 50–150 energy units or adding $0.5$–$1.5$ repair turns to a randomly selected device.
  - Klingon vessels have an identical chance to suffer a power drain.

### 3.3 Black Holes (`EntityBlackHole` / `@`)
- **Event Horizon & Movement Resistance:**
  - Sectors orthogonal or diagonal to `EntityBlackHole` (Chebyshev distance = 1) lie within the gravity well.
  - Moving into or through an event horizon sector doubles the warp/impulse energy expenditure ($2\times$).
- **Singularity Collision:**
  - Attempting to move into the exact coordinate occupied by `EntityBlackHole` destroys the vessel immediately, ending the game with `GameOverLost` (*"USS Enterprise torn apart by gravitational tidal forces."*).
- **Weapon Absorption:**
  - In `ActionFireTorpedo`, if a torpedo's line of sight intersects `EntityBlackHole`, the torpedo is swallowed by the singularity with an explosion event and does not proceed to sectors beyond.
  - Klingons repelled or maneuvering into `EntityBlackHole` are instantly crushed and removed from `g.CurrentQuad.Klingons`.

### 3.4 Wormholes (`EntityWormhole` / `>W<`)
- **Subspace Transit:**
  - Moving onto `EntityWormhole` does not block movement. Instead, it triggers an instant non-linear transit:
    - Target Quadrant: `Coord{rng.Intn(8)+1, rng.Intn(8)+1}`.
    - Target Sector: `Coord{rng.Intn(8)+1, rng.Intn(8)+1}` (rerolled if occupied).
    - Energy Cost: `0`.
    - Stardate Advance: $0.1$ to $0.3$ stardates.
  - Emits `EventWormholeJump`.

---

## 4. Event Stream Specifications (`pkg/engine/events.go`)

Four new event types will be introduced implementing the `engine.Event` interface:

```go
// EventAnomalyDiscovered is emitted when the ship discovers or enters an active environmental quadrant.
type EventAnomalyDiscovered struct {
    Quad Coord
    Env  EnvironmentType
}

// EventHazardTriggered is emitted when an environmental hazard affects the ship.
type EventHazardTriggered struct {
    HazardType  string  // "ion_storm_drift", "ion_surge", "gravity_well"
    Description string
    EnergyDrain float64
}

// EventWormholeJump is emitted upon entering a wormhole transit rift.
type EventWormholeJump struct {
    FromQuad   Coord
    FromSector Coord
    ToQuad     Coord
    ToSector   Coord
    TimeDelta  float64
}

// EventSingularityAbsorption is emitted when an entity or weapon is pulled into a black hole.
type EventSingularityAbsorption struct {
    Sector   Coord
    Target   EntityType
    Weapon   string // "torpedo", "ship"
}
```

---

## 5. UI & Presentation Specifications

### 5.1 Tactical Sector Grid (`pkg/tui/components/sectorgrid/grid.go`)
- `EntityBlackHole`: Renders as ` @ ` with deep purple / violet styling (`#7D56F4`).
- `EntityWormhole`: Renders as `>W<` with vibrant cyan styling (`#04D9FF`).

### 5.2 Status Bar & Ambient Warning Banner (`pkg/tui/`)
- When occupying an anomaly quadrant, the status bar displays an environmental advisory:
  - `Hazard: MUTARA NEBULA (SHIELDS OFFLINE)`
  - `Hazard: ION STORM (TURBULENCE DETECTED)`

### 5.3 WebAssembly Teletype Formatter (`cmd/wasm/formatter.go`)
- Adds ANSI-colored log lines for anomaly events:
  ```text
  *** ENVIRONMENT ALERT: Entering Mutara Nebula. Electromagnetic dispersion drops shields to 0! ***
  *** NAVIGATIONAL WARNING: Ion storm turbulence deflected course by 1 sector! ***
  *** GRAVITATIONAL SINGULARITY: Photon torpedo absorbed into event horizon! ***
  *** SUBSPACE RIFT: Wormhole transit completed to Quadrant [5, 2] Sector [3, 6]! ***
  ```

---

## 6. Procedural Generation & Save Game Parity

### 6.1 Galaxy Generation (`pkg/engine/state.go`)
When `rules.SpatialAnomalies == true`:
1. **Nebulae Placement:** 2 to 4 quadrants randomly selected and assigned `EnvNebula` (ensuring starting quadrant is not a nebula unless playing a designated scenario).
2. **Ion Storm Placement:** 2 to 4 quadrants randomly selected and assigned `EnvIonStorm`.
3. **Sector Objects:**
   - 1 to 2 quadrants seeded with a single `EntityBlackHole`.
   - 1 to 2 quadrants seeded with a single `EntityWormhole`.
   - Placed in empty sectors (`EntityEmpty`) avoiding starbases.

### 6.2 Serialization (`pkg/engine/save.go`)
- JSON tags for `QuadrantEnv` and `EntityWormhole` ensure standard Go JSON serialization handles save games smoothly.
- Unmarshaling older save files without `quadrant_env` will automatically default to zero values (`EnvNormal`), preserving 100% backward compatibility.

---

## 7. Testing Strategy & Verification Plan

### 7.1 Engine Tests (`pkg/engine/`)
1. **`anomalies_test.go`**:
   - Verify procedural generation with deterministic seeds.
   - Verify `rules.SpatialAnomalies == false` generates zero anomalies.
   - Verify proper counts when `rules.SpatialAnomalies == true`.
2. **`actions_test.go`**:
   - `TestNebulaShieldDischarge`: Shields drop to 0 on entry, energy returned to main power.
   - `TestNebulaShieldBlock`: `ActionShields` returns error inside `EnvNebula`.
   - `TestNebulaLRSMasking`: `ActionLRScan` masks nebula cells as `***`.
   - `TestIonStormDrift`: Movement with mocked drift verifying deflection.
   - `TestIonStormSurge`: Energy drain event emitted.
   - `TestBlackHoleGravityCost`: Adjacent movement costs $2\times$ energy.
   - `TestBlackHoleFatalStep`: Stepping into black hole ends game.
   - `TestBlackHoleTorpedoAbsorption`: Torpedo path blocked and absorbed.
   - `TestWormholeJump`: Stepping onto wormhole moves ship to new coordinates.
   - `TestKlingonNebulaSymmetry`: Klingons in nebulae take unshielded damage.
3. **`save_test.go`**:
   - Save/load round-trip verifying anomaly state retention.
   - Backward-compatibility verification of legacy save files.

### 7.2 UI & Component Tests
1. **`pkg/tui/components/sectorgrid/grid_test.go`**:
   - Test rendering of `EntityBlackHole` and `EntityWormhole`.
2. **`cmd/wasm/formatter_test.go`**:
   - Test ANSI teletype formatting for all new `engine.Event` types.

### 7.3 Golden Master & Regression Suites
- `tests/golden.sh` and `ctest --preset debug` must pass with 100% parity when anomalies are disabled.
- `go test -v -race ./...` must pass with zero race conditions.
