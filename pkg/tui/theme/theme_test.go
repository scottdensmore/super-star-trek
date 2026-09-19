package theme

import (
	"testing"

	"github.com/charmbracelet/lipgloss"
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

func TestThemePaletteStyles(t *testing.T) {
	themes := []Theme{
		ModernTheme{},
		LcarsTheme{},
		CrtTheme{},
	}

	for _, th := range themes {
		darkStyles := th.PaletteStyles(true)
		lightStyles := th.PaletteStyles(false)

		// Assert dark and light styles are non-empty
		if darkStyles.Title.GetForeground() == (lipgloss.NoColor{}) || darkStyles.Title.GetForeground() == nil ||
			lightStyles.Title.GetForeground() == (lipgloss.NoColor{}) || lightStyles.Title.GetForeground() == nil {
			t.Errorf("theme %s has empty title foreground", th.Name())
		}
		if darkStyles.Panel.GetBorderTopForeground() == (lipgloss.NoColor{}) || darkStyles.Panel.GetBorderTopForeground() == nil ||
			lightStyles.Panel.GetBorderTopForeground() == (lipgloss.NoColor{}) || lightStyles.Panel.GetBorderTopForeground() == nil {
			t.Errorf("theme %s has empty panel border foreground", th.Name())
		}

		// Dark and light palettes must differ in panel or title background
		darkBg := darkStyles.Panel.GetBackground()
		lightBg := lightStyles.Panel.GetBackground()
		if darkBg == lightBg {
			t.Errorf("theme %s dark and light panel background must not be identical (%q)", th.Name(), darkBg)
		}

		// WithColorMode should update ColorMode and produce correct Styles()
		thDark := th.WithColorMode(ColorModeDark)
		if thDark.ColorMode() != ColorModeDark {
			t.Errorf("expected theme %s to have ColorModeDark, got %s", th.Name(), thDark.ColorMode())
		}
		if thDark.Styles().Panel.GetBackground() != darkBg {
			t.Errorf("theme %s thDark.Styles() panel background (%q) != darkBg (%q)", th.Name(), thDark.Styles().Panel.GetBackground(), darkBg)
		}

		thLight := th.WithColorMode(ColorModeLight)
		if thLight.ColorMode() != ColorModeLight {
			t.Errorf("expected theme %s to have ColorModeLight, got %s", th.Name(), thLight.ColorMode())
		}
		if thLight.Styles().Panel.GetBackground() != lightBg {
			t.Errorf("theme %s thLight.Styles() panel background (%q) != lightBg (%q)", th.Name(), thLight.Styles().Panel.GetBackground(), lightBg)
		}
	}
}

func TestModernThemePaletteColors(t *testing.T) {
	th := ModernTheme{}
	dark := th.PaletteStyles(true)
	light := th.PaletteStyles(false)

	// Dark Modern
	if dark.Panel.GetBackground() != lipgloss.Color("#0B0F19") {
		t.Errorf("expected dark panel bg #0B0F19, got %v", dark.Panel.GetBackground())
	}
	if dark.Panel.GetBorderTopForeground() != lipgloss.Color("#1E293B") {
		t.Errorf("expected dark panel border #1E293B, got %v", dark.Panel.GetBorderTopForeground())
	}
	if dark.Enterprise.GetForeground() != lipgloss.Color("#00E5FF") {
		t.Errorf("expected dark enterprise fg #00E5FF, got %v", dark.Enterprise.GetForeground())
	}
	if dark.Klingon.GetForeground() != lipgloss.Color("#FF3344") {
		t.Errorf("expected dark klingon fg #FF3344, got %v", dark.Klingon.GetForeground())
	}
	if dark.Starbase.GetForeground() != lipgloss.Color("#FFD700") {
		t.Errorf("expected dark starbase fg #FFD700, got %v", dark.Starbase.GetForeground())
	}
	if dark.Star.GetForeground() != lipgloss.Color("#FFFFFF") {
		t.Errorf("expected dark star fg #FFFFFF, got %v", dark.Star.GetForeground())
	}
	if dark.Planet.GetForeground() != lipgloss.Color("#00E676") {
		t.Errorf("expected dark planet fg #00E676, got %v", dark.Planet.GetForeground())
	}
	if dark.BlackHole.GetForeground() != lipgloss.Color("#A855F7") {
		t.Errorf("expected dark black hole fg #A855F7, got %v", dark.BlackHole.GetForeground())
	}
	if dark.Wormhole.GetForeground() != lipgloss.Color("#04D9FF") {
		t.Errorf("expected dark wormhole fg #04D9FF, got %v", dark.Wormhole.GetForeground())
	}
	if dark.Empty.GetForeground() != lipgloss.Color("#334155") {
		t.Errorf("expected dark empty fg #334155, got %v", dark.Empty.GetForeground())
	}

	// Light Modern
	if light.Panel.GetBackground() != lipgloss.Color("#F8FAFC") {
		t.Errorf("expected light panel bg #F8FAFC, got %v", light.Panel.GetBackground())
	}
	if light.Panel.GetBorderTopForeground() != lipgloss.Color("#CBD5E1") {
		t.Errorf("expected light panel border #CBD5E1, got %v", light.Panel.GetBorderTopForeground())
	}
	if light.Title.GetForeground() != lipgloss.Color("#0284C7") {
		t.Errorf("expected light title fg #0284C7, got %v", light.Title.GetForeground())
	}
	if light.Enterprise.GetForeground() != lipgloss.Color("#0284C7") {
		t.Errorf("expected light enterprise fg #0284C7, got %v", light.Enterprise.GetForeground())
	}
	if light.Klingon.GetForeground() != lipgloss.Color("#DC2626") {
		t.Errorf("expected light klingon fg #DC2626, got %v", light.Klingon.GetForeground())
	}
	if light.Starbase.GetForeground() != lipgloss.Color("#D97706") {
		t.Errorf("expected light starbase fg #D97706, got %v", light.Starbase.GetForeground())
	}
	if light.Star.GetForeground() != lipgloss.Color("#475569") {
		t.Errorf("expected light star fg #475569, got %v", light.Star.GetForeground())
	}
	if light.Planet.GetForeground() != lipgloss.Color("#16A34A") {
		t.Errorf("expected light planet fg #16A34A, got %v", light.Planet.GetForeground())
	}
	if light.BlackHole.GetForeground() != lipgloss.Color("#7C3AED") {
		t.Errorf("expected light black hole fg #7C3AED, got %v", light.BlackHole.GetForeground())
	}
	if light.Wormhole.GetForeground() != lipgloss.Color("#0284C7") {
		t.Errorf("expected light wormhole fg #0284C7, got %v", light.Wormhole.GetForeground())
	}
	if light.Empty.GetForeground() != lipgloss.Color("#94A3B8") {
		t.Errorf("expected light empty fg #94A3B8, got %v", light.Empty.GetForeground())
	}
	if light.ConditionGreen.GetForeground() != lipgloss.Color("#16A34A") {
		t.Errorf("expected light condition green #16A34A, got %v", light.ConditionGreen.GetForeground())
	}
	if light.ConditionYellow.GetForeground() != lipgloss.Color("#D97706") {
		t.Errorf("expected light condition yellow #D97706, got %v", light.ConditionYellow.GetForeground())
	}
	if light.ConditionRed.GetForeground() != lipgloss.Color("#DC2626") {
		t.Errorf("expected light condition red #DC2626, got %v", light.ConditionRed.GetForeground())
	}
	if light.ProgressBarFilled.GetForeground() != lipgloss.Color("#0284C7") {
		t.Errorf("expected light progress filled #0284C7, got %v", light.ProgressBarFilled.GetForeground())
	}
	if light.ProgressBarEmpty.GetForeground() != lipgloss.Color("#E2E8F0") {
		t.Errorf("expected light progress empty #E2E8F0, got %v", light.ProgressBarEmpty.GetForeground())
	}
	if light.GaugeLabel.GetForeground() != lipgloss.Color("#334155") {
		t.Errorf("expected light gauge label #334155, got %v", light.GaugeLabel.GetForeground())
	}
	if light.GaugeValue.GetForeground() != lipgloss.Color("#0F172A") {
		t.Errorf("expected light gauge value #0F172A, got %v", light.GaugeValue.GetForeground())
	}
	if light.Prompt.GetForeground() != lipgloss.Color("#0284C7") {
		t.Errorf("expected light prompt #0284C7, got %v", light.Prompt.GetForeground())
	}
	if light.LogText.GetForeground() != lipgloss.Color("#0F172A") {
		t.Errorf("expected light log text #0F172A, got %v", light.LogText.GetForeground())
	}
}

func TestLcarsThemePaletteColors(t *testing.T) {
	th := LcarsTheme{}
	dark := th.PaletteStyles(true)
	light := th.PaletteStyles(false)

	// Dark LCARS
	if dark.Panel.GetBackground() != lipgloss.Color("#000000") {
		t.Errorf("expected dark panel bg #000000, got %v", dark.Panel.GetBackground())
	}
	if dark.Title.GetBackground() != lipgloss.Color("#FF9900") {
		t.Errorf("expected dark title bg #FF9900, got %v", dark.Title.GetBackground())
	}
	if dark.Title.GetForeground() != lipgloss.Color("#000000") {
		t.Errorf("expected dark title fg #000000, got %v", dark.Title.GetForeground())
	}
	if dark.Panel.GetBorderTopForeground() != lipgloss.Color("#FF9900") {
		t.Errorf("expected dark panel border #FF9900, got %v", dark.Panel.GetBorderTopForeground())
	}
	if dark.PanelTitle.GetForeground() != lipgloss.Color("#FFCC66") {
		t.Errorf("expected dark panel title fg #FFCC66, got %v", dark.PanelTitle.GetForeground())
	}
	if dark.Enterprise.GetForeground() != lipgloss.Color("#FFCC66") {
		t.Errorf("expected dark enterprise fg #FFCC66, got %v", dark.Enterprise.GetForeground())
	}
	if dark.Klingon.GetForeground() != lipgloss.Color("#FF3300") {
		t.Errorf("expected dark klingon fg #FF3300, got %v", dark.Klingon.GetForeground())
	}
	if dark.Starbase.GetForeground() != lipgloss.Color("#3399CC") {
		t.Errorf("expected dark starbase fg #3399CC, got %v", dark.Starbase.GetForeground())
	}
	if dark.Star.GetForeground() != lipgloss.Color("#FFFFFF") {
		t.Errorf("expected dark star fg #FFFFFF, got %v", dark.Star.GetForeground())
	}

	// Light LCARS
	if light.Panel.GetBackground() != lipgloss.Color("#FEF9EF") {
		t.Errorf("expected light panel bg #FEF9EF, got %v", light.Panel.GetBackground())
	}
	if light.Title.GetBackground() != lipgloss.Color("#C2410C") {
		t.Errorf("expected light title bg #C2410C, got %v", light.Title.GetBackground())
	}
	if light.Title.GetForeground() != lipgloss.Color("#FEF9EF") {
		t.Errorf("expected light title fg #FEF9EF, got %v", light.Title.GetForeground())
	}
	if light.Panel.GetBorderTopForeground() != lipgloss.Color("#C2410C") {
		t.Errorf("expected light panel border #C2410C, got %v", light.Panel.GetBorderTopForeground())
	}
	if light.PanelTitle.GetForeground() != lipgloss.Color("#9A3412") {
		t.Errorf("expected light panel title fg #9A3412, got %v", light.PanelTitle.GetForeground())
	}
	if light.Enterprise.GetForeground() != lipgloss.Color("#C2410C") {
		t.Errorf("expected light enterprise fg #C2410C, got %v", light.Enterprise.GetForeground())
	}
	if light.Klingon.GetForeground() != lipgloss.Color("#B91C1C") {
		t.Errorf("expected light klingon fg #B91C1C, got %v", light.Klingon.GetForeground())
	}
	if light.Starbase.GetForeground() != lipgloss.Color("#0E7490") {
		t.Errorf("expected light starbase fg #0E7490, got %v", light.Starbase.GetForeground())
	}
	if light.Star.GetForeground() != lipgloss.Color("#57534E") {
		t.Errorf("expected light star fg #57534E, got %v", light.Star.GetForeground())
	}
	if light.ProgressBarFilled.GetForeground() != lipgloss.Color("#C2410C") {
		t.Errorf("expected light progress filled #C2410C, got %v", light.ProgressBarFilled.GetForeground())
	}
	if light.ProgressBarEmpty.GetForeground() != lipgloss.Color("#E7E5E4") {
		t.Errorf("expected light progress empty #E7E5E4, got %v", light.ProgressBarEmpty.GetForeground())
	}
	if light.GaugeLabel.GetForeground() != lipgloss.Color("#1C1917") {
		t.Errorf("expected light telemetry label #1C1917, got %v", light.GaugeLabel.GetForeground())
	}
	if light.GaugeValue.GetForeground() != lipgloss.Color("#0E7490") {
		t.Errorf("expected light gauge value #0E7490, got %v", light.GaugeValue.GetForeground())
	}
}

func TestCrtThemePaletteColors(t *testing.T) {
	th := CrtTheme{}
	dark := th.PaletteStyles(true)
	light := th.PaletteStyles(false)

	// Dark CRT
	if dark.Panel.GetBackground() != lipgloss.Color("#050B05") {
		t.Errorf("expected dark panel bg #050B05, got %v", dark.Panel.GetBackground())
	}
	if dark.Title.GetBackground() != lipgloss.Color("#33FF33") {
		t.Errorf("expected dark title bg #33FF33, got %v", dark.Title.GetBackground())
	}
	if dark.Title.GetForeground() != lipgloss.Color("#050B05") {
		t.Errorf("expected dark title fg #050B05, got %v", dark.Title.GetForeground())
	}
	if dark.Panel.GetBorderTopForeground() != lipgloss.Color("#33FF33") {
		t.Errorf("expected dark panel border #33FF33, got %v", dark.Panel.GetBorderTopForeground())
	}
	if dark.Enterprise.GetForeground() != lipgloss.Color("#33FF33") {
		t.Errorf("expected dark enterprise fg #33FF33, got %v", dark.Enterprise.GetForeground())
	}
	if dark.Klingon.GetForeground() != lipgloss.Color("#33FF33") {
		t.Errorf("expected dark klingon fg #33FF33, got %v", dark.Klingon.GetForeground())
	}
	if dark.Starbase.GetForeground() != lipgloss.Color("#33FF33") {
		t.Errorf("expected dark starbase fg #33FF33, got %v", dark.Starbase.GetForeground())
	}
	if dark.Star.GetForeground() != lipgloss.Color("#22CC22") {
		t.Errorf("expected dark star fg #22CC22, got %v", dark.Star.GetForeground())
	}

	// Light CRT
	if light.Panel.GetBackground() != lipgloss.Color("#F0FDF4") {
		t.Errorf("expected light panel bg #F0FDF4, got %v", light.Panel.GetBackground())
	}
	if light.Title.GetBackground() != lipgloss.Color("#14532D") {
		t.Errorf("expected light title bg #14532D, got %v", light.Title.GetBackground())
	}
	if light.Title.GetForeground() != lipgloss.Color("#F0FDF4") {
		t.Errorf("expected light title fg #F0FDF4, got %v", light.Title.GetForeground())
	}
	if light.Panel.GetBorderTopForeground() != lipgloss.Color("#166534") {
		t.Errorf("expected light panel border #166534, got %v", light.Panel.GetBorderTopForeground())
	}
	if light.Enterprise.GetForeground() != lipgloss.Color("#14532D") {
		t.Errorf("expected light enterprise fg #14532D, got %v", light.Enterprise.GetForeground())
	}
	if light.Klingon.GetForeground() != lipgloss.Color("#14532D") {
		t.Errorf("expected light klingon fg #14532D, got %v", light.Klingon.GetForeground())
	}
	if light.Starbase.GetForeground() != lipgloss.Color("#14532D") {
		t.Errorf("expected light starbase fg #14532D, got %v", light.Starbase.GetForeground())
	}
	if light.Star.GetForeground() != lipgloss.Color("#15803D") {
		t.Errorf("expected light star fg #15803D, got %v", light.Star.GetForeground())
	}
	if light.GaugeLabel.GetForeground() != lipgloss.Color("#15803D") {
		t.Errorf("expected light gauge label #15803D, got %v", light.GaugeLabel.GetForeground())
	}
	if light.GaugeValue.GetForeground() != lipgloss.Color("#14532D") {
		t.Errorf("expected light gauge value #14532D, got %v", light.GaugeValue.GetForeground())
	}
	if light.Empty.GetForeground() != lipgloss.Color("#86EFAC") {
		t.Errorf("expected light empty fg #86EFAC, got %v", light.Empty.GetForeground())
	}
}

func TestThemeGridBackgrounds(t *testing.T) {
	themes := []Theme{
		ModernTheme{},
		LcarsTheme{},
		CrtTheme{},
	}

	for _, th := range themes {
		for _, isDark := range []bool{true, false} {
			modeStr := "light"
			if isDark {
				modeStr = "dark"
			}
			styles := th.PaletteStyles(isDark)
			panelBg := styles.Panel.GetBackground()
			gridBg := styles.Grid.GetBackground()

			if gridBg == (lipgloss.NoColor{}) || gridBg == nil {
				t.Errorf("theme %s (%s mode): Grid has empty background", th.Name(), modeStr)
			}
			if gridBg != panelBg {
				t.Errorf("theme %s (%s mode): Grid background (%v) does not match Panel background (%v)", th.Name(), modeStr, gridBg, panelBg)
			}
		}
	}
}
