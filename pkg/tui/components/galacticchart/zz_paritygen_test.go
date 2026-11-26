package galacticchart

// Parity reference generator for the Rust port
// (crates/sst-tui/src/components/galacticchart). Run with:
//   SST_PARITYGEN=1 go test ./pkg/tui/components/galacticchart -run TestParityGen -count=1
// Writes testdata/parity/components/galacticchart.txt. Every line is
// "<key>\t<%q of value>": CalculateTelemetry, the chart View for seeded
// games and synthetic charts, key sequences (cursor, help, warp, close,
// computer damage) with emitted messages, sizes, SetState/SetCursor clamping
// and HitTest, across themes x {dark, light} x profiles.

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

func pgCmd(cmd tea.Cmd) string {
	if cmd == nil {
		return "nil"
	}
	switch v := cmd().(type) {
	case WarpToQuadrantMsg:
		return fmt.Sprintf("warp %v %v", v.DestQuad, v.Warp)
	case WarpBlockedMsg:
		return fmt.Sprintf("blocked %q", v.Reason)
	case CloseChartMsg:
		return "close"
	}
	return "?"
}

func pgMState(m Model) string {
	return fmt.Sprintf("cursor=%v help=%v size=%dx%d comp=%v ent=%v", m.Cursor(), m.ShowingHelp(), m.Width(), m.Height(),
		m.ComputerDamaged(), m.enterpriseQuad)
}

func TestParityGen(t *testing.T) {
	if os.Getenv("SST_PARITYGEN") != "1" {
		t.Skip("set SST_PARITYGEN=1 to regenerate parity data")
	}
	out := filepath.Join("..", "..", "..", "..", "testdata", "parity", "components")
	if err := os.MkdirAll(out, 0o755); err != nil {
		t.Fatal(err)
	}
	f, err := os.Create(filepath.Join(out, "galacticchart.txt"))
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

	// Telemetry.
	for _, from := range []engine.Coord{{1, 1}, {4, 4}, {8, 8}, {3, 6}, {0, 0}, {9, -1}} {
		var b strings.Builder
		for r := 0; r <= 9; r++ {
			for c := 0; c <= 9; c++ {
				fmt.Fprintf(&b, "%v|", CalculateTelemetry(from, engine.Coord{r, c}))
			}
		}
		w(fmt.Sprintf("telemetry/%d,%d", from[0], from[1]), b.String())
	}

	// Seeded games.
	for i, gm := range pgGames() {
		g := gm.g
		comp := g.Enterprise.Devices[engine.DeviceComputer] > 0
		each := pgRot
		if gm.name == "hazards" {
			each = pgEach
		}
		m := New(th0, 64, 18)
		m.SetState(g.Enterprise.Quad, g.GalaxyChart, g.ChartDiscovered, g.ChartKnownBases, comp)
		if i%3 == 1 {
			m.SetCursor(engine.Coord{1 + i%8, 8 - i%8})
		}
		w("game/"+gm.name+"/state", pgMState(m))
		each(w, "game/"+gm.name, func(th theme.Theme) string { m.SetTheme(th); return m.View() })
	}

	// Synthetic chart: every quadrant discovered or a known base.
	var chart [9][9]int
	var disc, bases [9][9]bool
	for r := 0; r < 9; r++ {
		for c := 0; c < 9; c++ {
			chart[r][c] = (r*131 + c*17) % 1200
			disc[r][c] = (r+c)%3 != 0
			bases[r][c] = (r*c)%4 == 1
		}
	}
	chart[2][2] = -7
	seq := []string{"?", "k", "?", "esc", "?", "q", "?", "Q", "?", "x", "?", "j", "up", "up", "up", "up", "up", "left",
		"left", "left", "left", "h", "H", "K", "enter", "l", "L", "right", "J", "down", "j", "?", "enter", "esc", "f2",
		"tab", "space", "?", "?", "l", "enter"}
	for _, comp := range []bool{false, true} {
		m := New(th0, 64, 18)
		m.SetState(engine.Coord{3, 3}, chart, disc, bases, comp)
		for i, k := range seq {
			var cmd tea.Cmd
			m, cmd = m.Update(pgKey(k))
			key := fmt.Sprintf("seq/comp=%v/%02d", comp, i)
			w(key, fmt.Sprintf("key=%q cmd=%s %s", k, pgCmd(cmd), pgMState(m)))
			if i%8 == 0 {
				pgRot(w, key+"/view", func(th theme.Theme) string { m.SetTheme(th); return m.View() })
			}
		}
		m, cmd := m.Update(tea.WindowSizeMsg{Width: 200, Height: 100})
		w(fmt.Sprintf("seq/comp=%v/winsize", comp), pgCmd(cmd)+" "+pgMState(m))
		m, cmd = m.Update(tea.MouseMsg{X: 10, Y: 5, Button: tea.MouseButtonLeft})
		w(fmt.Sprintf("seq/comp=%v/mouse", comp), pgCmd(cmd)+" "+pgMState(m))
	}
	pgEach(w, "synthetic", func(th theme.Theme) string {
		m := New(th, 64, 18)
		m.SetState(engine.Coord{5, 2}, chart, disc, bases, false)
		m.SetCursor(engine.Coord{2, 7})
		return m.View()
	})
	pgLite(w, "help", func(th theme.Theme) string {
		m := New(th, 64, 18)
		m.SetShowingHelp(true)
		return m.View()
	})

	// Sizes and clamping.
	sizes := [][2]int{{0, 0}, {80, 24}, {40, 10}, {12, 5}, {3, 3}, {-1, 30}}
	for _, sz := range sizes {
		pgLite(w, fmt.Sprintf("new/%dx%d", sz[0], sz[1]), func(th theme.Theme) string {
			m := New(th, sz[0], sz[1])
			return pgMState(m) + "\n" + m.View()
		})
		pgRot(w, fmt.Sprintf("setsize/%dx%d", sz[0], sz[1]), func(th theme.Theme) string {
			m := New(th, 64, 18)
			m.SetSize(sz[0], sz[1])
			m.SetShowingHelp(sz[0] == 80)
			return pgMState(m) + "\n" + m.View()
		})
	}
	m := New(th0, 64, 18)
	for _, c := range []engine.Coord{{0, 10}, {-3, 4}, {9, 9}, {5, 5}} {
		m.SetState(c, chart, disc, bases, false)
		w(fmt.Sprintf("setstate/%d,%d", c[0], c[1]), pgMState(m))
		m.SetCursor(engine.Coord{c[1], c[0]})
		w(fmt.Sprintf("setcursor/%d,%d", c[1], c[0]), pgMState(m))
	}
	w("init", fmt.Sprint(m.Init() == nil))
	for _, help := range []bool{false, true} {
		m.SetShowingHelp(help)
		for y := -1; y <= 13; y++ {
			var b strings.Builder
			for x := -1; x <= 65; x++ {
				if c, ok := m.HitTest(x, y); ok {
					fmt.Fprintf(&b, "%d,%d ", c[0], c[1])
				} else {
					b.WriteString("- ")
				}
			}
			w(fmt.Sprintf("hit/help=%v/%d", help, y), b.String())
		}
	}
	for _, v := range pgVariants() {
		w("theme/"+v.key, New(v.th, 0, 0).Theme().Name())
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
