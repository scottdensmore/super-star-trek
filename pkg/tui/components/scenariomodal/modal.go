package scenariomodal

import (
	"fmt"
	"strings"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/charmbracelet/x/ansi"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

// MsgLaunchScenario is emitted when the player confirms launching a tactical scenario.
type MsgLaunchScenario struct {
	ScenarioID engine.ScenarioID
}

// MsgCloseScenarioModal is emitted when dismissing the scenario browser modal.
type MsgCloseScenarioModal struct{}

// Model represents the interactive Scenario Browser modal component.
type Model struct {
	theme            theme.Theme
	width            int
	height           int
	scenarios        []*engine.Scenario
	selectedIdx      int
	leaderboardCache map[engine.ScenarioID]*engine.ScoreEntry
}

type modalStyles struct {
	theme.Styles
	borderFg lipgloss.Style
}

func getStyles(th theme.Theme) modalStyles {
	if th == nil {
		th = theme.DefaultTheme()
	}
	base := th.Styles()
	borderFg := base.Panel.GetBorderTopForeground()
	bStyle := lipgloss.NewStyle()
	if borderFg != nil {
		bStyle = bStyle.Foreground(borderFg)
	}
	return modalStyles{
		Styles:   base,
		borderFg: bStyle,
	}
}

func padRight(s string, width int) string {
	w := ansi.StringWidth(s)
	if w >= width {
		return ansi.Truncate(s, width, "")
	}
	return s + strings.Repeat(" ", width-w)
}

func formatBadge(difficulty string, styles modalStyles) string {
	diffUpper := strings.ToUpper(difficulty)
	badgeStr := "[" + diffUpper + "]"
	switch diffUpper {
	case "EXTREME":
		return styles.ConditionRed.Bold(true).Render(badgeStr)
	case "HARD":
		return styles.ConditionYellow.Bold(true).Render(badgeStr)
	case "CHALLENGE":
		return styles.ConditionGreen.Bold(true).Render(badgeStr)
	default:
		return styles.GaugeValue.Render(badgeStr)
	}
}

func operationalConstraints(id engine.ScenarioID) []string {
	switch id {
	case engine.ScenarioKobayashiMaru:
		return []string{
			"• Neutral Zone border violation (illegal entry)",
			"• Waves of enemy Klingon battle cruisers assault",
		}
	case engine.ScenarioMutaraNebula:
		return []string{
			"• Shields completely offline (0.0 energy)",
			"• LRS blinded; cloaked Klingon flagship hunting",
		}
	case engine.ScenarioStarbaseSiege:
		return []string{
			"• Starbase 12 under persistent bombardment",
			"• Must neutralize Klingon fleet before base falls",
		}
	default:
		return []string{
			"• Standard Starfleet tactical engagement rules",
		}
	}
}

// New creates a new Scenario Modal model initialized with the given theme and dimensions.
func New(th theme.Theme, width, height int) Model {
	if th == nil {
		th = theme.DefaultTheme()
	}
	if width <= 0 {
		width = 72
	}
	if height <= 0 {
		height = 18
	}
	m := Model{
		theme:            th,
		width:            width,
		height:           height,
		scenarios:        engine.ListScenarios(),
		selectedIdx:      0,
		leaderboardCache: make(map[engine.ScenarioID]*engine.ScoreEntry),
	}
	m.refreshCache()
	return m
}

// refreshCache loads the top record for each scenario into memory.
func (m *Model) refreshCache() {
	if m.leaderboardCache == nil {
		m.leaderboardCache = make(map[engine.ScenarioID]*engine.ScoreEntry)
	}
	for _, sc := range m.scenarios {
		if lb, err := engine.LoadScenarioLeaderboard(sc.ID); err == nil && lb != nil && len(lb.Entries) > 0 {
			entry := lb.Entries[0]
			m.leaderboardCache[sc.ID] = &entry
		} else {
			m.leaderboardCache[sc.ID] = nil
		}
	}
}

// NewModel initializes and returns a Scenario Modal model with default dimensions.
func NewModel(th theme.Theme) Model {
	return New(th, 72, 18)
}

// SetDimensions sets the dimensions of the modal.
func (m *Model) SetDimensions(width, height int) {
	if width > 0 {
		m.width = width
	}
	if height > 0 {
		m.height = height
	}
}

// SetSize updates the width and height of the modal.
func (m *Model) SetSize(width, height int) {
	m.SetDimensions(width, height)
}

// SetTheme updates the active styling theme.
func (m *Model) SetTheme(th theme.Theme) {
	if th == nil {
		th = theme.DefaultTheme()
	}
	m.theme = th
}

// SelectedScenario returns the currently selected scenario, or nil if none.
func (m Model) SelectedScenario() *engine.Scenario {
	if len(m.scenarios) == 0 || m.selectedIdx < 0 || m.selectedIdx >= len(m.scenarios) {
		return nil
	}
	return m.scenarios[m.selectedIdx]
}

// Reset re-queries registered scenarios, refreshes top records, and resets cursor selection to top.
func (m *Model) Reset() {
	m.scenarios = engine.ListScenarios()
	m.selectedIdx = 0
	m.refreshCache()
}

// Update processes keyboard navigation, selection, and dismissal.
func (m Model) Update(msg tea.Msg) (Model, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.KeyMsg:
		switch msg.Type {
		case tea.KeyEsc:
			return m, func() tea.Msg { return MsgCloseScenarioModal{} }

		case tea.KeyEnter:
			sel := m.SelectedScenario()
			if sel != nil {
				return m, func() tea.Msg {
					return MsgLaunchScenario{ScenarioID: sel.ID}
				}
			}
			return m, nil

		case tea.KeyUp:
			if len(m.scenarios) > 0 {
				m.selectedIdx = (m.selectedIdx - 1 + len(m.scenarios)) % len(m.scenarios)
			}
			return m, nil

		case tea.KeyDown:
			if len(m.scenarios) > 0 {
				m.selectedIdx = (m.selectedIdx + 1) % len(m.scenarios)
			}
			return m, nil
		}

		switch msg.String() {
		case "esc", "q", "Q":
			return m, func() tea.Msg { return MsgCloseScenarioModal{} }

		case "k":
			if len(m.scenarios) > 0 {
				m.selectedIdx = (m.selectedIdx - 1 + len(m.scenarios)) % len(m.scenarios)
			}
			return m, nil

		case "j":
			if len(m.scenarios) > 0 {
				m.selectedIdx = (m.selectedIdx + 1) % len(m.scenarios)
			}
			return m, nil
		}
	}

	return m, nil
}

// View renders the 2-column tactical scenario browser dialog.
func (m Model) View() string {
	styles := getStyles(m.theme)

	const (
		w        = 72
		sidebarW = 25
		detailW  = 44
	)

	titleStr := "[TACTICAL CHALLENGE SIMULATOR]"
	styledTitle := styles.PanelTitle.Render(titleStr)
	dashesTop := w - 3 - ansi.StringWidth(titleStr) - 2
	if dashesTop < 0 {
		dashesTop = 0
	}
	borderTop := styles.borderFg.Render("┌─ ") + styledTitle + " " + styles.borderFg.Render(strings.Repeat("─", dashesTop)) + styles.borderFg.Render("┐")

	footerStr := "[↑/↓/k/j] Navigate  [Enter] Launch  [Esc/q] Dismiss"
	styledFooter := styles.LogText.Render(footerStr)
	dashesBottom := w - 3 - ansi.StringWidth(footerStr) - 2
	if dashesBottom < 0 {
		dashesBottom = 0
	}
	borderBottom := styles.borderFg.Render("└─ ") + styledFooter + " " + styles.borderFg.Render(strings.Repeat("─", dashesBottom)) + styles.borderFg.Render("┘")

	leftRows := m.renderCatalogRows(styles, sidebarW)
	rightRows := m.renderDetailRows(styles, detailW)

	rows := make([]string, 0, 18)
	rows = append(rows, borderTop)
	for i := 0; i < 16; i++ {
		row := styles.borderFg.Render("│") + leftRows[i] + styles.borderFg.Render("│") + rightRows[i] + styles.borderFg.Render("│")
		rows = append(rows, row)
	}
	rows = append(rows, borderBottom)

	return strings.Join(rows, "\n")
}

func (m Model) renderCatalogRows(styles modalStyles, width int) []string {
	rows := make([]string, 16)

	rows[0] = padRight(" "+styles.PanelTitle.Render("SCENARIO CATALOG"), width)
	rows[1] = padRight(" "+styles.borderFg.Render(strings.Repeat("─", width-2)), width)

	curRow := 2
	for i, s := range m.scenarios {
		if curRow+2 > 16 {
			break
		}
		badge := formatBadge(s.Difficulty, styles)
		if i == m.selectedIdx {
			nameLabel := " ▶ " + styles.PanelTitle.Bold(true).Render(s.Name)
			rows[curRow] = padRight(nameLabel, width)
			curRow++
			badgeLabel := "   " + badge
			rows[curRow] = padRight(badgeLabel, width)
			curRow++
		} else {
			nameLabel := "   " + styles.LogText.Render(s.Name)
			rows[curRow] = padRight(nameLabel, width)
			curRow++
			badgeLabel := "   " + badge
			rows[curRow] = padRight(badgeLabel, width)
			curRow++
		}
		if curRow < 16 {
			rows[curRow] = padRight("", width)
			curRow++
		}
	}

	for curRow < 16 {
		rows[curRow] = padRight("", width)
		curRow++
	}

	return rows
}

func (m Model) renderDetailRows(styles modalStyles, width int) []string {
	rows := make([]string, 16)
	s := m.SelectedScenario()
	if s == nil {
		for i := 0; i < 16; i++ {
			rows[i] = padRight("", width)
		}
		return rows
	}

	badge := formatBadge(s.Difficulty, styles)
	nameLine := " " + styles.PanelTitle.Bold(true).Render(strings.ToUpper(s.Name)) + " " + badge
	rows[0] = padRight(nameLine, width)
	rows[1] = padRight(" "+styles.LogText.Italic(true).Render(s.Subtitle), width)
	rows[2] = padRight(" "+styles.borderFg.Render(strings.Repeat("─", width-2)), width)

	rows[3] = padRight(" "+styles.GaugeLabel.Render("BRIEFING:"), width)
	briefLines := s.Briefing
	for i := 0; i < 4; i++ {
		line := ""
		if i < len(briefLines) {
			line = briefLines[i]
		}
		rows[4+i] = padRight(" "+styles.LogText.Render(line), width)
	}

	rows[8] = padRight(" "+styles.borderFg.Render(strings.Repeat("─", width-2)), width)

	rows[9] = padRight(" "+styles.GaugeLabel.Render("OPERATIONAL CONSTRAINTS:"), width)
	constraints := operationalConstraints(s.ID)
	c1 := ""
	c2 := ""
	if len(constraints) > 0 {
		c1 = constraints[0]
	}
	if len(constraints) > 1 {
		c2 = constraints[1]
	}
	rows[10] = padRight(" "+styles.LogText.Render(c1), width)
	rows[11] = padRight(" "+styles.LogText.Render(c2), width)

	rows[12] = padRight(" "+styles.borderFg.Render(strings.Repeat("─", width-2)), width)

	rows[13] = padRight(" "+styles.GaugeLabel.Render("TOP LEADERBOARD RECORD:"), width)

	var top *engine.ScoreEntry
	if m.leaderboardCache != nil {
		top = m.leaderboardCache[s.ID]
	}
	if top != nil {
		rec1 := fmt.Sprintf("%s  %s", styles.PanelTitle.Render(top.CaptainName), styles.GaugeValue.Render(fmt.Sprintf("%d pts", top.Score)))
		rec2 := fmt.Sprintf("Rank: %s | Stardate: %.1f | %s", top.Rank, top.Stardate, top.Date.Format("2006-01-02"))
		rows[14] = padRight(" "+rec1, width)
		rows[15] = padRight(" "+styles.LogText.Render(rec2), width)
	} else {
		rows[14] = padRight(" "+styles.LogText.Render("No record recorded yet."), width)
		rows[15] = padRight(" "+styles.LogText.Render("Be the first captain to conquer this challenge."), width)
	}

	return rows
}
