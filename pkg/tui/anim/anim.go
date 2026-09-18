package anim

import (
	"time"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
)

// CellOverride represents a temporary visual glyph and style replacing a grid cell.
type CellOverride struct {
	Glyph string         // Exact 3-rune cell glyph (e.g. " · ", " o ", " * ", "***", "---", " \\ ")
	Style lipgloss.Style // Foreground/background styling
}

// Frame represents the visual state of all overridden cells at a single animation step.
type Frame struct {
	Overrides map[engine.Coord]CellOverride
	Duration  time.Duration
}

// Animation defines an interactive visual sequence.
type Animation interface {
	TotalDuration() time.Duration
	Frames() []Frame
	IsFinished() bool
	Step() Frame
	Skip() Frame
}

// TickMsg is dispatched via tea.Tick for frame advancement.
type TickMsg struct {
	AnimID int
	Step   int
}

// TickCmd schedules a tick message for the animation engine.
func TickCmd(animID, step int, d time.Duration) tea.Cmd {
	return tea.Tick(d, func(t time.Time) tea.Msg {
		return TickMsg{AnimID: animID, Step: step}
	})
}

// FrameDuration returns the standard frame duration based on the speed setting.
// 0 = Off (0ms), 1 = Fast (~40ms), 2 = Normal (~80ms), 3 = Cinematic (~160ms).
func FrameDuration(speed int) time.Duration {
	switch speed {
	case 0:
		return 0
	case 1:
		return 40 * time.Millisecond
	case 2:
		return 80 * time.Millisecond
	case 3:
		return 160 * time.Millisecond
	default:
		return 80 * time.Millisecond
	}
}
