package main

import (
	"bytes"
	"errors"
	"reflect"
	"strings"
	"testing"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

func TestIsClassic(t *testing.T) {
	tests := []struct {
		name string
		args []string
		want bool
	}{
		{name: "empty args", args: []string{}, want: false},
		{name: "classic long flag", args: []string{"--classic"}, want: true},
		{name: "classic short flag", args: []string{"-classic"}, want: true},
		{name: "classic long flag with value", args: []string{"--classic=true"}, want: true},
		{name: "classic short flag with value", args: []string{"-classic=true"}, want: true},
		{name: "classic flag preceded by other flags", args: []string{"-seed", "123", "--classic"}, want: true},
		{name: "classic flag followed by classic flags", args: []string{"-classic", "-seed", "456"}, want: true},
		{name: "tui theme flag only", args: []string{"--theme", "lcars"}, want: false},
		{name: "tui seed flag only", args: []string{"-seed", "123"}, want: false},
		{name: "flag that merely has classic in name", args: []string{"--classic-rock"}, want: false},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := isClassic(tt.args)
			if got != tt.want {
				t.Errorf("isClassic(%v) = %v, want %v", tt.args, got, tt.want)
			}
		})
	}
}

func TestRun_ClassicMode(t *testing.T) {
	in := strings.NewReader("")
	var out, errOut bytes.Buffer

	// Running with --classic
	exitCode := run([]string{"--classic"}, in, &out, &errOut)
	if exitCode != 0 {
		t.Fatalf("expected exit code 0 for --classic, got %d", exitCode)
	}
	output := out.String()
	if !strings.Contains(output, "Super Star Trek (Go Edition)") {
		t.Errorf("expected classic banner in output, got %q", output)
	}
	if !strings.Contains(output, "Shields:") {
		t.Errorf("expected shield telemetry in classic output, got %q", output)
	}

	// Running with -classic and -seed
	out.Reset()
	errOut.Reset()
	exitCode = run([]string{"-classic", "-seed", "999"}, in, &out, &errOut)
	if exitCode != 0 {
		t.Fatalf("expected exit code 0 for -classic -seed 999, got %d", exitCode)
	}
	if !strings.Contains(out.String(), "Super Star Trek (Go Edition)") {
		t.Errorf("expected classic banner with -classic -seed, got %q", out.String())
	}
}

func TestRun_ClassicModeUnknownFlag(t *testing.T) {
	in := strings.NewReader("")
	var out, errOut bytes.Buffer

	// When classic mode is detected, TUI fs.Parse is bypassed and arguments are delegated
	// directly to classic.RunClassicCLI, which rejects flags not recognized by its flagset.
	exitCode := run([]string{"--classic", "-unknownflag"}, in, &out, &errOut)
	if exitCode == 0 {
		t.Fatalf("expected non-zero exit code when classic mode receives an unknown flag, got %d", exitCode)
	}
}

func TestRun_TUIModeValidFlags(t *testing.T) {
	origRunProgram := runProgram
	t.Cleanup(func() { runProgram = origRunProgram })

	called := false
	runProgram = func(m tea.Model, opts ...tea.ProgramOption) error {
		called = true
		return nil
	}

	in := strings.NewReader("")
	var out, errOut bytes.Buffer

	exitCode := run([]string{"--theme", "lcars", "-seed", "42"}, in, &out, &errOut)
	if exitCode != 0 {
		t.Fatalf("expected exit code 0 with valid TUI flags, got %d", exitCode)
	}
	if !called {
		t.Fatalf("expected runProgram to be called")
	}

	// Another theme
	called = false
	exitCode = run([]string{"--theme", "crt"}, in, &out, &errOut)
	if exitCode != 0 {
		t.Fatalf("expected exit code 0 with crt theme, got %d", exitCode)
	}
	if !called {
		t.Fatalf("expected runProgram to be called for crt theme")
	}

	// Default args
	called = false
	exitCode = run([]string{}, in, &out, &errOut)
	if exitCode != 0 {
		t.Fatalf("expected exit code 0 with no args, got %d", exitCode)
	}
	if !called {
		t.Fatalf("expected runProgram to be called with no args")
	}
}

func TestRun_TUIHelp(t *testing.T) {
	in := strings.NewReader("")
	var out, errOut bytes.Buffer

	exitCode := run([]string{"--help"}, in, &out, &errOut)
	if exitCode != 0 {
		t.Fatalf("expected exit code 0 for --help, got %d", exitCode)
	}

	out.Reset()
	errOut.Reset()
	exitCode = run([]string{"-h"}, in, &out, &errOut)
	if exitCode != 0 {
		t.Fatalf("expected exit code 0 for -h, got %d", exitCode)
	}
}

func TestRun_TUIUnknownFlag(t *testing.T) {
	in := strings.NewReader("")
	var out, errOut bytes.Buffer

	exitCode := run([]string{"--unsupported-flag"}, in, &out, &errOut)
	if exitCode == 0 {
		t.Fatalf("expected non-zero exit code for unsupported TUI flag, got %d", exitCode)
	}
	if errOut.Len() == 0 {
		t.Fatalf("expected error message in stderr for unsupported TUI flag")
	}
}

func TestRun_TUIProgramError(t *testing.T) {
	origRunProgram := runProgram
	t.Cleanup(func() { runProgram = origRunProgram })

	runProgram = func(m tea.Model, opts ...tea.ProgramOption) error {
		return errors.New("terminal failure")
	}

	in := strings.NewReader("")
	var out, errOut bytes.Buffer

	exitCode := run([]string{}, in, &out, &errOut)
	if exitCode != 1 {
		t.Fatalf("expected exit code 1 when runProgram fails, got %d", exitCode)
	}
	if !strings.Contains(errOut.String(), "terminal failure") {
		t.Fatalf("expected error output to contain 'terminal failure', got %q", errOut.String())
	}
}

func TestRun_TUIMouseCellMotionOption(t *testing.T) {
	origRunProgram := runProgram
	t.Cleanup(func() { runProgram = origRunProgram })

	var capturedOpts []tea.ProgramOption
	runProgram = func(m tea.Model, opts ...tea.ProgramOption) error {
		capturedOpts = opts
		return nil
	}

	in := strings.NewReader("")
	var out, errOut bytes.Buffer

	exitCode := run([]string{}, in, &out, &errOut)
	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d", exitCode)
	}

	dummy := tea.NewProgram(nil, capturedOpts...)
	field := reflect.ValueOf(dummy).Elem().FieldByName("startupOptions")
	if !field.IsValid() {
		t.Fatalf("could not inspect startupOptions field")
	}
	actualStartupOptions := field.Int()

	refMouse := reflect.ValueOf(tea.NewProgram(nil, tea.WithMouseCellMotion())).Elem().FieldByName("startupOptions").Int()
	refAlt := reflect.ValueOf(tea.NewProgram(nil, tea.WithAltScreen())).Elem().FieldByName("startupOptions").Int()

	if actualStartupOptions&refMouse == 0 {
		t.Errorf("expected runProgram to be called with tea.WithMouseCellMotion(), options bitmask: %b", actualStartupOptions)
	}
	if actualStartupOptions&refAlt == 0 {
		t.Errorf("expected runProgram to be called with tea.WithAltScreen(), options bitmask: %b", actualStartupOptions)
	}
}

func TestCLIFlags_DifficultyAndSurveillance(t *testing.T) {
	origRunProgram := runProgram
	t.Cleanup(func() { runProgram = origRunProgram })

	var capturedModel tea.Model
	runProgram = func(m tea.Model, opts ...tea.ProgramOption) error {
		capturedModel = m
		return nil
	}

	var stdout, stderr bytes.Buffer
	code := run([]string{"--difficulty=nightmare", "--seed=12345"}, strings.NewReader(""), &stdout, &stderr)
	if code != 0 {
		t.Fatalf("run failed with code %d: %s", code, stderr.String())
	}

	model, ok := capturedModel.(tui.Model)
	if !ok {
		t.Fatalf("captured model is not tui.Model: %T", capturedModel)
	}
	if model.Game.Rules.Profile != engine.ProfileNightmare {
		t.Errorf("expected Nightmare profile, got %v", model.Game.Rules.Profile)
	}
	if model.Game.Rules.Surveillance != engine.SurveillanceBlackout {
		t.Errorf("expected SurveillanceBlackout, got %v", model.Game.Rules.Surveillance)
	}
	if !model.Game.Rules.KlingonCloak {
		t.Errorf("expected KlingonCloak=true for nightmare, got false")
	}
	if model.Game.Rules.RepairMultiplier != 2.0 {
		t.Errorf("expected RepairMultiplier=2.0 for nightmare, got %v", model.Game.Rules.RepairMultiplier)
	}
}

func TestCLIFlags_Overrides(t *testing.T) {
	origRunProgram := runProgram
	t.Cleanup(func() { runProgram = origRunProgram })

	var capturedModel tea.Model
	runProgram = func(m tea.Model, opts ...tea.ProgramOption) error {
		capturedModel = m
		return nil
	}

	var stdout, stderr bytes.Buffer
	code := run([]string{"--difficulty=nightmare", "--surveillance=full", "--klingon-cloak=false", "--repair-multiplier=1.25", "--sensor-degradation=false"}, strings.NewReader(""), &stdout, &stderr)
	if code != 0 {
		t.Fatalf("run failed with code %d: %s", code, stderr.String())
	}

	model, ok := capturedModel.(tui.Model)
	if !ok {
		t.Fatalf("captured model is not tui.Model: %T", capturedModel)
	}
	if model.Game.Rules.Profile != engine.ProfileCustom {
		t.Errorf("expected Custom profile after overrides, got %v", model.Game.Rules.Profile)
	}
	if model.Game.Rules.Surveillance != engine.SurveillanceFull {
		t.Errorf("expected SurveillanceFull, got %v", model.Game.Rules.Surveillance)
	}
	if model.Game.Rules.KlingonCloak != false {
		t.Errorf("expected KlingonCloak=false, got %v", model.Game.Rules.KlingonCloak)
	}
	if model.Game.Rules.RepairMultiplier != 1.25 {
		t.Errorf("expected RepairMultiplier=1.25, got %v", model.Game.Rules.RepairMultiplier)
	}
	if model.Game.Rules.SensorDegradation != false {
		t.Errorf("expected SensorDegradation=false, got %v", model.Game.Rules.SensorDegradation)
	}
}

func TestCLIModeFlag(t *testing.T) {
	origRunProgram := runProgram
	t.Cleanup(func() { runProgram = origRunProgram })

	tests := []struct {
		name         string
		args         []string
		expectedErr  bool
		expectedMode theme.ColorMode
		errSubstring string
	}{
		{
			name:         "flag mode light",
			args:         []string{"-mode", "light"},
			expectedErr:  false,
			expectedMode: theme.ColorModeLight,
		},
		{
			name:         "flag mode dark",
			args:         []string{"-mode", "dark"},
			expectedErr:  false,
			expectedMode: theme.ColorModeDark,
		},
		{
			name:         "flag mode auto",
			args:         []string{"-mode", "auto"},
			expectedErr:  false,
			expectedMode: theme.ColorModeAuto,
		},
		{
			name:         "long flag with equals light",
			args:         []string{"--mode=light"},
			expectedErr:  false,
			expectedMode: theme.ColorModeLight,
		},
		{
			name:         "long flag with equals dark",
			args:         []string{"--mode=dark"},
			expectedErr:  false,
			expectedMode: theme.ColorModeDark,
		},
		{
			name:         "flag mode case insensitive",
			args:         []string{"-mode", "DARK"},
			expectedErr:  false,
			expectedMode: theme.ColorModeDark,
		},
		{
			name:         "default without mode flag defaults to auto",
			args:         []string{},
			expectedErr:  false,
			expectedMode: theme.ColorModeAuto,
		},
		{
			name:         "combined theme and mode",
			args:         []string{"-theme", "lcars", "-mode", "light"},
			expectedErr:  false,
			expectedMode: theme.ColorModeLight,
		},
		{
			name:         "invalid mode",
			args:         []string{"-mode", "invalid"},
			expectedErr:  true,
			errSubstring: "invalid color mode",
		},
		{
			name:         "invalid mode unknown",
			args:         []string{"--mode=solar"},
			expectedErr:  true,
			errSubstring: "invalid color mode",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			var capturedModel tea.Model
			called := false
			runProgram = func(m tea.Model, opts ...tea.ProgramOption) error {
				called = true
				capturedModel = m
				return nil
			}

			var stdout, stderr bytes.Buffer
			exitCode := run(tt.args, strings.NewReader(""), &stdout, &stderr)

			if tt.expectedErr {
				if exitCode == 0 {
					t.Fatalf("expected non-zero exit code for args %v, got 0", tt.args)
				}
				if called {
					t.Fatalf("expected runProgram NOT to be called when mode is invalid")
				}
				if tt.errSubstring != "" && !strings.Contains(stderr.String(), tt.errSubstring) {
					t.Errorf("expected stderr to contain %q, got %q", tt.errSubstring, stderr.String())
				}
			} else {
				if exitCode != 0 {
					t.Fatalf("expected exit code 0 for args %v, got %d: %s", tt.args, exitCode, stderr.String())
				}
				if !called {
					t.Fatalf("expected runProgram to be called for args %v", tt.args)
				}
				model, ok := capturedModel.(tui.Model)
				if !ok {
					t.Fatalf("captured model is not tui.Model: %T", capturedModel)
				}
				if model.Theme.ColorMode() != tt.expectedMode {
					t.Errorf("expected ColorMode %v, got %v", tt.expectedMode, model.Theme.ColorMode())
				}
			}
		})
	}
}


