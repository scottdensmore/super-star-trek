package classic

import (
	"bytes"
	"strings"
	"testing"
)

func TestRunClassicCLI_ClassicMode(t *testing.T) {
	in := strings.NewReader("")
	var out bytes.Buffer
	args := []string{"-classic", "-seed", "12345"}

	exitCode := RunClassicCLI(in, &out, args)
	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d", exitCode)
	}

	output := out.String()
	if !strings.Contains(output, "Super Star Trek (Go Edition)") {
		t.Errorf("expected banner in output, got %q", output)
	}
	if !strings.Contains(output, "Energy:") || !strings.Contains(output, "Shields:") {
		t.Errorf("expected initial status in output, got %q", output)
	}
}

func TestRunClassicCLI_DefaultPlaceholder(t *testing.T) {
	in := strings.NewReader("")
	var out bytes.Buffer
	args := []string{}

	exitCode := RunClassicCLI(in, &out, args)
	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d", exitCode)
	}

	output := out.String()
	expected := "Charmbracelet TUI placeholder - use --classic for teletype mode.\n"
	if output != expected {
		t.Fatalf("expected %q, got %q", expected, output)
	}
}

func TestRunClassicCLI_InvalidFlag(t *testing.T) {
	in := strings.NewReader("")
	var out bytes.Buffer
	args := []string{"-nonexistent-flag"}

	exitCode := RunClassicCLI(in, &out, args)
	if exitCode == 0 {
		t.Fatalf("expected non-zero exit code for invalid flag, got 0")
	}
}
