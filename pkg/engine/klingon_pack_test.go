package engine

import (
	"testing"
)

func TestKlingonPack_CrossfireBracketDetection(t *testing.T) {
	g := NewGameWithSeed(777)
	g.Enterprise.Sector = Coord{4, 4}

	// Two Klingons flanking Enterprise from opposing sides (180 degrees)
	k1 := &EnemyVessel{ID: 1, Faction: FactionKlingon, Sector: Coord{4, 1}, Energy: 200}
	k2 := &EnemyVessel{ID: 2, Faction: FactionKlingon, Sector: Coord{4, 7}, Energy: 200}
	g.CurrentQuad.Enemies = []*EnemyVessel{k1, k2}

	isBracketed := DetectCrossfireBracket(g)
	if !isBracketed {
		t.Fatalf("expected crossfire bracket detected for opposing Klingons")
	}

	mult := CalculatePackDamageMultiplier(g)
	if mult != 1.35 {
		t.Errorf("expected 1.35 crossfire damage multiplier, got %f", mult)
	}
}

func TestKlingonPack_CommanderScreeningInterposition(t *testing.T) {
	g := NewGameWithSeed(888)
	g.Enterprise.Sector = Coord{4, 1}

	// Commander at (4, 6) in direct row line-of-fire
	commander := &EnemyVessel{
		ID:          10,
		Faction:     FactionKlingon,
		Sector:      Coord{4, 6},
		IsCommander: true,
		Energy:      300,
	}
	// Raider escort at (3, 4) able to step down into (4, 4) to block
	raider := &EnemyVessel{
		ID:          11,
		Faction:     FactionKlingon,
		Sector:      Coord{3, 4},
		IsCommander: false,
		Energy:      200,
	}
	g.CurrentQuad.Enemies = []*EnemyVessel{commander, raider}
	g.CurrentQuad.Grid[4][6] = EntityCommander
	g.CurrentQuad.Grid[3][4] = EntityKlingon

	events := ExecuteCommanderScreening(g)
	if len(events) == 0 {
		t.Fatalf("expected screening maneuver events")
	}

	if raider.Sector != (Coord{4, 4}) {
		t.Errorf("expected raider to interpose at (4, 4), got %v", raider.Sector)
	}
	if g.CurrentQuad.Grid[4][4] != EntityKlingon {
		t.Errorf("expected grid at (4, 4) to be occupied by Klingon raider")
	}
	if g.CurrentQuad.Grid[3][4] != EntityEmpty {
		t.Errorf("expected departed cell (3, 4) to be empty, got %v", g.CurrentQuad.Grid[3][4])
	}

	screeningEvt, ok := events[0].(EventKlingonScreening)
	if !ok {
		t.Fatalf("expected EventKlingonScreening event type, got %T", events[0])
	}
	if screeningEvt.EventType() != "KlingonScreening" {
		t.Errorf("expected event type 'KlingonScreening', got %s", screeningEvt.EventType())
	}
	if screeningEvt.RaiderID != 11 || screeningEvt.CommanderID != 10 || screeningEvt.Interposed != (Coord{4, 4}) {
		t.Errorf("unexpected event payload: %+v", screeningEvt)
	}
}

func TestKlingonPack_AngleThresholds(t *testing.T) {
	g := NewGameWithSeed(101)
	g.Enterprise.Sector = Coord{4, 4}

	// Case 1: Angle ~14 degrees (< 60 degrees threshold)
	k1 := &EnemyVessel{ID: 1, Faction: FactionKlingon, Sector: Coord{4, 8}, Energy: 200}
	k2 := &EnemyVessel{ID: 2, Faction: FactionKlingon, Sector: Coord{5, 8}, Energy: 200}
	g.CurrentQuad.Enemies = []*EnemyVessel{k1, k2}

	if DetectCrossfireBracket(g) {
		t.Errorf("expected no crossfire for narrow angle Klingons (~14 deg)")
	}
	if mult := CalculatePackDamageMultiplier(g); mult != 1.0 {
		t.Errorf("expected 1.0 damage multiplier when not bracketed, got %f", mult)
	}

	// Case 2: Angle 90 degrees (>= 60 degrees threshold)
	k3 := &EnemyVessel{ID: 3, Faction: FactionKlingon, Sector: Coord{8, 4}, Energy: 200}
	g.CurrentQuad.Enemies = []*EnemyVessel{k1, k3}

	if !DetectCrossfireBracket(g) {
		t.Errorf("expected crossfire detected for 90 degree bracket")
	}
	if mult := CalculatePackDamageMultiplier(g); mult != 1.35 {
		t.Errorf("expected 1.35 multiplier for 90 degree bracket, got %f", mult)
	}

	// Case 3: Exactly same sector as Enterprise (mag == 0)
	kAtEnt := &EnemyVessel{ID: 4, Faction: FactionKlingon, Sector: Coord{4, 4}, Energy: 200}
	g.CurrentQuad.Enemies = []*EnemyVessel{k1, kAtEnt}
	if DetectCrossfireBracket(g) {
		t.Errorf("expected no crossfire when vessel has 0 distance to Enterprise")
	}
}

func TestKlingonPack_InactiveKlingonsFiltered(t *testing.T) {
	g := NewGameWithSeed(102)
	g.Enterprise.Sector = Coord{4, 4}

	// Opposing sectors (180 deg), but one is destroyed (Energy <= 0)
	k1 := &EnemyVessel{ID: 1, Faction: FactionKlingon, Sector: Coord{4, 1}, Energy: 200}
	kDead := &EnemyVessel{ID: 2, Faction: FactionKlingon, Sector: Coord{4, 7}, Energy: 0}
	g.CurrentQuad.Enemies = []*EnemyVessel{k1, kDead}

	if DetectCrossfireBracket(g) {
		t.Errorf("expected no crossfire when one Klingon has Energy <= 0")
	}

	kDead.Energy = -50.0
	if DetectCrossfireBracket(g) {
		t.Errorf("expected no crossfire when one Klingon has negative energy")
	}
}

func TestKlingonPack_FactionFiltering(t *testing.T) {
	g := NewGameWithSeed(103)
	g.Enterprise.Sector = Coord{4, 4}

	// Opposing sectors, but one vessel is Romulan
	k := &EnemyVessel{ID: 1, Faction: FactionKlingon, Sector: Coord{4, 1}, Energy: 200}
	r := &EnemyVessel{ID: 2, Faction: FactionRomulan, Sector: Coord{4, 7}, Energy: 400}
	g.CurrentQuad.Enemies = []*EnemyVessel{k, r}

	if DetectCrossfireBracket(g) {
		t.Errorf("expected no Klingon crossfire bracket when other vessel is Romulan")
	}
}

func TestKlingonPack_CommanderScreening_Vertical(t *testing.T) {
	g := NewGameWithSeed(104)
	g.Enterprise.Sector = Coord{1, 4}

	// Commander at (6, 4) in direct column line of fire
	commander := &EnemyVessel{
		ID:          20,
		Faction:     FactionKlingon,
		Sector:      Coord{6, 4},
		IsCommander: true,
		Energy:      300,
	}
	// Raider escort at (4, 3) steps right into (4, 4) to block
	raider := &EnemyVessel{
		ID:          21,
		Faction:     FactionKlingon,
		Sector:      Coord{4, 3},
		IsCommander: false,
		Energy:      200,
	}
	g.CurrentQuad.Enemies = []*EnemyVessel{commander, raider}
	g.CurrentQuad.Grid[6][4] = EntityCommander
	g.CurrentQuad.Grid[4][3] = EntityKlingon

	events := ExecuteCommanderScreening(g)
	if len(events) != 1 {
		t.Fatalf("expected 1 screening maneuver event, got %d", len(events))
	}

	if raider.Sector != (Coord{4, 4}) {
		t.Errorf("expected raider to interpose at (4, 4), got %v", raider.Sector)
	}
	if g.CurrentQuad.Grid[4][4] != EntityKlingon {
		t.Errorf("expected grid at (4, 4) to be occupied by Klingon raider")
	}
	if g.CurrentQuad.Grid[4][3] != EntityEmpty {
		t.Errorf("expected departed cell (4, 3) to be empty")
	}
}

func TestKlingonPack_CommanderScreening_TargetBlocked(t *testing.T) {
	g := NewGameWithSeed(105)
	g.Enterprise.Sector = Coord{4, 1}

	commander := &EnemyVessel{
		ID:          30,
		Faction:     FactionKlingon,
		Sector:      Coord{4, 6},
		IsCommander: true,
		Energy:      300,
	}
	raider := &EnemyVessel{
		ID:          31,
		Faction:     FactionKlingon,
		Sector:      Coord{3, 4},
		IsCommander: false,
		Energy:      200,
	}
	g.CurrentQuad.Enemies = []*EnemyVessel{commander, raider}
	g.CurrentQuad.Grid[4][6] = EntityCommander
	g.CurrentQuad.Grid[3][4] = EntityKlingon
	// Target cell (4, 4) is blocked by a star
	g.CurrentQuad.Grid[4][4] = EntityStar

	events := ExecuteCommanderScreening(g)
	if len(events) != 0 {
		t.Errorf("expected no screening when target cell is blocked, got %d events", len(events))
	}
	if raider.Sector != (Coord{3, 4}) {
		t.Errorf("expected raider to stay at (3, 4), got %v", raider.Sector)
	}
	if g.CurrentQuad.Grid[4][4] != EntityStar {
		t.Errorf("expected grid at (4, 4) to remain EntityStar")
	}
}

func TestKlingonPack_CommanderScreening_DestroyedUnits(t *testing.T) {
	g := NewGameWithSeed(106)
	g.Enterprise.Sector = Coord{4, 1}

	// Case 1: Destroyed Commander
	commanderDead := &EnemyVessel{
		ID:          40,
		Faction:     FactionKlingon,
		Sector:      Coord{4, 6},
		IsCommander: true,
		MaxEnergy:   300,
		Energy:      0,
	}
	raider := &EnemyVessel{
		ID:          41,
		Faction:     FactionKlingon,
		Sector:      Coord{3, 4},
		IsCommander: false,
		Energy:      200,
	}
	g.CurrentQuad.Enemies = []*EnemyVessel{commanderDead, raider}
	g.CurrentQuad.Grid[3][4] = EntityKlingon

	events := ExecuteCommanderScreening(g)
	if len(events) != 0 {
		t.Errorf("expected no screening for dead commander, got %d events", len(events))
	}

	// Case 2: Destroyed Raider
	commanderAlive := &EnemyVessel{
		ID:          42,
		Faction:     FactionKlingon,
		Sector:      Coord{4, 6},
		IsCommander: true,
		Energy:      300,
	}
	raiderDead := &EnemyVessel{
		ID:          43,
		Faction:     FactionKlingon,
		Sector:      Coord{3, 4},
		IsCommander: false,
		MaxEnergy:   200,
		Energy:      -10,
	}
	g.CurrentQuad.Enemies = []*EnemyVessel{commanderAlive, raiderDead}
	g.CurrentQuad.Grid[4][6] = EntityCommander

	events = ExecuteCommanderScreening(g)
	if len(events) != 0 {
		t.Errorf("expected no screening by dead raider, got %d events", len(events))
	}
}

func TestKlingonPack_DefensiveNilAndBounds(t *testing.T) {
	// Nil GameState
	if DetectCrossfireBracket(nil) {
		t.Errorf("expected false for nil GameState")
	}
	if mult := CalculatePackDamageMultiplier(nil); mult != 1.0 {
		t.Errorf("expected 1.0 for nil GameState, got %f", mult)
	}
	if events := ExecuteCommanderScreening(nil); events != nil {
		t.Errorf("expected nil events for nil GameState, got %v", events)
	}

	// GameState with nil enemies and empty lists
	g := NewGameWithSeed(107)
	g.CurrentQuad.Enemies = []*EnemyVessel{nil, nil}

	if DetectCrossfireBracket(g) {
		t.Errorf("expected false for nil enemies")
	}
	if events := ExecuteCommanderScreening(g); len(events) != 0 {
		t.Errorf("expected 0 events for nil enemies")
	}
}
