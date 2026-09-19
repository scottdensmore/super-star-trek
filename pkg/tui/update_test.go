package tui

import (
	"fmt"
	"strings"
	"testing"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/commandpalette"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/optionsmodal"
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
