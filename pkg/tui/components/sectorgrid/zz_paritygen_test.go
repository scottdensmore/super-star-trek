package sectorgrid

// Parity reference generator for the Rust port
// (crates/sst-tui/src/components/sectorgrid). Run with:
//   SST_PARITYGEN=1 go test ./pkg/tui/components/sectorgrid -run TestParityGen -count=1
// Writes testdata/parity/components/sectorgrid.txt. Every line is
// "<key>\t<%q of value>": View/ViewWithGame renders of seeded games across
// themes x {dark, light} x colour profiles, reticle selections of every
// entity type, cloaked contacts, animation overrides and HitTest.

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
	"github.com/scottdensmore/super-star-trek/pkg/tui/anim"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

var _ = pgKey

func pgFirstSel(q *engine.QuadrantState) engine.Coord {
	for r := 1; r <= 8; r++ {
		for c := 1; c <= 8; c++ {
			if e := q.Grid[r][c]; e != engine.EntityEmpty && e != engine.EntityEnterprise {
				return engine.Coord{r, c}
			}
		}
	}
	return engine.Coord{4, 4}
}

func TestParityGen(t *testing.T) {
	if os.Getenv("SST_PARITYGEN") != "1" {
		t.Skip("set SST_PARITYGEN=1 to regenerate parity data")
	}
	out := filepath.Join("..", "..", "..", "..", "testdata", "parity", "components")
	if err := os.MkdirAll(out, 0o755); err != nil {
		t.Fatal(err)
	}
	f, err := os.Create(filepath.Join(out, "sectorgrid.txt"))
	if err != nil {
		t.Fatal(err)
	}
	defer f.Close()
	bw := bufio.NewWriter(f)
	defer bw.Flush()
	w := func(k, v string) { fmt.Fprintf(bw, "%s\t%q\n", k, v) }
	lipgloss.SetHasDarkBackground(true)
	defer lipgloss.SetColorProfile(termenv.Ascii)

	full := map[string]bool{"hot42": true, "hazards": true}
	for gi, gm := range pgGames() {
		g := gm.g
		each := pgRot
		if full[gm.name] {
			each = pgEach
		}
		each(w, "game/"+gm.name+"/sel0", func(th theme.Theme) string {
			return New(th).View(&g.CurrentQuad, g.Enterprise.Sector, engine.Coord{})
		})
		sel := pgFirstSel(&g.CurrentQuad)
		pgRot(w, fmt.Sprintf("game/%s/sel%d%d", gm.name, sel[0], sel[1]), func(th theme.Theme) string {
			return New(th).View(&g.CurrentQuad, g.Enterprise.Sector, sel)
		})
		if gi < 6 {
			pgRot(w, "game/"+gm.name+"/withgame", func(th theme.Theme) string {
				return New(th).ViewWithGame(g)
			})
		}
	}

	// Every entity type, unselected and selected.
	var all engine.QuadrantState
	ents := []engine.EntityType{engine.EntityEmpty, engine.EntityEnterprise, engine.EntityKlingon, engine.EntityCommander,
		engine.EntitySuperCommander, engine.EntityStarbase, engine.EntityStar, engine.EntityPlanet, engine.EntityBlackHole,
		engine.EntityWormhole, engine.EntityRomulan, engine.EntityTholian, engine.EntityPlasmaTorpedo, engine.EntityTholianWeb,
		engine.EntityType(42)}
	for i, e := range ents {
		all.Grid[1+i/8][1+i%8] = e
	}
	all.Grid[4][1] = engine.EntityCommander
	all.Grid[4][2] = engine.EntityRomulan
	all.Klingons = []*engine.Klingon{nil, {ID: 1, Sector: engine.Coord{4, 1}, IsCloaked: true}}
	all.Enemies = []*engine.EnemyVessel{nil, {ID: 2, Faction: engine.FactionRomulan, Sector: engine.Coord{4, 2}, IsCloaked: true},
		{ID: 3, Sector: engine.Coord{1, 2}, IsCloaked: false}}
	pgEach(w, "all/sel0", func(th theme.Theme) string { return New(th).View(&all, engine.Coord{6, 6}, engine.Coord{}) })
	for i := range ents {
		sel := engine.Coord{1 + i/8, 1 + i%8}
		pgRot(w, fmt.Sprintf("all/sel%d%d", sel[0], sel[1]), func(th theme.Theme) string {
			return New(th).View(&all, engine.Coord{6, 6}, sel)
		})
	}
	for _, sel := range []engine.Coord{{4, 1}, {4, 2}, {6, 6}, {0, 3}, {9, 9}} {
		pgRot(w, fmt.Sprintf("all/sel%d%d", sel[0], sel[1]), func(th theme.Theme) string {
			return New(th).View(&all, engine.Coord{6, 6}, sel)
		})
	}
	// Enterprise over a cloaked contact, off-grid Enterprise, nil quadrant.
	pgLite(w, "all/ent-on-cloak", func(th theme.Theme) string { return New(th).View(&all, engine.Coord{4, 1}, engine.Coord{4, 1}) })
	pgLite(w, "all/ent-offgrid", func(th theme.Theme) string { return New(th).View(&all, engine.Coord{0, 9}, engine.Coord{}) })
	pgEach(w, "nilquad", func(th theme.Theme) string { return New(th).View(nil, engine.Coord{3, 4}, engine.Coord{5, 5}) })
	pgLite(w, "nilgame", func(th theme.Theme) string { return New(th).ViewWithGame(nil) })

	// Animation overrides (and theme switching on one model).
	ov := map[engine.Coord]anim.CellOverride{
		{1, 1}: {Glyph: "", Style: lipgloss.NewStyle()},
		{2, 2}: {Glyph: "*", Style: lipgloss.NewStyle().Foreground(lipgloss.Color("9"))},
		{3, 3}: {Glyph: "**", Style: lipgloss.NewStyle().Bold(true)},
		{4, 4}: {Glyph: "***", Style: lipgloss.NewStyle().Foreground(lipgloss.Color("#FF8800")).Background(lipgloss.Color("#001122"))},
		{5, 5}: {Glyph: "OVERLONG", Style: lipgloss.NewStyle().Reverse(true)},
		{6, 6}: {Glyph: "★☆✦✧", Style: lipgloss.NewStyle().Foreground(lipgloss.Color("201"))},
		{8, 8}: {Glyph: " \\ ", Style: lipgloss.NewStyle().Foreground(lipgloss.Color("#00FF00"))},
	}
	g0 := pgGames()[1].g
	pgEach(w, "anim/overrides", func(th theme.Theme) string {
		m := New(nil)
		m.SetTheme(th)
		m.SetAnimOverrides(ov)
		return m.View(&g0.CurrentQuad, g0.Enterprise.Sector, engine.Coord{2, 2})
	})
	pgLite(w, "anim/cleared", func(th theme.Theme) string {
		m := New(th)
		m.SetAnimOverrides(ov)
		m.ClearAnimOverrides()
		return m.View(&g0.CurrentQuad, g0.Enterprise.Sector, engine.Coord{2, 2})
	})
	tor := anim.NewTorpedoAnimation(engine.Coord{1, 1}, engine.Coord{8, 7}, true, engine.AnimSpeedFast)
	for i, fr := range tor.Frames() {
		if i > 3 {
			break
		}
		pgRot(w, fmt.Sprintf("anim/torpedo%d", i), func(th theme.Theme) string {
			m := New(th)
			m.SetAnimOverrides(fr.Overrides)
			return m.View(&g0.CurrentQuad, g0.Enterprise.Sector, engine.Coord{})
		})
	}

	for _, th := range pgVariants() {
		w("theme/"+th.key, New(th.th).Theme().Name())
	}
	for y := -2; y <= 12; y++ {
		var b strings.Builder
		for x := -2; x <= 40; x++ {
			c, ok := New(nil).HitTest(x, y)
			if ok {
				fmt.Fprintf(&b, "%d,%d ", c[0], c[1])
			} else {
				fmt.Fprintf(&b, "- ")
			}
		}
		w(fmt.Sprintf("hit/%d", y), b.String())
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
