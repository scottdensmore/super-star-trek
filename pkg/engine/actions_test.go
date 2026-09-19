package engine

import (
	"math"
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

func TestActionFirePhasers_DecloakOnHitAndCloakOnEvade(t *testing.T) {
	rules := GameRules{KlingonCloak: true}
	g := NewGameWithOptions(100, SkillGood, LengthMedium, rules)
	g.Enterprise.Energy = 3000
	g.Enterprise.Sector = Coord{1, 1}

	cmd1 := &Klingon{ID: 1, Sector: Coord{2, 2}, Energy: 500, IsCommander: true, IsCloaked: true}
	cmd2 := &Klingon{ID: 2, Sector: Coord{3, 3}, Energy: 500, IsCommander: true, IsCloaked: false}
	g.CurrentQuad.Klingons = []*Klingon{cmd1, cmd2}
	g.CurrentQuad.Grid[2][2] = EntityCommander
	g.CurrentQuad.Grid[3][3] = EntityCommander

	// Manual allocation: hit cmd1 (cloaked) with 200 energy, allocate 0 to cmd2 (uncloaked)
	act := ActionFirePhasers{
		ManualAllocation: map[int]float64{
			1: 200,
			2: 0,
		},
	}
	events, err := act.Execute(g)
	if err != nil {
		t.Fatalf("ActionFirePhasers failed: %v", err)
	}

	// cmd1 was hit and should be decloaked
	if cmd1.IsCloaked {
		t.Errorf("expected cmd1 to decloak after being hit by phasers")
	}
	// cmd2 was NOT targeted, so it evaded damage and should be cloaked
	if !cmd2.IsCloaked {
		t.Errorf("expected cmd2 to cloak after evading phaser damage")
	}

	// Verify events contain decloak for cmd1 and cloak for cmd2
	foundCmd1Decloak := false
	foundCmd2Cloak := false
	for _, ev := range events {
		if cEv, ok := ev.(EventKlingonCloakState); ok {
			if cEv.KlingonID == 1 && !cEv.Cloaked {
				foundCmd1Decloak = true
			}
			if cEv.KlingonID == 2 && cEv.Cloaked {
				foundCmd2Cloak = true
			}
		}
	}
	if !foundCmd1Decloak {
		t.Errorf("expected decloak event for cmd1")
	}
	if !foundCmd2Cloak {
		t.Errorf("expected cloak event for cmd2")
	}
}

func TestActionFireTorpedo_DecloakOnHitAndCloakOnEvade(t *testing.T) {
	rules := GameRules{KlingonCloak: true}
	g := NewGameWithOptions(100, SkillGood, LengthMedium, rules)
	g.Enterprise.Torpedoes = 10
	g.Enterprise.Sector = Coord{4, 1}

	// Cmd1 is cloaked in the path of the torpedo (due East from Enterprise at {4, 1} along row 4)
	cmd1 := &Klingon{ID: 1, Sector: Coord{4, 5}, Energy: 800, IsCommander: true, IsCloaked: true}
	// Cmd2 is uncloaked elsewhere in the quadrant
	cmd2 := &Klingon{ID: 2, Sector: Coord{7, 7}, Energy: 800, IsCommander: true, IsCloaked: false}
	g.CurrentQuad.Klingons = []*Klingon{cmd1, cmd2}
	g.CurrentQuad.Grid[4][5] = EntityCommander
	g.CurrentQuad.Grid[7][7] = EntityCommander

	// Fire torpedo along row 4 (angle = 0, due East)
	act := ActionFireTorpedo{Angle: 0.0}
	events, err := act.Execute(g)
	if err != nil {
		t.Fatalf("ActionFireTorpedo failed: %v", err)
	}

	// cmd1 was hit by torpedo -> decloaked
	if cmd1.IsCloaked {
		t.Errorf("expected cmd1 to decloak upon torpedo hit")
	}
	// cmd2 was elsewhere (evaded damage) -> cloaked
	if !cmd2.IsCloaked {
		t.Errorf("expected cmd2 to cloak after evading torpedo damage")
	}

	foundCmd1Decloak := false
	foundCmd2Cloak := false
	for _, ev := range events {
		if cEv, ok := ev.(EventKlingonCloakState); ok {
			if cEv.KlingonID == 1 && !cEv.Cloaked {
				foundCmd1Decloak = true
			}
			if cEv.KlingonID == 2 && cEv.Cloaked {
				foundCmd2Cloak = true
			}
		}
	}
	if !foundCmd1Decloak {
		t.Errorf("expected decloak event for cmd1")
	}
	if !foundCmd2Cloak {
		t.Errorf("expected cloak event for cmd2")
	}
}

func TestActionKlingonCounterAttack_Execute(t *testing.T) {
	rules := GameRules{KlingonCloak: true}
	g := NewGameWithOptions(100, SkillGood, LengthMedium, rules)
	g.Enterprise.Shields = 400
	g.Enterprise.Energy = 2500

	cmd := &Klingon{ID: 5, Sector: Coord{3, 3}, Energy: 500, IsCommander: true, IsCloaked: true}
	g.CurrentQuad.Klingons = []*Klingon{cmd}

	act := ActionKlingonCounterAttack{EnemyID: 5, Damage: 120}
	events, err := act.Execute(g)
	if err != nil {
		t.Fatalf("ActionKlingonCounterAttack failed: %v", err)
	}

	if cmd.IsCloaked {
		t.Errorf("expected attacker to decloak upon counter attack")
	}
	if len(events) != 2 {
		t.Fatalf("expected 2 events, got %d", len(events))
	}
	if cEv, ok := events[0].(EventKlingonCloakState); !ok || cEv.Cloaked || cEv.KlingonID != 5 {
		t.Errorf("expected EventKlingonCloakState with Cloaked=false, got %+v", events[0])
	}
	if aEv, ok := events[1].(EventKlingonCounterAttack); !ok || aEv.EnemyID != 5 || aEv.Damage != 120 {
		t.Errorf("expected EventKlingonCounterAttack, got %+v", events[1])
	}

	// Invalid ID returns error
	badAct := ActionKlingonCounterAttack{EnemyID: 999, Damage: 50}
	if _, err := badAct.Execute(g); err == nil {
		t.Errorf("expected error for non-existent enemy ID")
	}

	// Non-positive damage returns error
	badDmg := ActionKlingonCounterAttack{EnemyID: 5, Damage: 0}
	if _, err := badDmg.Execute(g); err == nil {
		t.Errorf("expected error for zero damage")
	}
}

func TestActionMove_BlackHoleFatalCollision(t *testing.T) {
	g := NewGame(10, SkillGood, LengthMedium)
	g.Enterprise.Sector = Coord{4, 4}
	g.CurrentQuad.Grid[4][4] = EntityEnterprise
	g.CurrentQuad.Grid[4][5] = EntityBlackHole
	g.CurrentQuad.Grid[3][5] = EntityBlackHole

	// Move directly east into black hole
	act := ActionMove{Course: 1.0, Warp: 0.125}
	events, err := act.Execute(g)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	foundGameOver := false
	for _, e := range events {
		if goe, ok := e.(EventGameOver); ok {
			foundGameOver = true
			if goe.Reason != GameOverLost {
				t.Errorf("expected GameOverLost, got %v", goe.Reason)
			}
		}
	}
	if !foundGameOver {
		t.Errorf("expected EventGameOver when colliding with Black Hole")
	}
}

func TestActionMove_BlackHoleGravityWellEnergyCost(t *testing.T) {
	g := NewGame(10, SkillGood, LengthMedium)
	g.Enterprise.Sector = Coord{4, 4}
	g.CurrentQuad.Grid[4][4] = EntityEnterprise
	g.CurrentQuad.Grid[5][5] = EntityBlackHole // Adjacent diagonal gravity well

	initialEnergy := g.Enterprise.Energy
	// Move away to Coord{3, 4}
	act := ActionMove{DestSector: Coord{3, 4}, Warp: 0.1}
	_, err := act.Execute(g)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	energyUsed := initialEnergy - g.Enterprise.Energy
	// In gravity well, cost is doubled
	expectedCost := 2.0 * (0.1 * 1.0 * 1.0 * 1.0 * 1.0)
	if math.Abs(energyUsed-expectedCost) > 0.001 {
		t.Errorf("expected energy cost %f, got %f", expectedCost, energyUsed)
	}
}

func TestActionMove_WormholeJump(t *testing.T) {
	g := NewGame(10, SkillGood, LengthMedium)
	g.Enterprise.Quad = Coord{2, 2}
	g.Enterprise.Sector = Coord{4, 4}
	g.CurrentQuad.Grid[4][4] = EntityEnterprise
	g.CurrentQuad.Grid[4][5] = EntityWormhole

	act := ActionMove{DestSector: Coord{4, 5}, Warp: 0.1}
	events, err := act.Execute(g)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	foundJump := false
	for _, e := range events {
		if j, ok := e.(EventWormholeJump); ok {
			foundJump = true
			if j.FromQuad != (Coord{2, 2}) || j.FromSector != (Coord{4, 4}) {
				t.Errorf("unexpected jump origin: %+v", j)
			}
		}
	}
	if !foundJump {
		t.Errorf("expected EventWormholeJump when entering wormhole")
	}
	if g.Enterprise.Quad == (Coord{2, 2}) && g.Enterprise.Sector == (Coord{4, 5}) {
		t.Errorf("Enterprise should have teleported away from wormhole cell")
	}
}

func TestActionMove_IonStormDrift(t *testing.T) {
	rules := DefaultRulesForProfile(ProfileHardcore)
	g := NewGameWithOptions(12345, SkillGood, LengthMedium, rules)
	g.Enterprise.Quad = Coord{1, 1}
	g.Enterprise.Sector = Coord{4, 1}
	g.CurrentQuad.Grid[4][1] = EntityEnterprise
	g.QuadrantEnv[1][1] = EnvIonStorm

	// Clear path in quadrant
	for c := 2; c <= 8; c++ {
		for r := 1; r <= 8; r++ {
			g.CurrentQuad.Grid[r][c] = EntityEmpty
		}
	}

	// Move East across 5 sectors (Warp 0.625 = 5 steps)
	act := ActionMove{Course: 0.0, Warp: 0.625}
	events, err := act.Execute(g)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	foundHazard := false
	for _, e := range events {
		if h, ok := e.(EventHazardTriggered); ok {
			if h.HazardType == "ion_storm_drift" {
				foundHazard = true
				break
			}
		}
	}
	if !foundHazard {
		t.Errorf("expected EventHazardTriggered for ion storm drift")
	}
}

func TestActionMove_BlackHoleFatalCollision_DestSector(t *testing.T) {
	g := NewGame(10, SkillGood, LengthMedium)
	g.Enterprise.Sector = Coord{4, 4}
	g.CurrentQuad.Grid[4][4] = EntityEnterprise
	g.CurrentQuad.Grid[4][5] = EntityBlackHole

	act := ActionMove{DestSector: Coord{4, 5}, Warp: 0.1}
	events, err := act.Execute(g)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	foundAbsorption := false
	foundGameOver := false
	for _, e := range events {
		if sa, ok := e.(EventSingularityAbsorption); ok {
			foundAbsorption = true
			if sa.Sector != (Coord{4, 5}) || sa.Target != EntityEnterprise || sa.Weapon != "ship" {
				t.Errorf("unexpected singularity absorption event: %+v", sa)
			}
		}
		if goe, ok := e.(EventGameOver); ok {
			foundGameOver = true
			if goe.Reason != GameOverLost {
				t.Errorf("expected GameOverLost, got %v", goe.Reason)
			}
		}
	}
	if !foundAbsorption {
		t.Errorf("expected EventSingularityAbsorption")
	}
	if !foundGameOver {
		t.Errorf("expected EventGameOver")
	}
}

func TestActionMove_BlackHoleGravityWellEnergyCost_MovingInto(t *testing.T) {
	g := NewGame(10, SkillGood, LengthMedium)
	// Enterprise at (1, 1), not adjacent to black hole at (3, 3)
	g.Enterprise.Sector = Coord{1, 1}
	g.CurrentQuad.Grid[1][1] = EntityEnterprise
	g.CurrentQuad.Grid[3][3] = EntityBlackHole

	initialEnergy := g.Enterprise.Energy
	// Move into (2, 2) which IS within Chebyshev distance 1 of (3, 3)
	act := ActionMove{DestSector: Coord{2, 2}, Warp: 0.1}
	_, err := act.Execute(g)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	energyUsed := initialEnergy - g.Enterprise.Energy
	expectedCost := 2.0 * (0.1 * 1.0 * 1.0 * 1.0 * 1.0)
	if math.Abs(energyUsed-expectedCost) > 0.001 {
		t.Errorf("expected energy cost %f, got %f", expectedCost, energyUsed)
	}
}

func TestActionMove_WormholeJump_Vector(t *testing.T) {
	g := NewGame(10, SkillGood, LengthMedium)
	g.Enterprise.Quad = Coord{2, 2}
	g.Enterprise.Sector = Coord{4, 4}
	g.CurrentQuad.Grid[4][4] = EntityEnterprise
	g.CurrentQuad.Grid[4][5] = EntityWormhole

	// Move east into wormhole with vector move (Course 0.0, Warp 0.125 = 1 step)
	act := ActionMove{Course: 0.0, Warp: 0.125}
	events, err := act.Execute(g)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	foundJump := false
	for _, e := range events {
		if j, ok := e.(EventWormholeJump); ok {
			foundJump = true
			if j.FromQuad != (Coord{2, 2}) || j.FromSector != (Coord{4, 4}) {
				t.Errorf("unexpected jump origin: %+v", j)
			}
		}
	}
	if !foundJump {
		t.Errorf("expected EventWormholeJump when entering wormhole via vector move")
	}
}


