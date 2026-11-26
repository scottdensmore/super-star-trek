package scenariomodal

// Parity reference generator for the Rust port
// (crates/sst-tui/src/components/scenariomodal.rs).
// Run with: SST_PARITYGEN=1 go test ./pkg/tui/components/scenariomodal -run TestParityGen -count=1
// Writes testdata/parity/components/scenariomodal.txt ("<key>\t<%q value>").
// XDG_CONFIG_HOME points at a temp dir so the scenario leaderboards are the
// built-in defaults, then custom / unreadable / corrupt boards are written
// and picked up through Reset.

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
	case MsgLaunchScenario:
		return fmt.Sprintf("MsgLaunchScenario id=%s", v.ScenarioID)
	case MsgCloseScenarioModal:
		return "MsgCloseScenarioModal"
	case tea.BatchMsg:
		return fmt.Sprintf("batch(%d)", len(v))
	}
	return fmt.Sprintf("%T", msg)
}

var pgSequence = []string{"down", "j", "enter", "down", "k", "up", "up", "enter", "x", "tab", "esc", "q", "Q", "left"}

func pgState(w func(k, v string), p string, m Model, view bool) {
	sel := "nil"
	if s := m.SelectedScenario(); s != nil {
		sel = string(s.ID)
	}
	w(p+"selected", sel)
	if view {
		w(p+"view", m.View())
	}
}

func pgWriteBoards(t *testing.T, cfg string) {
	dir := filepath.Join(cfg, "super-star-trek")
	if err := os.MkdirAll(dir, 0o755); err != nil {
		t.Fatal(err)
	}
	d := time.Date(2370, 6, 15, 0, 0, 0, 0, time.UTC)
	lb := &engine.Leaderboard{Entries: []engine.ScoreEntry{
		{CaptainName: "Lt. Worf", Score: 640, Rank: "[LT]", Stardate: 47.25, Date: d},
		{CaptainName: "Captain Jean-Luc Picard of the USS Enterprise-D", Score: 99999, Rank: "[ADMIRAL-OF-THE-FLEET]", Stardate: 1234.56, Date: d, GameWon: true},
	}}
	if err := engine.SaveScenarioLeaderboard(engine.ScenarioMutaraNebula, lb); err != nil {
		t.Fatal(err)
	}
	if err := os.MkdirAll(engine.ScenarioLeaderboardPath(engine.ScenarioKobayashiMaru), 0o755); err != nil {
		t.Fatal(err)
	}
	empty := &engine.Leaderboard{Entries: []engine.ScoreEntry{}}
	if err := engine.SaveScenarioLeaderboard(engine.ScenarioStarbaseSiege, empty); err != nil {
		t.Fatal(err)
	}
}

func TestParityGen(t *testing.T) {
	w, done := pgOpen(t, "scenariomodal")
	defer done()
	cfg := t.TempDir()
	t.Setenv("XDG_CONFIG_HOME", cfg)

	run := func(tag string, boards func()) {
		for _, c := range pgCombos() {
			th, pre := c.apply()
			pre += tag + "/"
			full := c.full()
			m := NewModel(th)
			if boards != nil {
				boards()
				m.Reset()
			}
			for i := 0; i < 3; i++ {
				if full || i == 1 || tag == "default" {
					w(fmt.Sprintf("%sinit/%d", pre, i), m.View())
				}
				m, _ = m.Update(pgKey("down"))
			}
			for ki, k := range pgSequence {
				var cmd tea.Cmd
				m, cmd = m.Update(pgKey(k))
				p := fmt.Sprintf("%sseq/%d/%s/", pre, ki, k)
				w(p+"msg", pgCmd(cmd))
				pgState(w, p, m, full)
			}
		}
	}
	run("default", nil)
	run("custom", func() { pgWriteBoards(t, cfg) })
	corrupt := func() {
		if err := os.WriteFile(engine.ScenarioLeaderboardPath(engine.ScenarioStarbaseSiege), []byte("{not json"), 0o644); err != nil {
			t.Fatal(err)
		}
	}
	run("corrupt", corrupt)

	lipgloss.SetColorProfile(termenv.Ascii)
	th := theme.GetTheme("lcars").WithColorMode(theme.ColorModeDark)
	m := New(th, 0, -5)
	m.SetDimensions(80, 24)
	m.SetSize(-1, 30)
	w("sized", fmt.Sprintf("%dx%d", m.width, m.height))
	m.SetTheme(theme.GetTheme("crt").WithColorMode(theme.ColorModeLight))
	w("sized/view", m.View())

	var z Model
	w("zero/selected", fmt.Sprintf("%v", z.SelectedScenario() == nil))
	_, cmd := z.Update(pgKey("enter"))
	w("zero/enter", pgCmd(cmd))
	_, cmd = z.Update(pgKey("down"))
	w("zero/down", pgCmd(cmd))
	w("zero/view", z.View())
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
