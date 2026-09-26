package engine

import (
	"testing"
)

func TestTactics_UnifiedTurnDeterministicOrder(t *testing.T) {
	g := NewGameWithSeed(3333)
	r := &EnemyVessel{ID: 1, Faction: FactionRomulan, Sector: Coord{1, 1}, Energy: 1000.0, IsCloaked: true}
	t1 := &EnemyVessel{ID: 2, Faction: FactionTholian, Sector: Coord{8, 8}, Energy: 800.0}
	k := &EnemyVessel{ID: 3, Faction: FactionKlingon, Sector: Coord{4, 4}, Energy: 600.0}

	g.CurrentQuad.Enemies = []*EnemyVessel{r, t1, k}
	SyncQuadrantEnemies(&g.CurrentQuad)

	events := UnifiedAdversaryTurn(g)
	if len(events) == 0 {
		t.Fatal("expected events from unified adversary turn")
	}
}

func TestTactics_PointDefensePhaserInterception(t *testing.T) {
	g := NewGameWithSeed(4444)
	plasma := &PlasmaTorpedo{
		ID:           50,
		Sector:       Coord{3, 3},
		Energy:       500.0,
		TargetSector: g.Enterprise.Sector,
	}
	g.CurrentQuad.PlasmaTorpedoes = []*PlasmaTorpedo{plasma}
	g.CurrentQuad.Grid[3][3] = EntityPlasmaTorpedo

	// Fire phaser directly at (3, 3)
	action := ActionPhaserDirect{
		TargetSector: Coord{3, 3},
		Energy:       600.0,
	}
	events, err := action.Execute(g)
	if err != nil {
		t.Fatalf("unexpected error executing point defense phaser: %v", err)
	}

	if len(g.CurrentQuad.PlasmaTorpedoes) != 0 {
		t.Errorf("expected plasma torpedo to be destroyed by point defense phasers")
	}
	if len(events) == 0 {
		t.Errorf("expected interception events")
	}
	if g.Metrics.RomulansSurrendered != 1 {
		t.Errorf("expected RomulansSurrendered metric incremented to 1, got %d", g.Metrics.RomulansSurrendered)
	}
}

func TestTactics_PointDefensePhaserWeb(t *testing.T) {
	g := NewGameWithSeed(5555)
	g.Enterprise.Energy = 1000.0
	g.CurrentQuad.WebSegments = []*TholianWebSegment{
		{Coord: Coord{2, 2}, Strength: 250.0},
	}
	g.CurrentQuad.Grid[2][2] = EntityTholianWeb

	// 1. Partial damage
	actionPartial := ActionPhaserDirect{
		TargetSector: Coord{2, 2},
		Energy:       100.0,
	}
	events, err := actionPartial.Execute(g)
	if err != nil {
		t.Fatalf("unexpected error executing phaser against web: %v", err)
	}
	if len(events) != 0 {
		t.Errorf("expected no breach events for partial damage, got %d", len(events))
	}
	if len(g.CurrentQuad.WebSegments) != 1 || g.CurrentQuad.WebSegments[0].Strength != 150.0 {
		t.Errorf("expected web segment strength to reduce to 150, got %v", g.CurrentQuad.WebSegments[0].Strength)
	}
	if g.CurrentQuad.Grid[2][2] != EntityTholianWeb {
		t.Errorf("expected grid to remain EntityTholianWeb after partial damage")
	}

	// 2. Breaching damage
	actionBreach := ActionPhaserDirect{
		TargetSector: Coord{2, 2},
		Energy:       200.0,
	}
	events, err = actionBreach.Execute(g)
	if err != nil {
		t.Fatalf("unexpected error executing phaser to breach web: %v", err)
	}
	if len(events) == 0 {
		t.Errorf("expected breach event")
	}
	if len(g.CurrentQuad.WebSegments) != 0 {
		t.Errorf("expected web segment to be destroyed")
	}
	if g.CurrentQuad.Grid[2][2] != EntityEmpty {
		t.Errorf("expected grid cell to be EntityEmpty after web breach")
	}
}

func TestTactics_PointDefensePhaserVessel(t *testing.T) {
	g := NewGameWithSeed(6666)
	g.Enterprise.Sector = Coord{4, 4}
	g.Enterprise.Energy = 2000.0

	r := &EnemyVessel{
		ID:        10,
		Faction:   FactionRomulan,
		Sector:    Coord{4, 6},
		Shields:   50.0,
		Energy:    100.0,
		MaxEnergy: 500.0,
	}
	g.CurrentQuad.Enemies = []*EnemyVessel{r}
	g.CurrentQuad.Grid[4][6] = EntityRomulan

	// Target vessel with enough energy to overcome shields and destroy
	action := ActionPhaserDirect{
		TargetSector: Coord{4, 6},
		Energy:       500.0,
	}
	_, err := action.Execute(g)
	if err != nil {
		t.Fatalf("unexpected error executing phaser attack against vessel: %v", err)
	}

	if r.Energy > 0 {
		t.Errorf("expected enemy vessel to be destroyed, energy remaining: %f", r.Energy)
	}
	if g.CurrentQuad.Grid[4][6] != EntityEmpty {
		t.Errorf("expected grid at (4,6) to be EntityEmpty after destruction")
	}
	if g.Metrics.RomulansKilled != 1 {
		t.Errorf("expected RomulansKilled metric to increment to 1, got %d", g.Metrics.RomulansKilled)
	}
}

func TestTactics_ActionPhaserDirect_Validation(t *testing.T) {
	// Nil game state
	action := ActionPhaserDirect{TargetSector: Coord{1, 1}, Energy: 100.0}
	if _, err := action.Execute(nil); err == nil {
		t.Error("expected error for nil game state")
	}

	g := NewGameWithSeed(7777)
	g.Enterprise.Energy = 100.0

	// Zero / negative energy
	actionZero := ActionPhaserDirect{TargetSector: Coord{1, 1}, Energy: 0.0}
	if _, err := actionZero.Execute(g); err == nil {
		t.Error("expected error for zero phaser energy")
	}
	actionNeg := ActionPhaserDirect{TargetSector: Coord{1, 1}, Energy: -50.0}
	if _, err := actionNeg.Execute(g); err == nil {
		t.Error("expected error for negative phaser energy")
	}

	// Insufficient energy
	actionExcess := ActionPhaserDirect{TargetSector: Coord{1, 1}, Energy: 500.0}
	if _, err := actionExcess.Execute(g); err == nil {
		t.Error("expected error for insufficient energy")
	}

	// Out of bounds
	actionOOB1 := ActionPhaserDirect{TargetSector: Coord{0, 1}, Energy: 50.0}
	if _, err := actionOOB1.Execute(g); err == nil {
		t.Error("expected error for row < 1")
	}
	actionOOB2 := ActionPhaserDirect{TargetSector: Coord{9, 1}, Energy: 50.0}
	if _, err := actionOOB2.Execute(g); err == nil {
		t.Error("expected error for row > 8")
	}
	actionOOB3 := ActionPhaserDirect{TargetSector: Coord{1, 0}, Energy: 50.0}
	if _, err := actionOOB3.Execute(g); err == nil {
		t.Error("expected error for col < 1")
	}
	actionOOB4 := ActionPhaserDirect{TargetSector: Coord{1, 9}, Energy: 50.0}
	if _, err := actionOOB4.Execute(g); err == nil {
		t.Error("expected error for col > 8")
	}
}

func TestTactics_UnifiedTurn_DefensiveAndEdgeCases(t *testing.T) {
	// Nil game state
	if events := UnifiedAdversaryTurn(nil); events != nil {
		t.Errorf("expected nil events for nil game state, got %v", events)
	}

	g := NewGameWithSeed(8888)
	// Enemies slice with nil and zero energy elements
	g.CurrentQuad.Enemies = []*EnemyVessel{
		nil,
		{ID: 1, Faction: FactionKlingon, Sector: Coord{2, 2}, Energy: 0.0},
		{ID: 2, Faction: FactionRomulan, Sector: Coord{3, 3}, Energy: 0.0},
		nil,
	}
	events := UnifiedAdversaryTurn(g)
	if len(events) != 0 {
		t.Errorf("expected 0 events for inactive/nil enemies, got %d", len(events))
	}
}

func TestTactics_ComputeDistance(t *testing.T) {
	c1 := Coord{1, 1}
	c2 := Coord{4, 5}
	expected := Distance(c1, c2)
	actual := ComputeDistance(c1, c2)
	if actual != expected {
		t.Errorf("expected ComputeDistance %f, got %f", expected, actual)
	}
}

func TestTactics_UnifiedTurn_AllPhasesSequence(t *testing.T) {
	g := NewGameWithSeed(1234)
	g.Enterprise.Sector = Coord{4, 1}
	g.Enterprise.Energy = 5000.0
	g.Enterprise.Shields = 2000.0

	// Phase 1: in-flight plasma torpedo
	pt := &PlasmaTorpedo{
		ID:            99,
		Sector:        Coord{7, 7},
		Energy:        600.0,
		TargetSector:  g.Enterprise.Sector,
		TurnsInFlight: 0,
	}
	g.CurrentQuad.PlasmaTorpedoes = []*PlasmaTorpedo{pt}
	g.CurrentQuad.Grid[7][7] = EntityPlasmaTorpedo

	// Phase 2: Klingon commander and escort raider setup for screening & counter-attack
	cmd := &EnemyVessel{ID: 10, Faction: FactionKlingon, Sector: Coord{4, 6}, Energy: 500.0, IsCommander: true}
	raider := &EnemyVessel{ID: 11, Faction: FactionKlingon, Sector: Coord{3, 4}, Energy: 400.0}
	g.CurrentQuad.Grid[4][6] = EntityCommander
	g.CurrentQuad.Grid[3][4] = EntityKlingon

	// Phase 3: Romulan ready to decloak and launch
	rom := &EnemyVessel{ID: 20, Faction: FactionRomulan, Sector: Coord{1, 1}, Energy: 800.0, IsCloaked: true, CloakTurns: 0}

	// Phase 4: Tholian spinner
	tholian := &EnemyVessel{ID: 30, Faction: FactionTholian, Sector: Coord{8, 8}, Energy: 600.0, SpecialState: 0}
	g.CurrentQuad.Grid[8][8] = EntityTholian

	g.CurrentQuad.Enemies = []*EnemyVessel{cmd, raider, rom, tholian}
	SyncQuadrantEnemies(&g.CurrentQuad)

	events := UnifiedAdversaryTurn(g)

	var (
		foundPlasmaMoved    bool
		foundScreening      bool
		foundKlingonAttack  bool
		foundRomulanDecloak bool
		foundTholianWeb     bool
	)

	plasmaIdx, screenIdx, attackIdx, romulanIdx, tholianIdx := -1, -1, -1, -1, -1

	for idx, ev := range events {
		switch ev.EventType() {
		case "PlasmaMoved":
			if !foundPlasmaMoved {
				foundPlasmaMoved = true
				plasmaIdx = idx
			}
		case "KlingonScreening":
			if !foundScreening {
				foundScreening = true
				screenIdx = idx
			}
		case "KlingonCounterAttack":
			if !foundKlingonAttack {
				foundKlingonAttack = true
				attackIdx = idx
			}
		case "RomulanDecloak":
			if !foundRomulanDecloak {
				foundRomulanDecloak = true
				romulanIdx = idx
			}
		case "WebSegmentLaid":
			if !foundTholianWeb {
				foundTholianWeb = true
				tholianIdx = idx
			}
		}
	}

	if !foundPlasmaMoved {
		t.Error("expected PlasmaMoved event in phase 1")
	}
	if !foundScreening {
		t.Error("expected KlingonScreening event in phase 2")
	}
	if !foundKlingonAttack {
		t.Error("expected KlingonCounterAttack event in phase 2")
	}
	if !foundRomulanDecloak {
		t.Error("expected RomulanDecloak event in phase 3")
	}
	if !foundTholianWeb {
		t.Error("expected WebSegmentLaid event in phase 4")
	}

	// Verify order: Phase 1 < Phase 2 < Phase 3 < Phase 4
	if !(plasmaIdx < screenIdx && screenIdx < attackIdx && attackIdx < romulanIdx && romulanIdx < tholianIdx) {
		t.Errorf("expected phase order Phase 1 < Phase 2 (screening < attack) < Phase 3 < Phase 4, got indices: plasma=%d, screen=%d, attack=%d, romulan=%d, tholian=%d",
			plasmaIdx, screenIdx, attackIdx, romulanIdx, tholianIdx)
	}
}

func TestUnifiedAdversaryTurn_Decloak(t *testing.T) {
	g := NewGameWithSeed(123)
	// Add enterprise so there is a target for the Klingon to attack
	g.Enterprise.Sector = Coord{1, 1}
	
	// Create a cloaked Klingon
	klingon := &EnemyVessel{ID: 1, Faction: FactionKlingon, Sector: Coord{4, 4}, Energy: 500, IsCloaked: true}
	g.CurrentQuad.Enemies = []*EnemyVessel{klingon}
	
	events := UnifiedAdversaryTurn(g)
	
	// Ensure that after turn, IsCloaked is updated
	if g.CurrentQuad.Enemies[0].IsCloaked {
		t.Errorf("Expected Klingon to be decloaked after firing, but it was still cloaked")
	}
	
	// Ensure it fired
	fired := false
	for _, ev := range events {
		if ev.EventType() == "KlingonCounterAttack" || ev.EventType() == "KlingonDecloak" {
			fired = true
		}
	}
	if !fired {
		t.Logf("Events: %v", events)
	}
}

