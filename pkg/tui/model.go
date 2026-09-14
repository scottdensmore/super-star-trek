package tui

import (
	"time"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/commandbar"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/commandpalette"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/galacticchart"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/optionsmodal"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/savebrowser"
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

	ActiveModal    ModalType
	TargetLock     targetlock.Model
	CommandPalette commandpalette.Model
	SaveBrowser    savebrowser.Model
	GalacticChart  galacticchart.Model
	optionsModal   optionsmodal.Model
	showOptions    bool
	LastClickTime  time.Time
	LastClickCoord engine.Coord
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

	return Model{
		Game:           g,
		Theme:          th,
		Width:          80,
		Height:         24,
		Grid:           sectorgrid.New(th),
		Status:         statuspanel.New(th),
		CommandBar:     cb,
		ActiveModal:    ModalNone,
		TargetLock:     targetlock.New(th),
		CommandPalette: commandpalette.New(th, 56, 16),
		SaveBrowser:    savebrowser.New(th, "."),
		GalacticChart:  galacticchart.New(th, 64, 18),
		optionsModal:   optionsmodal.New(th, rules),
		showOptions:    false,
	}
}

// Init initializes the Bubble Tea program lifecycle, activating keyboard focus
// and starting cursor blinking on the command bar.
func (m Model) Init() tea.Cmd {
	return m.CommandBar.Focus()
}
