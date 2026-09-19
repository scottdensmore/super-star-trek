package tui

import (
	"os"
	"time"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/scottdensmore/super-star-trek/pkg/audio"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/anim"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/commandbar"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/commandpalette"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/damageschematic"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/galacticchart"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/halloffame"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/manual"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/optionsmodal"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/savebrowser"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/scenariomodal"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/sectorgrid"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/statuspanel"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/targetlock"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

// ModalType identifies the currently active floating modal overlay.
type ModalType int

const (
	ModalNone ModalType = iota
	ModalTargetLock
	ModalCommandPalette
	ModalSaveBrowser
	ModalGalacticChart
	ModalDamageSchematic
	ModalHallOfFame
	ModalManual
	ModalScenario
)

// Ensure Model satisfies the Bubble Tea Model interface at compile time.
var _ tea.Model = Model{}

// Model represents the root Bubble Tea model for Super Star Trek.
// It orchestrates the 8x8 sector grid visualizer, status/telemetry panel,
// and interactive command bar within a split dashboard layout, along with
// modal overlays (Target Lock HUD, Spock Command Palette, Save Game Browser, Galactic Star Chart).
type Model struct {
	Game           *engine.GameState
	Theme          theme.Theme
	Width          int
	Height         int
	Grid           sectorgrid.Model
	Status         statuspanel.Model
	CommandBar     commandbar.Model
	SelectedSector engine.Coord

	ActiveModal     ModalType
	TargetLock      targetlock.Model
	CommandPalette  commandpalette.Model
	SaveBrowser     savebrowser.Model
	GalacticChart   galacticchart.Model
	DamageSchematic damageschematic.Model
	HallOfFame      halloffame.Model
	Manual          manual.Model
	scenarioModal   scenariomodal.Model
	optionsModal    optionsmodal.Model
	showOptions     bool
	AudioPlayer     audio.Player
	AudioDispatcher *audio.Dispatcher
	activeAnim      anim.Animation
	animID          int
	LastClickTime   time.Time
	LastClickCoord  engine.Coord
	lastDarkBg      bool
	redAlertCycle   int
	redAlertActive  bool
}

// SetSoundEnabled configures audio mute state across player, status telemetry, and options modal.
func (m *Model) SetSoundEnabled(enabled bool) {
	if m.AudioPlayer != nil {
		m.AudioPlayer.SetMuted(!enabled)
	}
	m.Status.SetSoundEnabled(enabled)
	m.optionsModal.SetAudioEnabled(enabled)
}

// SoundEnabled reports whether audio sound FX is currently enabled.
func (m Model) SoundEnabled() bool {
	if m.AudioPlayer != nil {
		return !m.AudioPlayer.IsMuted()
	}
	return m.Status.SoundEnabled()
}

// syncChildComponents synchronizes telemetry sub-component state such as red alert pulse oscillation.
func (m *Model) syncChildComponents() {
	m.Status.SetRedAlertCycle(m.redAlertCycle)
}

// NewModel initializes and returns a new root TUI Model for the provided GameState and Theme.
// If th is nil, theme.DefaultTheme() is applied.
func NewModel(g *engine.GameState, th theme.Theme) Model {
	if th == nil {
		th = theme.DefaultTheme()
	}

	cb := commandbar.New(th)
	cb.AddMessage("Super Star Trek — Tactical Display Online")
	cb.AddMessage("Type 'help' for commands, 'quit' to exit, F2 to cycle themes")

	var rules engine.GameRules
	if g != nil {
		rules = g.Rules
	} else {
		rules = engine.DefaultRulesForProfile(engine.ProfileNormal)
	}

	player := audio.NewNativeOSPlayer(os.Stdout, nil)
	dispatcher := audio.NewDispatcher(player)

	m := Model{
		Game:            g,
		Theme:           th,
		Width:           80,
		Height:          24,
		Grid:            sectorgrid.New(th),
		Status:          statuspanel.New(th),
		CommandBar:      cb,
		ActiveModal:     ModalNone,
		TargetLock:      targetlock.New(th),
		CommandPalette:  commandpalette.New(th, 56, 16),
		SaveBrowser:     savebrowser.New(th, "."),
		GalacticChart:   galacticchart.New(th, 64, 18),
		DamageSchematic: damageschematic.New(th, 66, 18),
		HallOfFame:      halloffame.New(th, 66, 18, engine.DefaultLeaderboardPath()),
		Manual:          manual.New(th, 66, 18),
		scenarioModal:   scenariomodal.NewModel(th),
		optionsModal:    optionsmodal.New(th, rules),
		showOptions:     false,
		AudioPlayer:     player,
		AudioDispatcher: dispatcher,
		lastDarkBg: func() bool {
			if th.ColorMode() == theme.ColorModeLight {
				return false
			}
			if th.ColorMode() == theme.ColorModeDark {
				return true
			}
			return theme.DetectDarkBackground()
		}(),
	}
	m.syncChildComponents()
	return m
}

// Init initializes the Bubble Tea program lifecycle, activating keyboard focus
// and starting cursor blinking on the command bar, along with starting red alert pulse if in Condition RED.
func (m Model) Init() tea.Cmd {
	cmd := m.CommandBar.Focus()
	if m.Game != nil && m.Game.Enterprise.Condition == engine.ConditionRed && m.Game.Rules.AnimSpeed != engine.AnimSpeedOff {
		return tea.Batch(cmd, redAlertPulseCmd())
	}
	return cmd
}

