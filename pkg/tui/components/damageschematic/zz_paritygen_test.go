package damageschematic

// Parity reference generator for the Rust port
// (crates/sst-tui/src/components/damageschematic). Run with:
//   SST_PARITYGEN=1 go test ./pkg/tui/components/damageschematic -run TestParityGen -count=1
// Writes testdata/parity/components/damageschematic.txt. Every line is
// "<key>\t<%q of value>": View for device damage patterns, conditions,
// docking, repair multipliers and sizes (including truncation), Update
// dismissal keys, across themes x {dark, light} x profiles.

import (
	"bufio"
	"fmt"
	"os"
	"path/filepath"
	"testing"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/muesli/termenv"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

func TestParityGen(t *testing.T) {
	if os.Getenv("SST_PARITYGEN") != "1" {
		t.Skip("set SST_PARITYGEN=1 to regenerate parity data")
	}
	out := filepath.Join("..", "..", "..", "..", "testdata", "parity", "components")
	if err := os.MkdirAll(out, 0o755); err != nil {
		t.Fatal(err)
	}
	f, err := os.Create(filepath.Join(out, "damageschematic.txt"))
	if err != nil {
		t.Fatal(err)
	}
	defer f.Close()
	bw := bufio.NewWriter(f)
	defer bw.Flush()
	w := func(k, v string) { fmt.Fprintf(bw, "%s\t%q\n", k, v) }
	lipgloss.SetHasDarkBackground(true)
	defer lipgloss.SetColorProfile(termenv.Ascii)

	type dcase struct {
		name   string
		devs   [8]float64
		cond   string
		docked bool
		mult   float64
	}
	cases := []dcase{
		{"nominal", [8]float64{}, "GREEN", false, 1},
		{"empty-cond", [8]float64{}, "", false, 1},
		{"docked-flag", [8]float64{}, "RED", true, 1},
		{"two", [8]float64{0, 0, 0, 0, 1.4, 0, 0, 2.1}, "YELLOW", false, 1},
		{"warp-mult", [8]float64{2.0}, "RED", false, 1.5},
		{"three", [8]float64{0.5, 0, 3, 0, 0, 1.99, 0, 0}, "YELLOW", false, 0.25},
		{"four", [8]float64{1, 1, 1, 1, 0, 0, 0, 0}, "GREEN", true, 1},
		{"five", [8]float64{1, 1, 1, 1, 1, 0, 0, 0}, "GREEN", true, 1},
		{"all", [8]float64{0.1, 0.2, 2.3, 4.4, 5.5, 6.6, 7.7, 8.8}, "RED", false, 2},
		{"neg-mult", [8]float64{0, 0, 0, 0, 0, 1.0, 0, 0}, "GREEN", false, -1},
		{"zero-mult", [8]float64{0, 3.25}, "DOCKED", false, 0},
		{"negative-dev", [8]float64{-1, 0, -0.5, 0, 0, 0, 0, 0.05}, "GREEN", false, 1},
		{"huge", [8]float64{0, 0, 0, 1234.56, 0, 0, 99999.9, 0}, "A VERY LONG ALERT CONDITION STRING THAT OVERFLOWS", false, 3},
	}
	for i, c := range cases {
		each := pgRot
		if i == 0 || i == 3 || i == 12 {
			each = pgEach
		}
		each(w, "case/"+c.name, func(th theme.Theme) string {
			m := New(th, 66, 18)
			var ent engine.EnterpriseState
			ent.Devices = c.devs
			m.SetState(ent, c.cond, c.docked, c.mult)
			return m.View()
		})
	}
	sizes := [][2]int{{0, 0}, {80, 20}, {50, 10}, {30, 5}, {2, 2}, {1, 1}, {-5, -5}}
	for _, sz := range sizes {
		pgLite(w, fmt.Sprintf("size/%dx%d", sz[0], sz[1]), func(th theme.Theme) string {
			m := New(th, sz[0], sz[1])
			var ent engine.EnterpriseState
			ent.Devices = cases[8].devs
			m.SetState(ent, "YELLOW", false, 1)
			return m.View()
		})
	}
	pgLite(w, "settheme-nil", func(th theme.Theme) string {
		m := New(th, 66, 18)
		m.SetTheme(nil)
		return m.View()
	})
	pgLite(w, "settheme", func(th theme.Theme) string {
		m := New(pgVariants()[5].th, 66, 18)
		m.SetTheme(th)
		return m.View()
	})
	pgLite(w, "unset", func(th theme.Theme) string { return New(th, 66, 18).View() })

	m := New(pgVariants()[0].th, 66, 18)
	keys := []string{"esc", "enter", "q", "Q", "d", "D", " ", "space", "x", "tab", "up", "ctrl+a", "alt+b", "f2", "dd"}
	for _, k := range keys {
		_, cmd := m.Update(pgKey(k))
		res := "nil"
		if cmd != nil {
			if _, ok := cmd().(CloseModalMsg); ok {
				res = "close"
			} else {
				res = "?"
			}
		}
		w("key/"+k, res)
	}
	for _, msg := range []tea.Msg{tea.WindowSizeMsg{Width: 3, Height: 3}, tea.MouseMsg{Button: tea.MouseButtonLeft}, struct{}{}} {
		_, cmd := m.Update(msg)
		w(fmt.Sprintf("msg/%T", msg), fmt.Sprint(cmd == nil))
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
