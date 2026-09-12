package tui

import (
	"fmt"
	"path/filepath"
	"strings"
	"time"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/commandbar"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/commandpalette"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/savebrowser"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/targetlock"
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

	case targetlock.FireTorpedoMsg:
		if m.Game != nil {
			events, err := m.Game.Dispatch(engine.ActionFireTorpedo{
				Target: msg.Target,
				Angle:  msg.Bearing,
			})
			if err != nil {
				m.CommandBar.AddMessage(err.Error())
			} else {
				m.logEvents(events)
			}
		}
		m.ActiveModal = ModalNone
		cmd := m.CommandBar.Focus()
		return m, cmd

	case targetlock.FirePhasersMsg:
		if m.Game != nil {
			events, err := m.Game.Dispatch(engine.ActionFirePhasers{
				Energy: msg.Energy,
			})
			if err != nil {
				m.CommandBar.AddMessage(err.Error())
			} else {
				m.logEvents(events)
			}
		}
		m.ActiveModal = ModalNone
		cmd := m.CommandBar.Focus()
		return m, cmd

	case targetlock.CloseHUDMsg:
		m.ActiveModal = ModalNone
		cmd := m.CommandBar.Focus()
		return m, cmd

	case commandpalette.CommandSelectedMsg:
		m.ActiveModal = ModalNone
		if msg.Parameterized {
			m.CommandBar.SetValue(msg.CommandPrefix)
			cmd := m.CommandBar.Focus()
			return m, cmd
		}
		cmd := m.CommandBar.Focus()
		resModel, resCmd := m.handleCommand(msg.CommandPrefix)
		return resModel, tea.Batch(cmd, resCmd)

	case commandpalette.ClosePaletteMsg:
		m.ActiveModal = ModalNone
		cmd := m.CommandBar.Focus()
		return m, cmd

	case savebrowser.LoadGameMsg:
		loaded, err := engine.LoadGame(msg.Path)
		if err != nil {
			m.CommandBar.AddMessage(fmt.Sprintf("Failed to thaw %s: %v", filepath.Base(msg.Path), err))
		} else {
			m.Game = loaded
			m.SelectedSector = engine.Coord{}
			m.CommandBar.AddMessage(fmt.Sprintf("Mission thawed: %s (Stardate %.1f)", filepath.Base(msg.Path), loaded.Stardate))
		}
		m.ActiveModal = ModalNone
		cmd := m.CommandBar.Focus()
		return m, cmd

	case savebrowser.CloseBrowserMsg:
		m.ActiveModal = ModalNone
		cmd := m.CommandBar.Focus()
		return m, cmd

	case tea.KeyMsg:
		if m.ActiveModal != ModalNone {
			if msg.Type == tea.KeyCtrlC || msg.String() == "ctrl+c" {
				return m, tea.Quit
			}
			if msg.Type == tea.KeyEsc || msg.String() == "esc" {
				m.ActiveModal = ModalNone
				cmd := m.CommandBar.Focus()
				return m, cmd
			}
			if msg.Type == tea.KeyF2 || msg.String() == "f2" {
				m = m.applyTheme(m.Theme.Next())
				return m, nil
			}
			var cmd tea.Cmd
			switch m.ActiveModal {
			case ModalTargetLock:
				m.TargetLock, cmd = m.TargetLock.Update(msg)
			case ModalCommandPalette:
				m.CommandPalette, cmd = m.CommandPalette.Update(msg)
			case ModalSaveBrowser:
				m.SaveBrowser, cmd = m.SaveBrowser.Update(msg)
			}
			return m, cmd
		}

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

		case msg.Type == tea.KeyCtrlP || msg.String() == "ctrl+p":
			m.CommandPalette.Reset()
			m.ActiveModal = ModalCommandPalette
			m.CommandBar.Blur()
			return m, nil

		case msg.Type == tea.KeyCtrlO || msg.String() == "ctrl+o":
			_ = m.SaveBrowser.Refresh()
			m.ActiveModal = ModalSaveBrowser
			m.CommandBar.Blur()
			return m, nil

		case strings.TrimSpace(m.CommandBar.Value()) == "":
			switch {
			case msg.String() == "t" || msg.String() == "T":
				if m.Game == nil || len(m.Game.CurrentQuad.Klingons) == 0 {
					m.CommandBar.AddMessage("Sensors detect no hostile targets in sector.")
					return m, nil
				}
				m.TargetLock.SetState(
					m.Game.Enterprise.Sector,
					m.Game.Enterprise.Energy,
					m.Game.Enterprise.Torpedoes,
					m.Game.CurrentQuad.Klingons,
					engine.Coord{},
				)
				m.ActiveModal = ModalTargetLock
				m.CommandBar.Blur()
				return m, nil

			case msg.String() == "/":
				m.CommandPalette.Reset()
				m.ActiveModal = ModalCommandPalette
				m.CommandBar.Blur()
				return m, nil
			}
		}

		var cmd tea.Cmd
		m.CommandBar, cmd = m.CommandBar.Update(msg)
		return m, cmd

	case tea.MouseMsg:
		if m.ActiveModal != ModalNone {
			return m, nil
		}
		isLeftClick := msg.Button == tea.MouseButtonLeft || msg.Type == tea.MouseLeft
		if !isLeftClick || msg.Action == tea.MouseActionRelease || msg.Action == tea.MouseActionMotion {
			return m, nil
		}

		relX := msg.X
		relY := msg.Y - 1

		coord, ok := m.Grid.HitTest(relX, relY)
		if !ok {
			return m, nil
		}

		now := time.Now()
		isDoubleClick := coord == m.LastClickCoord && !m.LastClickTime.IsZero() && time.Since(m.LastClickTime) < 400*time.Millisecond

		if isDoubleClick {
			m.LastClickTime = time.Time{}
			m.LastClickCoord = coord

			var ent engine.EntityType = engine.EntityEmpty
			if m.Game != nil && coord[0] >= 1 && coord[0] <= 8 && coord[1] >= 1 && coord[1] <= 8 {
				ent = m.Game.CurrentQuad.Grid[coord[0]][coord[1]]
			}
			if m.Game != nil && m.Game.Enterprise.Sector == coord {
				ent = engine.EntityEnterprise
			}

			if ent == engine.EntityEmpty {
				if m.Game != nil {
					events, err := m.Game.Dispatch(engine.ActionMove{DestSector: coord, Warp: 1.0})
					if err != nil {
						m.CommandBar.AddMessage(err.Error())
					} else {
						m.logEvents(events)
					}
				}
				return m, nil
			}

			if ent == engine.EntityStarbase || (m.Game != nil && m.Game.CurrentQuad.Starbase != nil && *m.Game.CurrentQuad.Starbase == coord) {
				if m.Game != nil {
					events, err := m.Game.Dispatch(engine.ActionDock{})
					if err != nil {
						m.CommandBar.AddMessage(err.Error())
					} else {
						m.logEvents(events)
					}
				}
				return m, nil
			}

			return m, nil
		}

		// Single click
		m.SelectedSector = coord
		m.LastClickTime = now
		m.LastClickCoord = coord

		isKlingon := false
		if m.Game != nil {
			for _, k := range m.Game.CurrentQuad.Klingons {
				if k != nil && k.Sector == coord {
					isKlingon = true
					break
				}
			}
			if !isKlingon && coord[0] >= 1 && coord[0] <= 8 && coord[1] >= 1 && coord[1] <= 8 {
				gridEnt := m.Game.CurrentQuad.Grid[coord[0]][coord[1]]
				if gridEnt == engine.EntityKlingon || gridEnt == engine.EntityCommander || gridEnt == engine.EntitySuperCommander {
					isKlingon = true
				}
			}
		}

		if isKlingon {
			m.TargetLock.SetState(
				m.Game.Enterprise.Sector,
				m.Game.Enterprise.Energy,
				m.Game.Enterprise.Torpedoes,
				m.Game.CurrentQuad.Klingons,
				coord,
			)
			m.ActiveModal = ModalTargetLock
			m.CommandBar.Blur()
			return m, nil
		}

		m.CommandBar.AddMessage(fmt.Sprintf("Target sector selected: [%d, %d]", coord[0], coord[1]))
		return m, nil

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
	m.TargetLock.SetTheme(th)
	m.CommandPalette.SetTheme(th)
	m.SaveBrowser.SetTheme(th)
	return m
}

// handleCommand tokenizes, parses, and executes player commands.
func (m Model) handleCommand(text string) (tea.Model, tea.Cmd) {
	trimmed := strings.ToLower(strings.TrimSpace(text))
	switch trimmed {
	case "target":
		if m.Game == nil || len(m.Game.CurrentQuad.Klingons) == 0 {
			m.CommandBar.AddMessage("Sensors detect no hostile targets in sector.")
			return m, nil
		}
		m.TargetLock.SetState(
			m.Game.Enterprise.Sector,
			m.Game.Enterprise.Energy,
			m.Game.Enterprise.Torpedoes,
			m.Game.CurrentQuad.Klingons,
			engine.Coord{},
		)
		m.ActiveModal = ModalTargetLock
		m.CommandBar.Blur()
		return m, nil
	case "srscan":
		m.CommandBar.AddMessage("Short-range scan updated.")
		return m, nil
	case "lrscan":
		m.CommandBar.AddMessage("Long-range scan complete.")
		return m, nil
	case "status":
		m.CommandBar.AddMessage("Ship status nominal.")
		return m, nil
	case "dam":
		m.CommandBar.AddMessage("Damage report: all systems operational.")
		return m, nil
	case "chart":
		m.CommandBar.AddMessage("Galactic chart displayed.")
		return m, nil
	case "saves", "thaw":
		_ = m.SaveBrowser.Refresh()
		m.ActiveModal = ModalSaveBrowser
		m.CommandBar.Blur()
		return m, nil
	}

	if strings.HasPrefix(trimmed, "thaw ") {
		rawTrimmed := strings.TrimSpace(text)
		path := strings.TrimSpace(rawTrimmed[5:])
		loaded, err := engine.LoadGame(path)
		if err != nil {
			m.CommandBar.AddMessage(fmt.Sprintf("Failed to thaw %s: %v", path, err))
		} else {
			m.Game = loaded
			m.SelectedSector = engine.Coord{}
			m.CommandBar.AddMessage(fmt.Sprintf("Mission thawed: %s (Stardate %.1f)", filepath.Base(path), loaded.Stardate))
		}
		return m, nil
	}

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
			m.CommandBar.AddMessage("COMMANDS: nav | tor | pha | she | doc | saves | theme")
			m.CommandBar.AddMessage("Type 'help <command>' (e.g. 'help nav') for detailed guide.")
			m.CommandBar.AddMessage("HOTKEYS: [Ctrl+P] Spock Palette | [Ctrl+O] Saves | [T] Target Lock | [F2] Theme")
			return m, nil

		case parsed.Special == "help nav":
			m.CommandBar.AddMessage("NAV: Direct Quad: nav q <r c> [warp] (e.g. 'nav q 3 5')")
			m.CommandBar.AddMessage("     Direct Sector: nav s <r c> (or double-click sector grid)")
			m.CommandBar.AddMessage("     Vector: nav <course> <warp> (0.0=East, 1.57=North, 3.14=West, 4.71=South)")
			m.CommandBar.AddMessage("     Warp 1.0 = 1 Quad (8 sec). Shields UP = 2x energy. Damaged = Max Warp 4.")
			return m, nil

		case parsed.Special == "help tor":
			m.CommandBar.AddMessage("TOR: Target Sector: tor <r c> (e.g. 'tor 4 7')")
			m.CommandBar.AddMessage("     Bearing Angle: tor <angle> (0.0=East, 1.57=North, 3.14=West, 4.71=South)")
			m.CommandBar.AddMessage("     Tactical HUD: Press [T] for Target Lock auto-aiming & telemetry")
			m.CommandBar.AddMessage("     Damaged launcher cannot fire; torpedoes do not pass obstacles.")
			return m, nil

		case parsed.Special == "help pha":
			m.CommandBar.AddMessage("PHA: Fire phaser banks: pha <energy> (e.g. 'pha 300')")
			m.CommandBar.AddMessage("     Energy is divided among all Klingons present in quadrant.")
			m.CommandBar.AddMessage("     Damage drops with target distance. Damaged phasers cannot fire.")
			return m, nil

		case parsed.Special == "help she":
			m.CommandBar.AddMessage("SHE: Transfer shield energy: she <amount> (e.g. 'she 500', 'she -200')")
			m.CommandBar.AddMessage("     Shields protect against incoming torpedo & phaser damage.")
			m.CommandBar.AddMessage("     Shields UP doubles warp movement energy consumption!")
			return m, nil

		case parsed.Special == "help doc":
			m.CommandBar.AddMessage("DOC: Starbase docking: doc (must be in adjacent sector)")
			m.CommandBar.AddMessage("     Replenishes full energy & photon torpedo supply.")
			m.CommandBar.AddMessage("     Repairs all damaged ship systems and lowers shields.")
			return m, nil

		case parsed.Special == "help saves":
			m.CommandBar.AddMessage("SAVES: Open Save Browser: saves or bare thaw (hotkey Ctrl+O)")
			m.CommandBar.AddMessage("       Inspects stardates, condition, and Klingons remaining.")
			m.CommandBar.AddMessage("       Direct load: thaw <filename> | Freeze/save: freeze <filename>")
			return m, nil

		case strings.HasPrefix(parsed.Special, "help "):
			cmdName := strings.TrimPrefix(parsed.Special, "help ")
			m.CommandBar.AddMessage(fmt.Sprintf("No detailed help for %q. Available: help nav, help tor, help pha, help she, help doc, help saves", cmdName))
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

		m.logEvents(events)
		return m, nil
	}

	return m, nil
}

// logEvents formats and appends engine events to the command bar log buffer.
func (m *Model) logEvents(events []engine.Event) {
	for _, ev := range events {
		formatted := formatEvent(ev)
		if formatted != "" {
			m.CommandBar.AddMessage(formatted)
		}
	}
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
