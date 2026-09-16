package theme

import (
	"testing"
)

func TestThemeCyclingAndLookup(t *testing.T) {
	def := DefaultTheme()
	if def.Name() != "modern" {
		t.Fatalf("expected modern as default, got %s", def.Name())
	}

	next := def.Next()
	if next.Name() != "lcars" {
		t.Fatalf("expected lcars next, got %s", next.Name())
	}

	third := next.Next()
	if third.Name() != "crt" {
		t.Fatalf("expected crt next, got %s", third.Name())
	}

	backToDef := third.Next()
	if backToDef.Name() != "modern" {
		t.Fatalf("expected cycle back to modern, got %s", backToDef.Name())
	}

	if GetTheme("lcars").Name() != "lcars" {
		t.Fatalf("failed to retrieve lcars by name")
	}
	if GetTheme("unknown").Name() != "modern" {
		t.Fatalf("unknown theme should fallback to modern")
	}
}

func TestThemeCaseInsensitiveLookup(t *testing.T) {
	tests := []struct {
		input    string
		expected string
	}{
		{"modern", "modern"},
		{"MODERN", "modern"},
		{"Lcars", "lcars"},
		{"LCARS", "lcars"},
		{"  crt  ", "crt"},
		{"CRT", "crt"},
		{"nonexistent", "modern"},
		{"", "modern"},
	}

	for _, tc := range tests {
		th := GetTheme(tc.input)
		if th.Name() != tc.expected {
			t.Errorf("GetTheme(%q).Name() = %q; want %q", tc.input, th.Name(), tc.expected)
		}
	}
}

func TestStylesNonNull(t *testing.T) {
	themes := []Theme{ModernTheme{}, LcarsTheme{}, CrtTheme{}}
	for _, th := range themes {
		s := th.Styles()
		if s.Title.Render("test") == "" {
			t.Errorf("theme %s failed to render title", th.Name())
		}
		if s.Panel.Render("panel") == "" {
			t.Errorf("theme %s failed to render panel", th.Name())
		}
		if s.Grid.Render("grid") == "" {
			t.Errorf("theme %s failed to render grid", th.Name())
		}
		if s.ConditionGreen.Render("GREEN") == "" {
			t.Errorf("theme %s failed to render ConditionGreen", th.Name())
		}
		if s.ConditionYellow.Render("YELLOW") == "" {
			t.Errorf("theme %s failed to render ConditionYellow", th.Name())
		}
		if s.ConditionRed.Render("RED") == "" {
			t.Errorf("theme %s failed to render ConditionRed", th.Name())
		}
		if s.ConditionDocked.Render("DOCKED") == "" {
			t.Errorf("theme %s failed to render ConditionDocked", th.Name())
		}
		if s.Prompt.Render("COMMAND> ") == "" {
			t.Errorf("theme %s failed to render Prompt", th.Name())
		}
	}
}

func TestColorModeResolution(t *testing.T) {
	tests := []struct {
		mode         ColorMode
		terminalDark bool
		expectedDark bool
	}{
		{ColorModeAuto, true, true},
		{ColorModeAuto, false, false},
		{ColorModeDark, true, true},
		{ColorModeDark, false, true},
		{ColorModeLight, true, false},
		{ColorModeLight, false, false},
	}

	for _, tt := range tests {
		got := tt.mode.Resolve(tt.terminalDark)
		if got != tt.expectedDark {
			t.Errorf("%s.Resolve(%v) = %v, expected %v", tt.mode, tt.terminalDark, got, tt.expectedDark)
		}
	}
}

func TestColorModeNext(t *testing.T) {
	if ColorModeAuto.Next() != ColorModeDark {
		t.Errorf("expected Auto.Next() == Dark, got %s", ColorModeAuto.Next())
	}
	if ColorModeDark.Next() != ColorModeLight {
		t.Errorf("expected Dark.Next() == Light, got %s", ColorModeDark.Next())
	}
	if ColorModeLight.Next() != ColorModeAuto {
		t.Errorf("expected Light.Next() == Auto, got %s", ColorModeLight.Next())
	}
}

func TestParseColorMode(t *testing.T) {
	tests := []struct {
		input       string
		expected    ColorMode
		expectError bool
	}{
		{"auto", ColorModeAuto, false},
		{"AUTO", ColorModeAuto, false},
		{"dark", ColorModeDark, false},
		{"Dark", ColorModeDark, false},
		{"light", ColorModeLight, false},
		{"LIGHT", ColorModeLight, false},
		{"invalid", ColorModeAuto, true},
	}

	for _, tt := range tests {
		got, err := ParseColorMode(tt.input)
		if tt.expectError && err == nil {
			t.Errorf("expected error for input %q, got nil", tt.input)
		}
		if !tt.expectError && err != nil {
			t.Errorf("unexpected error for input %q: %v", tt.input, err)
		}
		if got != tt.expected {
			t.Errorf("ParseColorMode(%q) = %v, expected %v", tt.input, got, tt.expected)
		}
	}
}

func TestDefaultThemeColorMode(t *testing.T) {
	th := DefaultTheme()
	if th.ColorMode() != ColorModeAuto {
		t.Errorf("expected DefaultTheme().ColorMode() == %v, got %v", ColorModeAuto, th.ColorMode())
	}
}

