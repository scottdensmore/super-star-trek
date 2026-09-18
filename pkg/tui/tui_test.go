package tui

import (
	"strings"
	"testing"

	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/commandbar"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

func TestModel_RedAlertPulse(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	g.Enterprise.Condition = engine.ConditionRed
	g.Rules.AnimSpeed = engine.AnimSpeedNormal

	m := NewModel(g, theme.DefaultTheme())

	// Verify Init schedules red alert pulse when Condition is Red
	initCmd := m.Init()
	if initCmd == nil {
		t.Fatalf("expected non-nil Init cmd when condition is Red")
	}

	if m.redAlertCycle != 0 {
		t.Errorf("expected initial redAlertCycle 0, got %d", m.redAlertCycle)
	}

	// Dispatch RedAlertPulseMsg
	updated, nextCmd := m.Update(RedAlertPulseMsg{})
	m = updated.(Model)

	if m.redAlertCycle != 1 {
		t.Fatalf("expected redAlertCycle 1 after first pulse, got %d", m.redAlertCycle)
	}
	if nextCmd == nil {
		t.Fatalf("expected recurring tick command after RedAlertPulseMsg")
	}

	// Dispatch second RedAlertPulseMsg
	updated, nextCmd = m.Update(RedAlertPulseMsg{})
	m = updated.(Model)

	if m.redAlertCycle != 2 {
		t.Fatalf("expected redAlertCycle 2 after second pulse, got %d", m.redAlertCycle)
	}
	if nextCmd == nil {
		t.Fatalf("expected recurring tick command after second pulse")
	}

	// View rendering should incorporate RedAlertBadgeStyle
	view := m.View()
	if !strings.Contains(view, "CONDITION RED") {
		t.Errorf("expected view to contain CONDITION RED, got:\n%s", view)
	}

	// When AnimSpeed is turned off, RedAlertPulseMsg stops recurring
	m.Game.Rules.AnimSpeed = engine.AnimSpeedOff
	updated, stopCmd := m.Update(RedAlertPulseMsg{})
	m = updated.(Model)
	if stopCmd != nil {
		t.Errorf("expected nil cmd when AnimSpeed is Off")
	}

	// When condition transitions to Red after action, pulse is scheduled
	m.Game.Rules.AnimSpeed = engine.AnimSpeedNormal
	prevCond := engine.ConditionGreen
	m.Game.Enterprise.Condition = engine.ConditionRed
	m.redAlertActive = false

	cmd := m.checkRedAlertCmd(prevCond)
	if cmd == nil {
		t.Errorf("expected pulse cmd scheduled on condition change to Red")
	}
}

func TestModel_MultiTargetPhaserAnimation(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	g.Enterprise.Sector = engine.Coord{4, 4}
	g.Enterprise.Energy = 3000
	g.Rules.AnimSpeed = engine.AnimSpeedNormal

	// Place 2 Klingons in the quadrant
	k1 := &engine.Klingon{ID: 1, Sector: engine.Coord{4, 7}, Energy: 200}
	k2 := &engine.Klingon{ID: 2, Sector: engine.Coord{1, 4}, Energy: 200}
	g.CurrentQuad.Klingons = []*engine.Klingon{k1, k2}
	g.CurrentQuad.Grid[4][7] = engine.EntityKlingon
	g.CurrentQuad.Grid[1][4] = engine.EntityKlingon

	m := NewModel(g, theme.DefaultTheme())

	// Fire phasers with enough energy to hit both targets
	updated, cmd := m.Update(commandbar.CommandSubmittedMsg{Text: "pha 400"})
	m = updated.(Model)

	if cmd == nil {
		t.Fatalf("expected non-nil animation cmd on phaser fire")
	}
	if m.activeAnim == nil {
		t.Fatalf("expected activeAnim populated for multi-target phasers")
	}

	frames := m.activeAnim.Frames()
	if len(frames) != 2 {
		t.Fatalf("expected 2 frames for multi-target phaser animation, got %d", len(frames))
	}

	f1 := frames[0]

	// Check target 1 bracket override
	ov1, ok1 := f1.Overrides[k1.Sector]
	if !ok1 || ov1.Glyph != "<K>" {
		t.Errorf("expected hit target override '<K>' at %v, got %+v", k1.Sector, ov1)
	}

	// Check target 2 bracket override
	ov2, ok2 := f1.Overrides[k2.Sector]
	if !ok2 || ov2.Glyph != "<K>" {
		t.Errorf("expected hit target override '<K>' at %v, got %+v", k2.Sector, ov2)
	}

	// Check intermediate beam raycast paths for both targets
	// k1 horizontal: intermediate {4, 5}
	if ovBeam1, ok := f1.Overrides[engine.Coord{4, 5}]; !ok || ovBeam1.Glyph != "---" {
		t.Errorf("expected horizontal beam override '---' at {4, 5}, got %+v", ovBeam1)
	}

	// k2 vertical: intermediate {2, 4}
	if ovBeam2, ok := f1.Overrides[engine.Coord{2, 4}]; !ok || ovBeam2.Glyph != " | " {
		t.Errorf("expected vertical beam override ' | ' at {2, 4}, got %+v", ovBeam2)
	}
}
