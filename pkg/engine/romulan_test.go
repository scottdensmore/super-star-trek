package engine

import (
	"testing"
)

func TestRomulan_CloakingAndRepositioning(t *testing.T) {
	g := NewGameWithSeed(1234)
	r := &EnemyVessel{
		ID:         1,
		Faction:    FactionRomulan,
		Sector:     Coord{3, 3},
		Energy:     1200.0,
		Shields:    500.0,
		MaxEnergy:  1200.0,
		IsCloaked:  true,
		CloakTurns: 2,
	}
	g.CurrentQuad.Enemies = []*EnemyVessel{r}
	g.CurrentQuad.Grid[3][3] = EntityRomulan

	events := RomulanTurn(g, r)
	if len(events) == 0 {
		t.Fatal("expected events from Romulan turn")
	}

	// While cloaked, it should reposition silently without revealing its grid entity
	if r.CloakTurns != 1 {
		t.Errorf("expected CloakTurns to decrement to 1, got %d", r.CloakTurns)
	}

	// Trigger next turn to cause decloak and attack
	events = RomulanTurn(g, r)
	if r.IsCloaked {
		t.Errorf("expected Romulan to decloak when CloakTurns reaches 0")
	}
	if len(g.CurrentQuad.PlasmaTorpedoes) != 1 {
		t.Fatalf("expected 1 in-flight plasma torpedo launched, got %d", len(g.CurrentQuad.PlasmaTorpedoes))
	}
}

func TestRomulan_PlasmaTorpedoTrackingAndDissipation(t *testing.T) {
	g := NewGameWithSeed(5678)
	g.Enterprise.Sector = Coord{8, 8}

	plasma := &PlasmaTorpedo{
		ID:            10,
		SourceID:      1,
		Sector:        Coord{2, 2},
		Energy:        1000.0,
		TargetSector:  g.Enterprise.Sector,
		TurnsInFlight: 0,
	}
	g.CurrentQuad.PlasmaTorpedoes = []*PlasmaTorpedo{plasma}
	g.CurrentQuad.Grid[2][2] = EntityPlasmaTorpedo

	// Turn 1
	events := AdvancePlasmaTorpedoes(g)
	if len(events) == 0 {
		t.Fatal("expected advance events")
	}
	if plasma.Energy >= 1000.0 {
		t.Errorf("expected plasma energy to dissipate, got %f", plasma.Energy)
	}
	if plasma.Sector == (Coord{2, 2}) {
		t.Errorf("expected plasma torpedo to advance toward target, stayed at %v", plasma.Sector)
	}

	// Intercept test
	intercepted, intEvents := InterceptPlasmaTorpedo(g, plasma.Sector, 900.0)
	if !intercepted || len(intEvents) == 0 {
		t.Fatalf("expected plasma torpedo to be intercepted by 900 damage")
	}
	if len(g.CurrentQuad.PlasmaTorpedoes) != 0 {
		t.Errorf("expected plasma torpedoes slice to be empty after interception")
	}
}

func TestRomulan_DecloakedFiringAndRecloaking(t *testing.T) {
	g := NewGameWithSeed(4321)
	g.Enterprise.Sector = Coord{5, 5}
	g.Enterprise.Shields = 400.0
	g.Enterprise.Energy = 2500.0

	r := &EnemyVessel{
		ID:         2,
		Faction:    FactionRomulan,
		Sector:     Coord{4, 4},
		Energy:     1000.0,
		Shields:    400.0,
		MaxEnergy:  1000.0,
		IsCloaked:  false, // Decloaked
		CloakTurns: 0,
	}
	g.CurrentQuad.Enemies = []*EnemyVessel{r}
	g.CurrentQuad.Grid[4][4] = EntityRomulan

	events := RomulanTurn(g, r)
	if len(events) == 0 {
		t.Fatal("expected counter-attack events when firing decloaked")
	}

	// Romulan should have re-cloaked after attacking
	if !r.IsCloaked {
		t.Errorf("expected Romulan to re-cloak after firing disruptors")
	}
	if r.CloakTurns != 3 {
		t.Errorf("expected CloakTurns to reset to 3, got %d", r.CloakTurns)
	}
	if g.CurrentQuad.Grid[4][4] != EntityEmpty {
		t.Errorf("expected Romulan sector to be empty on grid after re-cloaking, got %v", g.CurrentQuad.Grid[4][4])
	}

	// Verify counter-attack damage occurred
	hasAttackEvent := false
	for _, ev := range events {
		if ev.EventType() == "KlingonCounterAttack" {
			hasAttackEvent = true
			break
		}
	}
	if !hasAttackEvent {
		t.Errorf("expected EventKlingonCounterAttack in events, got %v", events)
	}
}

func TestRomulan_PlasmaTorpedoImpact_50PercentShieldPenetration(t *testing.T) {
	g := NewGameWithSeed(9999)
	g.Enterprise.Sector = Coord{4, 4}
	g.Enterprise.Shields = 500.0
	g.Enterprise.Energy = 2000.0

	// Plasma starts 1 sector away so 1 advance step causes direct impact
	pt := &PlasmaTorpedo{
		ID:            20,
		SourceID:      1,
		Sector:        Coord{3, 3},
		Energy:        600.0,
		TargetSector:  g.Enterprise.Sector,
		TurnsInFlight: 0,
	}
	g.CurrentQuad.PlasmaTorpedoes = []*PlasmaTorpedo{pt}
	g.CurrentQuad.Grid[3][3] = EntityPlasmaTorpedo

	events := AdvancePlasmaTorpedoes(g)
	if len(events) == 0 {
		t.Fatal("expected impact events")
	}

	// 600 * 0.80 = 480 total yield on turn 1
	// 50% absorbed by shields = 240
	// 50% penetrating to hull = 240
	var impactEv *EventPlasmaImpact
	for _, ev := range events {
		if imp, ok := ev.(EventPlasmaImpact); ok {
			impactEv = &imp
			break
		}
	}
	if impactEv == nil {
		t.Fatalf("expected EventPlasmaImpact, got %v", events)
	}
	if impactEv.ShieldDamage != 240.0 {
		t.Errorf("expected ShieldDamage 240.0, got %f", impactEv.ShieldDamage)
	}
	if impactEv.HullDamage != 240.0 {
		t.Errorf("expected HullDamage 240.0, got %f", impactEv.HullDamage)
	}

	// Enterprise state: Shields = 500 - 240 = 260; Energy = 2000 - 240 = 1760
	if g.Enterprise.Shields != 260.0 {
		t.Errorf("expected Enterprise.Shields 260.0, got %f", g.Enterprise.Shields)
	}
	if g.Enterprise.Energy != 1760.0 {
		t.Errorf("expected Enterprise.Energy 1760.0, got %f", g.Enterprise.Energy)
	}

	// Grid and torpedo list cleanup
	if len(g.CurrentQuad.PlasmaTorpedoes) != 0 {
		t.Errorf("expected 0 plasma torpedoes remaining after impact, got %d", len(g.CurrentQuad.PlasmaTorpedoes))
	}
	if g.CurrentQuad.Grid[3][3] != EntityEmpty {
		t.Errorf("expected origin sector to be cleared on impact, got %v", g.CurrentQuad.Grid[3][3])
	}
}

func TestRomulan_PlasmaTorpedoDissipation_LowEnergy(t *testing.T) {
	g := NewGameWithSeed(1111)
	g.Enterprise.Sector = Coord{8, 8}

	pt := &PlasmaTorpedo{
		ID:            21,
		SourceID:      1,
		Sector:        Coord{2, 2},
		Energy:        220.0, // 220 * 0.8 = 176 (< 200 threshold)
		TargetSector:  g.Enterprise.Sector,
		TurnsInFlight: 1,
	}
	g.CurrentQuad.PlasmaTorpedoes = []*PlasmaTorpedo{pt}
	g.CurrentQuad.Grid[2][2] = EntityPlasmaTorpedo

	events := AdvancePlasmaTorpedoes(g)
	if len(events) != 1 || events[0].EventType() != "PlasmaDissipated" {
		t.Fatalf("expected EventPlasmaDissipated, got %v", events)
	}
	if len(g.CurrentQuad.PlasmaTorpedoes) != 0 {
		t.Errorf("expected torpedo to be removed upon dissipation")
	}
	if g.CurrentQuad.Grid[2][2] != EntityEmpty {
		t.Errorf("expected grid cell to be empty after dissipation")
	}
}

func TestRomulan_PlasmaTorpedoDissipation_MaxTurns(t *testing.T) {
	g := NewGameWithSeed(2222)
	g.Enterprise.Sector = Coord{8, 8}

	pt := &PlasmaTorpedo{
		ID:            22,
		SourceID:      1,
		Sector:        Coord{2, 2},
		Energy:        800.0,
		TargetSector:  g.Enterprise.Sector,
		TurnsInFlight: 4, // Next turn will be 5 (>= 5 threshold)
	}
	g.CurrentQuad.PlasmaTorpedoes = []*PlasmaTorpedo{pt}
	g.CurrentQuad.Grid[2][2] = EntityPlasmaTorpedo

	events := AdvancePlasmaTorpedoes(g)
	if len(events) != 1 || events[0].EventType() != "PlasmaDissipated" {
		t.Fatalf("expected EventPlasmaDissipated at turns in flight 5, got %v", events)
	}
	if len(g.CurrentQuad.PlasmaTorpedoes) != 0 {
		t.Errorf("expected torpedo to be removed upon max turns dissipation")
	}
}

func TestRomulan_PlasmaTorpedoObstacleAvoidance(t *testing.T) {
	g := NewGameWithSeed(3333)
	g.Enterprise.Sector = Coord{4, 4}

	// Torpedo at {2, 2}, direct diagonal path to {4, 4} passes through {3, 3}
	pt := &PlasmaTorpedo{
		ID:            23,
		SourceID:      1,
		Sector:        Coord{2, 2},
		Energy:        900.0,
		TargetSector:  g.Enterprise.Sector,
		TurnsInFlight: 0,
	}
	g.CurrentQuad.PlasmaTorpedoes = []*PlasmaTorpedo{pt}
	g.CurrentQuad.Grid[2][2] = EntityPlasmaTorpedo
	// Block direct diagonal step with a star
	g.CurrentQuad.Grid[3][3] = EntityStar

	events := AdvancePlasmaTorpedoes(g)
	if len(events) == 0 {
		t.Fatal("expected advance event")
	}

	// Should have routed around {3, 3} into {3, 2} or {2, 3}
	if pt.Sector == (Coord{3, 3}) {
		t.Errorf("plasma torpedo collided with star at {3, 3}")
	}
	if pt.Sector != (Coord{3, 2}) && pt.Sector != (Coord{2, 3}) {
		t.Errorf("expected torpedo to route to {3, 2} or {2, 3}, got %v", pt.Sector)
	}
	if g.CurrentQuad.Grid[3][3] != EntityStar {
		t.Errorf("expected star to remain undisturbed at {3, 3}")
	}
	if g.CurrentQuad.Grid[pt.Sector[0]][pt.Sector[1]] != EntityPlasmaTorpedo {
		t.Errorf("expected new sector to have EntityPlasmaTorpedo")
	}
}

func TestRomulan_InterceptPlasmaTorpedo_Thresholds(t *testing.T) {
	g := NewGameWithSeed(4444)
	pt := &PlasmaTorpedo{
		ID:            24,
		SourceID:      1,
		Sector:        Coord{3, 3},
		Energy:        1000.0, // 75% threshold is 750.0
		TargetSector:  Coord{5, 5},
		TurnsInFlight: 0,
	}
	g.CurrentQuad.PlasmaTorpedoes = []*PlasmaTorpedo{pt}
	g.CurrentQuad.Grid[3][3] = EntityPlasmaTorpedo

	// 1. Insufficient damage: 700 < 750
	intercepted, events := InterceptPlasmaTorpedo(g, Coord{3, 3}, 700.0)
	if intercepted || len(events) != 0 {
		t.Errorf("expected interception to fail with damage 700.0 < 750.0")
	}
	if len(g.CurrentQuad.PlasmaTorpedoes) != 1 {
		t.Errorf("expected torpedo to remain in slice")
	}

	// 2. Sufficient damage: 750 >= 750
	intercepted, events = InterceptPlasmaTorpedo(g, Coord{3, 3}, 750.0)
	if !intercepted || len(events) != 1 {
		t.Fatalf("expected interception to succeed with damage 750.0 >= 750.0")
	}
	if events[0].EventType() != "PlasmaIntercepted" {
		t.Errorf("expected EventPlasmaIntercepted, got %s", events[0].EventType())
	}
	if len(g.CurrentQuad.PlasmaTorpedoes) != 0 {
		t.Errorf("expected torpedo to be removed after successful interception")
	}
	if g.CurrentQuad.Grid[3][3] != EntityEmpty {
		t.Errorf("expected grid cell to be empty after interception")
	}
}

func TestRomulan_NilAndInvalidHandling(t *testing.T) {
	g := NewGameWithSeed(5555)

	// RomulanTurn nil checks
	if events := RomulanTurn(nil, nil); events != nil {
		t.Errorf("expected nil events for nil GameState")
	}
	r := &EnemyVessel{Faction: FactionKlingon}
	if events := RomulanTurn(g, r); events != nil {
		t.Errorf("expected nil events for non-Romulan faction")
	}

	// AdvancePlasmaTorpedoes nil checks
	if events := AdvancePlasmaTorpedoes(nil); events != nil {
		t.Errorf("expected nil events for nil GameState")
	}

	// InterceptPlasmaTorpedo nil / invalid damage checks
	if ok, events := InterceptPlasmaTorpedo(nil, Coord{1, 1}, 500.0); ok || events != nil {
		t.Errorf("expected false/nil for nil GameState")
	}
	if ok, events := InterceptPlasmaTorpedo(g, Coord{1, 1}, -50.0); ok || events != nil {
		t.Errorf("expected false/nil for negative damage")
	}
	if ok, events := InterceptPlasmaTorpedo(g, Coord{1, 1}, 0.0); ok || events != nil {
		t.Errorf("expected false/nil for zero damage")
	}
	if ok, events := InterceptPlasmaTorpedo(g, Coord{7, 7}, 500.0); ok || events != nil {
		t.Errorf("expected false/nil for coordinate with no torpedo")
	}
}
