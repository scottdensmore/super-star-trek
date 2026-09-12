package savebrowser

import (
	"os"
	"path/filepath"
	"strings"
	"testing"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

func createTestSave(t *testing.T, dir, filename string, stardate float64, skill engine.SkillLevel) string {
	t.Helper()
	g := engine.NewGame(42, skill, engine.LengthShort)
	g.Stardate = stardate
	p := filepath.Join(dir, filename)
	if err := g.Save(p); err != nil {
		t.Fatalf("failed to create save %s: %v", filename, err)
	}
	return p
}

func TestSaveBrowserListingAndNavigation(t *testing.T) {
	tempDir := t.TempDir()
	createTestSave(t, tempDir, "GAME1.TRK", 3100.0, engine.SkillNovice)
	createTestSave(t, tempDir, "GAME2.TRK", 3200.0, engine.SkillGood)

	m := New(theme.DefaultTheme(), tempDir)
	if err := m.Refresh(); err != nil {
		t.Fatalf("Refresh failed: %v", err)
	}

	if len(m.saves) != 2 {
		t.Fatalf("expected 2 saves, got %d", len(m.saves))
	}

	if m.cursor != 0 {
		t.Errorf("expected cursor 0, got %d", m.cursor)
	}

	// Move down
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyDown})
	if m.cursor != 1 {
		t.Errorf("expected cursor 1 after KeyDown, got %d", m.cursor)
	}

	// Move down at bound (clamped)
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyDown})
	if m.cursor != 1 {
		t.Errorf("expected cursor clamped at 1, got %d", m.cursor)
	}

	// Move up
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyUp})
	if m.cursor != 0 {
		t.Errorf("expected cursor 0 after KeyUp, got %d", m.cursor)
	}
}

func TestSaveBrowserLoadGameMsg(t *testing.T) {
	tempDir := t.TempDir()
	p := createTestSave(t, tempDir, "LOADME.TRK", 3300.0, engine.SkillExpert)

	m := New(theme.DefaultTheme(), tempDir)
	_ = m.Refresh()

	var emitted tea.Msg
	m, cmd := m.Update(tea.KeyMsg{Type: tea.KeyEnter})
	if cmd != nil {
		emitted = cmd()
	}

	loadMsg, ok := emitted.(LoadGameMsg)
	if !ok {
		t.Fatalf("expected LoadGameMsg, got %T (%v)", emitted, emitted)
	}
	if filepath.Base(loadMsg.Path) != filepath.Base(p) {
		t.Errorf("expected LoadGameMsg.Path %s, got %s", p, loadMsg.Path)
	}
}

func TestSaveBrowserDeleteFlow(t *testing.T) {
	tempDir := t.TempDir()
	p := createTestSave(t, tempDir, "DELME.TRK", 3400.0, engine.SkillFair)

	m := New(theme.DefaultTheme(), tempDir)
	_ = m.Refresh()

	// Press 'd' -> enters deleting mode
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'d'}})
	if !m.deleting {
		t.Errorf("expected deleting mode true after 'd'")
	}
	if !strings.Contains(m.View(), "Delete 'DELME.TRK'?") {
		t.Errorf("expected delete prompt in View(), got %s", m.View())
	}

	// Press 'Esc' cancels delete mode
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyEsc})
	if m.deleting {
		t.Errorf("expected deleting mode false after Esc")
	}

	// Press 'd' again then Enter -> deletes file
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'d'}})
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyEnter})
	if m.deleting {
		t.Errorf("expected deleting mode false after confirmed delete")
	}
	if _, err := os.Stat(p); !os.IsNotExist(err) {
		t.Errorf("expected file %s to be deleted, stat err: %v", p, err)
	}
	if len(m.saves) != 0 {
		t.Errorf("expected 0 saves remaining, got %d", len(m.saves))
	}
}

func TestSaveBrowserCloseMsg(t *testing.T) {
	m := New(theme.DefaultTheme(), t.TempDir())
	_ = m.Refresh()

	m, cmd := m.Update(tea.KeyMsg{Type: tea.KeyEsc})
	if cmd == nil {
		t.Fatalf("expected command on Esc, got nil")
	}
	msg := cmd()
	if _, ok := msg.(CloseBrowserMsg); !ok {
		t.Errorf("expected CloseBrowserMsg on Esc, got %T", msg)
	}
}

func TestSaveBrowserJKNavigation(t *testing.T) {
	tempDir := t.TempDir()
	createTestSave(t, tempDir, "S1.TRK", 3100.0, engine.SkillNovice)
	createTestSave(t, tempDir, "S2.TRK", 3200.0, engine.SkillGood)

	m := New(theme.DefaultTheme(), tempDir)
	_ = m.Refresh()

	// Navigate with 'j'
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'j'}})
	if m.cursor != 1 {
		t.Errorf("expected cursor 1 after 'j', got %d", m.cursor)
	}
	// Bound clamped with 'j'
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'j'}})
	if m.cursor != 1 {
		t.Errorf("expected cursor clamped at 1, got %d", m.cursor)
	}
	// Navigate back with 'k'
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'k'}})
	if m.cursor != 0 {
		t.Errorf("expected cursor 0 after 'k', got %d", m.cursor)
	}
	// Bound clamped with 'k'
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'k'}})
	if m.cursor != 0 {
		t.Errorf("expected cursor clamped at 0, got %d", m.cursor)
	}
}

func TestSaveBrowserEmptyDirectory(t *testing.T) {
	tempDir := t.TempDir()
	m := New(nil, tempDir)
	if err := m.Refresh(); err != nil {
		t.Fatalf("expected nil error on empty dir, got %v", err)
	}
	if len(m.saves) != 0 {
		t.Errorf("expected 0 saves, got %d", len(m.saves))
	}
	view := m.View()
	if !strings.Contains(view, "No saved missions (*.TRK) found.") {
		t.Errorf("expected empty state message in view, got %s", view)
	}

	// Pressing Enter or Delete when empty does nothing
	m, cmd := m.Update(tea.KeyMsg{Type: tea.KeyEnter})
	if cmd != nil {
		t.Errorf("expected nil cmd on Enter with empty saves, got %v", cmd)
	}
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyDelete})
	if m.deleting {
		t.Errorf("expected deleting to remain false when no saves exist")
	}
}

func TestSaveBrowserResetAndConfiguration(t *testing.T) {
	tempDir := t.TempDir()
	createTestSave(t, tempDir, "RESET.TRK", 3500.0, engine.SkillEmeritus)

	m := New(theme.DefaultTheme(), tempDir)
	_ = m.Refresh()

	m.cursor = 1
	m.deleting = true
	m.errorMessage = "some error"

	m.Reset()
	if m.cursor != 0 || m.deleting != false || m.errorMessage != "" {
		t.Errorf("Reset did not restore state: cursor=%d, deleting=%v, err=%q",
			m.cursor, m.deleting, m.errorMessage)
	}

	m.SetTheme(nil)
	if m.theme == nil {
		t.Errorf("expected non-nil theme after SetTheme(nil)")
	}

	m.SetSize(80, 24)
	if m.width != 80 || m.height != 24 {
		t.Errorf("expected size 80x24, got %dx%d", m.width, m.height)
	}
}

func TestSaveBrowserInvalidDirectory(t *testing.T) {
	m := New(theme.DefaultTheme(), "/path/that/does/not/exist/hopefully")
	err := m.Refresh()
	if err == nil {
		t.Errorf("expected error when refreshing non-existent directory")
	}
	if m.errorMessage == "" {
		t.Errorf("expected errorMessage to be populated")
	}
}

func TestSaveBrowserNarrowWidthNoPanic(t *testing.T) {
	m := New(theme.DefaultTheme(), t.TempDir())
	m.SetSize(1, 1)
	defer func() {
		if r := recover(); r != nil {
			t.Fatalf("m.View() panicked on narrow size (1, 1): %v", r)
		}
	}()
	_ = m.View()
}

func TestSaveBrowserDeleteFailurePreservesError(t *testing.T) {
	tempDir := t.TempDir()
	p := createTestSave(t, tempDir, "NODELETE.TRK", 3400.0, engine.SkillFair)

	m := New(theme.DefaultTheme(), tempDir)
	_ = m.Refresh()

	// Delete file out of band before confirming delete in browser so os.Remove fails
	if err := os.Remove(p); err != nil {
		t.Fatalf("failed to remove test file: %v", err)
	}

	// Enter deleting mode
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'d'}})
	if !m.deleting {
		t.Fatalf("expected deleting mode true")
	}

	// Confirm delete -> os.Remove(p) will fail because file does not exist
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyEnter})

	if m.errorMessage == "" {
		t.Errorf("expected non-empty errorMessage after delete failure")
	}
	if !strings.Contains(m.errorMessage, "Delete failed") {
		t.Errorf("expected errorMessage to contain 'Delete failed', got %q", m.errorMessage)
	}
	view := m.View()
	if !strings.Contains(view, "Delete failed") {
		t.Errorf("expected View() to display error message %q, got: %s", m.errorMessage, view)
	}
}

