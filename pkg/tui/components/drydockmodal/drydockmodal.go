package drydockmodal

import (
	"fmt"
	"strings"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

// DisembarkMsg is emitted when the player concludes drydock refits and departs.
type DisembarkMsg struct{}

// RefitPurchasedMsg is emitted when a refit module is successfully purchased and installed.
type RefitPurchasedMsg struct {
	RefitID engine.RefitID
	Tier    int
}

// Model represents the interactive Starbase Drydock & Refit Facility modal overlay.
type Model struct {
	Tour          *engine.TourState
	Theme         theme.Theme
	Width         int
	Height        int
	selectedIndex int
	errorMessage  string
}

type drydockStyles struct {
	BorderNormal  lipgloss.TerminalColor
	Foreground    lipgloss.TerminalColor
	GaugeGood     lipgloss.TerminalColor
	ActiveCommand lipgloss.TerminalColor
	StatusMuted   lipgloss.TerminalColor
	ConditionRed  lipgloss.TerminalColor
}

func getStyles(th theme.Theme) drydockStyles {
	if th == nil {
		th = theme.DefaultTheme()
	}
	base := th.Styles()
	return drydockStyles{
		BorderNormal:  base.Panel.GetBorderTopForeground(),
		Foreground:    base.CommandText.GetForeground(),
		GaugeGood:     base.ConditionGreen.GetForeground(),
		ActiveCommand: base.Prompt.GetForeground(),
		StatusMuted:   base.LogText.GetForeground(),
		ConditionRed:  base.ConditionRed.GetForeground(),
	}
}

// New creates a new drydock modal Model with default dimensions and selection.
func New(tour *engine.TourState, th theme.Theme) Model {
	if th == nil {
		th = theme.DefaultTheme()
	}
	return Model{
		Tour:          tour,
		Theme:         th,
		Width:         80,
		Height:        24,
		selectedIndex: 0,
	}
}

// SetTheme updates the active theme for the drydock modal.
func (m *Model) SetTheme(th theme.Theme) {
	if th == nil {
		th = theme.DefaultTheme()
	}
	m.Theme = th
}

// SelectedIndex returns the index of the currently highlighted refit item.
func (m Model) SelectedIndex() int {
	return m.selectedIndex
}

// Update handles keyboard navigation, refit purchases, and disembarking.
func (m Model) Update(msg tea.Msg) (Model, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.KeyMsg:
		m.errorMessage = ""
		switch msg.String() {
		case "up", "k":
			if m.selectedIndex > 0 {
				m.selectedIndex--
			}
		case "down", "j":
			if m.selectedIndex < len(engine.RefitCatalog)-1 {
				m.selectedIndex++
			}
		case "enter":
			if m.selectedIndex < 0 || m.selectedIndex >= len(engine.RefitCatalog) {
				return m, nil
			}
			def := engine.RefitCatalog[m.selectedIndex]
			err := engine.PurchaseRefit(m.Tour, def.ID)
			if err != nil {
				m.errorMessage = err.Error()
				return m, nil
			}
			tier := 0
			if m.Tour != nil && m.Tour.InstalledRefits != nil {
				tier = m.Tour.InstalledRefits[def.ID]
			}
			return m, func() tea.Msg {
				return RefitPurchasedMsg{RefitID: def.ID, Tier: tier}
			}
		case " ", "space", "d", "D":
			return m, func() tea.Msg {
				return DisembarkMsg{}
			}
		}
	}
	return m, nil
}

// View renders the dual-pane Drydock & Refit Facility modal dialog.
func (m Model) View() string {
	styles := getStyles(m.Theme)

	boxStyle := lipgloss.NewStyle().
		Border(lipgloss.RoundedBorder()).
		BorderForeground(styles.BorderNormal).
		Width(78).
		Padding(0, 1)

	headerStyle := lipgloss.NewStyle().
		Foreground(styles.Foreground).
		Bold(true)

	accentStyle := lipgloss.NewStyle().
		Foreground(styles.GaugeGood).
		Bold(true)

	// Top Title
	secIdx := 0
	reqPts := 0
	numInstalled := 0
	if m.Tour != nil {
		secIdx = m.Tour.SectorsCompleted
		reqPts = m.Tour.RequisitionPoints
		numInstalled = len(m.Tour.InstalledRefits)
	}
	title := headerStyle.Render(fmt.Sprintf("★ STARBASE 01 DRYDOCK & REFIT FACILITY ★  (Sector %d Cleared)", secIdx))
	reqText := accentStyle.Render(fmt.Sprintf("Requisition: %d PTS", reqPts))
	topBar := fmt.Sprintf("%-52s %22s", title, reqText)

	// Left: ASCII Starship Cutaway
	schematic := `
      /================\
     /                  \
 ===|     NCC - 1701     |===
 #  \                    /  #
 #   \==================/   #
 #            ||            #
 #+-------+   ||   +-------+#
  | WARP  |=======| WARP  |
  +-------+       +-------+
`
	leftPane := lipgloss.NewStyle().
		Width(30).
		Render(fmt.Sprintf("%s\nModules Installed: %d/6", schematic, numInstalled))

	// Right: Refit Store List
	var items []string
	for i, refit := range engine.RefitCatalog {
		cursor := "  "
		if i == m.selectedIndex {
			cursor = "> "
		}
		currentTier := 0
		if m.Tour != nil && m.Tour.InstalledRefits != nil {
			currentTier = m.Tour.InstalledRefits[refit.ID]
		}
		tierStr := fmt.Sprintf("[Tier %d/3]", currentTier)

		costStr := "[MAX TIER]"
		if currentTier < 3 {
			costStr = fmt.Sprintf("Cost: %d", refit.TierCosts[currentTier])
		}

		itemStyle := lipgloss.NewStyle().Foreground(styles.Foreground)
		if i == m.selectedIndex {
			itemStyle = itemStyle.Bold(true).Foreground(styles.ActiveCommand)
		}

		line := fmt.Sprintf("%s%-26s %-12s %s", cursor, refit.Name, tierStr, costStr)
		desc := fmt.Sprintf("     %s", refit.Description)
		items = append(items, itemStyle.Render(line), lipgloss.NewStyle().Foreground(styles.StatusMuted).Render(desc))
	}
	rightPane := lipgloss.NewStyle().
		Width(44).
		Render(strings.Join(items, "\n"))

	content := lipgloss.JoinHorizontal(lipgloss.Top, leftPane, rightPane)

	footer := lipgloss.NewStyle().
		Foreground(styles.StatusMuted).
		Render("[↑/↓ / j/k]: Select   [Enter]: Purchase Refit   [Space / D]: Disembark to Next Sector")

	if m.errorMessage != "" {
		footer = lipgloss.NewStyle().Foreground(styles.ConditionRed).Render("Error: " + m.errorMessage)
	}

	return boxStyle.Render(fmt.Sprintf("%s\n\n%s\n\n%s", topBar, content, footer))
}
