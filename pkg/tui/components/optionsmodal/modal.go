package optionsmodal

import (
	"fmt"
	"strings"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

// Row identifies a configurable option or action row in the modal.
type Row int

const (
	RowProfile Row = iota
	RowSurveillance
	RowSensorDegradation
	RowRepairMultiplier
	RowKlingonCloak
	RowTimeMargin
	RowAnimSpeed
	RowColorMode
	RowDone
	NumRows
)

// Model represents the interactive options modal overlay component.
type Model struct {
	Theme       theme.Theme
	rules       engine.GameRules
	colorMode   theme.ColorMode
	SelectedRow Row
	Closed      bool
}

// New creates a new options modal initialized with the given theme and rules.
func New(th theme.Theme, rules engine.GameRules) Model {
	if th == nil {
		th = theme.DefaultTheme()
	}
	return Model{
		Theme:       th,
		rules:       rules,
		colorMode:   th.ColorMode(),
		SelectedRow: RowProfile,
		Closed:      false,
	}
}

// Rules returns the current game rules configured in the modal.
func (m Model) Rules() engine.GameRules {
	return m.rules
}

// SetRules updates the game rules inside the modal.
func (m *Model) SetRules(r engine.GameRules) {
	m.rules = r
}

// ColorMode returns the configured color mode in the modal.
func (m Model) ColorMode() theme.ColorMode {
	return m.colorMode
}

// SetColorMode updates the color mode inside the modal.
func (m *Model) SetColorMode(mode theme.ColorMode) {
	m.colorMode = mode
}

// SetTheme updates the active styling theme and syncs color mode.
func (m *Model) SetTheme(th theme.Theme) {
	if th == nil {
		th = theme.DefaultTheme()
	}
	m.Theme = th
	m.colorMode = th.ColorMode()
}

// Active reports whether the modal is currently open and accepting input.
func (m Model) Active() bool {
	return !m.Closed
}

// Update handles keyboard navigation and option cycling.
func (m Model) Update(msg tea.Msg) (Model, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.KeyMsg:
		switch msg.String() {
		case "up", "k":
			if m.SelectedRow > 0 {
				m.SelectedRow--
			} else {
				m.SelectedRow = NumRows - 1
			}
		case "down", "j":
			if m.SelectedRow < NumRows-1 {
				m.SelectedRow++
			} else {
				m.SelectedRow = 0
			}
		case "left", "h":
			m.cycleOption(-1)
		case "right", "l", " ", "space":
			m.cycleOption(1)
		case "enter":
			if m.SelectedRow == RowDone {
				m.Closed = true
			} else {
				m.cycleOption(1)
			}
		case "esc", "q":
			m.Closed = true
		}
	}
	return m, nil
}

func (m *Model) cycleOption(dir int) {
	profiles := []engine.DifficultyProfile{engine.ProfileCasual, engine.ProfileNormal, engine.ProfileHardcore, engine.ProfileNightmare, engine.ProfileCustom}
	survModes := []engine.SurveillanceMode{engine.SurveillanceFull, engine.SurveillanceClassic, engine.SurveillanceLocal, engine.SurveillanceBlackout}
	repMults := []float64{0.75, 1.00, 1.50, 2.00}
	timeMargins := []float64{1.25, 1.00, 0.80, 0.60}

	switch m.SelectedRow {
	case RowProfile:
		idx := 0
		for i, p := range profiles {
			if p == m.rules.Profile {
				idx = i
				break
			}
		}
		newIdx := (idx + dir + len(profiles)) % len(profiles)
		if profiles[newIdx] != engine.ProfileCustom {
			m.rules = engine.DefaultRulesForProfile(profiles[newIdx])
		} else {
			m.rules.Profile = engine.ProfileCustom
		}
	case RowSurveillance:
		idx := 0
		for i, s := range survModes {
			if s == m.rules.Surveillance {
				idx = i
				break
			}
		}
		m.rules.Surveillance = survModes[(idx+dir+len(survModes))%len(survModes)]
		m.rules.Profile = engine.ProfileCustom
	case RowSensorDegradation:
		m.rules.SensorDegradation = !m.rules.SensorDegradation
		m.rules.Profile = engine.ProfileCustom
	case RowRepairMultiplier:
		idx := 0
		for i, rm := range repMults {
			if rm == m.rules.RepairMultiplier {
				idx = i
				break
			}
		}
		m.rules.RepairMultiplier = repMults[(idx+dir+len(repMults))%len(repMults)]
		m.rules.Profile = engine.ProfileCustom
	case RowKlingonCloak:
		m.rules.KlingonCloak = !m.rules.KlingonCloak
		m.rules.Profile = engine.ProfileCustom
	case RowTimeMargin:
		idx := 0
		for i, tm := range timeMargins {
			if tm == m.rules.TimeMargin {
				idx = i
				break
			}
		}
		m.rules.TimeMargin = timeMargins[(idx+dir+len(timeMargins))%len(timeMargins)]
		m.rules.Profile = engine.ProfileCustom
	case RowAnimSpeed:
		m.rules.AnimSpeed = (m.rules.AnimSpeed + dir + 4) % 4
	case RowColorMode:
		colorModes := []theme.ColorMode{theme.ColorModeAuto, theme.ColorModeDark, theme.ColorModeLight}
		idx := 0
		for i, cm := range colorModes {
			if cm == m.colorMode {
				idx = i
				break
			}
		}
		m.colorMode = colorModes[(idx+dir+len(colorModes))%len(colorModes)]
	}
}

// View renders the options modal overlay dialog.
func (m Model) View() string {
	th := m.Theme
	if th == nil {
		th = theme.DefaultTheme()
	}
	styles := th.Styles()

	boxStyle := styles.Panel.Copy().
		Padding(1, 2).
		Width(66)

	titleStyle := styles.PanelTitle.Copy().
		Bold(true).
		Align(lipgloss.Center)

	var rows []string
	rows = append(rows, titleStyle.Render("⚙ STARFLEET CONFIGURATION & RULES"), "")

	renderRow := func(r Row, label, val string) string {
		prefix := "  "
		style := styles.LogText
		if m.SelectedRow == r {
			prefix = "▶ "
			style = styles.GaugeValue
		}
		return prefix + style.Render(fmt.Sprintf("%-30s ◀ %s ▶", label, val))
	}

	rows = append(rows, renderRow(RowProfile, "Difficulty Profile", strings.ToUpper(string(m.rules.Profile))))
	rows = append(rows, renderRow(RowSurveillance, "Starbase Surveillance", strings.ToUpper(string(m.rules.Surveillance))))
	degStr := "DISABLED"
	if m.rules.SensorDegradation {
		degStr = "ENABLED"
	}
	rows = append(rows, renderRow(RowSensorDegradation, "Sensor Degradation", degStr))
	rows = append(rows, renderRow(RowRepairMultiplier, "Repair Multiplier", fmt.Sprintf("%.2fx", m.rules.RepairMultiplier)))
	cloakStr := "DISABLED"
	if m.rules.KlingonCloak {
		cloakStr = "ENABLED"
	}
	rows = append(rows, renderRow(RowKlingonCloak, "Klingon Cloaking", cloakStr))
	rows = append(rows, renderRow(RowTimeMargin, "Stardate Time Margin", fmt.Sprintf("%.0f%%", m.rules.TimeMargin*100)))
	animSpeedStr := "NORMAL"
	switch m.rules.AnimSpeed {
	case 0:
		animSpeedStr = "OFF"
	case 1:
		animSpeedStr = "FAST"
	case 2:
		animSpeedStr = "NORMAL"
	case 3:
		animSpeedStr = "CINEMATIC"
	}
	rows = append(rows, renderRow(RowAnimSpeed, "Combat Animations", animSpeedStr))
	rows = append(rows, renderRow(RowColorMode, "Color Mode", strings.ToUpper(string(m.colorMode))))

	doneStyle := styles.LogText
	prefix := "  "
	if m.SelectedRow == RowDone {
		prefix = "▶ "
		doneStyle = styles.GaugeValue
	}
	rows = append(rows, "", prefix+doneStyle.Render("[ Done / Resume Mission ]"), "")

	footerStyle := styles.LogText.Copy().Italic(true)
	rows = append(rows, footerStyle.Render("↑/↓: Navigate • ←/→/Space: Change • Esc/q: Close"))

	return boxStyle.Render(strings.Join(rows, "\n"))
}
