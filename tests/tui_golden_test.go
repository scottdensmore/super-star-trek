package tests

import (
	"flag"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/savebrowser"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

var update = flag.Bool("update", false, "update golden files")

func compareOrUpdate(t *testing.T, name string, actual string) {
	t.Helper()
	goldenPath := filepath.Join("golden", "tui", name+".golden")

	if *update {
		if err := os.MkdirAll(filepath.Dir(goldenPath), 0755); err != nil {
			t.Fatalf("failed to create golden directory: %v", err)
		}
		if err := os.WriteFile(goldenPath, []byte(actual), 0644); err != nil {
			t.Fatalf("failed to write golden file %s: %v", goldenPath, err)
		}
		t.Logf("updated golden file: %s (%d bytes)", goldenPath, len(actual))
		return
	}

	expectedBytes, err := os.ReadFile(goldenPath)
	if err != nil {
		t.Fatalf("golden file %s not found: %v (run with -update to generate)", goldenPath, err)
	}

	expected := string(expectedBytes)
	if actual != expected {
		expectedLines := strings.Split(expected, "\n")
		actualLines := strings.Split(actual, "\n")
		diffMsg := fmt.Sprintf("golden mismatch for %s:\nexpected %d lines, got %d lines\n", name, len(expectedLines), len(actualLines))

		maxLines := len(expectedLines)
		if len(actualLines) > maxLines {
			maxLines = len(actualLines)
		}

		diffCount := 0
		for i := 0; i < maxLines; i++ {
			var expLine, actLine string
			if i < len(expectedLines) {
				expLine = expectedLines[i]
			}
			if i < len(actualLines) {
				actLine = actualLines[i]
			}
			if expLine != actLine {
				diffCount++
				if diffCount <= 5 {
					diffMsg += fmt.Sprintf("line %d diff:\n  want: %q\n   got: %q\n", i+1, expLine, actLine)
				}
			}
		}
		t.Fatalf("%s", diffMsg)
	}
}

func TestTUIGolden_ModernDashboard80x24(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	m := tui.NewModel(g, theme.ModernTheme{})
	updated, _ := m.Update(tea.WindowSizeMsg{Width: 80, Height: 24})
	m = updated.(tui.Model)
	compareOrUpdate(t, "modern_dashboard_80x24", m.View())
}

func TestTUIGolden_LcarsDashboard80x24(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	m := tui.NewModel(g, theme.LcarsTheme{})
	updated, _ := m.Update(tea.WindowSizeMsg{Width: 80, Height: 24})
	m = updated.(tui.Model)
	compareOrUpdate(t, "lcars_dashboard_80x24", m.View())
}

func TestTUIGolden_CrtDashboard80x24(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	m := tui.NewModel(g, theme.CrtTheme{})
	updated, _ := m.Update(tea.WindowSizeMsg{Width: 80, Height: 24})
	m = updated.(tui.Model)
	compareOrUpdate(t, "crt_dashboard_80x24", m.View())
}

func TestTUIGolden_ModernDashboard100x30(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	m := tui.NewModel(g, theme.ModernTheme{})
	updated, _ := m.Update(tea.WindowSizeMsg{Width: 100, Height: 30})
	m = updated.(tui.Model)
	compareOrUpdate(t, "modern_dashboard_100x30", m.View())
}

func TestTUIGolden_ModalTargetLock(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	klingon1 := &engine.Klingon{ID: 1, Sector: engine.Coord{4, 7}, Energy: 300}
	klingon2 := &engine.Klingon{ID: 2, Sector: engine.Coord{6, 2}, Energy: 250}
	g.CurrentQuad.Klingons = []*engine.Klingon{klingon1, klingon2}
	g.CurrentQuad.Grid[4][7] = engine.EntityKlingon
	g.CurrentQuad.Grid[6][2] = engine.EntityKlingon

	m := tui.NewModel(g, theme.ModernTheme{})
	updated, _ := m.Update(tea.WindowSizeMsg{Width: 80, Height: 24})
	m = updated.(tui.Model)

	m.TargetLock.SetState(
		g.Enterprise.Sector,
		g.Enterprise.Energy,
		g.Enterprise.Torpedoes,
		g.CurrentQuad.Klingons,
		engine.Coord{},
	)
	m.ActiveModal = tui.ModalTargetLock
	compareOrUpdate(t, "modal_target_lock", m.View())
}

func TestTUIGolden_ModalCommandPalette(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	m := tui.NewModel(g, theme.ModernTheme{})
	updated, _ := m.Update(tea.WindowSizeMsg{Width: 80, Height: 24})
	m = updated.(tui.Model)

	m.CommandPalette.Reset()
	m.ActiveModal = tui.ModalCommandPalette
	compareOrUpdate(t, "modal_command_palette", m.View())
}

func TestTUIGolden_ModalSaveBrowser(t *testing.T) {
	tempDir := t.TempDir()

	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	m := tui.NewModel(g, theme.ModernTheme{})
	updated, _ := m.Update(tea.WindowSizeMsg{Width: 80, Height: 24})
	m = updated.(tui.Model)

	m.SaveBrowser = savebrowser.New(theme.ModernTheme{}, tempDir)
	m.ActiveModal = tui.ModalSaveBrowser
	compareOrUpdate(t, "modal_save_browser", m.View())
}

func TestTUIGolden_ModalGalacticChart(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	g.Enterprise.Quad = engine.Coord{3, 3}
	g.GalaxyChart[3][3] = 3
	g.ChartDiscovered[3][3] = true
	g.GalaxyChart[2][5] = 105
	g.ChartDiscovered[2][5] = true
	g.GalaxyChart[3][4] = 12
	g.ChartDiscovered[3][4] = true

	m := tui.NewModel(g, theme.ModernTheme{})
	updated, _ := m.Update(tea.WindowSizeMsg{Width: 80, Height: 24})
	m = updated.(tui.Model)

	m.GalacticChart.SetState(g.Enterprise.Quad, g.GalaxyChart, g.ChartDiscovered, g.ChartKnownBases, false)
	m.ActiveModal = tui.ModalGalacticChart
	compareOrUpdate(t, "modal_galactic_chart", m.View())
}

func TestTUIGolden_ModalDamageSchematic(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	g.Enterprise.Devices[engine.DeviceComputer] = 2.1
	g.Enterprise.Devices[engine.DevicePhotonTubes] = 1.4

	m := tui.NewModel(g, theme.ModernTheme{})
	updated, _ := m.Update(tea.WindowSizeMsg{Width: 80, Height: 24})
	m = updated.(tui.Model)

	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune("d")})
	m = updated.(tui.Model)

	compareOrUpdate(t, "modal_damage_schematic", m.View())
}

func TestTUIGolden_LrsDamagedDashboard(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	g.Enterprise.Devices[engine.DeviceLRSensors] = 3.5 // Damaged!
	m := tui.NewModel(g, theme.ModernTheme{})
	updated, _ := m.Update(tea.WindowSizeMsg{Width: 80, Height: 24})
	m = updated.(tui.Model)
	compareOrUpdate(t, "lrs_damaged_dashboard_80x24", m.View())
}

func TestTUIGolden_SectorReticleSelected(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	m := tui.NewModel(g, theme.ModernTheme{})
	updated, _ := m.Update(tea.WindowSizeMsg{Width: 80, Height: 24})
	m = updated.(tui.Model)
	m.SelectedSector = engine.Coord{4, 5}
	compareOrUpdate(t, "sector_reticle_selected", m.View())
}

func TestTUIGolden_ConditionRedAlert(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	g.Enterprise.Condition = engine.ConditionRed
	klingon := &engine.Klingon{ID: 1, Sector: engine.Coord{3, 4}, Energy: 400}
	g.CurrentQuad.Klingons = []*engine.Klingon{klingon}
	g.CurrentQuad.Grid[3][4] = engine.EntityKlingon

	m := tui.NewModel(g, theme.ModernTheme{})
	updated, _ := m.Update(tea.WindowSizeMsg{Width: 80, Height: 24})
	m = updated.(tui.Model)
	compareOrUpdate(t, "condition_red_alert", m.View())
}

func TestTUIGolden_SizeWarningDialog(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	m := tui.NewModel(g, theme.ModernTheme{})
	updated, _ := m.Update(tea.WindowSizeMsg{Width: 70, Height: 20})
	m = updated.(tui.Model)
	compareOrUpdate(t, "size_warning_dialog", m.View())
}

func TestTUIGolden_ModalHallOfFame(t *testing.T) {
	t.Setenv("XDG_CONFIG_HOME", t.TempDir())

	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	g.Metrics.KlingonsKilled = 8
	g.Metrics.CommandersKilled = 2
	g.Metrics.Casualties = 12

	m := tui.NewModel(g, theme.ModernTheme{})
	updated, _ := m.Update(tea.WindowSizeMsg{Width: 80, Height: 24})
	m = updated.(tui.Model)

	// Tab 1: Telemetry
	updatedModal, _ := m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune("h")})
	m = updatedModal.(tui.Model)
	compareOrUpdate(t, "modal_hall_of_fame_telemetry", m.View())

	// Tab 2: Leaderboard
	updatedTab2, _ := m.Update(tea.KeyMsg{Type: tea.KeyTab})
	m = updatedTab2.(tui.Model)
	compareOrUpdate(t, "modal_hall_of_fame_leaderboard", m.View())
}

