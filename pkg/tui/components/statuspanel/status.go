package statuspanel

import (
	"fmt"
	"math"
	"strings"

	"github.com/charmbracelet/lipgloss"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

// Model represents the telemetry and status panel component.
type Model struct {
	theme theme.Theme
}

// New creates a new status panel Model with the provided theme.
func New(th theme.Theme) Model {
	if th == nil {
		th = theme.DefaultTheme()
	}
	return Model{theme: th}
}

// SetTheme updates the active theme for status rendering.
func (m *Model) SetTheme(th theme.Theme) {
	if th == nil {
		th = theme.DefaultTheme()
	}
	m.theme = th
}

// Theme returns the currently active theme.
func (m Model) Theme() theme.Theme {
	if m.theme == nil {
		return theme.DefaultTheme()
	}
	return m.theme
}

type deviceEntry struct {
	id   engine.DeviceID
	name string
}

var devicesCol1 = []deviceEntry{
	{engine.DeviceWarp, "Warp"},
	{engine.DeviceSRSensors, "SRS"},
	{engine.DeviceLRSensors, "LRS"},
	{engine.DevicePhasers, "Phasers"},
}

var devicesCol2 = []deviceEntry{
	{engine.DevicePhotonTubes, "Tubes"},
	{engine.DeviceDamageControl, "Damage Control"},
	{engine.DeviceShields, "Shields"},
	{engine.DeviceComputer, "Computer"},
}

// View renders the status panel containing condition alert, telemetry progress bars,
// torpedo inventory, stardate/time remaining, subsystem countdowns, and surrounding quadrant radar.
func (m Model) View(g *engine.GameState) string {
	th := m.theme
	if th == nil {
		th = theme.DefaultTheme()
	}
	styles := th.Styles()

	if g == nil {
		return styles.Panel.Render(styles.GaugeLabel.Render("NO TELEMETRY AVAILABLE"))
	}

	// 1. Condition alert banner
	var condStr string
	var condStyle lipgloss.Style
	switch g.Enterprise.Condition {
	case engine.ConditionGreen:
		condStr = "CONDITION GREEN"
		condStyle = styles.ConditionGreen
	case engine.ConditionYellow:
		condStr = "CONDITION YELLOW"
		condStyle = styles.ConditionYellow
	case engine.ConditionRed:
		condStr = "CONDITION RED"
		condStyle = styles.ConditionRed
	case engine.ConditionDocked:
		condStr = "CONDITION DOCKED"
		condStyle = styles.ConditionDocked
	default:
		condStr = "CONDITION GREEN"
		condStyle = styles.ConditionGreen
	}
	condBanner := condStyle.Render(condStr)

	// 2. Stardate & Time remaining
	stardateStr := styles.GaugeLabel.Render("Stardate: ") +
		styles.GaugeValue.Render(fmt.Sprintf("%.1f", g.Stardate))
	timeStr := styles.GaugeLabel.Render("Time Remaining: ") +
		styles.GaugeValue.Render(fmt.Sprintf("%.1f", g.TimeRemaining))
	stardateTimeLine := stardateStr + "   " + timeStr

	// 3. Energy & Shields telemetry meters
	energyLine := renderProgressBar("Energy", g.Enterprise.Energy, 5000, styles)
	shieldsLine := renderProgressBar("Shields", g.Enterprise.Shields, 2500, styles)

	// 4. Torpedo inventory
	torpLine := styles.GaugeLabel.Render("Torpedoes: ") +
		styles.GaugeValue.Render(fmt.Sprintf("[TORP: %d/10]", g.Enterprise.Torpedoes))

	// 5. Subsystem device repair countdowns (2 columns of 4 devices)
	devHeader := styles.PanelTitle.Render("SUBSYSTEM REPAIR STATUS:")
	var devRows [4]string
	for i := 0; i < 4; i++ {
		d1 := devicesCol1[i]
		d2 := devicesCol2[i]

		var status1, status2 string
		if g.Enterprise.Devices[d1.id] > 0 {
			status1 = styles.SubsystemDamaged.Render(fmt.Sprintf("%4.1f", g.Enterprise.Devices[d1.id]))
		} else {
			status1 = styles.SubsystemNormal.Render("  OK")
		}

		if g.Enterprise.Devices[d2.id] > 0 {
			status2 = styles.SubsystemDamaged.Render(fmt.Sprintf("%4.1f", g.Enterprise.Devices[d2.id]))
		} else {
			status2 = styles.SubsystemNormal.Render("  OK")
		}

		col1Str := fmt.Sprintf("%-8s %s", d1.name+":", status1)
		col2Str := fmt.Sprintf("%-16s %s", d2.name+":", status2)
		devRows[i] = col1Str + "  " + col2Str
	}

	// 6. 3x3 surrounding quadrant radar box
	radarHeader := styles.PanelTitle.Render("RADAR (3x3 QUADRANTS):")
	qr := g.Enterprise.Quad[0]
	qc := g.Enterprise.Quad[1]

	var radarRows [3]string
	for dr := -1; dr <= 1; dr++ {
		var rowCells [3]string
		for dc := -1; dc <= 1; dc++ {
			r := qr + dr
			c := qc + dc
			idx := dc + 1
			if r < 1 || r > 8 || c < 1 || c > 8 {
				rowCells[idx] = styles.Empty.Render("***")
			} else {
				cellVal := fmt.Sprintf("%03d", g.GalaxyChart[r][c])
				if dr == 0 && dc == 0 {
					rowCells[idx] = styles.Enterprise.Render(cellVal)
				} else {
					rowCells[idx] = styles.GaugeLabel.Render(cellVal)
				}
			}
		}
		radarRows[dr+1] = "  " + rowCells[0] + "  " + rowCells[1] + "  " + rowCells[2]
	}

	var b strings.Builder
	b.WriteString(condBanner)
	b.WriteByte('\n')
	b.WriteString(stardateTimeLine)
	b.WriteByte('\n')
	b.WriteString(energyLine)
	b.WriteByte('\n')
	b.WriteString(shieldsLine)
	b.WriteByte('\n')
	b.WriteString(torpLine)
	b.WriteByte('\n')
	b.WriteString(devHeader)
	for _, devRow := range devRows {
		b.WriteByte('\n')
		b.WriteString(devRow)
	}
	b.WriteByte('\n')
	b.WriteString(radarHeader)
	for _, radarRow := range radarRows {
		b.WriteByte('\n')
		b.WriteString(radarRow)
	}

	return styles.Panel.Render(b.String())
}

// renderProgressBar formats a gauge label, filled/empty segmented bar, and numeric ratio readout.
func renderProgressBar(label string, current, maxVal float64, styles theme.Styles) string {
	ratio := 0.0
	if maxVal > 0 {
		ratio = current / maxVal
	}
	if ratio < 0 {
		ratio = 0
	} else if ratio > 1 {
		ratio = 1
	}

	const barWidth = 10
	filled := int(math.Round(ratio * float64(barWidth)))
	if filled < 0 {
		filled = 0
	} else if filled > barWidth {
		filled = barWidth
	}
	empty := barWidth - filled

	filledStr := styles.ProgressBarFilled.Render(strings.Repeat("█", filled))
	emptyStr := styles.ProgressBarEmpty.Render(strings.Repeat("░", empty))
	bar := styles.GaugeLabel.Render("[") + filledStr + emptyStr + styles.GaugeLabel.Render("]")

	lbl := styles.GaugeLabel.Render(fmt.Sprintf("%-9s", label+":"))
	val := styles.GaugeValue.Render(fmt.Sprintf("%4.0f/%.0f", current, maxVal))

	return fmt.Sprintf("%s %s %s", lbl, bar, val)
}
