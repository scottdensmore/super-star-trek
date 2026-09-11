package commandbar

import (
	"strings"

	"github.com/charmbracelet/bubbles/textinput"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

// CommandSubmittedMsg is dispatched when the player submits a non-empty command via Enter.
type CommandSubmittedMsg struct {
	Text string
}

// Model represents the interactive command bar component, embedding bubbles/textinput.Model
// and maintaining a scrolling buffer of recent combat/tactical log messages.
type Model struct {
	textinput.Model
	theme       theme.Theme
	messages    []string
	maxMessages int
	width       int
}

// CommandBarModel is an alias for Model for backwards/spec compatibility.
type CommandBarModel = Model

// New creates a new command bar Model initialized with prompt "COMMAND> "
// and styles configured from the provided theme.
func New(th theme.Theme) Model {
	if th == nil {
		th = theme.DefaultTheme()
	}

	ti := textinput.New()
	ti.Prompt = "COMMAND> "
	styles := th.Styles()
	ti.PromptStyle = styles.Prompt
	ti.TextStyle = styles.CommandText
	ti.Focus()

	return Model{
		Model:       ti,
		theme:       th,
		messages:    make([]string, 0, 4),
		maxMessages: 4,
	}
}

// NewCommandBar is an alias for New.
func NewCommandBar(th theme.Theme) CommandBarModel {
	return New(th)
}

// SetTheme updates the active theme and reapplies input styles.
func (m *Model) SetTheme(th theme.Theme) {
	if th == nil {
		th = theme.DefaultTheme()
	}
	m.theme = th
	styles := th.Styles()
	m.PromptStyle = styles.Prompt
	m.TextStyle = styles.CommandText
}

// Theme returns the currently active theme.
func (m Model) Theme() theme.Theme {
	if m.theme == nil {
		return theme.DefaultTheme()
	}
	return m.theme
}

// AddMessage appends a message to the scrolling event log buffer.
// If the message contains newline characters, each line is treated
// as an individual log entry. When the buffer exceeds maxMessages (4),
// older entries scroll off.
func (m *Model) AddMessage(msg string) {
	if m.maxMessages <= 0 {
		m.maxMessages = 4
	}
	msg = strings.TrimRight(msg, "\r\n")
	if msg == "" {
		return
	}
	lines := strings.Split(msg, "\n")
	for _, line := range lines {
		line = strings.TrimRight(line, "\r")
		if len(m.messages) >= m.maxMessages {
			m.messages = m.messages[1:]
		}
		m.messages = append(m.messages, line)
	}
}

// Messages returns a copy of the current message log buffer.
func (m Model) Messages() []string {
	msgs := make([]string, len(m.messages))
	copy(msgs, m.messages)
	return msgs
}

// ClearMessages empties the message log buffer.
func (m *Model) ClearMessages() {
	m.messages = m.messages[:0]
}

// SetWidth sets the display width for the command bar panel.
func (m *Model) SetWidth(w int) {
	m.width = w
	if w > 0 {
		avail := w - 4
		if avail > 0 {
			m.Model.Width = avail
		}
	} else {
		m.Model.Width = 0
	}
}

// Width returns the configured panel display width.
func (m Model) Width() int {
	return m.width
}

// Focus focuses the text input element, enabling keyboard capture.
func (m *Model) Focus() tea.Cmd {
	return m.Model.Focus()
}

// Blur blurs the text input element, disabling keyboard capture.
func (m *Model) Blur() {
	m.Model.Blur()
}

// Update processes Bubble Tea messages, submitting commands on Enter
// or delegating keystrokes to the embedded textinput.
func (m Model) Update(msg tea.Msg) (Model, tea.Cmd) {
	if keyMsg, ok := msg.(tea.KeyMsg); ok {
		switch keyMsg.Type {
		case tea.KeyEnter:
			val := strings.TrimSpace(m.Value())
			if val != "" {
				m.Reset()
				return m, func() tea.Msg {
					return CommandSubmittedMsg{Text: val}
				}
			}
			return m, nil
		}
	}

	var cmd tea.Cmd
	m.Model, cmd = m.Model.Update(msg)
	return m, cmd
}

// View renders the 4-line scrolling message buffer and the styled COMMAND> input line.
func (m Model) View() string {
	th := m.theme
	if th == nil {
		th = theme.DefaultTheme()
	}
	styles := th.Styles()

	var b strings.Builder
	const logBufferLines = 4
	for i := 0; i < logBufferLines; i++ {
		msgIdx := i - (logBufferLines - len(m.messages))
		if msgIdx >= 0 && msgIdx < len(m.messages) {
			b.WriteString(styles.LogText.Render(m.messages[msgIdx]))
		}
		b.WriteByte('\n')
	}
	b.WriteString(m.Model.View())

	panelStyle := styles.Panel
	if m.width > 0 {
		borderPaddingWidth := panelStyle.GetHorizontalFrameSize()
		contentWidth := m.width - borderPaddingWidth
		if contentWidth > 0 {
			panelStyle = panelStyle.Width(contentWidth)
		}
	}

	return panelStyle.Render(b.String())
}
