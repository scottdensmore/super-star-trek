package anim

import (
	"time"

	"github.com/charmbracelet/lipgloss"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
)

type phaserAnimation struct {
	frames   []Frame
	current  int
	finished bool
}

// beamGlyph calculates the directional ray glyph based on delta row and delta col.
func beamGlyph(dr, dc int) string {
	absR := abs(dr)
	absC := abs(dc)
	if absR == 0 {
		return "---"
	}
	if absC == 0 {
		return " | "
	}
	if absC >= 2*absR {
		return "---"
	}
	if absR >= 2*absC {
		return " | "
	}
	if (dr > 0 && dc > 0) || (dr < 0 && dc < 0) {
		return " \\ "
	}
	return " / "
}

// NewPhaserAnimation creates a new phaser visual raycast and shield flash animation.
func NewPhaserAnimation(start, target engine.Coord, hit bool, speed int) Animation {
	d := FrameDuration(speed)
	line := BresenhamLine(start, target)

	dr := target.Row() - start.Row()
	dc := target.Col() - start.Col()
	glyph := beamGlyph(dr, dc)

	beamStyle := lipgloss.NewStyle().Foreground(lipgloss.Color("11")).Bold(true)
	targetStyle := lipgloss.NewStyle().Foreground(lipgloss.Color("14")).Bold(true)

	var intermediates []engine.Coord
	if len(line) > 2 {
		intermediates = line[1 : len(line)-1]
	}

	overrides := make(map[engine.Coord]CellOverride)
	for _, c := range intermediates {
		overrides[c] = CellOverride{Glyph: glyph, Style: beamStyle}
	}
	if hit {
		overrides[target] = CellOverride{Glyph: "<K>", Style: targetStyle}
	}

	// 2 frames of beam discharge
	f1Overrides := make(map[engine.Coord]CellOverride, len(overrides))
	for k, v := range overrides {
		f1Overrides[k] = v
	}
	f2Overrides := make(map[engine.Coord]CellOverride, len(overrides))
	for k, v := range overrides {
		f2Overrides[k] = v
	}

	frames := []Frame{
		{Overrides: f1Overrides, Duration: d},
		{Overrides: f2Overrides, Duration: d},
	}

	return &phaserAnimation{
		frames: frames,
	}
}

func (a *phaserAnimation) TotalDuration() time.Duration {
	var total time.Duration
	for _, f := range a.frames {
		total += f.Duration
	}
	return total
}

func (a *phaserAnimation) Frames() []Frame {
	return a.frames
}

func (a *phaserAnimation) IsFinished() bool {
	return a.finished || a.current >= len(a.frames)
}

func (a *phaserAnimation) Step() Frame {
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

func (a *phaserAnimation) Skip() Frame {
	a.current = len(a.frames)
	a.finished = true
	return Frame{}
}
