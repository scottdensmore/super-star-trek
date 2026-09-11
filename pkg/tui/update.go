package tui

import (
	"fmt"
	"strings"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/commandbar"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

// Update processes incoming Bubble Tea events, updating internal state
// and delegating to sub-components as needed.
func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.WindowSizeMsg:
		m.Width = msg.Width
		m.Height = msg.Height
		m.CommandBar.SetWidth(msg.Width)
		return m, nil

	case tea.KeyMsg:
		switch {
		case msg.Type == tea.KeyF2 || msg.String() == "f2":
			m = m.applyTheme(m.Theme.Next())
			return m, nil

		case msg.Type == tea.KeyCtrlC || msg.String() == "ctrl+c":
			return m, tea.Quit

		case msg.Type == tea.KeyEsc || msg.String() == "esc":
			if strings.TrimSpace(m.CommandBar.Value()) == "" {
				return m, tea.Quit
			}
			m.CommandBar.Reset()
			return m, nil
		}

		var cmd tea.Cmd
		m.CommandBar, cmd = m.CommandBar.Update(msg)
		return m, cmd

	case commandbar.CommandSubmittedMsg:
		return m.handleCommand(msg.Text)

	default:
		var cmd tea.Cmd
		m.CommandBar, cmd = m.CommandBar.Update(msg)
		return m, cmd
	}
}

// UpdateModel is a convenience method for tests and typed callers returning concrete Model.
func (m Model) UpdateModel(msg tea.Msg) (Model, tea.Cmd) {
	updated, cmd := m.Update(msg)
	if concrete, ok := updated.(Model); ok {
		return concrete, cmd
	}
	return m, cmd
}

// applyTheme returns a copy of the model with the new theme propagated to all sub-components.
func (m Model) applyTheme(th theme.Theme) Model {
	if th == nil {
		th = theme.DefaultTheme()
	}
	m.Theme = th
	m.Grid.SetTheme(th)
	m.Status.SetTheme(th)
	m.CommandBar.SetTheme(th)
	return m
}

// handleCommand tokenizes, parses, and executes player commands.
func (m Model) handleCommand(text string) (tea.Model, tea.Cmd) {
	parsed := ParseCommand(text)

	if parsed.Error != nil {
		m.CommandBar.AddMessage(parsed.Error.Error())
		return m, nil
	}

	if parsed.Special != "" {
		switch {
		case parsed.Special == "quit":
			return m, tea.Quit

		case parsed.Special == "help":
			m.CommandBar.AddMessage("COMMANDS: nav <c> <w> | tor <c|r c> | pha <e> | she <a> | doc")
			m.CommandBar.AddMessage("UI: theme [modern|lcars|crt] (F2) | help (?) | quit (Esc)")
			return m, nil

		case parsed.Special == "theme":
			m = m.applyTheme(m.Theme.Next())
			m.CommandBar.AddMessage(fmt.Sprintf("Theme switched to %s", m.Theme.Name()))
			return m, nil

		case strings.HasPrefix(parsed.Special, "theme "):
			name := strings.TrimSpace(strings.TrimPrefix(parsed.Special, "theme "))
			th := theme.GetTheme(name)
			m = m.applyTheme(th)
			m.CommandBar.AddMessage(fmt.Sprintf("Theme switched to %s", m.Theme.Name()))
			return m, nil
		}
	}

	if parsed.Action != nil {
		if m.Game == nil {
			m.CommandBar.AddMessage("Error: no active game")
			return m, nil
		}

		events, err := m.Game.Dispatch(parsed.Action)
		if err != nil {
			m.CommandBar.AddMessage(err.Error())
			return m, nil
		}

		for _, ev := range events {
			formatted := formatEvent(ev)
			if formatted != "" {
				m.CommandBar.AddMessage(formatted)
			}
		}
		return m, nil
	}

	return m, nil
}

// formatEvent translates an engine.Event into human-readable tactical log messages.
func formatEvent(ev engine.Event) string {
	switch e := ev.(type) {
	case engine.EventShieldTransfer:
		return fmt.Sprintf("Shields: %.0f  Energy: %.0f", e.NewShields, e.NewEnergy)

	case engine.EventTorpedoFired:
		return fmt.Sprintf("Torpedo fired on course %.2f", e.Angle)

	case engine.EventTorpedoHit:
		if e.Destroyed {
			return fmt.Sprintf("*** Target destroyed at [%d,%d] ***", e.Target[0], e.Target[1])
		}
		return fmt.Sprintf("Hit on [%d,%d]: %.0f units damage", e.Target[0], e.Target[1], e.Damage)

	case engine.EventPhaserFired:
		return fmt.Sprintf("Phasers discharged with %.0f energy", e.Energy)

	case engine.EventPhaserHit:
		if e.Destroyed {
			return fmt.Sprintf("*** Klingon #%d destroyed ***", e.KlingonID)
		}
		return fmt.Sprintf("Hit on Klingon #%d: %.0f units damage", e.KlingonID, e.Damage)

	case engine.EventDocked:
		return fmt.Sprintf("Docked with Starbase at [%d,%d]. Systems refueled.", e.Starbase[0], e.Starbase[1])

	case engine.EventShipMoved:
		return fmt.Sprintf("Ship arrived at Quadrant [%d,%d], Sector [%d,%d]", e.ToQuad[0], e.ToQuad[1], e.ToSector[0], e.ToSector[1])

	case engine.EventObstacleEncountered:
		return fmt.Sprintf("Maneuver stopped: obstacle at sector [%d,%d]", e.Sector[0], e.Sector[1])

	case engine.EventConditionChanged:
		return fmt.Sprintf("Alert status changed to %s", conditionString(e.To))

	case engine.EventKlingonCounterAttack:
		return fmt.Sprintf("Klingon #%d returned fire: %.0f damage", e.EnemyID, e.Damage)

	case engine.EventSubsystemDamaged:
		return fmt.Sprintf("%s damaged! Repair time: %.1f stardates", deviceString(e.Device), e.RepairTime)

	case engine.EventSubsystemRepaired:
		return fmt.Sprintf("%s repaired and operational", deviceString(e.Device))

	case engine.EventGameOver:
		switch e.Reason {
		case engine.GameOverWon:
			return fmt.Sprintf("*** MISSION ACCOMPLISHED! Score: %.0f ***", e.Score)
		case engine.GameOverEnergy:
			return "*** GAME OVER: Enterprise out of energy ***"
		case engine.GameOverDestroyed:
			return "*** GAME OVER: Enterprise destroyed ***"
		case engine.GameOverTime:
			return "*** GAME OVER: Federation conquered - time expired ***"
		case engine.GameOverStranded:
			return "*** GAME OVER: Enterprise stranded in space ***"
		default:
			return "*** GAME OVER ***"
		}

	default:
		return ""
	}
}

func conditionString(c engine.ConditionType) string {
	switch c {
	case engine.ConditionGreen:
		return "CONDITION GREEN"
	case engine.ConditionYellow:
		return "CONDITION YELLOW"
	case engine.ConditionRed:
		return "CONDITION RED"
	case engine.ConditionDocked:
		return "CONDITION DOCKED"
	default:
		return "CONDITION GREEN"
	}
}

func deviceString(d engine.DeviceID) string {
	switch d {
	case engine.DeviceWarp:
		return "Warp Engines"
	case engine.DeviceSRSensors:
		return "Short-Range Sensors"
	case engine.DeviceLRSensors:
		return "Long-Range Sensors"
	case engine.DevicePhasers:
		return "Phasers"
	case engine.DevicePhotonTubes:
		return "Photon Tubes"
	case engine.DeviceDamageControl:
		return "Damage Control"
	case engine.DeviceShields:
		return "Shields"
	case engine.DeviceComputer:
		return "Computer"
	default:
		return "Subsystem"
	}
}
