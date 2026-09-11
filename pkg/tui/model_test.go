package tui

import (
	"strings"
	"testing"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/commandbar"
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
