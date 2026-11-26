package targetlock

// Parity reference generator for the Rust port
// (crates/sst-tui/src/components/targetlock). Run with:
//   SST_PARITYGEN=1 go test ./pkg/tui/components/targetlock -run TestParityGen -count=1
// Writes testdata/parity/components/targetlock.txt. Every line is
// "<key>\t<%q of value>": target telemetry (sort.Slice order, ballistics)
// for seeded games and synthetic contact lists (ties, >12 entries), key
// sequences through Update with emitted messages, and the HUD View across
// themes x {dark, light} x profiles.

import (
	"bufio"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/muesli/termenv"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

func pgTargets(m Model) string {
	var b strings.Builder
	for _, t := range m.Targets() {
		fmt.Fprintf(&b, "%d %v %v %v %v %v %q %d|", t.KlingonID, t.Coord, t.Distance, t.Bearing, t.HitProbability, t.Power, t.Name, t.Type)
	}
	return b.String()
}

func pgMState(m Model) string {
	cur := "nil"
	if c := m.CurrentTarget(); c != nil {
		cur = fmt.Sprintf("%d@%v", c.KlingonID, c.Coord)
	}
	return fmt.Sprintf("idx=%d cur=%s phaser=%v input=%q warn=%q n=%d", m.TargetIndex(), cur, m.InputtingPhaser(), m.PhaserInput(),
		m.WarningMessage(), len(m.Targets()))
}

func pgCmd(cmd tea.Cmd) string {
	if cmd == nil {
		return "nil"
	}
	switch v := cmd().(type) {
	case FireTorpedoMsg:
		return fmt.Sprintf("torpedo %v %v", v.Target, v.Bearing)
	case FirePhasersMsg:
		return fmt.Sprintf("phasers %v", v.Energy)
	case CloseHUDMsg:
		return "close"
	}
	return "?"
}

func TestParityGen(t *testing.T) {
	if os.Getenv("SST_PARITYGEN") != "1" {
		t.Skip("set SST_PARITYGEN=1 to regenerate parity data")
	}
	out := filepath.Join("..", "..", "..", "..", "testdata", "parity", "components")
	if err := os.MkdirAll(out, 0o755); err != nil {
		t.Fatal(err)
	}
	f, err := os.Create(filepath.Join(out, "targetlock.txt"))
	if err != nil {
		t.Fatal(err)
	}
	defer f.Close()
	bw := bufio.NewWriter(f)
	defer bw.Flush()
	w := func(k, v string) { fmt.Fprintf(bw, "%s\t%q\n", k, v) }
	lipgloss.SetHasDarkBackground(true)
	defer lipgloss.SetColorProfile(termenv.Ascii)
	th0 := pgVariants()[0].th

	games := pgGames()
	for _, gm := range games {
		g := gm.g
		m := New(th0)
		m.SetStateFromGame(g, engine.Coord{})
		w("game/"+gm.name+"/targets", pgTargets(m))
		w("game/"+gm.name+"/state", pgMState(m))
		pgRot(w, "game/"+gm.name+"/view", func(th theme.Theme) string { m.SetTheme(th); return m.View() })
		if ts := m.Targets(); len(ts) > 0 {
			m.SetStateFromGame(g, ts[len(ts)-1].Coord)
			w("game/"+gm.name+"/initial", pgMState(m))
		}
	}
	nm := New(th0)
	nm.SetStateFromGame(nil, engine.Coord{1, 1})
	w("nilgame", pgMState(nm))

	// Key sequences.
	seq := []string{"tab", "tab", "right", "shift+tab", "left", "enter", "x", "p", "3", "5", "0", ".", "5", "a", "tab",
		"enter", "p", "enter", "backspace", "x", "1", "backspace", "backspace", "P", "p", ".", "enter", "9", "9",
		"backspace", "enter", "P", "esc", "p", "esc", "left", "left", "left", "enter", "space", "f2"}
	for _, name := range []string{"hot42", "hazards", "damaged", "seed1"} {
		var g *engine.GameState
		for _, gm := range games {
			if gm.name == name {
				g = gm.g
			}
		}
		m := New(th0)
		m.SetStateFromGame(g, engine.Coord{})
		for i, k := range seq {
			var cmd tea.Cmd
			m, cmd = m.Update(pgKey(k))
			key := fmt.Sprintf("seq/%s/%02d", name, i)
			w(key, fmt.Sprintf("key=%q cmd=%s %s", k, pgCmd(cmd), pgMState(m)))
			if name == "hot42" || i%8 == 0 {
				pgRot(w, key+"/view", func(th theme.Theme) string { m.SetTheme(th); return m.View() })
			}
		}
		m2, cmd := m.Update(tea.WindowSizeMsg{Width: 10, Height: 10})
		w("seq/"+name+"/winsize", pgCmd(cmd)+" "+pgMState(m2))
	}

	// Synthetic contact lists: ties, duplicate IDs, nil entries, >12 entries.
	var ties []*engine.Klingon
	for i := 0; i < 16; i++ {
		ties = append(ties, &engine.Klingon{ID: 5 - i%4, Sector: engine.Coord{4 + []int{-2, 0, 2, 0}[i%4], 4 + []int{0, 2, 0, -2}[i%4]}, Energy: float64(100 + i)})
	}
	ties = append(ties, nil)
	var all []*engine.Klingon
	for r := 1; r <= 8; r++ {
		for c := 1; c <= 8; c++ {
			all = append(all, &engine.Klingon{ID: (r*7 + c*3) % 11, Sector: engine.Coord{r, c}, Energy: float64(r*c) * 12.5})
		}
	}
	lists := []struct {
		name  string
		ent   engine.Coord
		ks    []*engine.Klingon
		init  engine.Coord
		torps int
	}{
		{"empty", engine.Coord{4, 4}, nil, engine.Coord{}, 10},
		{"ties", engine.Coord{4, 4}, ties, engine.Coord{4, 6}, 3},
		{"all-center", engine.Coord{4, 4}, all, engine.Coord{8, 8}, 0},
		{"all-corner", engine.Coord{1, 1}, all, engine.Coord{9, 9}, 10},
		{"all-edge", engine.Coord{8, 5}, all[10:40], engine.Coord{}, 1},
		{"far", engine.Coord{1, 1}, []*engine.Klingon{{ID: 4, Sector: engine.Coord{16, 16}, Energy: 400}, {ID: 3, Sector: engine.Coord{1, 1}, Energy: -1}}, engine.Coord{}, 10},
	}
	for _, l := range lists {
		m := New(th0)
		m.SetState(l.ent, 3450.5, l.torps, l.ks, l.init)
		w("list/"+l.name+"/targets", pgTargets(m))
		w("list/"+l.name+"/state", pgMState(m))
		pgRot(w, "list/"+l.name+"/view", func(th theme.Theme) string { m.SetTheme(th); return m.View() })
		m, cmd := m.Update(pgKey("enter"))
		w("list/"+l.name+"/enter", pgCmd(cmd)+" "+pgMState(m))
	}

	// Web-heavy quadrant (>12 targets from SetStateFromGame).
	wg := engine.NewGame(5, engine.SkillGood, engine.LengthMedium)
	wg.Enterprise.Sector = engine.Coord{4, 4}
	for r := 1; r <= 8; r++ {
		for c := 1; c <= 8; c += 3 {
			wg.CurrentQuad.WebSegments = append(wg.CurrentQuad.WebSegments, &engine.TholianWebSegment{Coord: engine.Coord{r, c}, Strength: float64(r + c)})
		}
	}
	wg.CurrentQuad.WebSegments = append(wg.CurrentQuad.WebSegments, nil)
	wg.CurrentQuad.PlasmaTorpedoes = []*engine.PlasmaTorpedo{nil, {ID: 10005, Sector: engine.Coord{4, 7}, Energy: 9}}
	wg.CurrentQuad.Klingons = []*engine.Klingon{{ID: 10001, Sector: engine.Coord{1, 4}, Energy: 50}, {ID: 2, Sector: engine.Coord{2, 2}, Energy: 0},
		{ID: 3, Sector: engine.Coord{3, 3}, Energy: 70, IsCloaked: true}, {ID: 4, Sector: engine.Coord{7, 7}, Energy: 80, IsCommander: true}}
	m := New(th0)
	m.SetStateFromGame(wg, engine.Coord{4, 7})
	w("web/targets", pgTargets(m))
	w("web/state", pgMState(m))
	pgEach(w, "web/view", func(th theme.Theme) string { m.SetTheme(th); return m.View() })
	m, _ = m.Update(pgKey("p"))
	m, _ = m.Update(pgKey("7"))
	pgEach(w, "web/phaser", func(th theme.Theme) string { m.SetTheme(th); return m.View() })
	pgEach(w, "new", func(th theme.Theme) string { return New(th).View() })
	for _, v := range pgVariants() {
		w("theme/"+v.key, New(v.th).Theme().Name()+" "+pgMState(New(v.th)))
	}
}

// ---- shared parity helpers (identical in every component generator) ----

type pgVariant struct {
	key string
	th  theme.Theme
}

func pgVariants() []pgVariant {
	var out []pgVariant
	for _, name := range []string{"modern", "lcars", "crt"} {
		for _, mode := range []theme.ColorMode{theme.ColorModeDark, theme.ColorModeLight} {
			out = append(out, pgVariant{name + "-" + string(mode), theme.GetTheme(name).WithColorMode(mode)})
		}
	}
	return out
}

type pgProfile struct {
	key string
	p   termenv.Profile
}

var pgFull = []pgProfile{{"ascii", termenv.Ascii}, {"truecolor", termenv.TrueColor}}

// pgEach renders under every theme variant with the Ascii and TrueColor
// profiles, plus ANSI256 for the first variant.
func pgEach(w func(k, v string), key string, render func(th theme.Theme) string) {
	for i, v := range pgVariants() {
		profs := pgFull
		if i == 0 {
			profs = append(append([]pgProfile{}, pgFull...), pgProfile{"ansi256", termenv.ANSI256})
		}
		for _, p := range profs {
			lipgloss.SetColorProfile(p.p)
			w(key+"/"+v.key+"/"+p.key, render(v.th))
		}
	}
	lipgloss.SetColorProfile(termenv.Ascii)
}

// pgLite renders under three representative variant/profile pairs.
func pgLite(w func(k, v string), key string, render func(th theme.Theme) string) {
	vs := pgVariants()
	pairs := []struct {
		v int
		p pgProfile
	}{{0, pgFull[1]}, {3, pgFull[1]}, {4, pgFull[0]}}
	for _, pr := range pairs {
		lipgloss.SetColorProfile(pr.p.p)
		w(key+"/"+vs[pr.v].key+"/"+pr.p.key, render(vs[pr.v].th))
	}
	lipgloss.SetColorProfile(termenv.Ascii)
}

var pgRotN int

// pgRot renders once, rotating through the theme variants (TrueColor).
func pgRot(w func(k, v string), key string, render func(th theme.Theme) string) {
	vs := pgVariants()
	v := vs[pgRotN%len(vs)]
	pgRotN++
	lipgloss.SetColorProfile(termenv.TrueColor)
	w(key+"/"+v.key+"/truecolor", render(v.th))
	lipgloss.SetColorProfile(termenv.Ascii)
}

func pgKey(s string) tea.KeyMsg {
	switch s {
	case "enter":
		return tea.KeyMsg{Type: tea.KeyEnter}
	case "esc":
		return tea.KeyMsg{Type: tea.KeyEsc}
	case "tab":
		return tea.KeyMsg{Type: tea.KeyTab}
	case "shift+tab":
		return tea.KeyMsg{Type: tea.KeyShiftTab}
	case "up":
		return tea.KeyMsg{Type: tea.KeyUp}
	case "down":
		return tea.KeyMsg{Type: tea.KeyDown}
	case "left":
		return tea.KeyMsg{Type: tea.KeyLeft}
	case "right":
		return tea.KeyMsg{Type: tea.KeyRight}
	case "backspace":
		return tea.KeyMsg{Type: tea.KeyBackspace}
	case "space":
		return tea.KeyMsg{Type: tea.KeySpace}
	case "home":
		return tea.KeyMsg{Type: tea.KeyHome}
	case "end":
		return tea.KeyMsg{Type: tea.KeyEnd}
	case "delete":
		return tea.KeyMsg{Type: tea.KeyDelete}
	case "ctrl+a":
		return tea.KeyMsg{Type: tea.KeyCtrlA}
	case "ctrl+e":
		return tea.KeyMsg{Type: tea.KeyCtrlE}
	case "ctrl+u":
		return tea.KeyMsg{Type: tea.KeyCtrlU}
	case "ctrl+k":
		return tea.KeyMsg{Type: tea.KeyCtrlK}
	case "ctrl+w":
		return tea.KeyMsg{Type: tea.KeyCtrlW}
	case "alt+b":
		return tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune("b"), Alt: true}
	case "f2":
		return tea.KeyMsg{Type: tea.KeyF2}
	}
	return tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune(s)}
}

// ---- shared seeded games ----

type pgGame struct {
	name string
	g    *engine.GameState
}

// pgHot populates the first quadrant (row-major) holding at least min
// Klingons, falling back to the Enterprise quadrant.
func pgHot(g *engine.GameState, min int) {
	for r := 1; r <= 8; r++ {
		for c := 1; c <= 8; c++ {
			if g.GalaxyChart[r][c]/100 >= min {
				g.PopulateQuadrant(engine.Coord{r, c}, g.Enterprise.Sector)
				return
			}
		}
	}
	g.PopulateQuadrant(g.Enterprise.Quad, g.Enterprise.Sector)
}

func pgAdvRules() engine.GameRules {
	r := engine.DefaultRulesForProfile(engine.ProfileNormal)
	r.Adversaries = true
	r.KlingonCloak = true
	r.SpatialAnomalies = true
	return r
}

func pgGames() []pgGame {
	var out []pgGame
	for _, seed := range []int64{1, 7, 42, 99, 314, 2024, 12345, 777} {
		skill := engine.SkillLevel(1 + seed%5)
		length := engine.GameLength(1 + seed%3)
		g := engine.NewGame(seed, skill, length)
		g.PopulateQuadrant(g.Enterprise.Quad, g.Enterprise.Sector)
		out = append(out, pgGame{fmt.Sprintf("seed%d", seed), g})
		h := engine.NewGame(seed, skill, length)
		pgHot(h, 2)
		out = append(out, pgGame{fmt.Sprintf("hot%d", seed), h})
	}
	for i, prof := range []engine.DifficultyProfile{engine.ProfileCasual, engine.ProfileHardcore, engine.ProfileNightmare, engine.ProfileExpert, engine.ProfileEmeritus} {
		g := engine.NewGameWithOptions(int64(5+i), engine.SkillGood, engine.LengthMedium, engine.DefaultRulesForProfile(prof))
		pgHot(g, 1)
		out = append(out, pgGame{"profile-" + string(prof), g})
	}
	for _, seed := range []int64{11, 23, 58, 64, 90} {
		g := engine.NewGameWithOptions(seed, engine.SkillExpert, engine.LengthLong, pgAdvRules())
		pgHot(g, 1)
		out = append(out, pgGame{fmt.Sprintf("adv%d", seed), g})
	}
	out = append(out, pgGame{"kobayashi", engine.BuildKobayashiMaru(3)})
	out = append(out, pgGame{"mutara", engine.BuildMutaraNebula(4)})
	out = append(out, pgGame{"siege", engine.BuildStarbaseSiege(5)})

	// Hazards: plasma torpedoes, web segments, a cloaked adversary.
	hz := engine.NewGameWithOptions(42, engine.SkillGood, engine.LengthMedium, pgAdvRules())
	pgHot(hz, 2)
	hz.CurrentQuad.PlasmaTorpedoes = []*engine.PlasmaTorpedo{
		{ID: 1, SourceID: 1, Sector: engine.Coord{2, 2}, Energy: 300, TargetSector: hz.Enterprise.Sector},
		{ID: 2, SourceID: 1, Sector: engine.Coord{7, 6}, Energy: 0},
		{ID: 3, SourceID: 2, Sector: engine.Coord{5, 8}, Energy: 120.5},
	}
	for i, c := range []engine.Coord{{1, 1}, {1, 2}, {1, 3}, {2, 3}, {3, 3}} {
		hz.CurrentQuad.WebSegments = append(hz.CurrentQuad.WebSegments, &engine.TholianWebSegment{Coord: c, Strength: float64(50 * i)})
	}
	if len(hz.CurrentQuad.Enemies) > 0 && hz.CurrentQuad.Enemies[0] != nil {
		hz.CurrentQuad.Enemies[0].IsCloaked = true
	}
	engine.SyncQuadrantEnemies(&hz.CurrentQuad)
	out = append(out, pgGame{"hazards", hz})

	// Damage, docking, red alert.
	dm := engine.NewGame(99, engine.SkillFair, engine.LengthShort)
	pgHot(dm, 1)
	for i := range dm.Enterprise.Devices {
		dm.Enterprise.Devices[i] = float64(i) * 0.75
	}
	dm.Enterprise.Energy = 1234.5
	dm.Enterprise.Shields = 2600
	dm.Enterprise.Torpedoes = 3
	dm.Enterprise.Condition = engine.ConditionRed
	out = append(out, pgGame{"damaged", dm})

	dk := engine.NewGame(314, engine.SkillGood, engine.LengthMedium)
	dk.PopulateQuadrant(dk.Enterprise.Quad, dk.Enterprise.Sector)
	dk.Enterprise.Condition = engine.ConditionDocked
	dk.Enterprise.Devices[engine.DeviceLRSensors] = 3.5
	dk.Enterprise.Devices[engine.DeviceComputer] = 1.25
	out = append(out, pgGame{"docked", dk})

	lrs := engine.NewGame(2024, engine.SkillGood, engine.LengthMedium)
	pgHot(lrs, 1)
	lrs.Enterprise.Devices[engine.DeviceLRSensors] = 0.5
	lrs.Enterprise.Condition = engine.ConditionYellow
	lrs.Enterprise.Energy = -10
	lrs.Enterprise.Torpedoes = 0
	out = append(out, pgGame{"lrs-degraded", lrs})

	off := engine.NewGame(7, engine.SkillGood, engine.LengthMedium)
	off.Rules.AnimSpeed = engine.AnimSpeedOff
	off.Rules.SensorDegradation = false
	off.Enterprise.Condition = engine.ConditionRed
	off.Enterprise.Quad = engine.Coord{8, 8}
	off.Enterprise.Devices[engine.DeviceLRSensors] = 1.0
	out = append(out, pgGame{"animoff-corner", off})
	return out
}
