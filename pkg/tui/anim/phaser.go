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

// NewMultiPhaserAnimation creates a multi-target phaser visual raycast and shield flash animation.
func NewMultiPhaserAnimation(start engine.Coord, targets []engine.Coord, hits []bool, speed int) Animation {
	d := FrameDuration(speed)
	beamStyle := lipgloss.NewStyle().Foreground(lipgloss.Color("11")).Bold(true)
	targetStyle := lipgloss.NewStyle().Foreground(lipgloss.Color("14")).Bold(true)

	overrides := make(map[engine.Coord]CellOverride)

	for i, target := range targets {
		hit := false
		if i < len(hits) {
			hit = hits[i]
		}

		line := BresenhamLine(start, target)
		dr := target.Row() - start.Row()
		dc := target.Col() - start.Col()
		glyph := beamGlyph(dr, dc)

		if len(line) > 2 {
			for _, c := range line[1 : len(line)-1] {
				overrides[c] = CellOverride{Glyph: glyph, Style: beamStyle}
			}
		}
		if hit {
			overrides[target] = CellOverride{Glyph: "<K>", Style: targetStyle}
		}
	}

	// 4 frames of beam discharge and sustained shield impact
	frames := make([]Frame, 4)
	for i := range frames {
		fOverrides := make(map[engine.Coord]CellOverride, len(overrides))
		for k, v := range overrides {
			fOverrides[k] = v
		}
		if i == 1 || i == 2 {
			for _, target := range targets {
				if hitTarget, ok := fOverrides[target]; ok && hitTarget.Glyph == "<K>" {
					fOverrides[target] = CellOverride{
						Glyph: "*K*",
						Style: lipgloss.NewStyle().Foreground(lipgloss.Color("11")).Bold(true),
					}
				}
			}
		}
		frames[i] = Frame{Overrides: fOverrides, Duration: d}
	}

	return &phaserAnimation{
		frames: frames,
	}
}

// NewPhaserAnimation creates a new phaser visual raycast and shield flash animation.
func NewPhaserAnimation(start, target engine.Coord, hit bool, speed int) Animation {
	return NewMultiPhaserAnimation(start, []engine.Coord{target}, []bool{hit}, speed)
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
