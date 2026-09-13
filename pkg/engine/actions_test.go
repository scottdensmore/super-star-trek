package engine

import (
	"testing"
)

func TestActionDock_SurveillanceModes(t *testing.T) {
	// 1. Full surveillance reveals all 64
	gFull := NewGameWithOptions(100, SkillGood, LengthMedium, GameRules{Surveillance: SurveillanceFull})
	gFull.CurrentQuad.Starbase = &Coord{4, 4}
	gFull.Enterprise.Sector = Coord{4, 5}
	eventsFull, err := ActionDock{}.Execute(gFull)
	if err != nil {
		t.Fatalf("ActionDock failed: %v", err)
	}
	discoveredCount := 0
	for r := 1; r <= 8; r++ {
		for c := 1; c <= 8; c++ {
			if gFull.ChartDiscovered[r][c] {
				discoveredCount++
			}
		}
	}
	if discoveredCount != 64 {
		t.Errorf("SurveillanceFull: expected 64 discovered quads, got %d", discoveredCount)
	}
	survFull := eventsFull[1].(EventStarbaseSurveillance)
	if survFull.Mode != SurveillanceFull {
		t.Errorf("SurveillanceFull: expected Mode=%v, got %v", SurveillanceFull, survFull.Mode)
	}

	// 2. Local surveillance reveals only docked base 3x3
	gLocal := NewGameWithOptions(200, SkillGood, LengthMedium, GameRules{Surveillance: SurveillanceLocal})
	// Reset discovered except starting
	for r := 1; r <= 8; r++ {
		for c := 1; c <= 8; c++ {
			gLocal.ChartDiscovered[r][c] = false
		}
	}
	sbQuad := Coord{2, 2}
	gLocal.Enterprise.Quad = sbQuad
	gLocal.CurrentQuad.Starbase = &Coord{5, 5}
	gLocal.Enterprise.Sector = Coord{5, 6}
	eventsLocal, err := ActionDock{}.Execute(gLocal)
	if err != nil {
		t.Fatalf("ActionDock failed: %v", err)
	}
	for r := 1; r <= 8; r++ {
		for c := 1; c <= 8; c++ {
			inLocal := r >= 1 && r <= 3 && c >= 1 && c <= 3
			if gLocal.ChartDiscovered[r][c] != inLocal {
				t.Errorf("SurveillanceLocal quad [%d,%d]: got discovered=%v, want %v", r, c, gLocal.ChartDiscovered[r][c], inLocal)
			}
		}
	}
	survLocal := eventsLocal[1].(EventStarbaseSurveillance)
	if survLocal.Mode != SurveillanceLocal {
		t.Errorf("SurveillanceLocal: expected Mode=%v, got %v", SurveillanceLocal, survLocal.Mode)
	}

	// 3. Blackout surveillance reveals 0 additional quadrants
	gBlack := NewGameWithOptions(300, SkillGood, LengthMedium, GameRules{Surveillance: SurveillanceBlackout})
	for r := 1; r <= 8; r++ {
		for c := 1; c <= 8; c++ {
			gBlack.ChartDiscovered[r][c] = false
		}
	}
	gBlack.Enterprise.Quad = Coord{3, 3}
	gBlack.CurrentQuad.Starbase = &Coord{1, 1}
	gBlack.Enterprise.Sector = Coord{1, 2}
	events, err := ActionDock{}.Execute(gBlack)
	if err != nil {
		t.Fatalf("ActionDock failed: %v", err)
	}
	survEvent := events[1].(EventStarbaseSurveillance)
	if survEvent.UpdatedQuads != 0 {
		t.Errorf("SurveillanceBlackout: expected 0 updated quads, got %d", survEvent.UpdatedQuads)
	}
	if survEvent.Mode != SurveillanceBlackout {
		t.Errorf("SurveillanceBlackout: expected Mode=%v, got %v", SurveillanceBlackout, survEvent.Mode)
	}
}

func TestActionLRScan_DegradedWarning(t *testing.T) {
	g := NewGameWithOptions(100, SkillGood, LengthMedium, DefaultRulesForProfile(ProfileNormal))
	g.Enterprise.Devices[DeviceLRSensors] = 1.2 // Light damage
	events, err := ActionLRScan{}.Execute(g)
	if err != nil {
		t.Fatalf("ActionLRScan should succeed with degraded warning: %v", err)
	}
	foundWarning := false
	for _, ev := range events {
		if lrs, ok := ev.(EventLRScanCompleted); ok && lrs.Degraded {
			foundWarning = true
		}
	}
	if !foundWarning {
		t.Errorf("expected EventLRScanCompleted with Degraded=true for light sensor damage")
	}
}
