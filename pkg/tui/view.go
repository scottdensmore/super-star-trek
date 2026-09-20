package tui

import (
	"fmt"
	"strings"

	"github.com/charmbracelet/lipgloss"
	"github.com/charmbracelet/x/ansi"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

// View renders the TUI screen. If terminal dimensions are smaller than 80x24,
// a size warning notice is rendered. Otherwise, the split dashboard is presented.
func (m Model) View() string {
	if m.Width < 80 || m.Height < 24 {
		return m.renderSizeNotice()
	}
	if m.showOptions {
		overlay := m.optionsModal.View()
		return lipgloss.Place(m.Width, m.Height, lipgloss.Center, lipgloss.Center, overlay)
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
// When an overlay modal is active, it is composited centered over the background dashboard.
func (m Model) renderDashboard() string {
	header := m.renderHeader()

	var quad *engine.QuadrantState
	var entSector engine.Coord
	if m.Game != nil {
		quad = &m.Game.CurrentQuad
		entSector = m.Game.Enterprise.Sector
	}

	gridView := m.Grid.View(quad, entSector, m.SelectedSector)
	statusView := m.Status.View(m.Game)

	var middle string
	if m.Width >= 115 && m.Game != nil {
		chartView := m.renderMiniGalacticChart()
		middle = lipgloss.JoinHorizontal(lipgloss.Top, gridView, statusView, chartView)
	} else {
		middle = lipgloss.JoinHorizontal(lipgloss.Top, gridView, statusView)
	}

	cb := m.CommandBar
	if m.Width > 0 {
		cb.SetWidth(m.Width)
	}
	commandBarView := cb.View()

	dashboard := lipgloss.JoinVertical(lipgloss.Left, header, middle, commandBarView)

	var modalView string
	switch m.ActiveModal {
	case ModalTargetLock:
		modalView = m.TargetLock.View()
	case ModalCommandPalette:
		modalView = m.CommandPalette.View()
	case ModalSaveBrowser:
		modalView = m.SaveBrowser.View()
	case ModalGalacticChart:
		modalView = m.GalacticChart.View()
	case ModalDamageSchematic:
		modalView = m.DamageSchematic.View()
	case ModalHallOfFame:
		modalView = m.HallOfFame.View()
	case ModalManual:
		modalView = m.Manual.View()
	case ModalScenario:
		modalView = m.scenarioModal.View()
	default:
		return dashboard
	}

	return compositeOverlay(dashboard, modalView, m.Width, m.Height)
}

// compositeOverlay composites a floating modal overlay centered over a background dashboard.
// It uses ANSI-aware line cutting and truncation so the background remains visible
// around the overlay modal.
func compositeOverlay(background, overlay string, totalWidth, totalHeight int) string {
	bgLines := strings.Split(background, "\n")
	overlayLines := strings.Split(overlay, "\n")

	// Calculate maximum overlay dimensions
	overlayWidth := 0
	for _, line := range overlayLines {
		if w := ansi.StringWidth(line); w > overlayWidth {
			overlayWidth = w
		}
	}
	overlayHeight := len(overlayLines)

	if totalHeight < overlayHeight {
		totalHeight = overlayHeight
	}
	if totalHeight < len(bgLines) {
		totalHeight = len(bgLines)
	}
	if totalWidth < overlayWidth {
		totalWidth = overlayWidth
	}
	if totalWidth <= 0 {
		for _, line := range bgLines {
			if w := ansi.StringWidth(line); w > totalWidth {
				totalWidth = w
			}
		}
	}

	// Pad background lines if fewer than totalHeight
	for len(bgLines) < totalHeight {
		bgLines = append(bgLines, "")
	}

	startX := (totalWidth - overlayWidth) / 2
	if startX < 0 {
		startX = 0
	}
	startY := (totalHeight - overlayHeight) / 2
	if startY < 0 {
		startY = 0
	}

	result := make([]string, len(bgLines))
	for y, bgLine := range bgLines {
		if y < startY || y >= startY+overlayHeight {
			result[y] = bgLine
			continue
		}

		modalLine := overlayLines[y-startY]
		modalLineWidth := ansi.StringWidth(modalLine)
		if modalLineWidth < overlayWidth {
			modalLine += strings.Repeat(" ", overlayWidth-modalLineWidth)
		}

		bgWidth := ansi.StringWidth(bgLine)
		var left string
		if bgWidth <= startX {
			left = bgLine + strings.Repeat(" ", startX-bgWidth)
		} else {
			left = ansi.Truncate(bgLine, startX, "")
			leftWidth := ansi.StringWidth(left)
			if leftWidth < startX {
				left += strings.Repeat(" ", startX-leftWidth)
			}
		}

		var right string
		if bgWidth > startX+overlayWidth {
			right = ansi.TruncateLeft(bgLine, startX+overlayWidth, "")
		}

		result[y] = left + "\x1b[0m" + modalLine + right
	}

	return strings.Join(result, "\n")
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

	return styles.Title.Width(w).Render(headerLine)
}

// renderMiniGalacticChart renders an 8x8 Galactic Star Chart and Mission Ops
// panel displayed side-by-side with Telemetry when terminal width >= 115.
func (m Model) renderMiniGalacticChart() string {
	if m.Game == nil {
		return ""
	}
	th := m.Theme
	if th == nil {
		th = theme.DefaultTheme()
	}
	styles := th.Styles()
	g := m.Game

	var b strings.Builder
	b.Grow(512)

	// Line 1: Panel title centered in 43 columns
	titleText := "GALACTIC STAR CHART [K-B-S]"
	titlePadding := (43 - lipgloss.Width(titleText)) / 2
	if titlePadding < 0 {
		titlePadding = 0
	}
	b.WriteString(styles.PanelTitle.Render(strings.Repeat(" ", titlePadding) + titleText))

	// Line 2: Column numbers header (1..8) with 5-character cell stride
	b.WriteByte('\n')
	var colHdr strings.Builder
	colHdr.WriteString("   ")
	for col := 1; col <= 8; col++ {
		fmt.Fprintf(&colHdr, "  %d  ", col)
	}
	b.WriteString(styles.GridHeader.Render(colHdr.String()))

	// Lines 3-10: 8 quadrant rows
	explored := 0
	for r := 1; r <= 8; r++ {
		b.WriteByte('\n')
		b.WriteString(styles.GridHeader.Render(fmt.Sprintf(" %d ", r)))
		for c := 1; c <= 8; c++ {
			isEnt := (r == g.Enterprise.Quad[0] && c == g.Enterprise.Quad[1])
			discovered := g.ChartDiscovered[r][c]
			if discovered {
				explored++
			}

			if isEnt {
				var valStr string
				if discovered {
					valStr = fmt.Sprintf("%03d", g.GalaxyChart[r][c])
				} else {
					valStr = "···"
				}
				b.WriteString(styles.Enterprise.Render("<" + valStr + ">"))
			} else if discovered {
				val := g.GalaxyChart[r][c]
				valStr := fmt.Sprintf("%03d", val)
				if val >= 100 {
					b.WriteString(styles.SubsystemDamaged.Render(" " + valStr + " "))
				} else if (val%100)/10 > 0 {
					b.WriteString(styles.CommandText.Render(" " + valStr + " "))
				} else {
					b.WriteString(styles.LogText.Render(" " + valStr + " "))
				}
			} else if g.ChartKnownBases[r][c] {
				b.WriteString(styles.CommandText.Render(" >B< "))
			} else {
				b.WriteString(styles.Empty.Render(" ··· "))
			}
		}
	}

	// Line 11: Divider
	b.WriteByte('\n')
	b.WriteString(styles.GridHeader.Render(strings.Repeat("─", 43)))

	// Line 12: Mission Objectives (Klingons & Starbases)
	b.WriteByte('\n')
	klingonLabel := styles.GaugeLabel.Render("Klingons Left: ")
	klingonVal := styles.GaugeValue.Render(fmt.Sprintf("%2d", g.RemainingKlingons))
	baseLabel := styles.GaugeLabel.Render("    Starbases: ")
	baseVal := styles.GaugeValue.Render(fmt.Sprintf("%d", g.RemainingStarbases))
	b.WriteString(klingonLabel + klingonVal + baseLabel + baseVal)

	// Line 13: Exploration Progress & Casualties
	b.WriteByte('\n')
	explLabel := styles.GaugeLabel.Render("Explored: ")
	explVal := styles.GaugeValue.Render(fmt.Sprintf("%2d/64", explored))
	casLabel := styles.GaugeLabel.Render("        Casualties: ")
	casVal := styles.GaugeValue.Render(fmt.Sprintf("%d", g.Metrics.Casualties))
	b.WriteString(explLabel + explVal + casLabel + casVal)

	// Line 14: Action shortcuts
	b.WriteByte('\n')
	b.WriteString(styles.GaugeLabel.Render("[Ctrl+G] Interactive Map   [T] Target Lock"))

	return styles.Panel.Render(b.String())
}
