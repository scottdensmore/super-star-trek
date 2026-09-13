package tui

import (
	"math"
	"testing"

	"github.com/scottdensmore/super-star-trek/pkg/engine"
)

func TestParseCommandsBrief(t *testing.T) {
	// NAV course warp
	res := ParseCommand("nav 1.5 2")
	if res.Error != nil {
		t.Fatalf("unexpected error for nav: %v", res.Error)
	}
	mv, ok := res.Action.(engine.ActionMove)
	if !ok || mv.Course != 1.5 || mv.Warp != 2.0 {
		t.Fatalf("mismatch in parsed nav action: %+v", res.Action)
	}

	// PHA energy
	res = ParseCommand("pha 500")
	if res.Error != nil {
		t.Fatalf("unexpected error for pha: %v", res.Error)
	}
	pha, ok := res.Action.(engine.ActionFirePhasers)
	if !ok || pha.Energy != 500 {
		t.Fatalf("mismatch in parsed pha action: %+v", res.Action)
	}

	// Special commands: theme, help, quit
	if ParseCommand("theme lcars").Special != "theme lcars" {
		t.Fatalf("expected theme special command")
	}
	if ParseCommand("quit").Special != "quit" {
		t.Fatalf("expected quit special command")
	}
}

func TestParseNavCourseWarp(t *testing.T) {
	tests := []struct {
		input      string
		wantCourse float64
		wantWarp   float64
	}{
		{"nav 1.5 2", 1.5, 2.0},
		{"NAV 0.5 4.5", 0.5, 4.5},
		{"move 2.0 1.0", 2.0, 1.0},
		{"nav course 3.14 2", 3.14, 2.0},
		{"nav c 0.0 5", 0.0, 5.0},
		{"nav 1 10", 1.0, 10.0}, // 10 is > 8, so can't be sector
	}

	for _, tc := range tests {
		res := ParseCommand(tc.input)
		if res.Error != nil {
			t.Fatalf("ParseCommand(%q) unexpected error: %v", tc.input, res.Error)
		}
		mv, ok := res.Action.(engine.ActionMove)
		if !ok {
			t.Fatalf("ParseCommand(%q) action is %T, want ActionMove", tc.input, res.Action)
		}
		if math.Abs(mv.Course-tc.wantCourse) > 1e-6 {
			t.Errorf("ParseCommand(%q) Course = %v, want %v", tc.input, mv.Course, tc.wantCourse)
		}
		if math.Abs(mv.Warp-tc.wantWarp) > 1e-6 {
			t.Errorf("ParseCommand(%q) Warp = %v, want %v", tc.input, mv.Warp, tc.wantWarp)
		}
		if mv.DestSector != (engine.Coord{}) {
			t.Errorf("ParseCommand(%q) DestSector = %v, want empty", tc.input, mv.DestSector)
		}
	}
}

func TestParseNavSector(t *testing.T) {
	tests := []struct {
		input      string
		wantSector engine.Coord
	}{
		{"nav 4 5", engine.Coord{4, 5}},
		{"NAV 1 8", engine.Coord{1, 8}},
		{"move 8 1", engine.Coord{8, 1}},
		{"nav sector 2 3", engine.Coord{2, 3}},
		{"nav s 6 7", engine.Coord{6, 7}},
		{"nav 1 2", engine.Coord{1, 2}},
	}

	for _, tc := range tests {
		res := ParseCommand(tc.input)
		if res.Error != nil {
			t.Fatalf("ParseCommand(%q) unexpected error: %v", tc.input, res.Error)
		}
		mv, ok := res.Action.(engine.ActionMove)
		if !ok {
			t.Fatalf("ParseCommand(%q) action is %T, want ActionMove", tc.input, res.Action)
		}
		if mv.DestSector != tc.wantSector {
			t.Errorf("ParseCommand(%q) DestSector = %v, want %v", tc.input, mv.DestSector, tc.wantSector)
		}
		if mv.Warp != 1.0 {
			t.Errorf("ParseCommand(%q) Warp = %v, want 1.0", tc.input, mv.Warp)
		}
	}
}

func TestParseNavErrors(t *testing.T) {
	badInputs := []string{
		"nav",
		"nav 1",
		"nav abc def",
		"nav 1.5 -2",
		"nav 1 2 3 4",
		"nav s 9 9",
		"nav s 0 1",
	}

	for _, input := range badInputs {
		res := ParseCommand(input)
		if res.Error == nil {
			t.Errorf("ParseCommand(%q) expected error, got nil (action: %+v)", input, res.Action)
		}
	}
}

func TestParseTorpedo(t *testing.T) {
	// Course/angle form (1 arg)
	t.Run("Course", func(t *testing.T) {
		tests := []struct {
			input     string
			wantAngle float64
		}{
			{"tor 1.5", 1.5},
			{"tor 0", 0.0},
			{"torpedo 3.14159", 3.14159},
			{"TOR 4.2", 4.2},
		}
		for _, tc := range tests {
			res := ParseCommand(tc.input)
			if res.Error != nil {
				t.Fatalf("ParseCommand(%q) unexpected error: %v", tc.input, res.Error)
			}
			tor, ok := res.Action.(engine.ActionFireTorpedo)
			if !ok {
				t.Fatalf("ParseCommand(%q) action is %T, want ActionFireTorpedo", tc.input, res.Action)
			}
			if math.Abs(tor.Angle-tc.wantAngle) > 1e-6 {
				t.Errorf("ParseCommand(%q) Angle = %v, want %v", tc.input, tor.Angle, tc.wantAngle)
			}
			if tor.Target != (engine.Coord{}) {
				t.Errorf("ParseCommand(%q) Target = %v, want empty", tc.input, tor.Target)
			}
		}
	})

	// Sector target form (2 args)
	t.Run("SectorTarget", func(t *testing.T) {
		tests := []struct {
			input      string
			wantTarget engine.Coord
		}{
			{"tor 4 7", engine.Coord{4, 7}},
			{"TOR 1 1", engine.Coord{1, 1}},
			{"torpedo 8 8", engine.Coord{8, 8}},
		}
		for _, tc := range tests {
			res := ParseCommand(tc.input)
			if res.Error != nil {
				t.Fatalf("ParseCommand(%q) unexpected error: %v", tc.input, res.Error)
			}
			tor, ok := res.Action.(engine.ActionFireTorpedo)
			if !ok {
				t.Fatalf("ParseCommand(%q) action is %T, want ActionFireTorpedo", tc.input, res.Action)
			}
			if tor.Target != tc.wantTarget {
				t.Errorf("ParseCommand(%q) Target = %v, want %v", tc.input, tor.Target, tc.wantTarget)
			}
		}
	})

	// Errors
	t.Run("Errors", func(t *testing.T) {
		badInputs := []string{
			"tor",
			"tor abc",
			"tor 1 2 3",
			"tor 9 4",
			"tor 4 0",
			"tor -1 5",
			"tor 4 9",
		}
		for _, input := range badInputs {
			res := ParseCommand(input)
			if res.Error == nil {
				t.Errorf("ParseCommand(%q) expected error, got nil (action: %+v)", input, res.Action)
			}
		}
	})
}

func TestParsePhasers(t *testing.T) {
	tests := []struct {
		input      string
		wantEnergy float64
	}{
		{"pha 500", 500},
		{"phaser 1000", 1000},
		{"PHASERS 250.5", 250.5},
	}
	for _, tc := range tests {
		res := ParseCommand(tc.input)
		if res.Error != nil {
			t.Fatalf("ParseCommand(%q) unexpected error: %v", tc.input, res.Error)
		}
		pha, ok := res.Action.(engine.ActionFirePhasers)
		if !ok {
			t.Fatalf("ParseCommand(%q) action is %T, want ActionFirePhasers", tc.input, res.Action)
		}
		if math.Abs(pha.Energy-tc.wantEnergy) > 1e-6 {
			t.Errorf("ParseCommand(%q) Energy = %v, want %v", tc.input, pha.Energy, tc.wantEnergy)
		}
	}

	badInputs := []string{
		"pha",
		"pha abc",
		"pha -50",
		"pha 0",
		"pha 100 200",
	}
	for _, input := range badInputs {
		res := ParseCommand(input)
		if res.Error == nil {
			t.Errorf("ParseCommand(%q) expected error, got nil", input)
		}
	}
}

func TestParseShields(t *testing.T) {
	tests := []struct {
		input      string
		wantAmount float64
	}{
		{"she 500", 500},
		{"shield -200", -200},
		{"shields 0", 0},
		{"SHE 1000", 1000},
	}
	for _, tc := range tests {
		res := ParseCommand(tc.input)
		if res.Error != nil {
			t.Fatalf("ParseCommand(%q) unexpected error: %v", tc.input, res.Error)
		}
		she, ok := res.Action.(engine.ActionShields)
		if !ok {
			t.Fatalf("ParseCommand(%q) action is %T, want ActionShields", tc.input, res.Action)
		}
		if math.Abs(she.Amount-tc.wantAmount) > 1e-6 {
			t.Errorf("ParseCommand(%q) Amount = %v, want %v", tc.input, she.Amount, tc.wantAmount)
		}
	}

	badInputs := []string{
		"she",
		"she abc",
		"she 100 200",
	}
	for _, input := range badInputs {
		res := ParseCommand(input)
		if res.Error == nil {
			t.Errorf("ParseCommand(%q) expected error, got nil", input)
		}
	}
}

func TestParseDock(t *testing.T) {
	for _, input := range []string{"doc", "dock", "DOC", "DOCK"} {
		res := ParseCommand(input)
		if res.Error != nil {
			t.Fatalf("ParseCommand(%q) unexpected error: %v", input, res.Error)
		}
		if _, ok := res.Action.(engine.ActionDock); !ok {
			t.Fatalf("ParseCommand(%q) action is %T, want ActionDock", input, res.Action)
		}
	}
}

func TestParseSpecialCommands(t *testing.T) {
	tests := []struct {
		input       string
		wantSpecial string
	}{
		{"theme", "theme"},
		{"theme lcars", "theme lcars"},
		{"theme CRT", "theme crt"},
		{"THEME modern", "theme modern"},
		{"help", "help"},
		{"?", "help"},
		{"commands", "help"},
		{"quit", "quit"},
		{"exit", "quit"},
		{"q", "quit"},
	}

	for _, tc := range tests {
		res := ParseCommand(tc.input)
		if res.Error != nil {
			t.Fatalf("ParseCommand(%q) unexpected error: %v", tc.input, res.Error)
		}
		if res.Special != tc.wantSpecial {
			t.Errorf("ParseCommand(%q) Special = %q, want %q", tc.input, res.Special, tc.wantSpecial)
		}
	}
}

func TestParseUnknownAndEmpty(t *testing.T) {
	badInputs := []string{
		"",
		"   ",
		"\t\n",
		"xyz 123",
		"foo",
		"fire",
	}

	for _, input := range badInputs {
		res := ParseCommand(input)
		if res.Error == nil {
			t.Errorf("ParseCommand(%q) expected error, got nil", input)
		}
	}
}

func TestParseNavQuadrant(t *testing.T) {
	tests := []struct {
		input     string
		wantQuad  engine.Coord
		wantWarp  float64
		wantError bool
	}{
		{"nav q 3 5", engine.Coord{3, 5}, 1.0, false},
		{"NAV QUAD 1 8", engine.Coord{1, 8}, 1.0, false},
		{"move quadrant 7 2", engine.Coord{7, 2}, 1.0, false},
		{"nav q 4 6 2.5", engine.Coord{4, 6}, 2.5, false},
		{"nav q 0 5", engine.Coord{}, 0, true},
		{"nav q 9 1", engine.Coord{}, 0, true},
		{"nav q 4 9", engine.Coord{}, 0, true},
		{"nav q abc 2", engine.Coord{}, 0, true},
		{"nav q 2 xyz", engine.Coord{}, 0, true},
		{"nav q 2 3 -1", engine.Coord{}, 0, true},
	}

	for _, tc := range tests {
		res := ParseCommand(tc.input)
		if tc.wantError {
			if res.Error == nil {
				t.Errorf("ParseCommand(%q) expected error, got nil", tc.input)
			}
			continue
		}
		if res.Error != nil {
			t.Fatalf("ParseCommand(%q) unexpected error: %v", tc.input, res.Error)
		}
		mv, ok := res.Action.(engine.ActionMove)
		if !ok {
			t.Fatalf("ParseCommand(%q) action is %T, want ActionMove", tc.input, res.Action)
		}
		if mv.DestQuad != tc.wantQuad {
			t.Errorf("ParseCommand(%q) DestQuad = %v, want %v", tc.input, mv.DestQuad, tc.wantQuad)
		}
		if math.Abs(mv.Warp-tc.wantWarp) > 1e-6 {
			t.Errorf("ParseCommand(%q) Warp = %v, want %v", tc.input, mv.Warp, tc.wantWarp)
		}
	}
}

func TestParseHelpContext(t *testing.T) {
	tests := []struct {
		input       string
		wantSpecial string
	}{
		{"help", "help"},
		{"?", "help"},
		{"commands", "help"},
		{"help nav", "help nav"},
		{"help move", "help nav"},
		{"help warp", "help nav"},
		{"help tor", "help tor"},
		{"help torpedo", "help tor"},
		{"help pha", "help pha"},
		{"help phasers", "help pha"},
		{"help she", "help she"},
		{"help shields", "help she"},
		{"help doc", "help doc"},
		{"help dock", "help doc"},
		{"help saves", "help saves"},
		{"help thaw", "help saves"},
	}

	for _, tc := range tests {
		res := ParseCommand(tc.input)
		if res.Error != nil {
			t.Fatalf("ParseCommand(%q) unexpected error: %v", tc.input, res.Error)
		}
		if res.Special != tc.wantSpecial {
			t.Errorf("ParseCommand(%q) Special = %q, want %q", tc.input, res.Special, tc.wantSpecial)
		}
	}
}

func TestParseOptionsCommand(t *testing.T) {
	for _, cmd := range []string{"opts", "options", "settings"} {
		parsed := ParseCommand(cmd)
		if parsed.Special != "options" {
			t.Errorf("command %q: expected Special 'options', got %q", cmd, parsed.Special)
		}
	}
}

