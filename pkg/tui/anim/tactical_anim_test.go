package anim

import (
	"testing"
	"time"

	"github.com/scottdensmore/super-star-trek/pkg/engine"
)

func TestPlasmaAnimation_Progression(t *testing.T) {
	from := engine.Coord{2, 2}
	to := engine.Coord{4, 4}
	p := NewPlasmaAnimation(from, to, 100*time.Millisecond)
	if p.Done() {
		t.Fatalf("expected new animation to not be done")
	}
	p.Tick()
	p.Tick()
	p.Tick()
	if !p.Done() {
		t.Errorf("expected animation to complete after ticks")
	}
}

func TestPlasmaAnimation_AnimationInterface(t *testing.T) {
	from := engine.Coord{1, 1}
	to := engine.Coord{1, 3}
	p := NewPlasmaAnimation(from, to, 50*time.Millisecond)

	if p.From() != from || p.To() != to {
		t.Errorf("expected from=%v to=%v, got from=%v to=%v", from, to, p.From(), p.To())
	}
	if p.IsFinished() {
		t.Errorf("expected not finished initially")
	}
	if p.TotalDuration() != 150*time.Millisecond {
		t.Errorf("expected 150ms total duration, got %v", p.TotalDuration())
	}
	if len(p.Frames()) != 3 {
		t.Errorf("expected 3 frames, got %d", len(p.Frames()))
	}
	if p.CurrentCoord() != from {
		t.Errorf("expected current coord to start at from %v, got %v", from, p.CurrentCoord())
	}

	f1 := p.Step()
	if ov, ok := f1.Overrides[from]; !ok || ov.Glyph != "*P*" {
		t.Errorf("expected '*P*' glyph at %v, got %v", from, ov)
	}

	p.Skip()
	if !p.IsFinished() || !p.Done() {
		t.Errorf("expected animation finished after Skip()")
	}
}

func TestCloakAnimation_Stages(t *testing.T) {
	c := NewCloakAnimation(engine.Coord{3, 3}, true) // true = cloaking
	stages := []string{}
	for !c.Done() {
		stages = append(stages, c.CurrentGlyph())
		c.Tick()
	}
	if len(stages) < 3 {
		t.Errorf("expected at least 3 cloak shimmer stages, got %d", len(stages))
	}
	// Cloaking expected sequence: +R+ -> ~R~ -> ·?· ->  . 
	expected := []string{"+R+", "~R~", "·?·", " . "}
	if len(stages) != len(expected) {
		t.Fatalf("expected %d stages, got %d", len(expected), len(stages))
	}
	for i, exp := range expected {
		if stages[i] != exp {
			t.Errorf("stage %d: expected %q, got %q", i, exp, stages[i])
		}
	}
}

func TestCloakAnimation_Decloak(t *testing.T) {
	coord := engine.Coord{4, 4}
	c := NewCloakAnimation(coord, false) // false = decloaking
	if c.IsCloaking() {
		t.Errorf("expected IsCloaking() to be false")
	}
	if c.Coord() != coord {
		t.Errorf("expected coord %v, got %v", coord, c.Coord())
	}

	stages := []string{}
	for !c.Done() {
		stages = append(stages, c.CurrentGlyph())
		c.Tick()
	}
	expected := []string{" . ", "·?·", "~R~", "+R+"}
	if len(stages) != len(expected) {
		t.Fatalf("expected %d stages, got %d", len(expected), len(stages))
	}
	for i, exp := range expected {
		if stages[i] != exp {
			t.Errorf("stage %d: expected %q, got %q", i, exp, stages[i])
		}
	}
}

func TestCloakAnimation_AnimationInterface(t *testing.T) {
	c := NewCloakAnimation(engine.Coord{2, 2}, true)
	if c.IsFinished() {
		t.Errorf("expected not finished initially")
	}
	if len(c.Frames()) != 4 {
		t.Errorf("expected 4 frames, got %d", len(c.Frames()))
	}
	if c.TotalDuration() <= 0 {
		t.Errorf("expected positive duration")
	}

	f := c.Step()
	if ov, ok := f.Overrides[engine.Coord{2, 2}]; !ok || ov.Glyph != "+R+" {
		t.Errorf("expected initial override '+R+', got %v", ov)
	}

	c.Skip()
	if !c.IsFinished() || !c.Done() {
		t.Errorf("expected finished after Skip()")
	}
}

func TestWebAnimation_Pulse(t *testing.T) {
	w := NewWebAnimation(engine.Coord{5, 5}, WebActionWeave)
	if w.Done() {
		t.Fatalf("expected web animation to start active")
	}
	for !w.Done() {
		w.Tick()
	}
	if !w.Done() {
		t.Errorf("expected web animation to complete")
	}
}

func TestWebAnimation_Breach(t *testing.T) {
	coord := engine.Coord{6, 6}
	w := NewWebAnimation(coord, WebActionBreach)
	if w.Done() {
		t.Fatalf("expected breach animation to start active")
	}
	if w.Action() != WebActionBreach {
		t.Errorf("expected action WebActionBreach, got %v", w.Action())
	}
	if w.Coord() != coord {
		t.Errorf("expected coord %v, got %v", coord, w.Coord())
	}

	stages := []string{}
	for !w.Done() {
		stages = append(stages, w.CurrentGlyph())
		w.Tick()
	}
	if len(stages) < 3 {
		t.Errorf("expected at least 3 breach fracture stages, got %d", len(stages))
	}
	if !w.Done() {
		t.Errorf("expected breach animation to complete")
	}
}

func TestWebAnimation_AnimationInterface(t *testing.T) {
	w := NewWebAnimation(engine.Coord{3, 3}, WebActionWeave)
	if w.IsFinished() {
		t.Errorf("expected not finished initially")
	}
	if len(w.Frames()) == 0 {
		t.Errorf("expected frames to be populated")
	}
	if w.TotalDuration() <= 0 {
		t.Errorf("expected positive duration")
	}

	_ = w.Step()
	w.Skip()
	if !w.IsFinished() || !w.Done() {
		t.Errorf("expected finished after Skip()")
	}
}

func TestWebAction_String(t *testing.T) {
	if WebActionWeave.String() != "Weave" {
		t.Errorf("expected 'Weave', got %q", WebActionWeave.String())
	}
	if WebActionBreach.String() != "Breach" {
		t.Errorf("expected 'Breach', got %q", WebActionBreach.String())
	}
	if WebAction(99).String() != "Unknown" {
		t.Errorf("expected 'Unknown', got %q", WebAction(99).String())
	}
}

func TestTacticalAnimations_GlyphRuneWidths(t *testing.T) {
	p := NewPlasmaAnimation(engine.Coord{1, 1}, engine.Coord{2, 2}, 100*time.Millisecond)
	for _, f := range p.Frames() {
		for _, co := range f.Overrides {
			if len([]rune(co.Glyph)) != 3 {
				t.Errorf("plasma glyph %q is not 3 runes wide", co.Glyph)
			}
		}
	}

	c1 := NewCloakAnimation(engine.Coord{1, 1}, true)
	for _, f := range c1.Frames() {
		for _, co := range f.Overrides {
			if len([]rune(co.Glyph)) != 3 {
				t.Errorf("cloak glyph %q is not 3 runes wide", co.Glyph)
			}
		}
	}

	c2 := NewCloakAnimation(engine.Coord{1, 1}, false)
	for _, f := range c2.Frames() {
		for _, co := range f.Overrides {
			if len([]rune(co.Glyph)) != 3 {
				t.Errorf("decloak glyph %q is not 3 runes wide", co.Glyph)
			}
		}
	}

	w1 := NewWebAnimation(engine.Coord{1, 1}, WebActionWeave)
	for _, f := range w1.Frames() {
		for _, co := range f.Overrides {
			if len([]rune(co.Glyph)) != 3 {
				t.Errorf("web weave glyph %q is not 3 runes wide", co.Glyph)
			}
		}
	}

	w2 := NewWebAnimation(engine.Coord{1, 1}, WebActionBreach)
	for _, f := range w2.Frames() {
		for _, co := range f.Overrides {
			if len([]rune(co.Glyph)) != 3 {
				t.Errorf("web breach glyph %q is not 3 runes wide", co.Glyph)
			}
		}
	}
}
