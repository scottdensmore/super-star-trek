package main

import (
	"strings"
	"testing"

	"github.com/scottdensmore/super-star-trek/pkg/engine"
)

func TestSession_NewAndExecute(t *testing.T) {
	s := NewSession(12345, engine.ProfileNormal)
	if s == nil || s.Game() == nil {
		t.Fatalf("expected non-nil session and game")
	}

	// Test help command
	helpOut := s.Execute("help")
	if !strings.Contains(helpOut, "COMMANDS:") {
		t.Errorf("expected help output to list commands, got: %s", helpOut)
	}
	if !strings.Contains(helpOut, "save [slot], load [slot]") {
		t.Errorf("expected help output to list save and load commands, got: %s", helpOut)
	}

	// Test status command
	statusOut := s.Execute("srs")
	if !strings.Contains(statusOut, "CONDITION") || !strings.Contains(statusOut, "<E>") {
		t.Errorf("expected short-range scan output, got: %s", statusOut)
	}

	// Test shields command
	shieldOut := s.Execute("she 500")
	if !strings.Contains(shieldOut, "Shields") {
		t.Errorf("expected shield transfer output, got: %s", shieldOut)
	}
}

func TestSession_SaveAndLoad(t *testing.T) {
	s1 := NewSession(12345, engine.ProfileNormal)
	s1.Execute("she 500")

	savedData, err := s1.Save()
	if err != nil {
		t.Fatalf("Save() failed: %v", err)
	}

	s2 := NewSession(99999, engine.ProfileHardcore)
	if err := s2.Load(savedData); err != nil {
		t.Fatalf("Load() failed: %v", err)
	}

	if s2.Game().Enterprise.Shields != s1.Game().Enterprise.Shields {
		t.Errorf("loaded shields = %v, want %v", s2.Game().Enterprise.Shields, s1.Game().Enterprise.Shields)
	}
}

func TestSession_EmptyAndInvalidCommands(t *testing.T) {
	s := NewSession(0, engine.ProfileNormal)
	if s == nil {
		t.Fatalf("expected session with seed 0 to initialize")
	}

	// Empty input
	if out := s.Execute(""); out != "" {
		t.Errorf("expected empty string for empty input, got: %q", out)
	}
	if out := s.Execute("   "); out != "" {
		t.Errorf("expected empty string for whitespace input, got: %q", out)
	}

	// Unknown / syntax error commands
	badOut := s.Execute("foobar")
	if !strings.Contains(badOut, "Error:") && !strings.Contains(badOut, "unknown") {
		t.Errorf("expected error message for unknown command, got: %q", badOut)
	}

	// Action error (exceeding available energy)
	errOut := s.Execute("she 999999")
	if !strings.Contains(errOut, "Cannot execute") {
		t.Errorf("expected execution error for excessive shields, got: %q", errOut)
	}
}

func TestSession_SpecialCommands(t *testing.T) {
	s := NewSession(12345, engine.ProfileNormal)

	// LRS
	lrsOut := s.Execute("lrs")
	if !strings.Contains(lrsOut, "LONG RANGE SENSOR SCAN") {
		t.Errorf("expected LRS scan output, got: %q", lrsOut)
	}

	// Chart
	chartOut := s.Execute("chart")
	if !strings.Contains(chartOut, "GALACTIC STAR CHART") {
		t.Errorf("expected chart output, got: %q", chartOut)
	}

	// Damages
	damOut := s.Execute("dam")
	if !strings.Contains(damOut, "DAMAGE CONTROL REPORT") {
		t.Errorf("expected damage report output, got: %q", damOut)
	}

	// Move command appends SRS
	navOut := s.Execute("nav 1 1")
	if !strings.Contains(navOut, "<E>") {
		t.Errorf("expected move command to append SRS output, got: %q", navOut)
	}
}

func TestSession_LoadCorruptedData(t *testing.T) {
	s := NewSession(12345, engine.ProfileNormal)
	err := s.Load("this is not valid json")
	if err == nil {
		t.Errorf("expected error loading invalid JSON, got nil")
	}
}

func TestSession_QuitAndExit(t *testing.T) {
	s := NewSession(12345, engine.ProfileNormal)

	for _, cmd := range []string{"quit", "exit", "q", "QUIT", "EXIT"} {
		out := s.Execute(cmd)
		if !strings.Contains(out, "Session terminated") {
			t.Errorf("command %q: expected session termination message, got: %q", cmd, out)
		}
	}
}

func TestSession_ScenarioCommands(t *testing.T) {
	s := NewSession(42)
	listOutput := s.Execute("scenario list")
	if !strings.Contains(listOutput, "kobayashi-maru") {
		t.Errorf("expected scenario list in WASM output, got: %s", listOutput)
	}

	loadOutput := s.Execute("scenario mutara")
	if !strings.Contains(loadOutput, "MUTARA NEBULA") {
		t.Errorf("expected Mutara briefing or header, got: %s", loadOutput)
	}
	if s.game.Scenario != engine.ScenarioMutaraNebula {
		t.Errorf("expected game scenario to be %v, got %v", engine.ScenarioMutaraNebula, s.game.Scenario)
	}
}

func TestSession_ScenarioCommands_Unknown(t *testing.T) {
	s := NewSession(42)
	out := s.Execute("scenario nonexistent")
	if !strings.Contains(strings.ToLower(out), "unknown scenario: nonexistent") {
		t.Errorf("expected unknown scenario error, got: %s", out)
	}

	multiOut := s.Execute("scenario starbase assault")
	if !strings.Contains(strings.ToLower(multiOut), "unknown scenario: starbase-assault") {
		t.Errorf("expected multi-word unknown scenario error, got: %s", multiOut)
	}
}

