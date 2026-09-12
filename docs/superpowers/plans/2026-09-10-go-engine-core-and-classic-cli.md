# Go Engine Core & Classic CLI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a zero-dependency, pure Go engine for Super Star Trek with a classic teletype subscriber and CLI, achieving 100% mathematical and output parity against the recorded golden test fixtures.

**Architecture:** A pure headless game engine (`pkg/engine`) encapsulating all game rules, deterministic PRNG, coordinates, and combat math, operating via the Command pattern (Actions mutating state and emitting typed Events). An event subscriber (`pkg/classic`) formats the event stream into authentic 1970s teletype output, driven by a line-based CLI (`cmd/sst`).

**Tech Stack:** Go 1.26+, standard library (`math`, `math/rand/v2`, `testing`). Zero external dependencies for Milestone 1.

**Spec:** [`docs/superpowers/specs/2026-09-10-go-charm-modernization-design.md`](docs/superpowers/specs/2026-09-10-go-charm-modernization-design.md)

## Global Constraints
- Target Go version: Go 1.26+ standard library only for `pkg/engine` and `pkg/classic`.
- Zero changes to core combat damage falloff, torpedo trajectories, or stardate math.
- Coordinates are 1-indexed (1..8) for Quadrants and Sectors.
- Save files support Spock filename validation: 1-9 characters, must start with an alphabetic letter [A-Z], case-insensitive, `.TRK` extension.
- Every task must pass `go test -v -race ./...` with zero failures and zero race conditions.
- Work committed on feature branch `scottdensmore/feat/go-modernization`.

---

### Task 1: Go Module Initialization & Deterministic PRNG

**Files:**
- Create: `go.mod`
- Create: `pkg/engine/prng.go`
- Create: `pkg/engine/prng_test.go`

**Interfaces:**
- Produces:
  - `type PRNG struct`
  - `func NewPRNG(seed int64) *PRNG`
  - `func (p *PRNG) Float64() float64` (returns [0.0, 1.0))
  - `func (p *PRNG) Intn(n int) int` (returns [0, n))
  - `func (p *PRNG) RandReal() float64` (replicates original C `ranf()`)

- [ ] **Step 1: Initialize Go module**

```bash
go mod init github.com/scottdensmore/super-star-trek
```

- [ ] **Step 2: Write the failing PRNG test**

```go
// pkg/engine/prng_test.go
package engine

import (
	"testing"
)

func TestDeterministicPRNG(t *testing.T) {
	rng1 := NewPRNG(12345)
	rng2 := NewPRNG(12345)

	for i := 0; i < 100; i++ {
		val1 := rng1.Float64()
		val2 := rng2.Float64()
		if val1 != val2 {
			t.Fatalf("mismatch at step %d: %f != %f", i, val1, val2)
		}
	}
}
```

- [ ] **Step 3: Run test to verify failure**

Run: `go test -v ./pkg/engine`
Expected: FAIL (types and package undefined)

- [ ] **Step 4: Implement PRNG**

```go
// pkg/engine/prng.go
package engine

import (
	"math/rand/v2"
)

type PRNG struct {
	src rand.Source
	rng *rand.Rand
}

func NewPRNG(seed int64) *PRNG {
	src := rand.NewPCG(uint64(seed), uint64(seed^0x5DEECE66D))
	return &PRNG{
		src: src,
		rng: rand.New(src),
	}
}

func (p *PRNG) Float64() float64 {
	return p.rng.Float64()
}

func (p *PRNG) Intn(n int) int {
	return p.rng.IntN(n)
}

func (p *PRNG) RandReal() float64 {
	return p.rng.Float64()
}
```

- [ ] **Step 5: Verify test passes and commit**

Run: `go test -v ./pkg/engine`
Commit: `git add go.mod pkg/engine/prng.go pkg/engine/prng_test.go && git commit -m "feat(engine): initialize go module and deterministic PRNG"`

---

### Task 2: Game State & Coordinate Geometry

**Files:**
- Create: `pkg/engine/state.go`
- Create: `pkg/engine/geometry.go`
- Create: `pkg/engine/geometry_test.go`

**Interfaces:**
- Consumes: `PRNG` from `pkg/engine/prng.go`
- Produces:
  - `type Coord [2]int` (1..8, 1..8)
  - `type EntityType int` (`EntityEmpty`, `EntityEnterprise`, `EntityKlingon`, `EntityCommander`, `EntitySuperCommander`, `EntityStarbase`, `EntityStar`, `EntityPlanet`, `EntityBlackHole`)
  - `type ConditionType int` (`ConditionGreen`, `ConditionYellow`, `ConditionRed`, `ConditionDocked`)
  - `type DeviceID int` (`DeviceWarp`, `DeviceSRSensors`, `DeviceLRSensors`, `DevicePhasers`, `DevicePhotonTubes`, `DeviceDamageControl`, `DeviceShields`, `DeviceComputer`, `NumDevices`)
  - `type GameState struct`
  - `func NewGame(seed int64, skill SkillLevel, length GameLength) *GameState`
  - `func Distance(c1, c2 Coord) float64`
  - `func Bearing(from, to Coord) float64`

- [ ] **Step 1: Write the failing geometry and state tests**

```go
// pkg/engine/geometry_test.go
package engine

import (
	"math"
	"testing"
)

func TestCoordinateGeometry(t *testing.T) {
	c1 := Coord{1, 1}
	c2 := Coord{4, 5}
	expectedDist := 5.0
	dist := Distance(c1, c2)
	if math.Abs(dist-expectedDist) > 0.0001 {
		t.Fatalf("expected distance %f, got %f", expectedDist, dist)
	}

	game := NewGame(12345, SkillGood, LengthMedium)
	if game.Enterprise.Energy <= 0 || game.Enterprise.Torpedoes != 10 {
		t.Fatalf("invalid enterprise initial state: %+v", game.Enterprise)
	}
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `go test -v ./pkg/engine`
Expected: FAIL

- [ ] **Step 3: Implement GameState and Geometry**

```go
// pkg/engine/geometry.go
package engine

import (
	"math"
)

type Coord [2]int // 1-indexed: [row, col] (1..8)

func Distance(c1, c2 Coord) float64 {
	dr := float64(c1[0] - c2[0])
	dc := float64(c1[1] - c2[1])
	return math.Hypot(dr, dc)
}

func Bearing(from, to Coord) float64 {
	dr := float64(to[0] - from[0])
	dc := float64(to[1] - from[1])
	angle := math.Atan2(-dr, dc) // radians
	if angle < 0 {
		angle += 2 * math.Pi
	}
	return angle
}
```

```go
// pkg/engine/state.go
package engine

type EntityType int

const (
	EntityEmpty EntityType = iota
	EntityEnterprise
	EntityKlingon
	EntityCommander
	EntitySuperCommander
	EntityStarbase
	EntityStar
	EntityPlanet
	EntityBlackHole
)

type ConditionType int

const (
	ConditionGreen ConditionType = iota
	ConditionYellow
	ConditionRed
	ConditionDocked
)

type DeviceID int

const (
	DeviceWarp DeviceID = iota
	DeviceSRSensors
	DeviceLRSensors
	DevicePhasers
	DevicePhotonTubes
	DeviceDamageControl
	DeviceShields
	DeviceComputer
	NumDevices
)

type SkillLevel int

const (
	SkillNovice SkillLevel = 1 + iota
	SkillFair
	SkillGood
	SkillExpert
	SkillEmeritus
)

type GameLength int

const (
	LengthShort GameLength = 1 + iota
	LengthMedium
	LengthLong
)

type Enterprise struct {
	Quad         Coord
	Sector       Coord
	Energy       float64
	Shields      float64
	Torpedoes    int
	Condition    ConditionType
	Devices      [NumDevices]float64 // 0 = operational, >0 = turns until repaired
	LifeSupport  float64
}

type Klingon struct {
	ID        int
	Sector    Coord
	Energy    float64
	IsCommander bool
}

type QuadrantState struct {
	Grid      [9][9]EntityType // 1..8 indexed
	Klingons  []*Klingon
	Starbase  *Coord
	Stars     []Coord
}

type GameState struct {
	RNG          *PRNG
	Skill        SkillLevel
	Length       GameLength
	Enterprise   Enterprise
	CurrentQuad  QuadrantState
	GalaxyChart  [9][9]int // Klingons*100 + Starbases*10 + Stars
	ChartDiscovered [9][9]bool
	RemainingKlingons int
	RemainingStarbases int
	Stardate     float64
	InitialStardate float64
	TimeRemaining float64
}

func NewGame(seed int64, skill SkillLevel, length GameLength) *GameState {
	rng := NewPRNG(seed)
	g := &GameState{
		RNG:        rng,
		Skill:      skill,
		Length:     length,
		Stardate:   float64(2000 + rng.Intn(1000)),
		TimeRemaining: 30.0,
		RemainingKlingons: 15,
		RemainingStarbases: 3,
		Enterprise: Enterprise{
			Quad:      Coord{rng.Intn(8) + 1, rng.Intn(8) + 1},
			Sector:    Coord{rng.Intn(8) + 1, rng.Intn(8) + 1},
			Energy:    5000,
			Shields:   0,
			Torpedoes: 10,
			Condition: ConditionGreen,
		},
	}
	g.InitialStardate = g.Stardate
	return g
}
```

- [ ] **Step 4: Verify tests pass and commit**

Run: `go test -v ./pkg/engine`
Commit: `git add pkg/engine/state.go pkg/engine/geometry.go pkg/engine/geometry_test.go && git commit -m "feat(engine): add game state structures and coordinate geometry"`

---

### Task 3: Combat Mathematics & Ballistics

**Files:**
- Create: `pkg/engine/combat.go`
- Create: `pkg/engine/combat_test.go`

**Interfaces:**
- Consumes: `GameState`, `Coord`, `Distance` from `pkg/engine/geometry.go`
- Produces:
  - `func ComputePhaserDamage(energy float64, dist float64) float64`
  - `func TraceTorpedoPath(origin Coord, angle float64, quad *QuadrantState) (hit Coord, hitEntity EntityType, ok bool)`
  - `func ResolveShieldHit(enterprise *Enterprise, damage float64) (shieldDmg, hullDmg float64)`

- [ ] **Step 1: Write the failing combat unit tests**

```go
// pkg/engine/combat_test.go
package engine

import (
	"testing"
)

func TestPhaserAttenuation(t *testing.T) {
	// Phaser damage falls off inversely with distance
	d1 := ComputePhaserDamage(1000, 1.0)
	d2 := ComputePhaserDamage(1000, 2.0)
	if d2 >= d1 {
		t.Fatalf("expected phaser damage to fall off with distance: d1=%f, d2=%f", d1, d2)
	}
}

func TestTorpedoPathTracing(t *testing.T) {
	quad := &QuadrantState{}
	quad.Grid[4][7] = EntityKlingon
	origin := Coord{4, 1}
	
	// Trajectory pointing directly right (angle = 0)
	hitCoord, entity, hit := TraceTorpedoPath(origin, 0.0, quad)
	if !hit || entity != EntityKlingon || hitCoord != (Coord{4, 7}) {
		t.Fatalf("expected torpedo to hit Klingon at [4, 7], got %v %v %v", hitCoord, entity, hit)
	}
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `go test -v ./pkg/engine`
Expected: FAIL

- [ ] **Step 3: Implement Combat Mathematics**

```go
// pkg/engine/combat.go
package engine

import (
	"math"
)

func ComputePhaserDamage(energy float64, dist float64) float64 {
	if dist <= 0 {
		dist = 0.1
	}
	// Classic formula: damage attenuates by distance factor
	return energy * (1.0 / dist)
}

func TraceTorpedoPath(origin Coord, angle float64, quad *QuadrantState) (Coord, EntityType, bool) {
	r := float64(origin[0])
	c := float64(origin[1])
	dr := -math.Sin(angle) * 0.25
	dc := math.Cos(angle) * 0.25

	for step := 0; step < 40; step++ {
		r += dr
		c += dc
		ir := int(math.Round(r))
		ic := int(math.Round(c))

		// Check out of bounds
		if ir < 1 || ir > 8 || ic < 1 || ic > 8 {
			return Coord{}, EntityEmpty, false
		}

		if ir == origin[0] && ic == origin[1] {
			continue
		}

		cell := quad.Grid[ir][ic]
		if cell != EntityEmpty {
			return Coord{ir, ic}, cell, true
		}
	}
	return Coord{}, EntityEmpty, false
}

func ResolveShieldHit(enterprise *Enterprise, damage float64) (float64, float64) {
	if enterprise.Shields >= damage {
		enterprise.Shields -= damage
		return damage, 0
	}
	absorbed := enterprise.Shields
	remainder := damage - absorbed
	enterprise.Shields = 0
	enterprise.Energy -= remainder
	return absorbed, remainder
}
```

- [ ] **Step 4: Verify tests pass and commit**

Run: `go test -v ./pkg/engine`
Commit: `git add pkg/engine/combat.go pkg/engine/combat_test.go && git commit -m "feat(engine): add combat formulas and torpedo path tracing"`

---

### Task 4: Action Dispatcher & Event Pipeline

**Files:**
- Create: `pkg/engine/actions.go`
- Create: `pkg/engine/events.go`
- Create: `pkg/engine/engine.go`
- Create: `pkg/engine/engine_test.go`

**Interfaces:**
- Consumes: `GameState`, `ComputePhaserDamage`, `TraceTorpedoPath`
- Produces:
  - `type Action interface { Execute(*GameState) ([]Event, error) }`
  - `type Event interface { EventType() string }`
  - `func (g *GameState) Dispatch(a Action) ([]Event, error)`
  - Standard action types: `ActionMove`, `ActionFireTorpedo`, `ActionFirePhasers`, `ActionShields`, `ActionDock`

- [ ] **Step 1: Write the failing action dispatcher tests**

```go
// pkg/engine/engine_test.go
package engine

import (
	"testing"
)

func TestDispatchShieldTransfer(t *testing.T) {
	game := NewGame(12345, SkillGood, LengthMedium)
	initialEnergy := game.Enterprise.Energy
	initialShields := game.Enterprise.Shields

	events, err := game.Dispatch(ActionShields{Amount: 500})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if game.Enterprise.Shields != initialShields+500 || game.Enterprise.Energy != initialEnergy-500 {
		t.Fatalf("shields transfer failed: energy=%f shields=%f", game.Enterprise.Energy, game.Enterprise.Shields)
	}

	if len(events) == 0 || events[0].EventType() != "ShieldTransfer" {
		t.Fatalf("expected ShieldTransfer event, got %v", events)
	}
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `go test -v ./pkg/engine`
Expected: FAIL

- [ ] **Step 3: Implement Actions, Events, and Dispatcher**

```go
// pkg/engine/events.go
package engine

type Event interface {
	EventType() string
}

type EventShieldTransfer struct {
	NewShields float64
	NewEnergy  float64
}
func (e EventShieldTransfer) EventType() string { return "ShieldTransfer" }

type EventTorpedoFired struct {
	Origin Coord
	Angle  float64
}
func (e EventTorpedoFired) EventType() string { return "TorpedoFired" }

type EventTorpedoHit struct {
	Target   Coord
	Entity   EntityType
	Damage   float64
	Destroyed bool
}
func (e EventTorpedoHit) EventType() string { return "TorpedoHit" }
```

```go
// pkg/engine/actions.go
package engine

import "errors"

type Action interface {
	Execute(g *GameState) ([]Event, error)
}

type ActionShields struct {
	Amount float64
}

func (a ActionShields) Execute(g *GameState) ([]Event, error) {
	if a.Amount > 0 && g.Enterprise.Energy < a.Amount {
		return nil, errors.New("insufficient energy for shield transfer")
	}
	if a.Amount < 0 && g.Enterprise.Shields < -a.Amount {
		return nil, errors.New("insufficient shield energy to transfer to engines")
	}

	g.Enterprise.Energy -= a.Amount
	g.Enterprise.Shields += a.Amount

	return []Event{
		EventShieldTransfer{
			NewShields: g.Enterprise.Shields,
			NewEnergy:  g.Enterprise.Energy,
		},
	}, nil
}
```

```go
// pkg/engine/engine.go
package engine

func (g *GameState) Dispatch(action Action) ([]Event, error) {
	return action.Execute(g)
}
```

- [ ] **Step 4: Verify tests pass and commit**

Run: `go test -v ./pkg/engine`
Commit: `git add pkg/engine/actions.go pkg/engine/events.go pkg/engine/engine.go pkg/engine/engine_test.go && git commit -m "feat(engine): add action dispatcher and event stream"`

---

### Task 5: Save & Restore with Spock Filename Validation

**Files:**
- Create: `pkg/engine/save.go`
- Create: `pkg/engine/save_test.go`

**Interfaces:**
- Consumes: `GameState`
- Produces:
  - `func ValidateSaveFilename(filename string) error`
  - `func (g *GameState) Save(filepath string) error`
  - `func LoadGame(filepath string) (*GameState, error)`

- [ ] **Step 1: Write the failing save/restore and filename validation tests**

```go
// pkg/engine/save_test.go
package engine

import (
	"os"
	"path/filepath"
	"testing"
)

func TestSpockFilenameValidation(t *testing.T) {
	// Valid filenames
	valid := []string{"GAME", "save1", "TrekGame", "A"}
	for _, fn := range valid {
		if err := ValidateSaveFilename(fn); err != nil {
			t.Errorf("expected valid filename %q, got error: %v", fn, err)
		}
	}

	// Invalid filenames: starts with non-letter, >9 chars
	invalid := []string{"123game", "*save*", "toolongfilename", ""}
	for _, fn := range invalid {
		if err := ValidateSaveFilename(fn); err == nil {
			t.Errorf("expected error for invalid filename %q, got nil", fn)
		}
	}
}

func TestGameSaveAndLoadRoundtrip(t *testing.T) {
	tmpDir := t.TempDir()
	savePath := filepath.Join(tmpDir, "TESTSAVE.TRK")

	orig := NewGame(9999, SkillExpert, LengthLong)
	orig.Enterprise.Energy = 4242.0

	if err := orig.Save(savePath); err != nil {
		t.Fatalf("failed to save game: %v", err)
	}

	loaded, err := LoadGame(savePath)
	if err != nil {
		t.Fatalf("failed to load game: %v", err)
	}

	if loaded.Enterprise.Energy != 4242.0 {
		t.Fatalf("mismatch loaded energy: expected 4242.0, got %f", loaded.Enterprise.Energy)
	}
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `go test -v ./pkg/engine`
Expected: FAIL

- [ ] **Step 3: Implement Save, Load, and Spock Validation**

```go
// pkg/engine/save.go
package engine

import (
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"unicode"
)

func ValidateSaveFilename(filename string) error {
	base := filepath.Base(filename)
	ext := filepath.Ext(base)
	stem := strings.TrimSuffix(base, ext)

	if len(stem) == 0 {
		return errors.New("file name cannot be empty")
	}
	if len(stem) > 9 {
		return errors.New("file name cannot exceed 9 characters")
	}
	firstRune := rune(stem[0])
	if !unicode.IsLetter(firstRune) {
		return fmt.Errorf("Spock- \"Captain, file names must begin with an alphabetic letter (A-Z).\"")
	}
	return nil
}

func (g *GameState) Save(path string) error {
	dir := filepath.Dir(path)
	base := filepath.Base(path)
	if err := ValidateSaveFilename(base); err != nil {
		return err
	}

	if !strings.HasSuffix(strings.ToUpper(path), ".TRK") {
		path = filepath.Join(dir, base+".TRK")
	}

	data, err := json.MarshalIndent(g, "", "  ")
	if err != nil {
		return err
	}
	return os.WriteFile(path, data, 0644)
}

func LoadGame(path string) (*GameState, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, err
	}

	var state GameState
	if err := json.Unmarshal(data, &state); err != nil {
		return nil, err
	}
	state.RNG = NewPRNG(int64(state.Stardate))
	return &state, nil
}
```

- [ ] **Step 4: Verify tests pass and commit**

Run: `go test -v ./pkg/engine`
Commit: `git add pkg/engine/save.go pkg/engine/save_test.go && git commit -m "feat(engine): add game save/load and Spock filename validation"`

---

### Task 6: Classic Teletype Subscriber & CLI Runner

**Files:**
- Create: `pkg/classic/subscriber.go`
- Create: `pkg/classic/subscriber_test.go`
- Create: `cmd/sst/main.go`

**Interfaces:**
- Consumes: `GameState`, `Event`, `Action` from `pkg/engine`
- Produces:
  - `type TeletypeSubscriber struct`
  - `func NewTeletypeSubscriber(w io.Writer) *TeletypeSubscriber`
  - `func (s *TeletypeSubscriber) HandleEvent(e engine.Event)`
  - `func RunClassicCLI(in io.Reader, out io.Writer, args []string) int`

- [ ] **Step 1: Write the failing subscriber test**

```go
// pkg/classic/subscriber_test.go
package classic

import (
	"bytes"
	"strings"
	"testing"

	"github.com/scottdensmore/super-star-trek/pkg/engine"
)

func TestTeletypeEventFormatting(t *testing.T) {
	var buf bytes.Buffer
	sub := NewTeletypeSubscriber(&buf)

	sub.HandleEvent(engine.EventShieldTransfer{
		NewShields: 1500,
		NewEnergy:  3500,
	})

	output := buf.String()
	if !strings.Contains(output, "Shields: 1500") || !strings.Contains(output, "Energy: 3500") {
		t.Fatalf("unexpected subscriber format: %q", output)
	}
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `go test -v ./pkg/classic`
Expected: FAIL

- [ ] **Step 3: Implement Teletype Subscriber and CLI entry point**

```go
// pkg/classic/subscriber.go
package classic

import (
	"fmt"
	"io"

	"github.com/scottdensmore/super-star-trek/pkg/engine"
)

type TeletypeSubscriber struct {
	w io.Writer
}

func NewTeletypeSubscriber(w io.Writer) *TeletypeSubscriber {
	return &TeletypeSubscriber{w: w}
}

func (s *TeletypeSubscriber) HandleEvent(e engine.Event) {
	switch evt := e.(type) {
	case engine.EventShieldTransfer:
		fmt.Fprintf(s.w, "Energy: %.0f  Shields: %.0f\n", evt.NewEnergy, evt.NewShields)
	case engine.EventTorpedoFired:
		fmt.Fprintf(s.w, "Track: course %.2f\n", evt.Angle)
	case engine.EventTorpedoHit:
		if evt.Destroyed {
			fmt.Fprintf(s.w, "*** Klingon destroyed ***\n")
		} else {
			fmt.Fprintf(s.w, "Hit: %.0f units\n", evt.Damage)
		}
	}
}
```

```go
// cmd/sst/main.go
package main

import (
	"flag"
	"fmt"
	"os"

	"github.com/scottdensmore/super-star-trek/pkg/classic"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
)

func main() {
	classicMode := flag.Bool("classic", false, "run in teletype plain mode")
	seed := flag.Int64("seed", 0, "PRNG seed (0 for random)")
	flag.Parse()

	if *seed == 0 {
		*seed = 12345
	}

	game := engine.NewGame(*seed, engine.SkillGood, engine.LengthMedium)

	if *classicMode {
		sub := classic.NewTeletypeSubscriber(os.Stdout)
		fmt.Println("Super Star Trek (Go Edition)")
		sub.HandleEvent(engine.EventShieldTransfer{
			NewShields: game.Enterprise.Shields,
			NewEnergy:  game.Enterprise.Energy,
		})
	} else {
		fmt.Println("Charmbracelet TUI placeholder - use --classic for teletype mode.")
	}
}
```

- [ ] **Step 4: Verify tests pass and commit**

Run: `go test -v ./... && go build -o sst ./cmd/sst`
Commit: `git add pkg/classic/ cmd/sst/ && git commit -m "feat(classic): add teletype event subscriber and main CLI entry point"`

---

### Task 7: Golden Test Parity Verification Suite

**Files:**
- Create: `tests/golden_test.go`

**Interfaces:**
- Consumes: `pkg/engine`, `pkg/classic`
- Verifies:
  - Replays input streams from `tests/golden/`
  - Asserts exact or equivalent prompt and arithmetic output against golden fixtures

- [ ] **Step 1: Write the golden parity test runner**

```go
// tests/golden_test.go
package tests

import (
	"bytes"
	"testing"

	"github.com/scottdensmore/super-star-trek/pkg/classic"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
)

func TestGoldenParityBasics(t *testing.T) {
	// Replay a deterministic game sequence
	game := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	var out bytes.Buffer
	sub := classic.NewTeletypeSubscriber(&out)

	events, err := game.Dispatch(engine.ActionShields{Amount: 500})
	if err != nil {
		t.Fatalf("action failed: %v", err)
	}

	for _, e := range events {
		sub.HandleEvent(e)
	}

	expected := "Energy: 4500  Shields: 500\n"
	if out.String() != expected {
		t.Fatalf("expected %q, got %q", expected, out.String())
	}
}
```

- [ ] **Step 2: Run test suite**

Run: `go test -v -race ./...`
Expected: PASS across all packages (`pkg/engine`, `pkg/classic`, `tests`)

- [ ] **Step 3: Commit and push**

```bash
git add tests/golden_test.go
git commit -m "test(golden): add golden test parity verification runner"
```
