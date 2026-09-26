package engine

import (
	"testing"
)

func TestTholian_BinarySpinningAndWebPlacement(t *testing.T) {
	g := NewGameWithSeed(9999)
	g.Enterprise.Sector = Coord{4, 4}

	alpha := &EnemyVessel{
		ID:           201,
		Faction:      FactionTholian,
		Sector:       Coord{2, 2},
		Energy:       800.0,
		SpecialState: 0, // Alpha
	}
	beta := &EnemyVessel{
		ID:           202,
		Faction:      FactionTholian,
		Sector:       Coord{6, 6},
		Energy:       800.0,
		SpecialState: 1, // Beta
	}
	g.CurrentQuad.Enemies = []*EnemyVessel{alpha, beta}
	g.CurrentQuad.Grid[2][2] = EntityTholian
	g.CurrentQuad.Grid[6][6] = EntityTholian

	events := TholianTurn(g)
	if len(events) == 0 {
		t.Fatal("expected events from Tholian turn")
	}

	if len(g.CurrentQuad.WebSegments) < 2 {
		t.Fatalf("expected at least 2 web segments deposited by pair, got %d", len(g.CurrentQuad.WebSegments))
	}

	// Verify containment percentage calculation
	containment := CalculateWebContainment(g)
	if containment <= 0 || containment > 100 {
		t.Errorf("expected containment percentage between 1 and 100, got %f", containment)
	}

	// Destroy a web segment
	firstSegment := g.CurrentQuad.WebSegments[0].Coord
	destroyed, breachEvents := DamageWebSegment(g, firstSegment, 300.0)
	if !destroyed || len(breachEvents) == 0 {
		t.Errorf("expected web segment at %v to be breached by 300 damage", firstSegment)
	}
}

func TestTholian_SpinnerDestructionCascade(t *testing.T) {
	g := NewGameWithSeed(9999)
	g.CurrentQuad.WebSegments = []*TholianWebSegment{
		{Coord: Coord{1, 1}, Strength: 250.0},
		{Coord: Coord{1, 2}, Strength: 250.0},
	}
	g.CurrentQuad.Grid[1][1] = EntityTholianWeb
	g.CurrentQuad.Grid[1][2] = EntityTholianWeb

	// No Tholians remaining in quadrant
	g.CurrentQuad.Enemies = nil
	events := TholianTurn(g)

	if len(g.CurrentQuad.WebSegments) != 0 {
		t.Errorf("expected web segments to collapse with 0 spinners remaining")
	}
	if g.CurrentQuad.Grid[1][1] != EntityEmpty {
		t.Errorf("expected grid cell to be empty after web collapse")
	}
	if len(events) == 0 {
		t.Errorf("expected collapse event")
	}
}

func TestTholian_DestroyedSpinnersCascade(t *testing.T) {
	g := NewGameWithSeed(9999)
	g.CurrentQuad.WebSegments = []*TholianWebSegment{
		{Coord: Coord{1, 1}, Strength: 250.0},
		{Coord: Coord{1, 2}, Strength: 250.0},
	}
	g.CurrentQuad.Grid[1][1] = EntityTholianWeb
	g.CurrentQuad.Grid[1][2] = EntityTholianWeb

	// Enemies slice contains Tholians with Energy <= 0 (destroyed) and a nil entry
	g.CurrentQuad.Enemies = []*EnemyVessel{
		{
			ID:           201,
			Faction:      FactionTholian,
			Sector:       Coord{2, 2},
			Energy:       0.0,
			SpecialState: 0,
		},
		{
			ID:           202,
			Faction:      FactionTholian,
			Sector:       Coord{6, 6},
			Energy:       -50.0,
			SpecialState: 1,
		},
		nil,
	}

	events := TholianTurn(g)

	if len(g.CurrentQuad.WebSegments) != 0 {
		t.Errorf("expected web segments to collapse when all spinners have Energy <= 0")
	}
	if g.CurrentQuad.Grid[1][1] != EntityEmpty || g.CurrentQuad.Grid[1][2] != EntityEmpty {
		t.Errorf("expected grid cells to be empty after web collapse")
	}
	if len(events) != 1 || events[0].EventType() != "WebCollapsed" {
		t.Errorf("expected WebCollapsed event, got %v", events)
	}
}

func TestTholian_PerimeterNavigation(t *testing.T) {
	// Test Alpha (clockwise):
	// Top edge: (1, 2) -> (1, 3)
	dr, dc := getPerimeterStep(Coord{1, 2}, 0)
	if dr != 0 || dc != 1 {
		t.Errorf("Alpha on top edge: expected (0, 1), got (%d, %d)", dr, dc)
	}
	// Right edge: (2, 8) -> (3, 8)
	dr, dc = getPerimeterStep(Coord{2, 8}, 0)
	if dr != 1 || dc != 0 {
		t.Errorf("Alpha on right edge: expected (1, 0), got (%d, %d)", dr, dc)
	}
	// Bottom edge: (8, 7) -> (8, 6)
	dr, dc = getPerimeterStep(Coord{8, 7}, 0)
	if dr != 0 || dc != -1 {
		t.Errorf("Alpha on bottom edge: expected (0, -1), got (%d, %d)", dr, dc)
	}
	// Left edge: (7, 1) -> (6, 1)
	dr, dc = getPerimeterStep(Coord{7, 1}, 0)
	if dr != -1 || dc != 0 {
		t.Errorf("Alpha on left edge: expected (-1, 0), got (%d, %d)", dr, dc)
	}

	// Test Beta (counter-clockwise):
	// Top edge: (1, 7) -> (1, 6)
	dr, dc = getPerimeterStep(Coord{1, 7}, 1)
	if dr != 0 || dc != -1 {
		t.Errorf("Beta on top edge: expected (0, -1), got (%d, %d)", dr, dc)
	}
	// Left edge: (2, 1) -> (3, 1)
	dr, dc = getPerimeterStep(Coord{2, 1}, 1)
	if dr != 1 || dc != 0 {
		t.Errorf("Beta on left edge: expected (1, 0), got (%d, %d)", dr, dc)
	}
	// Bottom edge: (8, 2) -> (8, 3)
	dr, dc = getPerimeterStep(Coord{8, 2}, 1)
	if dr != 0 || dc != 1 {
		t.Errorf("Beta on bottom edge: expected (0, 1), got (%d, %d)", dr, dc)
	}
	// Right edge: (7, 8) -> (6, 8)
	dr, dc = getPerimeterStep(Coord{7, 8}, 1)
	if dr != -1 || dc != 0 {
		t.Errorf("Beta on right edge: expected (-1, 0), got (%d, %d)", dr, dc)
	}
}

func TestTholian_WebDamageAndBreach(t *testing.T) {
	g := NewGameWithSeed(9999)
	g.CurrentQuad.WebSegments = []*TholianWebSegment{
		{Coord: Coord{3, 3}, Strength: 250.0},
	}
	g.CurrentQuad.Grid[3][3] = EntityTholianWeb

	// Test nil GameState
	destroyed, ev := DamageWebSegment(nil, Coord{3, 3}, 100.0)
	if destroyed || len(ev) != 0 {
		t.Error("expected false and no events for nil GameState")
	}

	// Test non-positive damage
	destroyed, ev = DamageWebSegment(g, Coord{3, 3}, 0.0)
	if destroyed || len(ev) != 0 {
		t.Error("expected false and no events for zero damage")
	}
	destroyed, ev = DamageWebSegment(g, Coord{3, 3}, -50.0)
	if destroyed || len(ev) != 0 {
		t.Error("expected false and no events for negative damage")
	}

	// Test partial damage (non-lethal)
	destroyed, ev = DamageWebSegment(g, Coord{3, 3}, 100.0)
	if destroyed || len(ev) != 0 {
		t.Error("expected false and no events for partial damage")
	}
	if g.CurrentQuad.WebSegments[0].Strength != 150.0 {
		t.Errorf("expected web strength 150.0, got %f", g.CurrentQuad.WebSegments[0].Strength)
	}
	if g.CurrentQuad.Grid[3][3] != EntityTholianWeb {
		t.Error("expected grid cell to still be EntityTholianWeb")
	}

	// Test non-existent coord
	destroyed, ev = DamageWebSegment(g, Coord{1, 1}, 100.0)
	if destroyed || len(ev) != 0 {
		t.Error("expected false and no events for non-existent segment")
	}

	// Test lethal damage
	destroyed, ev = DamageWebSegment(g, Coord{3, 3}, 150.0)
	if !destroyed || len(ev) != 1 {
		t.Fatal("expected web to be destroyed with 1 event")
	}
	if g.CurrentQuad.Grid[3][3] != EntityEmpty {
		t.Error("expected grid cell to be empty after breach")
	}
	if len(g.CurrentQuad.WebSegments) != 0 {
		t.Error("expected 0 web segments remaining")
	}
	if ev[0].EventType() != "WebBreached" {
		t.Errorf("expected EventType WebBreached, got %s", ev[0].EventType())
	}

	// Test nil element inside WebSegments
	g.CurrentQuad.WebSegments = []*TholianWebSegment{nil, {Coord: Coord{5, 5}, Strength: 250.0}}
	g.CurrentQuad.Grid[5][5] = EntityTholianWeb
	destroyed, ev = DamageWebSegment(g, Coord{5, 5}, 300.0)
	if !destroyed || len(ev) != 1 {
		t.Fatal("expected segment at (5,5) to be destroyed even with nil in slice")
	}
	if len(g.CurrentQuad.WebSegments) != 1 || g.CurrentQuad.WebSegments[0] != nil {
		t.Errorf("expected only nil element remaining in WebSegments, got %v", g.CurrentQuad.WebSegments)
	}
}

func TestTholian_CalculateWebContainment(t *testing.T) {
	// Nil state
	if val := CalculateWebContainment(nil); val != 0.0 {
		t.Errorf("expected 0.0 for nil state, got %f", val)
	}

	g := NewGameWithSeed(9999)
	g.CurrentQuad.WebSegments = nil
	if val := CalculateWebContainment(g); val != 0.0 {
		t.Errorf("expected 0.0 for 0 segments, got %f", val)
	}

	// 10 segments = 50%
	for i := 1; i <= 10; i++ {
		g.CurrentQuad.WebSegments = append(g.CurrentQuad.WebSegments, &TholianWebSegment{
			Coord:    Coord{1, i % 8 + 1},
			Strength: 250.0,
		})
	}
	if val := CalculateWebContainment(g); val != 50.0 {
		t.Errorf("expected 50.0%% containment, got %f", val)
	}

	// 20 segments = 100%
	for i := 11; i <= 20; i++ {
		g.CurrentQuad.WebSegments = append(g.CurrentQuad.WebSegments, &TholianWebSegment{
			Coord:    Coord{2, i % 8 + 1},
			Strength: 250.0,
		})
	}
	if val := CalculateWebContainment(g); val != 100.0 {
		t.Errorf("expected 100.0%% containment, got %f", val)
	}

	// 25 segments = capped at 100%
	for i := 21; i <= 25; i++ {
		g.CurrentQuad.WebSegments = append(g.CurrentQuad.WebSegments, &TholianWebSegment{
			Coord:    Coord{3, i % 8 + 1},
			Strength: 250.0,
		})
	}
	if val := CalculateWebContainment(g); val != 100.0 {
		t.Errorf("expected 100.0%% cap, got %f", val)
	}
}

func TestTholian_MovementBlockedByObstacle(t *testing.T) {
	g := NewGameWithSeed(9999)
	spinner := &EnemyVessel{
		ID:           201,
		Faction:      FactionTholian,
		Sector:       Coord{1, 2},
		Energy:       800.0,
		SpecialState: 0, // Alpha: moves right to (1, 3)
	}
	g.CurrentQuad.Enemies = []*EnemyVessel{spinner}
	g.CurrentQuad.Grid[1][2] = EntityTholian
	// Block (1, 3) with a star
	g.CurrentQuad.Grid[1][3] = EntityStar

	events := TholianTurn(g)
	if len(events) != 0 {
		t.Errorf("expected no events when spinner movement is blocked, got %d", len(events))
	}
	if spinner.Sector != (Coord{1, 2}) {
		t.Errorf("expected spinner to remain at (1, 2), got %v", spinner.Sector)
	}
	if len(g.CurrentQuad.WebSegments) != 0 {
		t.Errorf("expected no web segments placed when movement is blocked")
	}
}

func TestTholian_NilGameState(t *testing.T) {
	events := TholianTurn(nil)
	if events != nil {
		t.Errorf("expected nil events for nil GameState, got %v", events)
	}
}

func TestTholian_EventTypes(t *testing.T) {
	e1 := EventWebSegmentLaid{SpinnerID: 1, Coord: Coord{2, 3}}
	if e1.EventType() != "WebSegmentLaid" {
		t.Errorf("expected WebSegmentLaid, got %s", e1.EventType())
	}

	e2 := EventWebBreached{Coord: Coord{2, 3}}
	if e2.EventType() != "WebBreached" {
		t.Errorf("expected WebBreached, got %s", e2.EventType())
	}

	e3 := EventWebCollapsed{SegmentsCount: 5}
	if e3.EventType() != "WebCollapsed" {
		t.Errorf("expected WebCollapsed, got %s", e3.EventType())
	}
}
