package anim

import (
	"time"

	"github.com/charmbracelet/lipgloss"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
)

// CloakAnimation implements a 4-stage Romulan shimmer transition
// between visible cloaked state and empty sensor reading.
type CloakAnimation struct {
	coord    engine.Coord
	cloaking bool
	stages   []string
	frames   []Frame
	current  int
	finished bool
}

// NewCloakAnimation creates a new Romulan cloak/decloak shimmer animation with default normal speed.
func NewCloakAnimation(coord engine.Coord, cloaking bool) *CloakAnimation {
	return NewCloakAnimationWithSpeed(coord, cloaking, engine.AnimSpeedNormal)
}

// NewCloakAnimationWithSpeed creates a new Romulan cloak/decloak shimmer animation with a specific speed setting.
func NewCloakAnimationWithSpeed(coord engine.Coord, cloaking bool, speed int) *CloakAnimation {
	d := FrameDuration(speed)
	if d <= 0 {
		d = 120 * time.Millisecond
	}

	var stages []string
	var styles []lipgloss.Style

	styleRomulan := lipgloss.NewStyle().Foreground(lipgloss.Color("10")).Bold(true)
	styleShimmer := lipgloss.NewStyle().Foreground(lipgloss.Color("11")).Italic(true)
	stylePhased := lipgloss.NewStyle().Foreground(lipgloss.Color("8"))
	styleEmpty := lipgloss.NewStyle().Foreground(lipgloss.Color("240"))

	if cloaking {
		// Cloaking: +R+ -> ~R~ -> ·?· ->  . 
		stages = []string{"+R+", "~R~", "·?·", " . "}
		styles = []lipgloss.Style{styleRomulan, styleShimmer, stylePhased, styleEmpty}
	} else {
		// Decloaking:  .  -> ·?· -> ~R~ -> +R+
		stages = []string{" . ", "·?·", "~R~", "+R+"}
		styles = []lipgloss.Style{styleEmpty, stylePhased, styleShimmer, styleRomulan}
	}

	frames := make([]Frame, len(stages))
	for i, g := range stages {
		frames[i] = Frame{
			Overrides: map[engine.Coord]CellOverride{
				coord: {Glyph: g, Style: styles[i]},
			},
			Duration: d,
		}
	}

	return &CloakAnimation{
		coord:    coord,
		cloaking: cloaking,
		stages:   stages,
		frames:   frames,
	}
}

// Coord returns the sector coordinate where the shimmer is rendered.
func (c *CloakAnimation) Coord() engine.Coord {
	return c.coord
}

// IsCloaking returns true if transitioning to cloaked, false if decloaking.
func (c *CloakAnimation) IsCloaking() bool {
	return c.cloaking
}

// CurrentGlyph returns the visual glyph for the current phase stage.
func (c *CloakAnimation) CurrentGlyph() string {
	if c.current < len(c.stages) {
		return c.stages[c.current]
	}
	if len(c.stages) > 0 {
		return c.stages[len(c.stages)-1]
	}
	return " . "
}

// Done reports whether all shimmer stages have completed.
func (c *CloakAnimation) Done() bool {
	return c.finished || c.current >= len(c.frames)
}

// IsFinished satisfies the Animation interface.
func (c *CloakAnimation) IsFinished() bool {
	return c.Done()
}

// Tick advances to the next shimmer stage and returns the corresponding frame.
func (c *CloakAnimation) Tick() Frame {
	if c.Done() {
		c.finished = true
		return Frame{}
	}
	f := c.frames[c.current]
	c.current++
	if c.current >= len(c.frames) {
		c.finished = true
	}
	return f
}

// Step satisfies the Animation interface by advancing a single step.
func (c *CloakAnimation) Step() Frame {
	return c.Tick()
}

// Skip immediately completes the shimmer animation.
func (c *CloakAnimation) Skip() Frame {
	c.current = len(c.frames)
	c.finished = true
	return Frame{}
}

// TotalDuration satisfies the Animation interface by returning the sum of frame durations.
func (c *CloakAnimation) TotalDuration() time.Duration {
	var total time.Duration
	for _, f := range c.frames {
		total += f.Duration
	}
	return total
}

// Frames satisfies the Animation interface by returning all precomputed frames.
func (c *CloakAnimation) Frames() []Frame {
	return c.frames
}
