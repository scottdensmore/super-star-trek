package main

import (
	"encoding/hex"
	"encoding/json"
	"fmt"
	"math/rand/v2"
	"os"
	"strings"

	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/parser"
)

// fullDetail writes P/E lines for every step of every game (debugging aid:
// SST_PARITY_FULL=1 go run ./tools/paritygen/engine /tmp/full).
var fullDetail = os.Getenv("SST_PARITY_FULL") == "1"

type sim struct {
	mode string // "tui" or "wasm"
	g    *engine.GameState
	tour *engine.TourState
	lb   *engine.Leaderboard
	seed int64
	over bool
	r    *rand.Rand
	o    *out
	full bool
	// hazard placed by extra(); the next command often heads for it
	pending *engine.Coord
}

func hasGameOver(evs []engine.Event) (engine.EventGameOver, bool) {
	for _, ev := range evs {
		if g, ok := ev.(engine.EventGameOver); ok {
			return g, true
		}
	}
	return engine.EventGameOver{}, false
}

// executeKlingonTurnIfActive mirrors pkg/tui/update.go.
func executeKlingonTurnIfActive(g *engine.GameState, action engine.Action, events *[]engine.Event) {
	if _, ok := hasGameOver(*events); ok {
		return
	}
	shouldAttack := false
	switch act := action.(type) {
	case engine.ActionFireTorpedo, engine.ActionTorpedoDirect, engine.ActionFirePhasers, engine.ActionPhaserDirect, engine.ActionShields:
		shouldAttack = true
	case engine.ActionMove:
		isInterQuad := false
		for _, ev := range *events {
			if moveEv, ok := ev.(engine.EventShipMoved); ok && moveEv.FromQuad != moveEv.ToQuad {
				isInterQuad = true
				break
			}
		}
		if !isInterQuad && act.DestQuad == (engine.Coord{}) {
			shouldAttack = true
		}
	}
	if shouldAttack && g.Enterprise.Condition != engine.ConditionDocked {
		if len(g.CurrentQuad.Enemies) > 0 || g.Rules.Adversaries || len(g.CurrentQuad.PlasmaTorpedoes) > 0 || len(g.CurrentQuad.WebSegments) > 0 {
			*events = append(*events, engine.UnifiedAdversaryTurn(g)...)
		} else if len(g.CurrentQuad.Klingons) > 0 {
			*events = append(*events, engine.KlingonTurn(g)...)
		}
	}
}

// sessionCounterAttack mirrors cmd/wasm/session.go Execute.
func sessionCounterAttack(g *engine.GameState, action engine.Action, events *[]engine.Event) {
	if _, ok := hasGameOver(*events); ok {
		return
	}
	if len(g.CurrentQuad.Klingons) > 0 && g.Enterprise.Condition != engine.ConditionDocked {
		shouldAttack := false
		switch action.(type) {
		case engine.ActionFireTorpedo, engine.ActionTorpedoDirect, engine.ActionFirePhasers, engine.ActionShields:
			shouldAttack = true
		case engine.ActionMove:
			isInterQuad := false
			for _, ev := range *events {
				if moveEv, ok := ev.(engine.EventShipMoved); ok && moveEv.FromQuad != moveEv.ToQuad {
					isInterQuad = true
					break
				}
			}
			if !isInterQuad {
				shouldAttack = true
			}
		}
		if shouldAttack {
			*events = append(*events, engine.KlingonTurn(g)...)
		}
	}
}

func (s *sim) detail(tag, payload string) {
	if s.full || fullDetail {
		s.o.line("%s %s", tag, payload)
	} else {
		s.o.line("%s# %s", tag, fnv64([]byte(payload)))
	}
}

// evaluateTour mirrors Model.evaluateTourSector plus a scripted drydock visit.
func (s *sim) evaluateTour() {
	t := s.tour
	if t == nil || t.InDrydock || !t.Active {
		return
	}
	t.CurrentGameState = s.g
	cleared, failed, bounty := t.EvaluateSector()
	if cleared || failed {
		s.o.line("T %v %v %d", cleared, failed, bounty)
	}
	if cleared {
		t.AdvanceToDrydock(bounty)
		for k := 0; k < 4; k++ {
			id := refitIDs()[s.r.IntN(len(refitIDs()))]
			s.o.line("B %s %s", id, errStr(engine.PurchaseRefit(t, id)))
		}
		ng, err := t.DisembarkToNextSector()
		s.o.line("D %s %v", errStr(err), ng != nil)
		if err == nil && ng != nil {
			if len(ng.CurrentQuad.Enemies) == 0 {
				ng.PopulateQuadrant(ng.Enterprise.Quad, ng.Enterprise.Sector)
			}
			s.g = ng
			writeFull(s.o, s.g)
		} else if t.Completed {
			rec := s.lb.RecordTour(t, "Enterprise")
			rec.Date = ""
			s.lb.TourRecords[len(s.lb.TourRecords)-1].Date = ""
			s.o.line("C %s", mustJSON(s.lb))
			s.over = true
		}
		s.o.line("TS %s", mustJSON(t))
		return
	}
	if failed {
		s.lb.RecordTour(t, "Enterprise")
		s.lb.TourRecords[len(s.lb.TourRecords)-1].Date = ""
		s.o.line("F %s", mustJSON(s.lb))
		s.o.line("TS %s", mustJSON(t))
		s.over = true
	}
}

// handleGameOver mirrors Model.handleGameOver (state side effects only).
func (s *sim) handleGameOver(ev engine.EventGameOver) {
	s.g.GameOver = true
	s.g.GameOverReason = ev.Reason
	if ev.Reason == engine.GameOverWon {
		s.g.GameWon = true
	}
	sb := engine.ComputeScore(s.g, s.g.GameWon)
	s.o.line("G %d %s", ev.Reason, mustJSON(sb))
	if s.tour != nil && !s.tour.InDrydock && s.tour.Active {
		s.evaluateTour()
	}
	if s.tour == nil {
		s.over = true
	} else if !s.over && s.tour.InDrydock == false && !s.tour.Active {
		s.over = true
	}
}

func (s *sim) stepTUI(input string) {
	s.o.line("> %s", hex.EncodeToString([]byte(input)))
	trimmed := strings.ToLower(strings.TrimSpace(input))
	if trimmed == "lrscan" {
		events, err := s.g.Dispatch(engine.ActionLRScan{})
		if err != nil {
			s.detail("X", err.Error())
		} else {
			s.detail("E", eventsJSON(events))
		}
	} else {
		parsed := parser.ParseCommand(input)
		s.detail("P", parsedJSON(parsed))
		if parsed.Error == nil && parsed.Special == "" && parsed.Action != nil {
			events, err := s.g.Dispatch(parsed.Action)
			if err != nil {
				s.detail("X", err.Error())
			} else {
				executeKlingonTurnIfActive(s.g, parsed.Action, &events)
				s.detail("E", eventsJSON(events))
				if ev, ok := hasGameOver(events); ok {
					s.handleGameOver(ev)
				}
			}
		}
	}
	if !s.over {
		s.evaluateTour()
	}
	s.o.line("H %s", stateHash(s.g))
}

func (s *sim) stepWASM(input string) {
	s.o.line("> %s", hex.EncodeToString([]byte(input)))
	defer func() { s.o.line("H %s", stateHash(s.g)) }()
	trimmed := strings.TrimSpace(input)
	if trimmed == "" {
		return
	}
	tokens := strings.Fields(trimmed)
	cmd := strings.ToLower(tokens[0])
	switch cmd {
	case "help", "?", "srs", "srscan", "status", "lrs", "lrscan", "chart", "dam", "damages", "quit", "exit", "q":
		s.o.line("W bypass")
		return
	case "scenario":
		if len(tokens) == 1 || strings.ToLower(tokens[1]) == "list" {
			s.o.line("W list")
			return
		}
		id := tokens[1]
		if len(tokens) > 2 {
			id = strings.Join(tokens[1:], "-")
		}
		sc, ok := engine.GetScenario(engine.ScenarioID(id))
		if !ok {
			s.o.line("W unknown %s", id)
			return
		}
		s.g = sc.Build(s.seed)
		s.o.line("W build %s", sc.ID)
		return
	}
	parsed := parser.ParseCommand(trimmed)
	s.detail("P", parsedJSON(parsed))
	if parsed.Error != nil || parsed.Special != "" || parsed.Action == nil {
		return
	}
	events, err := s.g.Dispatch(parsed.Action)
	if err != nil {
		s.detail("X", err.Error())
		return
	}
	sessionCounterAttack(s.g, parsed.Action, &events)
	s.detail("E", eventsJSON(events))
	if _, ok := hasGameOver(events); ok {
		s.over = true
	}
}

var noise = []string{"", "xyzzy", "nav", "nav 9 9", "nav 1.5", "tor x y", "pha -5", "pha 0", "she abc", "she", "help nav", "help", "theme amber", "options", "tour", "scenarios", "tor c 12", "pha 100 9,9", "nav q 0 0", "nav s 1 x", "nav c a b", "she 99999", "pha 99999", "nav 0.5 100", "tor 1 2 3", "  ", "help xyz", "orders", "nav q 4 4 0", "she -99999"}

func emptyNeighbors(g *engine.GameState, c engine.Coord) []engine.Coord {
	var res []engine.Coord
	for dr := -1; dr <= 1; dr++ {
		for dc := -1; dc <= 1; dc++ {
			n := engine.Coord{c[0] + dr, c[1] + dc}
			if (dr != 0 || dc != 0) && n[0] >= 1 && n[0] <= 8 && n[1] >= 1 && n[1] <= 8 && g.CurrentQuad.Grid[n[0]][n[1]] == engine.EntityEmpty {
				res = append(res, n)
			}
		}
	}
	return res
}

func (s *sim) targets() []engine.Coord {
	g := s.g
	var ts []engine.Coord
	for _, k := range g.CurrentQuad.Klingons {
		if k != nil {
			ts = append(ts, k.Sector)
		}
	}
	for _, e := range g.CurrentQuad.Enemies {
		if e != nil {
			ts = append(ts, e.Sector)
		}
	}
	for _, p := range g.CurrentQuad.PlasmaTorpedoes {
		ts = append(ts, p.Sector)
	}
	for _, w := range g.CurrentQuad.WebSegments {
		if w != nil && s.r.IntN(3) == 0 {
			ts = append(ts, w.Coord)
		}
	}
	return ts
}

func (s *sim) command() string {
	r := s.r
	g := s.g
	if p := s.pending; p != nil {
		s.pending = nil
		if r.IntN(10) < 7 {
			switch r.IntN(4) {
			case 0, 1:
				return fmt.Sprintf("nav s %d %d", p[0], p[1])
			case 2:
				return fmt.Sprintf("tor %d %d", p[0], p[1])
			default:
				return fmt.Sprintf("nav q %d %d %d", g.Enterprise.Quad[0], g.Enterprise.Quad[1], 1)
			}
		}
	}
	if r.IntN(100) < 7 {
		return noise[r.IntN(len(noise))]
	}
	ts := s.targets()
	coord := func(c engine.Coord) string { return fmt.Sprintf("%d %d", c[0], c[1]) }
	if len(ts) > 0 {
		t := ts[r.IntN(len(ts))]
		switch x := r.IntN(100); {
		case x < 30:
			return "tor " + coord(t)
		case x < 48:
			return fmt.Sprintf("pha %d", []int{50, 100, 200, 300, 500, 800, 1500}[r.IntN(7)])
		case x < 62:
			if r.IntN(2) == 0 {
				return fmt.Sprintf("pha %d %d,%d", 100+r.IntN(900), t[0], t[1])
			}
			return fmt.Sprintf("pha %d %s", 100+r.IntN(900), coord(t))
		case x < 70:
			if g.Enterprise.Shields > 200 && r.IntN(3) == 0 {
				return fmt.Sprintf("she -%d", 100+r.IntN(int(g.Enterprise.Shields)-99))
			}
			return fmt.Sprintf("she %d", 100+r.IntN(900))
		case x < 80:
			if n := emptyNeighbors(g, g.Enterprise.Sector); len(n) > 0 && r.IntN(2) == 0 {
				c := n[r.IntN(len(n))]
				return []string{"nav ", "nav s "}[r.IntN(2)] + coord(c)
			}
			return fmt.Sprintf("nav %d %d", 1+r.IntN(8), 1+r.IntN(8))
		case x < 86:
			if r.IntN(2) == 0 {
				return fmt.Sprintf("tor %.2f", r.Float64()*6.28)
			}
			return fmt.Sprintf("tor c %.1f", 1+r.Float64()*8)
		case x < 90:
			return "lrscan"
		default:
			return fmt.Sprintf("nav q %d %d", 1+r.IntN(8), 1+r.IntN(8))
		}
	}
	// no hostiles here
	if sb := g.CurrentQuad.Starbase; sb != nil && r.IntN(100) < 75 {
		d0 := g.Enterprise.Sector[0] - sb[0]
		d1 := g.Enterprise.Sector[1] - sb[1]
		if d0 >= -1 && d0 <= 1 && d1 >= -1 && d1 <= 1 && g.Enterprise.Condition != engine.ConditionDocked {
			return []string{"doc", "dock"}[r.IntN(2)]
		}
		if n := emptyNeighbors(g, *sb); len(n) > 0 && g.Enterprise.Condition != engine.ConditionDocked {
			return "nav s " + coord(n[r.IntN(len(n))])
		}
	}
	switch x := r.IntN(100); {
	case x < 45:
		var qs []engine.Coord
		for qr := 1; qr <= 8; qr++ {
			for qc := 1; qc <= 8; qc++ {
				if g.GalaxyChart[qr][qc] >= 100 && (engine.Coord{qr, qc}) != g.Enterprise.Quad {
					qs = append(qs, engine.Coord{qr, qc})
				}
			}
		}
		if len(qs) == 0 || r.IntN(5) == 0 {
			return fmt.Sprintf("nav q %d %d", 1+r.IntN(8), 1+r.IntN(8))
		}
		q := qs[r.IntN(len(qs))]
		if r.IntN(3) == 0 {
			return fmt.Sprintf("nav q %d %d %.1f", q[0], q[1], 0.5+r.Float64()*2)
		}
		return "nav q " + coord(q)
	case x < 60:
		return fmt.Sprintf("nav %.2f %.2f", r.Float64()*6.28, 0.1+r.Float64()*1.5)
	case x < 68:
		return fmt.Sprintf("nav c %.1f %.1f", r.Float64()*6.28, 0.2+r.Float64())
	case x < 78:
		return "lrscan"
	case x < 85:
		if g.Enterprise.Shields > 0 {
			return fmt.Sprintf("she -%v", g.Enterprise.Shields)
		}
		return fmt.Sprintf("she %d", 200+r.IntN(800))
	case x < 92:
		if n := emptyNeighbors(g, g.Enterprise.Sector); len(n) > 0 {
			return "nav " + coord(n[r.IntN(len(n))])
		}
		return "nav 1 1"
	default:
		return fmt.Sprintf("tor %d %d", 1+r.IntN(8), 1+r.IntN(8))
	}
}

// extra performs engine-level operations the parser cannot produce:
// grid hazards ("M ent r c"), direct actions ("A <type> <json>"),
// AdvanceTurn ("V <elapsed>") and MoveKlingon ("MK <idx> r c").
func (s *sim) extra() {
	r := s.r
	g := s.g
	x := r.IntN(100)
	switch {
	case x < 9:
		ents := []engine.EntityType{engine.EntityBlackHole, engine.EntityBlackHole, engine.EntityWormhole, engine.EntityWormhole, engine.EntityPlanet}
		e := ents[r.IntN(len(ents))]
		c := engine.Coord{1 + r.IntN(8), 1 + r.IntN(8)}
		if g.CurrentQuad.Grid[c[0]][c[1]] == engine.EntityEnterprise {
			return
		}
		g.CurrentQuad.Grid[c[0]][c[1]] = e
		s.pending = &c
		s.o.line("M %d %d %d", e, c[0], c[1])
		s.o.line("H %s", stateHash(g))
	case x < 12:
		var a engine.Action
		switch r.IntN(3) {
		case 0:
			ts := s.targets()
			t := engine.Coord{1 + r.IntN(8), 1 + r.IntN(8)}
			if len(ts) > 0 {
				t = ts[r.IntN(len(ts))]
			}
			a = engine.ActionTorpedoDirect{TargetSector: t}
		case 1:
			a = engine.ActionCallHelp{}
		default:
			id := r.IntN(4)
			if len(g.CurrentQuad.Klingons) > 0 && r.IntN(4) != 0 {
				id = g.CurrentQuad.Klingons[r.IntN(len(g.CurrentQuad.Klingons))].ID
			}
			a = engine.ActionKlingonCounterAttack{EnemyID: id, Damage: float64(r.IntN(400)) - 20}
		}
		b, _ := json.Marshal(a)
		s.o.line("A %T %s", a, b)
		events, err := g.Dispatch(a)
		if err != nil {
			s.detail("X", err.Error())
		} else {
			executeKlingonTurnIfActive(g, a, &events)
			s.detail("E", eventsJSON(events))
			if ev, ok := hasGameOver(events); ok {
				s.handleGameOver(ev)
			}
		}
		if !s.over {
			s.evaluateTour()
		}
		s.o.line("H %s", stateHash(s.g))
	case x < 15:
		el := float64(r.IntN(30)) / 10
		if r.IntN(3) == 0 {
			g.Enterprise.Devices[r.IntN(int(engine.NumDevices))] = float64(r.IntN(50)) / 10
		}
		s.o.line("V %v %s", el, mustJSON(g.Enterprise.Devices))
		g.AdvanceTurn(el)
		s.o.line("H %s", stateHash(g))
	case x < 18:
		if len(g.CurrentQuad.Klingons) == 0 {
			return
		}
		idx := r.IntN(len(g.CurrentQuad.Klingons))
		k := g.CurrentQuad.Klingons[idx]
		dest := engine.Coord{k.Sector[0] + r.IntN(3) - 1, k.Sector[1] + r.IntN(3) - 1}
		if r.IntN(3) == 0 {
			dest = engine.Coord{r.IntN(10), r.IntN(10)}
		}
		s.o.line("MK %d %d %d", idx, dest[0], dest[1])
		s.detail("E", eventsJSON(engine.MoveKlingon(g, k, dest)))
		s.o.line("H %s", stateHash(g))
	}
}

// games.txt: scripted games. Header "game <n> <mode> <kind> <params json>",
// then per command "> <hex input>", P/E/X (full or "#<hash>"), extra
// T/B/D/C/F/TS/G/W lines, "H <state hash>", periodic and final S/K/R dumps.
func genGames() {
	o := create("games.txt")
	defer o.close()
	r := rand.New(rand.NewPCG(2026, 11))
	n := 0
	type spec struct {
		mode, kind string
		params     map[string]any
	}
	var specs []spec
	profiles := allProfiles()
	for i := 0; i < 130; i++ {
		p := profiles[i%len(profiles)]
		rules := engine.DefaultRulesForProfile(p)
		if i%3 == 1 {
			rules.Adversaries = true
		}
		if i%5 == 2 {
			rules.SpatialAnomalies = true
			rules.KlingonCloak = !rules.KlingonCloak
		}
		if i%7 == 3 {
			rules.RepairMultiplier = 0
		}
		specs = append(specs, spec{"tui", "game", map[string]any{"seed": int64(1000 + i*37), "rules": rules}})
	}
	for _, sc := range engine.ListScenarios() {
		for i := 0; i < 16; i++ {
			specs = append(specs, spec{"tui", "scenario", map[string]any{"seed": int64(50 + i*13), "id": string(sc.ID)}})
		}
	}
	for i := 0; i < 26; i++ {
		specs = append(specs, spec{"tui", "tour", map[string]any{"seed": int64(300 + i*101)}})
	}
	for i := 0; i < 60; i++ {
		specs = append(specs, spec{"wasm", "session", map[string]any{"seed": int64(i * 7), "difficulty": string(profiles[i%len(profiles)])}})
	}
	for _, sp := range specs {
		s := &sim{mode: sp.mode, lb: engine.NewLeaderboard(), r: rand.New(rand.NewPCG(uint64(n), 99)), o: o, full: n < 30}
		switch sp.kind {
		case "game":
			rules := sp.params["rules"].(engine.GameRules)
			s.seed = sp.params["seed"].(int64)
			s.g = engine.NewGameWithOptions(s.seed, engine.SkillGood, engine.LengthMedium, rules)
			s.g.PopulateQuadrant(s.g.Enterprise.Quad, s.g.Enterprise.Sector)
		case "scenario":
			s.seed = sp.params["seed"].(int64)
			sc, _ := engine.GetScenario(engine.ScenarioID(sp.params["id"].(string)))
			s.g = sc.Build(s.seed)
		case "tour":
			s.seed = sp.params["seed"].(int64)
			s.tour = engine.NewTour(s.seed)
			s.g, _ = s.tour.StartCurrentSector()
			if len(s.g.CurrentQuad.Enemies) == 0 {
				s.g.PopulateQuadrant(s.g.Enterprise.Quad, s.g.Enterprise.Sector)
			}
		case "session":
			s.seed = sp.params["seed"].(int64)
			if s.seed == 0 {
				s.seed = 12345
			}
			diff := engine.DifficultyProfile(sp.params["difficulty"].(string))
			s.g = engine.NewGameWithOptions(s.seed, engine.SkillGood, engine.LengthMedium, engine.DefaultRulesForProfile(diff))
			s.g.PopulateQuadrant(s.g.Enterprise.Quad, s.g.Enterprise.Sector)
		}
		o.line("game %d %s %s %s", n, sp.mode, sp.kind, mustJSON(sp.params))
		if s.full {
			writeFull(o, s.g)
		} else {
			o.line("H %s", stateHash(s.g))
		}
		maxSteps := 70
		if sp.kind == "tour" {
			maxSteps = 160
		}
		for step := 0; step < maxSteps && !s.over; step++ {
			var cmd string
			if sp.mode == "wasm" && r.IntN(20) == 0 {
				cmd = []string{"scenario km", "scenario mutara", "scenario starbase siege", "scenario list", "scenario nope", "srs", "lrs", "chart", "dam", "help"}[r.IntN(10)]
			} else {
				cmd = s.command()
			}
			if sp.mode == "tui" {
				if n%3 == 0 {
					s.extra()
					if s.over {
						break
					}
				}
				s.stepTUI(cmd)
			} else {
				s.stepWASM(cmd)
			}
			if step%60 == 59 {
				writeFull(o, s.g)
			}
		}
		o.line("end")
		writeFull(o, s.g)
		n++
	}
}
