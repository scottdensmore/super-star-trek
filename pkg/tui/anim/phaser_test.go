package anim

import (
	"testing"
	"time"

	"github.com/scottdensmore/super-star-trek/pkg/engine"
)

func TestNewMultiPhaserAnimation_MultipleTargets(t *testing.T) {
	start := engine.Coord{4, 4}
	target1 := engine.Coord{4, 7} // Horizontal East
	target2 := engine.Coord{1, 4} // Vertical North
	target3 := engine.Coord{7, 7} // Diagonal South-East

	targets := []engine.Coord{target1, target2, target3}
	hits := []bool{true, false, true}

	anim := NewMultiPhaserAnimation(start, targets, hits, engine.AnimSpeedNormal)

	if anim.IsFinished() {
		t.Fatalf("expected animation to be unfinished initially")
	}

	expectedDuration := 2 * 80 * time.Millisecond
	if anim.TotalDuration() != expectedDuration {
		t.Errorf("expected total duration %v, got %v", expectedDuration, anim.TotalDuration())
	}

	frames := anim.Frames()
	if len(frames) != 2 {
		t.Fatalf("expected 2 frames, got %d", len(frames))
	}

	f := anim.Step()

	// Check target 1: horizontal beam at {4, 5} and {4, 6}, and <K> at {4, 7}
	ovBeam1, ok := f.Overrides[engine.Coord{4, 5}]
	if !ok || ovBeam1.Glyph != "---" {
		t.Errorf("expected horizontal beam '---' at {4, 5}, got %+v", ovBeam1)
	}
	ovTarget1, ok := f.Overrides[target1]
	if !ok || ovTarget1.Glyph != "<K>" {
		t.Errorf("expected hit target glyph '<K>' at target1, got %+v", ovTarget1)
	}

	// Check target 2: vertical beam at {2, 4} and {3, 4}, but hit is false so no <K>
	ovBeam2, ok := f.Overrides[engine.Coord{2, 4}]
	if !ok || ovBeam2.Glyph != " | " {
		t.Errorf("expected vertical beam ' | ' at {2, 4}, got %+v", ovBeam2)
	}
	if _, hasHitTarget2 := f.Overrides[target2]; hasHitTarget2 {
		t.Errorf("expected no target override for missed target2")
	}

	// Check target 3: diagonal beam at {5, 5} and {6, 6}, and <K> at {7, 7}
	ovBeam3, ok := f.Overrides[engine.Coord{5, 5}]
	if !ok || ovBeam3.Glyph != " \\ " {
		t.Errorf("expected diagonal beam ' \\ ' at {5, 5}, got %+v", ovBeam3)
	}
	ovTarget3, ok := f.Overrides[target3]
	if !ok || ovTarget3.Glyph != "<K>" {
		t.Errorf("expected hit target glyph '<K>' at target3, got %+v", ovTarget3)
	}

	// Second frame
	f2 := anim.Step()
	if len(f2.Overrides) != len(f.Overrides) {
		t.Errorf("expected frame 2 to have same overrides as frame 1")
	}

	if !anim.IsFinished() {
		t.Errorf("expected animation to be finished after 2 steps")
	}
}

func TestNewMultiPhaserAnimation_EmptyTargets(t *testing.T) {
	anim := NewMultiPhaserAnimation(engine.Coord{4, 4}, nil, nil, engine.AnimSpeedFast)
	if len(anim.Frames()) != 2 {
		t.Fatalf("expected 2 frames for empty targets")
	}
	f := anim.Step()
	if len(f.Overrides) != 0 {
		t.Errorf("expected 0 overrides for empty targets, got %d", len(f.Overrides))
	}
}

func TestNewMultiPhaserAnimation_Skip(t *testing.T) {
	start := engine.Coord{4, 4}
	targets := []engine.Coord{{4, 7}}
	anim := NewMultiPhaserAnimation(start, targets, []bool{true}, engine.AnimSpeedCinematic)

	anim.Skip()
	if !anim.IsFinished() {
		t.Errorf("expected animation to be finished after Skip()")
	}
}

func TestNewPhaserAnimation_Delegation(t *testing.T) {
	start := engine.Coord{3, 3}
	target := engine.Coord{3, 6}

	anim := NewPhaserAnimation(start, target, true, engine.AnimSpeedNormal)
	if anim.IsFinished() {
		t.Fatalf("expected animation to start unfinished")
	}
	f := anim.Step()
	if ov, ok := f.Overrides[target]; !ok || ov.Glyph != "<K>" {
		t.Errorf("expected hit override '<K>' at target, got %+v", ov)
	}
}
