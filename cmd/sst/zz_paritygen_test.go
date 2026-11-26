package main

// CLI parity generator for the Rust port (crates/sst). Run with:
//
//	SST_PARITYGEN=1 go test ./cmd/sst -run TestParityGenCLI -count=1
//
// It writes testdata/parity/cli/cli.txt. Each case runs run() in a fresh HOME
// with runProgram replaced by a capture hook.

import (
	"bytes"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/muesli/termenv"
	"github.com/scottdensmore/super-star-trek/pkg/tui"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

type cliCase struct {
	name    string
	args    []string
	config  string // raw config.json content written before the run ("" = none)
	progErr bool   // runProgram returns an error
}

func pgSection(b *bytes.Buffer, key string, data string) {
	fmt.Fprintf(b, "%s %d\n%s\n", key, len(data), data)
}

func cliFacts(m tui.Model) string {
	var b strings.Builder
	fmt.Fprintf(&b, "sound_enabled=%v\n", m.SoundEnabled())
	if m.AudioPlayer != nil {
		fmt.Fprintf(&b, "player volume=%d muted=%v\n", m.AudioPlayer.Volume(), m.AudioPlayer.IsMuted())
	} else {
		b.WriteString("player nil\n")
	}
	if m.Theme != nil {
		fmt.Fprintf(&b, "theme=%s mode=%s\n", m.Theme.Name(), m.Theme.ColorMode())
	} else {
		b.WriteString("theme nil\n")
	}
	if g := m.Game; g != nil {
		r := g.Rules
		fmt.Fprintf(&b, "rules profile=%s surveillance=%s sensor=%v repair=%v cloak=%v anomalies=%v adversaries=%v margin=%v anim=%d\n",
			r.Profile, r.Surveillance, r.SensorDegradation, r.RepairMultiplier, r.KlingonCloak, r.SpatialAnomalies, r.Adversaries, r.TimeMargin, r.AnimSpeed)
		fmt.Fprintf(&b, "game scenario=%q stardate=%v klingons=%d quad=%v sector=%v energy=%v\n",
			string(g.Scenario), g.Stardate, g.RemainingKlingons, g.Enterprise.Quad, g.Enterprise.Sector, g.Enterprise.Energy)
	} else {
		b.WriteString("game nil\n")
	}
	if t := m.Tour; t != nil {
		fmt.Fprintf(&b, "tour active=%v sector=%d sectors=%d\n", t.Active, t.CurrentSectorIndex, len(t.Sectors))
	} else {
		b.WriteString("tour nil\n")
	}
	fmt.Fprintf(&b, "size=%dx%d\n", m.Width, m.Height)
	return b.String()
}

func cliCases() []cliCase {
	s := func(a ...string) []string { return a }
	cs := []cliCase{
		{name: "help-long", args: s("--help")},
		{name: "help-short", args: s("-h")},
		{name: "help-single", args: s("-help")},
		{name: "help-eq-true", args: s("--help=true")},
		{name: "help-after-flag", args: s("-seed", "5", "-h")},
		{name: "help-after-positional", args: s("foo", "-h")},
		{name: "help-with-bad-before", args: s("-nope", "--help")},
		{name: "version-long", args: s("--version")},
		{name: "version-short", args: s("-v")},
		{name: "version-eq", args: s("-version=true")},
		{name: "version-false", args: s("-version=false", "-seed", "3")},
		{name: "version-bad-bool", args: s("-v=maybe")},
		{name: "list-scenarios", args: s("--list-scenarios")},
		{name: "list-scenarios-single", args: s("-list-scenarios")},
		{name: "version-and-list", args: s("-list-scenarios", "-v")},
		{name: "unknown-flag", args: s("--unsupported-flag")},
		{name: "unknown-flag-single", args: s("-x")},
		{name: "unknown-flag-value", args: s("-zz=4")},
		{name: "bad-syntax-3dash", args: s("---x")},
		{name: "bad-syntax-eq", args: s("-=x")},
		{name: "bad-syntax-dash-eq", args: s("--=")},
		{name: "missing-value-seed", args: s("-seed")},
		{name: "missing-value-theme", args: s("--theme")},
		{name: "bad-int64", args: s("-seed", "abc")},
		{name: "bad-int64-range", args: s("-seed", "99999999999999999999")},
		{name: "bad-int", args: s("-volume", "x")},
		{name: "bad-int-float", args: s("-volume=1.5")},
		{name: "bad-int-range", args: s("-volume=-9223372036854775809")},
		{name: "bad-float", args: s("-repair-multiplier", "fast")},
		{name: "bad-float-range", args: s("-repair-multiplier=1e999")},
		{name: "bad-bool", args: s("-sound=maybe")},
		{name: "bad-bool-empty", args: s("-mute=")},
		{name: "dash-only", args: s("-", "-seed", "7")},
		{name: "terminator", args: s("-seed", "7", "--", "-nope")},
		{name: "positional-stops", args: s("-seed", "7", "extra", "-nope")},
		{name: "anomalies-conflict", args: s("--anomalies", "--no-anomalies")},
		{name: "anomalies-conflict-false", args: s("-anomalies=false", "-no-anomalies=false")},
		{name: "sound-conflict", args: s("--sound", "--no-sound")},
		{name: "sound-conflict-single", args: s("-sound", "-no-sound")},
		{name: "unknown-scenario", args: s("--scenario=nonexistent")},
		{name: "unknown-scenario-short", args: s("-s", "bogus", "-seed", "1")},
		{name: "bad-mode", args: s("-mode", "invalid")},
		{name: "bad-mode-solar", args: s("--mode=solar")},
		{name: "bad-mode-and-scenario", args: s("--mode=solar", "--scenario=nope")},
		{name: "classic", args: s("--classic")},
		{name: "classic-single", args: s("-classic")},
		{name: "classic-seed", args: s("-classic", "-seed", "999")},
		{name: "classic-eq-true", args: s("--classic=true")},
		{name: "classic-eq-false", args: s("--classic=false")},
		{name: "classic-eq-false-seed", args: s("-classic=false", "-seed", "42")},
		{name: "classic-unknown", args: s("--classic", "-unknownflag")},
		{name: "classic-theme", args: s("--classic", "--theme", "lcars")},
		{name: "classic-help", args: s("--classic", "-h")},
		{name: "classic-bad-seed", args: s("--classic", "-seed", "x")},
		{name: "classic-bad-bool", args: s("--classic=maybe")},
		{name: "classic-positional", args: s("--classic", "foo")},
		{name: "classic-late", args: s("-seed", "123", "--classic")},
		{name: "classic-rock", args: s("--classic-rock")},
		// TUI-launching cases with explicit seeds.
		{name: "tui-seed", args: s("-seed", "42")},
		{name: "tui-seed-eq", args: s("--seed=12345")},
		{name: "tui-seed-hex", args: s("-seed", "0x2A")},
		{name: "tui-seed-octal", args: s("-seed=017")},
		{name: "tui-seed-underscore", args: s("-seed=1_000")},
		{name: "tui-seed-neg", args: s("-seed", "-7")},
		{name: "tui-lcars", args: s("--theme", "lcars", "-seed", "42")},
		{name: "tui-crt-light", args: s("--theme=crt", "--mode=light", "-seed", "9")},
		{name: "tui-modern-dark", args: s("-theme", "modern", "-mode", "dark", "-seed", "9")},
		{name: "tui-mode-upper", args: s("-mode", "DARK", "-seed", "4")},
		{name: "tui-mode-auto", args: s("-mode", "auto", "-seed", "4")},
		{name: "tui-unknown-theme", args: s("-theme", "bogus", "-seed", "4")},
		{name: "tui-casual", args: s("--difficulty=casual", "-seed", "11")},
		{name: "tui-hardcore", args: s("--difficulty=hardcore", "-seed", "11")},
		{name: "tui-nightmare", args: s("--difficulty=nightmare", "--seed=12345")},
		{name: "tui-expert", args: s("--difficulty=expert", "-seed", "42")},
		{name: "tui-emeritus", args: s("--difficulty=emeritus", "-seed", "42")},
		{name: "tui-unknown-difficulty", args: s("--difficulty=zany", "-seed", "42")},
		{name: "tui-overrides", args: s("--difficulty=nightmare", "--surveillance=full", "--klingon-cloak=false", "--repair-multiplier=1.25", "--sensor-degradation=false", "-seed", "5")},
		{name: "tui-surveillance-local", args: s("-surveillance", "local", "-seed", "5")},
		{name: "tui-surveillance-weird", args: s("-surveillance", "weird", "-seed", "5")},
		{name: "tui-repair-hex", args: s("-repair-multiplier", "0x1p-1", "-seed", "5")},
		{name: "tui-repair-inf", args: s("-repair-multiplier", "inf", "-seed", "5")},
		{name: "tui-cloak", args: s("-klingon-cloak", "-seed", "5")},
		{name: "tui-anomalies", args: s("--anomalies", "-seed", "42")},
		{name: "tui-no-anomalies", args: s("--difficulty=nightmare", "--no-anomalies", "-seed", "42")},
		{name: "tui-anomalies-false", args: s("--difficulty=nightmare", "--anomalies=false", "-seed", "42")},
		{name: "tui-adversaries", args: s("--adversaries", "-seed", "42")},
		{name: "tui-a", args: s("-a", "-seed", "42")},
		{name: "tui-a-false", args: s("-a=false", "-difficulty", "expert", "-seed", "42")},
		{name: "tui-adv-false", args: s("--adversaries=false", "-seed", "42")},
		{name: "tui-scenario-mutara", args: s("--scenario", "mutara-nebula", "-seed", "8")},
		{name: "tui-scenario-kobayashi", args: s("-s", "kobayashi", "-seed", "8")},
		{name: "tui-scenario-kobayashi-full", args: s("-s=kobayashi-maru", "-seed", "8", "-theme", "lcars")},
		{name: "tui-scenario-siege", args: s("--scenario=starbase-siege", "-seed", "8")},
		{name: "tui-tour", args: s("--tour", "-seed", "42")},
		{name: "tui-campaign", args: s("--campaign", "-seed", "42")},
		{name: "tui-tour-scenario", args: s("--tour", "-s", "kobayashi", "-seed", "42")},
		{name: "tui-volume", args: s("--volume=45", "-seed", "42")},
		{name: "tui-volume-0", args: s("--volume=0", "-seed", "42")},
		{name: "tui-volume-neg", args: s("--volume=-5", "-seed", "42")},
		{name: "tui-volume-big", args: s("--volume=250", "-seed", "42")},
		{name: "tui-mute", args: s("--mute", "-seed", "42")},
		{name: "tui-volume-mute", args: s("--volume=60", "--mute", "-seed", "42")},
		{name: "tui-audio-off", args: s("--audio-mode=off", "-seed", "42")},
		{name: "tui-audio-bell", args: s("--audio-mode=bell", "-seed", "42")},
		{name: "tui-audio-bogus", args: s("--audio-mode=Bogus", "-seed", "42")},
		{name: "tui-audio-off-unmute", args: s("--audio-mode=OFF", "-mute=false", "-seed", "42")},
		{name: "tui-sound", args: s("--sound", "-seed", "42")},
		{name: "tui-no-sound", args: s("--no-sound", "-seed", "42")},
		{name: "tui-sound-false", args: s("--sound=false", "-seed", "42")},
		{name: "tui-no-sound-false", args: s("--no-sound=false", "-seed", "42")},
		{name: "tui-sound-mute", args: s("--sound", "--mute", "-seed", "42")},
		{name: "tui-classic-after-terminator", args: s("-seed", "42", "--", "--classic")},
		// Seed 0: random game, no view recorded.
		{name: "tui-default", args: s()},
		{name: "tui-crt-random", args: s("--theme", "crt")},
		{name: "tui-progerr", args: s("-seed", "42"), progErr: true},
		{name: "tui-progerr-random", args: s(), progErr: true},
	}
	cfgMuted := `{"volume":35,"muted":true,"audio_mode":"native","theme":"modern","anim_speed":"normal"}`
	cfgLoud := `{"volume":70,"muted":false,"audio_mode":"bell","theme":"crt","anim_speed":"fast"}`
	cfgBad := `{"volume": "loud"`
	cs = append(cs,
		cliCase{name: "cfg-muted", args: s("-seed", "42"), config: cfgMuted},
		cliCase{name: "cfg-muted-volume", args: s("--volume=90", "-seed", "42"), config: cfgMuted},
		cliCase{name: "cfg-muted-sound", args: s("--sound", "-seed", "42"), config: cfgMuted},
		cliCase{name: "cfg-muted-no-sound-false", args: s("--no-sound=false", "-seed", "42"), config: cfgMuted},
		cliCase{name: "cfg-loud", args: s("-seed", "42"), config: cfgLoud},
		cliCase{name: "cfg-loud-audio-off", args: s("-audio-mode", "off", "-seed", "42"), config: cfgLoud},
		cliCase{name: "cfg-loud-mute", args: s("-mute", "-volume", "10", "-seed", "42"), config: cfgLoud},
		cliCase{name: "cfg-bad", args: s("-seed", "42"), config: cfgBad},
	)
	return cs
}

func TestParityGenCLI(t *testing.T) {
	if os.Getenv("SST_PARITYGEN") != "1" {
		t.Skip("set SST_PARITYGEN=1 to regenerate CLI parity data")
	}
	os.Setenv("COLORFGBG", "15;0")
	os.Unsetenv("TERM_PROGRAM")
	os.Setenv("TZ", "UTC")
	lipgloss.SetColorProfile(termenv.Ascii)
	lipgloss.SetHasDarkBackground(true)

	origRun := runProgram
	defer func() { runProgram = origRun }()
	origHome, hadHome := os.LookupEnv("HOME")
	origXDG, hadXDG := os.LookupEnv("XDG_CONFIG_HOME")
	defer func() {
		if hadHome {
			os.Setenv("HOME", origHome)
		}
		if hadXDG {
			os.Setenv("XDG_CONFIG_HOME", origXDG)
		} else {
			os.Unsetenv("XDG_CONFIG_HOME")
		}
	}()

	var b bytes.Buffer
	cases := cliCases()
	fmt.Fprintf(&b, "cases %d\n", len(cases))
	for i, c := range cases {
		home := t.TempDir()
		os.Setenv("HOME", home)
		os.Setenv("XDG_CONFIG_HOME", filepath.Join(home, ".config"))
		if c.config != "" {
			dir := filepath.Join(home, ".super-star-trek")
			if err := os.MkdirAll(dir, 0o755); err != nil {
				t.Fatal(err)
			}
			if err := os.WriteFile(filepath.Join(dir, "config.json"), []byte(c.config), 0o644); err != nil {
				t.Fatal(err)
			}
		}
		theme.ResetDetectionCache()

		var captured *tui.Model
		var optsMask string
		runProgram = func(m tea.Model, opts ...tea.ProgramOption) error {
			tm := m.(tui.Model)
			captured = &tm
			optsMask = fmt.Sprintf("%d", len(opts))
			if c.progErr {
				return errors.New("terminal failure")
			}
			return nil
		}

		var out, errOut bytes.Buffer
		code := run(c.args, strings.NewReader(""), &out, &errOut)
		first := captured
		// Run again in a fresh HOME: a differing model means a random seed.
		randomSeed := false
		if first != nil {
			home2 := t.TempDir()
			os.Setenv("HOME", home2)
			os.Setenv("XDG_CONFIG_HOME", filepath.Join(home2, ".config"))
			if c.config != "" {
				dir := filepath.Join(home2, ".super-star-trek")
				_ = os.MkdirAll(dir, 0o755)
				_ = os.WriteFile(filepath.Join(dir, "config.json"), []byte(c.config), 0o644)
			}
			captured = nil
			var o2, e2 bytes.Buffer
			_ = run(c.args, strings.NewReader(""), &o2, &e2)
			if captured == nil || cliFacts(*captured)+captured.View() != cliFacts(*first)+first.View() {
				randomSeed = true
			}
			captured = first
		}

		fmt.Fprintf(&b, "case %d %s\n", i, c.name)
		fmt.Fprintf(&b, "args %d\n", len(c.args))
		for _, a := range c.args {
			pgSection(&b, "arg", a)
		}
		pgSection(&b, "config", c.config)
		fmt.Fprintf(&b, "progerr %v\n", c.progErr)
		fmt.Fprintf(&b, "exit %d\n", code)
		pgSection(&b, "stdout", out.String())
		pgSection(&b, "stderr", errOut.String())
		if captured == nil {
			b.WriteString("model none\n")
		} else if randomSeed {
			fmt.Fprintf(&b, "model random opts=%s\n", optsMask)
		} else {
			fmt.Fprintf(&b, "model seeded opts=%s\n", optsMask)
			pgSection(&b, "facts", cliFacts(*captured))
			pgSection(&b, "view", captured.View())
		}
		b.WriteString("end\n")
	}

	outDir := filepath.Join("..", "..", "testdata", "parity", "cli")
	if err := os.MkdirAll(outDir, 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(outDir, "cli.txt"), b.Bytes(), 0o644); err != nil {
		t.Fatal(err)
	}
	t.Logf("wrote %d cases, %d bytes", len(cases), b.Len())
}
