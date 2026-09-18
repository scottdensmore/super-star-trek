package statuspanel

import (
	"fmt"
	"math"
	"strings"

	"github.com/charmbracelet/lipgloss"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/anim"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

// PanelData encapsulates the telemetry and configuration state needed to render the status panel.
type PanelData struct {
	Enterprise    engine.Enterprise
	Devices       [8]float64
	TimeRemaining float64
	IsDocked      bool
	GalaxyChart   [9][9]int
	Stardate      float64
	Rules         engine.GameRules
}

// SamplePanelData returns a default populated PanelData for testing and preview rendering.
func SamplePanelData() PanelData {
	return PanelData{
		Enterprise: engine.Enterprise{
			Quad:      engine.Coord{4, 4},
			Sector:    engine.Coord{2, 3},
			Energy:    5000,
			Shields:   1000,
			Torpedoes: 10,
			Condition: engine.ConditionGreen,
		},
		Devices:       [8]float64{},
		TimeRemaining: 30.0,
		IsDocked:      false,
		GalaxyChart:   [9][9]int{},
		Stardate:      2800.0,
		Rules:         engine.DefaultRulesForProfile(engine.ProfileNormal),
	}
}

// Model represents the telemetry and status panel component.
type Model struct {
	theme         theme.Theme
	width         int
	height        int
	enterprise    engine.Enterprise
	timeRemaining float64
	isDocked      bool
	galaxyChart   [9][9]int
	stardate      float64
	rules         engine.GameRules
	hasState      bool
	redAlertCycle int
}

// SetRedAlertCycle updates the active cycle index for Condition Red klaxon pulse oscillation.
func (m *Model) SetRedAlertCycle(cycle int) {
	m.redAlertCycle = cycle
}

type panelStyles struct {
	theme.Styles
	TextWarn  lipgloss.Style
	TextMuted lipgloss.Style
	Normal    lipgloss.Style
}

func getPanelStyles(th theme.Theme) panelStyles {
	if th == nil {
		th = theme.DefaultTheme()
	}
	base := th.Styles()
	return panelStyles{
		Styles:    base,
		TextWarn:  base.SubsystemDamaged,
		TextMuted: base.Empty,
		Normal:    base.GaugeLabel,
	}
}

// New creates a new status panel Model with the provided theme and optional dimensions.
func New(th theme.Theme, dims ...int) Model {
	if th == nil {
		th = theme.DefaultTheme()
	}
	width := 0
	height := 0
	if len(dims) > 0 {
		width = dims[0]
	}
	if len(dims) > 1 {
		height = dims[1]
	}
	return Model{
		theme:  th,
		width:  width,
		height: height,
	}
}

// SetState updates the panel state from raw simulation parameters.
func (m *Model) SetState(ent engine.Enterprise, timeRemaining float64, klingonsLeft int, starbasesLeft int, isDocked bool, chart [9][9]int, rules ...engine.GameRules) {
	m.enterprise = ent
	m.timeRemaining = timeRemaining
	m.isDocked = isDocked
	m.galaxyChart = chart
	if len(rules) > 0 {
		m.rules = rules[0]
	} else if m.rules.Profile == "" {
		m.rules = engine.DefaultRulesForProfile(engine.ProfileNormal)
	}
	m.hasState = true
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

func (m Model) renderRadar(styles panelStyles) (string, [3]string) {
	qr := m.enterprise.Quad[0]
	qc := m.enterprise.Quad[1]

	lrsSensor := m.enterprise.Devices[engine.DeviceLRSensors]
	sensorDegradation := m.rules.SensorDegradation
	if m.rules.Profile == "" && !m.rules.SensorDegradation {
		sensorDegradation = true
	}

	isOffline := false
	isDegraded := false

	if !m.isDocked {
		if sensorDegradation {
			if lrsSensor >= 2.0 {
				isOffline = true
			} else if lrsSensor > 0 {
				isDegraded = true
			}
		} else {
			if lrsSensor >= 2.0 {
				isOffline = true
			}
		}
	}

	radarTitle := styles.TextMuted.Render("RADAR (±1) [K-B-S]:")
	if isOffline {
		radarTitle = styles.TextWarn.Render("RADAR (±1) [LRS OFFLINE]:")
	} else if isDegraded {
		radarTitle = styles.TextWarn.Render("RADAR (±1) [LRS DEGRADED]:")
	}

	radarHeader := radarTitle
	if !isOffline && !isDegraded {
		radarHeader += fmt.Sprintf("  %-4s %-4s %-4s", radarColHeader(qc-1), radarColHeader(qc), radarColHeader(qc+1))
	}

	var radarRows [3]string
	for dr := -1; dr <= 1; dr++ {
		r := qr + dr
		var rowCells [3]string
		for dc := -1; dc <= 1; dc++ {
			c := qc + dc
			idx := dc + 1
			if r < 1 || r > 8 || c < 1 || c > 8 {
				rowCells[idx] = styles.TextMuted.Render(" *** ")
			} else {
				val := m.galaxyChart[r][c]
				if dr == 0 && dc == 0 {
					// Enterprise quadrant remains visible via short range sensors
					rowCells[idx] = styles.Enterprise.Render(fmt.Sprintf("<%03d>", val))
				} else if isOffline {
					rowCells[idx] = styles.TextWarn.Render(" ??? ")
				} else if isDegraded {
					rowCells[idx] = styles.TextWarn.Render("  ?  ")
				} else {
					rowCells[idx] = styles.Normal.Render(fmt.Sprintf(" %03d ", val))
				}
			}
		}
		rowHdr := radarRowHeader(r)
		radarRows[dr+1] = fmt.Sprintf("  %s  %s%s%s", rowHdr, rowCells[0], rowCells[1], rowCells[2])
	}

	return radarHeader, radarRows
}

// Render renders the status panel for the provided PanelData.
func (m Model) Render(data PanelData) string {
	devices := data.Devices
	allZero := true
	for _, v := range devices {
		if v != 0 {
			allZero = false
			break
		}
	}
	if allZero {
		devices = data.Enterprise.Devices
	}
	ent := data.Enterprise
	ent.Devices = devices

	m.enterprise = ent
	m.timeRemaining = data.TimeRemaining
	m.isDocked = data.IsDocked
	m.galaxyChart = data.GalaxyChart
	m.stardate = data.Stardate
	m.rules = data.Rules
	if m.rules.Profile == "" && !m.rules.SensorDegradation {
		m.rules = engine.DefaultRulesForProfile(engine.ProfileNormal)
	}
	m.hasState = true
	return m.render()
}

// View renders the status panel containing condition alert, telemetry progress bars,
// torpedo inventory, stardate/time remaining, subsystem countdowns, and surrounding quadrant radar.
func (m Model) View(gs ...*engine.GameState) string {
	if len(gs) > 0 && gs[0] != nil {
		g := gs[0]
		return m.Render(PanelData{
			Enterprise:    g.Enterprise,
			Devices:       g.Enterprise.Devices,
			TimeRemaining: g.TimeRemaining,
			IsDocked:      (g.Enterprise.Condition == engine.ConditionDocked),
			GalaxyChart:   g.GalaxyChart,
			Stardate:      g.Stardate,
			Rules:         g.Rules,
		})
	}
	if !m.hasState {
		styles := getPanelStyles(m.theme)
		return styles.Panel.Render(styles.GaugeLabel.Render("NO TELEMETRY AVAILABLE"))
	}
	return m.render()
}

func (m Model) render() string {
	styles := getPanelStyles(m.theme)

	// 1. Condition alert banner & Location readout
	var condStr string
	var condStyle lipgloss.Style
	switch m.enterprise.Condition {
	case engine.ConditionGreen:
		condStr = "CONDITION GREEN"
		condStyle = styles.ConditionGreen
	case engine.ConditionYellow:
		condStr = "CONDITION YELLOW"
		condStyle = styles.ConditionYellow
	case engine.ConditionRed:
		condStr = "CONDITION RED"
		if m.rules.AnimSpeed != engine.AnimSpeedOff {
			condStyle = anim.RedAlertBadgeStyle(m.redAlertCycle)
		} else {
			condStyle = styles.ConditionRed
		}
	case engine.ConditionDocked:
		condStr = "CONDITION DOCKED"
		condStyle = styles.ConditionDocked
	default:
		condStr = "CONDITION GREEN"
		condStyle = styles.ConditionGreen
	}
	condBanner := condStyle.Render(condStr)
	condLocLine := condBanner + "   " + styles.GaugeLabel.Render("LOC: ") + styles.Prompt.Render(fmt.Sprintf("Q[%d,%d] S[%d,%d]", m.enterprise.Quad[0], m.enterprise.Quad[1], m.enterprise.Sector[0], m.enterprise.Sector[1]))

	// 3. Stardate & Time remaining
	stardateStr := styles.GaugeLabel.Render("Stardate: ") +
		styles.GaugeValue.Render(fmt.Sprintf("%.1f", m.stardate))
	timeStr := styles.GaugeLabel.Render("Time Remaining: ") +
		styles.GaugeValue.Render(fmt.Sprintf("%.1f", m.timeRemaining))
	stardateTimeLine := stardateStr + "   " + timeStr

	// 4. Energy & Shields telemetry meters
	energyLine := renderProgressBar("Energy", m.enterprise.Energy, 5000, styles.Styles)
	shieldsLine := renderProgressBar("Shields", m.enterprise.Shields, 2500, styles.Styles)

	// 5. Torpedo inventory
	torpLine := styles.GaugeLabel.Render("Torpedoes: ") +
		styles.GaugeValue.Render(fmt.Sprintf("[TORP: %d/10]", m.enterprise.Torpedoes))

	// 6. Subsystem device repair countdowns (2 columns of 4 devices)
	devHeader := styles.PanelTitle.Render("SUBSYSTEM REPAIR STATUS:")
	var devRows [4]string
	for i := 0; i < 4; i++ {
		d1 := devicesCol1[i]
		d2 := devicesCol2[i]

		var status1, status2 string
		if m.enterprise.Devices[d1.id] > 0 {
			status1 = styles.SubsystemDamaged.Render(fmt.Sprintf("%4.1f", m.enterprise.Devices[d1.id]))
		} else {
			status1 = styles.SubsystemNormal.Render("  OK")
		}

		if m.enterprise.Devices[d2.id] > 0 {
			status2 = styles.SubsystemDamaged.Render(fmt.Sprintf("%4.1f", m.enterprise.Devices[d2.id]))
		} else {
			status2 = styles.SubsystemNormal.Render("  OK")
		}

		col1Str := fmt.Sprintf("%-8s %s", d1.name+":", status1)
		col2Str := fmt.Sprintf("%-16s %s", d2.name+":", status2)
		devRows[i] = col1Str + "  " + col2Str
	}

	// 7. 3x3 surrounding quadrant radar box
	radarHeader, radarRows := m.renderRadar(styles)

	var b strings.Builder
	b.Grow(512)
	b.WriteString(condLocLine)
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

func radarColHeader(c int) string {
	if c < 1 || c > 8 {
		return " "
	}
	return fmt.Sprintf("%d", c)
}

func radarRowHeader(r int) string {
	if r < 1 || r > 8 {
		return " "
	}
	return fmt.Sprintf("%d", r)
}
