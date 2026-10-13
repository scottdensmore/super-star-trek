package anim

import (
	"time"

	"github.com/charmbracelet/lipgloss"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
)

type torpedoAnimation struct {
	frames   []Frame
	current  int
	finished bool
}

// NewTorpedoAnimation creates a new torpedo visual flight and impact animation.
func NewTorpedoAnimation(start, end engine.Coord, hit bool, speed int) Animation {
	return NewTorpedoAnimationWithEntity(start, end, hit, engine.EntityKlingon, speed)
}

// NewTorpedoAnimationWithEntity creates a torpedo visual flight and impact animation
// that preserves the target entity glyph at the impact sector while the torpedo is in flight.
func NewTorpedoAnimationWithEntity(start, end engine.Coord, hit bool, entity engine.EntityType, speed int) Animation {
	d := FrameDuration(speed)
	pts := BresenhamLine(start, end)

	var flightPoints []engine.Coord
	if hit {
		if len(pts) > 2 {
			flightPoints = pts[1 : len(pts)-1]
		}
	} else {
		if len(pts) > 1 {
			flightPoints = pts[1:]
		} else {
			flightPoints = pts
		}
	}

	projectileGlyphs := []string{" · ", " o ", " O "}
	torpStyle := lipgloss.NewStyle().Foreground(lipgloss.Color("9")).Bold(true)
	targetOverride := targetCellOverride(entity)

	var frames []Frame
	for i, pt := range flightPoints {
		idx := 0
		if len(flightPoints) > 1 {
			idx = (i * len(projectileGlyphs)) / len(flightPoints)
			if idx >= len(projectileGlyphs) {
				idx = len(projectileGlyphs) - 1
			}
		} else {
			idx = len(projectileGlyphs) - 1
		}
		overrides := map[engine.Coord]CellOverride{
			pt: {Glyph: projectileGlyphs[idx], Style: torpStyle},
		}
		if hit && pt != end {
			overrides[end] = targetOverride
		}
		frames = append(frames, Frame{
			Overrides: overrides,
			Duration:  d,
		})
	}

	if hit {
		frames = append(frames,
			Frame{
				Overrides: map[engine.Coord]CellOverride{
					end: {Glyph: " * ", Style: lipgloss.NewStyle().Foreground(lipgloss.Color("11")).Bold(true)},
				},
				Duration: d,
			},
			Frame{
				Overrides: map[engine.Coord]CellOverride{
					end: {Glyph: "***", Style: lipgloss.NewStyle().Foreground(lipgloss.Color("9")).Bold(true)},
				},
				Duration: d,
			},
			Frame{
				Overrides: map[engine.Coord]CellOverride{
					end: {Glyph: "#*#", Style: lipgloss.NewStyle().Foreground(lipgloss.Color("208")).Bold(true)},
				},
				Duration: d,
			},
		)
	} else {
		frames = append(frames, Frame{
			Overrides: map[engine.Coord]CellOverride{
				end: {Glyph: "   ", Style: lipgloss.NewStyle()},
			},
			Duration: d,
		})
	}

	return &torpedoAnimation{
		frames: frames,
	}
}

func targetCellOverride(entity engine.EntityType) CellOverride {
	switch entity {
	case engine.EntityRomulan:
		return CellOverride{Glyph: "+R+", Style: lipgloss.NewStyle().Foreground(lipgloss.Color("10")).Bold(true)}
	case engine.EntityTholian:
		return CellOverride{Glyph: "<T>", Style: lipgloss.NewStyle().Foreground(lipgloss.Color("11")).Bold(true)}
	case engine.EntityStarbase:
		return CellOverride{Glyph: ">B<", Style: lipgloss.NewStyle().Foreground(lipgloss.Color("14")).Bold(true)}
	case engine.EntityStar:
		return CellOverride{Glyph: " * ", Style: lipgloss.NewStyle().Foreground(lipgloss.Color("11")).Bold(true)}
	case engine.EntityPlanet:
		return CellOverride{Glyph: " O ", Style: lipgloss.NewStyle().Foreground(lipgloss.Color("12")).Bold(true)}
	case engine.EntityBlackHole:
		return CellOverride{Glyph: " @ ", Style: lipgloss.NewStyle().Foreground(lipgloss.Color("13")).Bold(true)}
	case engine.EntityPlasmaTorpedo:
		return CellOverride{Glyph: "*P*", Style: lipgloss.NewStyle().Foreground(lipgloss.Color("9")).Bold(true)}
	case engine.EntityTholianWeb:
		return CellOverride{Glyph: ":::", Style: lipgloss.NewStyle().Foreground(lipgloss.Color("11")).Bold(true)}
	default:
		return CellOverride{Glyph: "+K+", Style: lipgloss.NewStyle().Foreground(lipgloss.Color("9")).Bold(true)}
	}
}

func (a *torpedoAnimation) TotalDuration() time.Duration {
	var total time.Duration
	for _, f := range a.frames {
		total += f.Duration
	}
	return total
}

func (a *torpedoAnimation) Frames() []Frame {
	return a.frames
}

func (a *torpedoAnimation) IsFinished() bool {
	return a.finished || a.current >= len(a.frames)
}

func (a *torpedoAnimation) Step() Frame {
	if a.IsFinished() {
		a.finished = true
		return Frame{}
	}
	f := a.frames[a.current]
	a.current++
	if a.current >= len(a.frames) {
		a.finished = true
	}
	return f
}

func (a *torpedoAnimation) Skip() Frame {
	a.current = len(a.frames)
	a.finished = true
	return Frame{}
}
