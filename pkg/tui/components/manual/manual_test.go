package manual

import (
	"strings"
	"testing"

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

