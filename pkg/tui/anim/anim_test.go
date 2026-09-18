package anim

import (
	"testing"
	"time"

	"github.com/charmbracelet/lipgloss"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
)

func TestBresenhamLine(t *testing.T) {
	// Cardinal horizontal East
	pEast := BresenhamLine(engine.Coord{2, 2}, engine.Coord{2, 5})
	if len(pEast) != 4 {
		t.Fatalf("expected 4 points for horizontal line, got %d", len(pEast))
	}
	if pEast[0] != (engine.Coord{2, 2}) || pEast[3] != (engine.Coord{2, 5}) {
		t.Errorf("line endpoints mismatch: %v", pEast)
	}

	// Cardinal horizontal West
	pWest := BresenhamLine(engine.Coord{2, 5}, engine.Coord{2, 2})
	if len(pWest) != 4 {
		t.Fatalf("expected 4 points for west line, got %d", len(pWest))
	}
	if pWest[0] != (engine.Coord{2, 5}) || pWest[3] != (engine.Coord{2, 2}) {
		t.Errorf("line endpoints mismatch: %v", pWest)
	}

	// Cardinal vertical South
	pSouth := BresenhamLine(engine.Coord{1, 3}, engine.Coord{4, 3})
	if len(pSouth) != 4 {
		t.Fatalf("expected 4 points for south line, got %d", len(pSouth))
	}
	if pSouth[0] != (engine.Coord{1, 3}) || pSouth[3] != (engine.Coord{4, 3}) {
		t.Errorf("line endpoints mismatch: %v", pSouth)
	}

	// Cardinal vertical North
	pNorth := BresenhamLine(engine.Coord{4, 3}, engine.Coord{1, 3})
	if len(pNorth) != 4 {
		t.Fatalf("expected 4 points for north line, got %d", len(pNorth))
	}
	if pNorth[0] != (engine.Coord{4, 3}) || pNorth[3] != (engine.Coord{1, 3}) {
		t.Errorf("line endpoints mismatch: %v", pNorth)
	}

	// Diagonal South-East
	pDiag := BresenhamLine(engine.Coord{1, 1}, engine.Coord{4, 4})
	if len(pDiag) != 4 {
		t.Fatalf("expected 4 points for diagonal line, got %d", len(pDiag))
	}
	for i, pt := range pDiag {
		if pt.Row() != i+1 || pt.Col() != i+1 {
			t.Errorf("unexpected point %d in diagonal line: %v", i, pt)
		}
	}

	// Diagonal North-West
	pDiagNW := BresenhamLine(engine.Coord{4, 4}, engine.Coord{1, 1})
	if len(pDiagNW) != 4 {
		t.Fatalf("expected 4 points for diagonal line, got %d", len(pDiagNW))
	}
	if pDiagNW[0] != (engine.Coord{4, 4}) || pDiagNW[3] != (engine.Coord{1, 1}) {
		t.Errorf("line endpoints mismatch: %v", pDiagNW)
	}

	// Single point (start == end)
	pSingle := BresenhamLine(engine.Coord{3, 3}, engine.Coord{3, 3})
	if len(pSingle) != 1 || pSingle[0] != (engine.Coord{3, 3}) {
		t.Errorf("expected 1 point for single coordinate, got %v", pSingle)
	}
}

func TestTorpedoAnimation_StepAndSkip(t *testing.T) {
	start := engine.Coord{1, 1}
	target := engine.Coord{1, 4}
	anim := NewTorpedoAnimation(start, target, true, 2) // Normal speed

	if anim.IsFinished() {
		t.Fatalf("expected animation to start unfinished")
	}

	if anim.TotalDuration() <= 0 {
		t.Fatalf("expected positive total duration, got %v", anim.TotalDuration())
	}

	frames := anim.Frames()
	if len(frames) == 0 {
		t.Fatalf("expected frames to be populated")
	}

	// Step 1: In-flight trajectory
	f1 := anim.Step()
	if len(f1.Overrides) == 0 {
		t.Errorf("expected overrides in step 1")
	}

	// Skip to terminal
	finalFrame := anim.Skip()
	if !anim.IsFinished() {
		t.Errorf("expected animation to be finished after Skip()")
	}
	_ = finalFrame
}

func TestTorpedoAnimation_ImpactAndMiss(t *testing.T) {
	start := engine.Coord{1, 1}
	target := engine.Coord{1, 3}

	// Hit animation should finish with shockwave frames
	animHit := NewTorpedoAnimation(start, target, true, 2)
	var lastFrame Frame
	for !animHit.IsFinished() {
		lastFrame = animHit.Step()
	}
	ov, ok := lastFrame.Overrides[target]
	if !ok {
		t.Fatalf("expected final override at target %v", target)
	}
	if ov.Glyph != "#*#" {
		t.Errorf("expected final shockwave frame '#*#', got %q", ov.Glyph)
	}

	// Miss animation should end in empty space
	animMiss := NewTorpedoAnimation(start, target, false, 2)
	var lastMissFrame Frame
	for !animMiss.IsFinished() {
		lastMissFrame = animMiss.Step()
	}
	ovMiss, ok := lastMissFrame.Overrides[target]
	if !ok {
		t.Fatalf("expected final override at target %v on miss", target)
	}
	if ovMiss.Glyph != "   " {
		t.Errorf("expected empty space '   ' on miss, got %q", ovMiss.Glyph)
	}
}

func TestPhaserAnimation_DirectionalGlyphs(t *testing.T) {
	// Horizontal
	startH := engine.Coord{3, 1}
	targetH := engine.Coord{3, 5}
	animH := NewPhaserAnimation(startH, targetH, true, 2)

	fH := animH.Step()
	intermediateH := engine.Coord{3, 3}
	ovH, ok := fH.Overrides[intermediateH]
	if !ok {
		t.Fatalf("expected override along beam path at %v", intermediateH)
	}
	if ovH.Glyph != "---" {
		t.Errorf("expected horizontal beam '---', got %q", ovH.Glyph)
	}
	// Target should have shield bracket
	ovTarget, ok := fH.Overrides[targetH]
	if !ok {
		t.Fatalf("expected override at target %v", targetH)
	}
	if ovTarget.Glyph != "<K>" {
		t.Errorf("expected shield bracket '<K>', got %q", ovTarget.Glyph)
	}

	// Vertical
	startV := engine.Coord{1, 4}
	targetV := engine.Coord{5, 4}
	animV := NewPhaserAnimation(startV, targetV, true, 2)
	fV := animV.Step()
	ovV, ok := fV.Overrides[engine.Coord{3, 4}]
	if !ok {
		t.Fatalf("expected override along vertical beam path")
	}
	if ovV.Glyph != " | " {
		t.Errorf("expected vertical beam ' | ', got %q", ovV.Glyph)
	}

	// Diagonal South-East (\)
	startSE := engine.Coord{1, 1}
	targetSE := engine.Coord{4, 4}
	animSE := NewPhaserAnimation(startSE, targetSE, true, 2)
	fSE := animSE.Step()
	ovSE, ok := fSE.Overrides[engine.Coord{2, 2}]
	if !ok {
		t.Fatalf("expected override along diagonal SE beam path")
	}
	if ovSE.Glyph != " \\ " {
		t.Errorf("expected diagonal beam ' \\ ', got %q", ovSE.Glyph)
	}

	// Diagonal South-West (/)
	startSW := engine.Coord{1, 4}
	targetSW := engine.Coord{4, 1}
	animSW := NewPhaserAnimation(startSW, targetSW, true, 2)
	fSW := animSW.Step()
	ovSW, ok := fSW.Overrides[engine.Coord{2, 3}]
	if !ok {
		t.Fatalf("expected override along diagonal SW beam path")
	}
	if ovSW.Glyph != " / " {
		t.Errorf("expected diagonal beam ' / ', got %q", ovSW.Glyph)
	}

	// 2 frames total
	animCount := NewPhaserAnimation(startH, targetH, true, 2)
	if len(animCount.Frames()) != 2 {
		t.Errorf("expected 2 frames for phaser animation, got %d", len(animCount.Frames()))
	}
	animCount.Step()
	if animCount.IsFinished() {
		t.Errorf("expected not finished after 1 frame")
	}
	animCount.Step()
	if !animCount.IsFinished() {
		t.Errorf("expected finished after 2 frames")
	}
}

func TestRedAlertBadgeStyle(t *testing.T) {
	s0 := RedAlertBadgeStyle(0)
	s1 := RedAlertBadgeStyle(1)
	s2 := RedAlertBadgeStyle(2)

	if s0.GetForeground() == s1.GetForeground() {
		t.Errorf("expected cycle 0 and cycle 1 to have alternating foreground colors")
	}
	if s0.GetForeground() != s2.GetForeground() {
		t.Errorf("expected cycle 0 and cycle 2 to match (periodic oscillation)")
	}
	if !s0.GetBold() || s1.GetBold() {
		t.Errorf("expected cycle 0 to be bold and cycle 1 not to be bold")
	}
}

func TestTickCmd(t *testing.T) {
	cmd := TickCmd(42, 3, 10*time.Millisecond)
	if cmd == nil {
		t.Fatalf("expected non-nil tea.Cmd")
	}
	msg := cmd()
	tickMsg, ok := msg.(TickMsg)
	if !ok {
		t.Fatalf("expected msg to be TickMsg, got %T", msg)
	}
	if tickMsg.AnimID != 42 || tickMsg.Step != 3 {
		t.Errorf("unexpected TickMsg values: %+v", tickMsg)
	}
}

func TestCellOverride_Properties(t *testing.T) {
	co := CellOverride{
		Glyph: " · ",
		Style: lipgloss.NewStyle().Bold(true),
	}
	if co.Glyph != " · " {
		t.Errorf("expected ' · ', got %q", co.Glyph)
	}
}

func TestFrameDuration(t *testing.T) {
	tests := []struct {
		name     string
		speed    int
		expected time.Duration
	}{
		{"AnimSpeedOff", engine.AnimSpeedOff, 0},
		{"AnimSpeedFast", engine.AnimSpeedFast, 40 * time.Millisecond},
		{"AnimSpeedNormal", engine.AnimSpeedNormal, 80 * time.Millisecond},
		{"AnimSpeedCinematic", engine.AnimSpeedCinematic, 160 * time.Millisecond},
		{"Unexpected negative", -1, 80 * time.Millisecond},
		{"Unexpected 4", 4, 80 * time.Millisecond},
		{"Unexpected large", 999, 80 * time.Millisecond},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			got := FrameDuration(tc.speed)
			if got != tc.expected {
				t.Errorf("FrameDuration(%d) = %v; want %v", tc.speed, got, tc.expected)
			}
		})
	}
}

