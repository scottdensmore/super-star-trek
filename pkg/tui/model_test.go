package tui

import (
	"fmt"
	"path/filepath"
	"strings"
	"testing"
	"time"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/charmbracelet/x/ansi"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/anim"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/commandbar"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/commandpalette"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/damageschematic"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/galacticchart"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/halloffame"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/savebrowser"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/scenariomodal"
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
	TestModel_ViewStructure(t)
}

func TestModel_ViewStructure(t *testing.T) {
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
	if !strings.Contains(view, "USS ENTERPRISE") {
		t.Fatalf("view missing Enterprise title in header:\n%s", view)
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

	// Verify ModalCommandPalette compositing over background dashboard
	mPalette := m
	mPalette.ActiveModal = ModalCommandPalette
	paletteView := mPalette.View()

	if !strings.Contains(paletteView, "COMMAND PALETTE") {
		t.Fatalf("palette overlay missing modal title in view:\n%s", paletteView)
	}
	if !strings.Contains(paletteView, "SUPER STAR TREK") {
		t.Fatalf("palette overlay missing background header 'SUPER STAR TREK':\n%s", paletteView)
	}
	if !strings.Contains(paletteView, "USS ENTERPRISE") {
		t.Fatalf("palette overlay missing background header 'USS ENTERPRISE':\n%s", paletteView)
	}
	if !strings.Contains(paletteView, "COMMAND>") {
		t.Fatalf("palette overlay missing background command prompt 'COMMAND>':\n%s", paletteView)
	}

	// Verify ModalTargetLock compositing over background dashboard
	klingon := &engine.Klingon{ID: 1, Sector: engine.Coord{4, 7}, Energy: 300}
	g.CurrentQuad.Klingons = []*engine.Klingon{klingon}
	g.CurrentQuad.Grid[4][7] = engine.EntityKlingon
	updatedTarget, _ := m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'t'}})
	mTarget := updatedTarget.(Model)
	if mTarget.ActiveModal != ModalTargetLock {
		t.Fatalf("expected ActiveModal == ModalTargetLock, got %v", mTarget.ActiveModal)
	}
	targetView := mTarget.View()

	if !strings.Contains(targetView, "TACTICAL TARGET LOCK") {
		t.Fatalf("target lock overlay missing modal title in view:\n%s", targetView)
	}
	if !strings.Contains(targetView, "SUPER STAR TREK") {
		t.Fatalf("target lock overlay missing background header 'SUPER STAR TREK':\n%s", targetView)
	}
	if !strings.Contains(targetView, "USS ENTERPRISE") {
		t.Fatalf("target lock overlay missing background header 'USS ENTERPRISE':\n%s", targetView)
	}
	if !strings.Contains(targetView, "COMMAND>") {
		t.Fatalf("target lock overlay missing background command prompt 'COMMAND>':\n%s", targetView)
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

	// Row 3, Col 4: header is line 0, grid top border line 1, col header line 2, rows start line 3.
	// Row 3 is line 5. Col 4 starts at col 15, center is col 16.
	mouseMsg := tea.MouseMsg{
		X:      16,
		Y:      5,
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
		X:      16,
		Y:      5,
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

	// Cell [5, 5]: Row 5 is line 7. Col 5 center is col 20.
	mouseMsg := tea.MouseMsg{
		X:      20,
		Y:      7,
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

	// Starbase [4, 5]: Row 4 is line 6, Col 5 center is col 20.
	mouseMsg := tea.MouseMsg{
		X:      20,
		Y:      6,
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

	// Cell [5, 5]: Row 5 is line 7, Col 5 center is col 20.
	mouseMsg := tea.MouseMsg{
		X:      20,
		Y:      7,
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

	// Cell [5, 5]: Row 5 is line 7, Col 5 center is col 20.
	mouseMsg := tea.MouseMsg{
		X:      20,
		Y:      7,
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

func TestModalSaveBrowserIntegration(t *testing.T) {
	tempDir := t.TempDir()
	g := engine.NewGame(12345, engine.SkillNovice, engine.LengthShort)
	savePath := filepath.Join(tempDir, "TESTSAV.TRK")
	g.Stardate = 3150.0
	_ = g.Save(savePath)

	m := NewModel(g, theme.DefaultTheme())
	m.Width = 100
	m.Height = 30
	m.SaveBrowser = savebrowser.New(theme.DefaultTheme(), tempDir)

	// Test 1: Open via Ctrl+O
	m, _ = m.UpdateModel(tea.KeyMsg{Type: tea.KeyCtrlO})
	if m.ActiveModal != ModalSaveBrowser {
		t.Fatalf("expected ActiveModal == ModalSaveBrowser after Ctrl+O, got %v", m.ActiveModal)
	}

	// Test 2: Dismiss via CloseBrowserMsg
	m, _ = m.UpdateModel(savebrowser.CloseBrowserMsg{})
	if m.ActiveModal != ModalNone {
		t.Fatalf("expected ActiveModal == ModalNone after CloseBrowserMsg, got %v", m.ActiveModal)
	}

	// Test 3: Open via "saves" command
	m, _ = m.UpdateModel(commandbar.CommandSubmittedMsg{Text: "saves"})
	if m.ActiveModal != ModalSaveBrowser {
		t.Fatalf("expected ActiveModal == ModalSaveBrowser after 'saves' command, got %v", m.ActiveModal)
	}

	// Test 4: Thaw via LoadGameMsg
	m, _ = m.UpdateModel(savebrowser.LoadGameMsg{Path: savePath})
	if m.ActiveModal != ModalNone {
		t.Fatalf("expected ActiveModal == ModalNone after LoadGameMsg, got %v", m.ActiveModal)
	}
	if m.Game.Stardate != 3150.0 {
		t.Errorf("expected loaded Game.Stardate 3150.0, got %f", m.Game.Stardate)
	}

	// Test 5: Open via "thaw" command without args
	m, _ = m.UpdateModel(commandbar.CommandSubmittedMsg{Text: "thaw"})
	if m.ActiveModal != ModalSaveBrowser {
		t.Fatalf("expected ActiveModal == ModalSaveBrowser after bare 'thaw' command, got %v", m.ActiveModal)
	}

	// Test 6: Direct load via "thaw <filename>"
	m.ActiveModal = ModalNone
	m, _ = m.UpdateModel(commandbar.CommandSubmittedMsg{Text: "thaw " + savePath})
	if m.Game.Stardate != 3150.0 {
		t.Errorf("expected loaded Game.Stardate 3150.0 from thaw command, got %f", m.Game.Stardate)
	}
}

func TestModalSaveBrowser_ThemePropagation(t *testing.T) {
	tempDir := t.TempDir()
	g := engine.NewGame(12345, engine.SkillNovice, engine.LengthShort)
	m := NewModel(g, theme.DefaultTheme())
	m.Width = 100
	m.Height = 30
	m.SaveBrowser = savebrowser.New(theme.DefaultTheme(), tempDir)

	// Open save browser via Ctrl+O
	m, _ = m.UpdateModel(tea.KeyMsg{Type: tea.KeyCtrlO})
	if m.ActiveModal != ModalSaveBrowser {
		t.Fatalf("expected ActiveModal == ModalSaveBrowser, got %v", m.ActiveModal)
	}

	// Cycle theme with F2 while save browser is open
	m, _ = m.UpdateModel(tea.KeyMsg{Type: tea.KeyF2})
	if m.Theme.Name() != "lcars" {
		t.Fatalf("expected root theme 'lcars', got %s", m.Theme.Name())
	}

	// Cycle theme again with F2 -> CRT
	m, _ = m.UpdateModel(tea.KeyMsg{Type: tea.KeyF2})
	if m.Theme.Name() != "crt" {
		t.Fatalf("expected root theme 'crt', got %s", m.Theme.Name())
	}
}

func TestModalSaveBrowser_ViewOverlay(t *testing.T) {
	tempDir := t.TempDir()
	g := engine.NewGame(12345, engine.SkillNovice, engine.LengthShort)
	m := NewModel(g, theme.DefaultTheme())
	m.Width = 100
	m.Height = 30
	m.SaveBrowser = savebrowser.New(theme.DefaultTheme(), tempDir)

	m.ActiveModal = ModalSaveBrowser
	view := m.View()

	if !strings.Contains(view, "SAVED MISSIONS") {
		t.Fatalf("expected view to contain 'SAVED MISSIONS', got:\n%s", view)
	}
	if !strings.Contains(view, "SUPER STAR TREK") {
		t.Fatalf("expected background dashboard header in view:\n%s", view)
	}
}

func TestModalSaveBrowser_ThawError(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillNovice, engine.LengthShort)
	m := NewModel(g, theme.DefaultTheme())

	// Thaw non-existent file directly
	m, _ = m.UpdateModel(commandbar.CommandSubmittedMsg{Text: "thaw /path/that/does/not/exist.trk"})
	msgs := m.CommandBar.Messages()
	found := false
	for _, msg := range msgs {
		if strings.Contains(msg, "Failed to thaw") {
			found = true
			break
		}
	}
	if !found {
		t.Fatalf("expected 'Failed to thaw' error message in command bar, got %v", msgs)
	}

	// LoadGameMsg with invalid path
	m.ActiveModal = ModalSaveBrowser
	m, _ = m.UpdateModel(savebrowser.LoadGameMsg{Path: "/path/that/does/not/exist.trk"})
	if m.ActiveModal != ModalNone {
		t.Fatalf("expected ActiveModal == ModalNone after LoadGameMsg error, got %v", m.ActiveModal)
	}
	msgs = m.CommandBar.Messages()
	found = false
	for _, msg := range msgs {
		if strings.Contains(msg, "Failed to thaw") {
			found = true
			break
		}
	}
	if !found {
		t.Fatalf("expected 'Failed to thaw' error message after LoadGameMsg failure, got %v", msgs)
	}
}

func TestModel_RenderHeader_SingleLineNoWrap(t *testing.T) {
	themes := []theme.Theme{
		theme.ModernTheme{},
		theme.LcarsTheme{},
		theme.CrtTheme{},
	}
	widths := []int{80, 90, 100, 120}

	for _, th := range themes {
		for _, w := range widths {
			t.Run(fmt.Sprintf("%s_w%d", th.Name(), w), func(t *testing.T) {
				m := NewModel(nil, th)
				m.Width = w
				m.Height = 24

				header := m.renderHeader()

				// Header must be exactly a single line without wrapping.
				if strings.Contains(header, "\n") {
					t.Fatalf("expected header to be a single line without newlines, but got multiple lines:\n%s", header)
				}

				// The theme indicator must be present on this single line.
				expectedThemeTag := fmt.Sprintf("[Theme: %s (F2)]", strings.ToUpper(th.Name()))
				if !strings.Contains(header, expectedThemeTag) {
					t.Fatalf("expected header to contain %q, but got:\n%s", expectedThemeTag, header)
				}

				// Rendered width must match terminal width w.
				renderedWidth := lipgloss.Width(header)
				if renderedWidth != w {
					t.Fatalf("expected rendered width %d, got %d", w, renderedWidth)
				}
			})
		}
	}
}

func TestModel_HelpNavGuidance(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	m := NewModel(g, theme.DefaultTheme())

	m, _ = m.UpdateModel(commandbar.CommandSubmittedMsg{Text: "help nav"})
	msgs := m.CommandBar.Messages()

	joined := strings.Join(msgs, "\n")
	if !strings.Contains(joined, "NAV:") {
		t.Fatalf("expected NAV in help nav, got:\n%s", joined)
	}
	if !strings.Contains(joined, "Direct Quad: nav q <r c>") {
		t.Fatalf("expected direct quadrant help in help nav, got:\n%s", joined)
	}
	if !strings.Contains(joined, "Direct Sector: nav s <r c>") {
		t.Fatalf("expected direct sector help in help nav, got:\n%s", joined)
	}
	if !strings.Contains(joined, "Vector: nav <course> <warp>") {
		t.Fatalf("expected course & warp help in help nav, got:\n%s", joined)
	}
	if !strings.Contains(joined, "0.0=East") || !strings.Contains(joined, "1.57=North") {
		t.Fatalf("expected course angles in help nav, got:\n%s", joined)
	}
}

func TestModel_NavQuadrantCommand(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	m := NewModel(g, theme.DefaultTheme())

	// Move to Quadrant [4, 5]
	m, _ = m.UpdateModel(commandbar.CommandSubmittedMsg{Text: "nav q 4 5"})

	if m.Game.Enterprise.Quad != (engine.Coord{4, 5}) {
		t.Fatalf("expected enterprise in quad [4,5], got %v", m.Game.Enterprise.Quad)
	}

	msgs := m.CommandBar.Messages()
	found := false
	for _, msg := range msgs {
		if strings.Contains(msg, "Quadrant [4,5]") {
			found = true
			break
		}
	}
	if !found {
		t.Fatalf("expected quadrant arrival message in command bar, got %v", msgs)
	}
}

func TestModel_MoveQuadrantUpdatesSectorGrid(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	// Destination quadrant [3, 5]: 2 Klingons, 1 Starbase, 4 Stars = 214
	g.GalaxyChart[3][5] = 214

	m := NewModel(g, theme.ModernTheme{})
	updated, _ := m.Update(tea.WindowSizeMsg{Width: 80, Height: 24})
	m = updated.(Model)

	// Select a sector in old quadrant
	m.SelectedSector = engine.Coord{7, 7}

	// Move to Quadrant [3, 5]
	m, _ = m.UpdateModel(commandbar.CommandSubmittedMsg{Text: "nav q 3 5"})

	if m.Game.Enterprise.Quad != (engine.Coord{3, 5}) {
		t.Fatalf("expected enterprise in quad [3, 5], got %v", m.Game.Enterprise.Quad)
	}

	// Verify SelectedSector was reset to prevent stale reticle
	if m.SelectedSector != (engine.Coord{}) {
		t.Fatalf("expected SelectedSector reset upon quadrant change, got %v", m.SelectedSector)
	}

	// Verify CurrentQuad populated
	if len(m.Game.CurrentQuad.Klingons) != 2 {
		t.Fatalf("expected 2 Klingons in CurrentQuad, got %d", len(m.Game.CurrentQuad.Klingons))
	}
	if len(m.Game.CurrentQuad.Stars) != 4 {
		t.Fatalf("expected 4 stars in CurrentQuad, got %d", len(m.Game.CurrentQuad.Stars))
	}
	if m.Game.CurrentQuad.Starbase == nil {
		t.Fatalf("expected starbase in CurrentQuad, got nil")
	}

	// Verify View renders the entities and Red alert
	view := m.View()
	if !strings.Contains(view, "CONDITION RED") {
		t.Errorf("expected CONDITION RED in view, got:\n%s", view)
	}
	if !strings.Contains(view, "<E>") {
		t.Errorf("expected <E> in view, got:\n%s", view)
	}
	if !strings.Contains(view, ">B<") {
		t.Errorf("expected starbase >B< in view, got:\n%s", view)
	}
	if !strings.Contains(view, "+K+") {
		t.Errorf("expected Klingon glyph +K+ in view, got:\n%s", view)
	}
	if !strings.Contains(view, " * ") {
		t.Errorf("expected star glyph ' * ' in view, got:\n%s", view)
	}
}

func TestModel_GalacticChart_HotkeyAndCommand(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	m := NewModel(g, theme.DefaultTheme())

	// Test 1: 'chart' command activates ModalGalacticChart
	m, _ = m.UpdateModel(commandbar.CommandSubmittedMsg{Text: "chart"})
	if m.ActiveModal != ModalGalacticChart {
		t.Fatalf("expected ActiveModal=ModalGalacticChart after 'chart', got %v", m.ActiveModal)
	}

	// Test 2: Esc closes modal
	m, _ = m.UpdateModel(tea.KeyMsg{Type: tea.KeyEsc})
	if m.ActiveModal != ModalNone {
		t.Fatalf("expected ActiveModal=ModalNone after Esc, got %v", m.ActiveModal)
	}

	// Test 3: Ctrl+M hotkey activates ModalGalacticChart
	m, _ = m.UpdateModel(tea.KeyMsg{Type: tea.KeyCtrlM})
	if m.ActiveModal != ModalGalacticChart {
		t.Fatalf("expected ActiveModal=ModalGalacticChart after Ctrl+M, got %v", m.ActiveModal)
	}

	// Test 4: View composites modal over dashboard
	view := m.View()
	if !strings.Contains(view, "GALACTIC STAR CHART") {
		t.Fatalf("expected View to contain star chart overlay, got:\n%s", view)
	}
}

func TestModel_GalacticChart_WarpSelection(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	g.Enterprise.Quad = engine.Coord{3, 3}
	m := NewModel(g, theme.DefaultTheme())

	// Open chart
	m, _ = m.UpdateModel(commandbar.CommandSubmittedMsg{Text: "chart"})

	// Move cursor to [4, 5] and press Enter
	m, _ = m.UpdateModel(tea.KeyMsg{Type: tea.KeyDown})  // row 4
	m, _ = m.UpdateModel(tea.KeyMsg{Type: tea.KeyRight}) // col 4
	m, _ = m.UpdateModel(tea.KeyMsg{Type: tea.KeyRight}) // col 5

	var cmd tea.Cmd
	m, cmd = m.UpdateModel(tea.KeyMsg{Type: tea.KeyEnter})
	if cmd == nil {
		t.Fatalf("expected command returned on Enter in GalacticChart modal, got nil")
	}
	msg := cmd()
	warpMsg, ok := msg.(galacticchart.WarpToQuadrantMsg)
	if !ok {
		t.Fatalf("expected WarpToQuadrantMsg from cmd(), got %T", msg)
	}
	if warpMsg.DestQuad != (engine.Coord{4, 5}) {
		t.Fatalf("expected DestQuad [4,5], got %v", warpMsg.DestQuad)
	}
	if warpMsg.Warp != 2.2 {
		t.Fatalf("expected Warp 2.2, got %v", warpMsg.Warp)
	}
	m, _ = m.UpdateModel(msg)

	// Verify modal closed and Enterprise moved to [4, 5]
	if m.ActiveModal != ModalNone {
		t.Fatalf("expected modal closed, got %v", m.ActiveModal)
	}
	if m.Game.Enterprise.Quad != (engine.Coord{4, 5}) {
		t.Fatalf("expected Enterprise at quad [4,5], got %v", m.Game.Enterprise.Quad)
	}
}

func TestModel_HelpChartAndNavDistance(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	m := NewModel(g, theme.DefaultTheme())

	m, _ = m.UpdateModel(commandbar.CommandSubmittedMsg{Text: "help chart"})
	msgs := m.CommandBar.Messages()
	joined := strings.Join(msgs, "\n")
	if !strings.Contains(joined, "CHART:") || !strings.Contains(joined, "Ctrl+M") {
		t.Fatalf("expected help chart with Ctrl+M, got:\n%s", joined)
	}

	m, _ = m.UpdateModel(commandbar.CommandSubmittedMsg{Text: "help nav"})
	msgs = m.CommandBar.Messages()
	joined = strings.Join(msgs, "\n")
	if !strings.Contains(joined, "Ctrl+M") {
		t.Fatalf("expected help nav to mention Ctrl+M map tool, got:\n%s", joined)
	}
}

func TestModel_GalacticChart_SingleKeyHotkeys(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	m := NewModel(g, theme.DefaultTheme())

	// Press 'c' with empty input -> opens chart
	m, _ = m.UpdateModel(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'c'}})
	if m.ActiveModal != ModalGalacticChart {
		t.Fatalf("expected ActiveModal=ModalGalacticChart on 'c' with empty input, got %v", m.ActiveModal)
	}
	m, _ = m.UpdateModel(tea.KeyMsg{Type: tea.KeyEsc})

	// Press 'm' with empty input -> opens chart
	m, _ = m.UpdateModel(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'m'}})
	if m.ActiveModal != ModalGalacticChart {
		t.Fatalf("expected ActiveModal=ModalGalacticChart on 'm' with empty input, got %v", m.ActiveModal)
	}
	m, _ = m.UpdateModel(tea.KeyMsg{Type: tea.KeyEsc})

	// Type with existing text: 'c' should NOT open chart modal, should type into command bar
	m.CommandBar.SetValue("do")
	m, _ = m.UpdateModel(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'c'}})
	if m.ActiveModal != ModalNone {
		t.Fatalf("expected ActiveModal=ModalNone when typing 'c' into non-empty command bar, got %v", m.ActiveModal)
	}
	if m.CommandBar.Value() != "doc" {
		t.Fatalf("expected command bar value 'doc', got %q", m.CommandBar.Value())
	}
}

func TestModel_GalacticChart_ThemePropagation(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	m := NewModel(g, theme.DefaultTheme())

	// Open chart
	m, _ = m.UpdateModel(commandbar.CommandSubmittedMsg{Text: "chart"})
	if m.ActiveModal != ModalGalacticChart {
		t.Fatalf("expected ActiveModal=ModalGalacticChart, got %v", m.ActiveModal)
	}

	// Cycle theme with F2
	m, _ = m.UpdateModel(tea.KeyMsg{Type: tea.KeyF2})
	if m.Theme.Name() != "lcars" {
		t.Fatalf("expected root theme 'lcars', got %s", m.Theme.Name())
	}
	if m.GalacticChart.Theme().Name() != "lcars" {
		t.Fatalf("expected GalacticChart theme 'lcars', got %s", m.GalacticChart.Theme().Name())
	}

	// Cycle theme again with F2 -> CRT
	m, _ = m.UpdateModel(tea.KeyMsg{Type: tea.KeyF2})
	if m.Theme.Name() != "crt" {
		t.Fatalf("expected root theme 'crt', got %s", m.Theme.Name())
	}
	if m.GalacticChart.Theme().Name() != "crt" {
		t.Fatalf("expected GalacticChart theme 'crt', got %s", m.GalacticChart.Theme().Name())
	}
}

func TestModel_HelpMapCommand(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	m := NewModel(g, theme.DefaultTheme())

	m, _ = m.UpdateModel(commandbar.CommandSubmittedMsg{Text: "help map"})
	msgs := m.CommandBar.Messages()
	joined := strings.Join(msgs, "\n")
	if !strings.Contains(joined, "CHART:") || !strings.Contains(joined, "Ctrl+M") {
		t.Fatalf("expected help map to show chart help, got:\n%s", joined)
	}
}

func TestModel_LRScanCommand(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	g.Enterprise.Devices[engine.DeviceLRSensors] = 0
	for r := 1; r <= 8; r++ {
		for c := 1; c <= 8; c++ {
			g.ChartDiscovered[r][c] = false
		}
	}
	m := NewModel(g, theme.ModernTheme{})

	updated, _ := m.handleCommand("lrscan")
	mod := updated.(Model)

	messages := mod.CommandBar.Messages()
	if len(messages) == 0 || !strings.Contains(messages[len(messages)-1], "Long-range scan complete") {
		t.Errorf("expected completion message for lrscan, got: %v", messages)
	}
	if !g.ChartDiscovered[g.Enterprise.Quad[0]][g.Enterprise.Quad[1]] {
		t.Errorf("expected enterprise quad to be discovered after lrscan")
	}

	// Damaged LRS
	g.Enterprise.Devices[engine.DeviceLRSensors] = 2.0
	updatedDamaged, _ := mod.handleCommand("lrscan")
	modDamaged := updatedDamaged.(Model)
	messagesDamaged := modDamaged.CommandBar.Messages()
	if len(messagesDamaged) == 0 || !strings.Contains(messagesDamaged[len(messagesDamaged)-1], "LONG-RANGE SENSORS DAMAGED") {
		t.Errorf("expected damaged warning message for lrscan, got: %v", messagesDamaged)
	}
}

func TestModel_DockSurveillance(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	sbCoord := engine.Coord{4, 4}
	g.CurrentQuad.Starbase = &sbCoord
	g.CurrentQuad.Grid[4][4] = engine.EntityStarbase
	g.Enterprise.Sector = engine.Coord{4, 5}
	g.Enterprise.Condition = engine.ConditionGreen

	m := NewModel(g, theme.ModernTheme{})
	updated, _ := m.handleCommand("dock")
	mod := updated.(Model)

	messages := mod.CommandBar.Messages()
	foundSurveillance := false
	for _, msg := range messages {
		if strings.Contains(msg, "surveillance") {
			foundSurveillance = true
			break
		}
	}
	if !foundSurveillance {
		t.Errorf("expected docking to log starbase surveillance message, got: %v", messages)
	}
}

func TestModel_DynamicDamageReport(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	g.Enterprise.Devices[engine.DeviceLRSensors] = 3.2
	g.Enterprise.Devices[engine.DeviceComputer] = 1.5

	m := NewModel(g, theme.ModernTheme{})
	updated, _ := m.handleCommand("dam")
	mod := updated.(Model)

	if mod.ActiveModal != ModalDamageSchematic {
		t.Fatalf("expected ActiveModal = ModalDamageSchematic on 'dam', got %v", mod.ActiveModal)
	}
	view := mod.DamageSchematic.View()
	if !strings.Contains(view, "LRS") || !strings.Contains(view, "Computer") {
		t.Errorf("expected damage report to list damaged devices, got: %s", view)
	}
}

func TestModel_GalacticChart_WarpBlocked(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	g.Enterprise.Devices[engine.DeviceComputer] = 2.0 // Computer damaged!
	m := NewModel(g, theme.ModernTheme{})

	// Open chart
	m, _ = m.openGalacticChart()
	if m.ActiveModal != ModalGalacticChart {
		t.Fatalf("expected ModalGalacticChart")
	}

	// Send WarpBlockedMsg
	updated, _ := m.Update(galacticchart.WarpBlockedMsg{
		Reason: "COMPUTER DAMAGED, USE A POCKET CALCULATOR.",
	})
	mod := updated.(Model)

	if mod.ActiveModal != ModalNone {
		t.Errorf("expected modal to close on WarpBlockedMsg")
	}
	messages := mod.CommandBar.Messages()
	if len(messages) == 0 || !strings.Contains(messages[len(messages)-1], "COMPUTER DAMAGED") {
		t.Errorf("expected pocket calculator warning message, got: %v", messages)
	}
}

func TestModel_GalacticChart_MouseClick(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	g.Enterprise.Quad = engine.Coord{3, 3}
	m := NewModel(g, theme.DefaultTheme())
	updated, _ := m.Update(tea.WindowSizeMsg{Width: 80, Height: 24})
	m = updated.(Model)

	// Open galactic chart modal
	m, _ = m.UpdateModel(commandbar.CommandSubmittedMsg{Text: "chart"})
	if m.ActiveModal != ModalGalacticChart {
		t.Fatalf("expected ActiveModal == ModalGalacticChart, got %v", m.ActiveModal)
	}

	// Quadrant [2, 3]:
	// Chart overlay is 64x18 centered in 80x24: startX = 8, startY = 3.
	// Row 2 is line 5 in chart: screen Y = 3 + 5 = 8.
	// Col 3 center is col 23 in chart (7 + 7*2 + 2 = 23): screen X = 8 + 23 = 31.
	clickMsg := tea.MouseMsg{
		X:      31,
		Y:      8,
		Button: tea.MouseButtonLeft,
		Action: tea.MouseActionPress,
	}

	// Single click moves cursor
	updated, _ = m.Update(clickMsg)
	m = updated.(Model)
	if m.GalacticChart.Cursor() != (engine.Coord{2, 3}) {
		t.Fatalf("expected GalacticChart cursor [2, 3] after single click, got %v", m.GalacticChart.Cursor())
	}
	if m.ActiveModal != ModalGalacticChart {
		t.Fatalf("expected modal to remain open on single click")
	}

	// Double click warps to [2, 3]
	updated, _ = m.Update(clickMsg)
	m = updated.(Model)
	if m.ActiveModal != ModalNone {
		t.Fatalf("expected modal closed after double click warp, got %v", m.ActiveModal)
	}
	if m.Game.Enterprise.Quad != (engine.Coord{2, 3}) {
		t.Fatalf("expected Enterprise in quad [2, 3] after double click warp, got %v", m.Game.Enterprise.Quad)
	}
}

func TestModel_OptionsModal_Hotkey(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	m := NewModel(g, theme.DefaultTheme())

	// Press 'o' when command bar is empty
	updated, _ := m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'o'}})
	m = updated.(Model)
	if !m.showOptions {
		t.Fatalf("expected showOptions to be true after pressing 'o'")
	}
	if m.CommandBar.Focused() {
		t.Fatalf("expected CommandBar to be blurred")
	}
	if m.optionsModal.Rules().Profile != g.Rules.Profile {
		t.Fatalf("expected optionsModal rules synced to game rules")
	}

	// Close with esc
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyEsc})
	m = updated.(Model)
	if m.showOptions {
		t.Fatalf("expected showOptions to be false after Esc")
	}
	if !m.CommandBar.Focused() {
		t.Fatalf("expected CommandBar focused after closing options")
	}

	// Press 'O'
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'O'}})
	m = updated.(Model)
	if !m.showOptions {
		t.Fatalf("expected showOptions to be true after pressing 'O'")
	}
}

func TestModel_OptionsModal_Commands(t *testing.T) {
	for _, cmdStr := range []string{"opts", "options", "settings"} {
		t.Run(cmdStr, func(t *testing.T) {
			g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
			m := NewModel(g, theme.DefaultTheme())

			updated, _ := m.Update(commandbar.CommandSubmittedMsg{Text: cmdStr})
			m = updated.(Model)
			if !m.showOptions {
				t.Fatalf("expected showOptions to be true after submitting %q", cmdStr)
			}
			if m.CommandBar.Focused() {
				t.Fatalf("expected CommandBar to be blurred")
			}
		})
	}
}

func TestModel_OptionsModal_CycleAndSyncRules(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	m := NewModel(g, theme.DefaultTheme())

	// Open options modal
	updated, _ := m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'o'}})
	m = updated.(Model)

	// RowProfile is active. Press right arrow to cycle profile (normal -> hardcore)
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyRight})
	m = updated.(Model)
	if m.optionsModal.Rules().Profile != engine.ProfileHardcore {
		t.Fatalf("expected optionsModal profile to be hardcore, got %v", m.optionsModal.Rules().Profile)
	}

	// Close with 'q'
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'q'}})
	m = updated.(Model)
	if m.showOptions {
		t.Fatalf("expected showOptions to be false after 'q'")
	}
	// Game rules must now reflect hardcore!
	if m.Game.Rules.Profile != engine.ProfileHardcore {
		t.Fatalf("expected Game.Rules to be synced to hardcore, got %v", m.Game.Rules.Profile)
	}
	if !m.CommandBar.Focused() {
		t.Fatalf("expected CommandBar focused after close")
	}
	if m.optionsModal.Closed {
		t.Fatalf("expected optionsModal.Closed to be reset to false")
	}
}

func TestModel_OptionsModal_ViewRendering(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	m := NewModel(g, theme.DefaultTheme())

	m.showOptions = true
	view := m.View()
	if !strings.Contains(view, "STARFLEET CONFIGURATION & RULES") {
		t.Fatalf("expected view to contain modal title, got:\n%s", view)
	}
	if !strings.Contains(view, "Difficulty Profile") {
		t.Fatalf("expected view to contain 'Difficulty Profile', got:\n%s", view)
	}
	if !strings.Contains(view, "NORMAL") {
		t.Fatalf("expected view to contain 'NORMAL', got:\n%s", view)
	}
}

func TestModel_OptionsModal_ThemePropagation(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	m := NewModel(g, theme.DefaultTheme())

	m.showOptions = true
	// F2 while showOptions is true cycles theme
	updated, _ := m.Update(tea.KeyMsg{Type: tea.KeyF2})
	m = updated.(Model)
	if m.Theme.Name() != "lcars" {
		t.Fatalf("expected theme lcars, got %s", m.Theme.Name())
	}
	if m.optionsModal.Theme.Name() != "lcars" {
		t.Fatalf("expected optionsModal theme lcars, got %s", m.optionsModal.Theme.Name())
	}
}

func TestModel_DamageSchematicModal_OpenAndDismiss(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	mod := NewModel(g, theme.DefaultTheme())

	// 1. Open via command "dam"
	updated, _ := mod.handleCommand("dam")
	modDam := updated.(Model)
	if modDam.ActiveModal != ModalDamageSchematic {
		t.Fatalf("expected ActiveModal = ModalDamageSchematic on 'dam', got %v", modDam.ActiveModal)
	}

	// 2. Overlay rendered within 80x24 budget
	view := modDam.View()
	lines := strings.Split(view, "\n")
	if len(lines) != 24 {
		t.Errorf("expected exactly 24 lines, got %d", len(lines))
	}
	for i, line := range lines {
		w := ansi.StringWidth(line)
		if w != 80 {
			t.Errorf("line %d width = %d, expected 80", i, w)
		}
	}
	if !strings.Contains(view, "DAMAGE CONTROL SCHEMATIC") {
		t.Errorf("expected schematic title in overlay view")
	}

	// 3. Dismiss via Escape key
	escMsg := tea.KeyMsg{Type: tea.KeyEsc}
	updatedAfterEsc, cmd := modDam.Update(escMsg)
	if cmd != nil {
		updatedAfterEsc, _ = updatedAfterEsc.(Model).Update(cmd())
	}
	modClosed := updatedAfterEsc.(Model)
	if modClosed.ActiveModal != ModalNone {
		t.Errorf("expected ActiveModal = ModalNone after Esc, got %v", modClosed.ActiveModal)
	}
	if !modClosed.CommandBar.Focused() {
		t.Errorf("expected CommandBar focused after Esc")
	}
}

func TestModel_DamageSchematicModal_Commands(t *testing.T) {
	for _, cmd := range []string{"dam", "damage", "damages"} {
		t.Run(cmd, func(t *testing.T) {
			g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
			mod := NewModel(g, theme.DefaultTheme())
			updated, _ := mod.handleCommand(cmd)
			m := updated.(Model)
			if m.ActiveModal != ModalDamageSchematic {
				t.Fatalf("expected ActiveModal = ModalDamageSchematic on %q, got %v", cmd, m.ActiveModal)
			}
		})
	}
}

func TestModel_DamageSchematicModal_Hotkey(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	mod := NewModel(g, theme.DefaultTheme())

	// Press 'd' when command buffer is empty
	dKey := tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune("d")}
	updated, _ := mod.Update(dKey)
	modD := updated.(Model)
	if modD.ActiveModal != ModalDamageSchematic {
		t.Fatalf("expected ActiveModal = ModalDamageSchematic on 'd' hotkey, got %v", modD.ActiveModal)
	}

	// Press 'd' to close
	updatedClose, cmd := modD.Update(dKey)
	if cmd != nil {
		updatedClose, _ = updatedClose.(Model).Update(cmd())
	}
	modClosed := updatedClose.(Model)
	if modClosed.ActiveModal != ModalNone {
		t.Errorf("expected ActiveModal = ModalNone after 'd' dismiss, got %v", modClosed.ActiveModal)
	}
	if !modClosed.CommandBar.Focused() {
		t.Errorf("expected CommandBar focused after dismiss")
	}

	// Press 'D' when command buffer is empty
	dUpperKey := tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune("D")}
	updated, _ = mod.Update(dUpperKey)
	modDUpper := updated.(Model)
	if modDUpper.ActiveModal != ModalDamageSchematic {
		t.Fatalf("expected ActiveModal = ModalDamageSchematic on 'D' hotkey, got %v", modDUpper.ActiveModal)
	}

	// Dismiss via 'q'
	qKey := tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune("q")}
	updatedClose, cmd = modDUpper.Update(qKey)
	if cmd != nil {
		updatedClose, _ = updatedClose.(Model).Update(cmd())
	}
	modClosed = updatedClose.(Model)
	if modClosed.ActiveModal != ModalNone {
		t.Errorf("expected ActiveModal = ModalNone after 'q' dismiss, got %v", modClosed.ActiveModal)
	}

	// Press 'ctrl+d' when command buffer is empty
	ctrlDKey := tea.KeyMsg{Type: tea.KeyCtrlD}
	updated, _ = mod.Update(ctrlDKey)
	modCtrlD := updated.(Model)
	if modCtrlD.ActiveModal != ModalDamageSchematic {
		t.Fatalf("expected ActiveModal = ModalDamageSchematic on 'ctrl+d' hotkey, got %v", modCtrlD.ActiveModal)
	}

	// Dismiss via enter
	enterKey := tea.KeyMsg{Type: tea.KeyEnter}
	updatedClose, cmd = modCtrlD.Update(enterKey)
	if cmd != nil {
		updatedClose, _ = updatedClose.(Model).Update(cmd())
	}
	modClosed = updatedClose.(Model)
	if modClosed.ActiveModal != ModalNone {
		t.Errorf("expected ActiveModal = ModalNone after Enter dismiss, got %v", modClosed.ActiveModal)
	}

	// Press 'd' when command bar is not empty -> should NOT open modal, should append to command bar
	modWithText := mod
	modWithText.CommandBar.SetValue("com")
	updatedWithText, _ := modWithText.Update(dKey)
	modAppended := updatedWithText.(Model)
	if modAppended.ActiveModal != ModalNone {
		t.Errorf("expected ActiveModal = ModalNone when buffer not empty, got %v", modAppended.ActiveModal)
	}
	if modAppended.CommandBar.Value() != "comd" {
		t.Errorf("expected 'comd', got %q", modAppended.CommandBar.Value())
	}
}

func TestModel_DamageSchematicModal_CloseModalMsg(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	mod := NewModel(g, theme.DefaultTheme())

	updated, _ := mod.handleCommand("dam")
	modDam := updated.(Model)
	if modDam.ActiveModal != ModalDamageSchematic {
		t.Fatalf("expected ActiveModal = ModalDamageSchematic, got %v", modDam.ActiveModal)
	}

	updatedClose, _ := modDam.Update(damageschematic.CloseModalMsg{})
	modClosed := updatedClose.(Model)
	if modClosed.ActiveModal != ModalNone {
		t.Errorf("expected ActiveModal = ModalNone after CloseModalMsg, got %v", modClosed.ActiveModal)
	}
	if !modClosed.CommandBar.Focused() {
		t.Errorf("expected CommandBar focused after CloseModalMsg")
	}
}

func TestModel_DamageSchematicModal_ThemePropagation(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	mod := NewModel(g, theme.DefaultTheme())

	updated, _ := mod.handleCommand("dam")
	modDam := updated.(Model)
	if modDam.ActiveModal != ModalDamageSchematic {
		t.Fatalf("expected ActiveModal = ModalDamageSchematic, got %v", modDam.ActiveModal)
	}

	// F2 cycles theme while schematic is open
	updatedF2, _ := modDam.Update(tea.KeyMsg{Type: tea.KeyF2})
	modLcars := updatedF2.(Model)
	if modLcars.Theme.Name() != "lcars" {
		t.Errorf("expected theme lcars, got %s", modLcars.Theme.Name())
	}
	if modLcars.ActiveModal != ModalDamageSchematic {
		t.Errorf("expected modal to remain open after F2, got %v", modLcars.ActiveModal)
	}
}

func TestModel_HallOfFame_OpenAndDismiss(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	mod := NewModel(g, theme.DefaultTheme())

	// 1. Open via command "score"
	updated, _ := mod.handleCommand("score")
	modScore := updated.(Model)
	if modScore.ActiveModal != ModalHallOfFame {
		t.Fatalf("expected ActiveModal = ModalHallOfFame on 'score', got %v", modScore.ActiveModal)
	}

	// 2. Dimensions = 80x24
	view := modScore.View()
	lines := strings.Split(view, "\n")
	if len(lines) != 24 {
		t.Errorf("expected 24 lines, got %d", len(lines))
	}
	for i, l := range lines {
		if w := ansi.StringWidth(l); w != 80 {
			t.Errorf("line %d width = %d, expected 80", i, w)
		}
	}
	if !strings.Contains(view, "MISSION DEBRIEF") {
		t.Errorf("expected mission debrief title in view")
	}

	// 3. Dismiss via 'h' key
	hKey := tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune("h")}
	updatedAfterH, cmd := modScore.Update(hKey)
	if cmd != nil {
		updatedAfterH, _ = updatedAfterH.(Model).Update(cmd())
	}
	modClosed := updatedAfterH.(Model)
	if modClosed.ActiveModal != ModalNone {
		t.Errorf("expected ActiveModal = ModalNone after 'h' dismiss, got %v", modClosed.ActiveModal)
	}
	if !modClosed.CommandBar.Focused() {
		t.Errorf("expected CommandBar focused after dismiss")
	}
}

func TestModel_HallOfFame_Hotkeys(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	mod := NewModel(g, theme.DefaultTheme())

	// 'h' hotkey when command bar is empty
	hKey := tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune("h")}
	updated, _ := mod.Update(hKey)
	modH := updated.(Model)
	if modH.ActiveModal != ModalHallOfFame {
		t.Fatalf("expected ActiveModal = ModalHallOfFame on 'h' hotkey, got %v", modH.ActiveModal)
	}

	// Dismiss via 'esc'
	escKey := tea.KeyMsg{Type: tea.KeyEsc}
	updatedClose, cmd := modH.Update(escKey)
	if cmd != nil {
		updatedClose, _ = updatedClose.(Model).Update(cmd())
	}
	modClosed := updatedClose.(Model)
	if modClosed.ActiveModal != ModalNone {
		t.Errorf("expected ActiveModal = ModalNone after esc, got %v", modClosed.ActiveModal)
	}

	// 'H' hotkey when command bar is empty
	hUpperKey := tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune("H")}
	updated, _ = mod.Update(hUpperKey)
	modHUpper := updated.(Model)
	if modHUpper.ActiveModal != ModalHallOfFame {
		t.Fatalf("expected ActiveModal = ModalHallOfFame on 'H' hotkey, got %v", modHUpper.ActiveModal)
	}

	// Dismiss via 'q'
	qKey := tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune("q")}
	updatedClose, cmd = modHUpper.Update(qKey)
	if cmd != nil {
		updatedClose, _ = updatedClose.(Model).Update(cmd())
	}
	modClosed = updatedClose.(Model)
	if modClosed.ActiveModal != ModalNone {
		t.Errorf("expected ActiveModal = ModalNone after 'q', got %v", modClosed.ActiveModal)
	}

	// 'ctrl+h' hotkey when command bar is empty
	ctrlHKey := tea.KeyMsg{Type: tea.KeyCtrlH}
	updated, _ = mod.Update(ctrlHKey)
	modCtrlH := updated.(Model)
	if modCtrlH.ActiveModal != ModalHallOfFame {
		t.Fatalf("expected ActiveModal = ModalHallOfFame on 'ctrl+h' hotkey, got %v", modCtrlH.ActiveModal)
	}

	// 'h' when command bar is not empty -> should NOT open modal, should append
	modWithText := mod
	modWithText.CommandBar.SetValue("nav")
	updatedWithText, _ := modWithText.Update(hKey)
	modAppended := updatedWithText.(Model)
	if modAppended.ActiveModal != ModalNone {
		t.Errorf("expected ActiveModal = ModalNone when buffer not empty, got %v", modAppended.ActiveModal)
	}
	if modAppended.CommandBar.Value() != "navh" {
		t.Errorf("expected 'navh', got %q", modAppended.CommandBar.Value())
	}
}

func TestModel_HallOfFame_Commands(t *testing.T) {
	cmds := []string{"score", "scores", "halloffame", "hof"}
	for _, c := range cmds {
		t.Run(c, func(t *testing.T) {
			g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
			mod := NewModel(g, theme.DefaultTheme())
			updated, _ := mod.handleCommand(c)
			modCmd := updated.(Model)
			if modCmd.ActiveModal != ModalHallOfFame {
				t.Fatalf("expected ActiveModal = ModalHallOfFame on %q, got %v", c, modCmd.ActiveModal)
			}
		})
	}
}

func TestModel_HallOfFame_DismissalKeys(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	mod := NewModel(g, theme.DefaultTheme())

	// Dismiss via CloseModalMsg
	updated, _ := mod.handleCommand("score")
	modScore := updated.(Model)
	updatedClose, _ := modScore.Update(halloffame.CloseModalMsg{})
	modClosed := updatedClose.(Model)
	if modClosed.ActiveModal != ModalNone {
		t.Errorf("expected ActiveModal = ModalNone after CloseModalMsg, got %v", modClosed.ActiveModal)
	}
	if !modClosed.CommandBar.Focused() {
		t.Errorf("expected CommandBar focused after CloseModalMsg")
	}

	// Dismiss via 'q'
	updated, _ = mod.handleCommand("score")
	modScore = updated.(Model)
	qKey := tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune("q")}
	updatedClose, cmd := modScore.Update(qKey)
	if cmd != nil {
		updatedClose, _ = updatedClose.(Model).Update(cmd())
	}
	modClosed = updatedClose.(Model)
	if modClosed.ActiveModal != ModalNone {
		t.Errorf("expected ActiveModal = ModalNone after 'q', got %v", modClosed.ActiveModal)
	}
	if !modClosed.CommandBar.Focused() {
		t.Errorf("expected CommandBar focused after 'q'")
	}

	// Dismiss via 'esc'
	updated, _ = mod.handleCommand("score")
	modScore = updated.(Model)
	escKey := tea.KeyMsg{Type: tea.KeyEsc}
	updatedClose, cmd = modScore.Update(escKey)
	if cmd != nil {
		updatedClose, _ = updatedClose.(Model).Update(cmd())
	}
	modClosed = updatedClose.(Model)
	if modClosed.ActiveModal != ModalNone {
		t.Errorf("expected ActiveModal = ModalNone after 'esc', got %v", modClosed.ActiveModal)
	}
	if !modClosed.CommandBar.Focused() {
		t.Errorf("expected CommandBar focused after 'esc'")
	}
}

func TestModel_HallOfFame_ThemePropagation(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	mod := NewModel(g, theme.DefaultTheme())

	updated, _ := mod.handleCommand("score")
	modScore := updated.(Model)
	if modScore.ActiveModal != ModalHallOfFame {
		t.Fatalf("expected ActiveModal = ModalHallOfFame, got %v", modScore.ActiveModal)
	}

	// F2 cycles theme while modal is open
	updatedF2, _ := modScore.Update(tea.KeyMsg{Type: tea.KeyF2})
	modLcars := updatedF2.(Model)
	if modLcars.Theme.Name() != "lcars" {
		t.Errorf("expected theme lcars, got %s", modLcars.Theme.Name())
	}
	if modLcars.ActiveModal != ModalHallOfFame {
		t.Errorf("expected modal to remain open after F2, got %v", modLcars.ActiveModal)
	}
}

func TestModel_HallOfFame_GameOverFlow(t *testing.T) {
	// 1. Qualifying game-over event (Won with kills)
	gWin := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	gWin.Metrics.KlingonsKilled = 10
	gWin.GameWon = true
	modWin := NewModel(gWin, theme.DefaultTheme())

	updatedWin, _ := modWin.Update(engine.EventGameOver{Reason: engine.GameOverWon})
	modWinEnd := updatedWin.(Model)
	if modWinEnd.ActiveModal != ModalHallOfFame {
		t.Fatalf("expected ActiveModal = ModalHallOfFame on game-over won, got %v", modWinEnd.ActiveModal)
	}
	viewWin := modWinEnd.View()
	if !strings.Contains(viewWin, "ENTER CALLSIGN:") {
		t.Errorf("expected callsign prompt for qualifying game-over score, got:\n%s", viewWin)
	}

	// 2. Non-qualifying game-over event (Destroyed with negative/zero score)
	gLoss := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	gLoss.Metrics.Casualties = 1000
	gLoss.Metrics.StarbasesDestroyed = 5
	gLoss.GameWon = false
	modLoss := NewModel(gLoss, theme.DefaultTheme())

	updatedLoss, _ := modLoss.Update(engine.EventGameOver{Reason: engine.GameOverDestroyed})
	modLossEnd := updatedLoss.(Model)
	if modLossEnd.ActiveModal != ModalHallOfFame {
		t.Fatalf("expected ActiveModal = ModalHallOfFame on game-over destroyed, got %v", modLossEnd.ActiveModal)
	}
	viewLoss := modLossEnd.View()
	if strings.Contains(viewLoss, "ENTER CALLSIGN:") {
		t.Errorf("expected no callsign prompt for non-qualifying score, got:\n%s", viewLoss)
	}
	if !strings.Contains(viewLoss, "MISSION DEBRIEF") {
		t.Errorf("expected telemetry debrief view for non-qualifying score")
	}
}

func TestModel_HallOfFame_ScoreRecordedMsg(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	mod := NewModel(g, theme.DefaultTheme())

	entry := engine.ScoreEntry{
		CaptainName: "Kirk",
		Score:       1200,
		Rank:        "[ADM]",
	}
	updated, _ := mod.Update(halloffame.ScoreRecordedMsg{Entry: entry})
	modUpdated := updated.(Model)

	msgs := modUpdated.CommandBar.Messages()
	found := false
	for _, m := range msgs {
		if strings.Contains(m, "Score recorded") && strings.Contains(m, "Kirk") {
			found = true
			break
		}
	}
	if !found {
		t.Errorf("expected score recorded message in command bar, got %v", msgs)
	}
}

func TestModel_HallOfFame_TextInputKeystrokesNoLag(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	mod := NewModel(g, theme.DefaultTheme())

	// Open with promptName = true
	mod, _ = mod.openHallOfFame(true)
	if mod.ActiveModal != ModalHallOfFame {
		t.Fatalf("expected ActiveModal = ModalHallOfFame, got %v", mod.ActiveModal)
	}

	// Type callsign runes "Spock"
	// Each keystroke must return immediately without blocking on textinput.BlinkCmd() (530ms)
	callsign := "Spock"
	start := time.Now()
	for _, r := range callsign {
		keyMsg := tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{r}}
		updated, cmd := mod.Update(keyMsg)
		mod = updated.(Model)
		// cmd should be non-nil (the asynchronous cursor blink cmd from textinput),
		// but Update must NOT invoke it synchronously.
		if cmd == nil {
			t.Errorf("expected non-nil tea.Cmd for cursor blink on rune %c", r)
		}
	}
	elapsed := time.Since(start)

	// If BlinkCmd() was invoked synchronously, 5 keystrokes would take >2.5s.
	// Asynchronous return must take well under 100ms.
	if elapsed > 100*time.Millisecond {
		t.Errorf("typing keystrokes took %v, expected < 100ms (synchronous blink lag detected)", elapsed)
	}

	// Verify text input view contains "Spock"
	view := mod.View()
	if !strings.Contains(view, "Spock") {
		t.Errorf("expected view to contain callsign %q, got:\n%s", callsign, view)
	}

	// Press Enter to record score
	enterMsg := tea.KeyMsg{Type: tea.KeyEnter}
	updatedAfterEnter, cmd := mod.Update(enterMsg)
	mod = updatedAfterEnter.(Model)
	if cmd == nil {
		t.Fatalf("expected non-nil cmd from Enter key submitting score")
	}

	// Evaluate the command produced by Enter: should be ScoreRecordedMsg
	msg := cmd()
	scoreMsg, ok := msg.(halloffame.ScoreRecordedMsg)
	if !ok {
		t.Fatalf("expected halloffame.ScoreRecordedMsg on enter, got %T", msg)
	}
	if scoreMsg.Entry.CaptainName != "Spock" {
		t.Errorf("expected CaptainName 'Spock', got %q", scoreMsg.Entry.CaptainName)
	}

	// Dispatch ScoreRecordedMsg back to Model
	updatedAfterScore, _ := mod.Update(scoreMsg)
	mod = updatedAfterScore.(Model)

	// Verify logged confirmation in CommandBar
	msgs := mod.CommandBar.Messages()
	found := false
	for _, m := range msgs {
		if strings.Contains(m, "Score recorded") && strings.Contains(m, "Spock") {
			found = true
			break
		}
	}
	if !found {
		t.Errorf("expected ScoreRecordedMsg logged in command bar messages, got %v", msgs)
	}
}

func TestModel_GameOverLoggingNoDuplicates(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	g.Metrics.KlingonsKilled = 10
	g.GameWon = true
	mod := NewModel(g, theme.DefaultTheme())

	// Dispatch EventGameOver
	updated, _ := mod.Update(engine.EventGameOver{Reason: engine.GameOverWon, Score: 1000})
	modEnd := updated.(Model)

	count := 0
	for _, m := range modEnd.CommandBar.Messages() {
		if strings.Contains(m, "MISSION ACCOMPLISHED") {
			count++
		}
	}
	if count != 1 {
		t.Errorf("expected MISSION ACCOMPLISHED logged exactly once, got %d times in %v", count, modEnd.CommandBar.Messages())
	}

	// Test via action returning EventGameOver
	// Create simulated event slice with EventGameOver
	events := []engine.Event{
		engine.EventTorpedoHit{Target: engine.Coord{3, 3}, Destroyed: true},
		engine.EventGameOver{Reason: engine.GameOverWon, Score: 500},
	}
	mod2 := NewModel(g, theme.DefaultTheme())
	mod2.logEvents(events)
	updated2, _ := mod2.handleGameOver(engine.EventGameOver{Reason: engine.GameOverWon, Score: 500})
	mod2End := updated2
	count2 := 0
	for _, m := range mod2End.CommandBar.Messages() {
		if strings.Contains(m, "MISSION ACCOMPLISHED") {
			count2++
		}
	}
	if count2 != 1 {
		t.Errorf("expected MISSION ACCOMPLISHED logged exactly once when processed via logEvents + handleGameOver, got %d times", count2)
	}
}

func TestModel_Manual_OpenAndDismiss(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	mod := NewModel(g, theme.DefaultTheme())

	// 1. Open via command "help"
	updated, _ := mod.handleCommand("help")
	modHelp := updated.(Model)
	if modHelp.ActiveModal != ModalManual {
		t.Fatalf("expected ActiveModal = ModalManual on 'help', got %v", modHelp.ActiveModal)
	}

	// 2. Full-screen dimensions = 80x24
	view := modHelp.View()
	lines := strings.Split(view, "\n")
	if len(lines) != 24 {
		t.Errorf("expected 24 lines, got %d", len(lines))
	}
	for i, l := range lines {
		if w := ansi.StringWidth(l); w != 80 {
			t.Errorf("line %d width = %d, expected 80", i, w)
		}
	}

	// 3. Dismiss via 'esc' key
	escKey := tea.KeyMsg{Type: tea.KeyEsc}
	updatedAfterEsc, cmd := modHelp.Update(escKey)
	if cmd != nil {
		updatedAfterEsc, _ = updatedAfterEsc.(Model).Update(cmd())
	}
	modClosed := updatedAfterEsc.(Model)
	if modClosed.ActiveModal != ModalNone {
		t.Errorf("expected ActiveModal = ModalNone after 'esc' dismiss, got %v", modClosed.ActiveModal)
	}
}

func TestModel_Manual_TopicJumps(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	mod := NewModel(g, theme.DefaultTheme())

	// Open directly to combat via "help tor"
	updated, _ := mod.handleCommand("help tor")
	modTor := updated.(Model)
	if modTor.ActiveModal != ModalManual {
		t.Fatalf("expected ActiveModal = ModalManual on 'help tor', got %v", modTor.ActiveModal)
	}
	viewTor := modTor.View()
	if !strings.Contains(viewTor, "WEAPONS & COMBAT") {
		t.Errorf("expected Weapons & Combat chapter open for 'help tor'")
	}

	// Open directly to navigation via "man nav"
	updatedNav, _ := mod.handleCommand("man nav")
	modNav := updatedNav.(Model)
	viewNav := modNav.View()
	if !strings.Contains(viewNav, "NAVIGATION") {
		t.Errorf("expected Navigation chapter open for 'man nav'")
	}
}

func TestModel_Manual_Hotkeys(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	mod := NewModel(g, theme.DefaultTheme())

	// Hotkey '?' when command line empty
	qKey := tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune("?")}
	updated, _ := mod.Update(qKey)
	modQ := updated.(Model)
	if modQ.ActiveModal != ModalManual {
		t.Fatalf("expected ActiveModal = ModalManual on '?' hotkey, got %v", modQ.ActiveModal)
	}

	// Dismiss
	modQ.ActiveModal = ModalNone

	// Hotkey F1
	f1Key := tea.KeyMsg{Type: tea.KeyF1}
	updatedF1, _ := modQ.Update(f1Key)
	modF1 := updatedF1.(Model)
	if modF1.ActiveModal != ModalManual {
		t.Fatalf("expected ActiveModal = ModalManual on F1 hotkey, got %v", modF1.ActiveModal)
	}
}

func TestModel_CombatAnimation_KeySkip(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	mod := NewModel(g, theme.DefaultTheme())

	// Fire torpedo initiates animation
	updated, cmd := mod.handleCommand("tor 1 1")
	m := updated.(Model)
	if m.activeAnim == nil {
		t.Fatalf("expected activeAnim to be populated on torpedo fire")
	}
	if cmd == nil {
		t.Fatalf("expected TickCmd on active animation")
	}

	// Pressing any key immediately skips animation to completion
	keyMsg := tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune(" ")}
	updatedAfterKey, _ := m.Update(keyMsg)
	mSkipped := updatedAfterKey.(Model)

	if mSkipped.activeAnim != nil {
		t.Errorf("expected activeAnim cleared after keypress")
	}
}

func TestModel_CombatAnimation_SpeedOption(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	mod := NewModel(g, theme.DefaultTheme())

	// Disable animation via command: anim off
	updated, _ := mod.handleCommand("anim off")
	m := updated.(Model)
	if m.Game.Rules.AnimSpeed != 0 {
		t.Fatalf("expected AnimSpeed 0 on 'anim off', got %d", m.Game.Rules.AnimSpeed)
	}

	// Torpedo when speed is off does not start activeAnim
	updatedNoAnim, _ := m.handleCommand("tor 1 1")
	mNoAnim := updatedNoAnim.(Model)
	if mNoAnim.activeAnim != nil {
		t.Errorf("expected activeAnim to remain nil when AnimSpeed is off")
	}
}

func TestModel_CombatAnimation_TickMsg(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	mod := NewModel(g, theme.DefaultTheme())

	// Start animation
	updated, cmd := mod.handleCommand("tor 1 1")
	m := updated.(Model)
	if m.activeAnim == nil {
		t.Fatalf("expected activeAnim populated")
	}
	if cmd == nil {
		t.Fatalf("expected TickCmd")
	}

	animID := m.animID

	// Step ticks until finished
	step := 1
	for m.activeAnim != nil {
		tickMsg := anim.TickMsg{AnimID: animID, Step: step}
		resModel, nextCmd := m.Update(tickMsg)
		m = resModel.(Model)
		step++
		if m.activeAnim == nil {
			if nextCmd != nil {
				t.Errorf("expected nil cmd after animation finished")
			}
			break
		}
		if nextCmd == nil {
			t.Fatalf("expected non-nil nextCmd while animation in progress")
		}
		if step > 50 {
			t.Fatalf("animation did not terminate within 50 steps")
		}
	}

	if m.activeAnim != nil {
		t.Errorf("expected activeAnim to be nil upon completion")
	}
}

func TestModel_CombatAnimation_Phaser(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	// Place a Klingon in quadrant so phasers have a target
	g.CurrentQuad.Klingons = []*engine.Klingon{
		{ID: 1, Sector: engine.Coord{2, 2}, Energy: 200},
	}
	g.CurrentQuad.Grid[2][2] = engine.EntityKlingon
	g.Enterprise.Sector = engine.Coord{5, 5}
	g.Enterprise.Energy = 3000

	mod := NewModel(g, theme.DefaultTheme())

	// Fire phaser
	updated, cmd := mod.handleCommand("pha 200")
	m := updated.(Model)
	if m.activeAnim == nil {
		t.Fatalf("expected activeAnim to be populated on phaser fire")
	}
	if cmd == nil {
		t.Fatalf("expected TickCmd on active phaser animation")
	}

	// Key skip clears phaser animation
	keyMsg := tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune("x")}
	updatedAfterKey, _ := m.Update(keyMsg)
	mSkipped := updatedAfterKey.(Model)
	if mSkipped.activeAnim != nil {
		t.Errorf("expected activeAnim cleared after keypress on phasers")
	}
}

func TestModel_CombatAnimation_AnimCommands(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	mod := NewModel(g, theme.DefaultTheme())

	// anim fast
	updated, _ := mod.handleCommand("anim fast")
	m := updated.(Model)
	if m.Game.Rules.AnimSpeed != 1 {
		t.Errorf("expected speed 1, got %d", m.Game.Rules.AnimSpeed)
	}

	// anim cinematic
	updated, _ = m.handleCommand("anim cinematic")
	m = updated.(Model)
	if m.Game.Rules.AnimSpeed != 3 {
		t.Errorf("expected speed 3, got %d", m.Game.Rules.AnimSpeed)
	}

	// anim normal
	updated, _ = m.handleCommand("anim normal")
	m = updated.(Model)
	if m.Game.Rules.AnimSpeed != 2 {
		t.Errorf("expected speed 2, got %d", m.Game.Rules.AnimSpeed)
	}

	// bare anim cycles: 2 -> 3
	updated, _ = m.handleCommand("anim")
	m = updated.(Model)
	if m.Game.Rules.AnimSpeed != 3 {
		t.Errorf("expected speed 3 after bare anim, got %d", m.Game.Rules.AnimSpeed)
	}

	// bare anim cycles: 3 -> 0
	updated, _ = m.handleCommand("anim")
	m = updated.(Model)
	if m.Game.Rules.AnimSpeed != 0 {
		t.Errorf("expected speed 0 after bare anim, got %d", m.Game.Rules.AnimSpeed)
	}

	// bare anim cycles: 0 -> 1
	updated, _ = m.handleCommand("anim")
	m = updated.(Model)
	if m.Game.Rules.AnimSpeed != 1 {
		t.Errorf("expected speed 1 after bare anim, got %d", m.Game.Rules.AnimSpeed)
	}
}

func TestModel_CombatAnimation_TargetLockMsg(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	g.CurrentQuad.Klingons = []*engine.Klingon{
		{ID: 1, Sector: engine.Coord{2, 2}, Energy: 2000},
	}
	g.CurrentQuad.Grid[2][2] = engine.EntityKlingon
	g.Enterprise.Sector = engine.Coord{5, 5}
	g.Enterprise.Energy = 3000
	mod := NewModel(g, theme.DefaultTheme())

	// Target lock torpedo msg
	torpMsg := targetlock.FireTorpedoMsg{
		Target:  engine.Coord{2, 2},
		Bearing: 0.0,
	}
	updated, cmd := mod.Update(torpMsg)
	m := updated.(Model)
	if m.activeAnim == nil {
		t.Fatalf("expected activeAnim on targetlock.FireTorpedoMsg")
	}
	if cmd == nil {
		t.Fatalf("expected non-nil cmd on targetlock.FireTorpedoMsg")
	}

	// Clear animation
	m.activeAnim = nil
	m.Grid.ClearAnimOverrides()

	// Target lock phasers msg
	phaMsg := targetlock.FirePhasersMsg{
		Energy: 200,
	}
	updated, cmd = m.Update(phaMsg)
	m = updated.(Model)
	if m.activeAnim == nil {
		t.Fatalf("expected activeAnim on targetlock.FirePhasersMsg")
	}
	if cmd == nil {
		t.Fatalf("expected non-nil cmd on targetlock.FirePhasersMsg")
	}
}

func TestModel_ScenarioModal_Integration(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	mod := NewModel(g, theme.DefaultTheme())

	// 1. Open via command "scenarios"
	updated, _ := mod.handleCommand("scenarios")
	modScen := updated.(Model)
	if modScen.ActiveModal != ModalScenario {
		t.Fatalf("expected ActiveModal = ModalScenario on 'scenarios', got %v", modScen.ActiveModal)
	}

	// 2. View contains modal title and scenario names
	view := modScen.View()
	if !strings.Contains(view, "TACTICAL CHALLENGE SIMULATOR") {
		t.Errorf("expected View to contain modal title, got:\n%s", view)
	}
	if !strings.Contains(view, "Kobayashi Maru") {
		t.Errorf("expected View to contain 'Kobayashi Maru'")
	}
	if !strings.Contains(view, "[EXTREME]") {
		t.Errorf("expected View to contain '[EXTREME]'")
	}

	// 3. Navigate down to Mutara Nebula
	downKey := tea.KeyMsg{Type: tea.KeyDown}
	updated, _ = modScen.Update(downKey)
	modScen = updated.(Model)
	if modScen.scenarioModal.SelectedScenario().ID != engine.ScenarioMutaraNebula {
		t.Fatalf("expected selected scenario Mutara Nebula, got %v", modScen.scenarioModal.SelectedScenario().ID)
	}

	// 4. Launch on Enter
	enterKey := tea.KeyMsg{Type: tea.KeyEnter}
	updated, cmd := modScen.Update(enterKey)
	if cmd == nil {
		t.Fatalf("expected command on Enter, got nil")
	}
	launchMsg := cmd()
	updated, _ = updated.(Model).Update(launchMsg)
	modLaunched := updated.(Model)

	if modLaunched.ActiveModal != ModalNone {
		t.Errorf("expected ActiveModal = ModalNone after launch, got %v", modLaunched.ActiveModal)
	}
	if modLaunched.Game == nil || modLaunched.Game.Scenario != engine.ScenarioMutaraNebula {
		t.Errorf("expected new game scenario %v, got %+v", engine.ScenarioMutaraNebula, modLaunched.Game)
	}

	// 5. Open via short command "scen"
	updated, _ = modLaunched.handleCommand("scen")
	modScen2 := updated.(Model)
	if modScen2.ActiveModal != ModalScenario {
		t.Fatalf("expected ActiveModal = ModalScenario on 'scen', got %v", modScen2.ActiveModal)
	}

	// 6. Dismiss via 'q'
	qKey := tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'q'}}
	updated, cmd = modScen2.Update(qKey)
	if cmd != nil {
		updated, _ = updated.(Model).Update(cmd())
	}
	modDismissed := updated.(Model)
	if modDismissed.ActiveModal != ModalNone {
		t.Errorf("expected ActiveModal = ModalNone after 'q', got %v", modDismissed.ActiveModal)
	}

	// 7. Open via CommandPalette selection
	palMsg := commandpalette.CommandSelectedMsg{
		CommandPrefix: "scenarios",
		Parameterized: false,
	}
	updated, _ = modDismissed.Update(palMsg)
	modFromPal := updated.(Model)
	if modFromPal.ActiveModal != ModalScenario {
		t.Fatalf("expected ActiveModal = ModalScenario from palette, got %v", modFromPal.ActiveModal)
	}

	// 8. Dismiss via MsgCloseScenarioModal
	closeMsg := scenariomodal.MsgCloseScenarioModal{}
	updated, _ = modFromPal.Update(closeMsg)
	modClosed := updated.(Model)
	if modClosed.ActiveModal != ModalNone {
		t.Errorf("expected ActiveModal = ModalNone after MsgCloseScenarioModal, got %v", modClosed.ActiveModal)
	}
}



