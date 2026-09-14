package damageschematic

import (
	"fmt"
	"strings"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/charmbracelet/x/ansi"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

type deviceMeta struct {
	id     engine.DeviceID
	tag    string
	name   string
	impact string
}

var devices = []deviceMeta{
	{engine.DeviceWarp, "WARP", "Warp Engines", "Max Warp 0.2"},
	{engine.DeviceSRSensors, "SRS", "Short-Range Sens", "SRS Offline"},
	{engine.DeviceLRSensors, "LRS", "Long-Range Sens", "LRS Offline"},
	{engine.DevicePhasers, "PHAS", "Phaser Controls", "Phasers Locked"},
	{engine.DevicePhotonTubes, "TUB", "Photon Tubes", "Tubes Locked"},
	{engine.DeviceDamageControl, "DAM", "Damage Control", "Repairs Slow"},
	{engine.DeviceShields, "SHL", "Shield System", "Shields Locked"},
	{engine.DeviceComputer, "COMP", "Library Computer", "No Chart/Nav"},
}

// Model represents the Damage Control Schematic modal component.
type Model struct {
	width      int
	height     int
	theme      theme.Theme
	devices    [engine.NumDevices]float64
	condition  string
	isDocked   bool
	repairMult float64
}

type schematicStyles struct {
	theme.Styles
	Border   lipgloss.Style
	TextWarn lipgloss.Style
}

func getSchematicStyles(th theme.Theme) schematicStyles {
	if th == nil {
		th = theme.DefaultTheme()
	}
	base := th.Styles()
	borderFg := base.Panel.GetBorderTopForeground()
	bStyle := lipgloss.NewStyle()
	if borderFg != nil {
		bStyle = bStyle.Foreground(borderFg)
	}
	return schematicStyles{
		Styles:   base,
		Border:   bStyle,
		TextWarn: base.ConditionRed,
	}
}

// New constructs a new schematic Model with the specified dimensions.
func New(th theme.Theme, width, height int) Model {
	if th == nil {
		th = theme.DefaultTheme()
	}
	if width <= 0 {
		width = 66
	}
	if height <= 0 {
		height = 18
	}
	return Model{
		width:      width,
		height:     height,
		theme:      th,
		repairMult: 1.0,
	}
}

// SetTheme updates the active theme.
func (m *Model) SetTheme(th theme.Theme) {
	if th != nil {
		m.theme = th
	}
}

// SetState updates device damage, condition, docking status, and repair scaling.
func (m *Model) SetState(enterprise engine.EnterpriseState, condition string, isDocked bool, repairMult float64) {
	m.devices = enterprise.Devices
	m.condition = condition
	m.isDocked = isDocked
	if repairMult <= 0 {
		repairMult = 1.0
	}
	m.repairMult = repairMult
}

// CloseModalMsg is emitted when the player presses a dismissal key in the schematic modal.
type CloseModalMsg struct{}

// Update handles keyboard messages for the damage schematic modal.
func (m Model) Update(msg tea.Msg) (Model, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.KeyMsg:
		switch msg.String() {
		case "esc", "enter", "q", "Q", "d", "D", " ", "space":
			return m, func() tea.Msg {
				return CloseModalMsg{}
			}
		}
	}
	return m, nil
}

func (m Model) formatPin(dev engine.DeviceID, tag string, styles schematicStyles) string {
	dmg := m.devices[dev]
	if dmg <= 0 {
		return styles.SubsystemNormal.Render(fmt.Sprintf("[%s: OK]", tag))
	}
	scaled := dmg * m.repairMult
	if dmg >= 2.0 {
		return styles.TextWarn.Render(fmt.Sprintf("[%s: %.1f]", tag, scaled))
	}
	return styles.SubsystemDamaged.Render(fmt.Sprintf("[%s: %.1f]", tag, scaled))
}

// View renders the bordered 66x18 damage schematic overlay.
func (m Model) View() string {
	styles := getSchematicStyles(m.theme)

	srsPin := m.formatPin(engine.DeviceSRSensors, "SRS", styles)
	compPin := m.formatPin(engine.DeviceComputer, "COMP", styles)
	lrsPin := m.formatPin(engine.DeviceLRSensors, "LRS", styles)
	phasPin := m.formatPin(engine.DevicePhasers, "PHAS", styles)
	tubPin := m.formatPin(engine.DevicePhotonTubes, "TUB", styles)
	shlPin := m.formatPin(engine.DeviceShields, "SHL", styles)
	damPin := m.formatPin(engine.DeviceDamageControl, "DAM", styles)
	warpPin := m.formatPin(engine.DeviceWarp, "WARP", styles)

	condText := m.condition
	if m.isDocked {
		condText = "DOCKED"
	}
	if condText == "" {
		condText = "GREEN"
	}

	innerW := m.width - 2 // 64 chars inside borders

	headerTitle := styles.PanelTitle.Render("DAMAGE CONTROL SCHEMATIC")
	titleW := ansi.StringWidth(headerTitle)
	topFill := innerW - titleW - 1
	if topFill < 0 {
		topFill = 0
	}
	borderTop := styles.Border.Render("┌") + styles.Border.Render("─") + headerTitle + styles.Border.Render(strings.Repeat("─", topFill)) + styles.Border.Render("┐")

	formatRow := func(content string) string {
		w := ansi.StringWidth(content)
		pad := innerW - w
		if pad < 0 {
			content = ansi.Truncate(content, innerW, "")
			pad = 0
		}
		return styles.Border.Render("│") + content + strings.Repeat(" ", pad) + styles.Border.Render("│")
	}

	row1 := fmt.Sprintf(" USS ENTERPRISE  NCC-1701                ALERT STATUS: %s", condText)
	row2 := ""
	row3 := fmt.Sprintf("          .---%s---.              %s", srsPin, lrsPin)
	row4 := fmt.Sprintf("         /    %s  \\             %s", compPin, phasPin)
	row5 := fmt.Sprintf("        |   (=) Saucer      |===%s===.", tubPin)
	row6 := "         \\     Bridge      /                 |"
	row7 := fmt.Sprintf("          '---%s---'                  |", shlPin)
	row8 := "                   \\                         |"
	row9 := fmt.Sprintf("                    \\===%s======%s", damPin, warpPin)
	row10 := ""

	repairMult := m.repairMult
	if repairMult <= 0 {
		repairMult = 1.0
	}

	var damagedRows []string
	for _, d := range devices {
		dmg := m.devices[d.id]
		if dmg > 0 {
			inFlight := dmg * repairMult
			docked := inFlight * 0.25
			statusStr := "DAMAGED"
			if dmg >= 2.0 {
				statusStr = "OFFLINE"
			}
			damagedRows = append(damagedRows, fmt.Sprintf(" %-18s %-9s %5.1f SD  %5.1f SD  %-13s", d.name, statusStr, inFlight, docked, d.impact))
		}
	}

	row11 := " SUBSYSTEM          STATUS    IN-FLIGHT  DOCKED  EFFECT"
	var row12, row13, row14, row15 string

	if len(damagedRows) == 0 {
		row12 = " All primary and auxiliary subsystems operational."
		row13 = " All other primary and tactical systems operational."
		row14 = ""
		row15 = " Scotty- \"All systems purring like kittens, Captain!\""
	} else {
		if len(damagedRows) > 0 {
			row12 = damagedRows[0]
		}
		if len(damagedRows) > 1 {
			row13 = damagedRows[1]
		} else {
			row13 = " All other primary and tactical systems operational."
		}
		if len(damagedRows) > 2 {
			row14 = damagedRows[2]
		} else if len(damagedRows) == 2 {
			row14 = " All other primary and tactical systems operational."
		} else {
			row14 = ""
		}
		if len(damagedRows) > 3 {
			row15 = fmt.Sprintf(" (+%d more damaged subsystems: view in status panel)", len(damagedRows)-3)
		} else {
			row15 = " Dock at starbase for accelerated 4x damage repair."
		}
	}

	row16 := " [ESC / ENTER / Q / D] Dismiss Damage Schematic"
	bottomFill := innerW
	if bottomFill < 0 {
		bottomFill = 0
	}
	borderBottom := styles.Border.Render("└") + styles.Border.Render(strings.Repeat("─", bottomFill)) + styles.Border.Render("┘")

	rows := []string{
		borderTop,
		formatRow(row1),
		formatRow(row2),
		formatRow(row3),
		formatRow(row4),
		formatRow(row5),
		formatRow(row6),
		formatRow(row7),
		formatRow(row8),
		formatRow(row9),
		formatRow(row10),
		formatRow(row11),
		formatRow(row12),
		formatRow(row13),
		formatRow(row14),
		formatRow(row15),
		formatRow(row16),
		borderBottom,
	}

	return strings.Join(rows, "\n")
}
