package tui

import (
	"strings"
	"testing"
	"time"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/commandbar"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/commandpalette"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/targetlock"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

func TestModelSizeGuard(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	m := NewModel(g, theme.DefaultTheme())

	// Small size triggers warning
	updated, _ := m.Update(tea.WindowSizeMsg{Width: 70, Height: 20})
	m = updated.(Model)
	view := m.View()
	if !strings.Contains(view, "TERMINAL WINDOW TOO SMALL") {
		t.Fatalf("expected size warning view, got:\n%s", view)
	}

	// Small width only
	updated, _ = m.Update(tea.WindowSizeMsg{Width: 79, Height: 25})
	m = updated.(Model)
	if !strings.Contains(m.View(), "TERMINAL WINDOW TOO SMALL") {
		t.Fatalf("expected size warning for width 79")
	}

	// Small height only
	updated, _ = m.Update(tea.WindowSizeMsg{Width: 85, Height: 23})
	m = updated.(Model)
	if !strings.Contains(m.View(), "TERMINAL WINDOW TOO SMALL") {
		t.Fatalf("expected size warning for height 23")
	}

	// Standard size renders dashboard
	updated, _ = m.Update(tea.WindowSizeMsg{Width: 80, Height: 24})
	m = updated.(Model)
	view = m.View()
	if strings.Contains(view, "TOO SMALL") || !strings.Contains(view, "<E>") {
		t.Fatalf("expected dashboard view, got:\n%s", view)
	}
}

func TestModelThemeToggle(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	m := NewModel(g, theme.DefaultTheme())

	if m.Theme.Name() != "modern" {
		t.Fatalf("expected initial modern theme, got: %s", m.Theme.Name())
	}

	// Press F2 to cycle theme to LCARS
	updated, _ := m.Update(tea.KeyMsg{Type: tea.KeyF2})
	m = updated.(Model)
	if m.Theme.Name() != "lcars" {
		t.Fatalf("expected theme cycled to lcars, got: %s", m.Theme.Name())
	}
	if m.Grid.Theme().Name() != "lcars" || m.Status.Theme().Name() != "lcars" || m.CommandBar.Theme().Name() != "lcars" {
		t.Fatalf("subcomponents theme not updated on F2")
	}

	// Press F2 again to cycle to CRT
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyF2})
	m = updated.(Model)
	if m.Theme.Name() != "crt" {
		t.Fatalf("expected theme cycled to crt, got: %s", m.Theme.Name())
	}

	// Press F2 again to cycle back to Modern
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyF2})
	m = updated.(Model)
	if m.Theme.Name() != "modern" {
		t.Fatalf("expected theme cycled back to modern, got: %s", m.Theme.Name())
	}
}

func TestModelQuitHandling(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	m := NewModel(g, theme.DefaultTheme())

	// Ctrl+C quits immediately
	_, cmd := m.Update(tea.KeyMsg{Type: tea.KeyCtrlC})
	if cmd == nil {
		t.Fatalf("expected quit command on Ctrl+C")
	}
	msg := cmd()
	if _, ok := msg.(tea.QuitMsg); !ok {
		t.Fatalf("expected tea.QuitMsg on Ctrl+C, got: %T", msg)
	}

	// Esc when input is empty quits
	_, cmd = m.Update(tea.KeyMsg{Type: tea.KeyEsc})
	if cmd == nil {
		t.Fatalf("expected quit command on Esc with empty input")
	}
	msg = cmd()
	if _, ok := msg.(tea.QuitMsg); !ok {
		t.Fatalf("expected tea.QuitMsg on Esc with empty input, got: %T", msg)
	}

	// Esc when input has text clears input and does NOT quit
	m.CommandBar.SetValue("some text")
	updated, cmd := m.Update(tea.KeyMsg{Type: tea.KeyEsc})
	m = updated.(Model)
	if cmd != nil {
		t.Fatalf("expected no command on Esc when input is non-empty")
	}
	if m.CommandBar.Value() != "" {
		t.Fatalf("expected command bar input reset on Esc, got: %q", m.CommandBar.Value())
	}

	// Submitting "quit" command returns tea.Quit
	_, cmd = m.Update(commandbar.CommandSubmittedMsg{Text: "quit"})
	if cmd == nil {
		t.Fatalf("expected quit command on 'quit'")
	}
	if _, ok := cmd().(tea.QuitMsg); !ok {
		t.Fatalf("expected tea.QuitMsg on 'quit'")
	}

	// Submitting "exit" command returns tea.Quit
	_, cmd = m.Update(commandbar.CommandSubmittedMsg{Text: "exit"})
	if cmd == nil {
		t.Fatalf("expected quit command on 'exit'")
	}
	if _, ok := cmd().(tea.QuitMsg); !ok {
		t.Fatalf("expected tea.QuitMsg on 'exit'")
	}
}

func TestModelCommandSubmitted_Actions(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	m := NewModel(g, theme.DefaultTheme())

	initialEnergy := g.Enterprise.Energy
	initialShields := g.Enterprise.Shields

	// Submit valid shields command: she 500
	updated, cmd := m.Update(commandbar.CommandSubmittedMsg{Text: "she 500"})
	m = updated.(Model)
	if cmd != nil {
		t.Fatalf("unexpected command from she 500: %v", cmd)
	}

	if g.Enterprise.Shields != 500 {
		t.Fatalf("expected shields 500, got: %.0f", g.Enterprise.Shields)
	}
	if g.Enterprise.Energy != initialEnergy-(500-initialShields) {
		t.Fatalf("expected energy deducted, got: %.0f", g.Enterprise.Energy)
	}

	// Verify event was logged in CommandBar
	msgs := m.CommandBar.Messages()
	foundShieldMsg := false
	for _, msg := range msgs {
		if strings.Contains(msg, "Shield") || strings.Contains(msg, "Energy") {
			foundShieldMsg = true
			break
		}
	}
	if !foundShieldMsg {
		t.Fatalf("expected shield transfer event in command bar messages, got: %v", msgs)
	}

	// Submit invalid action: excessive shields transfer
	updated, _ = m.Update(commandbar.CommandSubmittedMsg{Text: "she 999999"})
	m = updated.(Model)
	msgs = m.CommandBar.Messages()
	foundError := false
	for _, msg := range msgs {
		if strings.Contains(strings.ToLower(msg), "energy") || strings.Contains(strings.ToLower(msg), "shield") {
			foundError = true
			break
		}
	}
	if !foundError {
		t.Fatalf("expected error message for excessive shields in command bar messages: %v", msgs)
	}
}

func TestModelCommandSubmitted_Special(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	m := NewModel(g, theme.DefaultTheme())

	// Special: theme cycling
	updated, _ := m.Update(commandbar.CommandSubmittedMsg{Text: "theme"})
	m = updated.(Model)
	if m.Theme.Name() != "lcars" {
		t.Fatalf("expected theme lcars after 'theme', got: %s", m.Theme.Name())
	}

	// Special: named theme switch
	updated, _ = m.Update(commandbar.CommandSubmittedMsg{Text: "theme crt"})
	m = updated.(Model)
	if m.Theme.Name() != "crt" {
		t.Fatalf("expected theme crt after 'theme crt', got: %s", m.Theme.Name())
	}

	// Special: help
	updated, _ = m.Update(commandbar.CommandSubmittedMsg{Text: "help"})
	m = updated.(Model)
	msgs := m.CommandBar.Messages()
	foundHelp := false
	for _, msg := range msgs {
		if strings.Contains(strings.ToLower(msg), "command") || strings.Contains(strings.ToLower(msg), "nav") {
			foundHelp = true
			break
		}
	}
	if !foundHelp {
		t.Fatalf("expected help message in command bar messages: %v", msgs)
	}

	// Error: unknown command
	updated, _ = m.Update(commandbar.CommandSubmittedMsg{Text: "xyz"})
	m = updated.(Model)
	msgs = m.CommandBar.Messages()
	foundUnknown := false
	for _, msg := range msgs {
		if strings.Contains(strings.ToLower(msg), "unknown") || strings.Contains(strings.ToLower(msg), "xyz") {
			foundUnknown = true
			break
		}
	}
	if !foundUnknown {
		t.Fatalf("expected unknown command error in command bar messages: %v", msgs)
	}
}

func TestModelKeyForwarding(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	m := NewModel(g, theme.DefaultTheme())

	// Type characters 'p', 'h', 'a'
	for _, r := range []rune{'p', 'h', 'a'} {
		updated, _ := m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{r}})
		m = updated.(Model)
	}

	if m.CommandBar.Value() != "pha" {
		t.Fatalf("expected command bar input 'pha', got: %q", m.CommandBar.Value())
	}
}

func TestModelInit(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	m := NewModel(g, theme.DefaultTheme())

	cmd := m.Init()
	if cmd == nil {
		t.Fatalf("expected non-nil tea.Cmd from Init() for cursor blink")
	}
}

func TestModelViewStructure(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	m := NewModel(g, theme.DefaultTheme())

	// Resize to 80x24
	updated, _ := m.Update(tea.WindowSizeMsg{Width: 80, Height: 24})
	m = updated.(Model)
	view := m.View()

	// Verify Header
	if !strings.Contains(view, "SUPER STAR TREK") {
		t.Fatalf("view missing header title:\n%s", view)
	}
	// Verify Grid axes and Enterprise
	if !strings.Contains(view, "<E>") {
		t.Fatalf("view missing Enterprise in grid:\n%s", view)
	}
	// Verify Status Panel
	if !strings.Contains(view, "CONDITION GREEN") {
		t.Fatalf("view missing condition banner:\n%s", view)
	}
	if !strings.Contains(view, "Stardate:") {
		t.Fatalf("view missing stardate:\n%s", view)
	}
	// Verify Command Bar prompt
	if !strings.Contains(view, "COMMAND>") {
		t.Fatalf("view missing command bar prompt:\n%s", view)
	}
}

func TestModelNilGameGraceful(t *testing.T) {
	m := NewModel(nil, nil)

	// Update size
	updated, _ := m.Update(tea.WindowSizeMsg{Width: 80, Height: 24})
	m = updated.(Model)

	// View should render without panic
	view := m.View()
	if !strings.Contains(view, "SUPER STAR TREK") {
		t.Fatalf("expected view to render with nil game:\n%s", view)
	}

	// Action should be rejected cleanly
	updated, _ = m.Update(commandbar.CommandSubmittedMsg{Text: "she 100"})
	m = updated.(Model)
	msgs := m.CommandBar.Messages()
	foundErr := false
	for _, msg := range msgs {
		if strings.Contains(strings.ToLower(msg), "no active game") || strings.Contains(strings.ToLower(msg), "error") {
			foundErr = true
			break
		}
	}
	if !foundErr {
		t.Fatalf("expected error message when game is nil, got: %v", msgs)
	}
}

func TestModelSelectedSector_ViewReticle(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	m := NewModel(g, theme.DefaultTheme())

	// Resize to standard 80x24 dashboard
	updated, _ := m.Update(tea.WindowSizeMsg{Width: 80, Height: 24})
	m = updated.(Model)

	// Initially SelectedSector is {0, 0}, so no reticle brackets
	viewInitial := m.View()
	if strings.Contains(viewInitial, "[.]") {
		t.Fatalf("expected no reticle brackets when SelectedSector is empty")
	}

	// Set SelectedSector to [3, 4]
	m.SelectedSector = engine.Coord{3, 4}
	viewSelected := m.View()
	if !strings.Contains(viewSelected, "[.]") && !strings.Contains(viewSelected, "[E]") && !strings.Contains(viewSelected, "[K]") {
		t.Fatalf("expected reticle bracketed cell in view when SelectedSector is [3, 4], got:\n%s", viewSelected)
	}
}

func TestModel_OpenTargetLockHotkey(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	g.Enterprise.Sector = engine.Coord{4, 4}
	klingon := &engine.Klingon{ID: 1, Sector: engine.Coord{4, 7}, Energy: 300}
	g.CurrentQuad.Klingons = []*engine.Klingon{klingon}
	g.CurrentQuad.Grid[4][7] = engine.EntityKlingon

	m := NewModel(g, theme.DefaultTheme())
	updated, _ := m.Update(tea.WindowSizeMsg{Width: 80, Height: 24})
	m = updated.(Model)

	// Press 't'
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'t'}})
	m = updated.(Model)

	if m.ActiveModal != ModalTargetLock {
		t.Fatalf("expected ActiveModal == ModalTargetLock, got %v", m.ActiveModal)
	}
	if m.TargetLock.CurrentTarget() == nil || m.TargetLock.CurrentTarget().KlingonID != 1 {
		t.Fatalf("expected target lock loaded with Klingon #1, got %v", m.TargetLock.CurrentTarget())
	}
	if !strings.Contains(m.View(), "TACTICAL TARGET LOCK") {
		t.Fatalf("expected view to contain target lock HUD, got:\n%s", m.View())
	}
}

func TestModel_OpenTargetLockNoEnemies(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	g.CurrentQuad.Klingons = nil

	m := NewModel(g, theme.DefaultTheme())
	updated, _ := m.Update(tea.WindowSizeMsg{Width: 80, Height: 24})
	m = updated.(Model)

	// Press 'T'
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'T'}})
	m = updated.(Model)

	if m.ActiveModal != ModalNone {
		t.Fatalf("expected ActiveModal == ModalNone, got %v", m.ActiveModal)
	}
	msgs := m.CommandBar.Messages()
	found := false
	for _, msg := range msgs {
		if strings.Contains(msg, "Sensors detect no hostile targets in sector.") {
			found = true
			break
		}
	}
	if !found {
		t.Fatalf("expected warning message in command bar, got %v", msgs)
	}
}

func TestModel_OpenCommandPaletteHotkey(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	m := NewModel(g, theme.DefaultTheme())
	updated, _ := m.Update(tea.WindowSizeMsg{Width: 80, Height: 24})
	m = updated.(Model)

	// Ctrl+P opens command palette
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyCtrlP})
	m = updated.(Model)
	if m.ActiveModal != ModalCommandPalette {
		t.Fatalf("expected ActiveModal == ModalCommandPalette after Ctrl+P, got %v", m.ActiveModal)
	}
	if !strings.Contains(m.View(), "COMMAND PALETTE") {
		t.Fatalf("expected view to contain COMMAND PALETTE modal, got:\n%s", m.View())
	}

	// Close palette
	m.ActiveModal = ModalNone

	// '/' also opens command palette
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'/'}})
	m = updated.(Model)
	if m.ActiveModal != ModalCommandPalette {
		t.Fatalf("expected ActiveModal == ModalCommandPalette after '/', got %v", m.ActiveModal)
	}
}

func TestModel_ModalDismissalEsc(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	klingon := &engine.Klingon{ID: 1, Sector: engine.Coord{4, 7}, Energy: 300}
	g.CurrentQuad.Klingons = []*engine.Klingon{klingon}
	m := NewModel(g, theme.DefaultTheme())

	// Open TargetLock modal
	updated, _ := m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'t'}})
	m = updated.(Model)
	if m.ActiveModal != ModalTargetLock {
		t.Fatalf("expected ActiveModal == ModalTargetLock, got %v", m.ActiveModal)
	}

	// Press Esc
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyEsc})
	m = updated.(Model)
	if m.ActiveModal != ModalNone {
		t.Fatalf("expected ActiveModal == ModalNone on Esc, got %v", m.ActiveModal)
	}
	if !m.CommandBar.Focused() {
		t.Fatalf("expected CommandBar refocused on Esc")
	}

	// Now open CommandPalette
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyCtrlP})
	m = updated.(Model)
	if m.ActiveModal != ModalCommandPalette {
		t.Fatalf("expected ActiveModal == ModalCommandPalette, got %v", m.ActiveModal)
	}

	// Press Esc
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyEsc})
	m = updated.(Model)
	if m.ActiveModal != ModalNone {
		t.Fatalf("expected ActiveModal == ModalNone on Esc, got %v", m.ActiveModal)
	}
	if !m.CommandBar.Focused() {
		t.Fatalf("expected CommandBar refocused on Esc")
	}
}

func TestModel_MouseClickSelectSector(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	m := NewModel(g, theme.DefaultTheme())

	// Row 3, Col 4: relY = 3 => msg.Y = 4, c = 4 => relX = 2 + 4*3 = 14 => msg.X = 14
	mouseMsg := tea.MouseMsg{
		X:      14,
		Y:      4,
		Button: tea.MouseButtonLeft,
		Action: tea.MouseActionPress,
	}
	updated, _ := m.Update(mouseMsg)
	m = updated.(Model)

	if m.SelectedSector != (engine.Coord{3, 4}) {
		t.Fatalf("expected SelectedSector == [3, 4], got %v", m.SelectedSector)
	}
	msgs := m.CommandBar.Messages()
	found := false
	for _, msg := range msgs {
		if strings.Contains(msg, "Target sector selected: [3, 4]") {
			found = true
			break
		}
	}
	if !found {
		t.Fatalf("expected sector selection logged, got %v", msgs)
	}
}

func TestModel_MouseClickKlingonOpensHUD(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	klingon := &engine.Klingon{ID: 2, Sector: engine.Coord{3, 4}, Energy: 300}
	g.CurrentQuad.Klingons = []*engine.Klingon{klingon}
	g.CurrentQuad.Grid[3][4] = engine.EntityKlingon

	m := NewModel(g, theme.DefaultTheme())
	mouseMsg := tea.MouseMsg{
		X:      14,
		Y:      4,
		Button: tea.MouseButtonLeft,
		Action: tea.MouseActionPress,
	}
	updated, _ := m.Update(mouseMsg)
	m = updated.(Model)

	if m.ActiveModal != ModalTargetLock {
		t.Fatalf("expected ActiveModal == ModalTargetLock, got %v", m.ActiveModal)
	}
	if m.TargetLock.CurrentTarget() == nil || m.TargetLock.CurrentTarget().Coord != (engine.Coord{3, 4}) {
		t.Fatalf("expected TargetLock pre-targeted at [3, 4], got %v", m.TargetLock.CurrentTarget())
	}
}

func TestModel_MouseDoubleClickImpulseMove(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	g.Enterprise.Sector = engine.Coord{4, 4}
	g.CurrentQuad.Grid[4][4] = engine.EntityEnterprise
	g.CurrentQuad.Grid[5][5] = engine.EntityEmpty

	m := NewModel(g, theme.DefaultTheme())

	// Cell [5, 5]: relY = 5 => msg.Y = 6, c = 5 => relX = 2 + 4*4 = 18 => msg.X = 18
	mouseMsg := tea.MouseMsg{
		X:      18,
		Y:      6,
		Button: tea.MouseButtonLeft,
		Action: tea.MouseActionPress,
	}

	// First click
	updated, _ := m.Update(mouseMsg)
	m = updated.(Model)
	if m.SelectedSector != (engine.Coord{5, 5}) {
		t.Fatalf("expected SelectedSector [5, 5], got %v", m.SelectedSector)
	}

	// Second click immediately (double-click < 400ms)
	updated, _ = m.Update(mouseMsg)
	m = updated.(Model)

	if g.Enterprise.Sector != (engine.Coord{5, 5}) {
		t.Fatalf("expected Enterprise moved to [5, 5], got %v", g.Enterprise.Sector)
	}
}

func TestModel_MouseDoubleClickDock(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	g.Enterprise.Sector = engine.Coord{4, 4}
	g.CurrentQuad.Grid[4][4] = engine.EntityEnterprise
	sbCoord := engine.Coord{4, 5}
	g.CurrentQuad.Starbase = &sbCoord
	g.CurrentQuad.Grid[4][5] = engine.EntityStarbase

	m := NewModel(g, theme.DefaultTheme())

	// Starbase [4, 5]: relY = 4 => msg.Y = 5, c = 5 => relX = 18 => msg.X = 18
	mouseMsg := tea.MouseMsg{
		X:      18,
		Y:      5,
		Button: tea.MouseButtonLeft,
		Action: tea.MouseActionPress,
	}

	// First click
	updated, _ := m.Update(mouseMsg)
	m = updated.(Model)

	// Second click
	updated, _ = m.Update(mouseMsg)
	m = updated.(Model)

	if g.Enterprise.Condition != engine.ConditionDocked {
		t.Fatalf("expected Enterprise ConditionDocked after double-clicking Starbase, got %v", g.Enterprise.Condition)
	}
}

func TestModel_ModalActionDispatch(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	g.Enterprise.Sector = engine.Coord{4, 4}
	g.Enterprise.Torpedoes = 10
	klingon := &engine.Klingon{ID: 1, Sector: engine.Coord{4, 7}, Energy: 200}
	g.CurrentQuad.Klingons = []*engine.Klingon{klingon}
	g.CurrentQuad.Grid[4][7] = engine.EntityKlingon

	m := NewModel(g, theme.DefaultTheme())
	m.ActiveModal = ModalTargetLock

	// Dispatch FireTorpedoMsg
	updated, _ := m.Update(targetlock.FireTorpedoMsg{Target: engine.Coord{4, 7}, Bearing: 1.0})
	m = updated.(Model)
	if m.ActiveModal != ModalNone {
		t.Fatalf("expected modal closed after torpedo fire, got %v", m.ActiveModal)
	}
	if g.Enterprise.Torpedoes != 9 {
		t.Fatalf("expected torpedo count decremented to 9, got %d", g.Enterprise.Torpedoes)
	}
	if !m.CommandBar.Focused() {
		t.Fatalf("expected CommandBar refocused after torpedo fire")
	}

	// Dispatch FirePhasersMsg
	m.ActiveModal = ModalTargetLock
	initEnergy := g.Enterprise.Energy
	updated, _ = m.Update(targetlock.FirePhasersMsg{Energy: 200})
	m = updated.(Model)
	if m.ActiveModal != ModalNone {
		t.Fatalf("expected modal closed after phaser fire, got %v", m.ActiveModal)
	}
	if g.Enterprise.Energy >= initEnergy {
		t.Fatalf("expected energy deducted after phaser fire, got %.0f", g.Enterprise.Energy)
	}
}

func TestModel_ModalMessageRouting_Palette(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	m := NewModel(g, theme.DefaultTheme())
	m.ActiveModal = ModalCommandPalette

	// Parameterized command selection
	updated, _ := m.Update(commandpalette.CommandSelectedMsg{
		CommandPrefix: "tor ",
		Parameterized: true,
	})
	m = updated.(Model)
	if m.ActiveModal != ModalNone {
		t.Fatalf("expected modal closed, got %v", m.ActiveModal)
	}
	if m.CommandBar.Value() != "tor " {
		t.Fatalf("expected command bar text 'tor ', got %q", m.CommandBar.Value())
	}
	if !m.CommandBar.Focused() {
		t.Fatalf("expected CommandBar focused")
	}

	// Non-parameterized command selection (e.g. quit)
	m.ActiveModal = ModalCommandPalette
	_, cmd := m.Update(commandpalette.CommandSelectedMsg{
		CommandPrefix: "quit",
		Parameterized: false,
	})
	if cmd == nil {
		t.Fatalf("expected quit cmd on 'quit' palette selection")
	}

	// ClosePaletteMsg
	m.ActiveModal = ModalCommandPalette
	updated, _ = m.Update(commandpalette.ClosePaletteMsg{})
	m = updated.(Model)
	if m.ActiveModal != ModalNone {
		t.Fatalf("expected modal closed on ClosePaletteMsg, got %v", m.ActiveModal)
	}
	if !m.CommandBar.Focused() {
		t.Fatalf("expected CommandBar focused on ClosePaletteMsg")
	}
}

func TestModel_MouseClicksSeparatedByTimeDoNotDoubleClick(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	g.Enterprise.Sector = engine.Coord{4, 4}
	g.CurrentQuad.Grid[4][4] = engine.EntityEnterprise
	g.CurrentQuad.Grid[5][5] = engine.EntityEmpty

	m := NewModel(g, theme.DefaultTheme())

	// Cell [5, 5]: relY = 5 => msg.Y = 6, c = 5 => relX = 18 => msg.X = 18
	mouseMsg := tea.MouseMsg{
		X:      18,
		Y:      6,
		Button: tea.MouseButtonLeft,
		Action: tea.MouseActionPress,
	}

	updated, _ := m.Update(mouseMsg)
	m = updated.(Model)

	// Simulate passage of time > 400ms
	m.LastClickTime = time.Now().Add(-500 * time.Millisecond)

	updated, _ = m.Update(mouseMsg)
	m = updated.(Model)

	// Enterprise should NOT have moved
	if g.Enterprise.Sector != (engine.Coord{4, 4}) {
		t.Fatalf("expected Enterprise to remain at [4, 4], got %v", g.Enterprise.Sector)
	}
}

func TestModel_OpenCommandPaletteWithInputBuffer(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	m := NewModel(g, theme.DefaultTheme())
	m.CommandBar.SetValue("nav 1.0")

	// Ctrl+P opens command palette even with text in command bar
	updated, _ := m.Update(tea.KeyMsg{Type: tea.KeyCtrlP})
	m = updated.(Model)
	if m.ActiveModal != ModalCommandPalette {
		t.Fatalf("expected ActiveModal == ModalCommandPalette with non-empty input buffer, got %v", m.ActiveModal)
	}
}

func TestModel_ThemePropagationToOpenOverlays(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	klingon := &engine.Klingon{ID: 1, Sector: engine.Coord{4, 7}, Energy: 300}
	g.CurrentQuad.Klingons = []*engine.Klingon{klingon}
	m := NewModel(g, theme.DefaultTheme())

	// Open TargetLock modal
	updated, _ := m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'t'}})
	m = updated.(Model)
	if m.ActiveModal != ModalTargetLock {
		t.Fatalf("expected ActiveModal == ModalTargetLock, got %v", m.ActiveModal)
	}

	// Press F2 to cycle theme while TargetLock is open
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyF2})
	m = updated.(Model)
	if m.Theme.Name() != "lcars" {
		t.Fatalf("expected root theme lcars, got %s", m.Theme.Name())
	}
	if m.TargetLock.Theme().Name() != "lcars" {
		t.Fatalf("expected TargetLock theme propagated to lcars, got %s", m.TargetLock.Theme().Name())
	}

	// Close modal and open CommandPalette
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyEsc})
	m = updated.(Model)
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyCtrlP})
	m = updated.(Model)
	if m.ActiveModal != ModalCommandPalette {
		t.Fatalf("expected ActiveModal == ModalCommandPalette, got %v", m.ActiveModal)
	}

	// Press F2 to cycle theme while CommandPalette is open
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyF2})
	m = updated.(Model)
	if m.Theme.Name() != "crt" {
		t.Fatalf("expected root theme crt, got %s", m.Theme.Name())
	}
	if m.CommandPalette.Theme().Name() != "crt" {
		t.Fatalf("expected CommandPalette theme propagated to crt, got %s", m.CommandPalette.Theme().Name())
	}
}

func TestModel_MouseDoubleClickThirdClickDoesNotDoubleClick(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	g.Enterprise.Sector = engine.Coord{4, 4}
	g.CurrentQuad.Grid[4][4] = engine.EntityEnterprise
	g.CurrentQuad.Grid[5][5] = engine.EntityEmpty

	m := NewModel(g, theme.DefaultTheme())

	// Cell [5, 5]: relY = 5 => msg.Y = 6, c = 5 => relX = 18 => msg.X = 18
	mouseMsg := tea.MouseMsg{
		X:      18,
		Y:      6,
		Button: tea.MouseButtonLeft,
		Action: tea.MouseActionPress,
	}

	// First click
	updated, _ := m.Update(mouseMsg)
	m = updated.(Model)

	// Second click (double click -> moves Enterprise to [5, 5])
	updated, _ = m.Update(mouseMsg)
	m = updated.(Model)
	if g.Enterprise.Sector != (engine.Coord{5, 5}) {
		t.Fatalf("expected Enterprise moved to [5, 5] on 2nd click")
	}

	// Rapid 3rd click on [5, 5] - should NOT trigger another double-click (since click pair was consumed)
	// LastClickTime should be reset to time.Time{} right after double-click, and then 3rd click sets it to time.Now()
	updated, _ = m.Update(mouseMsg)
	m = updated.(Model)
	if m.LastClickTime.IsZero() {
		t.Fatalf("expected 3rd click to record new non-zero LastClickTime")
	}
}

