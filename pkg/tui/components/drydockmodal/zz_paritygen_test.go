package drydockmodal

// Parity reference generator for the Rust port
// (crates/sst-tui/src/components/drydockmodal.rs).
// Run with: SST_PARITYGEN=1 go test ./pkg/tui/components/drydockmodal -run TestParityGen -count=1
// Writes testdata/parity/components/drydockmodal.txt ("<key>\t<%q value>"):
// seeded tours with various requisition budgets and installed refits, key
// sequences (navigation, purchases, errors, disembark) with the shared tour
// state, view and emitted message after each key.

import (
	"bufio"
	"fmt"
	"os"
	"path/filepath"
	"testing"
	"time"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/muesli/termenv"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

func pgMsg(msg tea.Msg) string {
	switch v := msg.(type) {
	case nil:
		return "nilmsg"
	case DisembarkMsg:
		return "DisembarkMsg"
	case RefitPurchasedMsg:
		return fmt.Sprintf("RefitPurchasedMsg id=%s tier=%d", v.RefitID, v.Tier)
	case tea.BatchMsg:
		return fmt.Sprintf("batch(%d)", len(v))
	}
	return fmt.Sprintf("%T", msg)
}

type pgTourCase struct {
	seed   int64
	req    int
	secs   int
	refits map[engine.RefitID]int
	nilMap bool
	width  int
}

var pgTours = []pgTourCase{
	{seed: 404, req: 2000, secs: 1},
	{seed: 405, req: 100, secs: 0, width: 60},
	{seed: 409, req: 1250, secs: 2, refits: map[engine.RefitID]int{engine.RefitDeflectorGrid: 1}},
	{seed: 777, req: 99999, secs: 5, refits: map[engine.RefitID]int{engine.RefitDilithiumCore: 3, engine.RefitTorpedoBays: 2, engine.RefitDamageNanites: 0}, width: 120},
	{seed: 12, req: 5000, secs: 3, nilMap: true, width: 1},
}

var pgSequences = [][]string{
	{"enter", "enter", "enter", "enter", "down", "enter", "j", "j", "enter", "k", "up", "up", "up", "enter"},
	{"down", "down", "down", "down", "down", "down", "down", "enter", "enter", "enter", "enter", "x", "space"},
	{"d", "D", "space", "esc", "tab", "up"},
}

func pgTourState(t *engine.TourState) string {
	if t == nil {
		return "tour=nil"
	}
	return fmt.Sprintf("req=%d secs=%d refits=%v", t.RequisitionPoints, t.SectorsCompleted, t.InstalledRefits)
}

func pgMakeTour(c pgTourCase) *engine.TourState {
	t := engine.NewTour(c.seed)
	t.InDrydock = true
	t.RequisitionPoints = c.req
	t.SectorsCompleted = c.secs
	for k, v := range c.refits {
		t.InstalledRefits[k] = v
	}
	if c.nilMap {
		t.InstalledRefits = nil
	}
	return t
}

func TestParityGen(t *testing.T) {
	w, done := pgOpen(t, "drydockmodal")
	defer done()

	for i, def := range engine.RefitCatalog {
		w(fmt.Sprintf("catalog/%d", i), fmt.Sprintf("%s|%s|%v", def.ID, def.Name, def.TierCosts))
	}

	for _, c := range pgCombos() {
		th, pre := c.apply()
		full := c.full()
		for ti, tc := range pgTours {
			if !full && ti != 2 {
				continue
			}
			tour := pgMakeTour(tc)
			m := New(tour, th)
			if tc.width != 0 {
				m.Width = tc.width
			}
			w(fmt.Sprintf("%stour%d/init", pre, ti), m.View())
			for si, seq := range pgSequences {
				if !full && si > 0 {
					break
				}
				for ki, k := range seq {
					var cmd tea.Cmd
					m, cmd = m.Update(pgKey(k))
					p := fmt.Sprintf("%stour%d/seq%d/%d/%s/", pre, ti, si, ki, k)
					w(p+"msg", pgCmd(cmd))
					w(p+"state", fmt.Sprintf("sel=%d err=%q %s", m.SelectedIndex(), m.errorMessage, pgTourState(m.Tour)))
					if (full && ti == 0 && si == 0) || ki == len(seq)-1 {
						w(p+"view", m.View())
					}
				}
			}
		}
		if full {
			m := New(nil, nil)
			w(pre+"nil/view", m.View())
			var cmd tea.Cmd
			m, cmd = m.Update(pgKey("enter"))
			w(pre+"nil/enter", pgCmd(cmd)+" "+m.errorMessage)
			m.SetTheme(th)
			w(pre+"nil/view2", m.View())
			m.SetTheme(nil)
			w(pre+"nil/theme", m.Theme.Name()+" "+string(m.Theme.ColorMode()))
			m.Tour = pgMakeTour(pgTours[0])
			m.selectedIndex = 99
			m, cmd = m.Update(pgKey("enter"))
			w(pre+"oob/enter", pgCmd(cmd)+" "+pgTourState(m.Tour))
			m.selectedIndex = -1
			m, cmd = m.Update(pgKey("enter"))
			w(pre+"neg/enter", pgCmd(cmd)+" "+pgTourState(m.Tour))
		}
	}
	_ = theme.ThemeModern
}
func pgKey(s string) tea.Msg {
	switch s {
	case "up":
		return tea.KeyMsg{Type: tea.KeyUp}
	case "down":
		return tea.KeyMsg{Type: tea.KeyDown}
	case "left":
		return tea.KeyMsg{Type: tea.KeyLeft}
	case "right":
		return tea.KeyMsg{Type: tea.KeyRight}
	case "enter":
		return tea.KeyMsg{Type: tea.KeyEnter}
	case "esc":
		return tea.KeyMsg{Type: tea.KeyEsc}
	case "tab":
		return tea.KeyMsg{Type: tea.KeyTab}
	case "shift+tab":
		return tea.KeyMsg{Type: tea.KeyShiftTab}
	case "pgup":
		return tea.KeyMsg{Type: tea.KeyPgUp}
	case "pgdown":
		return tea.KeyMsg{Type: tea.KeyPgDown}
	case "home":
		return tea.KeyMsg{Type: tea.KeyHome}
	case "end":
		return tea.KeyMsg{Type: tea.KeyEnd}
	case "backspace":
		return tea.KeyMsg{Type: tea.KeyBackspace}
	case "delete":
		return tea.KeyMsg{Type: tea.KeyDelete}
	case "space":
		return tea.KeyMsg{Type: tea.KeySpace}
	case "f1":
		return tea.KeyMsg{Type: tea.KeyF1}
	case "ctrl+u":
		return tea.KeyMsg{Type: tea.KeyCtrlU}
	case "ctrl+d":
		return tea.KeyMsg{Type: tea.KeyCtrlD}
	case "ctrl+x":
		return tea.KeyMsg{Type: tea.KeyCtrlX}
	case "alt+a":
		return tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune("a"), Alt: true}
	case "winsize":
		return tea.WindowSizeMsg{Width: 120, Height: 40}
	}
	return tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune(s)}
}

func pgCmd(cmd tea.Cmd) string {
	if cmd == nil {
		return "nil"
	}
	ch := make(chan tea.Msg, 1)
	go func() { ch <- cmd() }()
	select {
	case msg := <-ch:
		return pgMsg(msg)
	case <-time.After(100 * time.Millisecond):
		return "timeout"
	}
}

type pgCombo struct {
	name  string
	mode  theme.ColorMode
	pname string
	prof  termenv.Profile
}

func pgCombos() []pgCombo {
	var out []pgCombo
	for _, n := range []string{theme.ThemeModern, theme.ThemeLcars, theme.ThemeCrt} {
		for _, md := range []theme.ColorMode{theme.ColorModeDark, theme.ColorModeLight} {
			out = append(out, pgCombo{n, md, "Ascii", termenv.Ascii}, pgCombo{n, md, "TrueColor", termenv.TrueColor})
		}
	}
	out = append(out, pgCombo{theme.ThemeModern, theme.ColorModeDark, "ANSI256", termenv.ANSI256},
		pgCombo{theme.ThemeLcars, theme.ColorModeLight, "ANSI256", termenv.ANSI256})
	return out
}

func (c pgCombo) full() bool {
	return c.prof == termenv.Ascii && c.mode == theme.ColorModeDark && c.name == theme.ThemeModern
}

func (c pgCombo) apply() (theme.Theme, string) {
	lipgloss.SetHasDarkBackground(true)
	lipgloss.SetColorProfile(c.prof)
	return theme.GetTheme(c.name).WithColorMode(c.mode), fmt.Sprintf("%s/%s/%s/", c.name, c.mode, c.pname)
}

// pgOpen creates testdata/parity/components/<name>.txt and returns the
// line writer plus a flush/close function.
func pgOpen(t *testing.T, name string) (func(k, v string), func()) {
	if os.Getenv("SST_PARITYGEN") != "1" {
		t.Skip("set SST_PARITYGEN=1 to regenerate parity data")
	}
	out := filepath.Join("..", "..", "..", "..", "testdata", "parity", "components")
	if err := os.MkdirAll(out, 0o755); err != nil {
		t.Fatal(err)
	}
	f, err := os.Create(filepath.Join(out, name+".txt"))
	if err != nil {
		t.Fatal(err)
	}
	bw := bufio.NewWriter(f)
	return func(k, v string) { fmt.Fprintf(bw, "%s\t%q\n", k, v) }, func() {
		bw.Flush()
		f.Close()
		lipgloss.SetColorProfile(termenv.Ascii)
	}
}
