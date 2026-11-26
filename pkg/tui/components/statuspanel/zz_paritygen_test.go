package statuspanel

// Parity reference generator for the Rust port
// (crates/sst-tui/src/components/statuspanel). Run with:
//   SST_PARITYGEN=1 go test ./pkg/tui/components/statuspanel -run TestParityGen -count=1
// Writes testdata/parity/components/statuspanel.txt. Every line is
// "<key>\t<%q of value>": View(game) for seeded games (hazard alerts,
// damage, docking, red alert pulse), audio telemetry setters, Render of
// PanelData variants (rules defaults, device overrides, odd values),
// SetState and the no-telemetry view, across themes x {dark, light} x profiles.

import (
	"bufio"
	"fmt"
	"math"
	"os"
	"path/filepath"
	"testing"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/muesli/termenv"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

var _ = pgKey

func pgTelemetry(m Model) string {
	return fmt.Sprintf("vol=%d muted=%v sound=%v", m.AudioVolume(), m.AudioMuted(), m.SoundEnabled())
}

func TestParityGen(t *testing.T) {
	if os.Getenv("SST_PARITYGEN") != "1" {
		t.Skip("set SST_PARITYGEN=1 to regenerate parity data")
	}
	out := filepath.Join("..", "..", "..", "..", "testdata", "parity", "components")
	if err := os.MkdirAll(out, 0o755); err != nil {
		t.Fatal(err)
	}
	f, err := os.Create(filepath.Join(out, "statuspanel.txt"))
	if err != nil {
		t.Fatal(err)
	}
	defer f.Close()
	bw := bufio.NewWriter(f)
	defer bw.Flush()
	w := func(k, v string) { fmt.Fprintf(bw, "%s\t%q\n", k, v) }
	lipgloss.SetHasDarkBackground(true)
	defer lipgloss.SetColorProfile(termenv.Ascii)

	games := pgGames()
	for _, gm := range games {
		g := gm.g
		each := pgRot
		if gm.name == "hazards" || gm.name == "damaged" {
			each = pgEach
		}
		each(w, "game/"+gm.name, func(th theme.Theme) string { return New(th).View(g) })
	}

	// Audio telemetry setters and red alert cycles on one model.
	g1 := games[0].g
	dmg := games[len(games)-4].g
	m := New(nil)
	steps := []struct {
		name string
		f    func()
	}{
		{"init", func() {}},
		{"sound-off", func() { m.SetSoundEnabled(false) }},
		{"sound-on", func() { m.SetSoundEnabled(true) }},
		{"vol0", func() { m.SetAudioTelemetry(0, false) }},
		{"vol35", func() { m.SetAudioTelemetry(35, false) }},
		{"vol100-muted", func() { m.SetAudioTelemetry(100, true) }},
		{"vol-neg", func() { m.SetAudioTelemetry(-5, false) }},
		{"vol55", func() { m.SetAudioTelemetry(55, false) }},
	}
	for _, st := range steps {
		st.f()
		w("audio/"+st.name, pgTelemetry(m))
		pgRot(w, "audio/"+st.name+"/game", func(th theme.Theme) string { m.SetTheme(th); return m.View(g1) })
		pgRot(w, "audio/"+st.name+"/nogame", func(th theme.Theme) string { m.SetTheme(th); return m.View() })
	}
	for cycle := 0; cycle < 4; cycle++ {
		m.SetRedAlertCycle(cycle)
		pgRot(w, fmt.Sprintf("redalert/%d", cycle), func(th theme.Theme) string { m.SetTheme(th); return m.View(dmg) })
	}
	m.SetHazardAlerts([]string{"custom alert one", "⚠️ two"})
	pgLite(w, "hazard-setter", func(th theme.Theme) string { m.SetTheme(th); return m.View() })

	// Render(PanelData) variants.
	type pd struct {
		name string
		f    func(d *PanelData)
	}
	datas := []pd{
		{"sample", func(d *PanelData) {}},
		{"lrs1", func(d *PanelData) { d.Devices[engine.DeviceLRSensors] = 1.0 }},
		{"lrs2.5", func(d *PanelData) { d.Devices[engine.DeviceLRSensors] = 2.5 }},
		{"lrs2.5-docked", func(d *PanelData) { d.Devices[engine.DeviceLRSensors] = 2.5; d.IsDocked = true }},
		{"rules-zero", func(d *PanelData) { d.Rules = engine.GameRules{}; d.Devices[engine.DeviceLRSensors] = 1.0 }},
		{"rules-zero-degr", func(d *PanelData) {
			d.Rules = engine.GameRules{SensorDegradation: true, AnimSpeed: engine.AnimSpeedOff}
			d.Devices[engine.DeviceLRSensors] = 1.0
			d.Enterprise.Condition = engine.ConditionRed
		}},
		{"no-degr", func(d *PanelData) {
			d.Rules.SensorDegradation = false
			d.Devices[engine.DeviceLRSensors] = 1.5
		}},
		{"ent-devices", func(d *PanelData) {
			d.Enterprise.Devices[engine.DeviceWarp] = 9.99
			d.Enterprise.Devices[engine.DeviceComputer] = 12.345
		}},
		{"both-devices", func(d *PanelData) {
			d.Enterprise.Devices[engine.DeviceWarp] = 9.99
			d.Devices[engine.DevicePhasers] = 0.04
			d.Devices[engine.DeviceShields] = -1
		}},
		{"audio-neg", func(d *PanelData) { d.AudioVolume = -1; d.SoundEnabled = false }},
		{"audio-neg-muted", func(d *PanelData) { d.AudioVolume = -1; d.AudioMuted = true }},
		{"audio-0", func(d *PanelData) { d.AudioVolume = 0 }},
		{"alerts", func(d *PanelData) { d.HazardAlerts = []string{"A", "", "⚠️ B ⚠️"} }},
		{"cond-odd", func(d *PanelData) { d.Enterprise.Condition = engine.ConditionType(9) }},
		{"cond-yellow", func(d *PanelData) { d.Enterprise.Condition = engine.ConditionYellow }},
		{"cond-docked", func(d *PanelData) { d.Enterprise.Condition = engine.ConditionDocked }},
		{"cond-red", func(d *PanelData) { d.Enterprise.Condition = engine.ConditionRed }},
		{"energy-odd", func(d *PanelData) {
			d.Enterprise.Energy = math.NaN()
			d.Enterprise.Shields = math.Inf(1)
			d.Stardate = math.Inf(-1)
			d.TimeRemaining = -0.04
		}},
		{"energy-big", func(d *PanelData) {
			d.Enterprise.Energy = 123456.78
			d.Enterprise.Shields = -250
			d.Enterprise.Torpedoes = -3
			d.Stardate = 2525.25
			d.TimeRemaining = 0.05
		}},
		{"corner", func(d *PanelData) {
			d.Enterprise.Quad = engine.Coord{1, 8}
			d.Enterprise.Sector = engine.Coord{8, 1}
			for r := 0; r < 9; r++ {
				for c := 0; c < 9; c++ {
					d.GalaxyChart[r][c] = r*100 + c*10 + (r+c)%10
				}
			}
		}},
		{"offgrid", func(d *PanelData) {
			d.Enterprise.Quad = engine.Coord{0, 9}
			d.GalaxyChart[1][8] = 7
		}},
	}
	for _, d := range datas {
		data := SamplePanelData()
		d.f(&data)
		pgRot(w, "render/"+d.name, func(th theme.Theme) string { return New(th).Render(data) })
	}
	data := SamplePanelData()
	data.Devices[engine.DeviceLRSensors] = 1.0
	pgEach(w, "render/lrs1-full", func(th theme.Theme) string {
		mm := New(th, 30, 16)
		mm.SetRedAlertCycle(1)
		return mm.Render(data)
	})
	// Render does not mutate the model (value receiver).
	pgLite(w, "render/after", func(th theme.Theme) string {
		mm := New(th)
		_ = mm.Render(data)
		return mm.View()
	})

	// SetState with and without rules, on New and on a zero model.
	ent := engine.Enterprise{Quad: engine.Coord{4, 4}, Sector: engine.Coord{2, 3}, Energy: 4500, Shields: 1000, Torpedoes: 8,
		Condition: engine.ConditionGreen}
	ent.Devices[engine.DeviceLRSensors] = 2.5
	var chart [9][9]int
	chart[4][4] = 105
	chart[3][4] = 203
	pgLite(w, "setstate/new", func(th theme.Theme) string {
		mm := New(th, 30, 16)
		mm.SetState(ent, 25.0, 10, 3, false, chart)
		return mm.View()
	})
	pgLite(w, "setstate/docked", func(th theme.Theme) string {
		mm := New(th)
		mm.SetState(ent, 25.0, 10, 3, true, chart)
		return mm.View(nil)
	})
	pgLite(w, "setstate/zero", func(th theme.Theme) string {
		var mm Model
		mm.SetTheme(th)
		mm.SetState(ent, 1.5, 0, 0, false, chart)
		return mm.View()
	})
	pgLite(w, "setstate/rules", func(th theme.Theme) string {
		var mm Model
		mm.SetTheme(th)
		r := engine.DefaultRulesForProfile(engine.ProfileCasual)
		r.SensorDegradation = false
		e2 := ent
		e2.Devices[engine.DeviceLRSensors] = 1.5
		e2.Condition = engine.ConditionRed
		mm.SetState(e2, 1.5, 0, 0, false, chart, r)
		return mm.View()
	})
	pgEach(w, "notelemetry", func(th theme.Theme) string {
		var mm Model
		mm.SetTheme(th)
		return mm.View()
	})
	for _, th := range pgVariants() {
		mm := New(th.th)
		w("theme/"+th.key, mm.Theme().Name()+" "+pgTelemetry(mm))
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
