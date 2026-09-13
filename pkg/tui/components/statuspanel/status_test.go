package statuspanel

import (
	"strings"
	"testing"

	"github.com/charmbracelet/lipgloss"
	"github.com/muesli/termenv"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

func TestStatusPanelRendering(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th)

	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	g.Enterprise.Energy = 4500
	g.Enterprise.Shields = 1500
	g.Enterprise.Condition = engine.ConditionYellow

	view := m.View(g)

	if !strings.Contains(view, "YELLOW") {
		t.Fatalf("status view missing condition alert:\n%s", view)
	}
	if !strings.Contains(view, "4500") {
		t.Fatalf("status view missing energy readout:\n%s", view)
	}
	if !strings.Contains(view, "1500") {
		t.Fatalf("status view missing shields readout:\n%s", view)
	}
}

func TestConditionAlerts(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th)

	testCases := []struct {
		cond     engine.ConditionType
		expected string
	}{
		{engine.ConditionGreen, "GREEN"},
		{engine.ConditionYellow, "YELLOW"},
		{engine.ConditionRed, "RED"},
		{engine.ConditionDocked, "DOCKED"},
	}

	for _, tc := range testCases {
		g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
		g.Enterprise.Condition = tc.cond
		view := m.View(g)
		if !strings.Contains(view, tc.expected) {
			t.Errorf("status view missing expected condition %q:\n%s", tc.expected, view)
		}
	}
}

func TestTelemetryProgressBars(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th)

	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	g.Enterprise.Energy = 4500
	g.Enterprise.Shields = 1500

	view := m.View(g)

	// Verify progress bar glyphs are present
	if !strings.Contains(view, "█") {
		t.Fatalf("status view missing filled progress bar glyph '█':\n%s", view)
	}
	if !strings.Contains(view, "░") {
		t.Fatalf("status view missing empty progress bar glyph '░':\n%s", view)
	}

	// Verify gauge readouts with max denominators
	if !strings.Contains(view, "4500/5000") && !strings.Contains(view, "4500") {
		t.Errorf("status view missing energy readout:\n%s", view)
	}
	if !strings.Contains(view, "1500/2500") && !strings.Contains(view, "1500") {
		t.Errorf("status view missing shields readout:\n%s", view)
	}
}

func TestTorpedoInventory(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th)

	testCases := []struct {
		torps    int
		expected string
	}{
		{10, "[TORP: 10/10]"},
		{5, "[TORP: 5/10]"},
		{0, "[TORP: 0/10]"},
	}

	for _, tc := range testCases {
		g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
		g.Enterprise.Torpedoes = tc.torps
		view := m.View(g)
		if !strings.Contains(view, tc.expected) {
			t.Errorf("status view missing torpedo readout %q:\n%s", tc.expected, view)
		}
	}
}

func TestStardateAndTimeRemaining(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th)

	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	g.Stardate = 2851.4
	g.TimeRemaining = 24.5

	view := m.View(g)
	if !strings.Contains(view, "2851.4") {
		t.Errorf("status view missing stardate readout 2851.4:\n%s", view)
	}
	if !strings.Contains(view, "24.5") {
		t.Errorf("status view missing time remaining readout 24.5:\n%s", view)
	}
}

func TestSubsystemDeviceRepairCountdowns(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th)

	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)

	// All devices start operational
	view := m.View(g)
	requiredDevices := []string{
		"Warp",
		"SRS",
		"LRS",
		"Phasers",
		"Tubes",
		"Damage Control",
		"Shields",
		"Computer",
	}

	for _, dev := range requiredDevices {
		if !strings.Contains(view, dev) {
			t.Errorf("status view missing device label %q:\n%s", dev, view)
		}
	}
	if !strings.Contains(view, "OK") {
		t.Errorf("expected operational status indicator 'OK' in view:\n%s", view)
	}

	// Damage Warp and Damage Control
	g.Enterprise.Devices[engine.DeviceWarp] = 4.2
	g.Enterprise.Devices[engine.DeviceDamageControl] = 7.5

	viewDamaged := m.View(g)
	if !strings.Contains(viewDamaged, "4.2") {
		t.Errorf("status view missing warp repair countdown '4.2':\n%s", viewDamaged)
	}
	if !strings.Contains(viewDamaged, "7.5") {
		t.Errorf("status view missing damage control repair countdown '7.5':\n%s", viewDamaged)
	}
}

func TestSurroundingQuadrantRadar(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th)

	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	g.Enterprise.Quad = engine.Coord{4, 4}

	// Set specific quadrant values around Enterprise (Coord{4, 4})
	// Surrounding rows 3..5, cols 3..5
	g.GalaxyChart[3][3] = 105
	g.GalaxyChart[3][4] = 203
	g.GalaxyChart[4][4] = 312
	g.GalaxyChart[5][5] = 407

	view := m.View(g)
	for _, expected := range []string{"105", "203", "312", "407"} {
		if !strings.Contains(view, expected) {
			t.Errorf("status view radar missing expected density %q:\n%s", expected, view)
		}
	}

	// Place Enterprise at corner Coord{1, 1} where out-of-bounds cells exist
	g.Enterprise.Quad = engine.Coord{1, 1}
	viewCorner := m.View(g)
	if !strings.Contains(viewCorner, "***") {
		t.Errorf("expected out-of-bounds '***' in radar for corner quadrant:\n%s", viewCorner)
	}
}

func TestStatusPanel_LocationReadoutAndRadarHeaders(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	g.Enterprise.Quad = engine.Coord{3, 5}
	g.Enterprise.Sector = engine.Coord{2, 6}
	g.GalaxyChart[3][5] = 3
	g.GalaxyChart[2][5] = 105
	g.GalaxyChart[3][6] = 12

	m := New(theme.DefaultTheme())
	view := m.View(g)

	if !strings.Contains(view, "LOC: ") || !strings.Contains(view, "Q[3,5] S[2,6]") {
		t.Fatalf("expected combined location readout 'LOC: Q[3,5] S[2,6]', got:\n%s", view)
	}
	if !strings.Contains(view, "RADAR (±1) [K-B-S]:") {
		t.Fatalf("expected radar header 'RADAR (±1) [K-B-S]:', got:\n%s", view)
	}
	// Verify coordinate headers for row 2, 3, 4 and col 4, 5, 6
	if !strings.Contains(view, "4    5    6") {
		t.Fatalf("expected radar column headers '4    5    6', got:\n%s", view)
	}
	for _, rowHdr := range []string{"  2  ", "  3  ", "  4  "} {
		if !strings.Contains(view, rowHdr) {
			t.Fatalf("expected radar row header %q, got:\n%s", rowHdr, view)
		}
	}
	if !strings.Contains(view, "<003>") {
		t.Fatalf("expected Enterprise current quad cell '<003>', got:\n%s", view)
	}
}

func TestSetTheme(t *testing.T) {
	lipgloss.SetColorProfile(termenv.TrueColor)
	defer lipgloss.SetColorProfile(termenv.Ascii)

	m := New(theme.ModernTheme{})
	if m.Theme().Name() != "modern" {
		t.Fatalf("expected modern theme, got %s", m.Theme().Name())
	}

	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	viewModern := m.View(g)

	m.SetTheme(theme.LcarsTheme{})
	if m.Theme().Name() != "lcars" {
		t.Fatalf("expected lcars theme, got %s", m.Theme().Name())
	}
	viewLcars := m.View(g)

	if viewModern == "" || viewLcars == "" {
		t.Fatalf("expected non-empty rendered views")
	}
	if viewModern == viewLcars {
		t.Errorf("expected different styled output between Modern and LCARS themes")
	}

	m.SetTheme(theme.CrtTheme{})
	if m.Theme().Name() != "crt" {
		t.Fatalf("expected crt theme, got %s", m.Theme().Name())
	}
	viewCrt := m.View(g)
	if viewCrt == "" || viewCrt == viewModern {
		t.Errorf("expected different styled output for CRT theme")
	}

	m.SetTheme(nil)
	if m.Theme().Name() != "modern" {
		t.Fatalf("expected fallback to modern theme, got %s", m.Theme().Name())
	}
}

func TestNilGameAndDefaults(t *testing.T) {
	m := New(nil)
	if m.Theme().Name() != "modern" {
		t.Fatalf("expected default modern theme for nil argument")
	}

	// Nil GameState should not panic
	viewNil := m.View(nil)
	if viewNil == "" {
		t.Fatalf("expected non-empty view even with nil GameState")
	}
}
