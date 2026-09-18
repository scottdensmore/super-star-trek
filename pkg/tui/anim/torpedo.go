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
	d := FrameDuration(speed)
	pts := BresenhamLine(start, end)

	var flightPoints []engine.Coord
	if len(pts) > 1 {
		flightPoints = pts[1:]
	} else {
		flightPoints = pts
	}

	projectileGlyphs := []string{" · ", " o ", " O "}
	torpStyle := lipgloss.NewStyle().Foreground(lipgloss.Color("9")).Bold(true)

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
		frames = append(frames, Frame{
			Overrides: map[engine.Coord]CellOverride{
				pt: {Glyph: projectileGlyphs[idx], Style: torpStyle},
			},
			Duration: d,
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
