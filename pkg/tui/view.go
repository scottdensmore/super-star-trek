package tui

import (
	"fmt"
	"strings"

	"github.com/charmbracelet/lipgloss"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

// View renders the TUI screen. If terminal dimensions are smaller than 80x24,
// a size warning notice is rendered. Otherwise, the split dashboard is presented.
func (m Model) View() string {
	if m.Width < 80 || m.Height < 24 {
		return m.renderSizeNotice()
	}
	return m.renderDashboard()
}

// renderSizeNotice renders a centered warning message instructing the player
// to resize their terminal window to at least 80x24.
func (m Model) renderSizeNotice() string {
	th := m.Theme
	if th == nil {
		th = theme.DefaultTheme()
	}
	styles := th.Styles()

	title := styles.Title.Render("TERMINAL WINDOW TOO SMALL")
	dims := styles.GaugeLabel.Render(fmt.Sprintf("Current dimensions: %d columns x %d rows", m.Width, m.Height))
	minReq := styles.GaugeValue.Render("Required minimum:   80 columns x 24 rows")
	hint := styles.LogText.Render("Please resize your terminal window to continue.")

	content := fmt.Sprintf("%s\n\n%s\n%s\n\n%s", title, dims, minReq, hint)
	box := styles.Panel.Render(content)

	if m.Width > 0 && m.Height > 0 {
		return lipgloss.Place(m.Width, m.Height, lipgloss.Center, lipgloss.Center, box)
	}
	return box
}

// renderDashboard composes the full-screen layout:
// - Top: Header banner with game title and active theme
// - Middle: Horizontal split of 8x8 sector grid (left) and telemetry/status panel (right)
// - Bottom: Interactive command bar with scrolling event history log
func (m Model) renderDashboard() string {
	header := m.renderHeader()

	var quad *engine.QuadrantState
	var entSector engine.Coord
	if m.Game != nil {
		quad = &m.Game.CurrentQuad
		entSector = m.Game.Enterprise.Sector
	}

	gridView := m.Grid.View(quad, entSector)
	statusView := m.Status.View(m.Game)
	middle := lipgloss.JoinHorizontal(lipgloss.Top, gridView, statusView)

	cb := m.CommandBar
	if m.Width > 0 {
		cb.SetWidth(m.Width)
	}
	commandBarView := cb.View()

	return lipgloss.JoinVertical(lipgloss.Left, header, middle, commandBarView)
}

// renderHeader formats the top status banner with game title and theme indicator.
func (m Model) renderHeader() string {
	th := m.Theme
	if th == nil {
		th = theme.DefaultTheme()
	}
	styles := th.Styles()

	title := "★ SUPER STAR TREK ★  USS ENTERPRISE NCC-1701"
	themeInfo := fmt.Sprintf("[Theme: %s (F2)]", strings.ToUpper(th.Name()))

	w := m.Width
	if w < 80 {
		w = 80
	}

	frame := styles.Title.GetHorizontalFrameSize()
	contentWidth := w - frame
	gap := contentWidth - lipgloss.Width(title) - lipgloss.Width(themeInfo)
	if gap < 2 {
		gap = 2
	}
	headerLine := title + strings.Repeat(" ", gap) + themeInfo

	return styles.Title.Width(contentWidth).Render(headerLine)
}
