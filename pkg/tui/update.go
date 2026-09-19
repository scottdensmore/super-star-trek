package tui

import (
	"fmt"
	"math"
	"path/filepath"
	"strings"
	"time"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/anim"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/commandbar"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/commandpalette"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/damageschematic"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/galacticchart"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/halloffame"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/manual"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/savebrowser"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/scenariomodal"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/targetlock"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

// RedAlertPulseMsg is dispatched periodically to animate the Condition Red klaxon visual oscillation.
type RedAlertPulseMsg struct{}

func redAlertPulseCmd() tea.Cmd {
	return tea.Tick(600*time.Millisecond, func(time.Time) tea.Msg {
		return RedAlertPulseMsg{}
	})
}

func (m *Model) checkRedAlertCmd(prevCond engine.ConditionType) tea.Cmd {
	if m.Game != nil && m.Game.Enterprise.Condition == engine.ConditionRed && m.Game.Rules.AnimSpeed != engine.AnimSpeedOff {
		if prevCond != engine.ConditionRed || !m.redAlertActive {
			m.redAlertActive = true
			return redAlertPulseCmd()
		}
	}
	return nil
}

// Update processes incoming Bubble Tea events, updating internal state
// and delegating to sub-components as needed.
func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.WindowSizeMsg:
		m.Width = msg.Width
		m.Height = msg.Height
		m.CommandBar.SetWidth(msg.Width)
		if m.Theme != nil && m.Theme.ColorMode() == theme.ColorModeAuto {
			hasDark := theme.DetectDarkBackground()
			if hasDark != m.lastDarkBg {
				m.lastDarkBg = hasDark
				m = m.applyTheme(m.Theme)
			}
		}
		return m, nil

	case targetlock.FireTorpedoMsg:
		var animCmd tea.Cmd
		var pulseCmd tea.Cmd
		if m.Game != nil {
			prevCond := m.Game.Enterprise.Condition
			action := engine.ActionFireTorpedo{
				Target: msg.Target,
				Angle:  msg.Bearing,
			}
			events, err := m.Game.Dispatch(action)
			if err != nil {
				m.CommandBar.AddMessage(err.Error())
			} else {
				m.logEvents(events)
				for _, ev := range events {
					if goEv, ok := ev.(engine.EventGameOver); ok {
						return m.handleGameOver(goEv)
					}
				}
				if m.Game.Rules.AnimSpeed > 0 {
					if a := m.createCombatAnimation(action, events); a != nil {
						m, animCmd = m.startCombatAnimation(a)
					}
				}
				pulseCmd = m.checkRedAlertCmd(prevCond)
			}
		}
		m.ActiveModal = ModalNone
		cmd := m.CommandBar.Focus()
		return m, tea.Batch(cmd, animCmd, pulseCmd)

	case targetlock.FirePhasersMsg:
		var animCmd tea.Cmd
		var pulseCmd tea.Cmd
		if m.Game != nil {
			prevCond := m.Game.Enterprise.Condition
			action := engine.ActionFirePhasers{
				Energy: msg.Energy,
			}
			events, err := m.Game.Dispatch(action)
			if err != nil {
				m.CommandBar.AddMessage(err.Error())
			} else {
				m.logEvents(events)
				for _, ev := range events {
					if goEv, ok := ev.(engine.EventGameOver); ok {
						return m.handleGameOver(goEv)
					}
				}
				if m.Game.Rules.AnimSpeed > 0 {
					if a := m.createCombatAnimation(action, events); a != nil {
						m, animCmd = m.startCombatAnimation(a)
					}
				}
				pulseCmd = m.checkRedAlertCmd(prevCond)
			}
		}
		m.ActiveModal = ModalNone
		cmd := m.CommandBar.Focus()
		return m, tea.Batch(cmd, animCmd, pulseCmd)

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
		pulseCmd := m.checkRedAlertCmd(engine.ConditionGreen)
		return m, tea.Batch(cmd, pulseCmd)

	case savebrowser.CloseBrowserMsg:
		m.ActiveModal = ModalNone
		cmd := m.CommandBar.Focus()
		return m, cmd

	case galacticchart.WarpToQuadrantMsg:
		m.ActiveModal = ModalNone
		var pulseCmd tea.Cmd
		if m.Game != nil {
			prevCond := m.Game.Enterprise.Condition
			events, err := m.Game.Dispatch(engine.ActionMove{DestQuad: msg.DestQuad, Warp: msg.Warp})
			if err != nil {
				m.CommandBar.AddMessage(err.Error())
			} else {
				m.SelectedSector = engine.Coord{}
				m.logEvents(events)
				for _, ev := range events {
					if goEv, ok := ev.(engine.EventGameOver); ok {
						return m.handleGameOver(goEv)
					}
				}
				pulseCmd = m.checkRedAlertCmd(prevCond)
			}
		}
		cmd := m.CommandBar.Focus()
		return m, tea.Batch(cmd, pulseCmd)

	case galacticchart.WarpBlockedMsg:
		m.ActiveModal = ModalNone
		cmd := m.CommandBar.Focus()
		m.CommandBar.AddMessage(msg.Reason)
		return m, cmd

	case galacticchart.CloseChartMsg:
		m.ActiveModal = ModalNone
		cmd := m.CommandBar.Focus()
		return m, cmd

	case damageschematic.CloseModalMsg:
		m.ActiveModal = ModalNone
		cmd := m.CommandBar.Focus()
		return m, cmd

	case halloffame.CloseModalMsg:
		m.ActiveModal = ModalNone
		cmd := m.CommandBar.Focus()
		return m, cmd

	case manual.CloseModalMsg:
		m.ActiveModal = ModalNone
		cmd := m.CommandBar.Focus()
		return m, cmd

	case scenariomodal.MsgLaunchScenario:
		m.ActiveModal = ModalNone
		sc, ok := engine.GetScenario(msg.ScenarioID)
		if ok && sc.Build != nil {
			seed := time.Now().UnixNano()
			m.Game = sc.Build(seed)
			m.SelectedSector = engine.Coord{}
			m.CommandBar.AddMessage(fmt.Sprintf("Tactical Scenario Launched: %s", sc.Name))
			cmd := m.CommandBar.Focus()
			pulseCmd := m.checkRedAlertCmd(engine.ConditionGreen)
			return m, tea.Batch(cmd, pulseCmd)
		}
		cmd := m.CommandBar.Focus()
		return m, cmd

	case scenariomodal.MsgCloseScenarioModal:
		m.ActiveModal = ModalNone
		cmd := m.CommandBar.Focus()
		return m, cmd

	case halloffame.ScoreRecordedMsg:
		m.CommandBar.AddMessage(fmt.Sprintf("Score recorded for Captain %s: %d points (%s)", msg.Entry.CaptainName, msg.Entry.Score, msg.Entry.Rank))
		return m, nil

	case engine.EventGameOver:
		return m.handleGameOver(msg)

	case RedAlertPulseMsg:
		if m.Game != nil && m.Game.Enterprise.Condition == engine.ConditionRed && m.Game.Rules.AnimSpeed != engine.AnimSpeedOff {
			m.redAlertCycle++
			m.syncChildComponents()
			m.redAlertActive = true
			return m, redAlertPulseCmd()
		}
		m.redAlertActive = false
		return m, nil

	case anim.TickMsg:
		if msg.AnimID == m.animID && m.activeAnim != nil {
			if m.activeAnim.IsFinished() {
				m.activeAnim = nil
				m.Grid.ClearAnimOverrides()
				return m, nil
			}
			f := m.activeAnim.Step()
			m.Grid.SetAnimOverrides(f.Overrides)
			return m, anim.TickCmd(m.animID, msg.Step+1, f.Duration)
		}
		return m, nil

	case tea.KeyMsg:
		if m.activeAnim != nil {
			m.activeAnim.Skip()
			m.activeAnim = nil
			m.Grid.ClearAnimOverrides()
		}

		if m.showOptions {
			if msg.Type == tea.KeyCtrlC || msg.String() == "ctrl+c" {
				return m, tea.Quit
			}
			if msg.Type == tea.KeyF2 || msg.String() == "f2" {
				m = m.applyTheme(m.Theme.Next())
				return m, nil
			}
			if msg.Type == tea.KeyCtrlT || msg.String() == "ctrl+t" || msg.String() == "shift+f2" {
				newMode := m.Theme.ColorMode().Next()
				m = m.applyTheme(m.Theme.WithColorMode(newMode))
				m.CommandBar.AddMessage(fmt.Sprintf("Color mode set to %s (%s)", newMode, m.Theme.Name()))
				return m, nil
			}
			var cmd tea.Cmd
			m.optionsModal, cmd = m.optionsModal.Update(msg)
			if m.optionsModal.Closed {
				m.showOptions = false
				if m.Game != nil {
					m.Game.Rules = m.optionsModal.Rules()
				}
				if m.Theme != nil {
					m = m.applyTheme(m.Theme.WithColorMode(m.optionsModal.ColorMode()))
				}
				m.SetSoundEnabled(m.optionsModal.AudioEnabled())
				m.optionsModal.Closed = false
				return m, m.CommandBar.Focus()
			}
			return m, cmd
		}

		if m.ActiveModal != ModalNone {
			if msg.Type == tea.KeyCtrlC || msg.String() == "ctrl+c" {
				return m, tea.Quit
			}
			if msg.Type == tea.KeyF2 || msg.String() == "f2" {
				m = m.applyTheme(m.Theme.Next())
				return m, nil
			}
			if msg.Type == tea.KeyCtrlT || msg.String() == "ctrl+t" || msg.String() == "shift+f2" {
				newMode := m.Theme.ColorMode().Next()
				m = m.applyTheme(m.Theme.WithColorMode(newMode))
				m.CommandBar.AddMessage(fmt.Sprintf("Color mode set to %s (%s)", newMode, m.Theme.Name()))
				return m, nil
			}
			if m.ActiveModal == ModalHallOfFame {
				var cmd tea.Cmd
				m.HallOfFame, cmd = m.HallOfFame.Update(msg)
				return m, cmd
			}
			if m.ActiveModal == ModalDamageSchematic {
				var cmd tea.Cmd
				m.DamageSchematic, cmd = m.DamageSchematic.Update(msg)
				return m, cmd
			}
			if m.ActiveModal == ModalManual {
				var cmd tea.Cmd
				m.Manual, cmd = m.Manual.Update(msg)
				return m, cmd
			}
			if m.ActiveModal == ModalScenario {
				var cmd tea.Cmd
				m.scenarioModal, cmd = m.scenarioModal.Update(msg)
				return m, cmd
			}
			if msg.Type == tea.KeyEsc || msg.String() == "esc" {
				m.ActiveModal = ModalNone
				cmd := m.CommandBar.Focus()
				return m, cmd
			}

			var cmd tea.Cmd
			switch m.ActiveModal {
			case ModalTargetLock:
				m.TargetLock, cmd = m.TargetLock.Update(msg)
			case ModalCommandPalette:
				m.CommandPalette, cmd = m.CommandPalette.Update(msg)
			case ModalSaveBrowser:
				m.SaveBrowser, cmd = m.SaveBrowser.Update(msg)
			case ModalGalacticChart:
				m.GalacticChart, cmd = m.GalacticChart.Update(msg)
			}
			return m, cmd
		}

		switch {
		case msg.Type == tea.KeyF1 || msg.String() == "f1":
			return m.openManual("")

		case msg.Type == tea.KeyF2 || msg.String() == "f2":
			m = m.applyTheme(m.Theme.Next())
			return m, nil

		case msg.Type == tea.KeyCtrlT || msg.String() == "ctrl+t" || msg.String() == "shift+f2":
			newMode := m.Theme.ColorMode().Next()
			m = m.applyTheme(m.Theme.WithColorMode(newMode))
			m.CommandBar.AddMessage(fmt.Sprintf("Color mode set to %s (%s)", newMode, m.Theme.Name()))
			return m, nil

		case msg.Type == tea.KeyCtrlC || msg.String() == "ctrl+c":
			return m, tea.Quit

		case msg.Type == tea.KeyCtrlS || msg.String() == "ctrl+s":
			return m.toggleSound()

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

		case msg.Type == tea.KeyCtrlM || msg.String() == "ctrl+m":
			return m.openGalacticChart()

		case msg.Type == tea.KeyCtrlD || msg.String() == "ctrl+d":
			if strings.TrimSpace(m.CommandBar.Value()) == "" {
				return m.openDamageSchematic()
			}

		case msg.Type == tea.KeyCtrlH || msg.String() == "ctrl+h":
			if strings.TrimSpace(m.CommandBar.Value()) == "" {
				return m.openHallOfFame(false)
			}

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

			case msg.String() == "c" || msg.String() == "C":
				return m.openGalacticChart()

			case msg.String() == "m" || msg.String() == "M":
				return m.toggleSound()

			case msg.String() == "d" || msg.String() == "D":
				return m.openDamageSchematic()

			case msg.String() == "h" || msg.String() == "H":
				return m.openHallOfFame(false)

			case msg.String() == "o" || msg.String() == "O":
				if m.Game != nil {
					m.optionsModal.SetRules(m.Game.Rules)
				}
				if m.Theme != nil {
					m.optionsModal.SetColorMode(m.Theme.ColorMode())
				}
				m.optionsModal.SetAudioEnabled(m.SoundEnabled())
				m.showOptions = true
				m.CommandBar.Blur()
				return m, nil

			case msg.String() == "?":
				return m.openManual("")
			}
		}

		var cmd tea.Cmd
		m.CommandBar, cmd = m.CommandBar.Update(msg)
		return m, cmd

	case tea.MouseMsg:
		if m.showOptions {
			return m, nil
		}
		isLeftClick := msg.Button == tea.MouseButtonLeft
		if !isLeftClick || msg.Action == tea.MouseActionRelease || msg.Action == tea.MouseActionMotion {
			return m, nil
		}

		if m.ActiveModal == ModalGalacticChart {
			if m.GalacticChart.ShowingHelp() {
				m.GalacticChart.SetShowingHelp(false)
				return m, nil
			}
			chartW := m.GalacticChart.Width()
			if chartW <= 0 {
				chartW = 64
			}
			chartH := m.GalacticChart.Height()
			if chartH <= 0 {
				chartH = 18
			}
			startX := (m.Width - chartW) / 2
			startY := (m.Height - chartH) / 2
			relX := msg.X - startX
			relY := msg.Y - startY
			coord, ok := m.GalacticChart.HitTest(relX, relY)
			if !ok {
				return m, nil
			}

			now := time.Now()
			isDoubleClick := coord == m.LastClickCoord && !m.LastClickTime.IsZero() && time.Since(m.LastClickTime) < 400*time.Millisecond

			if isDoubleClick {
				m.LastClickTime = time.Time{}
				m.LastClickCoord = coord
				if m.GalacticChart.ComputerDamaged() {
					m.CommandBar.AddMessage("COMPUTER DAMAGED, USE A POCKET CALCULATOR. Manual navigation required (nav q <r> <c> [warp]).")
					return m, nil
				}
				dest := coord
				var curQuad engine.Coord
				if m.Game != nil {
					curQuad = m.Game.Enterprise.Quad
				}
				telem := galacticchart.CalculateTelemetry(curQuad, dest)
				m.ActiveModal = ModalNone
				if m.Game != nil {
					prevCond := m.Game.Enterprise.Condition
					events, err := m.Game.Dispatch(engine.ActionMove{DestQuad: dest, Warp: telem.RecommendedWarp})
					if err != nil {
						m.CommandBar.AddMessage(err.Error())
					} else {
						m.SelectedSector = engine.Coord{}
						m.logEvents(events)
						for _, ev := range events {
							if goEv, ok := ev.(engine.EventGameOver); ok {
								return m.handleGameOver(goEv)
							}
						}
						pulseCmd := m.checkRedAlertCmd(prevCond)
						return m, pulseCmd
					}
				}
				return m, nil
			}

			m.LastClickTime = now
			m.LastClickCoord = coord
			m.GalacticChart.SetCursor(coord)
			return m, nil
		}

		if m.ActiveModal != ModalNone {
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

			ent := engine.EntityEmpty
			if m.Game != nil && coord[0] >= 1 && coord[0] <= 8 && coord[1] >= 1 && coord[1] <= 8 {
				ent = m.Game.CurrentQuad.Grid[coord[0]][coord[1]]
			}
			if m.Game != nil && m.Game.Enterprise.Sector == coord {
				ent = engine.EntityEnterprise
			}

			if ent == engine.EntityEmpty {
				if m.Game != nil {
					prevCond := m.Game.Enterprise.Condition
					events, err := m.Game.Dispatch(engine.ActionMove{DestSector: coord, Warp: 1.0})
					if err != nil {
						m.CommandBar.AddMessage(err.Error())
					} else {
						m.logEvents(events)
						for _, ev := range events {
							if goEv, ok := ev.(engine.EventGameOver); ok {
								return m.handleGameOver(goEv)
							}
						}
						pulseCmd := m.checkRedAlertCmd(prevCond)
						return m, pulseCmd
					}
				}
				return m, nil
			}

			if ent == engine.EntityStarbase || (m.Game != nil && m.Game.CurrentQuad.Starbase != nil && *m.Game.CurrentQuad.Starbase == coord) {
				if m.Game != nil {
					prevCond := m.Game.Enterprise.Condition
					events, err := m.Game.Dispatch(engine.ActionDock{})
					if err != nil {
						m.CommandBar.AddMessage(err.Error())
					} else {
						m.logEvents(events)
						for _, ev := range events {
							if goEv, ok := ev.(engine.EventGameOver); ok {
								return m.handleGameOver(goEv)
							}
						}
						pulseCmd := m.checkRedAlertCmd(prevCond)
						return m, pulseCmd
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
	m.lastDarkBg = theme.DetectDarkBackground()
	m.Grid.SetTheme(th)
	m.Status.SetTheme(th)
	m.CommandBar.SetTheme(th)
	m.TargetLock.SetTheme(th)
	m.CommandPalette.SetTheme(th)
	m.SaveBrowser.SetTheme(th)
	m.GalacticChart.SetTheme(th)
	m.DamageSchematic.SetTheme(th)
	m.HallOfFame.SetTheme(th)
	m.Manual.SetTheme(th)
	m.scenarioModal.SetTheme(th)
	m.optionsModal.SetTheme(th)
	m.syncChildComponents()
	return m
}

// openGalacticChart updates star chart state from the active game and activates ModalGalacticChart.
func (m Model) openGalacticChart() (Model, tea.Cmd) {
	entQuad := engine.Coord{1, 1}
	var chart [9][9]int
	var discovered [9][9]bool
	var knownBases [9][9]bool
	var compDamaged bool
	if m.Game != nil {
		entQuad = m.Game.Enterprise.Quad
		chart = m.Game.GalaxyChart
		discovered = m.Game.ChartDiscovered
		knownBases = m.Game.ChartKnownBases
		compDamaged = m.Game.Enterprise.Devices[engine.DeviceComputer] > 0
	}
	m.GalacticChart.SetState(entQuad, chart, discovered, knownBases, compDamaged)
	m.ActiveModal = ModalGalacticChart
	m.CommandBar.Blur()
	return m, nil
}

// openDamageSchematic synchronizes Enterprise damage state and activates ModalDamageSchematic.
func (m Model) openDamageSchematic() (Model, tea.Cmd) {
	cond := "GREEN"
	isDocked := false
	repairMult := 1.0
	var ent engine.EnterpriseState
	if m.Game != nil {
		ent = m.Game.Enterprise
		isDocked = (m.Game.Enterprise.Condition == engine.ConditionDocked)
		switch m.Game.Enterprise.Condition {
		case engine.ConditionYellow:
			cond = "YELLOW"
		case engine.ConditionRed:
			cond = "RED"
		case engine.ConditionDocked:
			cond = "DOCKED"
		default:
			cond = "GREEN"
		}
		if m.Game.Rules.RepairMultiplier > 0 {
			repairMult = m.Game.Rules.RepairMultiplier
		}
	}
	m.DamageSchematic.SetState(ent, cond, isDocked, repairMult)
	m.ActiveModal = ModalDamageSchematic
	m.CommandBar.Blur()
	return m, nil
}

// openHallOfFame synchronizes score and leaderboard state and activates ModalHallOfFame.
func (m Model) openHallOfFame(promptName bool) (Model, tea.Cmd) {
	lb, err := engine.LoadLeaderboard(engine.DefaultLeaderboardPath())
	if err != nil || lb == nil {
		lb = engine.DefaultLeaderboard()
	}
	var gameWon bool
	if m.Game != nil {
		gameWon = m.Game.GameWon
	}
	score := engine.ComputeScore(m.Game, gameWon)
	m.HallOfFame.SetState(score, lb, promptName)
	m.ActiveModal = ModalHallOfFame
	m.CommandBar.Blur()
	return m, nil
}

// openManual selects the requested topic/chapter and activates ModalManual.
func (m Model) openManual(topic string) (Model, tea.Cmd) {
	if topic != "" {
		m.Manual.SelectChapter(topic)
		fields := strings.Fields(topic)
		if len(fields) > 1 {
			m.Manual.SelectChapter(fields[0])
		}
	}
	m.ActiveModal = ModalManual
	m.CommandBar.Blur()
	return m, nil
}

// handleGameOver processes a game over event, logs the message, and opens the Hall of Fame modal.
func (m Model) handleGameOver(ev engine.EventGameOver) (Model, tea.Cmd) {
	formatted := formatEvent(ev)
	if formatted != "" {
		m.CommandBar.AddMessage(formatted)
	}
	if ev.Reason == engine.GameOverWon && m.Game != nil {
		m.Game.GameWon = true
	}
	var gameWon bool
	if m.Game != nil {
		gameWon = m.Game.GameWon
	}
	score := engine.ComputeScore(m.Game, gameWon)
	lb, err := engine.LoadLeaderboard(engine.DefaultLeaderboardPath())
	if err != nil || lb == nil {
		lb = engine.DefaultLeaderboard()
	}
	qualifies := lb.Qualifies(score.TotalScore)
	return m.openHallOfFame(qualifies)
}

// handleCommand tokenizes, parses, and executes player commands.
func (m Model) handleCommand(text string) (tea.Model, tea.Cmd) {
	trimmed := strings.ToLower(strings.TrimSpace(text))
	fields := strings.Fields(trimmed)
	if len(fields) == 3 && fields[0] == "theme" && fields[1] == "mode" {
		switch fields[2] {
		case "auto", "dark", "light":
			newMode, _ := theme.ParseColorMode(fields[2])
			m = m.applyTheme(m.Theme.WithColorMode(newMode))
			m.CommandBar.AddMessage(fmt.Sprintf("Color mode set to %s (%s)", newMode, m.Theme.Name()))
			return m, nil
		default:
			m.CommandBar.AddMessage(fmt.Sprintf("Invalid color mode: %q (expected auto, dark, or light)", fields[2]))
			return m, nil
		}
	}
	if len(fields) == 2 && fields[0] == "color" {
		switch fields[1] {
		case "auto", "dark", "light":
			newMode, _ := theme.ParseColorMode(fields[1])
			m = m.applyTheme(m.Theme.WithColorMode(newMode))
			m.CommandBar.AddMessage(fmt.Sprintf("Color mode set to %s (%s)", newMode, m.Theme.Name()))
			return m, nil
		default:
			m.CommandBar.AddMessage(fmt.Sprintf("Invalid color mode: %q (expected auto, dark, or light)", fields[1]))
			return m, nil
		}
	}
	if len(fields) == 2 && fields[0] == "theme" && fields[1] == "mode" {
		m.CommandBar.AddMessage("Usage: theme mode <auto|dark|light>")
		return m, nil
	}
	if len(fields) == 1 && fields[0] == "color" {
		m.CommandBar.AddMessage("Usage: color <auto|dark|light>")
		return m, nil
	}

	if len(fields) > 0 && fields[0] == "anim" {
		if len(fields) == 1 {
			if m.Game != nil {
				m.Game.Rules.AnimSpeed = (m.Game.Rules.AnimSpeed + 1) % 4
				m.optionsModal.SetRules(m.Game.Rules)
				speedNames := []string{"OFF", "FAST", "NORMAL", "CINEMATIC"}
				m.CommandBar.AddMessage(fmt.Sprintf("Combat animation speed set to %s", speedNames[m.Game.Rules.AnimSpeed]))
			} else {
				m.CommandBar.AddMessage("Usage: anim <off|fast|normal|cinematic>")
			}
			return m, nil
		}

		if len(fields) == 2 {
			speedNames := []string{"OFF", "FAST", "NORMAL", "CINEMATIC"}
			var newSpeed int
			switch fields[1] {
			case "off", "0":
				newSpeed = 0
			case "fast", "1":
				newSpeed = 1
			case "normal", "2":
				newSpeed = 2
			case "cinematic", "3":
				newSpeed = 3
			default:
				m.CommandBar.AddMessage(fmt.Sprintf("Invalid animation speed: %q (expected off, fast, normal, or cinematic)", fields[1]))
				return m, nil
			}
			if m.Game != nil {
				m.Game.Rules.AnimSpeed = newSpeed
				m.optionsModal.SetRules(m.Game.Rules)
			}
			m.CommandBar.AddMessage(fmt.Sprintf("Combat animation speed set to %s", speedNames[newSpeed]))
			return m, nil
		}

		m.CommandBar.AddMessage("Usage: anim <off|fast|normal|cinematic>")
		return m, nil
	}

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
		if m.Game == nil {
			m.CommandBar.AddMessage("No active game.")
			return m, nil
		}
		events, err := m.Game.Dispatch(engine.ActionLRScan{})
		if err != nil {
			m.CommandBar.AddMessage(fmt.Sprintf("LONG-RANGE SENSORS DAMAGED. %v", err))
			return m, nil
		}
		for _, ev := range events {
			if scanEvt, ok := ev.(engine.EventLRScanCompleted); ok {
				if scanEvt.RelayedByBase {
					m.CommandBar.AddMessage("Starbase relay: Long-range scan complete. Star chart updated.")
				} else {
					m.CommandBar.AddMessage("Long-range scan complete. Star chart updated for 3x3 surrounding quadrants.")
				}
			}
		}
		return m, nil
	case "status":
		m.CommandBar.AddMessage("Ship status nominal.")
		return m, nil
	case "dam", "damage", "damages":
		return m.openDamageSchematic()
	case "chart", "map":
		return m.openGalacticChart()
	case "saves", "thaw":
		_ = m.SaveBrowser.Refresh()
		m.ActiveModal = ModalSaveBrowser
		m.CommandBar.Blur()
		return m, nil
	case "score", "scores", "halloffame", "hof":
		return m.openHallOfFame(false)
	case "opts", "options", "settings":
		if m.Game != nil {
			m.optionsModal.SetRules(m.Game.Rules)
		}
		if m.Theme != nil {
			m.optionsModal.SetColorMode(m.Theme.ColorMode())
		}
		m.optionsModal.SetAudioEnabled(m.SoundEnabled())
		m.showOptions = true
		m.CommandBar.Blur()
		return m, nil
	case "help", "man", "manual", "doc", "docs", "codex", "guide":
		return m.openManual("")
	}

	for _, p := range []string{"help ", "man ", "manual ", "doc ", "docs ", "codex ", "guide "} {
		if strings.HasPrefix(trimmed, p) {
			topic := strings.TrimSpace(trimmed[len(p):])
			switch topic {
			case "nav", "move", "warp":
				m.CommandBar.AddMessage("NAV: Direct Quad: nav q <r c> [warp] (e.g. 'nav q 3 5')")
				m.CommandBar.AddMessage("     Direct Sector: nav s <r c> (or double-click sector grid)")
				m.CommandBar.AddMessage("     Vector: nav <course> <warp> (0.0=East, 1.57=North, 3.14=West, 4.71=South)")
				m.CommandBar.AddMessage("     Warp 1.0 = 1 Quad. Dist = sqrt(ΔR²+ΔC²). [Ctrl+M] map tool. Shields UP = 2x energy.")
			case "tor", "torpedo", "torpedoes", "target", "reticle":
				m.CommandBar.AddMessage("TOR: Target Sector: tor <r c> (e.g. 'tor 4 7')")
				m.CommandBar.AddMessage("     Bearing Angle: tor <angle> (0.0=East, 1.57=North, 3.14=West, 4.71=South)")
				m.CommandBar.AddMessage("     Tactical HUD: Press [T] for Target Lock auto-aiming & telemetry")
				m.CommandBar.AddMessage("     Damaged launcher cannot fire; torpedoes do not pass obstacles.")
			case "pha", "phaser", "phasers":
				m.CommandBar.AddMessage("PHA: Fire phaser banks: pha <energy> (e.g. 'pha 300')")
				m.CommandBar.AddMessage("     Energy is divided among all Klingons present in quadrant.")
				m.CommandBar.AddMessage("     Damage drops with target distance. Damaged phasers cannot fire.")
			case "she", "shield", "shields", "def":
				m.CommandBar.AddMessage("SHE: Transfer shield energy: she <amount> (e.g. 'she 500', 'she -200')")
				m.CommandBar.AddMessage("     Shields protect against incoming torpedo & phaser damage.")
				m.CommandBar.AddMessage("     Shields UP doubles warp movement energy consumption!")
			case "doc", "dock":
				m.CommandBar.AddMessage("DOC: Starbase docking: doc (must be in adjacent sector)")
				m.CommandBar.AddMessage("     Replenishes full energy & photon torpedo supply.")
				m.CommandBar.AddMessage("     Repairs all damaged ship systems and lowers shields.")
			case "chart", "map":
				m.CommandBar.AddMessage("CHART: Interactive Galactic Star Chart & Warp Planner (Ctrl+M or 'chart')")
				m.CommandBar.AddMessage("       Inspect 8x8 quadrant grid, telemetry vectors, and distance calculations.")
				m.CommandBar.AddMessage("       [Arrows/HJKL] Move cursor  [Enter] Warp to quadrant  [Esc] Close")
			case "saves", "thaw", "freeze":
				m.CommandBar.AddMessage("SAVES: Open Save Browser: saves or bare thaw (hotkey Ctrl+O)")
				m.CommandBar.AddMessage("       Inspects stardates, condition, and Klingons remaining.")
				m.CommandBar.AddMessage("       Direct load: thaw <filename> | Freeze/save: freeze <filename>")
			case "opts", "options", "settings":
				m.CommandBar.AddMessage("OPTIONS: Configure game difficulty & realism settings (hotkey [O] or 'options')")
				m.CommandBar.AddMessage("         Adjust difficulty profile, surveillance mode, sensors, repair, and cloaking.")
			default:
				if strings.HasPrefix(p, "help") {
					m.CommandBar.AddMessage(fmt.Sprintf("No detailed help for %q. Available: help nav, help tor, help pha, help she, help doc, help chart, help saves, help options", topic))
				}
			}
			return m.openManual(topic)
		}
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

		case parsed.Special == "options":
			if m.Game != nil {
				m.optionsModal.SetRules(m.Game.Rules)
			}
			if m.Theme != nil {
				m.optionsModal.SetColorMode(m.Theme.ColorMode())
			}
			m.optionsModal.SetAudioEnabled(m.SoundEnabled())
			m.showOptions = true
			m.CommandBar.Blur()
			return m, nil

		case parsed.Special == "scenarios":
			m.scenarioModal.Reset()
			m.ActiveModal = ModalScenario
			m.CommandBar.Blur()
			return m, nil

		case parsed.Special == "help":
			m.CommandBar.AddMessage("COMMANDS: nav | tor | pha | she | doc | chart | saves | theme | options")
			m.CommandBar.AddMessage("Type 'help <command>' (e.g. 'help nav') for detailed guide.")
			m.CommandBar.AddMessage("HOTKEYS: [Ctrl+P] Spock Palette | [Ctrl+M] Star Chart | [Ctrl+O] Saves | [O] Options | [T] Target Lock | [F2] Theme | [F1/?] Manual")
			return m.openManual("")

		case parsed.Special == "help nav":
			m.CommandBar.AddMessage("NAV: Direct Quad: nav q <r c> [warp] (e.g. 'nav q 3 5')")
			m.CommandBar.AddMessage("     Direct Sector: nav s <r c> (or double-click sector grid)")
			m.CommandBar.AddMessage("     Vector: nav <course> <warp> (0.0=East, 1.57=North, 3.14=West, 4.71=South)")
			m.CommandBar.AddMessage("     Warp 1.0 = 1 Quad. Dist = sqrt(ΔR²+ΔC²). [Ctrl+M] map tool. Shields UP = 2x energy.")
			return m.openManual("nav")

		case parsed.Special == "help tor":
			m.CommandBar.AddMessage("TOR: Target Sector: tor <r c> (e.g. 'tor 4 7')")
			m.CommandBar.AddMessage("     Bearing Angle: tor <angle> (0.0=East, 1.57=North, 3.14=West, 4.71=South)")
			m.CommandBar.AddMessage("     Tactical HUD: Press [T] for Target Lock auto-aiming & telemetry")
			m.CommandBar.AddMessage("     Damaged launcher cannot fire; torpedoes do not pass obstacles.")
			return m.openManual("tor")

		case parsed.Special == "help pha":
			m.CommandBar.AddMessage("PHA: Fire phaser banks: pha <energy> (e.g. 'pha 300')")
			m.CommandBar.AddMessage("     Energy is divided among all Klingons present in quadrant.")
			m.CommandBar.AddMessage("     Damage drops with target distance. Damaged phasers cannot fire.")
			return m.openManual("pha")

		case parsed.Special == "help she":
			m.CommandBar.AddMessage("SHE: Transfer shield energy: she <amount> (e.g. 'she 500', 'she -200')")
			m.CommandBar.AddMessage("     Shields protect against incoming torpedo & phaser damage.")
			m.CommandBar.AddMessage("     Shields UP doubles warp movement energy consumption!")
			return m.openManual("she")

		case parsed.Special == "help doc":
			m.CommandBar.AddMessage("DOC: Starbase docking: doc (must be in adjacent sector)")
			m.CommandBar.AddMessage("     Replenishes full energy & photon torpedo supply.")
			m.CommandBar.AddMessage("     Repairs all damaged ship systems and lowers shields.")
			return m.openManual("doc")

		case parsed.Special == "help chart":
			m.CommandBar.AddMessage("CHART: Interactive Galactic Star Chart & Warp Planner (Ctrl+M or 'chart')")
			m.CommandBar.AddMessage("       Inspect 8x8 quadrant grid, telemetry vectors, and distance calculations.")
			m.CommandBar.AddMessage("       [Arrows/HJKL] Move cursor  [Enter] Warp to quadrant  [Esc] Close")
			return m.openManual("chart")

		case parsed.Special == "help saves":
			m.CommandBar.AddMessage("SAVES: Open Save Browser: saves or bare thaw (hotkey Ctrl+O)")
			m.CommandBar.AddMessage("       Inspects stardates, condition, and Klingons remaining.")
			m.CommandBar.AddMessage("       Direct load: thaw <filename> | Freeze/save: freeze <filename>")
			return m.openManual("saves")

		case parsed.Special == "help options":
			m.CommandBar.AddMessage("OPTIONS: Configure game difficulty & realism settings (hotkey [O] or 'options')")
			m.CommandBar.AddMessage("         Adjust difficulty profile, surveillance mode, sensors, repair, and cloaking.")
			return m.openManual("options")

		case strings.HasPrefix(parsed.Special, "help "):
			cmdName := strings.TrimPrefix(parsed.Special, "help ")
			m.CommandBar.AddMessage(fmt.Sprintf("No detailed help for %q. Available: help nav, help tor, help pha, help she, help doc, help chart, help saves, help options", cmdName))
			return m.openManual(cmdName)

		case parsed.Special == "theme":
			m = m.applyTheme(m.Theme.Next())
			m.CommandBar.AddMessage(fmt.Sprintf("Theme switched to %s", m.Theme.Name()))
			return m, nil

		case strings.HasPrefix(parsed.Special, "theme "):
			name := strings.TrimSpace(strings.TrimPrefix(parsed.Special, "theme "))
			th := theme.GetTheme(name)
			if m.Theme != nil {
				th = th.WithColorMode(m.Theme.ColorMode())
			}
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

		prevCond := m.Game.Enterprise.Condition
		events, err := m.Game.Dispatch(parsed.Action)
		if err != nil {
			m.CommandBar.AddMessage(err.Error())
			return m, nil
		}

		m.logEvents(events)
		for _, ev := range events {
			if moveEv, ok := ev.(engine.EventShipMoved); ok && moveEv.FromQuad != moveEv.ToQuad {
				m.SelectedSector = engine.Coord{}
			}
			if goEv, ok := ev.(engine.EventGameOver); ok {
				return m.handleGameOver(goEv)
			}
		}

		var animCmd tea.Cmd
		if m.Game.Rules.AnimSpeed > 0 {
			if a := m.createCombatAnimation(parsed.Action, events); a != nil {
				m, animCmd = m.startCombatAnimation(a)
			}
		}
		pulseCmd := m.checkRedAlertCmd(prevCond)
		return m, tea.Batch(animCmd, pulseCmd)
	}

	return m, nil
}

// toggleSound inverts the current sound enabled state, synchronizing the player,
// status panel, and options modal, and displaying a tactical notification.
func (m *Model) toggleSound() (Model, tea.Cmd) {
	enabled := !m.SoundEnabled()
	m.SetSoundEnabled(enabled)
	if enabled {
		m.CommandBar.AddMessage("*** Audio: Enabled ***")
	} else {
		m.CommandBar.AddMessage("*** Audio: Muted ***")
	}
	return *m, nil
}

// logEvents formats and appends engine events to the command bar log buffer,
// and dispatches them to the audio engine.
func (m *Model) logEvents(events []engine.Event) {
	if m.AudioDispatcher != nil && len(events) > 0 {
		m.AudioDispatcher.DispatchEvents(events)
	}
	for _, ev := range events {
		if _, ok := ev.(engine.EventGameOver); ok {
			continue
		}
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

	case engine.EventStarbaseSurveillance:
		return fmt.Sprintf("Starbase at [%d,%d] downloaded surveillance. Updated %d quadrants.", e.StarbaseCoord[0], e.StarbaseCoord[1], e.UpdatedQuads)

	case engine.EventLRScanCompleted:
		if e.RelayedByBase {
			return "Starbase relay: Long-range scan complete. Star chart updated."
		}
		return "Long-range scan complete. Star chart updated for 3x3 surrounding quadrants."

	case engine.EventShipMoved:
		return fmt.Sprintf("Ship arrived at Quadrant [%d,%d], Sector [%d,%d]", e.ToQuad[0], e.ToQuad[1], e.ToSector[0], e.ToSector[1])

	case engine.EventObstacleEncountered:
		return fmt.Sprintf("Maneuver stopped: obstacle at sector [%d,%d]", e.Sector[0], e.Sector[1])

	case engine.EventConditionChanged:
		return fmt.Sprintf("Alert status changed to %s", conditionString(e.To))

	case engine.EventKlingonCounterAttack:
		return fmt.Sprintf("Klingon #%d returned fire: %.0f damage", e.EnemyID, e.Damage)

	case engine.EventKlingonCloakState:
		if e.Cloaked {
			return fmt.Sprintf("Klingon #%d engaged cloaking device.", e.KlingonID)
		}
		return fmt.Sprintf("Klingon #%d decloaked!", e.KlingonID)

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

// startCombatAnimation initializes and begins playback of an interactive combat animation sequence.
func (m Model) startCombatAnimation(a anim.Animation) (Model, tea.Cmd) {
	m.animID++
	m.activeAnim = a
	f := a.Step()
	m.Grid.SetAnimOverrides(f.Overrides)
	return m, anim.TickCmd(m.animID, 1, f.Duration)
}

func (m Model) createCombatAnimation(action engine.Action, events []engine.Event) anim.Animation {
	if m.Game == nil {
		return nil
	}
	switch act := action.(type) {
	case engine.ActionFireTorpedo:
		start := m.Game.Enterprise.Sector
		var hitEv *engine.EventTorpedoHit
		firedAngle := act.Angle
		hasFiredAngle := act.Angle != 0

		for _, ev := range events {
			switch e := ev.(type) {
			case engine.EventTorpedoFired:
				start = e.Origin
				firedAngle = e.Angle
				hasFiredAngle = true
			case engine.EventTorpedoHit:
				hitCopy := e
				hitEv = &hitCopy
			}
		}

		var end engine.Coord
		hit := hitEv != nil
		if hit {
			end = hitEv.Target
		} else if act.Target != (engine.Coord{}) {
			end = act.Target
		} else if hasFiredAngle {
			end = traceTorpedoBoundary(start, firedAngle)
		} else {
			end = start
		}

		return anim.NewTorpedoAnimation(start, end, hit, m.Game.Rules.AnimSpeed)

	case engine.ActionFirePhasers:
		start := m.Game.Enterprise.Sector
		var targets []engine.Coord
		var hits []bool
		for _, ev := range events {
			if h, ok := ev.(engine.EventPhaserHit); ok {
				targets = append(targets, h.Target)
				hits = append(hits, true)
			}
		}
		if len(targets) == 0 {
			return nil
		}
		return anim.NewMultiPhaserAnimation(start, targets, hits, m.Game.Rules.AnimSpeed)
	}
	return nil
}

func traceTorpedoBoundary(start engine.Coord, angle float64) engine.Coord {
	r := float64(start.Row())
	c := float64(start.Col())
	dr := -math.Sin(angle) * 0.25
	dc := math.Cos(angle) * 0.25

	last := start
	for step := 0; step < 40; step++ {
		r += dr
		c += dc
		ir := int(math.Round(r))
		ic := int(math.Round(c))
		if ir < 1 || ir > 8 || ic < 1 || ic > 8 {
			break
		}
		last = engine.Coord{ir, ic}
	}
	return last
}

