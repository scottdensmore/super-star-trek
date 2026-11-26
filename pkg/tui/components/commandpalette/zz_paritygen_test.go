package commandpalette

// Parity reference generator for the Rust port
// (crates/sst-tui/src/components/commandpalette.rs).
// Run with: SST_PARITYGEN=1 go test ./pkg/tui/components/commandpalette -run TestParityGen -count=1
// Writes testdata/parity/components/commandpalette.txt. Every line is
// "<key>\t<%q of value>": views after each key of several key sequences,
// the message each command produces, and the palette accessors, for every
// theme x {dark, light} x {Ascii, TrueColor} (plus two ANSI256 combos).

import (
	"bufio"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/muesli/termenv"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

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

func pgMsg(msg tea.Msg) string {
	switch v := msg.(type) {
	case nil:
		return "nilmsg"
	case CommandSelectedMsg:
		return fmt.Sprintf("CommandSelectedMsg prefix=%q param=%v", v.CommandPrefix, v.Parameterized)
	case ClosePaletteMsg:
		return "ClosePaletteMsg"
	case tea.BatchMsg:
		return fmt.Sprintf("batch(%d)", len(v))
	}
	return fmt.Sprintf("%T", msg)
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

var pgSequences = [][]string{
	{"down", "down", "down", "up", "pgdown", "pgdown", "pgup", "end", "home", "enter", "esc"},
	{"p", "h", "a", "enter", "backspace", "backspace", "backspace", "backspace"},
	{"n", "a", "v", "space", "q", "backspace", "delete", "down", "enter"},
	{"t", "h", "e", "m", "e", "end", "up", "enter"},
	{"z", "z", "z", "z", "enter", "down", "end", "home"},
	{"🚀", "é", "漢", "backspace", "backspace", "backspace"},
	{"tab", "ctrl+x", "f1", "alt+a", "winsize", "shift+tab", "~", "/", "?"},
	{"s", "c", "a", "n", "pgdown", "pgup"},
}

var pgSizes = [][2]int{{56, 16}, {60, 20}, {40, 10}, {0, 0}, {80, 30}, {20, 5}, {12, 3}}

func pgItems(items []PaletteItem) string {
	var parts []string
	for _, it := range items {
		parts = append(parts, it.Title())
	}
	return strings.Join(parts, "|")
}

func pgState(w func(k, v string), p string, m Model) {
	w(p+"view", m.View())
	w(p+"filter", m.FilterValue())
	w(p+"visible", pgItems(m.VisibleItems()))
	sel := "nil"
	if s := m.SelectedItem(); s != nil {
		sel = s.Title()
	}
	w(p+"selected", sel)
}

func TestParityGen(t *testing.T) {
	if os.Getenv("SST_PARITYGEN") != "1" {
		t.Skip("set SST_PARITYGEN=1 to regenerate parity data")
	}
	out := filepath.Join("..", "..", "..", "..", "testdata", "parity", "components")
	if err := os.MkdirAll(out, 0o755); err != nil {
		t.Fatal(err)
	}
	f, err := os.Create(filepath.Join(out, "commandpalette.txt"))
	if err != nil {
		t.Fatal(err)
	}
	defer f.Close()
	bw := bufio.NewWriter(f)
	defer bw.Flush()
	w := func(k, v string) { fmt.Fprintf(bw, "%s\t%q\n", k, v) }

	lipgloss.SetHasDarkBackground(true)
	defer lipgloss.SetColorProfile(termenv.Ascii)

	// Catalog accessors (theme independent).
	{
		lipgloss.SetColorProfile(termenv.Ascii)
		m := New(theme.GetTheme("modern").WithColorMode(theme.ColorModeDark), 56, 16)
		for i, it := range m.Items() {
			w(fmt.Sprintf("catalog/%d", i), fmt.Sprintf("%s|%s|%s|%s|%v", it.Title(), it.Description(), it.FilterValue(), it.Prefix(), it.Parameterized()))
		}
	}

	for _, c := range pgCombos() {
		lipgloss.SetColorProfile(c.prof)
		th := theme.GetTheme(c.name).WithColorMode(c.mode)
		pre := fmt.Sprintf("%s/%s/%s/", c.name, c.mode, c.pname)

		full := c.prof == termenv.Ascii && c.mode == theme.ColorModeDark && c.name == theme.ThemeModern
		for zi, sz := range pgSizes {
			if !full && zi > 1 {
				break
			}
			m := New(th, sz[0], sz[1])
			w(fmt.Sprintf("%ssize/%dx%d", pre, sz[0], sz[1]), m.View())
		}
		for si, seq := range pgSequences {
			m := New(th, 56, 16)
			for ki, k := range seq {
				var cmd tea.Cmd
				m, cmd = m.Update(pgKey(k))
				p := fmt.Sprintf("%sseq%d/%d/%s/", pre, si, ki, k)
				w(p+"msg", pgCmd(cmd))
				if full || (ki == len(seq)-1 && si%2 == 0) {
					pgState(w, p, m)
				}
			}
		}
		// Reset, resize and re-theme a filtered palette.
		m := New(th, 56, 16)
		for _, k := range []string{"d", "o", "c"} {
			m, _ = m.Update(pgKey(k))
		}
		m.SetSize(70, 12)
		pgState(w, pre+"resized/", m)
		m.SetTheme(theme.GetTheme("crt").WithColorMode(theme.ColorModeLight))
		pgState(w, pre+"rethemed/", m)
		w(pre+"rethemed/theme", m.Theme().Name()+" "+string(m.Theme().ColorMode()))
		m.Reset()
		pgState(w, pre+"reset/", m)
		m.SetSize(0, -1)
		pgState(w, pre+"zero/", m)
	}
}
