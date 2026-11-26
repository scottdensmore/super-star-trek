package audio

// Parity reference generator for the Rust port (crates/sst-audio).
// Run with: SST_PARITYGEN=1 go test ./pkg/audio -run TestParityGen -count=1
// Writes testdata/parity/audio/ at the repository root:
//   wav_hashes.txt  SHA-256 of SynthesizeWavWithVolume and synthesizeOSPlaybackWav per sound/volume
//   wav/<id>.wav    full SynthesizeWav (volume 100) output for every sound
//   gomath.txt      bit patterns of math.Exp/Log/Sin/Cos/Pow results for many inputs
//   dispatcher.txt  random event sequences and the sounds the Dispatcher plays
//   bell.txt        TerminalBellPlayer operation sequences and observable output

import (
	"bytes"
	"crypto/sha256"
	"fmt"
	"math"
	"math/rand/v2"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/scottdensmore/super-star-trek/pkg/engine"
)

var paritySounds = []SoundID{
	SoundPhaser, SoundTorpedoLaunch, SoundExplosion, SoundRedAlert, SoundDock, SoundWarp,
	SoundDamage, SoundShields, SoundVictory, SoundDefeat, SoundCloak, SoundDecloak,
	SoundPlasmaLaunch, SoundPlasmaImpact, SoundTholianWeb, SoundWebBreached,
	SoundPointDefense, SoundCommChime, SoundComputerBeep, SoundID("unknown_sound"),
}

func TestParityGen(t *testing.T) {
	if os.Getenv("SST_PARITYGEN") != "1" {
		t.Skip("set SST_PARITYGEN=1 to regenerate parity data")
	}
	out := filepath.Join("..", "..", "testdata", "parity", "audio")
	if err := os.MkdirAll(filepath.Join(out, "wav"), 0o755); err != nil {
		t.Fatal(err)
	}
	write := func(name string, data []byte) {
		if err := os.WriteFile(filepath.Join(out, name), data, 0o644); err != nil {
			t.Fatal(err)
		}
	}

	// 1. WAV hashes and full files.
	var hb strings.Builder
	vols := []int{-20, 0, 1, 7, 25, 33, 50, 66, 73, 80, 99, 100, 150}
	for _, id := range paritySounds {
		for _, v := range vols {
			w := SynthesizeWavWithVolume(id, v)
			fmt.Fprintf(&hb, "wav %s %d %d %x\n", id, v, len(w), sha256.Sum256(w))
			o := synthesizeOSPlaybackWav(id, v)
			fmt.Fprintf(&hb, "os %s %d %d %x\n", id, v, len(o), sha256.Sum256(o))
		}
		write(filepath.Join("wav", string(id)+".wav"), SynthesizeWav(id))
	}
	write("wav_hashes.txt", []byte(hb.String()))

	// 2. math function bit patterns.
	r := rand.New(rand.NewPCG(1, 2))
	var mb strings.Builder
	one := func(fn string, x float64, y float64) {
		switch fn {
		case "exp":
			fmt.Fprintf(&mb, "exp %016x %016x\n", math.Float64bits(x), math.Float64bits(math.Exp(x)))
		case "log":
			fmt.Fprintf(&mb, "log %016x %016x\n", math.Float64bits(x), math.Float64bits(math.Log(x)))
		case "sin":
			fmt.Fprintf(&mb, "sin %016x %016x\n", math.Float64bits(x), math.Float64bits(math.Sin(x)))
		case "cos":
			fmt.Fprintf(&mb, "cos %016x %016x\n", math.Float64bits(x), math.Float64bits(math.Cos(x)))
		case "pow":
			fmt.Fprintf(&mb, "pow %016x %016x %016x\n", math.Float64bits(x), math.Float64bits(y), math.Float64bits(math.Pow(x, y)))
		}
	}
	specials := []float64{0, math.Copysign(0, -1), 1, -1, 0.5, 2, math.Inf(1), math.Inf(-1), math.NaN(),
		1e-310, -1e-310, 1e-300, 1e300, -1e300, 709.78, 709.79, -745.1, -745.2, -800, 1e-9, -1e-9,
		math.Pi, math.Pi / 4, 1 << 29, 1<<29 + 0.5, 1e10, 1e20, 1e300, math.MaxFloat64, math.SmallestNonzeroFloat64}
	for _, fn := range []string{"exp", "log", "sin", "cos"} {
		for _, x := range specials {
			one(fn, x, 0)
		}
	}
	for i := 0; i < 3000; i++ {
		one("exp", -30+35*r.Float64(), 0)
		one("exp", -5*r.Float64(), 0)
		one("log", 10*r.Float64(), 0)
		one("log", math.Exp(-700+1400*r.Float64()), 0)
		one("sin", 12000*r.Float64(), 0)
		one("sin", -10*r.Float64(), 0)
		one("cos", 12000*r.Float64(), 0)
		one("sin", math.Exp(20+600*r.Float64()), 0)
		one("cos", math.Exp(20+600*r.Float64()), 0)
		one("pow", r.Float64(), 0.75)
		one("pow", r.Float64(), 0.45)
		one("pow", 100*r.Float64(), -20+40*r.Float64())
		one("pow", 3*r.Float64(), float64(r.IntN(40)-20))
	}
	for _, x := range specials {
		for _, y := range specials {
			one("pow", x, y)
		}
	}
	write("gomath.txt", []byte(mb.String()))

	// 3. Dispatcher event sequences.
	write("dispatcher.txt", []byte(genDispatcher(rand.New(rand.NewPCG(3, 4)))))

	// 4. Terminal bell player sequences.
	write("bell.txt", []byte(genBell(rand.New(rand.NewPCG(5, 6)))))
}

type recordPlayer struct {
	played []SoundID
	muted  bool
	volume int
}

func (m *recordPlayer) Play(s SoundID)      { m.played = append(m.played, s) }
func (m *recordPlayer) SetMuted(muted bool) { m.muted = muted }
func (m *recordPlayer) IsMuted() bool       { return m.muted }
func (m *recordPlayer) SetVolume(vol int)   { m.volume = vol }
func (m *recordPlayer) Volume() int         { return m.volume }

var eventKinds = []string{
	"ShieldTransfer", "TorpedoFired", "TorpedoHit", "PhaserFired", "PhaserHit", "Docked", "ShipMoved",
	"ObstacleEncountered", "ConditionChanged", "GameOver", "KlingonCounterAttack", "SubsystemDamaged",
	"SubsystemRepaired", "LRScanCompleted", "StarbaseSurveillance", "KlingonCloakState", "HelpCalled",
	"HazardTriggered", "WormholeJump", "SingularityAbsorption", "AnomalyDiscovered", "KlingonScreening",
	"RomulanDecloak", "PlasmaLaunched", "PlasmaMoved", "PlasmaImpact", "PlasmaDissipated",
	"PlasmaIntercepted", "EntityMove", "WebSegmentLaid", "WebBreached", "WebCollapsed",
}

func genDispatcher(r *rand.Rand) string {
	damages := []float64{0, -5, 0.25, 100, 1e-9, math.Copysign(0, -1)}
	var sb strings.Builder
	for c := 0; c < 400; c++ {
		muted := r.IntN(8) == 0
		n := 1 + r.IntN(14)
		if c < len(eventKinds) {
			n = 1
		}
		var events []engine.Event
		var lines []string
		for i := 0; i < n; i++ {
			kind := eventKinds[r.IntN(len(eventKinds))]
			if c < len(eventKinds) {
				kind = eventKinds[c]
			}
			dmg := damages[r.IntN(len(damages))]
			destroyed := r.IntN(2) == 0
			var ev engine.Event
			line := "ev " + kind
			switch kind {
			case "ShieldTransfer":
				ev = engine.EventShieldTransfer{NewShields: 1, NewEnergy: 2}
			case "TorpedoFired":
				ev = engine.EventTorpedoFired{}
			case "TorpedoHit":
				ev = engine.EventTorpedoHit{Damage: dmg, Destroyed: destroyed}
				line += fmt.Sprintf(" damage=%v destroyed=%v", dmg, destroyed)
			case "PhaserFired":
				ev = engine.EventPhaserFired{Energy: 100}
			case "PhaserHit":
				ev = engine.EventPhaserHit{Damage: dmg, Destroyed: destroyed}
				line += fmt.Sprintf(" damage=%v destroyed=%v", dmg, destroyed)
			case "Docked":
				ev = engine.EventDocked{}
			case "ShipMoved":
				ev = engine.EventShipMoved{}
			case "ObstacleEncountered":
				ev = engine.EventObstacleEncountered{}
			case "ConditionChanged":
				from, to := r.IntN(4), r.IntN(4)
				ev = engine.EventConditionChanged{From: engine.ConditionType(from), To: engine.ConditionType(to)}
				line += fmt.Sprintf(" from=%d to=%d", from, to)
			case "GameOver":
				reason := r.IntN(6)
				ev = engine.EventGameOver{Reason: engine.GameOverReason(reason), Score: 10}
				line += fmt.Sprintf(" reason=%d", reason)
			case "KlingonCounterAttack":
				ev = engine.EventKlingonCounterAttack{Damage: dmg}
				line += fmt.Sprintf(" damage=%v", dmg)
			case "SubsystemDamaged":
				ev = engine.EventSubsystemDamaged{}
			case "SubsystemRepaired":
				ev = engine.EventSubsystemRepaired{}
			case "LRScanCompleted":
				ev = engine.EventLRScanCompleted{}
			case "StarbaseSurveillance":
				ev = engine.EventStarbaseSurveillance{}
			case "KlingonCloakState":
				ev = engine.EventKlingonCloakState{Cloaked: destroyed}
				line += fmt.Sprintf(" cloaked=%v", destroyed)
			case "HelpCalled":
				ev = engine.EventHelpCalled{}
			case "HazardTriggered":
				ev = engine.EventHazardTriggered{}
			case "WormholeJump":
				ev = engine.EventWormholeJump{}
			case "SingularityAbsorption":
				ev = engine.EventSingularityAbsorption{}
			case "AnomalyDiscovered":
				ev = engine.EventAnomalyDiscovered{}
			case "KlingonScreening":
				ev = engine.EventKlingonScreening{}
			case "RomulanDecloak":
				ev = engine.EventRomulanDecloak{}
			case "PlasmaLaunched":
				ev = engine.EventPlasmaLaunched{}
			case "PlasmaMoved":
				ev = engine.EventPlasmaMoved{}
			case "PlasmaImpact":
				ev = engine.EventPlasmaImpact{}
			case "PlasmaDissipated":
				ev = engine.EventPlasmaDissipated{}
			case "PlasmaIntercepted":
				ev = engine.EventPlasmaIntercepted{}
			case "EntityMove":
				ev = engine.EventEntityMove{}
			case "WebSegmentLaid":
				ev = engine.EventWebSegmentLaid{}
			case "WebBreached":
				ev = engine.EventWebBreached{}
			case "WebCollapsed":
				ev = engine.EventWebCollapsed{}
			}
			events = append(events, ev)
			lines = append(lines, line)
		}
		p := &recordPlayer{muted: muted}
		NewDispatcher(p).DispatchEvents(events)
		fmt.Fprintf(&sb, "case %d muted=%v\n", c, muted)
		for _, l := range lines {
			sb.WriteString(l + "\n")
		}
		names := make([]string, len(p.played))
		for i, s := range p.played {
			names[i] = string(s)
		}
		fmt.Fprintf(&sb, "sounds %s\n", strings.Join(names, ","))
	}
	return sb.String()
}

func genBell(r *rand.Rand) string {
	var sb strings.Builder
	for c := 0; c < 60; c++ {
		var buf bytes.Buffer
		visual := 0
		hasWriter := c%5 != 4
		hasVisual := c%3 != 2
		var p *TerminalBellPlayer
		var vb func()
		if hasVisual {
			vb = func() { visual++ }
		}
		if hasWriter {
			p = NewTerminalBellPlayer(&buf, vb)
		} else {
			p = NewTerminalBellPlayer(nil, vb)
		}
		fmt.Fprintf(&sb, "case %d writer=%v visual=%v\n", c, hasWriter, hasVisual)
		n := 5 + r.IntN(25)
		for i := 0; i < n; i++ {
			switch k := r.IntN(10); {
			case k < 7:
				id := paritySounds[r.IntN(len(paritySounds))]
				p.Play(id)
				fmt.Fprintf(&sb, "play %s", id)
			case k < 8:
				m := r.IntN(3) != 0
				p.SetMuted(m)
				fmt.Fprintf(&sb, "mute %v", m)
			default:
				v := r.IntN(140) - 20
				p.SetVolume(v)
				fmt.Fprintf(&sb, "vol %d", v)
			}
			fmt.Fprintf(&sb, " -> bytes=%q visual=%d muted=%v volume=%d\n", buf.String(), visual, p.IsMuted(), p.Volume())
		}
	}
	return sb.String()
}
