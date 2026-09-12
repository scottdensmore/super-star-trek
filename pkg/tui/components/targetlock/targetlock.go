package targetlock

import (
	"fmt"
	"math"
	"sort"
	"strconv"
	"strings"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

// TargetInfo contains tactical telemetry and ballistics data for an enemy vessel.
type TargetInfo struct {
	KlingonID      int
	Coord          engine.Coord
	Distance       float64
	Bearing        float64
	HitProbability float64
	Power          float64
}

// FireTorpedoMsg is emitted when firing a torpedo at the locked target.
type FireTorpedoMsg struct {
	Target  engine.Coord
	Bearing float64
}

// FirePhasersMsg is emitted when firing phasers with the specified energy.
type FirePhasersMsg struct {
	Energy float64
}

// CloseHUDMsg is emitted when dismissing the tactical target lock HUD.
type CloseHUDMsg struct{}

// Model represents the Tactical Target Lock HUD component.
type Model struct {
	theme           theme.Theme
	targets         []TargetInfo
	targetIdx       int
	entSector       engine.Coord
	entEnergy       float64
	torpedoCount    int
	inputtingPhaser bool
	phaserInput     string
	warningMessage  string
}

// New creates a new Model with the provided theme.
func New(th theme.Theme) Model {
	if th == nil {
		th = theme.DefaultTheme()
	}
	return Model{
		theme:        th,
		torpedoCount: 10,
	}
}

// SetTheme updates the active styling theme.
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

// SetState updates ship status and populates telemetry for hostile targets,
// sorting them by distance from Enterprise and selecting initialTarget if provided.
func (m *Model) SetState(
	entSector engine.Coord,
	entEnergy float64,
	torpedoes int,
	klingons []*engine.Klingon,
	initialTarget engine.Coord,
) {
	m.entSector = entSector
	m.entEnergy = entEnergy
	m.torpedoCount = torpedoes
	m.inputtingPhaser = false
	m.phaserInput = ""
	m.warningMessage = ""
	m.targets = nil

	for _, k := range klingons {
		if k == nil {
			continue
		}
		dist := engine.Distance(entSector, k.Sector)
		rad := engine.Bearing(entSector, k.Sector)
		bearing := 1.0 + rad*4.0/math.Pi
		if bearing >= 9.0 {
			bearing -= 8.0
		}
		if bearing < 1.0 {
			bearing += 8.0
		}
		hitProb := math.Max(0.10, math.Min(0.95, 1.0-dist/15.0))

		m.targets = append(m.targets, TargetInfo{
			KlingonID:      k.ID,
			Coord:          k.Sector,
			Distance:       dist,
			Bearing:        bearing,
			HitProbability: hitProb,
			Power:          k.Energy,
		})
	}

	// Sort targets ascending by Distance, tie-breaking by KlingonID
	sort.Slice(m.targets, func(i, j int) bool {
		if math.Abs(m.targets[i].Distance-m.targets[j].Distance) > 1e-6 {
			return m.targets[i].Distance < m.targets[j].Distance
		}
		return m.targets[i].KlingonID < m.targets[j].KlingonID
	})

	m.targetIdx = 0
	if initialTarget != (engine.Coord{}) {
		for i, t := range m.targets {
			if t.Coord == initialTarget {
				m.targetIdx = i
				break
			}
		}
	}
}

// CurrentTarget returns telemetry for the active target, or nil if none.
func (m Model) CurrentTarget() *TargetInfo {
	if len(m.targets) == 0 || m.targetIdx < 0 || m.targetIdx >= len(m.targets) {
		return nil
	}
	return &m.targets[m.targetIdx]
}

// Targets returns a copy of all loaded target telemetry records.
func (m Model) Targets() []TargetInfo {
	if m.targets == nil {
		return nil
	}
	out := make([]TargetInfo, len(m.targets))
	copy(out, m.targets)
	return out
}

// WarningMessage returns the current warning message, if any.
func (m Model) WarningMessage() string {
	return m.warningMessage
}

// InputtingPhaser returns whether phaser energy input mode is active.
func (m Model) InputtingPhaser() bool {
	return m.inputtingPhaser
}

// PhaserInput returns the current text in the phaser energy input prompt.
func (m Model) PhaserInput() string {
	return m.phaserInput
}

// TargetIndex returns the active target index.
func (m Model) TargetIndex() int {
	return m.targetIdx
}

// Update handles keyboard interactions: firing torpedoes, phaser prompt input,
// cycling targets, and dismissing the HUD.
func (m Model) Update(msg tea.Msg) (Model, tea.Cmd) {
	keyMsg, ok := msg.(tea.KeyMsg)
	if !ok {
		return m, nil
	}

	keyStr := keyMsg.String()

	// Esc always cancels HUD
	if keyMsg.Type == tea.KeyEsc || keyStr == "esc" {
		return m, func() tea.Msg { return CloseHUDMsg{} }
	}

	// 'P' toggles phaser energy input mode
	if keyStr == "p" || keyStr == "P" {
		m.inputtingPhaser = !m.inputtingPhaser
		m.phaserInput = ""
		m.warningMessage = ""
		return m, nil
	}

	if m.inputtingPhaser {
		switch {
		case keyMsg.Type == tea.KeyEnter || keyStr == "enter":
			val := strings.TrimSpace(m.phaserInput)
			energy, err := strconv.ParseFloat(val, 64)
			if err == nil && energy > 0 {
				m.inputtingPhaser = false
				m.phaserInput = ""
				m.warningMessage = ""
				return m, func() tea.Msg {
					return FirePhasersMsg{Energy: energy}
				}
			}
			m.warningMessage = "*** INVALID ENERGY AMOUNT ***"
			return m, nil
		case keyMsg.Type == tea.KeyBackspace || keyStr == "backspace":
			m.warningMessage = ""
			if len(m.phaserInput) > 0 {
				m.phaserInput = m.phaserInput[:len(m.phaserInput)-1]
			}
			return m, nil
		default:
			if (len(keyStr) == 1 && keyStr[0] >= '0' && keyStr[0] <= '9') || keyStr == "." {
				m.warningMessage = ""
				m.phaserInput += keyStr
			}
			return m, nil
		}
	}

	// Target cycling and firing controls
	switch {
	case keyMsg.Type == tea.KeyTab || keyMsg.Type == tea.KeyRight || keyStr == "tab" || keyStr == "right":
		if len(m.targets) > 0 {
			m.targetIdx = (m.targetIdx + 1) % len(m.targets)
			m.warningMessage = ""
		}
		return m, nil

	case keyMsg.Type == tea.KeyShiftTab || keyMsg.Type == tea.KeyLeft || keyStr == "shift+tab" || keyStr == "left":
		if len(m.targets) > 0 {
			m.targetIdx = (m.targetIdx - 1 + len(m.targets)) % len(m.targets)
			m.warningMessage = ""
		}
		return m, nil

	case keyMsg.Type == tea.KeyEnter || keyStr == "enter":
		target := m.CurrentTarget()
		if target == nil {
			return m, nil
		}
		if m.torpedoCount <= 0 {
			m.warningMessage = "*** NO TORPEDOES REMAINING ***"
			return m, nil
		}
		cur := *target
		return m, func() tea.Msg {
			return FireTorpedoMsg{
				Target:  cur.Coord,
				Bearing: cur.Bearing,
			}
		}
	}

	return m, nil
}

// View renders the 52x12 tactical targeting computer panel.
func (m Model) View() string {
	th := m.theme
	if th == nil {
		th = theme.DefaultTheme()
	}
	styles := th.Styles()

	const targetWidth = 52
	const targetHeight = 12

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

	widthNoBorders := targetWidth - borderH
	heightNoBorders := targetHeight - borderV

	target := m.CurrentTarget()

	// Line 1: Centered Title
	title := styles.PanelTitle.Render("TACTICAL TARGET LOCK")
	line1 := lipgloss.NewStyle().Width(innerWidth).Align(lipgloss.Center).Render(title)

	// Line 2: Target Identifier
	var line2 string
	if target != nil {
		line2 = styles.Klingon.Render(fmt.Sprintf("Target: KLINGON BATTLECRUISER #%d", target.KlingonID))
	} else {
		line2 = styles.GaugeLabel.Render("Target: NO HOSTILE VESSELS DETECTED")
	}

	// Line 3: Target Position
	var line3 string
	if target != nil {
		line3 = styles.GaugeLabel.Render(fmt.Sprintf("Position: Sector [%d, %d]", target.Coord[0], target.Coord[1]))
	} else {
		line3 = styles.GaugeLabel.Render("Position: Sector [--, --]")
	}

	// Line 4: Range and Bearing
	var line4 string
	if target != nil {
		dir := directionLabel(target.Bearing)
		line4 = styles.GaugeLabel.Render(fmt.Sprintf("Range: %.2f sectors  Bearing: %.2f (%s)", target.Distance, target.Bearing, dir))
	} else {
		line4 = styles.GaugeLabel.Render("Range: --             Bearing: --")
	}

	// Line 5: Hit Probability and Target Shielding/Power
	var line5 string
	if target != nil {
		line5 = styles.GaugeLabel.Render(fmt.Sprintf("Hit Prob: %.0f%%     Target Shielding: ~%.0f units", target.HitProbability*100, target.Power))
	} else {
		line5 = styles.GaugeLabel.Render("Hit Prob: --        Target Shielding: --")
	}

	// Line 6: Horizontal Divider
	line6 := styles.GridHeader.Render(strings.Repeat("─", innerWidth))

	// Line 7: Ship Readiness
	line7 := styles.GaugeValue.Render(fmt.Sprintf("Enterprise Weapons: [TORP: %d/10] [ENERGY: %.0f]", m.torpedoCount, m.entEnergy))

	// Line 8: Warning or Phaser Prompt
	var line8 string
	if m.warningMessage != "" {
		line8 = styles.ConditionRed.Render(m.warningMessage)
	} else if m.inputtingPhaser {
		line8 = styles.Prompt.Render("PHASER ENERGY> ") + styles.CommandText.Render(m.phaserInput+"█")
	} else {
		line8 = ""
	}

	// Line 9: Action Legend
	line9 := styles.GaugeLabel.Render("[Enter] Fire Torpedo [P] Phaser [Tab] Next [Esc]")

	// Line 10: Padding
	line10 := ""

	content := strings.Join([]string{
		line1,
		line2,
		line3,
		line4,
		line5,
		line6,
		line7,
		line8,
		line9,
		line10,
	}, "\n")

	return panelStyle.Width(widthNoBorders).Height(heightNoBorders).Render(content)
}

// directionLabel returns compass direction abbreviation for Super Star Trek bearings (1.0..9.0).
func directionLabel(bearing float64) string {
	b := math.Mod(bearing-0.5, 8.0)
	if b < 0 {
		b += 8.0
	}
	switch int(b) {
	case 0:
		return "E"
	case 1:
		return "NE"
	case 2:
		return "N"
	case 3:
		return "NW"
	case 4:
		return "W"
	case 5:
		return "SW"
	case 6:
		return "S"
	case 7:
		return "SE"
	default:
		return "E"
	}
}
