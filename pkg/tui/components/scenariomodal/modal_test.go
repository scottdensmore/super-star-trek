package scenariomodal

import (
	"strings"
	"testing"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

func TestScenarioModal_NavigationAndSelection(t *testing.T) {
	th := theme.GetTheme("modern")
	m := NewModel(th)
	m.SetDimensions(80, 24)

	// Initial selection is index 0 (Kobayashi Maru)
	if m.SelectedScenario().ID != engine.ScenarioKobayashiMaru {
		t.Errorf("expected first selection %v, got %v", engine.ScenarioKobayashiMaru, m.SelectedScenario().ID)
	}

	// Move down
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyDown})
	if m.SelectedScenario().ID != engine.ScenarioMutaraNebula {
		t.Errorf("expected second selection %v, got %v", engine.ScenarioMutaraNebula, m.SelectedScenario().ID)
	}

	// Press Enter to trigger launch message
	_, cmd := m.Update(tea.KeyMsg{Type: tea.KeyEnter})
	if cmd == nil {
		t.Fatalf("expected launch command on Enter, got nil")
	}
	msg := cmd()
	launchMsg, ok := msg.(MsgLaunchScenario)
	if !ok || launchMsg.ScenarioID != engine.ScenarioMutaraNebula {
		t.Errorf("expected MsgLaunchScenario for %v, got %+v", engine.ScenarioMutaraNebula, msg)
	}
}

func TestScenarioModal_NavigationKeys(t *testing.T) {
	th := theme.GetTheme("modern")
	m := NewModel(th)

	// Test wrapping Up from 0 to last item
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyUp})
	if m.SelectedScenario().ID != engine.ScenarioStarbaseSiege {
		t.Errorf("expected wrapped selection %v, got %v", engine.ScenarioStarbaseSiege, m.SelectedScenario().ID)
	}

	// Test wrapping Down from last to 0
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyDown})
	if m.SelectedScenario().ID != engine.ScenarioKobayashiMaru {
		t.Errorf("expected wrapped selection %v, got %v", engine.ScenarioKobayashiMaru, m.SelectedScenario().ID)
	}

	// Test 'j' key (down)
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'j'}})
	if m.SelectedScenario().ID != engine.ScenarioMutaraNebula {
		t.Errorf("expected selection after 'j' %v, got %v", engine.ScenarioMutaraNebula, m.SelectedScenario().ID)
	}

	// Test 'k' key (up)
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'k'}})
	if m.SelectedScenario().ID != engine.ScenarioKobayashiMaru {
		t.Errorf("expected selection after 'k' %v, got %v", engine.ScenarioKobayashiMaru, m.SelectedScenario().ID)
	}
}

func TestScenarioModal_DismissKeys(t *testing.T) {
	th := theme.GetTheme("modern")
	m := NewModel(th)

	// Esc dismiss
	_, cmd := m.Update(tea.KeyMsg{Type: tea.KeyEsc})
	if cmd == nil {
		t.Fatalf("expected close command on Esc, got nil")
	}
	if _, ok := cmd().(MsgCloseScenarioModal); !ok {
		t.Errorf("expected MsgCloseScenarioModal on Esc, got %T", cmd())
	}

	// 'q' dismiss
	_, cmd = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'q'}})
	if cmd == nil {
		t.Fatalf("expected close command on 'q', got nil")
	}
	if _, ok := cmd().(MsgCloseScenarioModal); !ok {
		t.Errorf("expected MsgCloseScenarioModal on 'q', got %T", cmd())
	}

	// 'Q' dismiss
	_, cmd = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'Q'}})
	if cmd == nil {
		t.Fatalf("expected close command on 'Q', got nil")
	}
	if _, ok := cmd().(MsgCloseScenarioModal); !ok {
		t.Errorf("expected MsgCloseScenarioModal on 'Q', got %T", cmd())
	}
}

func TestScenarioModal_ViewRendering(t *testing.T) {
	th := theme.GetTheme("modern")
	m := NewModel(th)

	v := m.View()

	// Verify catalog items and badges rendered
	if !strings.Contains(v, "Kobayashi Maru") {
		t.Errorf("expected View to contain 'Kobayashi Maru', got:\n%s", v)
	}
	if !strings.Contains(v, "Mutara Nebula") {
		t.Errorf("expected View to contain 'Mutara Nebula', got:\n%s", v)
	}
	if !strings.Contains(v, "[EXTREME]") {
		t.Errorf("expected View to contain '[EXTREME]', got:\n%s", v)
	}
	if !strings.Contains(v, "[HARD]") {
		t.Errorf("expected View to contain '[HARD]', got:\n%s", v)
	}
	if !strings.Contains(v, "[CHALLENGE]") {
		t.Errorf("expected View to contain '[CHALLENGE]', got:\n%s", v)
	}

	// Verify briefing, constraints, and leaderboard for Kobayashi Maru
	if !strings.Contains(v, "BRIEFING") {
		t.Errorf("expected View to contain 'BRIEFING'")
	}
	if !strings.Contains(v, "CONSTRAINTS") {
		t.Errorf("expected View to contain 'CONSTRAINTS'")
	}
	if !strings.Contains(v, "James T. Kirk") {
		t.Errorf("expected View to contain top record 'James T. Kirk', got:\n%s", v)
	}

	// Switch to Mutara Nebula
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyDown})
	v2 := m.View()
	if !strings.Contains(v2, "MUTARA NEBULA") {
		t.Errorf("expected View to contain 'MUTARA NEBULA', got:\n%s", v2)
	}
	if !strings.Contains(v2, "Admiral James T. Kirk") {
		t.Errorf("expected View to contain top record 'Admiral James T. Kirk', got:\n%s", v2)
	}

	// Switch to Starbase Siege
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyDown})
	v3 := m.View()
	if !strings.Contains(v3, "STARBASE") {
		t.Errorf("expected View to contain 'STARBASE', got:\n%s", v3)
	}
	if !strings.Contains(v3, "Hikaru Sulu") {
		t.Errorf("expected View to contain top record 'Hikaru Sulu', got:\n%s", v3)
	}
}

func TestScenarioModal_ThemeAndDimensions(t *testing.T) {
	th := theme.GetTheme("lcars")
	m := New(th, 72, 18)
	m.SetTheme(theme.GetTheme("crt"))
	m.SetDimensions(80, 24)
	m.SetSize(80, 24)

	v := m.View()
	lines := strings.Split(strings.TrimRight(v, "\n"), "\n")
	if len(lines) != 18 {
		t.Errorf("expected 18 lines rendered, got %d", len(lines))
	}
}

func TestScenarioModal_EmptyScenarios(t *testing.T) {
	m := Model{}
	if m.SelectedScenario() != nil {
		t.Errorf("expected nil for empty scenarios, got %v", m.SelectedScenario())
	}
	_, cmd := m.Update(tea.KeyMsg{Type: tea.KeyEnter})
	if cmd != nil {
		t.Errorf("expected nil cmd on Enter for empty scenarios, got %v", cmd)
	}
	v := m.View()
	if v == "" {
		t.Errorf("expected non-empty View even if empty scenarios")
	}
}
