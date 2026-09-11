package engine

import (
	"math"
	"testing"
)

func TestDispatchShieldTransfer(t *testing.T) {
	game := NewGame(12345, SkillGood, LengthMedium)
	initialEnergy := game.Enterprise.Energy
	initialShields := game.Enterprise.Shields

	events, err := game.Dispatch(ActionShields{Amount: 500})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if game.Enterprise.Shields != initialShields+500 || game.Enterprise.Energy != initialEnergy-500 {
		t.Fatalf("shields transfer failed: energy=%f shields=%f", game.Enterprise.Energy, game.Enterprise.Shields)
	}

	if len(events) == 0 || events[0].EventType() != "ShieldTransfer" {
		t.Fatalf("expected ShieldTransfer event, got %v", events)
	}

	transferEvt, ok := events[0].(EventShieldTransfer)
	if !ok {
		t.Fatalf("expected EventShieldTransfer type, got %T", events[0])
	}
	if transferEvt.NewShields != game.Enterprise.Shields || transferEvt.NewEnergy != game.Enterprise.Energy {
		t.Fatalf("event values mismatch: %+v", transferEvt)
	}
}

func TestDispatchShieldTransfer_Errors(t *testing.T) {
	game := NewGame(12345, SkillGood, LengthMedium)
	game.Enterprise.Energy = 100
	game.Enterprise.Shields = 50

	// Insufficient energy
	_, err := game.Dispatch(ActionShields{Amount: 200})
	if err == nil {
		t.Fatal("expected error for transferring more energy than available, got nil")
	}

	// Insufficient shields to transfer back
	_, err = game.Dispatch(ActionShields{Amount: -100})
	if err == nil {
		t.Fatal("expected error for transferring more shields than available, got nil")
	}

	// Transfer back within shield budget
	events, err := game.Dispatch(ActionShields{Amount: -50})
	if err != nil {
		t.Fatalf("unexpected error on negative transfer: %v", err)
	}
	if game.Enterprise.Shields != 0 || game.Enterprise.Energy != 150 {
		t.Fatalf("transfer back failed: energy=%f shields=%f", game.Enterprise.Energy, game.Enterprise.Shields)
	}
	if len(events) != 1 || events[0].EventType() != "ShieldTransfer" {
		t.Fatalf("unexpected events: %v", events)
	}
}

func TestDispatchDock(t *testing.T) {
	game := NewGame(12345, SkillGood, LengthMedium)
	game.Enterprise.Sector = Coord{4, 4}
	sbCoord := Coord{4, 5}
	game.CurrentQuad.Starbase = &sbCoord
	game.CurrentQuad.Grid[4][5] = EntityStarbase
	game.Enterprise.Energy = 2000
	game.Enterprise.Torpedoes = 3
	game.Enterprise.Condition = ConditionYellow
	game.Enterprise.Devices[DeviceWarp] = 5.0

	events, err := game.Dispatch(ActionDock{})
	if err != nil {
		t.Fatalf("unexpected error docking: %v", err)
	}

	if game.Enterprise.Condition != ConditionDocked {
		t.Fatalf("expected ConditionDocked, got %v", game.Enterprise.Condition)
	}
	if game.Enterprise.Energy != 5000 {
		t.Fatalf("expected energy replenished to 5000, got %f", game.Enterprise.Energy)
	}
	if game.Enterprise.Torpedoes != 10 {
		t.Fatalf("expected torpedoes replenished to 10, got %d", game.Enterprise.Torpedoes)
	}
	if game.Enterprise.Devices[DeviceWarp] != 0 {
		t.Fatalf("expected devices repaired upon docking, got warp damage=%f", game.Enterprise.Devices[DeviceWarp])
	}

	if len(events) == 0 || events[0].EventType() != "Docked" {
		t.Fatalf("expected Docked event, got %v", events)
	}
	dockEvt, ok := events[0].(EventDocked)
	if !ok || dockEvt.Starbase != sbCoord {
		t.Fatalf("unexpected dock event: %+v", dockEvt)
	}

	// Cannot dock again when already docked
	_, err = game.Dispatch(ActionDock{})
	if err == nil {
		t.Fatal("expected error docking when already docked")
	}
}

func TestDispatchDock_Errors(t *testing.T) {
	game := NewGame(12345, SkillGood, LengthMedium)
	game.Enterprise.Sector = Coord{1, 1}

	// No starbase in quadrant
	game.CurrentQuad.Starbase = nil
	_, err := game.Dispatch(ActionDock{})
	if err == nil {
		t.Fatal("expected error when no starbase in quadrant")
	}

	// Starbase far away
	sbCoord := Coord{8, 8}
	game.CurrentQuad.Starbase = &sbCoord
	game.CurrentQuad.Grid[8][8] = EntityStarbase
	_, err = game.Dispatch(ActionDock{})
	if err == nil {
		t.Fatal("expected error when starbase is not adjacent")
	}

	// Starbase at same location (invalid state)
	sbCoordSame := Coord{1, 1}
	game.CurrentQuad.Starbase = &sbCoordSame
	_, err = game.Dispatch(ActionDock{})
	if err == nil {
		t.Fatal("expected error when starbase is at identical coord")
	}
}

func TestDispatchFireTorpedo_HitAndDestroyKlingon(t *testing.T) {
	game := NewGame(12345, SkillGood, LengthMedium)
	game.Enterprise.Sector = Coord{4, 1}
	game.Enterprise.Torpedoes = 5
	game.RemainingKlingons = 10

	klingon := &Klingon{
		ID:     1,
		Sector: Coord{4, 7},
		Energy: 300,
	}
	game.CurrentQuad.Klingons = []*Klingon{klingon}
	game.CurrentQuad.Grid[4][7] = EntityKlingon

	// Fire due east (angle 0.0)
	events, err := game.Dispatch(ActionFireTorpedo{Angle: 0.0})
	if err != nil {
		t.Fatalf("unexpected error firing torpedo: %v", err)
	}

	if game.Enterprise.Torpedoes != 4 {
		t.Fatalf("expected torpedoes decremented to 4, got %d", game.Enterprise.Torpedoes)
	}
	if game.RemainingKlingons != 9 {
		t.Fatalf("expected remaining Klingons decremented to 9, got %d", game.RemainingKlingons)
	}
	if game.CurrentQuad.Grid[4][7] != EntityEmpty {
		t.Fatalf("expected grid cell cleared to EntityEmpty, got %v", game.CurrentQuad.Grid[4][7])
	}
	if len(game.CurrentQuad.Klingons) != 0 {
		t.Fatalf("expected Klingons slice empty, got %d klingons", len(game.CurrentQuad.Klingons))
	}

	if len(events) != 2 {
		t.Fatalf("expected 2 events (fired, hit), got %d: %v", len(events), events)
	}
	if events[0].EventType() != "TorpedoFired" {
		t.Fatalf("expected TorpedoFired, got %s", events[0].EventType())
	}
	if events[1].EventType() != "TorpedoHit" {
		t.Fatalf("expected TorpedoHit, got %s", events[1].EventType())
	}

	hitEvt, ok := events[1].(EventTorpedoHit)
	if !ok || !hitEvt.Destroyed || hitEvt.Target != (Coord{4, 7}) || hitEvt.Entity != EntityKlingon {
		t.Fatalf("unexpected hit event: %+v", hitEvt)
	}
}

func TestDispatchFireTorpedo_HitKlingonPartialDamage(t *testing.T) {
	game := NewGame(12345, SkillGood, LengthMedium)
	game.Enterprise.Sector = Coord{4, 1}
	game.Enterprise.Torpedoes = 5
	game.RemainingKlingons = 10

	klingon := &Klingon{
		ID:     1,
		Sector: Coord{4, 7},
		Energy: 1000, // higher than default 500 torpedo damage
	}
	game.CurrentQuad.Klingons = []*Klingon{klingon}
	game.CurrentQuad.Grid[4][7] = EntityKlingon

	events, err := game.Dispatch(ActionFireTorpedo{Angle: 0.0})
	if err != nil {
		t.Fatalf("unexpected error firing torpedo: %v", err)
	}

	if klingon.Energy != 500 {
		t.Fatalf("expected klingon energy 500 after partial torpedo hit, got %f", klingon.Energy)
	}
	if game.RemainingKlingons != 10 {
		t.Fatalf("expected remaining Klingons unchanged at 10, got %d", game.RemainingKlingons)
	}
	if len(events) != 2 || events[1].(EventTorpedoHit).Destroyed {
		t.Fatalf("expected non-destroyed hit event, got %v", events[1])
	}
}

func TestDispatchFireTorpedo_TargetCoordBearing(t *testing.T) {
	game := NewGame(12345, SkillGood, LengthMedium)
	game.Enterprise.Sector = Coord{4, 1}
	game.Enterprise.Torpedoes = 5

	klingon := &Klingon{
		ID:     1,
		Sector: Coord{4, 7},
		Energy: 300,
	}
	game.CurrentQuad.Klingons = []*Klingon{klingon}
	game.CurrentQuad.Grid[4][7] = EntityKlingon

	// Fire using Target coordinate instead of explicit angle
	events, err := game.Dispatch(ActionFireTorpedo{Target: Coord{4, 7}})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(events) != 2 || events[1].EventType() != "TorpedoHit" {
		t.Fatalf("expected hit event, got %v", events)
	}
}

func TestDispatchFireTorpedo_ObstaclesAndStarbase(t *testing.T) {
	game := NewGame(12345, SkillGood, LengthMedium)
	game.Enterprise.Sector = Coord{4, 1}
	game.Enterprise.Torpedoes = 5
	game.CurrentQuad.Grid[4][5] = EntityStar

	// Hit star: absorbed without destruction
	events, err := game.Dispatch(ActionFireTorpedo{Angle: 0.0})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(events) != 2 {
		t.Fatalf("expected 2 events, got %d", len(events))
	}
	hitEvt := events[1].(EventTorpedoHit)
	if hitEvt.Entity != EntityStar || hitEvt.Destroyed {
		t.Fatalf("expected star hit without destruction, got %+v", hitEvt)
	}

	// Hit planet
	game.CurrentQuad.Grid[4][5] = EntityPlanet
	events, err = game.Dispatch(ActionFireTorpedo{Angle: 0.0})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	hitEvt = events[1].(EventTorpedoHit)
	if hitEvt.Entity != EntityPlanet || hitEvt.Destroyed {
		t.Fatalf("expected planet hit without destruction, got %+v", hitEvt)
	}

	// Hit starbase: starbase destroyed
	sbCoord := Coord{4, 5}
	game.CurrentQuad.Starbase = &sbCoord
	game.CurrentQuad.Grid[4][5] = EntityStarbase
	game.RemainingStarbases = 3
	events, err = game.Dispatch(ActionFireTorpedo{Angle: 0.0})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	hitEvt = events[1].(EventTorpedoHit)
	if hitEvt.Entity != EntityStarbase || !hitEvt.Destroyed {
		t.Fatalf("expected starbase destroyed, got %+v", hitEvt)
	}
	if game.CurrentQuad.Starbase != nil {
		t.Fatal("expected CurrentQuad.Starbase to be nil after destruction")
	}
	if game.RemainingStarbases != 2 {
		t.Fatalf("expected remaining starbases 2, got %d", game.RemainingStarbases)
	}

	// Torpedo fired while docked does NOT consume torpedoes
	game.Enterprise.Condition = ConditionDocked
	game.Enterprise.Torpedoes = 10
	game.CurrentQuad.Grid[4][5] = EntityStar
	_, err = game.Dispatch(ActionFireTorpedo{Angle: 0.0})
	if err != nil {
		t.Fatalf("unexpected error firing while docked: %v", err)
	}
	if game.Enterprise.Torpedoes != 10 {
		t.Fatalf("expected torpedoes unchanged while docked, got %d", game.Enterprise.Torpedoes)
	}

	// Torpedo miss (exits quadrant)
	game.Enterprise.Condition = ConditionGreen
	events, err = game.Dispatch(ActionFireTorpedo{Angle: math.Pi / 2}) // Fire North from row 4, nothing above
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(events) != 1 || events[0].EventType() != "TorpedoFired" {
		t.Fatalf("expected only TorpedoFired event on miss, got %v", events)
	}

	// Empty torpedoes error
	game.Enterprise.Torpedoes = 0
	_, err = game.Dispatch(ActionFireTorpedo{Angle: 0.0})
	if err == nil {
		t.Fatal("expected error when out of torpedoes")
	}
}

func TestDispatchFirePhasers_Automatic(t *testing.T) {
	game := NewGame(12345, SkillGood, LengthMedium)
	game.Enterprise.Sector = Coord{4, 1}
	game.Enterprise.Energy = 4000
	game.RemainingKlingons = 10

	k1 := &Klingon{ID: 1, Sector: Coord{4, 3}, Energy: 100} // dist = 2.0
	k2 := &Klingon{ID: 2, Sector: Coord{4, 5}, Energy: 50}  // dist = 4.0
	game.CurrentQuad.Klingons = []*Klingon{k1, k2}
	game.CurrentQuad.Grid[4][3] = EntityKlingon
	game.CurrentQuad.Grid[4][5] = EntityKlingon

	// Fire 1000 energy total: 500 to k1 (dist 2.0 -> damage 250), 500 to k2 (dist 4.0 -> damage 125)
	events, err := game.Dispatch(ActionFirePhasers{Energy: 1000})
	if err != nil {
		t.Fatalf("unexpected error firing phasers: %v", err)
	}

	if game.Enterprise.Energy != 3000 {
		t.Fatalf("expected energy 3000, got %f", game.Enterprise.Energy)
	}
	// Both k1 (needs 100 dmg, takes 250) and k2 (needs 50 dmg, takes 125) should be destroyed
	if len(game.CurrentQuad.Klingons) != 0 {
		t.Fatalf("expected all Klingons destroyed, got %d", len(game.CurrentQuad.Klingons))
	}
	if game.RemainingKlingons != 8 {
		t.Fatalf("expected remaining Klingons 8, got %d", game.RemainingKlingons)
	}

	if len(events) != 3 { // PhaserFired, PhaserHit k1, PhaserHit k2
		t.Fatalf("expected 3 events, got %d: %v", len(events), events)
	}
	if events[0].EventType() != "PhaserFired" {
		t.Fatalf("expected PhaserFired, got %s", events[0].EventType())
	}
	hit1 := events[1].(EventPhaserHit)
	if !hit1.Destroyed || hit1.KlingonID != 1 {
		t.Fatalf("expected destroyed hit on k1, got %+v", hit1)
	}
	hit2 := events[2].(EventPhaserHit)
	if !hit2.Destroyed || hit2.KlingonID != 2 {
		t.Fatalf("expected destroyed hit on k2, got %+v", hit2)
	}

	// Automatic phaser fire with partial damage (not destroying)
	k3 := &Klingon{ID: 3, Sector: Coord{4, 3}, Energy: 500}
	game.CurrentQuad.Klingons = []*Klingon{k3}
	game.CurrentQuad.Grid[4][3] = EntityKlingon
	events, err = game.Dispatch(ActionFirePhasers{Energy: 200})
	if err != nil {
		t.Fatalf("unexpected error firing phasers: %v", err)
	}
	if len(events) != 2 || events[1].(EventPhaserHit).Destroyed {
		t.Fatalf("expected non-destroyed hit on k3, got %v", events[1])
	}
	if k3.Energy != 400 {
		t.Fatalf("expected k3 energy 400, got %f", k3.Energy)
	}
}

func TestDispatchFirePhasers_ManualAndErrors(t *testing.T) {
	game := NewGame(12345, SkillGood, LengthMedium)
	game.Enterprise.Sector = Coord{4, 1}
	game.Enterprise.Energy = 4000

	k1 := &Klingon{ID: 1, Sector: Coord{4, 3}, Energy: 500}
	game.CurrentQuad.Klingons = []*Klingon{k1}
	game.CurrentQuad.Grid[4][3] = EntityKlingon

	// Manual allocation: 200 energy to k1 (dist 2.0 -> damage 100)
	events, err := game.Dispatch(ActionFirePhasers{
		Energy: 200,
		ManualAllocation: map[int]float64{
			1: 200,
		},
	})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if k1.Energy != 400 {
		t.Fatalf("expected k1 energy 400, got %f", k1.Energy)
	}
	if len(events) != 2 || events[1].EventType() != "PhaserHit" {
		t.Fatalf("unexpected events: %v", events)
	}
	hit := events[1].(EventPhaserHit)
	if hit.Destroyed || hit.Damage != 100 {
		t.Fatalf("expected non-destroyed hit with 100 damage, got %+v", hit)
	}

	// Manual allocation with energy omitted (auto-inferred from allocation map)
	events, err = game.Dispatch(ActionFirePhasers{
		ManualAllocation: map[int]float64{
			1:  200,
			99: 100, // ID 99 does not exist, should be ignored
			2:  -50, // negative allocation, ignored
		},
	})
	if err != nil {
		t.Fatalf("unexpected error on inferred manual allocation: %v", err)
	}
	if k1.Energy != 300 {
		t.Fatalf("expected k1 energy 300, got %f", k1.Energy)
	}

	// Manual allocation with only non-positive values
	_, err = game.Dispatch(ActionFirePhasers{
		ManualAllocation: map[int]float64{
			1: -10,
		},
	})
	if err == nil {
		t.Fatal("expected error for non-positive manual allocation")
	}

	// Non-positive energy error
	_, err = game.Dispatch(ActionFirePhasers{Energy: 0})
	if err == nil {
		t.Fatal("expected error for non-positive energy")
	}

	// Insufficient energy
	game.Enterprise.Energy = 50
	_, err = game.Dispatch(ActionFirePhasers{Energy: 100})
	if err == nil {
		t.Fatal("expected error for insufficient energy")
	}

	// Cannot fire while docked
	game.Enterprise.Energy = 5000
	game.Enterprise.Condition = ConditionDocked
	_, err = game.Dispatch(ActionFirePhasers{Energy: 100})
	if err == nil {
		t.Fatal("expected error firing phasers while docked")
	}

	// No enemies in quadrant
	game.Enterprise.Condition = ConditionGreen
	game.CurrentQuad.Klingons = nil
	_, err = game.Dispatch(ActionFirePhasers{Energy: 100})
	if err == nil {
		t.Fatal("expected error firing phasers with no enemies")
	}
}

func TestDispatchFirePhasers_ManualAllocationExploit(t *testing.T) {
	game := NewGame(12345, SkillGood, LengthMedium)
	game.Enterprise.Sector = Coord{4, 1}
	game.Enterprise.Energy = 500

	k1 := &Klingon{ID: 1, Sector: Coord{4, 3}, Energy: 500}
	k2 := &Klingon{ID: 2, Sector: Coord{4, 5}, Energy: 500}
	game.CurrentQuad.Klingons = []*Klingon{k1, k2}
	game.CurrentQuad.Grid[4][3] = EntityKlingon
	game.CurrentQuad.Grid[4][5] = EntityKlingon

	// Exploit attempt: low a.Energy (100) but high ManualAllocation (300 + 300 = 600)
	// Enterprise has 500 energy, so 600 should be rejected as insufficient energy
	_, err := game.Dispatch(ActionFirePhasers{
		Energy: 100,
		ManualAllocation: map[int]float64{
			1: 300,
			2: 300,
		},
	})
	if err == nil {
		t.Fatal("expected error when total manual allocation exceeds Enterprise energy")
	}

	// Now increase energy to 1000 so 600 is affordable.
	// Firing with a.Energy=100 and allocation of 300+300=600 must deduct 600, not 100!
	game.Enterprise.Energy = 1000
	events, err := game.Dispatch(ActionFirePhasers{
		Energy: 100,
		ManualAllocation: map[int]float64{
			1: 300,
			2: 300,
		},
	})
	if err != nil {
		t.Fatalf("unexpected error firing phasers: %v", err)
	}
	if game.Enterprise.Energy != 400 {
		t.Fatalf("expected energy deducted by actual manual allocation 600 (remaining 400), got %f", game.Enterprise.Energy)
	}
	if len(events) < 1 || events[0].EventType() != "PhaserFired" {
		t.Fatalf("expected PhaserFired event, got %v", events)
	}
	firedEvt := events[0].(EventPhaserFired)
	if firedEvt.Energy != 600 {
		t.Fatalf("expected PhaserFired energy 600, got %f", firedEvt.Energy)
	}
}

func TestDispatchMove(t *testing.T) {
	game := NewGame(12345, SkillGood, LengthMedium)
	game.Enterprise.Quad = Coord{1, 1}
	game.Enterprise.Sector = Coord{4, 1}
	game.CurrentQuad.Grid[4][1] = EntityEnterprise
	game.Enterprise.Energy = 5000
	initialStardate := game.Stardate

	// Move east 2 sectors at Warp 0.25 (2 sectors)
	events, err := game.Dispatch(ActionMove{
		Course: 0.0,  // East
		Warp:   0.25, // 0.25 quadrant = 2 sectors
	})
	if err != nil {
		t.Fatalf("unexpected error moving: %v", err)
	}

	if game.Enterprise.Sector != (Coord{4, 3}) {
		t.Fatalf("expected sector [4, 3], got %v", game.Enterprise.Sector)
	}
	if game.CurrentQuad.Grid[4][1] != EntityEmpty {
		t.Fatalf("expected old sector cleared to EntityEmpty, got %v", game.CurrentQuad.Grid[4][1])
	}
	if game.CurrentQuad.Grid[4][3] != EntityEnterprise {
		t.Fatalf("expected new sector occupied by EntityEnterprise, got %v", game.CurrentQuad.Grid[4][3])
	}
	if game.Enterprise.Energy >= 5000 {
		t.Fatalf("expected energy consumed by movement, got %f", game.Enterprise.Energy)
	}
	if game.Stardate <= initialStardate {
		t.Fatalf("expected stardate advanced, got %f", game.Stardate)
	}

	if len(events) == 0 || events[0].EventType() != "ShipMoved" {
		t.Fatalf("expected ShipMoved event, got %v", events)
	}
	moveEvt := events[0].(EventShipMoved)
	if moveEvt.FromSector != (Coord{4, 1}) || moveEvt.ToSector != (Coord{4, 3}) {
		t.Fatalf("unexpected move event: %+v", moveEvt)
	}
}

func TestDispatchMove_DestSector(t *testing.T) {
	game := NewGame(12345, SkillGood, LengthMedium)
	game.Enterprise.Sector = Coord{2, 2}
	game.CurrentQuad.Grid[2][2] = EntityEnterprise
	game.Enterprise.Condition = ConditionDocked

	// Direct DestSector move
	events, err := game.Dispatch(ActionMove{
		Warp:       1.0,
		DestSector: Coord{5, 5},
	})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if game.Enterprise.Sector != (Coord{5, 5}) {
		t.Fatalf("expected sector [5, 5], got %v", game.Enterprise.Sector)
	}
	if game.Enterprise.Condition != ConditionGreen {
		t.Fatalf("expected undocked to ConditionGreen, got %v", game.Enterprise.Condition)
	}
	if len(events) == 0 || events[0].EventType() != "ShipMoved" {
		t.Fatalf("expected ShipMoved event, got %v", events)
	}

	// Out of bounds DestSector
	_, err = game.Dispatch(ActionMove{
		Warp:       1.0,
		DestSector: Coord{9, 9},
	})
	if err == nil {
		t.Fatal("expected error for out of bounds DestSector")
	}

	// Occupied DestSector
	game.CurrentQuad.Grid[6][6] = EntityStar
	_, err = game.Dispatch(ActionMove{
		Warp:       1.0,
		DestSector: Coord{6, 6},
	})
	if err == nil {
		t.Fatal("expected error for occupied DestSector")
	}
}

func TestDispatchMove_ObstacleCollision(t *testing.T) {
	game := NewGame(12345, SkillGood, LengthMedium)
	game.Enterprise.Sector = Coord{4, 1}
	game.CurrentQuad.Grid[4][1] = EntityEnterprise
	game.CurrentQuad.Grid[4][3] = EntityStar // obstacle at sector 3

	// Move east 4 sectors; should halt before sector 3 (at sector 2)
	events, err := game.Dispatch(ActionMove{
		Course: 0.0,
		Warp:   0.5,
	})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if game.Enterprise.Sector != (Coord{4, 2}) {
		t.Fatalf("expected ship stopped at [4, 2], got %v", game.Enterprise.Sector)
	}
	if game.CurrentQuad.Grid[4][2] != EntityEnterprise {
		t.Fatalf("expected [4, 2] to have EntityEnterprise, got %v", game.CurrentQuad.Grid[4][2])
	}

	// Should have emitted ObstacleEncountered event
	hasObstacle := false
	for _, e := range events {
		if e.EventType() == "ObstacleEncountered" {
			hasObstacle = true
			break
		}
	}
	if !hasObstacle {
		t.Fatalf("expected ObstacleEncountered event in %v", events)
	}
}

func TestDispatchMove_InterQuadrant(t *testing.T) {
	game := NewGame(12345, SkillGood, LengthMedium)
	game.Enterprise.Quad = Coord{2, 2}
	game.Enterprise.Sector = Coord{4, 4}
	game.CurrentQuad.Grid[4][4] = EntityEnterprise
	game.Enterprise.Energy = 5000

	// Warp factor 2.0 East (Course 0.0) -> 16 steps
	// from Quad {2, 2}, Sector {4, 4}:
	// totalR = 8*1 + 4 = 12
	// totalC = 8*1 + 4 = 12
	// destR = 12
	// destC = 12 + 16 = 28
	// toQuad: Coord{(12-1)/8 + 1, (28-1)/8 + 1} = Coord{2, 4}
	// toSector: Coord{(12-1)%8 + 1, (28-1)%8 + 1} = Coord{4, 4}
	events, err := game.Dispatch(ActionMove{
		Course: 0.0,
		Warp:   2.0,
	})
	if err != nil {
		t.Fatalf("unexpected error on inter-quadrant move: %v", err)
	}

	if game.Enterprise.Quad != (Coord{2, 4}) {
		t.Fatalf("expected destination quad {2, 4}, got %v", game.Enterprise.Quad)
	}
	if game.Enterprise.Sector != (Coord{4, 4}) {
		t.Fatalf("expected destination sector {4, 4}, got %v", game.Enterprise.Sector)
	}
	// Verify fromSector cleared in CurrentQuad.Grid
	if game.CurrentQuad.Grid[4][4] != EntityEmpty {
		t.Fatalf("expected fromSector cleared in old quad grid, got %v", game.CurrentQuad.Grid[4][4])
	}
	if len(events) == 0 || events[0].EventType() != "ShipMoved" {
		t.Fatalf("expected ShipMoved event, got %v", events)
	}
	moveEvt := events[0].(EventShipMoved)
	if moveEvt.FromQuad != (Coord{2, 2}) || moveEvt.ToQuad != (Coord{2, 4}) {
		t.Fatalf("unexpected quads in move event: %+v", moveEvt)
	}

	// Galaxy edge clamping test:
	// From Quad {1, 1}, Sector {2, 2}, moving North (Course math.Pi/2) with Warp 4.0 (32 steps)
	game.Enterprise.Quad = Coord{1, 1}
	game.Enterprise.Sector = Coord{2, 2}
	game.CurrentQuad.Grid[2][2] = EntityEnterprise
	game.Enterprise.Energy = 5000

	_, err = game.Dispatch(ActionMove{
		Course: math.Pi / 2, // North (dr = -1, dc = 0)
		Warp:   4.0,
	})
	if err != nil {
		t.Fatalf("unexpected error on galaxy edge clamping move: %v", err)
	}
	// TotalR = 2. Clamped at row 1. Quad {1, 1}, Sector {1, 2}.
	if game.Enterprise.Quad != (Coord{1, 1}) {
		t.Fatalf("expected clamped quad {1, 1}, got %v", game.Enterprise.Quad)
	}
	if game.Enterprise.Sector != (Coord{1, 2}) {
		t.Fatalf("expected clamped sector {1, 2}, got %v", game.Enterprise.Sector)
	}

	// Obstacle in current quadrant prevents inter-quadrant exit:
	game.Enterprise.Quad = Coord{1, 1}
	game.Enterprise.Sector = Coord{4, 1}
	game.CurrentQuad.Grid[4][1] = EntityEnterprise
	game.CurrentQuad.Grid[4][3] = EntityStar // obstacle at sector 3
	game.Enterprise.Energy = 5000

	events, err = game.Dispatch(ActionMove{
		Course: 0.0, // East, attempting Warp 2.0 (16 steps)
		Warp:   2.0,
	})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	// Movement must stop before obstacle at sector 2, staying in current quadrant
	if game.Enterprise.Quad != (Coord{1, 1}) {
		t.Fatalf("expected to remain in quad {1, 1} due to obstacle, got %v", game.Enterprise.Quad)
	}
	if game.Enterprise.Sector != (Coord{4, 2}) {
		t.Fatalf("expected stopped at sector {4, 2}, got %v", game.Enterprise.Sector)
	}
	if game.CurrentQuad.Grid[4][2] != EntityEnterprise {
		t.Fatalf("expected EntityEnterprise at {4, 2}, got %v", game.CurrentQuad.Grid[4][2])
	}
}

func TestDispatchMove_ExplicitDestQuad(t *testing.T) {
	game := NewGame(12345, SkillGood, LengthMedium)
	game.Enterprise.Quad = Coord{1, 1}
	game.Enterprise.Sector = Coord{3, 3}
	game.CurrentQuad.Grid[3][3] = EntityEnterprise
	game.Enterprise.Energy = 5000

	// Move with explicit DestQuad and DestSector
	events, err := game.Dispatch(ActionMove{
		Warp:       1.0,
		DestQuad:   Coord{5, 6},
		DestSector: Coord{2, 7},
	})
	if err != nil {
		t.Fatalf("unexpected error with explicit DestQuad/DestSector: %v", err)
	}
	if game.Enterprise.Quad != (Coord{5, 6}) {
		t.Fatalf("expected Enterprise Quad {5, 6}, got %v", game.Enterprise.Quad)
	}
	if game.Enterprise.Sector != (Coord{2, 7}) {
		t.Fatalf("expected Enterprise Sector {2, 7}, got %v", game.Enterprise.Sector)
	}
	if game.CurrentQuad.Grid[3][3] != EntityEmpty {
		t.Fatalf("expected old sector cleared in grid, got %v", game.CurrentQuad.Grid[3][3])
	}
	if len(events) == 0 || events[0].EventType() != "ShipMoved" {
		t.Fatalf("expected ShipMoved event, got %v", events)
	}

	// Move with explicit DestQuad and default DestSector (Coord{4, 4})
	game.Enterprise.Quad = Coord{1, 1}
	game.Enterprise.Sector = Coord{2, 2}
	game.CurrentQuad.Grid[2][2] = EntityEnterprise
	game.Enterprise.Energy = 5000

	_, err = game.Dispatch(ActionMove{
		Warp:     1.0,
		DestQuad: Coord{7, 8},
	})
	if err != nil {
		t.Fatalf("unexpected error with explicit DestQuad without DestSector: %v", err)
	}
	if game.Enterprise.Quad != (Coord{7, 8}) {
		t.Fatalf("expected Enterprise Quad {7, 8}, got %v", game.Enterprise.Quad)
	}
	if game.Enterprise.Sector != (Coord{4, 4}) {
		t.Fatalf("expected default Enterprise Sector {4, 4}, got %v", game.Enterprise.Sector)
	}

	// Out of bounds DestQuad
	outOfBoundsQuads := []Coord{{0, 1}, {9, 5}, {4, 0}, {4, 9}}
	for _, dq := range outOfBoundsQuads {
		_, err = game.Dispatch(ActionMove{
			Warp:     1.0,
			DestQuad: dq,
		})
		if err == nil {
			t.Fatalf("expected error for out of bounds DestQuad %v, got nil", dq)
		}
	}

	// Out of bounds DestSector with valid DestQuad
	outOfBoundsSectors := []Coord{{0, 1}, {9, 5}, {4, 0}, {4, 9}}
	for _, ds := range outOfBoundsSectors {
		_, err = game.Dispatch(ActionMove{
			Warp:       1.0,
			DestQuad:   Coord{3, 3},
			DestSector: ds,
		})
		if err == nil {
			t.Fatalf("expected error for out of bounds DestSector %v, got nil", ds)
		}
	}
}

func TestDispatchMove_ErrorsAndWarpLimits(t *testing.T) {
	game := NewGame(12345, SkillGood, LengthMedium)
	game.Enterprise.Devices[DeviceWarp] = 2.0 // damaged, limit warp 4

	// Invalid negative or 0 warp
	_, err := game.Dispatch(ActionMove{Warp: 0})
	if err == nil {
		t.Fatal("expected error for non-positive warp")
	}

	// Warp > 4 when damaged
	_, err = game.Dispatch(ActionMove{Course: 0, Warp: 5.0})
	if err == nil {
		t.Fatal("expected error exceeding warp limit with damaged engine")
	}

	// Inoperative engine (damage >= 10.0)
	game.Enterprise.Devices[DeviceWarp] = 10.0
	_, err = game.Dispatch(ActionMove{Course: 0, Warp: 2.0})
	if err == nil {
		t.Fatal("expected error for inoperative warp engines")
	}

	// Insufficient energy
	game.Enterprise.Devices[DeviceWarp] = 0
	game.Enterprise.Energy = 1.0
	_, err = game.Dispatch(ActionMove{Course: 0, Warp: 8.0})
	if err == nil {
		t.Fatal("expected error for insufficient energy on high warp move")
	}
}

func TestEventTypes(t *testing.T) {
	events := []Event{
		EventShieldTransfer{},
		EventTorpedoFired{},
		EventTorpedoHit{},
		EventPhaserFired{},
		EventPhaserHit{},
		EventDocked{},
		EventShipMoved{},
		EventObstacleEncountered{},
		EventConditionChanged{},
		EventGameOver{Reason: GameOverWon},
		EventKlingonCounterAttack{EnemyID: 1, Damage: 50},
		EventSubsystemDamaged{Device: DeviceWarp, RepairTime: 3.5},
		EventSubsystemRepaired{Device: DeviceWarp},
	}

	expectedTypes := []string{
		"ShieldTransfer",
		"TorpedoFired",
		"TorpedoHit",
		"PhaserFired",
		"PhaserHit",
		"Docked",
		"ShipMoved",
		"ObstacleEncountered",
		"ConditionChanged",
		"GameOver",
		"KlingonCounterAttack",
		"SubsystemDamaged",
		"SubsystemRepaired",
	}

	for i, e := range events {
		if e.EventType() != expectedTypes[i] {
			t.Errorf("event %d: expected %q, got %q", i, expectedTypes[i], e.EventType())
		}
	}
}
