package savebrowser

// Parity reference generator for the Rust port
// (crates/sst-tui/src/components/savebrowser.rs).
// Run with: SST_PARITYGEN=1 go test ./pkg/tui/components/savebrowser -run TestParityGen -count=1
// Writes testdata/parity/components/savebrowser.txt ("<key>\t<%q value>").
// Each case builds a fixture directory "saves" (relative to a temp working
// directory) with envelope and legacy saves, a corrupt file, a non-save file
// and a directory, all with fixed UTC modification times, then records the
// view, state and emitted message after each key (including deletions).

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
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

func pgMsg(msg tea.Msg) string {
	switch v := msg.(type) {
	case nil:
		return "nilmsg"
	case LoadGameMsg:
		return fmt.Sprintf("LoadGameMsg path=%q", v.Path)
	case CloseBrowserMsg:
		return "CloseBrowserMsg"
	case tea.BatchMsg:
		return fmt.Sprintf("batch(%d)", len(v))
	}
	return fmt.Sprintf("%T", msg)
}

type pgSave struct {
	name     string
	kind     string // env, legacy, corrupt, other, dir
	skill    int
	cond     int
	stardate float64
	klingons int
	mod      time.Time
}

func pgT(mo, d, h, mi int) time.Time {
	return time.Date(2026, time.Month(mo), d, h, mi, 0, 0, time.UTC)
}

var pgFixture = []pgSave{
	{"ALPHA.TRK", "env", 1, 0, 2100.0, 15, pgT(3, 1, 10, 0)},
	{"bravo.trk", "env", 2, 1, 2200.55, 9, pgT(3, 2, 11, 30)},
	{"CHARLIE.TRK", "legacy", 3, 2, 2300.25, 3, pgT(1, 15, 8, 5)},
	{"ENTERPRISE_LONG_NAME.TRK", "env", 4, 3, 3456.78, 0, pgT(12, 31, 23, 59)},
	{"DELTA.TRK", "env", 5, 0, 1999.94, 120, pgT(3, 2, 11, 30)},
	{"ECHO.TRK", "env", 0, 9, -4.5, -1, pgT(7, 4, 0, 1)},
	{"FOXTROT.TRK", "env", 3, 1, 2001.0, 7, pgT(2, 28, 13, 13)},
	{"GOLF.TRK", "legacy", 1, 3, 2002.0, 8, pgT(6, 6, 6, 6)},
	{"ÉCLAT★LONGNAME.TRK", "env", 2, 2, 2003.0, 1, pgT(9, 9, 9, 9)},
	{"HOTEL.TRK", "env", 4, 0, 2004.5, 2, pgT(10, 10, 10, 10)},
	{"corrupt.TRK", "corrupt", 0, 0, 0, 0, pgT(11, 11, 11, 11)},
	{"notes.txt", "other", 0, 0, 0, 0, pgT(11, 12, 11, 11)},
	{"subdir.TRK", "dir", 0, 0, 0, 0, pgT(11, 13, 11, 11)},
}

func pgBuild(t *testing.T, dir string, saves []pgSave) {
	if err := os.RemoveAll(dir); err != nil {
		t.Fatal(err)
	}
	if err := os.MkdirAll(dir, 0o755); err != nil {
		t.Fatal(err)
	}
	for _, s := range saves {
		p := filepath.Join(dir, s.name)
		switch s.kind {
		case "env", "legacy":
			g := engine.NewGame(42, engine.SkillGood, engine.LengthShort)
			g.Skill = engine.SkillLevel(s.skill)
			g.Enterprise.Condition = engine.ConditionType(s.cond)
			g.Stardate = s.stardate
			g.RemainingKlingons = s.klingons
			var err error
			if s.kind == "env" {
				err = engine.SaveGame(g, p)
			} else {
				err = g.Save(p)
			}
			if err != nil {
				t.Fatal(err)
			}
		case "corrupt":
			os.WriteFile(p, []byte("{this is not json"), 0o644)
		case "other":
			os.WriteFile(p, []byte("hello"), 0o644)
		case "dir":
			os.MkdirAll(p, 0o755)
		}
		if err := os.Chtimes(p, s.mod, s.mod); err != nil {
			t.Fatal(err)
		}
	}
}

func pgState(m Model) string {
	var names []string
	for _, s := range m.saves {
		names = append(names, s.Filename)
	}
	return fmt.Sprintf("cursor=%d deleting=%v err=%q saves=%s", m.cursor, m.deleting, m.errorMessage, strings.Join(names, "|"))
}

var pgSequences = [][]string{
	{"down", "down", "down", "down", "down", "down", "down", "down", "down", "down", "up", "up", "up", "k", "K", "j", "J", "enter", "esc"},
	{"d", "down", "up", "j", "esc", "D", "enter", "delete", "delete", "down", "d", "enter", "end", "x", "enter"},
	{"j", "j", "d", "rm:CHARLIE.TRK", "enter", "down", "enter", "d", "esc", "esc"},
	{"end", "down", "down", "down", "down", "down", "down", "down", "down", "down", "d", "enter", "d", "enter", "d", "enter", "up", "enter"},
}

func TestParityGen(t *testing.T) {
	w, done := pgOpen(t, "savebrowser")
	defer done()
	time.Local = time.UTC
	t.Chdir(t.TempDir())

	for _, c := range pgCombos() {
		th, pre := c.apply()
		full := c.full()
		for si, seq := range pgSequences {
			if !full && si != 0 && si != 2 {
				continue
			}
			pgBuild(t, "saves", pgFixture)
			m := New(th, "saves")
			err := m.Refresh()
			p0 := fmt.Sprintf("%sseq%d/", pre, si)
			w(p0+"refresh", fmt.Sprintf("%v", err))
			w(p0+"init", m.View())
			for ki, k := range seq {
				p := fmt.Sprintf("%sseq%d/%d/%s/", pre, si, ki, k)
				if strings.HasPrefix(k, "rm:") {
					os.Remove(filepath.Join("saves", k[3:]))
					w(p+"state", pgState(m))
					continue
				}
				var cmd tea.Cmd
				m, cmd = m.Update(pgKey(k))
				w(p+"msg", pgCmd(cmd))
				w(p+"state", pgState(m))
				if full || ki == len(seq)-1 {
					w(p+"view", m.View())
				}
			}
		}
		if !full {
			continue
		}
		// Sizes.
		pgBuild(t, "saves", pgFixture)
		for _, wd := range []int{62, 80, 30, 2, 1, 0, -5} {
			m := New(th, "saves")
			m.Refresh()
			m.SetSize(wd, 10)
			w(fmt.Sprintf("%ssize/%d", pre, wd), m.View())
		}
		// Empty and missing directories.
		pgBuild(t, "empty", nil)
		m := New(th, "empty")
		w(pre+"empty/refresh", fmt.Sprintf("%v", m.Refresh()))
		w(pre+"empty/view", m.View())
		for _, k := range []string{"d", "delete", "enter", "down", "up"} {
			var cmd tea.Cmd
			m, cmd = m.Update(pgKey(k))
			w(pre+"empty/"+k, pgCmd(cmd)+" "+pgState(m))
		}
		m = New(th, "missing-dir")
		w(pre+"missing/refresh", fmt.Sprintf("%v", m.Refresh()))
		w(pre+"missing/view", m.View())
		w(pre+"missing/state", pgState(m))
		// Refresh keeps/clamps the cursor; Reset clears it.
		m = New(nil, "")
		w(pre+"dot/dir", m.directory)
		m = New(th, "saves")
		m.Refresh()
		m.cursor = 8
		m.deleting = true
		pgBuild(t, "saves", pgFixture[:3])
		m.Refresh()
		w(pre+"clamp", pgState(m))
		m.Reset()
		w(pre+"reset", pgState(m))
		m.SetTheme(nil)
		w(pre+"theme", m.theme.Name()+" "+string(m.theme.ColorMode()))
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
