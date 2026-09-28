package anim

import (
	"time"

	"github.com/charmbracelet/lipgloss"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
)

// PlasmaAnimation implements a step-wise tracking projectile visual sequence
// traversing sector coordinates along a Bresenham trajectory.
type PlasmaAnimation struct {
	from     engine.Coord
	to       engine.Coord
	path     []engine.Coord
	frames   []Frame
	current  int
	finished bool
}

// NewPlasmaAnimation creates a new PlasmaAnimation step-wise tracking movement from from to to.
func NewPlasmaAnimation(from, to engine.Coord, stepDuration time.Duration) *PlasmaAnimation {
	if stepDuration < 0 {
		stepDuration = 100 * time.Millisecond
	}

	pts := BresenhamLine(from, to)
	if len(pts) == 0 {
		pts = []engine.Coord{from}
	}

	plasmaStyle := lipgloss.NewStyle().Foreground(lipgloss.Color("208")).Bold(true)
	frames := make([]Frame, len(pts))
	for i, pt := range pts {
		frames[i] = Frame{
			Overrides: map[engine.Coord]CellOverride{
				pt: {Glyph: "*P*", Style: plasmaStyle},
			},
			Duration: stepDuration,
		}
	}

	anim := &PlasmaAnimation{
		from:   from,
		to:     to,
		path:   pts,
		frames: frames,
	}
	if stepDuration == 0 {
		anim.finished = true
		anim.current = len(frames)
	}
	return anim
}

// From returns the initial launch coordinate.
func (p *PlasmaAnimation) From() engine.Coord {
	return p.from
}

// To returns the target destination coordinate.
func (p *PlasmaAnimation) To() engine.Coord {
	return p.to
}

// Path returns the slice of coordinates along the projectile trajectory.
func (p *PlasmaAnimation) Path() []engine.Coord {
	return p.path
}

// CurrentCoord returns the current sector coordinate of the projectile.
func (p *PlasmaAnimation) CurrentCoord() engine.Coord {
	if len(p.path) == 0 {
		return p.to
	}
	if p.current < len(p.path) {
		return p.path[p.current]
	}
	return p.path[len(p.path)-1]
}

// CurrentGlyph returns the visual projectile glyph.
func (p *PlasmaAnimation) CurrentGlyph() string {
	return "*P*"
}

// Done reports whether the animation has completed all steps.
func (p *PlasmaAnimation) Done() bool {
	return p.finished || p.current >= len(p.frames)
}

// IsFinished satisfies the Animation interface.
func (p *PlasmaAnimation) IsFinished() bool {
	return p.Done()
}

// Tick advances the animation by one frame and returns that frame.
func (p *PlasmaAnimation) Tick() Frame {
	if p.Done() {
		p.finished = true
		return Frame{}
	}
	f := p.frames[p.current]
	p.current++
	if p.current >= len(p.frames) {
		p.finished = true
	}
	return f
}

// Step satisfies the Animation interface by advancing a single step.
func (p *PlasmaAnimation) Step() Frame {
	return p.Tick()
}

// Skip advances immediately to completion.
func (p *PlasmaAnimation) Skip() Frame {
	p.current = len(p.frames)
	p.finished = true
	return Frame{}
}

// TotalDuration satisfies the Animation interface by returning the sum of frame durations.
func (p *PlasmaAnimation) TotalDuration() time.Duration {
	var total time.Duration
	for _, f := range p.frames {
		total += f.Duration
	}
	return total
}

// Frames satisfies the Animation interface by returning all precomputed frames.
func (p *PlasmaAnimation) Frames() []Frame {
	return p.frames
}
