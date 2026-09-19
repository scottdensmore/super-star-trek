package commandbar

import (
	"strings"
	"testing"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/muesli/termenv"
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

func TestAddMessage_TrailingNewlines(t *testing.T) {
	m := New(theme.DefaultTheme())

	// Add single line with trailing newline
	m.AddMessage("Single line with newline\n")
	// Add message with CRLF
	m.AddMessage("Line with CRLF\r\n")
	// Add multiline message with trailing newline
	m.AddMessage("First of multi\nSecond of multi\n")

	msgs := m.Messages()
	if len(msgs) != 4 {
		t.Fatalf("expected 4 messages, got %d: %v", len(msgs), msgs)
	}
	expected := []string{
		"Single line with newline",
		"Line with CRLF",
		"First of multi",
		"Second of multi",
	}
	for i, exp := range expected {
		if msgs[i] != exp {
			t.Errorf("msg[%d]: expected %q, got %q", i, exp, msgs[i])
		}
	}

	for _, msg := range msgs {
		if msg == "" {
			t.Errorf("expected no empty lines in messages buffer, but found an empty entry")
		}
	}

	// Empty string or only newlines should not add blank lines
	m.AddMessage("\n\n\r\n")
	if len(m.Messages()) != 4 {
		t.Errorf("adding only newlines should not alter messages buffer, got %d messages", len(m.Messages()))
	}
}

func TestCommandBarHistory(t *testing.T) {
	th := theme.DefaultTheme()
	cb := New(th)

	// Type and submit "nav 1 2"
	cb.SetValue("nav 1 2")
	cb, _ = cb.Update(tea.KeyMsg{Type: tea.KeyEnter})

	// Type and submit "tor 3 4"
	cb.SetValue("tor 3 4")
	cb, _ = cb.Update(tea.KeyMsg{Type: tea.KeyEnter})

	// Start typing draft "ph"
	cb.SetValue("ph")

	// Press Up -> recall "tor 3 4"
	cb, _ = cb.Update(tea.KeyMsg{Type: tea.KeyUp})
	if cb.Value() != "tor 3 4" {
		t.Errorf("expected recalled history 'tor 3 4', got %q", cb.Value())
	}

	// Press Up again -> recall "nav 1 2"
	cb, _ = cb.Update(tea.KeyMsg{Type: tea.KeyUp})
	if cb.Value() != "nav 1 2" {
		t.Errorf("expected recalled history 'nav 1 2', got %q", cb.Value())
	}

	// Press Up again at the oldest entry -> stays at "nav 1 2"
	cb, _ = cb.Update(tea.KeyMsg{Type: tea.KeyUp})
	if cb.Value() != "nav 1 2" {
		t.Errorf("expected oldest history to remain 'nav 1 2', got %q", cb.Value())
	}

	// Press Down -> back to "tor 3 4"
	cb, _ = cb.Update(tea.KeyMsg{Type: tea.KeyDown})
	if cb.Value() != "tor 3 4" {
		t.Errorf("expected recalled history 'tor 3 4', got %q", cb.Value())
	}

	// Press Down again -> restore draft "ph"
	cb, _ = cb.Update(tea.KeyMsg{Type: tea.KeyDown})
	if cb.Value() != "ph" {
		t.Errorf("expected restored draft 'ph', got %q", cb.Value())
	}

	// Press Down again when already at draft -> stays at "ph"
	cb, _ = cb.Update(tea.KeyMsg{Type: tea.KeyDown})
	if cb.Value() != "ph" {
		t.Errorf("expected draft to remain 'ph', got %q", cb.Value())
	}
}

func TestCommandBarHistoryDeduplication(t *testing.T) {
	th := theme.DefaultTheme()
	cb := New(th)

	// Submitting duplicate consecutive commands should not duplicate in history
	cb.SetValue("status")
	cb, _ = cb.Update(tea.KeyMsg{Type: tea.KeyEnter})

	cb.SetValue("status")
	cb, _ = cb.Update(tea.KeyMsg{Type: tea.KeyEnter})

	cb.SetValue("nav 1 2")
	cb, _ = cb.Update(tea.KeyMsg{Type: tea.KeyEnter})

	// Up -> "nav 1 2"
	cb, _ = cb.Update(tea.KeyMsg{Type: tea.KeyUp})
	if cb.Value() != "nav 1 2" {
		t.Errorf("expected 'nav 1 2', got %q", cb.Value())
	}

	// Up -> "status"
	cb, _ = cb.Update(tea.KeyMsg{Type: tea.KeyUp})
	if cb.Value() != "status" {
		t.Errorf("expected 'status', got %q", cb.Value())
	}

	// Up again -> should stay "status" because duplicate wasn't added
	cb, _ = cb.Update(tea.KeyMsg{Type: tea.KeyUp})
	if cb.Value() != "status" {
		t.Errorf("expected 'status' (no duplicate), got %q", cb.Value())
	}
}

func TestCommandBarHistoryEmpty(t *testing.T) {
	th := theme.DefaultTheme()
	cb := New(th)

	// With empty history, Up / Down does nothing
	cb.SetValue("draft")
	cb, _ = cb.Update(tea.KeyMsg{Type: tea.KeyUp})
	if cb.Value() != "draft" {
		t.Errorf("expected 'draft' on Up with empty history, got %q", cb.Value())
	}
	cb, _ = cb.Update(tea.KeyMsg{Type: tea.KeyDown})
	if cb.Value() != "draft" {
		t.Errorf("expected 'draft' on Down with empty history, got %q", cb.Value())
	}
}

func TestCommandBarTabCompletion(t *testing.T) {
	th := theme.DefaultTheme()
	cb := New(th)

	// Single match: "ph" -> "pha "
	cb.SetValue("ph")
	cb, _ = cb.Update(tea.KeyMsg{Type: tea.KeyTab})
	if cb.Value() != "pha " {
		t.Errorf("expected tab completion 'pha ', got %q", cb.Value())
	}

	// Multiple matches: "s" -> ["saves", "she ", "srscan", "status"]
	cb.SetValue("s")
	cb, _ = cb.Update(tea.KeyMsg{Type: tea.KeyTab})
	firstMatch := cb.Value()
	if firstMatch != "saves" {
		t.Errorf("expected first tab candidate 'saves', got %q", firstMatch)
	}

	// Tab cycle 2
	cb, _ = cb.Update(tea.KeyMsg{Type: tea.KeyTab})
	if cb.Value() != "she " {
		t.Errorf("expected second tab candidate 'she ', got %q", cb.Value())
	}

	// Tab cycle 3
	cb, _ = cb.Update(tea.KeyMsg{Type: tea.KeyTab})
	if cb.Value() != "srscan" {
		t.Errorf("expected third tab candidate 'srscan', got %q", cb.Value())
	}

	// Tab cycle 4
	cb, _ = cb.Update(tea.KeyMsg{Type: tea.KeyTab})
	if cb.Value() != "status" {
		t.Errorf("expected fourth tab candidate 'status', got %q", cb.Value())
	}

	// Tab cycle wraps back to 1
	cb, _ = cb.Update(tea.KeyMsg{Type: tea.KeyTab})
	if cb.Value() != "saves" {
		t.Errorf("expected wrapped tab candidate 'saves', got %q", cb.Value())
	}

	// Typing key resets tab cycle
	cb, _ = cb.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'x'}})
	cb.SetValue("doc")
	cb, _ = cb.Update(tea.KeyMsg{Type: tea.KeyTab})
	if cb.Value() != "doc" {
		t.Errorf("expected instant match 'doc', got %q", cb.Value())
	}

	// Non-matching tab completion preserves input
	cb.SetValue("xyz")
	cb, _ = cb.Update(tea.KeyMsg{Type: tea.KeyTab})
	if cb.Value() != "xyz" {
		t.Errorf("expected 'xyz' to remain unchanged on no match, got %q", cb.Value())
	}

	// Empty input tab completion does nothing
	cb.SetValue("")
	cb, _ = cb.Update(tea.KeyMsg{Type: tea.KeyTab})
	if cb.Value() != "" {
		t.Errorf("expected empty string to remain on tab with empty input, got %q", cb.Value())
	}
}

func TestCommandBarVisualBell(t *testing.T) {
	lipgloss.SetColorProfile(termenv.TrueColor)
	defer lipgloss.SetColorProfile(termenv.Ascii)

	th := theme.DefaultTheme()
	cb := New(th)
	cb.SetWidth(80)

	// Initially visual bell is false
	if cb.VisualBell() {
		t.Errorf("expected visual bell initially false")
	}

	normalView := cb.View()

	// Activate visual bell
	cb.SetVisualBell(true)
	if !cb.VisualBell() {
		t.Errorf("expected visual bell true after SetVisualBell(true)")
	}

	bellView := cb.View()

	// Views should differ in border styling/ANSI escape codes
	if bellView == normalView {
		t.Errorf("expected visual bell view to differ from normal view due to pulsed accent border")
	}

	// Deactivate visual bell
	cb.SetVisualBell(false)
	if cb.VisualBell() {
		t.Errorf("expected visual bell false after SetVisualBell(false)")
	}

	restoredView := cb.View()
	if restoredView != normalView {
		t.Errorf("expected restored view to match normal view")
	}
}
