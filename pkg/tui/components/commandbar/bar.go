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
// and maintaining a scrolling buffer of recent combat/tactical log messages,
// a readline command history buffer, and tab keyword completion.
type Model struct {
	textinput.Model
	theme       theme.Theme
	messages    []string
	maxMessages int
	width       int

	history     []string
	historyIdx  int
	draftInput  string
	tabMatches  []string
	tabMatchIdx int
}

// CommandBarModel is an alias for Model for backwards/spec compatibility.
type CommandBarModel = Model

var canonicalKeywords = []string{
	"chart",
	"damage",
	"doc",
	"help",
	"lrscan",
	"nav ",
	"pha ",
	"quit",
	"saves",
	"she ",
	"srscan",
	"status",
	"target",
	"thaw",
	"theme ",
	"tor ",
}

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
		historyIdx:  -1,
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

// Update processes Bubble Tea messages, submitting commands on Enter,
// navigating history with Up/Down, auto-completing keywords on Tab,
// or delegating keystrokes to the embedded textinput.
func (m Model) Update(msg tea.Msg) (Model, tea.Cmd) {
	if keyMsg, ok := msg.(tea.KeyMsg); ok {
		switch keyMsg.Type {
		case tea.KeyUp:
			m.tabMatches = nil
			m.tabMatchIdx = 0
			if m.historyIdx == -1 {
				if len(m.history) > 0 {
					m.draftInput = m.Value()
					m.historyIdx = len(m.history) - 1
					m.SetValue(m.history[m.historyIdx])
					m.CursorEnd()
				}
			} else if m.historyIdx > 0 {
				m.historyIdx--
				m.SetValue(m.history[m.historyIdx])
				m.CursorEnd()
			}
			return m, nil

		case tea.KeyDown:
			m.tabMatches = nil
			m.tabMatchIdx = 0
			if m.historyIdx != -1 {
				if m.historyIdx < len(m.history)-1 {
					m.historyIdx++
					m.SetValue(m.history[m.historyIdx])
					m.CursorEnd()
				} else if m.historyIdx == len(m.history)-1 {
					m.historyIdx = -1
					m.SetValue(m.draftInput)
					m.CursorEnd()
				}
			}
			return m, nil

		case tea.KeyTab:
			if len(m.tabMatches) == 0 || m.tabMatchIdx >= len(m.tabMatches) || m.Value() != m.tabMatches[m.tabMatchIdx] {
				prefix := strings.ToLower(strings.TrimSpace(m.Value()))
				if prefix != "" {
					var matches []string
					for _, kw := range canonicalKeywords {
						trimmedKw := strings.TrimSpace(kw)
						if strings.HasPrefix(trimmedKw, prefix) {
							matches = append(matches, kw)
						}
					}
					if len(matches) > 0 {
						m.tabMatches = matches
						m.tabMatchIdx = 0
						m.SetValue(m.tabMatches[0])
						m.CursorEnd()
					}
				}
			} else {
				m.tabMatchIdx = (m.tabMatchIdx + 1) % len(m.tabMatches)
				m.SetValue(m.tabMatches[m.tabMatchIdx])
				m.CursorEnd()
			}
			return m, nil

		case tea.KeyEnter:
			val := strings.TrimSpace(m.Value())
			if val != "" {
				if len(m.history) == 0 || m.history[len(m.history)-1] != val {
					m.history = append(m.history, val)
				}
				m.historyIdx = -1
				m.draftInput = ""
				m.tabMatches = nil
				m.tabMatchIdx = 0
				m.Reset()
				return m, func() tea.Msg {
					return CommandSubmittedMsg{Text: val}
				}
			}
			return m, nil

		default:
			// Any key other than Tab resets active tab cycling
			m.tabMatches = nil
			m.tabMatchIdx = 0
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
