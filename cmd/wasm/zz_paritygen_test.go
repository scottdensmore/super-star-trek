package main

// Parity data generator for the Rust port (crates/sst-wasm).
//
// Run with:
//
//	SST_PARITYGEN=1 go test ./cmd/wasm -run TestParityGen -count=1
//
// It writes testdata/parity/wasm/{sessions,events,states}.txt. Every record is
// length-prefixed so that payloads (ANSI codes, \r\n, JSON) are stored raw.

import (
	"bufio"
	"encoding/json"
	"fmt"
	"math"
	"math/rand/v2"
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"testing"

	"github.com/scottdensmore/super-star-trek/pkg/engine"
)

type pgWriter struct {
	w *bufio.Writer
}

// rec writes "<op> <len(arg)> <len(out)>\n<arg><out>\n".
func (p *pgWriter) rec(op, arg, out string) {
	fmt.Fprintf(p.w, "%s %d %d\n%s%s\n", op, len(arg), len(out), arg, out)
}

// pgSession drives a Session the way the JS bridge does.
type pgSession struct {
	s        *Session
	w        *pgWriter
	reseed   bool // model the Rust deviation: reseed the PRNG after Load
	lastSave string
	dead     bool // a Go panic ended the case
}

func (p *pgSession) newSession(seed int64, diff string) {
	p.s = NewSession(seed, engine.DifficultyProfile(diff))
	p.w.rec("new", fmt.Sprintf("%d %s", seed, diff), FormatSRS(p.s.Game()))
}

func (p *pgSession) reset() {
	p.s = NewSession(0, engine.ProfileNormal)
	p.w.rec("reset", "", FormatSRS(p.s.Game()))
}

func (p *pgSession) cmd(input string) {
	if p.dead {
		return
	}
	var out string
	panicked := func() (panicked bool) {
		defer func() {
			if r := recover(); r != nil {
				out = fmt.Sprint(r)
				panicked = true
			}
		}()
		out = p.s.Execute(input)
		return false
	}()
	if panicked {
		p.w.rec("panic", input, out)
		p.dead = true
		return
	}
	p.w.rec("cmd", input, out)
}

func (p *pgSession) save() {
	if p.dead {
		return
	}
	s, err := p.s.Save()
	if err != nil {
		s = ""
	}
	p.lastSave = s
	p.w.rec("save", "", s)
}

func (p *pgSession) load(data string) {
	if p.dead {
		return
	}
	arg := data
	if data == p.lastSave && data != "" {
		arg = "@last"
	}
	err := p.s.Load(data)
	res := "ok"
	if err != nil {
		res = "err"
	} else if p.reseed {
		p.s.game.RNG = engine.NewPRNG(int64(p.s.game.Stardate))
	}
	p.w.rec("load", arg, res)
}

func (p *pgSession) begin(name string, reseed bool) {
	r := 0
	if reseed {
		r = 1
	}
	fmt.Fprintf(p.w.w, "CASE %s %d\n", name, r)
	p.reseed = reseed
	p.dead = false
	p.lastSave = ""
}

func (p *pgSession) end() {
	fmt.Fprintf(p.w.w, "END\n")
}

var pgProfiles = []string{"casual", "normal", "hardcore", "nightmare", "expert", "emeritus", "custom", "", "bogus"}

var pgFixedCommands = []string{
	"help", "?", "HELP", "help nav", "srs", "srscan", "status", "SRS", "lrs", "lrscan", "chart",
	"dam", "damages", "scenario", "scenario list", "scenario LIST", "scenario bogus",
	"scenario starbase assault", "scenario a b c", "com", "commands", "opts", "options", "settings",
	"scen", "scenarios", "tour", "orders", "theme", "theme Amber", "foobar", "xyzzy 1 2", "",
	"   ", "\t", "she", "she abc", "she 999999", "she -500", "she 0", "pha", "pha abc", "pha -5",
	"tor", "tor x", "tor 1 2 3", "nav", "nav 1", "nav q 9 9", "nav q a b", "nav s 0 3", "nav c 1 0",
	"nav 1.5 0", "nav x y", "nav q 1 1 0", "nav q 1 1 -2", "nav a b c d e", "help tor",
	"  srs  ", "Nav 1 1", "doc", "dock",
}

func pgRandomCommand(r *rand.Rand, scenarioIDs []string) string {
	coord := func() int { return r.IntN(10) } // includes out-of-range 0 and 9
	switch r.IntN(30) {
	case 0, 1:
		return fmt.Sprintf("nav %d %d", coord(), coord())
	case 2:
		return fmt.Sprintf("nav s %d %d", coord(), coord())
	case 3, 4:
		return fmt.Sprintf("nav q %d %d %d", r.IntN(8)+1, r.IntN(8)+1, r.IntN(9)+1)
	case 5:
		return fmt.Sprintf("nav q %d %d", r.IntN(8)+1, r.IntN(8)+1)
	case 6:
		return fmt.Sprintf("nav %.1f %.1f", 1+r.Float64()*8, 0.1+r.Float64()*3)
	case 7:
		return fmt.Sprintf("nav c %.2f %.1f", 1+r.Float64()*8, 0.1+r.Float64()*8)
	case 8, 9:
		return fmt.Sprintf("tor %d %d", r.IntN(8)+1, r.IntN(8)+1)
	case 10:
		return fmt.Sprintf("tor %.1f", r.Float64()*360)
	case 11:
		return fmt.Sprintf("tor c %.1f", r.Float64()*10)
	case 12, 13:
		return fmt.Sprintf("pha %d", r.IntN(1500))
	case 14, 15:
		return fmt.Sprintf("she %d", r.IntN(3000)-200)
	case 16:
		return "doc"
	case 17:
		return "srs"
	case 18:
		return "lrs"
	case 19:
		return "chart"
	case 20:
		return "dam"
	case 21:
		if r.IntN(4) == 0 {
			return "scenario " + scenarioIDs[r.IntN(len(scenarioIDs))]
		}
		return "help"
	case 22:
		return pgFixedCommands[r.IntN(len(pgFixedCommands))]
	case 23:
		return fmt.Sprintf("pha %d", 100+r.IntN(400))
	case 24:
		return fmt.Sprintf("nav %d %d", r.IntN(8)+1, r.IntN(8)+1)
	case 25:
		return fmt.Sprintf("tor %d %d", r.IntN(8)+1, r.IntN(8)+1)
	default:
		return fmt.Sprintf("nav q %d %d 6", r.IntN(8)+1, r.IntN(8)+1)
	}
}

// pgSmartCommand picks a command from the Go state (the command text is
// recorded, so the Rust replay needs no access to this logic).
func pgSmartCommand(r *rand.Rand, g *engine.GameState) string {
	q := &g.CurrentQuad
	if len(q.Klingons) > 0 && g.Enterprise.Condition != engine.ConditionDocked {
		k := q.Klingons[r.IntN(len(q.Klingons))]
		switch r.IntN(5) {
		case 0:
			return fmt.Sprintf("pha %d", 200+r.IntN(600))
		case 1:
			return fmt.Sprintf("she %d", 300+r.IntN(700))
		default:
			return fmt.Sprintf("tor %d %d", k.Sector[0], k.Sector[1])
		}
	}
	if q.Starbase != nil && g.Enterprise.Condition != engine.ConditionDocked {
		sb := *q.Starbase
		e := g.Enterprise.Sector
		if absInt(sb[0]-e[0]) <= 1 && absInt(sb[1]-e[1]) <= 1 {
			return "doc"
		}
		for _, d := range [][2]int{{0, 1}, {1, 0}, {0, -1}, {-1, 0}, {1, 1}, {-1, -1}, {1, -1}, {-1, 1}} {
			rr, cc := sb[0]+d[0], sb[1]+d[1]
			if rr >= 1 && rr <= 8 && cc >= 1 && cc <= 8 && q.Grid[rr][cc] == engine.EntityEmpty {
				return fmt.Sprintf("nav s %d %d", rr, cc)
			}
		}
	}
	switch r.IntN(8) {
	case 0:
		return "dam"
	case 1:
		return "lrs"
	case 2:
		return "chart"
	}
	// Head for a quadrant with Klingons or a starbase.
	var cands [][2]int
	for rr := 1; rr <= 8; rr++ {
		for cc := 1; cc <= 8; cc++ {
			v := g.GalaxyChart[rr][cc]
			if (v/100 > 0 || (v/10)%10 > 0) && (rr != g.Enterprise.Quad[0] || cc != g.Enterprise.Quad[1]) {
				cands = append(cands, [2]int{rr, cc})
			}
		}
	}
	if len(cands) == 0 {
		return fmt.Sprintf("nav q %d %d %d", r.IntN(8)+1, r.IntN(8)+1, 2+r.IntN(5))
	}
	c := cands[r.IntN(len(cands))]
	return fmt.Sprintf("nav q %d %d %d", c[0], c[1], 2+r.IntN(5))
}

func absInt(x int) int {
	if x < 0 {
		return -x
	}
	return x
}

func pgScenarioIDs() []string {
	var ids []string
	for _, sc := range engine.ListScenarios() {
		ids = append(ids, string(sc.ID))
		ids = append(ids, sc.Aliases...)
	}
	ids = append(ids, "KM", " Mutara", "Siege")
	return ids
}

func genSessions(t *testing.T, path string) {
	f, err := os.Create(path)
	if err != nil {
		t.Fatal(err)
	}
	defer f.Close()
	w := &pgWriter{w: bufio.NewWriter(f)}
	defer w.w.Flush()
	p := &pgSession{w: w}
	ids := pgScenarioIDs()

	// 1. Every fixed command on fresh sessions across profiles.
	for pi, diff := range pgProfiles {
		p.begin("fixed-"+diff, true)
		p.newSession(12345, diff)
		for _, c := range pgFixedCommands {
			p.cmd(c)
		}
		for _, id := range ids {
			if pi >= 3 {
				break
			}
			p.cmd("scenario " + id)
			p.cmd("srs")
			p.cmd("she 300")
			p.cmd("pha 200")
			p.cmd("tor 4 4")
		}
		p.cmd("quit")
		p.cmd("exit")
		p.cmd("q")
		p.cmd("QUIT")
		p.end()
	}

	// 2. Random scripts: seeds x profiles.
	seeds := []int64{0, 1, 2, 7, 42, 99, 777, 2024, 12345, 99999, -1, -12345, 1 << 40}
	n := 0
	for _, seed := range seeds {
		for _, diff := range pgProfiles[:7] {
			r := rand.New(rand.NewPCG(uint64(seed), uint64(n)))
			p.begin(fmt.Sprintf("random-%d-%s", seed, diff), true)
			p.newSession(seed, diff)
			steps := 40 + r.IntN(40)
			for i := 0; i < steps && !p.dead; i++ {
				switch r.IntN(25) {
				case 0:
					p.save()
					p.load(p.lastSave)
				case 1:
					if n%3 == 0 {
						p.save()
						p.reset()
						p.load(p.lastSave)
					} else {
						p.cmd("srs")
					}
				default:
					p.cmd(pgRandomCommand(r, ids))
				}
			}
			p.cmd("srs")
			p.cmd("dam")
			p.save()
			p.end()
			n++
		}
	}

	// 3. Scenario-heavy sessions.
	for i, id := range []string{"kobayashi-maru", "mutara", "siege"} {
		for k := 0; k < 6; k++ {
			r := rand.New(rand.NewPCG(uint64(i), uint64(k)))
			p.begin(fmt.Sprintf("scenario-%s-%d", id, k), true)
			p.newSession(int64(1000*i+k), pgProfiles[k%7])
			p.cmd("scenario " + id)
			for j := 0; j < 50 && !p.dead; j++ {
				p.cmd(pgRandomCommand(r, []string{id}))
			}
			p.save()
			p.end()
		}
	}

	// 3b. State-guided sessions: hunt Klingons, dock at starbases, save/load midway.
	for i, seed := range []int64{3, 11, 12345, 31337, 4242, 8, 600, 77} {
		for j, diff := range []string{"casual", "normal", "hardcore"} {
			r := rand.New(rand.NewPCG(uint64(seed), uint64(100+j)))
			p.begin(fmt.Sprintf("smart-%d-%s", seed, diff), true)
			p.newSession(seed, diff)
			for k := 0; k < 90 && !p.dead; k++ {
				if k == 45 && i%2 == 0 {
					p.save()
					p.load(p.lastSave)
				}
				p.cmd(pgSmartCommand(r, p.s.Game()))
			}
			p.cmd("srs")
			p.save()
			p.end()
		}
	}
	for i, id := range []string{"kobayashi-maru", "mutara", "siege"} {
		for k := 0; k < 4; k++ {
			r := rand.New(rand.NewPCG(uint64(50+i), uint64(k)))
			p.begin(fmt.Sprintf("smart-scenario-%s-%d", id, k), true)
			p.newSession(int64(77*k+i), "normal")
			p.cmd("scenario " + id)
			for j := 0; j < 40 && !p.dead; j++ {
				p.cmd(pgSmartCommand(r, p.s.Game()))
			}
			p.save()
			p.end()
		}
	}

	// 4. Save/load journeys (like the JS journey test), with the reseed deviation.
	for _, seed := range []int64{12345, 42, 7} {
		p.begin(fmt.Sprintf("saveload-%d", seed), true)
		p.newSession(seed, "normal")
		p.cmd("she 1500")
		p.cmd("nav 1 1")
		p.save()
		p.newSession(99999, "hardcore")
		p.cmd("srs")
		p.load(p.lastSave)
		for _, c := range []string{"srs", "lrs", "chart", "dam", "help", "pha 300", "tor 3 3", "she 200", "nav 5 5", "nav q 2 2 6", "srs", "doc"} {
			p.cmd(c)
		}
		p.save()
		p.load(p.lastSave)
		p.cmd("srs")
		p.end()
	}

	// 5. Raw Go behaviour after Load (no reseed): record until the first panic.
	for _, seed := range []int64{12345, 42, 7, 2024} {
		p.begin(fmt.Sprintf("rawload-%d", seed), false)
		p.newSession(seed, "normal")
		p.cmd("she 800")
		p.cmd("nav 2 2")
		p.save()
		p.load(p.lastSave)
		for _, c := range []string{"srs", "lrs", "chart", "dam", "help", "scenario list", "she 100", "she 200", "pha 100", "nav 3 3", "tor 4 4", "nav q 1 1 6", "srs"} {
			p.cmd(c)
		}
		p.save()
		p.end()
	}

	// 6. Load of malformed / odd payloads.
	odd := []string{"this is not valid json", "", "{}", "null", "[]", "42", `{"Enterprise":{"Energy":"x"}}`,
		`{"Enterprise":{"Energy":1234.5,"Shields":77,"Quad":[2,3],"Sector":[4,5]},"Stardate":2500.25}`, `{"Stardate":3100.9,"RNG":{}}`}
	for i, data := range odd {
		for _, reseed := range []bool{false, true} {
			p.begin(fmt.Sprintf("oddload-%d-%v", i, reseed), reseed)
			p.newSession(int64(i+1), "normal")
			p.load(data)
			for _, c := range []string{"srs", "lrs", "chart", "dam", "she 100", "nav 1 1", "tor 2 2", "pha 50"} {
				p.cmd(c)
			}
			p.save()
			p.end()
		}
	}
}

// --- events.txt: FormatCombatEvents over synthetic event lists ---

func pgFloat(r *rand.Rand) float64 {
	switch r.IntN(8) {
	case 0:
		return float64(r.IntN(2000)) + 0.5
	case 1:
		return float64(r.IntN(200))/10 + 0.05
	case 2:
		return float64(r.IntN(2000)) / 4
	case 3:
		return -r.Float64() * 500
	case 4:
		return float64(r.IntN(5000))
	case 5:
		return r.Float64() * 1e6
	case 6:
		return []float64{0, math.Copysign(0, -1), 0.125, 0.375, 2.675, 1e21, 123456789.5, -0.4}[r.IntN(8)]
	default:
		return r.Float64() * 5000
	}
}

func pgFmtF(f float64) string { return strconv.FormatFloat(f, 'g', -1, 64) }

func pgCoord(r *rand.Rand) engine.Coord { return engine.Coord{r.IntN(10), r.IntN(10)} }

// pgEvent returns an event and its DSL description.
func pgEvent(r *rand.Rand) (engine.Event, string) {
	c := pgCoord(r)
	d := func(x engine.Coord) string { return fmt.Sprintf("%d %d", x[0], x[1]) }
	b := func(v bool) string {
		if v {
			return "1"
		}
		return "0"
	}
	switch r.IntN(18) {
	case 0:
		a := pgFloat(r)
		return engine.EventTorpedoFired{Origin: c, Angle: a}, "TorpedoFired " + d(c) + " " + pgFmtF(a)
	case 1:
		ent := engine.EntityType(r.IntN(14))
		dmg := pgFloat(r)
		des := r.IntN(3) == 0
		return engine.EventTorpedoHit{Target: c, Entity: ent, Damage: dmg, Destroyed: des},
			fmt.Sprintf("TorpedoHit %s %d %s %s", d(c), ent, pgFmtF(dmg), b(des))
	case 2:
		e := pgFloat(r)
		return engine.EventPhaserFired{Energy: e}, "PhaserFired " + pgFmtF(e)
	case 3:
		dmg := pgFloat(r)
		des := r.IntN(2) == 0
		return engine.EventPhaserHit{Target: c, KlingonID: r.IntN(5), Damage: dmg, Destroyed: des},
			fmt.Sprintf("PhaserHit %s %s %s", d(c), pgFmtF(dmg), b(des))
	case 4:
		s, e := pgFloat(r), pgFloat(r)
		return engine.EventShieldTransfer{NewShields: s, NewEnergy: e}, "ShieldTransfer " + pgFmtF(s) + " " + pgFmtF(e)
	case 5:
		fq, tq, ts := pgCoord(r), pgCoord(r), pgCoord(r)
		if r.IntN(2) == 0 {
			tq = fq
		}
		return engine.EventShipMoved{FromQuad: fq, ToQuad: tq, ToSector: ts, Warp: 1},
			"ShipMoved " + d(fq) + " " + d(tq) + " " + d(ts)
	case 6:
		return engine.EventObstacleEncountered{Sector: c, Entity: engine.EntityStar}, "ObstacleEncountered " + d(c)
	case 7:
		return engine.EventDocked{Starbase: c}, "Docked " + d(c)
	case 8:
		dmg := pgFloat(r)
		return engine.EventKlingonCounterAttack{EnemyID: 1, Damage: dmg}, "KlingonCounterAttack " + pgFmtF(dmg)
	case 9:
		dev := engine.DeviceID(r.IntN(12) - 2)
		rt := pgFloat(r)
		return engine.EventSubsystemDamaged{Device: dev, RepairTime: rt}, fmt.Sprintf("SubsystemDamaged %d %s", dev, pgFmtF(rt))
	case 10:
		dev := engine.DeviceID(r.IntN(12) - 2)
		return engine.EventSubsystemRepaired{Device: dev}, fmt.Sprintf("SubsystemRepaired %d", dev)
	case 11:
		desc := []string{"Ion storm turbulence deflected course", "", "Ünïcödé 100% %d {}"}[r.IntN(3)]
		return engine.EventHazardTriggered{HazardType: "x", Description: desc}, "HazardTriggered " + desc
	case 12:
		return engine.EventSingularityAbsorption{Sector: c, Weapon: "torpedo"}, "SingularityAbsorption"
	case 13:
		tq, ts := pgCoord(r), pgCoord(r)
		return engine.EventWormholeJump{ToQuad: tq, ToSector: ts}, "WormholeJump " + d(tq) + " " + d(ts)
	case 14:
		env := engine.EnvironmentType(r.IntN(4))
		return engine.EventAnomalyDiscovered{Quad: c, Env: env}, fmt.Sprintf("AnomalyDiscovered %d", env)
	case 15:
		reason := engine.GameOverReason(r.IntN(7))
		sc := pgFloat(r)
		return engine.EventGameOver{Reason: reason, Score: sc}, fmt.Sprintf("GameOver %d %s", reason, pgFmtF(sc))
	case 16:
		return engine.EventConditionChanged{From: 0, To: 2}, "ConditionChanged"
	default:
		return engine.EventHelpCalled{}, "HelpCalled"
	}
}

func genEvents(t *testing.T, path string) {
	f, err := os.Create(path)
	if err != nil {
		t.Fatal(err)
	}
	defer f.Close()
	w := &pgWriter{w: bufio.NewWriter(f)}
	defer w.w.Flush()
	r := rand.New(rand.NewPCG(7, 11))
	for i := 0; i < 600; i++ {
		n := r.IntN(6)
		var evs []engine.Event
		var dsl []string
		for j := 0; j < n; j++ {
			ev, s := pgEvent(r)
			evs = append(evs, ev)
			dsl = append(dsl, s)
		}
		w.rec("events", strings.Join(dsl, "\n"), FormatCombatEvents(evs))
	}
}

// --- states.txt: formatters over randomized states ---

func genStates(t *testing.T, path string) {
	f, err := os.Create(path)
	if err != nil {
		t.Fatal(err)
	}
	defer f.Close()
	w := &pgWriter{w: bufio.NewWriter(f)}
	defer w.w.Flush()
	r := rand.New(rand.NewPCG(3, 5))
	for i := 0; i < 100; i++ {
		g := engine.NewGame(int64(i*31+1), engine.SkillLevel(1+i%5), engine.GameLength(1+i%3))
		g.PopulateQuadrant(g.Enterprise.Quad, g.Enterprise.Sector)
		for k := 0; k < r.IntN(30); k++ {
			g.CurrentQuad.Grid[1+r.IntN(8)][1+r.IntN(8)] = engine.EntityType(r.IntN(15))
		}
		if r.IntN(3) == 0 {
			g.Enterprise.Sector = engine.Coord{r.IntN(10), r.IntN(10)}
		}
		if r.IntN(4) == 0 {
			g.Enterprise.Quad = engine.Coord{r.IntN(10), r.IntN(10)}
		}
		g.Enterprise.Condition = engine.ConditionType(r.IntN(5))
		g.Enterprise.Energy = pgFloat(r)
		g.Enterprise.Shields = pgFloat(r)
		g.Enterprise.Torpedoes = r.IntN(12) - 1
		g.Stardate = pgFloat(r)
		g.RemainingKlingons = r.IntN(40)
		g.RemainingStarbases = r.IntN(6)
		for dev := range g.Enterprise.Devices {
			if r.IntN(3) == 0 {
				g.Enterprise.Devices[dev] = pgFloat(r)
			}
		}
		for rr := 1; rr <= 8; rr++ {
			for cc := 1; cc <= 8; cc++ {
				g.ChartDiscovered[rr][cc] = r.IntN(3) == 0
				g.ChartKnownBases[rr][cc] = r.IntN(6) == 0
				if r.IntN(10) == 0 {
					g.GalaxyChart[rr][cc] = r.IntN(2000) - 100
				}
			}
		}
		data, err := json.Marshal(g)
		if err != nil {
			t.Fatal(err)
		}
		out := FormatSRS(g) + "\x00" + FormatLRS(g) + "\x00" + FormatChart(g) + "\x00" + FormatDamages(g)
		w.rec("state", string(data), out)
	}
	// Catalog and briefings.
	w.rec("scenarios", "", FormatScenarios())
	for _, sc := range engine.ListScenarios() {
		w.rec("briefing", string(sc.ID), FormatScenarioBriefing(sc))
	}
}

func TestParityGen(t *testing.T) {
	if os.Getenv("SST_PARITYGEN") != "1" {
		t.Skip("set SST_PARITYGEN=1 to regenerate testdata/parity/wasm")
	}
	dir := filepath.Join("..", "..", "testdata", "parity", "wasm")
	if err := os.MkdirAll(dir, 0o755); err != nil {
		t.Fatal(err)
	}
	genSessions(t, filepath.Join(dir, "sessions.txt"))
	genEvents(t, filepath.Join(dir, "events.txt"))
	genStates(t, filepath.Join(dir, "states.txt"))
}
