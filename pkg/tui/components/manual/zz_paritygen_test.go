package manual

// Parity reference generator for the Rust port
// (crates/sst-tui/src/components/manual.rs).
// Run with: SST_PARITYGEN=1 go test ./pkg/tui/components/manual -run TestParityGen -count=1
// Writes testdata/parity/components/manual.txt ("<key>\t<%q value>"): every
// chapter, scroll/focus key sequences with the state after each key,
// SelectChapter topics and the messages emitted, per theme combo.

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
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

func pgMsg(msg tea.Msg) string {
	switch v := msg.(type) {
	case nil:
		return "nilmsg"
	case CloseModalMsg:
		return "CloseModalMsg"
	case tea.BatchMsg:
		return fmt.Sprintf("batch(%d)", len(v))
	}
	return fmt.Sprintf("%T", msg)
}

var pgSequences = [][]string{
	{"down", "j", "k", "up", "up", "down", "5", "1", "8", "9", "0", "enter"},
	{"tab", "down", "j", "end", "down", "pgup", "b", "home", "pgdown", "space", "ctrl+d", "G", "g", "ctrl+u", "x", "esc", "esc"},
	{"3", "right", "pgdown", "pgdown", "pgdown", "pgdown", "left", "down", "l", "up", "h", "shift+tab", "k", "tab"},
	{"q", "Q", "?", "f1", "winsize", "alt+a", "6", "l", "end", "7", "end", "4", "end", "2", "end"},
}

var pgTopics = []string{"", "  ", "combat", "COMBAT", "7", "9", "0", "unknown", "nav", " Nav ", "systems", "warp", "tor", "doc", "cloak", "points", "guide", "help nav", "shield", "commands", "Scoring", "12"}

func pgState(w func(k, v string), p string, m Model) {
	w(p+"view", m.View())
	w(p+"state", fmt.Sprintf("sel=%d focus=%d off=%v", m.selectedIdx, m.focus, m.scrollOffsets))
}

func TestParityGen(t *testing.T) {
	w, done := pgOpen(t, "manual")
	defer done()

	for _, c := range pgCombos() {
		th, pre := c.apply()
		full := c.full()
		for i := 0; i < 8; i++ {
			if !full && i != 2 {
				continue
			}
			m := New(th, 66, 18)
			m.selectedIdx = i
			w(fmt.Sprintf("%schapter/%d", pre, i), m.View())
			m.focus = FocusContent
			m.scrollOffsets[i] = 1000
			w(fmt.Sprintf("%schapter/%d/bottom", pre, i), m.View())
		}
		for si, seq := range pgSequences {
			m := New(th, 80, 24)
			for ki, k := range seq {
				var cmd tea.Cmd
				m, cmd = m.Update(pgKey(k))
				p := fmt.Sprintf("%sseq%d/%d/%s/", pre, si, ki, k)
				w(p+"msg", pgCmd(cmd))
				if full || ki == len(seq)-1 {
					pgState(w, p, m)
				} else {
					w(p+"state", fmt.Sprintf("sel=%d focus=%d off=%v", m.selectedIdx, m.focus, m.scrollOffsets))
				}
			}
		}
		if full {
			m := New(th, 0, 0)
			for _, tp := range pgTopics {
				m.SelectChapter(tp)
				w(fmt.Sprintf("%stopic/%q", pre, tp), fmt.Sprintf("%d", m.selectedIdx))
			}
			// Out-of-range state is clamped by Update and blank in View.
			m.selectedIdx = 42
			w(pre+"oob/view", m.View())
			m, _ = m.Update(pgKey("down"))
			pgState(w, pre+"oob/", m)
			m.selectedIdx = -3
			m, _ = m.Update(pgKey("x"))
			pgState(w, pre+"neg/", m)
			m.SetTheme(nil)
			w(pre+"niltheme", m.theme.Name()+" "+string(m.theme.ColorMode()))
		}
		for i, ch := range New(th, 66, 18).chapters {
			if !full {
				break
			}
			w(fmt.Sprintf("%smeta/%d", pre, i), fmt.Sprintf("%s|%d|%s|%s|%d", ch.ID, ch.Number, ch.Title, ch.ShortTag, len(ch.Lines)))
		}
	}
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
