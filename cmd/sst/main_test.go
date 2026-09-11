package main

import (
	"bytes"
	"errors"
	"strings"
	"testing"

	tea "github.com/charmbracelet/bubbletea"
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
