package anim

import (
	"time"

	"github.com/charmbracelet/lipgloss"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
)

// WebAction defines the type of web animation sequence.
type WebAction int

const (
	// WebActionWeave represents the pulsating lattice placement sequence.
	WebActionWeave WebAction = iota
	// WebActionBreach represents the particle fracture and collapse sequence.
	WebActionBreach
)

// String returns the name of the web animation action.
func (a WebAction) String() string {
	switch a {
	case WebActionWeave:
		return "Weave"
	case WebActionBreach:
		return "Breach"
	default:
		return "Unknown"
	}
}

// WebAnimation implements Tholian web filament weaving pulse and breach fracture animations.
type WebAnimation struct {
	coord    engine.Coord
	action   WebAction
	stages   []string
	frames   []Frame
	current  int
	finished bool
}

// NewWebAnimation creates a new WebAnimation at coord with default normal speed.
func NewWebAnimation(coord engine.Coord, action WebAction) *WebAnimation {
	return NewWebAnimationWithSpeed(coord, action, engine.AnimSpeedNormal)
}

// NewWebAnimationWithSpeed creates a new WebAnimation with a specified speed setting.
func NewWebAnimationWithSpeed(coord engine.Coord, action WebAction, speed int) *WebAnimation {
	d := FrameDuration(speed)
	if speed < 0 || speed > 3 {
		d = 120 * time.Millisecond
	}

	var stages []string
	var styles []lipgloss.Style

	switch action {
	case WebActionWeave:
		// Weave pulse: faint filament -> strand expansion -> high energy pulse -> stabilized mesh
		stages = []string{" : ", "·:·", ":::", ":::"}
		styles = []lipgloss.Style{
			lipgloss.NewStyle().Foreground(lipgloss.Color("#06B6D4")),
			lipgloss.NewStyle().Bold(true).Foreground(lipgloss.Color("#22D3EE")),
			lipgloss.NewStyle().Bold(true).Foreground(lipgloss.Color("#67E8F9")),
			lipgloss.NewStyle().Bold(true).Foreground(lipgloss.Color("#22D3EE")),
		}

	case WebActionBreach:
		// Breach fracture: stressed lattice -> fracture burst -> splinter particles -> fading embers -> empty
		stages = []string{":::", "*#*", "·*·", " · ", " . "}
		styles = []lipgloss.Style{
			lipgloss.NewStyle().Bold(true).Foreground(lipgloss.Color("#F43F5E")),
			lipgloss.NewStyle().Bold(true).Foreground(lipgloss.Color("#FFD700")),
			lipgloss.NewStyle().Bold(true).Foreground(lipgloss.Color("#FB923C")),
			lipgloss.NewStyle().Foreground(lipgloss.Color("#94A3B8")),
			lipgloss.NewStyle().Foreground(lipgloss.Color("#64748B")),
		}

	default:
		stages = []string{":::"}
		styles = []lipgloss.Style{lipgloss.NewStyle().Bold(true).Foreground(lipgloss.Color("#22D3EE"))}
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

	anim := &WebAnimation{
		coord:   coord,
		action:  action,
		stages:  stages,
		frames:  frames,
	}
	if speed == engine.AnimSpeedOff || d == 0 {
		anim.finished = true
		anim.current = len(frames)
	}
	return anim
}

// Coord returns the sector coordinate where the web animation occurs.
func (w *WebAnimation) Coord() engine.Coord {
	return w.coord
}

// Action returns the WebAction performed by this animation.
func (w *WebAnimation) Action() WebAction {
	return w.action
}

// CurrentGlyph returns the visual glyph for the current stage.
func (w *WebAnimation) CurrentGlyph() string {
	if w.current < len(w.stages) {
		return w.stages[w.current]
	}
	if len(w.stages) > 0 {
		return w.stages[len(w.stages)-1]
	}
	return " . "
}

// Done reports whether all animation stages have completed.
func (w *WebAnimation) Done() bool {
	return w.finished || w.current >= len(w.frames)
}

// IsFinished satisfies the Animation interface.
func (w *WebAnimation) IsFinished() bool {
	return w.Done()
}

// Tick advances to the next stage and returns the corresponding frame.
func (w *WebAnimation) Tick() Frame {
	if w.Done() {
		w.finished = true
		return Frame{}
	}
	f := w.frames[w.current]
	w.current++
	if w.current >= len(w.frames) {
		w.finished = true
	}
	return f
}

// Step satisfies the Animation interface by advancing a single step.
func (w *WebAnimation) Step() Frame {
	return w.Tick()
}

// Skip immediately completes the web animation.
func (w *WebAnimation) Skip() Frame {
	w.current = len(w.frames)
	w.finished = true
	return Frame{}
}

// TotalDuration satisfies the Animation interface by returning the sum of frame durations.
func (w *WebAnimation) TotalDuration() time.Duration {
	var total time.Duration
	for _, f := range w.frames {
		total += f.Duration
	}
	return total
}

// Frames satisfies the Animation interface by returning all precomputed frames.
func (w *WebAnimation) Frames() []Frame {
	return w.frames
}
