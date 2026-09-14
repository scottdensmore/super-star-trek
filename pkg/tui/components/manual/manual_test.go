package manual

import (
	"strings"
	"testing"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/x/ansi"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

func TestManual_DimensionsAndLayoutBudget(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th, 66, 18)

	for i := 0; i < len(m.chapters); i++ {
		m.selectedIdx = i
		view := m.View()
		lines := strings.Split(view, "\n")

		if len(lines) != 18 {
			t.Fatalf("chapter %d: expected exactly 18 lines, got %d", i+1, len(lines))
		}

		for lineIdx, line := range lines {
			width := ansi.StringWidth(line)
			if width != 66 {
				t.Errorf("chapter %d line %d: expected width 66, got %d: %q", i+1, lineIdx, width, line)
			}
		}
	}
}

func TestManual_BorderIntegrity(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th, 66, 18)
	view := m.View()
	lines := strings.Split(view, "\n")

	if !strings.HasPrefix(lines[0], "┌") || !strings.HasSuffix(lines[0], "┐") {
		t.Errorf("top border corrupted: %q", lines[0])
	}
	if !strings.HasPrefix(lines[17], "└") || !strings.HasSuffix(lines[17], "┘") {
		t.Errorf("bottom border corrupted: %q", lines[17])
	}

	for i := 1; i <= 16; i++ {
		if !strings.HasPrefix(lines[i], "│") || !strings.HasSuffix(lines[i], "│") {
			t.Errorf("inner row %d missing side borders: %q", i, lines[i])
		}
	}
}

func TestManual_CurriculumChapters(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th, 66, 18)

	if len(m.chapters) != 8 {
		t.Fatalf("expected 8 core chapters, got %d", len(m.chapters))
	}

	expectedIDs := []string{
		"systems", "nav", "combat", "shields",
		"starbases", "tactics", "scoring", "commands",
	}

	for i, expectedID := range expectedIDs {
		if m.chapters[i].ID != expectedID {
			t.Errorf("chapter %d ID mismatch: expected %q, got %q", i+1, expectedID, m.chapters[i].ID)
		}
		if len(m.chapters[i].Lines) == 0 {
			t.Errorf("chapter %q has empty lines buffer", expectedID)
		}
		for lineIdx, line := range m.chapters[i].Lines {
			if w := ansi.StringWidth(line); w > 43 {
				t.Errorf("chapter %q line %d exceeds 43-column reader pane width (got %d): %q", expectedID, lineIdx, w, line)
			}
		}
	}
}

func TestManual_SelectChapter(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th, 66, 18)

	m.SelectChapter("combat")
	if m.selectedIdx != 2 {
		t.Errorf("expected chapter index 2 for 'combat', got %d", m.selectedIdx)
	}

	m.SelectChapter("7")
	if m.selectedIdx != 6 {
		t.Errorf("expected chapter index 6 for '7', got %d", m.selectedIdx)
	}

	m.SelectChapter("unknown")
	if m.selectedIdx != 6 {
		t.Errorf("expected unchanged index on unknown chapter, got %d", m.selectedIdx)
	}
}

func TestManual_SetTheme(t *testing.T) {
	th1 := theme.DefaultTheme()
	m := New(th1, 66, 18)

	th2 := theme.LcarsTheme{}
	m.SetTheme(th2)

	if m.theme.Name() != "lcars" {
		t.Errorf("expected theme name 'lcars', got %q", m.theme.Name())
	}
}

func TestManual_NonStandardDimensionsConstraint(t *testing.T) {
	th := theme.DefaultTheme()
	// Non-standard terminal dimensions (e.g. 80x24) must still render exact 66x18 box
	m := New(th, 80, 24)
	view := m.View()
	lines := strings.Split(view, "\n")

	if len(lines) != 18 {
		t.Fatalf("expected exactly 18 lines, got %d", len(lines))
	}

	for lineIdx, line := range lines {
		width := ansi.StringWidth(line)
		if width != 66 {
			t.Errorf("line %d: expected width 66, got %d: %q", lineIdx, width, line)
		}
	}

	if !strings.HasPrefix(lines[0], "┌") || !strings.HasSuffix(lines[0], "┐") {
		t.Errorf("top border corrupted: %q", lines[0])
	}
	if !strings.HasPrefix(lines[17], "└") || !strings.HasSuffix(lines[17], "┘") {
		t.Errorf("bottom border corrupted: %q", lines[17])
	}

	for i := 1; i <= 16; i++ {
		if !strings.HasPrefix(lines[i], "│") || !strings.HasSuffix(lines[i], "│") {
			t.Errorf("inner row %d missing side borders: %q", i, lines[i])
		}
	}
}

func TestManual_DualPaneFocus(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th, 66, 18)

	if m.focus != FocusChapters {
		t.Fatalf("expected initial focus on FocusChapters")
	}

	// Tab toggles focus to FocusContent
	updated, _ := m.Update(tea.KeyMsg{Type: tea.KeyTab})
	m = updated
	if m.focus != FocusContent {
		t.Errorf("expected focus on FocusContent after Tab, got %v", m.focus)
	}

	// Tab toggles back to FocusChapters
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyTab})
	m = updated
	if m.focus != FocusChapters {
		t.Errorf("expected focus on FocusChapters after second Tab, got %v", m.focus)
	}

	// ShiftTab also toggles focus
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyShiftTab})
	m = updated
	if m.focus != FocusContent {
		t.Errorf("expected focus on FocusContent after ShiftTab, got %v", m.focus)
	}
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyShiftTab})
	m = updated
	if m.focus != FocusChapters {
		t.Errorf("expected focus on FocusChapters after second ShiftTab, got %v", m.focus)
	}

	// Enter or Right switches to FocusContent
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyRight})
	m = updated
	if m.focus != FocusContent {
		t.Errorf("expected focus on FocusContent after KeyRight, got %v", m.focus)
	}

	// Left or Esc returns to FocusChapters
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyLeft})
	m = updated
	if m.focus != FocusChapters {
		t.Errorf("expected focus on FocusChapters after KeyLeft, got %v", m.focus)
	}

	// Enter switches to FocusContent
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyEnter})
	m = updated
	if m.focus != FocusContent {
		t.Errorf("expected focus on FocusContent after KeyEnter, got %v", m.focus)
	}

	// Esc in FocusContent switches back to FocusChapters without closing
	updated, cmd := m.Update(tea.KeyMsg{Type: tea.KeyEsc})
	m = updated
	if cmd != nil {
		t.Errorf("expected nil cmd on Esc when in FocusContent, got %v", cmd())
	}
	if m.focus != FocusChapters {
		t.Errorf("expected focus on FocusChapters after Esc in FocusContent, got %v", m.focus)
	}

	// 'l' switches to FocusContent
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune("l")})
	m = updated
	if m.focus != FocusContent {
		t.Errorf("expected focus on FocusContent after 'l', got %v", m.focus)
	}

	// 'h' returns to FocusChapters
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune("h")})
	m = updated
	if m.focus != FocusChapters {
		t.Errorf("expected focus on FocusChapters after 'h', got %v", m.focus)
	}
}

func TestManual_KeyboardNavigationAndJumps(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th, 66, 18)

	// In FocusChapters, down advances index
	updated, _ := m.Update(tea.KeyMsg{Type: tea.KeyDown})
	m = updated
	if m.selectedIdx != 1 {
		t.Errorf("expected selectedIdx 1 after down, got %d", m.selectedIdx)
	}

	// 'j' also advances index
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune("j")})
	m = updated
	if m.selectedIdx != 2 {
		t.Errorf("expected selectedIdx 2 after 'j', got %d", m.selectedIdx)
	}

	// 'k' moves back
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune("k")})
	m = updated
	if m.selectedIdx != 1 {
		t.Errorf("expected selectedIdx 1 after 'k', got %d", m.selectedIdx)
	}

	// up moves back to 0
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyUp})
	m = updated
	if m.selectedIdx != 0 {
		t.Errorf("expected selectedIdx 0 after up, got %d", m.selectedIdx)
	}

	// up at 0 wraps to last chapter
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyUp})
	m = updated
	if m.selectedIdx != len(m.chapters)-1 {
		t.Errorf("expected selectedIdx wrap to %d, got %d", len(m.chapters)-1, m.selectedIdx)
	}

	// down at last chapter wraps to 0
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyDown})
	m = updated
	if m.selectedIdx != 0 {
		t.Errorf("expected selectedIdx wrap to 0, got %d", m.selectedIdx)
	}

	// Direct numeric key '5'
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune("5")})
	m = updated
	if m.selectedIdx != 4 {
		t.Errorf("expected selectedIdx 4 for key '5', got %d", m.selectedIdx)
	}

	// Direct numeric key '1'
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune("1")})
	m = updated
	if m.selectedIdx != 0 {
		t.Errorf("expected selectedIdx 0 for key '1', got %d", m.selectedIdx)
	}

	// Direct numeric key '8'
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune("8")})
	m = updated
	if m.selectedIdx != 7 {
		t.Errorf("expected selectedIdx 7 for key '8', got %d", m.selectedIdx)
	}
}

func TestManual_ScrollBoundsClamping(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th, 66, 18)
	m.focus = FocusContent

	// Scrolling up at offset 0 remains 0
	updated, _ := m.Update(tea.KeyMsg{Type: tea.KeyUp})
	m = updated
	if m.scrollOffsets[m.selectedIdx] != 0 {
		t.Errorf("expected scroll offset clamped to 0, got %d", m.scrollOffsets[m.selectedIdx])
	}

	// 'k' up at offset 0 remains 0
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune("k")})
	m = updated
	if m.scrollOffsets[m.selectedIdx] != 0 {
		t.Errorf("expected scroll offset clamped to 0 after 'k', got %d", m.scrollOffsets[m.selectedIdx])
	}

	// Scrolling down advances offset
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyDown})
	m = updated
	if m.scrollOffsets[m.selectedIdx] != 1 {
		t.Errorf("expected scroll offset 1, got %d", m.scrollOffsets[m.selectedIdx])
	}

	// 'j' advances offset
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune("j")})
	m = updated
	if m.scrollOffsets[m.selectedIdx] != 2 {
		t.Errorf("expected scroll offset 2 after 'j', got %d", m.scrollOffsets[m.selectedIdx])
	}

	// End jumps to bottom
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyEnd})
	m = updated
	maxOffset := len(m.chapters[m.selectedIdx].Lines) - 14
	if maxOffset < 0 {
		maxOffset = 0
	}
	if m.scrollOffsets[m.selectedIdx] != maxOffset {
		t.Errorf("expected scroll offset clamped to maxOffset %d, got %d", maxOffset, m.scrollOffsets[m.selectedIdx])
	}

	// Scrolling down past bottom remains at maxOffset
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyDown})
	m = updated
	if m.scrollOffsets[m.selectedIdx] != maxOffset {
		t.Errorf("expected scroll offset remaining clamped at maxOffset %d, got %d", maxOffset, m.scrollOffsets[m.selectedIdx])
	}

	// Home jumps to top
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyHome})
	m = updated
	if m.scrollOffsets[m.selectedIdx] != 0 {
		t.Errorf("expected scroll offset 0 after Home, got %d", m.scrollOffsets[m.selectedIdx])
	}

	// 'g' also jumps to top
	m.scrollOffsets[m.selectedIdx] = 5
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune("g")})
	m = updated
	if m.scrollOffsets[m.selectedIdx] != 0 {
		t.Errorf("expected scroll offset 0 after 'g', got %d", m.scrollOffsets[m.selectedIdx])
	}

	// 'G' jumps to bottom
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune("G")})
	m = updated
	if m.scrollOffsets[m.selectedIdx] != maxOffset {
		t.Errorf("expected scroll offset maxOffset %d after 'G', got %d", maxOffset, m.scrollOffsets[m.selectedIdx])
	}

	// PageUp scrolls up 10 lines
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyPgUp})
	m = updated
	expectedAfterPgUp := maxOffset - 10
	if expectedAfterPgUp < 0 {
		expectedAfterPgUp = 0
	}
	if m.scrollOffsets[m.selectedIdx] != expectedAfterPgUp {
		t.Errorf("expected scroll offset %d after PgUp, got %d", expectedAfterPgUp, m.scrollOffsets[m.selectedIdx])
	}

	// 'b' and ctrl+u scroll up 10 lines
	m.scrollOffsets[m.selectedIdx] = 12
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune("b")})
	m = updated
	if m.scrollOffsets[m.selectedIdx] != 2 {
		t.Errorf("expected scroll offset 2 after 'b', got %d", m.scrollOffsets[m.selectedIdx])
	}

	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyCtrlU})
	m = updated
	if m.scrollOffsets[m.selectedIdx] != 0 {
		t.Errorf("expected scroll offset clamped to 0 after Ctrl+U, got %d", m.scrollOffsets[m.selectedIdx])
	}

	// PageDown, space, ctrl+d scroll down 10 lines
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyPgDown})
	m = updated
	expectedPgDown := 10
	if expectedPgDown > maxOffset {
		expectedPgDown = maxOffset
	}
	if m.scrollOffsets[m.selectedIdx] != expectedPgDown {
		t.Errorf("expected scroll offset %d after PgDown, got %d", expectedPgDown, m.scrollOffsets[m.selectedIdx])
	}

	m.scrollOffsets[m.selectedIdx] = 0
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeySpace})
	m = updated
	if m.scrollOffsets[m.selectedIdx] != expectedPgDown {
		t.Errorf("expected scroll offset %d after Space, got %d", expectedPgDown, m.scrollOffsets[m.selectedIdx])
	}

	m.scrollOffsets[m.selectedIdx] = 0
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyCtrlD})
	m = updated
	if m.scrollOffsets[m.selectedIdx] != expectedPgDown {
		t.Errorf("expected scroll offset %d after Ctrl+D, got %d", expectedPgDown, m.scrollOffsets[m.selectedIdx])
	}
}

func TestManual_DismissalKeys(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th, 66, 18)

	for _, k := range []string{"esc", "q", "Q", "F1", "?"} {
		var keyMsg tea.KeyMsg
		switch k {
		case "esc":
			keyMsg = tea.KeyMsg{Type: tea.KeyEsc}
		case "F1":
			keyMsg = tea.KeyMsg{Type: tea.KeyF1}
		default:
			keyMsg = tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune(k)}
		}

		_, cmd := m.Update(keyMsg)
		if cmd == nil {
			t.Fatalf("expected dismissal command on key %s", k)
		}
		if _, ok := cmd().(CloseModalMsg); !ok {
			t.Errorf("expected CloseModalMsg on key %s, got %T", k, cmd())
		}
	}

	// In FocusContent, q, Q, F1, ? still dismiss
	m.focus = FocusContent
	for _, k := range []string{"q", "Q", "F1", "?"} {
		var keyMsg tea.KeyMsg
		switch k {
		case "F1":
			keyMsg = tea.KeyMsg{Type: tea.KeyF1}
		default:
			keyMsg = tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune(k)}
		}

		_, cmd := m.Update(keyMsg)
		if cmd == nil {
			t.Fatalf("expected dismissal command on key %s in FocusContent", k)
		}
		if _, ok := cmd().(CloseModalMsg); !ok {
			t.Errorf("expected CloseModalMsg on key %s in FocusContent, got %T", k, cmd())
		}
	}
}


