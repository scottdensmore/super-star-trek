package galacticchart

import (
	"fmt"
	"strings"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

// WarpToQuadrantMsg is emitted when the user confirms warping to a destination quadrant.
type WarpToQuadrantMsg struct {
	DestQuad engine.Coord
	Warp     float64
}

// WarpBlockedMsg is emitted when quick-warp is attempted with a damaged library computer.
type WarpBlockedMsg struct {
	Reason string
}

// CloseChartMsg is emitted when dismissing the galactic chart modal without warping.
type CloseChartMsg struct{}

// Model represents the interactive galactic star chart modal.
type Model struct {
	theme           theme.Theme
	cursor          engine.Coord
	enterpriseQuad  engine.Coord
	galaxyChart     [9][9]int
	chartDiscovered [9][9]bool
	knownBases      [9][9]bool
	computerDamaged bool
	showHelp        bool
	width           int
	height          int
}

// New creates a new galactic chart model with default dimensions (64x18).
func New(th theme.Theme, width, height int) Model {
	if th == nil {
		th = theme.DefaultTheme()
	}
	if width <= 0 {
		width = 64
	}
	if height <= 0 {
		height = 18
	}
	return Model{
		theme:          th,
		cursor:         engine.Coord{1, 1},
		enterpriseQuad: engine.Coord{1, 1},
		width:          width,
		height:         height,
	}
}

// Init initializes the component commands.
func (m Model) Init() tea.Cmd {
	return nil
}

// SetTheme updates the component theme.
func (m *Model) SetTheme(th theme.Theme) {
	if th == nil {
		th = theme.DefaultTheme()
	}
	m.theme = th
}

// Theme returns the current component theme.
func (m Model) Theme() theme.Theme {
	return m.theme
}

// SetSize updates the modal container dimensions.
func (m *Model) SetSize(w, h int) {
	m.width = w
	m.height = h
}

// SetState updates Enterprise position, chart data, and discovered flags,
// and resets cursor position to the Enterprise quadrant.
func (m *Model) SetState(entQuad engine.Coord, chart [9][9]int, discovered [9][9]bool, knownBases [9][9]bool, computerDamaged bool) {
	m.enterpriseQuad = entQuad
	m.enterpriseQuad[0] = clampCoord(m.enterpriseQuad[0], 1, 8)
	m.enterpriseQuad[1] = clampCoord(m.enterpriseQuad[1], 1, 8)
	m.galaxyChart = chart
	m.chartDiscovered = discovered
	m.knownBases = knownBases
	m.computerDamaged = computerDamaged
	m.showHelp = false
	m.cursor = m.enterpriseQuad
	m.cursor[0] = clampCoord(m.cursor[0], 1, 8)
	m.cursor[1] = clampCoord(m.cursor[1], 1, 8)
}

// Cursor returns the active cursor coordinates.
func (m Model) Cursor() engine.Coord {
	return m.cursor
}

// ShowingHelp returns whether the chart is currently displaying the help guide.
func (m Model) ShowingHelp() bool {
	return m.showHelp
}

// SetCursor sets the active cursor coordinates (clamped between 1 and 8).
func (m *Model) SetCursor(c engine.Coord) {
	m.cursor[0] = clampCoord(c[0], 1, 8)
	m.cursor[1] = clampCoord(c[1], 1, 8)
}

// Width returns the modal width.
func (m Model) Width() int {
	return m.width
}

// Height returns the modal height.
func (m Model) Height() int {
	return m.height
}

// ComputerDamaged returns whether the ship's library computer is damaged.
func (m Model) ComputerDamaged() bool {
	return m.computerDamaged
}

// HitTest maps relative (x, y) coordinates within the modal's bounding box
// to a 1-indexed engine.Coord [Row, Col] representing a quadrant.
func (m Model) HitTest(relX, relY int) (engine.Coord, bool) {
	if m.showHelp {
		return engine.Coord{}, false
	}
	// Quadrant rows 1..8 occupy lines 4..11
	if relY < 4 || relY > 11 {
		return engine.Coord{}, false
	}
	r := relY - 3

	// Col 0 is border, col 1 is padding, cols 2..6 are row label.
	// Quadrant columns start at col 7 with 7-char stride (5 chars cell + 2 chars separator).
	if relX < 7 {
		return engine.Coord{}, false
	}
	offset := relX - 7
	c := (offset / 7) + 1
	charWithinCell := offset % 7
	if c < 1 || c > 8 || charWithinCell >= 5 {
		return engine.Coord{}, false
	}
	return engine.Coord{r, c}, true
}

// SetShowingHelp sets whether the chart displays the help guide.
func (m *Model) SetShowingHelp(s bool) {
	m.showHelp = s
}

// Update handles directional navigation, Enter warp dispatch, Esc close,
// and ignores WindowSizeMsg to maintain fixed modal geometry.
func (m Model) Update(msg tea.Msg) (Model, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.WindowSizeMsg:
		// Safely ignore WindowSizeMsg to preserve fixed modal dimensions.
		return m, nil

	case tea.KeyMsg:
		if m.showHelp {
			switch {
			case msg.Type == tea.KeyEsc || msg.String() == "esc" || msg.String() == "?" || msg.String() == "q" || msg.String() == "Q":
				m.showHelp = false
				return m, nil
			case msg.Type == tea.KeyEnter || msg.String() == "enter":
				m.showHelp = false
				if m.computerDamaged {
					return m, func() tea.Msg {
						return WarpBlockedMsg{
							Reason: "COMPUTER DAMAGED, USE A POCKET CALCULATOR. Manual navigation required (nav q <r> <c> [warp]).",
						}
					}
				}
				dest := m.cursor
				telem := CalculateTelemetry(m.enterpriseQuad, dest)
				return m, func() tea.Msg {
					return WarpToQuadrantMsg{DestQuad: dest, Warp: telem.RecommendedWarp}
				}
			default:
				m.showHelp = false
			}
		}

		switch {
		case msg.String() == "?":
			m.showHelp = true
			return m, nil

		case msg.Type == tea.KeyUp || msg.String() == "k" || msg.String() == "K" || msg.String() == "up":
			if m.cursor[0] > 1 {
				m.cursor[0]--
			}
			return m, nil

		case msg.Type == tea.KeyDown || msg.String() == "j" || msg.String() == "J" || msg.String() == "down":
			if m.cursor[0] < 8 {
				m.cursor[0]++
			}
			return m, nil

		case msg.Type == tea.KeyLeft || msg.String() == "h" || msg.String() == "H" || msg.String() == "left":
			if m.cursor[1] > 1 {
				m.cursor[1]--
			}
			return m, nil

		case msg.Type == tea.KeyRight || msg.String() == "l" || msg.String() == "L" || msg.String() == "right":
			if m.cursor[1] < 8 {
				m.cursor[1]++
			}
			return m, nil

		case msg.Type == tea.KeyEnter || msg.String() == "enter":
			if m.computerDamaged {
				return m, func() tea.Msg {
					return WarpBlockedMsg{
						Reason: "COMPUTER DAMAGED, USE A POCKET CALCULATOR. Manual navigation required (nav q <r> <c> [warp]).",
					}
				}
			}
			dest := m.cursor
			telem := CalculateTelemetry(m.enterpriseQuad, dest)
			return m, func() tea.Msg {
				return WarpToQuadrantMsg{DestQuad: dest, Warp: telem.RecommendedWarp}
			}

		case msg.Type == tea.KeyEsc || msg.String() == "esc":
			return m, func() tea.Msg {
				return CloseChartMsg{}
			}
		}
	}

	return m, nil
}

type chartStyles struct {
	theme.Styles
	TextWarn  lipgloss.Style
	TextMuted lipgloss.Style
}

func getChartStyles(th theme.Theme) chartStyles {
	if th == nil {
		th = theme.DefaultTheme()
	}
	base := th.Styles()
	return chartStyles{
		Styles:    base,
		TextWarn:  base.SubsystemDamaged,
		TextMuted: base.Empty,
	}
}

func (m Model) renderFooter(styles chartStyles) (string, string) {
	var lineTelem1, lineTelem2 string
	if m.computerDamaged {
		targetStr := styles.GaugeLabel.Render("Target: ") + styles.Prompt.Render(fmt.Sprintf("Quad [%d, %d]", m.cursor[0], m.cursor[1]))
		distanceStr := styles.TextWarn.Render("Distance: [CALC OFFLINE]")
		lineTelem1 = targetStr + styles.GaugeLabel.Render("  •  ") + distanceStr
		lineTelem2 = styles.TextWarn.Render("Course: [CALC OFFLINE] • Warp: [CALC OFFLINE]")
	} else {
		telem := CalculateTelemetry(m.enterpriseQuad, m.cursor)
		if telem.IsCurrent {
			lineTelem1 = styles.GaugeLabel.Render("Target: ") +
				styles.Prompt.Render(fmt.Sprintf("Quad [%d, %d]", telem.To[0], telem.To[1])) +
				styles.GaugeLabel.Render("  •  Current Position")
			lineTelem2 = styles.GaugeLabel.Render("Course: --  •  Warp: --")
		} else {
			lineTelem1 = styles.GaugeLabel.Render("Target: ") +
				styles.Prompt.Render(fmt.Sprintf("Quad [%d, %d]", telem.To[0], telem.To[1])) +
				styles.GaugeLabel.Render(fmt.Sprintf("  •  Dist: %.1f quads (ΔR: %s, ΔC: %s)",
					telem.Distance, formatDelta(telem.DeltaR), formatDelta(telem.DeltaC)))
			lineTelem2 = styles.GaugeLabel.Render(fmt.Sprintf("Course: %.2f rad (%s)  •  Warp: %.1f",
				telem.Course, telem.Direction, telem.RecommendedWarp))
		}
	}
	return lineTelem1, lineTelem2
}

// View renders the 64x18 galactic star chart dialog with 8x8 grid and navigation telemetry.
func (m Model) View() string {
	styles := getChartStyles(m.theme)

	targetWidth := m.width
	if targetWidth <= 0 {
		targetWidth = 64
	}
	targetHeight := m.height
	if targetHeight <= 0 {
		targetHeight = 18
	}

	panelStyle := styles.Panel.Border(styles.Border, true)
	borderH := panelStyle.GetHorizontalBorderSize()
	if borderH == 0 {
		borderH = 2
	}
	paddingH := panelStyle.GetHorizontalPadding()
	borderV := panelStyle.GetVerticalBorderSize()
	if borderV == 0 {
		borderV = 2
	}
	paddingV := panelStyle.GetVerticalPadding()

	innerWidth := targetWidth - borderH - paddingH
	if innerWidth < 10 {
		innerWidth = 10
	}
	innerHeight := targetHeight - borderV - paddingV
	if innerHeight < 1 {
		innerHeight = 1
	}

	// In Lip Gloss, Style.Width(w) and Style.Height(h) define the container dimensions
	// excluding outer borders, but including padding (similar to CSS padding-box).
	// Therefore, widthNoBorders = targetWidth - borderH and heightNoBorders = targetHeight - borderV
	// ensure the final rendered container with outer borders matches targetWidth (64) and targetHeight (18).
	// innerWidth = targetWidth - borderH - paddingH defines the inner content width between left/right padding.
	widthNoBorders := targetWidth - borderH
	heightNoBorders := targetHeight - borderV

	if m.showHelp {
		return m.renderHelpView(styles, panelStyle, innerWidth, widthNoBorders, heightNoBorders)
	}

	// Line 1: Centered title banner
	title := styles.PanelTitle.Render("GALACTIC STAR CHART")
	lineTitle := lipgloss.NewStyle().Width(innerWidth).Align(lipgloss.Center).Render(title)

	// Line 2: Empty spacer
	lineSpacer1 := ""

	// Line 3: Column numbers header (1..8) with 7-character stride matching 5-char cell + 2-space separator
	var colHeaderBuilder strings.Builder
	colHeaderBuilder.WriteString("     ")
	for col := 1; col <= 8; col++ {
		if col < 8 {
			colHeaderBuilder.WriteString(fmt.Sprintf("  %d    ", col))
		} else {
			colHeaderBuilder.WriteString(fmt.Sprintf("  %d  ", col))
		}
	}
	lineColHeader := styles.GridHeader.Render(colHeaderBuilder.String())

	// Lines 4-11: Quadrant grid rows (1..8)
	var gridRows []string
	for row := 1; row <= 8; row++ {
		var rowBuilder strings.Builder
		rowLabel := styles.GridHeader.Render(fmt.Sprintf("  %d  ", row))
		rowBuilder.WriteString(rowLabel)

		for col := 1; col <= 8; col++ {
			valStr := "···"
			if m.chartDiscovered[row][col] {
				valStr = fmt.Sprintf("%03d", m.galaxyChart[row][col])
			} else if m.knownBases[row][col] {
				valStr = ".1."
			}

			isEnt := (row == m.enterpriseQuad[0] && col == m.enterpriseQuad[1])
			isCur := (row == m.cursor[0] && col == m.cursor[1])

			var cell string
			if isCur {
				bracketCell := "[" + valStr + "]"
				if isEnt {
					cell = styles.Enterprise.Render(bracketCell)
				} else {
					cell = styles.CommandText.Render(bracketCell)
				}
			} else if isEnt {
				angleCell := "<" + valStr + ">"
				cell = styles.Enterprise.Render(angleCell)
			} else {
				spaceCell := " " + valStr + " "
				cell = styles.LogText.Render(spaceCell)
			}

			rowBuilder.WriteString(cell)
			if col < 8 {
				rowBuilder.WriteString("  ")
			}
		}
		gridRows = append(gridRows, rowBuilder.String())
	}

	// Line 12: Horizontal divider
	lineDivider := styles.GridHeader.Render(strings.Repeat("─", innerWidth))

	// Lines 13-14: Navigation telemetry
	lineTelem1, lineTelem2 := m.renderFooter(styles)

	// Line 15: Notation legend
	lineLegend := styles.TextMuted.Render("Legend: ··· Unexplored • .1. Base • KBS (Klingon/Base/Star)")

	// Line 16: Action controls hint
	var lineHint string
	if m.computerDamaged {
		lineHint = styles.TextMuted.Render("[Enter] Disabled (Comp Offline)  [?] Help  [Esc] Close")
	} else {
		lineHint = styles.LogText.Render("[Enter] Warp  [?] Help  [Arrows/HJKL] Move  [Esc] Close")
	}

	contentLines := []string{
		lineTitle,
		lineSpacer1,
		lineColHeader,
		gridRows[0],
		gridRows[1],
		gridRows[2],
		gridRows[3],
		gridRows[4],
		gridRows[5],
		gridRows[6],
		gridRows[7],
		lineDivider,
		lineTelem1,
		lineTelem2,
		lineLegend,
		lineHint,
	}

	content := strings.Join(contentLines, "\n")
	return panelStyle.Width(widthNoBorders).Height(heightNoBorders).Render(content)
}

// renderHelpView renders the 16-line in-modal help guide for the Galactic Star Chart.
func (m Model) renderHelpView(styles chartStyles, panelStyle lipgloss.Style, innerWidth, widthNoBorders, heightNoBorders int) string {
	title := styles.PanelTitle.Render("GALACTIC STAR CHART GUIDE")
	lineTitle := lipgloss.NewStyle().Width(innerWidth).Align(lipgloss.Center).Render(title)
	lineDivider := styles.GridHeader.Render(strings.Repeat("─", innerWidth))

	helpLines := []string{
		lineTitle,
		"",
		styles.LogText.Render("The 8x8 chart maps the 64 quadrants of the galaxy:"),
		"",
		styles.CommandText.Render("  ··· ") + styles.LogText.Render(" Unexplored quadrant (not yet scanned by sensors)"),
		styles.CommandText.Render("  .1. ") + styles.LogText.Render(" Starbase detected via Starfleet Command report"),
		styles.CommandText.Render("  KBS ") + styles.LogText.Render(" 3-digit quadrant sensor scan:"),
		styles.LogText.Render("        • K (hundreds) = Klingon battlecruisers"),
		styles.LogText.Render("        • B (tens)     = Federation starbases"),
		styles.LogText.Render("        • S (units)    = Stars"),
		"",
		styles.Enterprise.Render("  <···>") + styles.LogText.Render(" Enterprise current position"),
		styles.CommandText.Render("  [···]") + styles.LogText.Render(" Targeted destination reticle"),
		lineDivider,
		styles.GaugeLabel.Render("Telemetry displays course, distance, and recommended warp."),
		styles.TextMuted.Render("[?] / [Esc] Return to chart  •  [Enter] Warp to target"),
	}

	content := strings.Join(helpLines, "\n")
	return panelStyle.Width(widthNoBorders).Height(heightNoBorders).Render(content)
}

func formatDelta(d int) string {
	if d > 0 {
		return fmt.Sprintf("+%d", d)
	}
	return fmt.Sprintf("%d", d)
}

func clampCoord(val, min, max int) int {
	if val < min {
		return min
	}
	if val > max {
		return max
	}
	return val
}
