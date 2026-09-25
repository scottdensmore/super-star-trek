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

func TestCLIFlags_SpatialAnomalies(t *testing.T) {
	origRunProgram := runProgram
	t.Cleanup(func() { runProgram = origRunProgram })

	var capturedModel tea.Model
	runProgram = func(m tea.Model, opts ...tea.ProgramOption) error {
		capturedModel = m
		return nil
	}

	tests := []struct {
		name          string
		args          []string
		wantAnomalies bool
		wantProfile   engine.DifficultyProfile
	}{
		{
			name:          "default normal difficulty has anomalies disabled",
			args:          []string{"-seed", "42"},
			wantAnomalies: false,
			wantProfile:   engine.ProfileNormal,
		},
		{
			name:          "enable anomalies with --anomalies",
			args:          []string{"--anomalies", "-seed", "42"},
			wantAnomalies: true,
			wantProfile:   engine.ProfileCustom,
		},
		{
			name:          "enable anomalies with -anomalies",
			args:          []string{"-anomalies", "-seed", "42"},
			wantAnomalies: true,
			wantProfile:   engine.ProfileCustom,
		},
		{
			name:          "disable anomalies with --no-anomalies on nightmare",
			args:          []string{"--difficulty=nightmare", "--no-anomalies", "-seed", "42"},
			wantAnomalies: false,
			wantProfile:   engine.ProfileCustom,
		},
		{
			name:          "disable anomalies with -no-anomalies",
			args:          []string{"-difficulty=nightmare", "-no-anomalies", "-seed", "42"},
			wantAnomalies: false,
			wantProfile:   engine.ProfileCustom,
		},
		{
			name:          "anomalies flag set to false",
			args:          []string{"--difficulty=nightmare", "--anomalies=false", "-seed", "42"},
			wantAnomalies: false,
			wantProfile:   engine.ProfileCustom,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			var stdout, stderr bytes.Buffer
			code := run(tc.args, strings.NewReader(""), &stdout, &stderr)
			if code != 0 {
				t.Fatalf("run failed with code %d: %s", code, stderr.String())
			}
			model, ok := capturedModel.(tui.Model)
			if !ok {
				t.Fatalf("captured model is not tui.Model: %T", capturedModel)
			}
			if model.Game.Rules.SpatialAnomalies != tc.wantAnomalies {
				t.Errorf("expected SpatialAnomalies=%v, got %v", tc.wantAnomalies, model.Game.Rules.SpatialAnomalies)
			}
			if model.Game.Rules.Profile != tc.wantProfile {
				t.Errorf("expected Profile=%v, got %v", tc.wantProfile, model.Game.Rules.Profile)
			}
		})
	}
}

func TestCLIFlags_SpatialAnomalies_MutualExclusion(t *testing.T) {
	origRunProgram := runProgram
	t.Cleanup(func() { runProgram = origRunProgram })

	called := false
	runProgram = func(m tea.Model, opts ...tea.ProgramOption) error {
		called = true
		return nil
	}

	var stdout, stderr bytes.Buffer
	code := run([]string{"--anomalies", "--no-anomalies"}, strings.NewReader(""), &stdout, &stderr)
	if code != 1 {
		t.Fatalf("expected exit code 1 when specifying both flags, got %d", code)
	}
	if called {
		t.Fatalf("expected runProgram NOT to be called")
	}
	expectedErr := "Error: cannot specify both --anomalies and --no-anomalies"
	if !strings.Contains(stderr.String(), expectedErr) {
		t.Errorf("expected stderr to contain %q, got %q", expectedErr, stderr.String())
	}
}

func TestCLIFlags_Sound(t *testing.T) {
	origRunProgram := runProgram
	t.Cleanup(func() { runProgram = origRunProgram })

	var capturedModel tea.Model
	runProgram = func(m tea.Model, opts ...tea.ProgramOption) error {
		capturedModel = m
		return nil
	}

	tests := []struct {
		name        string
		args        []string
		wantEnabled bool
	}{
		{
			name:        "default has sound enabled",
			args:        []string{"-seed", "42"},
			wantEnabled: true,
		},
		{
			name:        "enable sound explicitly with --sound",
			args:        []string{"--sound", "-seed", "42"},
			wantEnabled: true,
		},
		{
			name:        "enable sound explicitly with -sound",
			args:        []string{"-sound", "-seed", "42"},
			wantEnabled: true,
		},
		{
			name:        "disable sound with --no-sound",
			args:        []string{"--no-sound", "-seed", "42"},
			wantEnabled: false,
		},
		{
			name:        "disable sound with -no-sound",
			args:        []string{"-no-sound", "-seed", "42"},
			wantEnabled: false,
		},
		{
			name:        "sound flag set to false with --sound=false",
			args:        []string{"--sound=false", "-seed", "42"},
			wantEnabled: false,
		},
		{
			name:        "no-sound flag set to false with --no-sound=false",
			args:        []string{"--no-sound=false", "-seed", "42"},
			wantEnabled: true,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			var stdout, stderr bytes.Buffer
			code := run(tc.args, strings.NewReader(""), &stdout, &stderr)
			if code != 0 {
				t.Fatalf("run failed with code %d: %s", code, stderr.String())
			}
			model, ok := capturedModel.(tui.Model)
			if !ok {
				t.Fatalf("captured model is not tui.Model: %T", capturedModel)
			}
			if model.SoundEnabled() != tc.wantEnabled {
				t.Errorf("expected SoundEnabled=%v, got %v", tc.wantEnabled, model.SoundEnabled())
			}
		})
	}
}

func TestCLIFlags_Sound_MutualExclusion(t *testing.T) {
	origRunProgram := runProgram
	t.Cleanup(func() { runProgram = origRunProgram })

	called := false
	runProgram = func(m tea.Model, opts ...tea.ProgramOption) error {
		called = true
		return nil
	}

	// Long flags: --sound and --no-sound
	var stdout, stderr bytes.Buffer
	code := run([]string{"--sound", "--no-sound"}, strings.NewReader(""), &stdout, &stderr)
	if code != 1 {
		t.Fatalf("expected exit code 1 when specifying both flags, got %d", code)
	}
	if called {
		t.Fatalf("expected runProgram NOT to be called")
	}
	expectedErr := "Error: cannot specify both --sound and --no-sound"
	if !strings.Contains(stderr.String(), expectedErr) {
		t.Errorf("expected stderr to contain %q, got %q", expectedErr, stderr.String())
	}

	// Short flags: -sound and -no-sound
	stdout.Reset()
	stderr.Reset()
	called = false
	code = run([]string{"-sound", "-no-sound"}, strings.NewReader(""), &stdout, &stderr)
	if code != 1 {
		t.Fatalf("expected exit code 1 when specifying both flags with single dash, got %d", code)
	}
	if called {
		t.Fatalf("expected runProgram NOT to be called")
	}
	if !strings.Contains(stderr.String(), expectedErr) {
		t.Errorf("expected stderr to contain %q, got %q", expectedErr, stderr.String())
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

func TestCLI_ListScenarios(t *testing.T) {
	out := &bytes.Buffer{}
	errOut := &bytes.Buffer{}
	code := run([]string{"--list-scenarios"}, strings.NewReader(""), out, errOut)
	if code != 0 {
		t.Errorf("expected exit code 0, got %d", code)
	}
	output := out.String()
	if !strings.Contains(output, "kobayashi-maru") || !strings.Contains(output, "mutara-nebula") {
		t.Errorf("expected scenario list in output, got: %s", output)
	}
}

func TestCLI_ScenarioLaunch_Unknown(t *testing.T) {
	out := &bytes.Buffer{}
	errOut := &bytes.Buffer{}
	code := run([]string{"--scenario=nonexistent"}, strings.NewReader(""), out, errOut)
	if code != 1 {
		t.Errorf("expected exit code 1 for invalid scenario, got %d", code)
	}
	if !strings.Contains(errOut.String(), "unknown scenario") {
		t.Errorf("expected 'unknown scenario' error, got: %s", errOut.String())
	}
}

func TestCLI_ScenarioLaunch(t *testing.T) {
	origRunProgram := runProgram
	t.Cleanup(func() { runProgram = origRunProgram })

	var capturedModel tea.Model
	runProgram = func(m tea.Model, opts ...tea.ProgramOption) error {
		capturedModel = m
		return nil
	}

	out := &bytes.Buffer{}
	errOut := &bytes.Buffer{}
	code := run([]string{"--scenario", "mutara-nebula"}, strings.NewReader(""), out, errOut)
	if code != 0 {
		t.Fatalf("expected exit code 0, got %d: %s", code, errOut.String())
	}
	model, ok := capturedModel.(tui.Model)
	if !ok {
		t.Fatalf("captured model is not tui.Model: %T", capturedModel)
	}
	if model.Game.Scenario != engine.ScenarioMutaraNebula {
		t.Errorf("expected scenario %v, got %v", engine.ScenarioMutaraNebula, model.Game.Scenario)
	}
}

func TestCLI_ScenarioLaunch_Shorthand(t *testing.T) {
	origRunProgram := runProgram
	t.Cleanup(func() { runProgram = origRunProgram })

	var capturedModel tea.Model
	runProgram = func(m tea.Model, opts ...tea.ProgramOption) error {
		capturedModel = m
		return nil
	}

	out := &bytes.Buffer{}
	errOut := &bytes.Buffer{}
	code := run([]string{"-s", "kobayashi"}, strings.NewReader(""), out, errOut)
	if code != 0 {
		t.Fatalf("expected exit code 0, got %d: %s", code, errOut.String())
	}
	model, ok := capturedModel.(tui.Model)
	if !ok {
		t.Fatalf("captured model is not tui.Model: %T", capturedModel)
	}
	if model.Game.Scenario != engine.ScenarioKobayashiMaru {
		t.Errorf("expected scenario %v, got %v", engine.ScenarioKobayashiMaru, model.Game.Scenario)
	}
}

func TestMain_VersionFlag(t *testing.T) {
	cases := []struct {
		name string
		flag string
	}{
		{"LongFlag", "--version"},
		{"ShortFlag", "-v"},
	}

	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			var out, errOut bytes.Buffer
			code := run([]string{tc.flag}, strings.NewReader(""), &out, &errOut)
			if code != 0 {
				t.Fatalf("expected exit code 0 for %s, got %d. stderr: %s", tc.flag, code, errOut.String())
			}
			stdout := out.String()
			if !strings.Contains(stdout, "sst version") {
				t.Errorf("expected 'sst version' in output, got: %q", stdout)
			}
			if !strings.Contains(stdout, "commit:") {
				t.Errorf("expected 'commit:' in output, got: %q", stdout)
			}
			if !strings.Contains(stdout, "built at:") {
				t.Errorf("expected 'built at:' in output, got: %q", stdout)
			}
		})
	}
}

func TestMain_TourFlag(t *testing.T) {
	var out, errOut bytes.Buffer
	// Provide --help with tour flag check
	code := run([]string{"--help"}, strings.NewReader(""), &out, &errOut)
	if code != 0 {
		t.Fatalf("expected code 0, got %d", code)
	}
	if !strings.Contains(out.String(), "--tour") {
		t.Errorf("expected --tour flag documented in help output, got:\n%s", out.String())
	}
}

func TestCLI_TourLaunch(t *testing.T) {
	origRunProgram := runProgram
	t.Cleanup(func() { runProgram = origRunProgram })

	var capturedModel tea.Model
	runProgram = func(m tea.Model, opts ...tea.ProgramOption) error {
		capturedModel = m
		return nil
	}

	out := &bytes.Buffer{}
	errOut := &bytes.Buffer{}
	code := run([]string{"--tour", "-seed", "42"}, strings.NewReader(""), out, errOut)
	if code != 0 {
		t.Fatalf("expected exit code 0, got %d: %s", code, errOut.String())
	}
	model, ok := capturedModel.(tui.Model)
	if !ok {
		t.Fatalf("captured model is not tui.Model: %T", capturedModel)
	}
	if model.Tour == nil || !model.Tour.Active {
		t.Errorf("expected active Tour in model")
	}
	if model.Tour.CurrentSectorIndex != 0 {
		t.Errorf("expected sector index 0, got %d", model.Tour.CurrentSectorIndex)
	}
	if model.Game == nil {
		t.Errorf("expected Game initialized in model")
	}
}

func TestCLI_CampaignLaunch(t *testing.T) {
	origRunProgram := runProgram
	t.Cleanup(func() { runProgram = origRunProgram })

	var capturedModel tea.Model
	runProgram = func(m tea.Model, opts ...tea.ProgramOption) error {
		capturedModel = m
		return nil
	}

	out := &bytes.Buffer{}
	errOut := &bytes.Buffer{}
	code := run([]string{"--campaign", "-seed", "42"}, strings.NewReader(""), out, errOut)
	if code != 0 {
		t.Fatalf("expected exit code 0, got %d: %s", code, errOut.String())
	}
	model, ok := capturedModel.(tui.Model)
	if !ok {
		t.Fatalf("captured model is not tui.Model: %T", capturedModel)
	}
	if model.Tour == nil || !model.Tour.Active {
		t.Errorf("expected active Tour in model")
	}
}



