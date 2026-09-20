package tui

import (
	"fmt"
	"strings"
	"testing"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/commandpalette"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/optionsmodal"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/targetlock"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

func TestColorModeKeyboardToggle(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	th := theme.ModernTheme{}.WithColorMode(theme.ColorModeAuto)
	m := NewModel(g, th)

	if m.Theme.ColorMode() != theme.ColorModeAuto {
		t.Fatalf("expected initial mode Auto, got %s", m.Theme.ColorMode())
	}

	// Press Ctrl+T (by KeyType) to cycle from Auto to Dark
	newM, _ := m.Update(tea.KeyMsg{Type: tea.KeyCtrlT})
	updated := newM.(Model)
	if updated.Theme.ColorMode() != theme.ColorModeDark {
		t.Fatalf("expected mode Dark after Ctrl+T, got %s", updated.Theme.ColorMode())
	}
	msgs := updated.CommandBar.Messages()
	if len(msgs) == 0 || !strings.Contains(msgs[len(msgs)-1], "Color mode set to dark (modern)") {
		t.Fatalf("expected message confirming dark mode, got: %v", msgs)
	}

	// Press Ctrl+T (by string representation) to cycle from Dark to Light
	newM2, _ := updated.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune("ctrl+t")})
	updated2 := newM2.(Model)
	if updated2.Theme.ColorMode() != theme.ColorModeLight {
		t.Fatalf("expected mode Light after second Ctrl+T, got %s", updated2.Theme.ColorMode())
	}
	msgs2 := updated2.CommandBar.Messages()
	if len(msgs2) == 0 || !strings.Contains(msgs2[len(msgs2)-1], "Color mode set to light (modern)") {
		t.Fatalf("expected message confirming light mode, got: %v", msgs2)
	}

	// Press Shift+F2 to cycle from Light to Auto
	newM3, _ := updated2.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune("shift+f2")})
	updated3 := newM3.(Model)
	if updated3.Theme.ColorMode() != theme.ColorModeAuto {
		t.Fatalf("expected mode Auto after Shift+F2, got %s", updated3.Theme.ColorMode())
	}
	msgs3 := updated3.CommandBar.Messages()
	if len(msgs3) == 0 || !strings.Contains(msgs3[len(msgs3)-1], "Color mode set to auto (modern)") {
		t.Fatalf("expected message confirming auto mode, got: %v", msgs3)
	}
}

func TestColorModeKeyboardToggle_Modals(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	th := theme.ModernTheme{}.WithColorMode(theme.ColorModeAuto)
	m := NewModel(g, th)

	// Active modal: TargetLock
	m.ActiveModal = ModalTargetLock
	newM, _ := m.Update(tea.KeyMsg{Type: tea.KeyCtrlT})
	updated := newM.(Model)
	if updated.Theme.ColorMode() != theme.ColorModeDark {
		t.Fatalf("expected mode Dark during active modal after Ctrl+T, got %s", updated.Theme.ColorMode())
	}
	if updated.ActiveModal != ModalTargetLock {
		t.Fatalf("expected ActiveModal to remain TargetLock, got %d", updated.ActiveModal)
	}

	// Active modal: OptionsModal
	updated.ActiveModal = ModalNone
	updated.showOptions = true
	newM2, _ := updated.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune("shift+f2")})
	updated2 := newM2.(Model)
	if updated2.Theme.ColorMode() != theme.ColorModeLight {
		t.Fatalf("expected mode Light during showOptions after Shift+F2, got %s", updated2.Theme.ColorMode())
	}
	if !updated2.showOptions {
		t.Fatalf("expected showOptions to remain true")
	}
}

func TestColorModeTextCommands(t *testing.T) {
	tests := []struct {
		cmd      string
		wantMode theme.ColorMode
	}{
		{"theme mode dark", theme.ColorModeDark},
		{"theme mode light", theme.ColorModeLight},
		{"theme mode auto", theme.ColorModeAuto},
		{"color dark", theme.ColorModeDark},
		{"color light", theme.ColorModeLight},
		{"color auto", theme.ColorModeAuto},
		{"THEME MODE DARK", theme.ColorModeDark},
		{"COLOR LIGHT", theme.ColorModeLight},
	}

	for _, tc := range tests {
		t.Run(tc.cmd, func(t *testing.T) {
			g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
			th := theme.ModernTheme{}.WithColorMode(theme.ColorModeAuto)
			m := NewModel(g, th)

			resModel, _ := m.handleCommand(tc.cmd)
			updated := resModel.(Model)
			if updated.Theme.ColorMode() != tc.wantMode {
				t.Fatalf("handleCommand(%q): expected mode %s, got %s", tc.cmd, tc.wantMode, updated.Theme.ColorMode())
			}

			expectedMsg := fmt.Sprintf("Color mode set to %s (modern)", tc.wantMode)
			msgs := updated.CommandBar.Messages()
			if len(msgs) == 0 || !strings.Contains(msgs[len(msgs)-1], expectedMsg) {
				t.Fatalf("handleCommand(%q): expected message %q, got: %v", tc.cmd, expectedMsg, msgs)
			}
		})
	}
}

func TestDynamicBackgroundAdaptationOnWindowSize(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	th := theme.ModernTheme{}.WithColorMode(theme.ColorModeAuto)
	m := NewModel(g, th)

	currentBg := theme.DetectDarkBackground()
	// Artificially simulate that last cached background was the opposite of current
	m.lastDarkBg = !currentBg

	newM, _ := m.Update(tea.WindowSizeMsg{Width: 80, Height: 24})
	updated := newM.(Model)

	if updated.lastDarkBg != currentBg {
		t.Fatalf("expected lastDarkBg to update to %v, got %v", currentBg, updated.lastDarkBg)
	}

	// Verify that when mode is NOT Auto (e.g. ColorModeDark), changes are not adapted
	fixedTh := theme.ModernTheme{}.WithColorMode(theme.ColorModeDark)
	mFixed := NewModel(g, fixedTh)
	mFixed.lastDarkBg = !currentBg

	newMFixed, _ := mFixed.Update(tea.WindowSizeMsg{Width: 80, Height: 24})
	updatedFixed := newMFixed.(Model)
	if updatedFixed.Theme.ColorMode() != theme.ColorModeDark {
		t.Fatalf("expected mode to remain Dark, got %s", updatedFixed.Theme.ColorMode())
	}
}

func TestColorModeCommandPaletteSelection(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	th := theme.ModernTheme{}.WithColorMode(theme.ColorModeAuto)
	m := NewModel(g, th)

	m.ActiveModal = ModalCommandPalette

	// Select THEME MODE DARK from palette
	newM, _ := m.Update(commandpalette.CommandSelectedMsg{
		CommandPrefix: "theme mode dark",
		Parameterized: false,
	})
	updated := newM.(Model)

	if updated.ActiveModal != ModalNone {
		t.Fatalf("expected modal to close after selection, got %d", updated.ActiveModal)
	}
	if updated.Theme.ColorMode() != theme.ColorModeDark {
		t.Fatalf("expected mode Dark, got %s", updated.Theme.ColorMode())
	}

	// Select THEME MODE LIGHT from palette
	newM2, _ := updated.Update(commandpalette.CommandSelectedMsg{
		CommandPrefix: "theme mode light",
		Parameterized: false,
	})
	updated2 := newM2.(Model)
	if updated2.Theme.ColorMode() != theme.ColorModeLight {
		t.Fatalf("expected mode Light, got %s", updated2.Theme.ColorMode())
	}

	// Select THEME MODE AUTO from palette
	newM3, _ := updated2.Update(commandpalette.CommandSelectedMsg{
		CommandPrefix: "theme mode auto",
		Parameterized: false,
	})
	updated3 := newM3.(Model)
	if updated3.Theme.ColorMode() != theme.ColorModeAuto {
		t.Fatalf("expected mode Auto, got %s", updated3.Theme.ColorMode())
	}
}

func TestColorModeOptionsModalIntegration(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	th := theme.ModernTheme{}.WithColorMode(theme.ColorModeLight)
	m := NewModel(g, th)

	// Open options modal with 'o'
	newM, _ := m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'o'}})
	updated := newM.(Model)
	if !updated.showOptions {
		t.Fatalf("expected showOptions to be true after pressing 'o'")
	}
	if updated.optionsModal.ColorMode() != theme.ColorModeLight {
		t.Fatalf("expected optionsModal color mode to be initialized to Light, got %s", updated.optionsModal.ColorMode())
	}

	// Change color mode in modal to Auto
	updated.optionsModal.SetColorMode(theme.ColorModeAuto)

	// Close modal by pressing 'q'
	newM2, _ := updated.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'q'}})
	updated2 := newM2.(Model)
	if updated2.showOptions {
		t.Fatalf("expected showOptions to be false after closing")
	}
	if updated2.Theme.ColorMode() != theme.ColorModeAuto {
		t.Fatalf("expected Model.Theme.ColorMode to be Auto after modal close, got %s", updated2.Theme.ColorMode())
	}
}

func TestThemeNameCommandPreservesColorMode(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	th := theme.ModernTheme{}.WithColorMode(theme.ColorModeLight)
	m := NewModel(g, th)

	resModel, _ := m.handleCommand("theme lcars")
	updated := resModel.(Model)
	if updated.Theme.Name() != "lcars" {
		t.Fatalf("expected theme lcars, got %s", updated.Theme.Name())
	}
	if updated.Theme.ColorMode() != theme.ColorModeLight {
		t.Fatalf("expected ColorMode Light to be preserved, got %s", updated.Theme.ColorMode())
	}
}

func TestColorModeOptionsModalKeyInteraction(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	th := theme.ModernTheme{}.WithColorMode(theme.ColorModeAuto)
	m := NewModel(g, th)

	// Open options modal
	newM, _ := m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'o'}})
	updated := newM.(Model)

	// Navigate directly or set to RowColorMode
	for i := 0; i < int(optionsmodal.RowColorMode); i++ {
		stepM, _ := updated.Update(tea.KeyMsg{Type: tea.KeyDown})
		updated = stepM.(Model)
	}

	// Press Right to cycle from Auto to Dark
	newM2, _ := updated.Update(tea.KeyMsg{Type: tea.KeyRight})
	updated2 := newM2.(Model)
	if updated2.optionsModal.ColorMode() != theme.ColorModeDark {
		t.Fatalf("expected modal ColorMode to be Dark, got %s", updated2.optionsModal.ColorMode())
	}

	// Navigate to RowDone
	for updated2.optionsModal.SelectedRow != optionsmodal.RowDone {
		newMDone, _ := updated2.Update(tea.KeyMsg{Type: tea.KeyDown})
		updated2 = newMDone.(Model)
	}
	updatedDone := updated2

	// Press Enter to confirm and close
	newM3, _ := updatedDone.Update(tea.KeyMsg{Type: tea.KeyEnter})
	updated3 := newM3.(Model)
	if updated3.showOptions {
		t.Fatalf("expected modal to close after Enter on RowDone")
	}
	if updated3.Theme.ColorMode() != theme.ColorModeDark {
		t.Fatalf("expected active Theme.ColorMode to be Dark after saving modal, got %s", updated3.Theme.ColorMode())
	}
}

func TestTypeSheCommandAndEnter(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	m := NewModel(g, theme.DefaultTheme())

	// Type "she 2500"
	for _, ch := range "she 2500" {
		newM, _ := m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{ch}})
		m = newM.(Model)
	}

	// Press Enter
	newM, cmd := m.Update(tea.KeyMsg{Type: tea.KeyEnter})
	m = newM.(Model)
	if cmd != nil {
		newM, _ = m.Update(cmd())
		m = newM.(Model)
	}

	if m.ActiveModal == ModalGalacticChart {
		t.Fatalf("BUG: typing she 2500 and pressing Enter opened GalacticChart modal!")
	}
	if g.Enterprise.Shields != 2500 {
		t.Fatalf("expected Enterprise.Shields == 2500, got %f", g.Enterprise.Shields)
	}
}

func TestKlingonCounterAttackInCombat(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	g.Enterprise.Sector = engine.Coord{4, 1}
	g.Enterprise.Shields = 1000
	g.Enterprise.Energy = 3000

	// Place surviving Klingon in sector [4, 5]
	k := &engine.Klingon{
		ID:     1,
		Sector: engine.Coord{4, 5},
		Energy: 500,
	}
	g.CurrentQuad.Klingons = []*engine.Klingon{k}
	g.CurrentQuad.Grid[4][5] = engine.EntityKlingon

	m := NewModel(g, theme.DefaultTheme())

	// Fire low phaser energy so Klingon survives
	updated, _ := m.handleCommand("pha 100")
	m = updated.(Model)

	// Klingon should have returned fire
	msgs := m.CommandBar.Messages()
	foundReturnFire := false
	for _, msg := range msgs {
		if strings.Contains(msg, "Klingon #1 returned fire") {
			foundReturnFire = true
			break
		}
	}

	if !foundReturnFire {
		t.Fatalf("expected log to contain Klingon return fire, got messages: %v", msgs)
	}

	// Enterprise shields should have absorbed damage
	if g.Enterprise.Shields >= 1000 {
		t.Fatalf("expected shields to decrease from return fire, got %f", g.Enterprise.Shields)
	}

	foundShieldsAbsorbed := false
	for _, msg := range msgs {
		if strings.Contains(msg, "Shields absorbed") {
			foundShieldsAbsorbed = true
			break
		}
	}
	if !foundShieldsAbsorbed {
		t.Errorf("expected message to show shields absorbed, got messages: %v", msgs)
	}

	// Case 2: Enterprise has 0 shields -> direct hull hit on Energy
	g2 := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	g2.Enterprise.Sector = engine.Coord{4, 1}
	g2.Enterprise.Shields = 0
	g2.Enterprise.Energy = 3000
	k2 := &engine.Klingon{
		ID:     2,
		Sector: engine.Coord{4, 5},
		Energy: 500,
	}
	g2.CurrentQuad.Klingons = []*engine.Klingon{k2}
	g2.CurrentQuad.Grid[4][5] = engine.EntityKlingon

	m2 := NewModel(g2, theme.DefaultTheme())
	updated2, _ := m2.handleCommand("pha 100")
	m2 = updated2.(Model)

	if g2.Enterprise.Energy >= 3000 {
		t.Fatalf("expected Enterprise energy to decrease from direct hull hit, got %f", g2.Enterprise.Energy)
	}
	msgs2 := m2.CommandBar.Messages()
	foundHullHit := false
	for _, msg := range msgs2 {
		if strings.Contains(msg, "HULL HIT: -") {
			foundHullHit = true
			break
		}
	}
	if !foundHullHit {
		t.Errorf("expected message to report 'HULL HIT', got messages: %v", msgs2)
	}
}

func TestDockCommand_DocAndDock(t *testing.T) {
	// Case 1: Adjacent to starbase - "doc" docks successfully and does not open manual
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	sbCoord := engine.Coord{4, 4}
	g.CurrentQuad.Starbase = &sbCoord
	g.CurrentQuad.Grid[4][4] = engine.EntityStarbase
	g.Enterprise.Sector = engine.Coord{4, 5}
	g.Enterprise.Condition = engine.ConditionGreen
	g.Enterprise.Energy = 2000
	g.Enterprise.Torpedoes = 3
	g.Enterprise.Devices[engine.DevicePhasers] = 4.5

	m := NewModel(g, theme.DefaultTheme())

	updated, _ := m.handleCommand("doc")
	mod := updated.(Model)

	if mod.ActiveModal != ModalNone {
		t.Fatalf("expected ActiveModal = ModalNone on 'doc', got %v (manual should not open)", mod.ActiveModal)
	}
	if mod.Game.Enterprise.Condition != engine.ConditionDocked {
		t.Fatalf("expected Enterprise ConditionDocked, got %v", mod.Game.Enterprise.Condition)
	}
	if mod.Game.Enterprise.Energy != 5000 {
		t.Errorf("expected energy 5000, got %f", mod.Game.Enterprise.Energy)
	}
	if mod.Game.Enterprise.Torpedoes != 10 {
		t.Errorf("expected torpedoes 10, got %d", mod.Game.Enterprise.Torpedoes)
	}
	if mod.Game.Enterprise.Devices[engine.DevicePhasers] != 0 {
		t.Errorf("expected phasers repaired to 0, got %f", mod.Game.Enterprise.Devices[engine.DevicePhasers])
	}

	// Case 2: Not adjacent to starbase - "doc" outputs docking error, does not open manual
	g2 := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	g2.Enterprise.Sector = engine.Coord{1, 1}
	sbCoord2 := engine.Coord{8, 8}
	g2.CurrentQuad.Starbase = &sbCoord2
	g2.CurrentQuad.Grid[8][8] = engine.EntityStarbase

	m2 := NewModel(g2, theme.DefaultTheme())
	updated2, _ := m2.handleCommand("doc")
	mod2 := updated2.(Model)

	if mod2.ActiveModal != ModalNone {
		t.Fatalf("expected ActiveModal = ModalNone on 'doc' away from starbase, got %v", mod2.ActiveModal)
	}
	msgs := mod2.CommandBar.Messages()
	foundErr := false
	for _, msg := range msgs {
		if strings.Contains(strings.ToLower(msg), "enterprise not adjacent to starbase") {
			foundErr = true
			break
		}
	}
	if !foundErr {
		t.Fatalf("expected error 'enterprise not adjacent to starbase', got: %v", msgs)
	}

	// Case 3: "docs" opens manual
	updated3, _ := m.handleCommand("docs")
	mod3 := updated3.(Model)
	if mod3.ActiveModal != ModalManual {
		t.Fatalf("expected ActiveModal = ModalManual on 'docs', got %v", mod3.ActiveModal)
	}

	// Case 4: "help doc" opens manual at docking chapter
	updated4, _ := m.handleCommand("help doc")
	mod4 := updated4.(Model)
	if mod4.ActiveModal != ModalManual {
		t.Fatalf("expected ActiveModal = ModalManual on 'help doc', got %v", mod4.ActiveModal)
	}
}

func TestTorpedoDamageAndMissLogging(t *testing.T) {
	// Case 1: TargetLock fires torpedo at Klingon to West (Bearing 5.0) -> must hit and log damage
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	g.Enterprise.Sector = engine.Coord{4, 5}
	g.Enterprise.Torpedoes = 10
	klingon := &engine.Klingon{ID: 1, Sector: engine.Coord{4, 1}, Energy: 1000}
	g.CurrentQuad.Klingons = []*engine.Klingon{klingon}
	g.CurrentQuad.Grid[4][1] = engine.EntityKlingon

	m := NewModel(g, theme.DefaultTheme())
	m.ActiveModal = ModalTargetLock

	// TargetLock emits Bearing 5.0 (classic course system for West) and Target [4, 1]
	updated, _ := m.Update(targetlock.FireTorpedoMsg{Target: engine.Coord{4, 1}, Bearing: 5.0})
	m = updated.(Model)

	// 1000 - 500 damage = 500; then surviving Klingon returns fire consuming energy: 500 * 0.75 = 375
	if klingon.Energy != 375 {
		t.Fatalf("expected Klingon energy reduced to 375 by torpedo + counter-fire, got %f", klingon.Energy)
	}

	msgs := m.CommandBar.Messages()
	foundDamage := false
	for _, msg := range msgs {
		if strings.Contains(msg, "500 units damage") {
			foundDamage = true
			break
		}
	}
	if !foundDamage {
		t.Fatalf("expected command bar to list 500 units damage, got messages: %v", msgs)
	}

	// Case 2: Second torpedo destroys Klingon -> message includes damage
	updated2, _ := m.Update(targetlock.FireTorpedoMsg{Target: engine.Coord{4, 1}, Bearing: 5.0})
	m2 := updated2.(Model)
	msgs2 := m2.CommandBar.Messages()
	foundDestroyedWithDamage := false
	for _, msg := range msgs2 {
		if strings.Contains(msg, "Target destroyed at [4,1] (375 damage)") || strings.Contains(msg, "Target destroyed at [4,1] (500 damage)") {
			foundDestroyedWithDamage = true
			break
		}
	}
	if !foundDestroyedWithDamage {
		t.Fatalf("expected command bar to list destroyed with damage, got: %v", msgs2)
	}

	// Case 3: Torpedo misses -> logs "Torpedo missed."
	g3 := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	g3.Enterprise.Sector = engine.Coord{1, 1}
	g3.Enterprise.Torpedoes = 5
	// Clear quadrant of obstacles in row 1
	for c := 1; c <= 8; c++ {
		g3.CurrentQuad.Grid[1][c] = engine.EntityEmpty
	}
	m3 := NewModel(g3, theme.DefaultTheme())
	// Fire East along row 1 (0 rad) into empty space
	updated3, _ := m3.handleCommand("tor 0")
	m3 = updated3.(Model)
	msgs3 := m3.CommandBar.Messages()
	foundMiss := false
	for _, msg := range msgs3 {
		if strings.Contains(msg, "Torpedo missed.") {
			foundMiss = true
			break
		}
	}
	if !foundMiss {
		t.Fatalf("expected command bar to report 'Torpedo missed.', got: %v", msgs3)
	}

	// Case 4: Command "tor c 5" (classic course 5 = West)
	g4 := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	g4.Enterprise.Sector = engine.Coord{4, 5}
	g4.Enterprise.Torpedoes = 5
	k4 := &engine.Klingon{ID: 2, Sector: engine.Coord{4, 1}, Energy: 800}
	g4.CurrentQuad.Klingons = []*engine.Klingon{k4}
	g4.CurrentQuad.Grid[4][1] = engine.EntityKlingon
	m4 := NewModel(g4, theme.DefaultTheme())

	updated4, _ := m4.handleCommand("tor c 5")
	_ = updated4.(Model)
	// 800 - 500 = 300; then 300 * 0.75 = 225
	if k4.Energy != 225 {
		t.Fatalf("expected Klingon to take 500 damage from 'tor c 5', got energy %f", k4.Energy)
	}
}


