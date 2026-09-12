package tui

import (
	tea "github.com/charmbracelet/bubbletea"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/commandbar"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/sectorgrid"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/statuspanel"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

// Ensure Model satisfies the Bubble Tea Model interface at compile time.
var _ tea.Model = Model{}

// Model represents the root Bubble Tea model for Super Star Trek.
// It orchestrates the 8x8 sector grid visualizer, status/telemetry panel,
// and interactive command bar within a split dashboard layout.
type Model struct {
	Game           *engine.GameState
	Theme          theme.Theme
	Width          int
	Height         int
	Grid           sectorgrid.Model
	Status         statuspanel.Model
	CommandBar     commandbar.Model
	SelectedSector engine.Coord
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

	return Model{
		Game:       g,
		Theme:      th,
		Grid:       sectorgrid.New(th),
		Status:     statuspanel.New(th),
		CommandBar: cb,
	}
}

// Init initializes the Bubble Tea program lifecycle, activating keyboard focus
// and starting cursor blinking on the command bar.
func (m Model) Init() tea.Cmd {
	return m.CommandBar.Focus()
}
