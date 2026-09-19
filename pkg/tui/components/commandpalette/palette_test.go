package commandpalette

import (
	"strings"
	"testing"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

func TestCommandPalette_Catalog(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th, 56, 16)

	items := m.Items()
	if len(items) != 22 {
		t.Fatalf("expected 22 catalog items, got %d", len(items))
	}

	expectedCatalog := []struct {
		title         string
		prefix        string
		parameterized bool
	}{
		{"TOR", "tor ", true},
		{"PHA", "pha ", true},
		{"SHE", "she ", true},
		{"TARGET", "target", false},
		{"NAV", "nav ", true},
		{"DOC", "doc", false},
		{"SRSCAN", "srscan", false},
		{"LRSCAN", "lrscan", false},
		{"STATUS", "status", false},
		{"DAM", "dam", false},
		{"CHART", "chart", false},
		{"SAVES", "saves", false},
		{"SCENARIOS", "scenarios", false},
		{"THEME: Modern", "theme modern", false},
		{"THEME: LCARS", "theme lcars", false},
		{"THEME: CRT", "theme crt", false},
		{"THEME MODE AUTO", "theme mode auto", false},
		{"THEME MODE DARK", "theme mode dark", false},
		{"THEME MODE LIGHT", "theme mode light", false},
		{"HELP", "help", false},
		{"HELP NAV", "help nav", false},
		{"QUIT", "quit", false},
	}

	itemMap := make(map[string]PaletteItem)
	for _, it := range items {
		itemMap[it.Title()] = it
	}

	for _, exp := range expectedCatalog {
		it, ok := itemMap[exp.title]
		if !ok {
			t.Errorf("expected command %q in catalog, but it was missing", exp.title)
			continue
		}
		if it.Prefix() != exp.prefix {
			t.Errorf("command %q: expected prefix %q, got %q", exp.title, exp.prefix, it.Prefix())
		}
		if it.Parameterized() != exp.parameterized {
			t.Errorf("command %q: expected parameterized=%v, got %v", exp.title, exp.parameterized, it.Parameterized())
		}
		if it.Description() == "" {
			t.Errorf("command %q: expected non-empty description", exp.title)
		}
		if it.FilterValue() == "" {
			t.Errorf("command %q: expected non-empty filter value", exp.title)
		}
	}

	chartItem := itemMap["CHART"]
	expectedChartDesc := "Interactive 8x8 galactic star chart and warp planner (Ctrl+M)"
	if chartItem.Description() != expectedChartDesc {
		t.Errorf("expected CHART description %q, got %q", expectedChartDesc, chartItem.Description())
	}
}

func TestCommandPalette_FuzzyFilter(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th, 56, 16)

	// Type "pha"
	for _, r := range "pha" {
		var cmd tea.Cmd
		m, cmd = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{r}})
		if cmd != nil {
			msg := cmd()
			m, _ = m.Update(msg)
		}
	}

	if m.FilterValue() != "pha" {
		t.Fatalf("expected FilterValue %q, got %q", "pha", m.FilterValue())
	}

	visible := m.VisibleItems()
	if len(visible) == 0 {
		t.Fatalf("expected matches for query 'pha', got 0")
	}

	selected := m.SelectedItem()
	if selected == nil {
		t.Fatalf("expected selected item for query 'pha', got nil")
	}
	if selected.Title() != "PHA" {
		t.Errorf("expected top match for 'pha' to be 'PHA', got %q", selected.Title())
	}

	// Reset and type "nav"
	m.Reset()
	if m.FilterValue() != "" {
		t.Fatalf("expected FilterValue empty after Reset, got %q", m.FilterValue())
	}
	if len(m.VisibleItems()) != 22 {
		t.Fatalf("expected 22 items after Reset, got %d", len(m.VisibleItems()))
	}

	for _, r := range "nav" {
		var cmd tea.Cmd
		m, cmd = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{r}})
		if cmd != nil {
			msg := cmd()
			m, _ = m.Update(msg)
		}
	}

	visible = m.VisibleItems()
	if len(visible) == 0 {
		t.Fatalf("expected matches for query 'nav', got 0")
	}

	selected = m.SelectedItem()
	if selected == nil {
		t.Fatalf("expected selected item for query 'nav', got nil")
	}
	if selected.Title() != "NAV" {
		t.Errorf("expected top match for 'nav' to be 'NAV', got %q", selected.Title())
	}

	// Backspace test
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyBackspace})
	if m.FilterValue() != "na" {
		t.Errorf("expected FilterValue 'na' after backspace, got %q", m.FilterValue())
	}
}

func TestCommandPalette_Selection(t *testing.T) {
	th := theme.DefaultTheme()

	// Test 1: Parameterized command selection ("pha")
	m := New(th, 56, 16)
	for _, r := range "pha" {
		m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{r}})
	}

	var cmd tea.Cmd
	m, cmd = m.Update(tea.KeyMsg{Type: tea.KeyEnter})
	if cmd == nil {
		t.Fatalf("expected tea.Cmd on Enter, got nil")
	}

	msg := cmd()
	selMsg, ok := msg.(CommandSelectedMsg)
	if !ok {
		t.Fatalf("expected CommandSelectedMsg, got %T", msg)
	}
	if selMsg.CommandPrefix != "pha " {
		t.Errorf("expected CommandPrefix 'pha ', got %q", selMsg.CommandPrefix)
	}
	if !selMsg.Parameterized {
		t.Errorf("expected Parameterized=true for 'pha ', got false")
	}

	// Test 2: Instant command selection ("doc")
	m.Reset()
	for _, r := range "doc" {
		m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{r}})
	}

	m, cmd = m.Update(tea.KeyMsg{Type: tea.KeyEnter})
	if cmd == nil {
		t.Fatalf("expected tea.Cmd on Enter, got nil")
	}

	msg = cmd()
	selMsg, ok = msg.(CommandSelectedMsg)
	if !ok {
		t.Fatalf("expected CommandSelectedMsg, got %T", msg)
	}
	if selMsg.CommandPrefix != "doc" {
		t.Errorf("expected CommandPrefix 'doc', got %q", selMsg.CommandPrefix)
	}
	if selMsg.Parameterized {
		t.Errorf("expected Parameterized=false for 'doc', got true")
	}
}

func TestCommandPalette_Close(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th, 56, 16)

	m, cmd := m.Update(tea.KeyMsg{Type: tea.KeyEsc})
	if cmd == nil {
		t.Fatalf("expected tea.Cmd on Esc, got nil")
	}

	msg := cmd()
	if _, ok := msg.(ClosePaletteMsg); !ok {
		t.Fatalf("expected ClosePaletteMsg, got %T", msg)
	}
}

func TestCommandPalette_Navigation(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th, 56, 16)

	initial := m.SelectedItem()
	if initial == nil || initial.Title() != "TOR" {
		t.Fatalf("expected initial selected item 'TOR', got %v", initial)
	}

	// Press Down arrow
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyDown})
	next := m.SelectedItem()
	if next == nil || next.Title() != "PHA" {
		t.Fatalf("expected selected item after Down to be 'PHA', got %v", next)
	}

	// Press Up arrow
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyUp})
	prev := m.SelectedItem()
	if prev == nil || prev.Title() != "TOR" {
		t.Fatalf("expected selected item after Up to be 'TOR', got %v", prev)
	}
}

func TestCommandPalette_ViewDimensionsAndLayout(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th, 56, 16)

	view := m.View()

	// Verify required text elements in View
	if !strings.Contains(view, "COMMAND PALETTE") {
		t.Errorf("expected View to contain 'COMMAND PALETTE', got:\n%s", view)
	}

	// Verify outer dimensions: 56 columns wide x 16 rows tall
	lines := strings.Split(strings.TrimRight(view, "\n"), "\n")
	if len(lines) != 16 {
		t.Errorf("expected View height 16 rows, got %d rows:\n%s", len(lines), view)
	}
	for i, line := range lines {
		w := lipgloss.Width(line)
		if w != 56 {
			t.Errorf("expected line %d width 56, got %d: %q", i, w, line)
		}
	}
}

func TestCommandPalette_ThemeAndSize(t *testing.T) {
	m := New(nil, 0, 0) // Should default to Modern theme and 56x16
	if m.SelectedItem() == nil {
		t.Fatalf("expected default model to have items")
	}

	m.SetTheme(theme.LcarsTheme{})
	m.SetSize(60, 20)

	view := m.View()
	lines := strings.Split(strings.TrimRight(view, "\n"), "\n")
	if len(lines) != 20 {
		t.Errorf("expected View height 20 rows after resize, got %d rows", len(lines))
	}
	for i, line := range lines {
		w := lipgloss.Width(line)
		if w != 60 {
			t.Errorf("expected line %d width 60, got %d: %q", i, w, line)
		}
	}

	m.SetTheme(theme.CrtTheme{})
	crtView := m.View()
	if crtView == "" {
		t.Errorf("expected non-empty View with CRT theme")
	}
}

func TestCommandPalette_NoMatch(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th, 56, 16)

	for _, r := range "zzzznotfound" {
		m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{r}})
	}

	if len(m.VisibleItems()) != 0 {
		t.Errorf("expected 0 visible items for impossible query, got %d", len(m.VisibleItems()))
	}
	if m.SelectedItem() != nil {
		t.Errorf("expected nil selected item, got %v", m.SelectedItem())
	}

	// Pressing Enter when no match should not emit selection
	_, cmd := m.Update(tea.KeyMsg{Type: tea.KeyEnter})
	if cmd != nil {
		t.Errorf("expected nil cmd on Enter with no matches, got %v", cmd)
	}
}

func TestCommandPalette_RuneSafeBackspaceAndWindowSize(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th, 56, 16)

	// Type multi-byte runes: emoji, non-ASCII latin, CJK
	input := []rune{'🚀', '🎯', 'é', '漢', '字'}
	for _, r := range input {
		m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{r}})
	}

	expectedQuery := "🚀🎯é漢字"
	if m.FilterValue() != expectedQuery {
		t.Fatalf("expected query %q, got %q", expectedQuery, m.FilterValue())
	}

	// Backspace once: should safely remove '字'
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyBackspace})
	if m.FilterValue() != "🚀🎯é漢" {
		t.Fatalf("expected query %q after backspace, got %q", "🚀🎯é漢", m.FilterValue())
	}

	// Backspace again: should safely remove '漢'
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyBackspace})
	if m.FilterValue() != "🚀🎯é" {
		t.Fatalf("expected query %q after backspace, got %q", "🚀🎯é", m.FilterValue())
	}

	// Backspace again: should safely remove 'é'
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyBackspace})
	if m.FilterValue() != "🚀🎯" {
		t.Fatalf("expected query %q after backspace, got %q", "🚀🎯", m.FilterValue())
	}

	// Backspace again: should safely remove '🎯'
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyBackspace})
	if m.FilterValue() != "🚀" {
		t.Fatalf("expected query %q after backspace, got %q", "🚀", m.FilterValue())
	}

	// Backspace again: should safely remove '🚀'
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyBackspace})
	if m.FilterValue() != "" {
		t.Fatalf("expected query empty after backspace, got %q", m.FilterValue())
	}

	// Backspace on empty query should be a safe no-op
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyBackspace})
	if m.FilterValue() != "" {
		t.Fatalf("expected query empty after backspace on empty, got %q", m.FilterValue())
	}

	// Verify WindowSizeMsg is safely ignored to maintain fixed dialog dimensions
	m, _ = m.Update(tea.WindowSizeMsg{Width: 120, Height: 40})
	view := m.View()
	lines := strings.Split(strings.TrimRight(view, "\n"), "\n")
	if len(lines) != 16 {
		t.Fatalf("expected palette height to remain 16 after WindowSizeMsg, got %d", len(lines))
	}
	for i, line := range lines {
		w := lipgloss.Width(line)
		if w != 56 {
			t.Fatalf("expected line %d width to remain 56, got %d", i, w)
		}
	}
}
