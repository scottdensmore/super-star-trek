package optionsmodal

// Parity reference generator for the Rust port
// (crates/sst-tui/src/components/optionsmodal.rs).
// Run with: SST_PARITYGEN=1 go test ./pkg/tui/components/optionsmodal -run TestParityGen -count=1
// Writes testdata/parity/components/optionsmodal.txt ("<key>\t<%q value>"):
// key sequences over every row with the state, view, emitted message and
// the calls received by a recording audio player after each key.

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
	"github.com/scottdensmore/super-star-trek/pkg/audio"
	"github.com/scottdensmore/super-star-trek/pkg/config"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

func pgMsg(msg tea.Msg) string {
	switch v := msg.(type) {
	case nil:
		return "nilmsg"
	case PlaySoundMsg:
		return "PlaySoundMsg " + string(v.Sound)
	case tea.BatchMsg:
		return fmt.Sprintf("batch(%d)", len(v))
	}
	return fmt.Sprintf("%T", msg)
}

type pgPlayer struct {
	log   []string
	vol   int
	muted bool
}

func (p *pgPlayer) Play(s audio.SoundID) { p.log = append(p.log, "play="+string(s)) }
func (p *pgPlayer) SetMuted(m bool)      { p.muted = m; p.log = append(p.log, fmt.Sprintf("muted=%v", m)) }
func (p *pgPlayer) IsMuted() bool        { return p.muted }
func (p *pgPlayer) SetVolume(v int)      { p.vol = v; p.log = append(p.log, fmt.Sprintf("vol=%d", v)) }
func (p *pgPlayer) Volume() int          { return p.vol }
func (p *pgPlayer) take() string {
	s := strings.Join(p.log, ",")
	p.log = nil
	return s
}

func pgRules(r engine.GameRules) string {
	return fmt.Sprintf("profile=%s surv=%s deg=%v rep=%v cloak=%v anom=%v adv=%v tm=%v anim=%d",
		r.Profile, r.Surveillance, r.SensorDegradation, r.RepairMultiplier, r.KlingonCloak,
		r.SpatialAnomalies, r.Adversaries, r.TimeMargin, r.AnimSpeed)
}

func pgState(m Model) string {
	return fmt.Sprintf("row=%d closed=%v active=%v cm=%s audio=%v mode=%s vol=%d sound=%s rules{%s}",
		m.SelectedRow, m.Closed, m.Active(), m.ColorMode(), m.AudioEnabled(), m.AudioMode(), m.Volume(),
		m.SelectedSound(), pgRules(m.Rules()))
}

var pgSequences = [][]string{
	// Profile row: cycle presets both ways (Custom keeps the rules).
	{"right", "right", "right", "right", "right", "right", "right", "left", "left", "space", "enter", "l", "h"},
	// Rule rows.
	{"down", "right", "right", "left", "down", "space", "down", "right", "right", "right", "right", "left",
		"down", "enter", "down", "right", "right", "right", "right", "left", "down", "right", "right", "left", "left", "left"},
	// Color mode, audio, audio mode, volume.
	{"j", "j", "j", "j", "j", "j", "j", "right", "right", "right", "left", "j", "space", "enter", "j",
		"right", "right", "right", "right", "left", "left", "j", "left", "left", "right", "right", "right", "right", "right", "right", "right", "right", "right", "right", "right"},
	// Sound test: cycle and audition, then Done.
	{"up", "up", "space", "right", "enter", "left", "left", "space", "l", "l", "l", "enter", "down", "space", "up", "down", "enter"},
	// Closing keys and ignored keys.
	{"x", "tab", "k", "q"},
	{"esc", "down", "enter"},
}

func TestParityGen(t *testing.T) {
	w, done := pgOpen(t, "optionsmodal")
	defer done()

	for _, c := range pgCombos() {
		th, pre := c.apply()
		full := c.full()
		for si, seq := range pgSequences {
			m := New(th, engine.DefaultRulesForProfile(engine.ProfileNormal))
			pl := &pgPlayer{vol: 80}
			if si != 5 {
				m.SetPlayer(pl)
			}
			if full || si == 0 {
				w(fmt.Sprintf("%sseq%d/init", pre, si), m.View())
			}
			for ki, k := range seq {
				var cmd tea.Cmd
				m, cmd = m.Update(pgKey(k))
				p := fmt.Sprintf("%sseq%d/%d/%s/", pre, si, ki, k)
				msg, played := pgCmd(cmd), pl.take()
				if c.prof == termenv.Ascii {
					w(p+"msg", msg)
					w(p+"player", played)
					w(p+"state", pgState(m))
				}
				if full || (ki == len(seq)-1 && si%3 == 0) {
					w(p+"view", m.View())
				}
			}
		}
		if !full {
			continue
		}
		for _, prof := range []engine.DifficultyProfile{engine.ProfileCasual, engine.ProfileNormal, engine.ProfileHardcore,
			engine.ProfileNightmare, engine.ProfileExpert, engine.ProfileEmeritus, engine.ProfileCustom, "bogus"} {
			m := New(th, engine.DefaultRulesForProfile(prof))
			m.SelectedRow = RowDone
			w(pre+"profile/"+string(prof), pgState(m)+"\n"+m.View())
		}
		m := New(nil, engine.GameRules{})
		w(pre+"zero", pgState(m))
		pl := &pgPlayer{}
		m.SetPlayer(pl)
		for _, v := range []int{150, -3, 55, 100, 0} {
			m.SetVolume(v)
			w(fmt.Sprintf("%svol/%d", pre, v), fmt.Sprintf("%d %s", m.Volume(), pl.take()))
		}
		for _, md := range []config.AudioMode{"native", "bell", "off", "auto", "weird", ""} {
			m.SetAudioMode(md)
			w(pre+"mode/"+string(md), string(m.AudioMode()))
		}
		for _, s := range []audio.SoundID{audio.SoundDock, "nope", audio.SoundComputerBeep} {
			m.SetSelectedSound(s)
			w(pre+"sound/"+string(s), string(m.SelectedSound()))
		}
		m.SetTheme(theme.GetTheme("lcars").WithColorMode(theme.ColorModeLight))
		w(pre+"settheme", string(m.ColorMode()))
		m.SetColorMode(theme.ColorModeDark)
		m.SetAudioEnabled(false)
		m.SetActive(false)
		w(pre+"inactive", pgState(m))
		m.Closed = true
		m.SetActive(true)
		w(pre+"reactivated", pgState(m))
		m.SelectedRow = RowRepairMultiplier
		r := m.Rules()
		r.RepairMultiplier = 1.3
		r.TimeMargin = 0.333
		r.AnimSpeed = 7
		m.SetRules(r)
		w(pre+"odd/view", m.View())
		m, _ = m.Update(pgKey("left"))
		w(pre+"odd/left", pgState(m))
		m.SelectedRow = RowAnimSpeed
		m, _ = m.Update(pgKey("left"))
		w(pre+"odd/anim", pgState(m))
		m.SelectedRow = Row(20)
		m, _ = m.Update(pgKey("right"))
		w(pre+"odd/row20", pgState(m)+"\n"+m.View())
		w(pre+"player/nil", fmt.Sprintf("%v", New(th, engine.GameRules{}).Player() == nil))
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
