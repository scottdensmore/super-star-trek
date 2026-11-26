package commandbar

// Parity reference generator for the Rust port
// (crates/sst-tui/src/components/commandbar). Run with:
//   SST_PARITYGEN=1 go test ./pkg/tui/components/commandbar -run TestParityGen -count=1
// Writes testdata/parity/components/commandbar.txt. Every line is
// "<key>\t<%q of value>": scripted sessions of key presses (typing, line
// editing, history, tab completion, submit), log messages, widths, visual
// bell, focus/blur and theme changes; after each step the input state, the
// emitted command and the View, across themes x {dark, light} x profiles.

import (
	"bufio"
	"fmt"
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"testing"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/muesli/termenv"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

func pgState(m Model) string {
	return fmt.Sprintf("val=%q pos=%d focused=%v width=%d/%d bell=%v msgs=%q", m.Value(), m.Position(), m.Focused(),
		m.Width(), m.Model.Width, m.VisualBell(), m.Messages())
}

// pgStep applies one scripted action and returns a description of the
// resulting command.
func pgStep(m *Model, act string) string {
	switch {
	case strings.HasPrefix(act, "@add:"):
		m.AddMessage(act[5:])
		return "-"
	case act == "@clear":
		m.ClearMessages()
		return "-"
	case strings.HasPrefix(act, "@width:"):
		n, _ := strconv.Atoi(act[7:])
		m.SetWidth(n)
		return "-"
	case act == "@bell:1":
		m.SetVisualBell(true)
		return "-"
	case act == "@bell:0":
		m.SetVisualBell(false)
		return "-"
	case act == "@blur":
		m.Blur()
		return "-"
	case act == "@focus":
		if m.Focus() == nil {
			return "nil"
		}
		return "cmd"
	case strings.HasPrefix(act, "@set:"):
		m.SetValue(act[5:])
		return "-"
	case act == "@reset":
		m.Reset()
		return "-"
	case strings.HasPrefix(act, "@paste:"):
		var cmd tea.Cmd
		*m, cmd = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune(act[7:]), Paste: true})
		if cmd == nil {
			return "nil"
		}
		return "cmd"
	case act == "@winsize":
		var cmd tea.Cmd
		*m, cmd = m.Update(tea.WindowSizeMsg{Width: 100, Height: 40})
		if cmd == nil {
			return "nil"
		}
		return "cmd"
	case act == "@mouse":
		var cmd tea.Cmd
		*m, cmd = m.Update(tea.MouseMsg{X: 3, Y: 4, Button: tea.MouseButtonLeft, Action: tea.MouseActionPress})
		if cmd == nil {
			return "nil"
		}
		return "cmd"
	}
	var cmd tea.Cmd
	*m, cmd = m.Update(pgKey(act))
	if cmd == nil {
		return "nil"
	}
	if act == "enter" {
		if sub, ok := cmd().(CommandSubmittedMsg); ok {
			return fmt.Sprintf("submit %q", sub.Text)
		}
		return "?"
	}
	return "cmd"
}

func TestParityGen(t *testing.T) {
	if os.Getenv("SST_PARITYGEN") != "1" {
		t.Skip("set SST_PARITYGEN=1 to regenerate parity data")
	}
	out := filepath.Join("..", "..", "..", "..", "testdata", "parity", "components")
	if err := os.MkdirAll(out, 0o755); err != nil {
		t.Fatal(err)
	}
	f, err := os.Create(filepath.Join(out, "commandbar.txt"))
	if err != nil {
		t.Fatal(err)
	}
	defer f.Close()
	bw := bufio.NewWriter(f)
	defer bw.Flush()
	w := func(k, v string) { fmt.Fprintf(bw, "%s\t%q\n", k, v) }
	lipgloss.SetHasDarkBackground(true)
	defer lipgloss.SetColorProfile(termenv.Ascii)

	sessions := [][]string{
		// Typing, submit, history and draft restore.
		{"n", "a", "v", " ", "1", " ", "2", "enter", "t", "o", "r", "enter", "s", "t", "a", "t", "u", "s", "enter",
			"@set:status", "enter", "@set:  status  ", "enter", "p", "h", "up", "up", "up", "up", "down", "down", "down",
			"down", "up", "x", "down", "enter", "enter", "@set:   ", "enter", "down", "up", "up"},
		// Tab completion cycling and resets.
		{"@set:s", "tab", "tab", "tab", "tab", "tab", "x", "@set:doc", "tab", "tab", "@set:xyz", "tab", "@set:", "tab",
			"@set:  T ", "tab", "tab", "tab", "tab", "tab", "left", "tab", "@set:pha ", "tab", "tab", "@set:N", "tab",
			"up", "tab", "@set:she", "tab", "enter", "@set:s", "tab", "down", "tab"},
		// Line editing and scrolling in a narrow bar.
		{"@width:30", "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o", "p", "q", "r", "s", "t",
			"u", "v", "w", "x", "y", "z", "0", "1", "2", "3", "home", "right", "right", "delete", "end", "left", "left",
			"backspace", "ctrl+a", "ctrl+k", "@set:one two three", "ctrl+w", "alt+b", "ctrl+u", "★", "é", "@paste:pasted text",
			"ctrl+e", "space", "f2", "@winsize", "@mouse", "enter"},
		// Log messages, widths, visual bell, focus.
		{"@add:Enterprise systems online.", "@add:Sensors detect 3 Klingons.\nShields up.", "@add:\r\n", "@add:CRLF line\r\n",
			"@add:fifth\nsixth\nseventh", "@width:80", "@bell:1", "@width:0", "@width:3", "@width:4", "@width:5", "@width:-2",
			"@width:120", "@bell:0", "@blur", "a", "@focus", "b", "@add:" + strings.Repeat("long message ", 12), "@width:40",
			"@clear", "@add:after clear", "@reset", "@blur", "@bell:1"},
	}
	for si, sess := range sessions {
		m := New(pgVariants()[0].th)
		for i, act := range sess {
			res := pgStep(&m, act)
			key := fmt.Sprintf("s%d/%02d", si, i)
			w(key+"/state", fmt.Sprintf("act=%q cmd=%s %s", act, res, pgState(m)))
			pgRot(w, key+"/view", func(th theme.Theme) string { m.SetTheme(th); return m.View() })
		}
		pgEach(w, fmt.Sprintf("s%d/final", si), func(th theme.Theme) string { m.SetTheme(th); return m.View() })
	}

	// Constructors and themes.
	for _, v := range pgVariants() {
		m := New(v.th)
		a := NewCommandBar(v.th)
		w("new/"+v.key, fmt.Sprintf("%s %s prompt=%q alias=%q", m.Theme().Name(), pgState(m), m.Prompt, a.Prompt))
	}
	pgEach(w, "new/view", func(th theme.Theme) string {
		m := New(th)
		m.AddMessage("Super Star Trek — Tactical Display Online")
		m.AddMessage("Type 'help' for commands, 'quit' to exit, F2 to cycle themes")
		m.SetWidth(80)
		return m.View()
	})
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
