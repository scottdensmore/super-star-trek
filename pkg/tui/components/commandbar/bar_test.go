package commandbar

import (
	"strings"
	"testing"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

func TestNewAndAlias(t *testing.T) {
	th := theme.DefaultTheme()
	m1 := New(th)
	m2 := NewCommandBar(th)

	if m1.Prompt != "COMMAND> " {
		t.Fatalf("expected prompt 'COMMAND> ', got %q", m1.Prompt)
	}
	if m2.Prompt != "COMMAND> " {
		t.Fatalf("expected prompt 'COMMAND> ', got %q", m2.Prompt)
	}
	if len(m1.Messages()) != 0 {
		t.Fatalf("expected empty messages, got %v", m1.Messages())
	}
	if m1.Theme().Name() != "modern" {
		t.Fatalf("expected modern theme, got %s", m1.Theme().Name())
	}

	// Nil theme defaults safely
	mNil := New(nil)
	if mNil.Theme().Name() != "modern" {
		t.Fatalf("nil theme should default to modern, got %s", mNil.Theme().Name())
	}
}

func TestAddMessageAndScrollingBuffer(t *testing.T) {
	m := New(theme.DefaultTheme())

	m.AddMessage("msg 1")
	m.AddMessage("msg 2")
	m.AddMessage("msg 3")
	m.AddMessage("msg 4")

	msgs := m.Messages()
	if len(msgs) != 4 {
		t.Fatalf("expected 4 messages, got %d", len(msgs))
	}
	if msgs[0] != "msg 1" || msgs[3] != "msg 4" {
		t.Fatalf("unexpected message ordering: %v", msgs)
	}

	// 5th message scrolls off msg 1
	m.AddMessage("msg 5")
	msgs = m.Messages()
	if len(msgs) != 4 {
		t.Fatalf("expected 4 messages after 5th added, got %d", len(msgs))
	}
	if msgs[0] != "msg 2" || msgs[3] != "msg 5" {
		t.Fatalf("expected msg 1 to scroll off, got %v", msgs)
	}

	// Multiline message splits and scrolls
	m.AddMessage("msg 6\nmsg 7")
	msgs = m.Messages()
	if len(msgs) != 4 {
		t.Fatalf("expected 4 messages after multiline, got %d", len(msgs))
	}
	if msgs[2] != "msg 6" || msgs[3] != "msg 7" {
		t.Fatalf("unexpected messages after multiline split: %v", msgs)
	}

	// ClearMessages
	m.ClearMessages()
	if len(m.Messages()) != 0 {
		t.Fatalf("expected 0 messages after ClearMessages, got %d", len(m.Messages()))
	}
}

func TestSetTheme(t *testing.T) {
	m := New(theme.DefaultTheme())
	lcars := theme.GetTheme("lcars")

	m.SetTheme(lcars)
	if m.Theme().Name() != "lcars" {
		t.Fatalf("expected lcars theme, got %s", m.Theme().Name())
	}

	// Nil theme defaults safely
	m.SetTheme(nil)
	if m.Theme().Name() != "modern" {
		t.Fatalf("expected default modern theme on nil SetTheme, got %s", m.Theme().Name())
	}
}

func TestFocusAndBlur(t *testing.T) {
	m := New(theme.DefaultTheme())
	cmd := m.Focus()
	if !m.Focused() {
		t.Fatalf("expected model to be focused")
	}
	_ = cmd

	m.Blur()
	if m.Focused() {
		t.Fatalf("expected model to be blurred")
	}
}

func TestUpdateTyping(t *testing.T) {
	m := New(theme.DefaultTheme())
	m.Focus()

	// Type characters 'n', 'a', 'v'
	var cmd tea.Cmd
	m, cmd = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'n'}})
	_ = cmd
	m, cmd = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'a'}})
	_ = cmd
	m, cmd = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'v'}})
	_ = cmd

	if m.Value() != "nav" {
		t.Fatalf("expected textinput value 'nav', got %q", m.Value())
	}
}

func TestUpdateEnterWithText(t *testing.T) {
	m := New(theme.DefaultTheme())
	m.SetValue("nav 1.5 2")

	m, cmd := m.Update(tea.KeyMsg{Type: tea.KeyEnter})
	if cmd == nil {
		t.Fatalf("expected non-nil tea.Cmd on Enter with text")
	}

	// Input should be reset
	if m.Value() != "" {
		t.Fatalf("expected textinput to be reset after Enter, got %q", m.Value())
	}

	// Executing cmd returns CommandSubmittedMsg
	msg := cmd()
	subMsg, ok := msg.(CommandSubmittedMsg)
	if !ok {
		t.Fatalf("expected CommandSubmittedMsg, got %T: %+v", msg, msg)
	}
	if subMsg.Text != "nav 1.5 2" {
		t.Fatalf("expected CommandSubmittedMsg.Text to be 'nav 1.5 2', got %q", subMsg.Text)
	}
}

func TestUpdateEnterEmpty(t *testing.T) {
	m := New(theme.DefaultTheme())
	m.SetValue("")

	m, cmd := m.Update(tea.KeyMsg{Type: tea.KeyEnter})
	if cmd != nil {
		t.Fatalf("expected nil cmd on Enter with empty text, got %v", cmd)
	}

	// Whitespace only
	m.SetValue("   ")
	m, cmd = m.Update(tea.KeyMsg{Type: tea.KeyEnter})
	if cmd != nil {
		t.Fatalf("expected nil cmd on Enter with whitespace-only text, got %v", cmd)
	}
}

func TestView(t *testing.T) {
	m := New(theme.DefaultTheme())
	m.AddMessage("Enterprise systems online.")
	m.AddMessage("Sensors detect 3 Klingons.")

	view := m.View()

	if !strings.Contains(view, "COMMAND> ") {
		t.Fatalf("expected view to contain 'COMMAND> ', got:\n%s", view)
	}
	if !strings.Contains(view, "Enterprise systems online.") {
		t.Fatalf("expected view to contain first log line, got:\n%s", view)
	}
	if !strings.Contains(view, "Sensors detect 3 Klingons.") {
		t.Fatalf("expected view to contain second log line, got:\n%s", view)
	}

	// Set width and verify render
	m.SetWidth(80)
	view80 := m.View()
	if !strings.Contains(view80, "COMMAND> ") {
		t.Fatalf("expected width 80 view to contain prompt, got:\n%s", view80)
	}
}
