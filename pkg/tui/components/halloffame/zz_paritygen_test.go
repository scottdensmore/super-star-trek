package halloffame

// Parity reference generator for the Rust port
// (crates/sst-tui/src/components/halloffame.rs).
// Run with: SST_PARITYGEN=1 go test ./pkg/tui/components/halloffame -run TestParityGen -count=1
// Writes testdata/parity/components/halloffame.txt ("<key>\t<%q value>"):
// both tabs for several score breakdowns, leaderboards and widths, tab and
// dismissal keys, and callsign entry. time.Now() entry dates are rewritten
// to 2401-02-03 04:05:06 UTC (the Rust side injects that clock).

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
	case CloseModalMsg:
		return "CloseModalMsg"
	case ScoreRecordedMsg:
		e := v.Entry
		return fmt.Sprintf("ScoreRecordedMsg name=%q score=%d rank=%q sd=%v won=%v skill=%q", e.CaptainName, e.Score, e.Rank, e.Stardate, e.GameWon, e.Skill)
	case tea.BatchMsg:
		return fmt.Sprintf("batch(%d)", len(v))
	}
	return fmt.Sprintf("%T", msg)
}

var pgFixedNow = time.Date(2401, 2, 3, 4, 5, 6, 0, time.UTC)

func pgFixDates(lb *engine.Leaderboard) {
	for i := range lb.Entries {
		if y := lb.Entries[i].Date.Year(); y >= 2000 && y < 2200 {
			lb.Entries[i].Date = pgFixedNow
		}
	}
}

func pgScores() []engine.ScoreBreakdown {
	return []engine.ScoreBreakdown{
		{},
		{KlingonsKilled: 5, KlingonPoints: 50, KillRate: 1.0, KillRatePoints: 500, TotalScore: 550, RankBadge: "[COMM]", RankTitle: "Commodore", ElapsedStardates: 5.0},
		{KlingonsKilled: 50, KlingonPoints: 5000, CommandersKilled: 10, CommanderPoints: 1000, SuperCommandersKilled: 5, SuperCommanderPoints: 1000,
			RomulansKilled: 20, RomulanPoints: 800, RomulansSurrendered: 5, SurrenderedPoints: 150, Casualties: 150, CasualtyPenalty: 1500,
			StarbasesLost: 2, StarbasePenalty: 200, HelpCalls: 5, HelpPenalty: 225, PlanetsDestroyed: 3, PlanetPenalty: 30, StarsDestroyed: 4,
			StarPenalty: 20, StarshipsLost: 1, StarshipPenalty: 100, KillRate: 25.50, KillRatePoints: 12750, WinBonus: 2500, TotalScore: 15000,
			RankBadge: "[FADM]", RankTitle: "Fleet Admiral", ElapsedStardates: 123.4, GameWon: true},
		{TotalScore: -50},
		{KlingonsKilled: 123, KlingonPoints: 123456, Casualties: 9999, CasualtyPenalty: 99999, StarshipsLost: 3, StarshipPenalty: 30000,
			KillRate: 1234.5678, KillRatePoints: 999999, WinBonus: 123456, TotalScore: 1234567, ElapsedStardates: 9999.95, GameWon: true},
		{KlingonsKilled: 7, KlingonPoints: 70, TotalScore: 175, ElapsedStardates: 0.04, KillRate: 0.005, RankTitle: "Custom Title Only"},
	}
}

func pgBoards() []*engine.Leaderboard {
	d := time.Date(2299, 12, 31, 23, 59, 0, 0, time.UTC)
	long := &engine.Leaderboard{Entries: []engine.ScoreEntry{
		{CaptainName: "Captain Jean-Luc Picard of the Enterprise", Score: 123456, Rank: "[ADMIRAL-X]", Stardate: 41153.75, Date: d},
		{CaptainName: "艦長 スールー 光", Score: -42, Rank: "[DISHONOR]", Stardate: -1.25, Date: d},
		{CaptainName: "", Score: 0, Rank: "", Stardate: 0, Date: time.Time{}},
	}}
	var many []engine.ScoreEntry
	for i := 0; i < 12; i++ {
		many = append(many, engine.ScoreEntry{CaptainName: fmt.Sprintf("Officer %02d", i), Score: 1000 - i*37, Rank: "[LT]", Stardate: float64(i) * 1.5, Date: d})
	}
	return []*engine.Leaderboard{engine.DefaultLeaderboard(), {Entries: []engine.ScoreEntry{}}, long, {Entries: many}}
}

var pgWidths = []int{66, 80, 40, 30, 12, 5}

var pgTabKeys = []string{"tab", "tab", "shift+tab", "right", "left", "2", "1", "x", "down", "esc", "enter", "q", "Q", "h", "H", "space", " "}

func TestParityGen(t *testing.T) {
	w, done := pgOpen(t, "halloffame")
	defer done()

	for _, c := range pgCombos() {
		th, pre := c.apply()
		full := c.full()
		// Score x width x tab views.
		for si, sc := range pgScores() {
			for wi, wd := range pgWidths {
				if !full && (wi > 0 || si != 2) {
					continue
				}
				m := New(th, wd, 18, "")
				m.SetState(sc, nil, false)
				p := fmt.Sprintf("%sscore%d/w%d/", pre, si, wd)
				w(p+"tab1", m.View())
				m.activeTab = tabLeaderboard
				w(p+"tab2", m.View())
			}
		}
		// Leaderboards.
		for bi, lb := range pgBoards() {
			if !full && bi != 2 {
				continue
			}
			m := New(th, 66, 18, "")
			m.SetState(pgScores()[1], lb, false)
			m.activeTab = tabLeaderboard
			w(fmt.Sprintf("%sboard%d", pre, bi), m.View())
		}
		// Tab / dismissal keys.
		m := New(th, 66, 18, "")
		m.SetState(pgScores()[1], nil, false)
		for ki, k := range pgTabKeys {
			var cmd tea.Cmd
			m, cmd = m.Update(pgKey(k))
			p := fmt.Sprintf("%skeys/%d/%s/", pre, ki, k)
			w(p+"msg", pgCmd(cmd))
			w(p+"tab", fmt.Sprintf("%d", m.activeTab))
		}
		// Callsign entry.
		entries := [][]string{
			{"P", "i", "c", "a", "r", "d", "backspace", "d", "enter", "enter"},
			{"space", "enter"},
			{"x", "esc", "tab"},
			{"A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R", "S", "T", "U", "V", "left", "left", "Z", "home", "<", "end", "enter"},
			{"tab", "1", "q", "winsize", "enter"},
		}
		for ei, seq := range entries {
			if !full && ei > 0 {
				continue
			}
			lb := engine.DefaultLeaderboard()
			m := New(th, 66, 18, "")
			m.SetState(pgScores()[2], lb, true)
			w(fmt.Sprintf("%sentry%d/init", pre, ei), m.View())
			for ki, k := range seq {
				var cmd tea.Cmd
				m, cmd = m.Update(pgKey(k))
				pgFixDates(lb)
				p := fmt.Sprintf("%sentry%d/%d/%s/", pre, ei, ki, k)
				w(p+"msg", pgCmd(cmd))
				w(p+"state", fmt.Sprintf("prompt=%v tab=%d value=%q entries=%d", m.promptName, m.activeTab, m.textInput.Value(), len(lb.Entries)))
				if full || ki == len(seq)-1 {
					w(p+"view", m.View())
				}
			}
		}
		if full {
			// Re-theme while prompting, nil leaderboard fallback, nil theme.
			m := New(nil, 0, 0, "")
			w(pre+"zero/size", fmt.Sprintf("%dx%d", m.width, m.height))
			m.SetTheme(th)
			m.SetState(engine.ScoreBreakdown{TotalScore: 2000}, nil, true)
			w(pre+"zero/prompt", m.View())
			m.SetTheme(theme.GetTheme("lcars").WithColorMode(theme.ColorModeLight))
			w(pre+"zero/retheme", m.View())
			m.leaderboard = nil
			m.SetState(engine.ScoreBreakdown{TotalScore: 10}, nil, false)
			m.activeTab = tabLeaderboard
			w(pre+"zero/nillb", m.View())
			m.leaderboard = nil
			w(pre+"zero/nillb2", m.View())
			m.SetTheme(nil)
			w(pre+"zero/theme", m.theme.Name()+" "+string(m.theme.ColorMode()))
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
