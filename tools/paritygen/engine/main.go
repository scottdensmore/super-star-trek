// Command engine generates Go reference transcripts for the Rust port of
// pkg/engine and pkg/tui/parser (crates/sst-engine). Run from the repo root:
//
//	go run ./tools/paritygen/engine
//
// Output: testdata/parity/engine/*.txt (line-oriented text, see each gen*
// function for the record format). The Rust tests in
// crates/sst-engine/tests/parity_*.rs replay the same inputs and compare
// byte for byte. Hashes are FNV-1a 64 over the bytes indicated.
package main

import (
	"bufio"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"hash/fnv"
	"math"
	"math/rand/v2"
	"os"
	"path/filepath"
	"reflect"
	"sort"
	"strconv"
	"strings"
	"unsafe"

	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/parser"
)

var outDir = "testdata/parity/engine"

type out struct {
	f *os.File
	w *bufio.Writer
}

func create(name string) *out {
	if err := os.MkdirAll(outDir, 0o755); err != nil {
		panic(err)
	}
	f, err := os.Create(filepath.Join(outDir, name))
	if err != nil {
		panic(err)
	}
	return &out{f, bufio.NewWriterSize(f, 1<<20)}
}

func (o *out) line(format string, args ...any) {
	fmt.Fprintf(o.w, format, args...)
	o.w.WriteByte('\n')
}

func (o *out) close() {
	o.w.Flush()
	o.f.Close()
}

func fnv64(b []byte) string {
	h := fnv.New64a()
	h.Write(b)
	return fmt.Sprintf("%016x", h.Sum64())
}

func mustJSON(v any) string {
	b, err := json.Marshal(v)
	if err != nil {
		return "ERR " + err.Error()
	}
	return string(b)
}

// rngState reads the PCG state of an engine.PRNG (unexported fields).
func rngState(p *engine.PRNG) string {
	if p == nil {
		return "nil"
	}
	v := reflect.ValueOf(p).Elem().FieldByName("src")
	v = reflect.NewAt(v.Type(), unsafe.Pointer(v.UnsafeAddr())).Elem()
	if v.IsNil() {
		return "nilsrc"
	}
	pcg := v.Interface().(*rand.PCG)
	b, _ := pcg.MarshalBinary()
	return hex.EncodeToString(b[4:])
}

// stateParts returns state JSON, legacy Klingons JSON and RNG state.
func stateParts(g *engine.GameState) (string, string, string) {
	if g == nil {
		return "null", "null", "nil"
	}
	return mustJSON(g), mustJSON(g.CurrentQuad.Klingons), rngState(g.RNG)
}

func stateHash(g *engine.GameState) string {
	s, k, r := stateParts(g)
	return fnv64([]byte(s + "|" + k + "|" + r))
}

func writeFull(o *out, g *engine.GameState) {
	s, k, r := stateParts(g)
	o.line("S %s", s)
	o.line("K %s", k)
	o.line("R %s", r)
}

type kv struct {
	Coord engine.Coord
	Value int
}

func eventJSON(ev engine.Event) string {
	var data any = ev
	if lr, ok := ev.(engine.EventLRScanCompleted); ok {
		var list []kv
		for c, v := range lr.ReadingsMap {
			list = append(list, kv{c, v})
		}
		sort.Slice(list, func(i, j int) bool {
			if list[i].Coord[0] != list[j].Coord[0] {
				return list[i].Coord[0] < list[j].Coord[0]
			}
			return list[i].Coord[1] < list[j].Coord[1]
		})
		data = struct {
			CenterQuad    engine.Coord
			ScannedQuads  []engine.Coord
			RelayedByBase bool
			Degraded      bool
			Readings      [3][3]int
			ReadingsMap   []kv
		}{lr.CenterQuad, lr.ScannedQuads, lr.RelayedByBase, lr.Degraded, lr.Readings, list}
	}
	return mustJSON(struct {
		Type string `json:"type"`
		Data any    `json:"data"`
	}{ev.EventType(), data})
}

func eventsJSON(evs []engine.Event) string {
	parts := make([]string, len(evs))
	for i, e := range evs {
		parts[i] = eventJSON(e)
	}
	return "[" + strings.Join(parts, ",") + "]"
}

func actionJSON(a engine.Action) string {
	if a == nil {
		return "null"
	}
	return mustJSON(struct {
		Type string
		Data any
	}{fmt.Sprintf("%T", a), a})
}

func parsedJSON(p parser.ParsedCommand) string {
	var e any
	if p.Error != nil {
		e = p.Error.Error()
	}
	return "{\"Action\":" + actionJSON(p.Action) + ",\"Special\":" + mustJSON(p.Special) + ",\"Error\":" + mustJSON(e) + "}"
}

func errStr(err error) string {
	if err == nil {
		return "<nil>"
	}
	return err.Error()
}

// ---------------------------------------------------------------------------

func main() {
	if len(os.Args) > 1 {
		outDir = os.Args[1]
	}
	genPRNG()
	genMath()
	genNewGame()
	genScenarios()
	genTours()
	genScore()
	genSave()
	genParser()
	genGames()
}

// prng.txt: "seed <s>" then lines "f <bits>" / "i <n> <v>" / "r <bits>".
func genPRNG() {
	o := create("prng.txt")
	defer o.close()
	seeds := []int64{0, 1, 2, 7, 42, 999, 12345, -1, -12345, math.MaxInt64, math.MinInt64, 0x5DEECE66D, 1 << 40, 2026}
	meta := rand.New(rand.NewPCG(1, 2))
	for i := 0; i < 40; i++ {
		seeds = append(seeds, int64(meta.Uint64()))
	}
	ns := []int{1, 2, 3, 5, 7, 8, 9, 10, 16, 100, 1000, 1 << 20, 1<<31 - 1, 1000003}
	for _, s := range seeds {
		o.line("seed %d", s)
		p := engine.NewPRNG(s)
		for k := 0; k < 60; k++ {
			switch k % 3 {
			case 0:
				o.line("f %016x", math.Float64bits(p.Float64()))
			case 1:
				n := ns[(k/3)%len(ns)]
				o.line("i %d %d", n, p.Intn(n))
			default:
				o.line("r %016x", math.Float64bits(p.RandReal()))
			}
		}
		o.line("state %s", rngState(p))
	}
}

// math.txt: "<fn> <argbits...> <resultbits>" and fmt lines.
func genMath() {
	o := create("math.txt")
	defer o.close()
	r := rand.New(rand.NewPCG(3, 4))
	var xs []float64
	specials := []float64{0, math.Copysign(0, -1), 1, -1, 0.5, 2, math.Pi, math.Pi / 2, -math.Pi, 1e-300, 1e300, 5e-324, math.Inf(1), math.Inf(-1), math.NaN(), 1 << 29, 1<<29 + 0.5, 1e10, 1e17, 700, 709.7, 709.8, -745, -746, 0.80, 0.85, 0.825}
	xs = append(xs, specials...)
	for i := 0; i < 300; i++ {
		xs = append(xs, (r.Float64()-0.5)*20)
	}
	for i := 0; i < 100; i++ {
		xs = append(xs, (r.Float64()-0.5)*1e6)
	}
	for i := 0; i < 50; i++ {
		xs = append(xs, math.Ldexp(r.Float64(), r.IntN(200)-100))
	}
	b := math.Float64bits
	for _, x := range xs {
		o.line("sin %016x %016x", b(x), b(math.Sin(x)))
		o.line("cos %016x %016x", b(x), b(math.Cos(x)))
		o.line("atan %016x %016x", b(x), b(math.Atan(x)))
		o.line("exp %016x %016x", b(x), b(math.Exp(x)))
		o.line("log %016x %016x", b(x), b(math.Log(x)))
		o.line("log10 %016x %016x", b(x), b(math.Log10(x)))
		o.line("round %016x %016x", b(x), b(math.Round(x)))
		if x >= -1 && x <= 1 || math.IsNaN(x) {
			o.line("acos %016x %016x", b(x), b(math.Acos(x)))
			o.line("asin %016x %016x", b(x), b(math.Asin(x)))
		}
	}
	for i := 0; i+1 < len(xs); i++ {
		x, y := xs[i], xs[len(xs)-1-i]
		o.line("atan2 %016x %016x %016x", b(x), b(y), b(math.Atan2(x, y)))
		o.line("hypot %016x %016x %016x", b(x), b(y), b(math.Hypot(x, y)))
		o.line("mod %016x %016x %016x", b(x), b(y), b(math.Mod(x, y)))
		o.line("max %016x %016x %016x", b(x), b(y), b(math.Max(x, y)))
		o.line("min %016x %016x %016x", b(x), b(y), b(math.Min(x, y)))
	}
	for i := 0; i < 400; i++ {
		x := 0.8 + 0.05*r.Float64()
		y := r.Float64() * 12
		if i%5 == 0 {
			x = (r.Float64() - 0.3) * 10
			y = math.Round((r.Float64()-0.5)*20) / 2
		}
		o.line("pow %016x %016x %016x", b(x), b(y), b(math.Pow(x, y)))
	}
	for _, xy := range [][2]float64{{0, 0}, {0, -1}, {-2, 3}, {-2, 0.5}, {2, math.Inf(1)}, {0.5, math.Inf(-1)}, {math.Inf(-1), 3}, {math.NaN(), 0}, {1, math.NaN()}, {-1, math.Inf(1)}, {10, 308}, {10, -330}} {
		o.line("pow %016x %016x %016x", b(xy[0]), b(xy[1]), b(math.Pow(xy[0], xy[1])))
	}
	// fmt / strconv / json float formatting
	fl := append([]float64{}, specials...)
	fl = append(fl, 100000, 1e6, 123456789, 1e20, 1e21, 1e-5, 1e-6, 1e-7, 0.0001, 1.5, 2.675, 0.125, 1234567, 3000.5, 4999.999999, 0.1, 0.2+0.1, 1.0/3, 2.0/3, 123.456, -0.0001234, 9.999999e20, 5e-7)
	for i := 0; i < 150; i++ {
		fl = append(fl, math.Ldexp(r.Float64(), r.IntN(160)-80))
	}
	for i := 0; i < 100; i++ {
		fl = append(fl, math.Round(r.Float64()*1e6)/math.Pow(10, float64(r.IntN(8))))
	}
	for _, x := range fl {
		js, err := json.Marshal(x)
		jstr := string(js)
		if err != nil {
			jstr = "ERR " + err.Error()
		}
		o.line("fmt %016x|%v|%g|%e|%.1f|%.2f|%.0f|%5.1f|%8.3g|%.3e|%s|%s|%s", b(x), x, x, x, x, x, x, x, x, x,
			strconv.FormatFloat(x, 'f', -1, 64), strconv.FormatFloat(x, 'g', 4, 64), jstr)
	}
	strs := []string{"", "abc", "a\"b", "back\\slash", "tab\there", "nl\n", "\x01\x7f", "héllo", "日本", " ", "<&>", "emoji 🚀", "\xff\xfe", "nav q 3 4", "'single'", "­", "​"}
	for _, s := range strs {
		js, _ := json.Marshal(s)
		o.line("str %s|%s|%s", hex.EncodeToString([]byte(s)), strconv.Quote(s), string(js))
	}
	nums := []string{"1", "-1", "+5", "0", "08", "1.5", ".5", "5.", "1e3", "1E-2", "-.5e+1", "inf", "+Inf", "-infinity", "NaN", "nan", "-nan", "0x1p3", "0x1.8p1", "0X10", "1_000", "0x_1p0", "abc", "", "-", "+", "1e", "1e400", "-1e400", "1e-400", "9223372036854775807", "9223372036854775808", "-9223372036854775808", "4294967297", "3.14159", "1.57", "  1", "1 ", "0.0", "-0", "00.5", "1..2", "e5", "0x", "0b101", "0o17", "1__0", "٣"}
	for _, s := range nums {
		f, ferr := strconv.ParseFloat(s, 64)
		n, nerr := strconv.Atoi(s)
		o.line("num %s|%016x|%s|%d|%s", hex.EncodeToString([]byte(s)), b(f), errStr(ferr), n, errStr(nerr))
	}
}

func allProfiles() []engine.DifficultyProfile {
	return []engine.DifficultyProfile{engine.ProfileCasual, engine.ProfileNormal, engine.ProfileHardcore, engine.ProfileNightmare, engine.ProfileExpert, engine.ProfileEmeritus, engine.ProfileCustom, "bogus"}
}

type gameCfg struct {
	Seed   int64
	Skill  engine.SkillLevel
	Length engine.GameLength
	Rules  engine.GameRules
}

func cfgList() []gameCfg {
	var list []gameCfg
	r := rand.New(rand.NewPCG(5, 6))
	seeds := []int64{1, 2, 3, 42, 12345, 999, 2026, 77, 31337, -5}
	for i := 0; i < 26; i++ {
		seeds = append(seeds, int64(r.IntN(1<<40)))
	}
	surv := []engine.SurveillanceMode{engine.SurveillanceFull, engine.SurveillanceClassic, engine.SurveillanceLocal, engine.SurveillanceBlackout, "", "weird"}
	for i, s := range seeds {
		p := allProfiles()[i%len(allProfiles())]
		rules := engine.DefaultRulesForProfile(p)
		if i%3 == 1 {
			rules.Adversaries = !rules.Adversaries
			rules.SpatialAnomalies = !rules.SpatialAnomalies
		}
		if i%4 == 2 {
			rules.KlingonCloak = !rules.KlingonCloak
			rules.Profile = engine.ProfileCustom
		}
		if i%5 == 3 {
			rules.Surveillance = surv[i%len(surv)]
			rules.RepairMultiplier = []float64{0, 0.5, 1.25, 3}[i%4]
			rules.TimeMargin = []float64{0.7, 1.1, 1.0 / 3, 2}[i%4]
		}
		list = append(list, gameCfg{s, engine.SkillLevel(1 + i%5), engine.GameLength(1 + i%3), rules})
	}
	// every profile x a few seeds x adversary toggles
	for _, p := range allProfiles() {
		for k := 0; k < 3; k++ {
			rules := engine.DefaultRulesForProfile(p)
			rules.Adversaries = k != 0
			rules.SpatialAnomalies = k == 2 || rules.SpatialAnomalies
			list = append(list, gameCfg{int64(100 + k*17 + len(p)), engine.SkillGood, engine.LengthMedium, rules})
		}
	}
	return list
}

// newgame.txt: "cfg <json>" then full state of NewGameWithOptions, then for
// every quadrant (row-major) PopulateQuadrant at a varying sector: "q r c" + hash,
// with full dumps for some.
func genNewGame() {
	o := create("newgame.txt")
	defer o.close()
	for i, c := range cfgList() {
		o.line("cfg %s", mustJSON(c))
		g := engine.NewGameWithOptions(c.Seed, c.Skill, c.Length, c.Rules)
		writeFull(o, g)
		for qr := 1; qr <= 8; qr++ {
			for qc := 1; qc <= 8; qc++ {
				sec := engine.Coord{(qr+qc)%8 + 1, (qr*3+qc)%8 + 1}
				if (qr+qc+i)%11 == 0 {
					sec = engine.Coord{0, 9} // invalid -> defaults to 4,4
				}
				g.PopulateQuadrant(engine.Coord{qr, qc}, sec)
				o.line("q %d %d %d %d %s", qr, qc, sec[0], sec[1], stateHash(g))
				if (qr*8+qc+i)%23 == 0 {
					writeFull(o, g)
				}
			}
		}
		// out-of-range quadrant is a no-op
		g.PopulateQuadrant(engine.Coord{0, 3}, engine.Coord{4, 4})
		o.line("q 0 3 4 4 %s", stateHash(g))
	}
	// NewGame / NewGameWithSeed
	for _, s := range []int64{1, 42, 12345} {
		o.line("newgame %d", s)
		writeFull(o, engine.NewGame(s, engine.SkillFair, engine.LengthLong))
		o.line("newgamewithseed %d", s)
		writeFull(o, engine.NewGameWithSeed(s))
	}
}

// scenarios.txt: "build <id> <seed>" + full state; "eval" lines after
// state mutations: "<label> done won reason" + hash.
func genScenarios() {
	o := create("scenarios.txt")
	defer o.close()
	for _, s := range engine.ListScenarios() {
		o.line("meta %s", mustJSON(s))
	}
	for _, q := range []string{"kobayashi-maru", "KM", " Mutara ", "nebula", "siege", "STARBASE", "", "none", "starbase-siege"} {
		s, ok := engine.GetScenario(engine.ScenarioID(q))
		id := ""
		if ok {
			id = string(s.ID)
		}
		o.line("get %q %v %s", q, ok, id)
	}
	seeds := []int64{1, 7, 42, 12345, 2026, 99999, -3}
	for _, s := range engine.ListScenarios() {
		for _, seed := range seeds {
			g := s.Build(seed)
			o.line("build %s %d", s.ID, seed)
			writeFull(o, g)
			eval := func(label string) {
				done, won, reason := s.Evaluate(g)
				sb := s.ComputeScore(g, won)
				o.line("eval %s %v %v %d %s %s", label, done, won, reason, mustJSON(sb), stateHash(g))
			}
			eval("initial")
			// kill some klingons
			for len(g.CurrentQuad.Klingons) > 0 {
				k := g.CurrentQuad.Klingons[0]
				g.CurrentQuad.Grid[k.Sector[0]][k.Sector[1]] = engine.EntityEmpty
				g.CurrentQuad.Klingons = g.CurrentQuad.Klingons[1:]
				g.Metrics.KlingonsKilled++
				g.Stardate += 1.25
				eval("kill")
				if g.Metrics.KlingonsKilled > 12 {
					break
				}
			}
			g.Enterprise.Quad = engine.Coord{3, 3}
			eval("moved")
			g.Enterprise.Quad = engine.Coord{4, 4}
			g.TimeRemaining = 0
			eval("timeout")
			g.TimeRemaining = 5
			g.Enterprise.Energy = 0
			eval("energy")
			g.Enterprise.Energy = 100
			g.Metrics.StarbasesDestroyed = 1
			eval("starbase")
			for _, won := range []bool{true, false} {
				o.line("score %v %s", won, mustJSON(s.ComputeScore(g, won)))
			}
		}
	}
}

func refitIDs() []engine.RefitID {
	ids := []engine.RefitID{"bogus"}
	for _, d := range engine.RefitCatalog {
		ids = append(ids, d.ID)
	}
	return ids
}

// tours.txt: tour lifecycle traces.
func genTours() {
	o := create("tours.txt")
	defer o.close()
	o.line("catalog %s", mustJSON(engine.RefitCatalog))
	for _, id := range refitIDs() {
		d := engine.GetRefitDefinition(id)
		o.line("def %s %s", id, mustJSON(d))
	}
	r := rand.New(rand.NewPCG(7, 8))
	for ti, seed := range []int64{1, 42, 1000, 12345, 777, 5, 31337, 9} {
		t := engine.NewTour(seed)
		o.line("tour %d %s", seed, mustJSON(t))
		g, err := t.StartCurrentSector()
		o.line("start %s", errStr(err))
		writeFull(o, g)
		for step := 0; step < 12 && t.Active && !t.Completed; step++ {
			cleared, failed, bounty := t.EvaluateSector()
			o.line("evaluate %v %v %d", cleared, failed, bounty)
			// mutate the sector state
			switch r.IntN(5) {
			case 0:
				g.KlingonsRemaining = 0
			case 1:
				g.RemainingKlingons = 0
				g.Enterprise.Devices[engine.DeviceShields] = 1.5
			case 2:
				g.CurrentQuad.Enemies = nil
				g.CurrentQuad.Klingons = nil
				g.DaysRemaining = r.Float64() * 40
			case 3:
				g.Score = r.IntN(900)
				g.Enterprise.Damage[engine.DeviceWarp] = 2
			case 4:
				if ti%3 == 2 && step > 3 {
					g.GameOver = true
					g.GameOverReason = engine.GameOverReason(1 + r.IntN(5))
				}
			}
			cleared, failed, bounty = t.EvaluateSector()
			o.line("evaluate %v %v %d", cleared, failed, bounty)
			if cleared {
				t.AdvanceToDrydock(bounty)
				for k := 0; k < 6; k++ {
					id := refitIDs()[r.IntN(len(refitIDs()))]
					err := engine.PurchaseRefit(t, id)
					o.line("purchase %s %s", id, errStr(err))
				}
				ng, err := t.DisembarkToNextSector()
				o.line("disembark %s %v", errStr(err), ng != nil)
				if ng != nil {
					if len(ng.CurrentQuad.Enemies) == 0 {
						ng.PopulateQuadrant(ng.Enterprise.Quad, ng.Enterprise.Sector)
					}
					g = ng
					writeFull(o, g)
				}
			}
			_, err := t.DisembarkToNextSector()
			o.line("disembark-again %s", errStr(err))
			o.line("tourstate %s", fnv64([]byte(mustJSON(t))))
		}
		rank, medals := engine.CalculateTourCommission(t)
		o.line("commission %s %s", rank, mustJSON(medals))
		o.line("final %s", mustJSON(t))
		lb := engine.NewLeaderboard()
		rec := lb.RecordTour(t, []string{"", "Excelsior"}[ti%2])
		rec.Date = "FIXED"
		lb.TourRecords[0].Date = "FIXED"
		o.line("record %s %s", mustJSON(rec), mustJSON(lb))
	}
	// explicit sector indices, invalid index, nil-ish refits
	t := engine.NewTour(55)
	for _, idx := range []int{3, 2, 1, 0, 4, -1} {
		g, err := t.StartCurrentSector(idx)
		o.line("startidx %d %s", idx, errStr(err))
		if g != nil {
			writeFull(o, g)
		}
	}
	t.InstalledRefits = nil
	t.RequisitionPoints = 5000
	for _, id := range refitIDs() {
		o.line("purchase %s %s %d", id, errStr(engine.PurchaseRefit(t, id)), t.RequisitionPoints)
	}
	o.line("purchase-nil %s", errStr(engine.PurchaseRefit(nil, "x")))
	for _, id := range refitIDs() {
		for i := 0; i < 3; i++ {
			engine.PurchaseRefit(t, id)
		}
	}
	o.line("maxed %s", mustJSON(t))
	g, _ := t.StartCurrentSector(2)
	writeFull(o, g)
	o.line("torpdmg %v %v", engine.CalculateTorpedoDamage(g, 500), engine.CalculateShieldDamageAbsorption(g, 300))
	rank, medals := engine.CalculateTourCommission(nil)
	o.line("commission-nil %s %s", rank, mustJSON(medals))
	for sc := 0; sc <= 5; sc++ {
		t.SectorsCompleted = sc
		rank, medals := engine.CalculateTourCommission(t)
		o.line("commission %d %s %s", sc, rank, mustJSON(medals))
	}
}

// score.txt: ComputeScore/RankForScore and leaderboard logic.
func genScore() {
	o := create("score.txt")
	defer o.close()
	for _, s := range []int{-1000, -1, 0, 1, 99, 100, 199, 200, 349, 350, 499, 500, 749, 750, 999, 1000, 5000} {
		t, b := engine.RankForScore(s)
		o.line("rank %d %s %s", s, t, b)
	}
	r := rand.New(rand.NewPCG(9, 10))
	for i := 0; i < 80; i++ {
		g := engine.NewGame(int64(i), engine.SkillLevel(r.IntN(6)), engine.LengthMedium)
		g.Stardate = g.InitialStardate + (r.Float64()-0.2)*20
		m := &g.Metrics
		m.KlingonsKilled = r.IntN(20)
		m.CommandersKilled = r.IntN(4)
		m.SuperCommandersKilled = r.IntN(2)
		m.RomulansKilled = r.IntN(3)
		m.RomulansSurrendered = r.IntN(3)
		m.StarbasesDestroyed = r.IntN(2)
		m.StarsDestroyed = r.IntN(3)
		m.PlanetsDestroyed = r.IntN(2)
		m.Casualties = r.IntN(300)
		m.HelpCalls = r.IntN(3)
		m.StarshipsLost = r.IntN(2)
		won := r.IntN(2) == 0
		o.line("score %s %v %s", mustJSON(g.Metrics), won, mustJSON(engine.ComputeScore(g, won)))
		o.line("scoregame %v %v %d %s", g.Stardate, g.InitialStardate, g.Skill, mustJSON(engine.ComputeScore(g, !won)))
	}
	o.line("score-nil %s", mustJSON(engine.ComputeScore(nil, true)))

	// leaderboard add / qualifies sequences
	lb := engine.DefaultLeaderboard()
	o.line("default %s", mustJSON(lb))
	o.line("new %s", mustJSON(engine.NewLeaderboard()))
	for i := 0; i < 40; i++ {
		sc := r.IntN(1500) - 100
		q := lb.Qualifies(sc)
		e := engine.ScoreEntry{CaptainName: fmt.Sprintf("Cap%d", i), Score: sc, Rank: "[X]", Stardate: float64(i) * 1.5}
		if i%3 == 0 {
			e.Skill = "Good"
			e.GameWon = true
		}
		lb.Add(e)
		o.line("add %d %v %s", sc, q, fnv64([]byte(mustJSON(lb))))
	}
	o.line("after %s", mustJSON(lb))
	small := engine.NewLeaderboard()
	o.line("qualifies-empty %v", small.Qualifies(-5))
	for _, id := range []engine.ScenarioID{engine.ScenarioKobayashiMaru, "km", engine.ScenarioMutaraNebula, "siege", "unknown", ""} {
		o.line("scenario-default %q %s", id, mustJSON(engine.DefaultScenarioLeaderboard(id)))
	}
	// unmarshal edge cases for leaderboards (corruption recovery decisions)
	inputs := []string{
		`{"entries":[{"captain_name":"A","score":5,"rank":"r","date":"2020-01-02T03:04:05Z"},{"captain_name":"B","score":50,"rank":"r","date":"2021-06-07T08:09:10.5+02:00"}]}`,
		`{"entries":null}`,
		`{}`,
		`{"Entries":[{"CAPTAIN_NAME":"x","Score":1,"date":"0001-01-01T00:00:00Z"}],"tour_records":[{"date":"d","medals":null}]}`,
		`{"entries":[{"captain_name":"A","score":"5"}]}`,
		`{"entries":[{"date":"bad"}]}`,
		`{"entries":[{"date":12}]}`,
		`{"entries":[{"date":"2020-13-01T00:00:00Z"},{"date":"x"}]}`,
		`{"entries":[{"date":"2020-01-01"}]}`,
		`{"entries":[{"date":"2020-01-01T00:00:00"}]}`,
		`{"entries":[{"date":"2020-01-01T25:00:00Z"}]}`,
		`{"entries":[{"date":"2021-02-29T00:00:00Z"}]}`,
		`{"entries":[{"date":"2020-02-29T23:59:59.999999999-11:45","score":1}]}`,
		`{"entries":[{"date":"2020-01-01t00:00:00z"}]}`,
		`{"entries":[{"date":""}]}`,
		`{"entries":[{"date":"20x0-01-01T00:00:00Z"}]}`,
		`{"entries":[{"date":"2020-01-01T00:00:00.Z"}]}`,
		`{"entries":[{"date":"2020-01-01T00:00:60Z"}]}`,
		`{"entries":[{"date":"2020-01-01 00:00:00Z"}]}`,
		`{"entries":[{"date":"2020-01-01T00:00:00Zjunk"}]}`,
		`{"entries":[{"date":"2020-01-01T00:00:00+0500"}]}`,
		`{"entries":[{"date":"2020-1-01T00:00:00Z"}]}`,
		`{"entries":[{"date":"2020-01-01T7:00:00Z"}]}`,
		`{"entries":[{"date":"0000-01-01T00:00:00Z"}, {"date":"9999-12-31T23:59:59.5Z"}]}`,
		`{"entries":[{"date":true}],"tour_records":[{"medals":["a"],"score":"x"}]}`,
		`[1,2]`,
		`not json`,
		`{"entries":[` + strings.Repeat(`{"captain_name":"Z","score":3,"date":"2022-01-01T00:00:00Z"},`, 12) + `{"captain_name":"Y","score":9,"date":"2022-01-01T00:00:00-07:30"}]}`,
	}
	for _, in := range inputs {
		var l engine.Leaderboard
		err := json.Unmarshal([]byte(in), &l)
		o.line("unmarshal %s", hex.EncodeToString([]byte(in)))
		o.line("result %s %s", errStr(err), mustJSON(l))
		b, _ := json.MarshalIndent(l, "", "  ")
		o.line("indent %s", hex.EncodeToString(b))
	}
}

// save.txt: save/load round trips.
func genSave() {
	o := create("save.txt")
	defer o.close()
	for _, n := range []string{"", "  ", "a", "abcdefghi", "abcdefghij", "1abc", "_x", "dir/name.TRK", "name.trk", ".TRK", "Z.tar.gz", "ok.json", "/abs/path/B12", "é"} {
		o.line("validate %q %s", n, errStr(engine.ValidateSaveFilename(n)))
	}
	var states []*engine.GameState
	var srcs []string
	for i, c := range cfgList() {
		if i%4 != 0 {
			continue
		}
		g := engine.NewGameWithOptions(c.Seed, c.Skill, c.Length, c.Rules)
		g.PopulateQuadrant(g.Enterprise.Quad, g.Enterprise.Sector)
		extra := false
		if i%8 == 0 {
			g.ActiveRefits = map[engine.RefitID]int{"damage_nanites": 2, "deflector_grid": 1}
			g.GameOver = true
			g.GameOverReason = engine.GameOverEnergy
			extra = true
		}
		states = append(states, g)
		srcs = append(srcs, fmt.Sprintf("cfg %v %s", extra, mustJSON(c)))
	}
	for _, s := range engine.ListScenarios() {
		states = append(states, s.Build(11))
		srcs = append(srcs, "scenario "+string(s.ID))
	}
	tour := engine.NewTour(4242)
	tg, _ := tour.StartCurrentSector(3)
	states = append(states, tg)
	srcs = append(srcs, "tour")
	for i, g := range states {
		o.line("src %s", srcs[i])
		legacy, _ := json.MarshalIndent(g, "", "  ")
		env, _ := json.MarshalIndent(engine.SaveEnvelope{Version: 2, GameState: g}, "", "  ")
		o.line("state %d", i)
		o.line("compact %s", mustJSON(g))
		o.line("klingons %s", mustJSON(g.CurrentQuad.Klingons))
		o.line("legacyhash %s %d", fnv64(legacy), len(legacy))
		if i%3 == 0 {
			o.line("legacy %s", hex.EncodeToString(legacy))
		}
		o.line("envhash %s %d", fnv64(env), len(env))
		// load both forms through temp files using the real LoadGame
		for _, data := range [][]byte{legacy, env} {
			path := filepath.Join(os.TempDir(), "sstparity.TRK")
			os.WriteFile(path, data, 0o644)
			lg, err := engine.LoadGame(path)
			o.line("load %s %s", errStr(err), stateHash(lg))
			m, err := engine.InspectSave(path)
			if m != nil {
				o.line("inspect %s %d %v %v %d %d", errStr(err), m.Skill, m.Stardate, m.TimeRemaining, m.Condition, m.KlingonsLeft)
			}
			g2, t2, err := engine.LoadTourGame(path)
			o.line("loadtour %s %s %v", errStr(err), stateHash(g2), t2 != nil)
		}
	}
	// tour envelopes
	tenv, _ := json.MarshalIndent(engine.SaveEnvelope{Version: 2, GameState: tg, TourState: tour}, "", "  ")
	o.line("tourenv %s", hex.EncodeToString(tenv))
	tour.InstalledRefits = map[engine.RefitID]int{"dilithium_core": 3, "torpedo_bays": 1}
	tenv2, _ := json.MarshalIndent(engine.SaveEnvelope{Version: 2, GameState: tg, TourState: tour}, "", "  ")
	o.line("tourenv2 %s", hex.EncodeToString(tenv2))
	tonly, _ := json.MarshalIndent(engine.SaveEnvelope{Version: 2, TourState: &engine.TourState{ID: "t", Seed: 3}}, "", "  ")
	o.line("touronly %s", hex.EncodeToString(tonly))

	// decode edge cases through LoadGame/LoadTourGame/InspectSave
	cases := []string{
		string(tenv), string(tenv2), string(tonly),
		`{"Stardate": 2500.5, "Enterprise": {"Energy": 1200, "Quad": [2,3], "sector": [4,5]}}`,
		`{"stardate": 2600, "enterprise": {"energy": 99}, "rules": {"profile": "expert", "klingon_cloak": true}, "currentquad": {"enemies": [{"id": 3, "faction": 0, "sector": [1,1], "energy": 50}, null]}}`,
		`{"version": 2}`,
		`{"version": 2, "game_state": null}`,
		`{"version": 2, "game_state": {"Stardate": 3000, "RNG": null}}`,
		`{"version": 2, "game_state": {"Stardate": 3000}, "tour_state": {"installed_refits": {"dilithium_core": 2}, "current_game_state": {"Stardate": 1}}}`,
		`{"version": "2", "game_state": {"Stardate": 3000}}`,
		`{"Stardate": "bad"}`,
		`{"Skill": 1.5, "Stardate": 2000}`,
		`{"GalaxyChart": [[1,2,3]], "Stardate": 2001}`,
		`{"GalaxyChart": [[1,2,3,4,5,6,7,8,9,10,11]], "Stardate": 2001}`,
		`{"Enterprise": {"Devices": [1,2,3,4,5,6,7,8,9]}, "Stardate": 1e3}`,
		`{"Enterprise": {"Torpedoes": 1e2}}`,
		`{"Enterprise": {"Energy": 1e400}}`,
		`{"active_refits": {"x": 1, "y": "2"}, "Stardate": 5}`,
		`{"CurrentQuad": {"starbase": [3,3], "stars": [[1,1],[2,2]], "grid": [[0,1]]}, "Stardate": 7}`,
		`{"scenario": "kobayashi-maru", "Stardate": 2222, "Unknown": {"deep": [1,2,{"x":null}]}}`,
		`{"stardate": 10, "STARDATE": 20}`,
		`{"Stardate": 10, "stardate": 20}`,
		`{"metrics": {"klingons_killed": 3}, "game_won": true, "options": {"difficulty": "hardcore"}, "Stardate": 1}`,
		`[]`,
		`null`,
		`{"Stardate": 1,}`,
		``,
		`{"Stardate": 0, "Enterprise": {"Energy": 0}}`,
		"{\"Stardate\": 2000, \"scenario\": \"a\\u0041\\ud83d\\ude80\\ud800x\\n\"}",
		`{"Stardate": 2000, "RNG": 5}`,
		`{"Stardate": 2000, "rules": "x"}`,
		`{"Stardate": 2000, "CurrentQuad": {"enemies": {"a":1}}}`,
		`{"Stardate": 2000, "Skill": -7, "Length": 99, "quadrant_env": [[0,0],[0,2,1]]}`,
		`{"Stardate": 2000, "game_over_reason": 3, "game_over": true}`,
		`  {"Stardate": 2000}  `,
		`{"Stardate": 2000} x`,
		`{"Stardate": tru}`,
		`{"Stardate" 1}`,
		`{"a":"\x"}`,
	}
	for i, c := range cases {
		path := filepath.Join(os.TempDir(), fmt.Sprintf("sstcase%d.TRK", i))
		os.WriteFile(path, []byte(c), 0o644)
		o.line("case %s", hex.EncodeToString([]byte(c)))
		lg, err := engine.LoadGame(path)
		o.line("load %s", errStr(err))
		if lg != nil {
			writeFull(o, lg)
		}
		m, err := engine.InspectSave(path)
		if err != nil {
			o.line("inspect %s", strings.ReplaceAll(errStr(err), path, "PATH"))
		} else {
			o.line("inspect <nil> %d %v %v %d %d", m.Skill, m.Stardate, m.TimeRemaining, m.Condition, m.KlingonsLeft)
		}
		g2, t2, err := engine.LoadTourGame(path)
		o.line("loadtour %s", strings.ReplaceAll(errStr(err), path, "PATH"))
		if g2 != nil {
			writeFull(o, g2)
		}
		if t2 != nil {
			o.line("T %s", mustJSON(t2))
			o.line("alias %v", t2.CurrentGameState == g2 && g2 != nil)
		}
		// plain unmarshal into a fresh GameState (wasm Session.Load)
		var sg engine.GameState
		err = json.Unmarshal([]byte(c), &sg)
		o.line("unmarshal %s", errStr(err))
		writeFull(o, &sg)
	}
	missing := filepath.Join(os.TempDir(), "sst-does-not-exist.TRK")
	_, err := engine.LoadGame(missing)
	o.line("missing %s", strings.ReplaceAll(errStr(err), missing, "PATH"))
	_, _, err = engine.LoadTourGame(missing)
	o.line("missingtour %s", strings.ReplaceAll(errStr(err), missing, "PATH"))
	_, err = engine.InspectSave(missing)
	o.line("missinginspect %s", strings.ReplaceAll(errStr(err), missing, "PATH"))
}

// parser.txt: "in <hex>" then "P <parsed json>".
func genParser() {
	o := create("parser.txt")
	defer o.close()
	inputs := []string{"", "   ", "nav", "nav 1", "nav 2 5", "nav 9 9", "nav 0 3", "nav 2.0 5", "nav 1.57 0.5", "nav 3 -1", "nav q 3 4", "nav Q 3 4 2.5", "nav quad 8 8 0", "nav quadrant 1 1 -1", "nav q 9 1", "nav q a b", "nav s 2 3", "nav sec 9 9", "nav sector x 1", "nav c 1.5 2", "nav course 0 0", "nav c x y", "nav x 1 2", "nav 1 2 3 4 5", "move 4 4", "WARP 0.5 1", "nav nan 1", "nav 1 nan", "nav inf 1", "nav 1 inf", "nav 1e400 1", "nav 4294967297 1", "nav 1_0 2",
		"tor", "tor 4 5", "tor 0 5", "tor 3.14", "tor x", "tor c 3", "tor c 0.5", "tor c 9", "tor course 9.5", "tor c x", "tor a b", "torpedo 1 1", "tor 1 2 3", "tor nan", "tor c nan", "tor 1.5 2",
		"pha", "pha 300", "pha -5", "pha 0", "pha x", "pha 200 3,4", "pha 200 3, 4", "pha 200 9,9", "pha 200 3", "pha 100 4 5", "pha 100 4 x", "pha x 4 5", "pha 10 1 2 3", "phasers 1e3", "pha nan", "pha inf",
		"she", "she 500", "she -200", "she x", "she 1 2", "shields 0", "def 1e3", "she nan",
		"doc", "DOCK", "opts", "options", "settings", "scenarios", "scen", "tour", "orders", "TOUR", "theme", "theme Amber", "theme a b", "help", "?", "commands", "help nav", "help TORPEDO", "help reticle", "help pha", "help shields", "help dock", "help map", "help freeze", "help settings", "help xyz", "help Foo bar", "quit", "exit", "q", "Q", "lrscan", "srscan", "xyzzy", "\"quoted\"", "日本", "nav\t2\t5", "  she   100  ", " she 1", "nav 2 5　"}
	r := rand.New(rand.NewPCG(11, 12))
	words := []string{"nav", "tor", "pha", "she", "doc", "q", "s", "c", "1", "4", "8", "9", "0", "-1", "2.5", "x", "3,4", "help", "theme", "1e2", "0.5", ".", ","}
	for i := 0; i < 300; i++ {
		n := 1 + r.IntN(5)
		var parts []string
		for k := 0; k < n; k++ {
			parts = append(parts, words[r.IntN(len(words))])
		}
		inputs = append(inputs, strings.Join(parts, " "))
	}
	for _, in := range inputs {
		o.line("in %s", hex.EncodeToString([]byte(in)))
		o.line("P %s", parsedJSON(parser.ParseCommand(in)))
	}
}
