package savebrowser

import (
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strings"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

// LoadGameMsg is emitted when the user confirms thawing a mission.
type LoadGameMsg struct {
	Path string
}

// CloseBrowserMsg is emitted when the user dismisses the save browser without selection.
type CloseBrowserMsg struct{}

// Model represents the interactive save game browser modal.
type Model struct {
	theme        theme.Theme
	directory    string
	saves        []engine.SaveMetadata
	cursor       int
	deleting     bool
	errorMessage string
	width        int
	height       int
}

// New creates a new save game browser model.
func New(th theme.Theme, dir string) Model {
	if th == nil {
		th = theme.DefaultTheme()
	}
	if dir == "" {
		dir = "."
	}
	return Model{
		theme:     th,
		directory: dir,
		width:     62,
		height:    14,
	}
}

// SetTheme updates the component theme.
func (m *Model) SetTheme(th theme.Theme) {
	if th == nil {
		th = theme.DefaultTheme()
	}
	m.theme = th
}

// SetSize updates the dimensions for the modal dialog.
func (m *Model) SetSize(width, height int) {
	m.width = width
	m.height = height
}

// Reset resets cursor position, deletion state, and error message.
func (m *Model) Reset() {
	m.cursor = 0
	m.deleting = false
	m.errorMessage = ""
}

// Refresh scans the target directory for *.TRK files and parses their metadata.
func (m *Model) Refresh() error {
	m.errorMessage = ""
	entries, err := os.ReadDir(m.directory)
	if err != nil {
		m.errorMessage = fmt.Sprintf("Failed to read directory: %v", err)
		m.saves = nil
		m.cursor = 0
		return err
	}

	var loaded []engine.SaveMetadata
	for _, entry := range entries {
		if entry.IsDir() {
			continue
		}
		name := entry.Name()
		if strings.HasSuffix(strings.ToUpper(name), ".TRK") {
			fullPath := filepath.Join(m.directory, name)
			meta, err := engine.InspectSave(fullPath)
			if err == nil && meta != nil {
				loaded = append(loaded, *meta)
			}
		}
	}

	// Sort saves by modification time descending (most recent first)
	sort.Slice(loaded, func(i, j int) bool {
		return loaded[i].ModTime.After(loaded[j].ModTime)
	})

	m.saves = loaded
	if m.cursor >= len(m.saves) {
		m.cursor = len(m.saves) - 1
	}
	if m.cursor < 0 {
		m.cursor = 0
	}
	return nil
}

// Update processes keyboard navigation, selection, and deletion.
func (m Model) Update(msg tea.Msg) (Model, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.KeyMsg:
		switch msg.Type {
		case tea.KeyUp:
			if !m.deleting && m.cursor > 0 {
				m.cursor--
			}
			return m, nil

		case tea.KeyDown:
			if !m.deleting && m.cursor < len(m.saves)-1 {
				m.cursor++
			}
			return m, nil

		case tea.KeyEnter:
			if m.deleting {
				if len(m.saves) > 0 && m.cursor >= 0 && m.cursor < len(m.saves) {
					target := m.saves[m.cursor].Path
					if err := os.Remove(target); err != nil {
						m.errorMessage = fmt.Sprintf("Delete failed: %v", err)
					}
					m.deleting = false
					_ = m.Refresh()
				}
				return m, nil
			}

			if len(m.saves) > 0 && m.cursor >= 0 && m.cursor < len(m.saves) {
				selectedPath := m.saves[m.cursor].Path
				return m, func() tea.Msg {
					return LoadGameMsg{Path: selectedPath}
				}
			}
			return m, nil

		case tea.KeyEsc:
			if m.deleting {
				m.deleting = false
				return m, nil
			}
			return m, func() tea.Msg {
				return CloseBrowserMsg{}
			}

		case tea.KeyDelete:
			if len(m.saves) > 0 {
				m.deleting = true
			}
			return m, nil

		case tea.KeyRunes:
			s := msg.String()
			switch s {
			case "k", "K":
				if !m.deleting && m.cursor > 0 {
					m.cursor--
				}
				return m, nil
			case "j", "J":
				if !m.deleting && m.cursor < len(m.saves)-1 {
					m.cursor++
				}
				return m, nil
			case "d", "D":
				if len(m.saves) > 0 {
					m.deleting = !m.deleting
				}
				return m, nil
			}

		default:
			switch msg.String() {
			case "k", "K", "up":
				if !m.deleting && m.cursor > 0 {
					m.cursor--
				}
				return m, nil
			case "j", "J", "down":
				if !m.deleting && m.cursor < len(m.saves)-1 {
					m.cursor++
				}
				return m, nil
			case "d", "D":
				if len(m.saves) > 0 {
					m.deleting = !m.deleting
				}
				return m, nil
			case "enter":
				if m.deleting {
					if len(m.saves) > 0 && m.cursor >= 0 && m.cursor < len(m.saves) {
						target := m.saves[m.cursor].Path
						if err := os.Remove(target); err != nil {
							m.errorMessage = fmt.Sprintf("Delete failed: %v", err)
						}
						m.deleting = false
						_ = m.Refresh()
					}
					return m, nil
				}
				if len(m.saves) > 0 && m.cursor >= 0 && m.cursor < len(m.saves) {
					selectedPath := m.saves[m.cursor].Path
					return m, func() tea.Msg {
						return LoadGameMsg{Path: selectedPath}
					}
				}
				return m, nil
			case "esc":
				if m.deleting {
					m.deleting = false
					return m, nil
				}
				return m, func() tea.Msg {
					return CloseBrowserMsg{}
				}
			}
		}
	}
	return m, nil
}

// View renders the 62x14 styled modal dialog.
func (m Model) View() string {
	th := m.theme
	if th == nil {
		th = theme.DefaultTheme()
	}
	styles := th.Styles()

	dialogWidth := 60
	if m.width > 0 {
		dialogWidth = m.width - 2
	}

	titleText := " SAVED MISSIONS (Ctrl+O) "
	header := styles.Title.Render(titleText)

	colHeader := styles.GaugeLabel.Render("  FILE       STARDATE  SKILL   COND    KLINGONS  MODIFIED")

	var rows []string
	const visibleRows = 7
	startIdx := 0
	if m.cursor >= visibleRows {
		startIdx = m.cursor - visibleRows + 1
	}
	endIdx := startIdx + visibleRows
	if endIdx > len(m.saves) {
		endIdx = len(m.saves)
	}

	if len(m.saves) == 0 {
		emptyMsg := styles.LogText.Render("  No saved missions (*.TRK) found.")
		rows = append(rows, emptyMsg)
		for len(rows) < visibleRows {
			rows = append(rows, "")
		}
	} else {
		for i := startIdx; i < endIdx; i++ {
			s := m.saves[i]
			cursorMark := "  "
			if i == m.cursor {
				cursorMark = "> "
			}

			skillStr := formatSkill(s.Skill)
			condStr := formatCondition(s.Condition)
			modStr := s.ModTime.Format("01-02 15:04")
			baseName := s.Filename
			if len(baseName) > 10 {
				baseName = baseName[:10]
			}

			line := fmt.Sprintf("%s%-10s %-9.1f %-7s %-7s %-9d %s",
				cursorMark, baseName, s.Stardate, skillStr, condStr, s.KlingonsLeft, modStr)

			if i == m.cursor {
				rows = append(rows, styles.CommandText.Render(line))
			} else {
				rows = append(rows, styles.LogText.Render(line))
			}
		}
		for len(rows) < visibleRows {
			rows = append(rows, "")
		}
	}

	borderFg := styles.Panel.GetBorderTopForeground()
	dividerStyle := lipgloss.NewStyle()
	if borderFg != nil {
		dividerStyle = dividerStyle.Foreground(borderFg)
	}
	divider := dividerStyle.Render(strings.Repeat("─", dialogWidth))

	var footer string
	if m.deleting && len(m.saves) > 0 && m.cursor < len(m.saves) {
		delPrompt := fmt.Sprintf("Delete '%s'? [Enter: Confirm / Esc: Cancel]", m.saves[m.cursor].Filename)
		footer = styles.GaugeValue.Render(delPrompt)
	} else if m.errorMessage != "" {
		footer = styles.GaugeValue.Render(m.errorMessage)
	} else {
		footer = styles.LogText.Render("[Enter] Thaw  [D] Delete  [↑/↓] Select  [Esc] Close")
	}

	body := fmt.Sprintf("%s\n\n%s\n%s\n%s\n%s",
		header,
		colHeader,
		strings.Join(rows, "\n"),
		divider,
		footer,
	)

	return styles.Panel.Width(dialogWidth).Render(body)
}

func formatSkill(skill engine.SkillLevel) string {
	switch skill {
	case engine.SkillNovice:
		return "NOVICE"
	case engine.SkillFair:
		return "FAIR"
	case engine.SkillGood:
		return "GOOD"
	case engine.SkillExpert:
		return "EXPERT"
	case engine.SkillEmeritus:
		return "EMERITUS"
	default:
		return "UNKNOWN"
	}
}

func formatCondition(c engine.ConditionType) string {
	switch c {
	case engine.ConditionGreen:
		return "GREEN"
	case engine.ConditionYellow:
		return "YELLOW"
	case engine.ConditionRed:
		return "RED"
	case engine.ConditionDocked:
		return "DOCKED"
	default:
		return "UNKNOWN"
	}
}
