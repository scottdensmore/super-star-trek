package sectorgrid

import (
	"fmt"
	"strings"

	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/anim"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

// Model represents the 8x8 sector grid visualizer component.
type Model struct {
	theme         theme.Theme
	animOverrides map[engine.Coord]anim.CellOverride
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

// SetAnimOverrides sets temporary cell overrides for animation rendering.
func (m *Model) SetAnimOverrides(overrides map[engine.Coord]anim.CellOverride) {
	m.animOverrides = overrides
}

// ClearAnimOverrides removes all active cell animation overrides.
func (m *Model) ClearAnimOverrides() {
	m.animOverrides = nil
}

// Theme returns the currently active theme.
func (m Model) Theme() theme.Theme {
	if m.theme == nil {
		return theme.DefaultTheme()
	}
	return m.theme
}

// HitTest maps relative (x, y) coordinates within the grid's bounding box
// (including its outer border) to a 1-indexed engine.Coord [Row, Col].
func (m Model) HitTest(relX, relY int) (engine.Coord, bool) {
	// In the rendered View:
	// Line 0 is top border (┌──────...┐).
	// Line 1 is column header (│  1   2   3   4   5   6   7   8  │).
	// Lines 2..9 are the 8 sector row lines (relY = r + 1, where r = 1..8).
	// Line 10 is bottom border (└──────...┘).
	if relY < 2 || relY > 9 {
		return engine.Coord{}, false
	}
	r := relY - 1

	// In the rendered View:
	// Col 0 is left border (│).
	// Cols 1..2 are row label ("r ").
	// Then for c = 1..8:
	// c=1: cols 3..5 (width 3), col 6 space
	// c=2: cols 7..9 (width 3), col 10 space
	// c=k: cols 3 + 4*(k-1) .. 5 + 4*(k-1)
	// Col 34 is right border (│).
	if relX < 3 {
		return engine.Coord{}, false
	}
	offset := relX - 3
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
			coord := engine.Coord{r, c}
			if ov, ok := m.animOverrides[coord]; ok {
				glyph := padCell(ov.Glyph)
				b.WriteString(ov.Style.Render(glyph))
				if c < 8 {
					b.WriteByte(' ')
				}
				continue
			}

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

// padCell ensures a cell override glyph is strictly clamped and padded to exactly 3 runes.
func padCell(s string) string {
	runes := []rune(s)
	switch len(runes) {
	case 0:
		return "   "
	case 1:
		return " " + s + " "
	case 2:
		return " " + s
	case 3:
		return s
	default:
		return string(runes[:3])
	}
}

