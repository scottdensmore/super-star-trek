package sectorgrid

import (
	"fmt"
	"strings"

	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

// Model represents the 8x8 sector grid visualizer component.
type Model struct {
	theme theme.Theme
}

// New creates a new sector grid Model with the provided theme.
func New(th theme.Theme) Model {
	if th == nil {
		th = theme.DefaultTheme()
	}
	return Model{theme: th}
}

// SetTheme updates the active theme for grid rendering.
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

// View renders the 8x8 sector grid for the current quadrant and Enterprise position.
func (m Model) View(quad *engine.QuadrantState, entSector engine.Coord) string {
	th := m.theme
	if th == nil {
		th = theme.DefaultTheme()
	}
	styles := th.Styles()

	var b strings.Builder
	b.WriteString(styles.GridHeader.Render("  1   2   3   4   5   6   7   8  "))

	for r := 1; r <= 8; r++ {
		b.WriteByte('\n')
		b.WriteString(styles.GridHeader.Render(fmt.Sprintf("%d ", r)))
		for c := 1; c <= 8; c++ {
			ent := engine.EntityEmpty
			if quad != nil && r >= 1 && r <= 8 && c >= 1 && c <= 8 {
				ent = quad.Grid[r][c]
			}
			if entSector[0] == r && entSector[1] == c {
				ent = engine.EntityEnterprise
			}
			b.WriteString(entityGlyph(ent, styles))
			if c < 8 {
				b.WriteByte(' ')
			}
		}
	}

	return styles.Grid.Render(b.String())
}

// entityGlyph formats and styles the 3-character glyph for an entity.
func entityGlyph(ent engine.EntityType, styles theme.Styles) string {
	switch ent {
	case engine.EntityEnterprise:
		return styles.Enterprise.Render("<E>")
	case engine.EntityKlingon, engine.EntityCommander, engine.EntitySuperCommander:
		return styles.Klingon.Render("+K+")
	case engine.EntityStarbase:
		return styles.Starbase.Render(">B<")
	case engine.EntityStar:
		return styles.Star.Render(" * ")
	case engine.EntityPlanet:
		return styles.Planet.Render(" O ")
	case engine.EntityBlackHole:
		return styles.BlackHole.Render(" @ ")
	case engine.EntityEmpty:
		return styles.Empty.Render(" . ")
	default:
		return styles.Empty.Render(" . ")
	}
}
