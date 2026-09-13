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

// HitTest maps relative (x, y) coordinates within the grid's bounding box
// to a 1-indexed engine.Coord [Row, Col].
func (m Model) HitTest(relX, relY int) (engine.Coord, bool) {
	// relY: row 0 is header, row 1..8 are lines relY = r
	// In View: line 0 is col header.
	// Lines 1..8 are the 8 row lines (relY = 1..8).
	if relY < 1 || relY > 8 {
		return engine.Coord{}, false
	}
	r := relY

	// Each row starts with "r " (2 chars).
	// Then for c = 1..8:
	// c=1: chars 2..4 (width 3), char 5 space
	// c=2: chars 6..8, char 9 space
	// c=k: chars 2 + 4*(k-1) .. 4 + 4*(k-1)
	if relX < 2 {
		return engine.Coord{}, false
	}
	offset := relX - 2
	c := (offset / 4) + 1
	charWithinCell := offset % 4
	if c < 1 || c > 8 || charWithinCell >= 3 {
		return engine.Coord{}, false
	}
	return engine.Coord{r, c}, true
}

// View renders the 8x8 sector grid for the current quadrant, Enterprise position, and selected reticle cell.
func (m Model) View(quad *engine.QuadrantState, entSector engine.Coord, selected engine.Coord) string {
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
			isCloaked := false
			if quad != nil && ent != engine.EntityEnterprise {
				for _, k := range quad.Klingons {
					if k != nil && k.Sector[0] == r && k.Sector[1] == c && k.IsCloaked {
						isCloaked = true
						break
					}
				}
			}
			isSelected := selected[0] == r && selected[1] == c
			b.WriteString(entityGlyph(ent, styles, isSelected, isCloaked))
			if c < 8 {
				b.WriteByte(' ')
			}
		}
	}

	return styles.Grid.Render(b.String())
}

// entityGlyph formats and styles the 3-character glyph for an entity, applying reticle styling when selected.
func entityGlyph(ent engine.EntityType, styles theme.Styles, selected bool, cloaked bool) string {
	if cloaked {
		if selected {
			return styles.Empty.Reverse(true).Render("[?]")
		}
		return styles.Empty.Render(" ? ")
	}

	if selected {
		switch ent {
		case engine.EntityEnterprise:
			return styles.Enterprise.Reverse(true).Render("[E]")
		case engine.EntityKlingon, engine.EntityCommander, engine.EntitySuperCommander:
			return styles.Klingon.Reverse(true).Render("[K]")
		case engine.EntityStarbase:
			return styles.Starbase.Reverse(true).Render("[B]")
		case engine.EntityStar:
			return styles.Star.Reverse(true).Render("[*]")
		case engine.EntityPlanet:
			return styles.Planet.Reverse(true).Render("[O]")
		case engine.EntityBlackHole:
			return styles.BlackHole.Reverse(true).Render("[@]")
		case engine.EntityEmpty:
			return styles.Empty.Reverse(true).Render("[.]")
		default:
			return styles.Empty.Reverse(true).Render("[.]")
		}
	}

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
