package engine

import (
	"testing"
)

func TestAdversaries_EntityConstantsAndTypes(t *testing.T) {
	if EntityRomulan <= EntityWormhole {
		t.Errorf("expected EntityRomulan > EntityWormhole, got %d", EntityRomulan)
	}
	if EntityTholian <= EntityRomulan {
		t.Errorf("expected EntityTholian > EntityRomulan, got %d", EntityTholian)
	}
	if EntityPlasmaTorpedo <= EntityTholian {
		t.Errorf("expected EntityPlasmaTorpedo > EntityTholian, got %d", EntityPlasmaTorpedo)
	}
	if EntityTholianWeb <= EntityPlasmaTorpedo {
		t.Errorf("expected EntityTholianWeb > EntityPlasmaTorpedo, got %d", EntityTholianWeb)
	}

	enemy := &EnemyVessel{
		ID:          1,
		Faction:     FactionRomulan,
		Sector:      Coord{3, 4},
		Energy:      1200.0,
		Shields:     400.0,
		MaxEnergy:   1200.0,
		IsCommander: false,
		IsCloaked:   true,
	}

	if enemy.Faction != FactionRomulan || !enemy.IsCloaked {
		t.Fatalf("unexpected enemy vessel state: %+v", enemy)
	}

	plasma := &PlasmaTorpedo{
		ID:            101,
		SourceID:      1,
		Sector:        Coord{4, 4},
		Energy:        1000.0,
		TargetSector:  Coord{6, 6},
		TurnsInFlight: 1,
	}
	if plasma.Energy != 1000.0 || plasma.TurnsInFlight != 1 {
		t.Fatalf("unexpected plasma torpedo state: %+v", plasma)
	}

	web := &TholianWebSegment{
		Coord:    Coord{5, 5},
		Strength: 250.0,
	}
	if web.Strength != 250.0 {
		t.Fatalf("unexpected web segment state: %+v", web)
	}
}

func TestAdversaries_QuadrantStateSyncWithLegacyKlingons(t *testing.T) {
	g := NewGameWithSeed(42)

	// Add an enemy to Enemies slice
	klingonEnemy := &EnemyVessel{
		ID:          1,
		Faction:     FactionKlingon,
		Sector:      Coord{2, 3},
		Energy:      500.0,
		Shields:     200.0,
		IsCommander: true,
	}
	g.CurrentQuad.Enemies = []*EnemyVessel{klingonEnemy}
	SyncQuadrantEnemies(&g.CurrentQuad)

	if len(g.CurrentQuad.Klingons) != 1 {
		t.Fatalf("expected 1 Klingon in legacy slice, got %d", len(g.CurrentQuad.Klingons))
	}
	if g.CurrentQuad.Klingons[0].ID != 1 || !g.CurrentQuad.Klingons[0].IsCommander {
		t.Errorf("mismatched Klingon in legacy slice: %+v", g.CurrentQuad.Klingons[0])
	}
}

func TestAdversaries_QuadrantStateSync_EdgeCases(t *testing.T) {
	// Nil quadrant should not panic
	SyncQuadrantEnemies(nil)

	quad := &QuadrantState{
		Enemies: []*EnemyVessel{
			{
				ID:          1,
				Faction:     FactionRomulan,
				Sector:      Coord{1, 1},
				Energy:      1000,
				Shields:     300,
				IsCommander: false,
			},
			{
				ID:          2,
				Faction:     FactionKlingon,
				Sector:      Coord{2, 2},
				Energy:      600,
				Shields:     250,
				IsCommander: true,
				IsCloaked:   true,
			},
			{
				ID:          3,
				Faction:     FactionTholian,
				Sector:      Coord{3, 3},
				Energy:      500,
				Shields:     500,
				IsCommander: false,
			},
			{
				ID:          4,
				Faction:     FactionKlingon,
				Sector:      Coord{4, 4},
				Energy:      400,
				Shields:     150,
				IsCommander: false,
			},
		},
	}

	SyncQuadrantEnemies(quad)

	if len(quad.Klingons) != 2 {
		t.Fatalf("expected 2 Klingons in legacy slice, got %d", len(quad.Klingons))
	}
	if quad.Klingons[0].ID != 2 || !quad.Klingons[0].IsCommander || !quad.Klingons[0].IsCloaked {
		t.Errorf("unexpected first legacy Klingon: %+v", quad.Klingons[0])
	}
	if quad.Klingons[1].ID != 4 || quad.Klingons[1].IsCommander {
		t.Errorf("unexpected second legacy Klingon: %+v", quad.Klingons[1])
	}

	// Empty enemies slice
	quad.Enemies = nil
	SyncQuadrantEnemies(quad)
	if len(quad.Klingons) != 0 {
		t.Errorf("expected 0 legacy Klingons, got %d", len(quad.Klingons))
	}
}

func TestAdversaries_SandboxSpawningInPopulateQuadrant(t *testing.T) {
	// 1. When Rules.Adversaries is false, PopulateQuadrant should only populate Klingons, not Enemies.
	gClassic := NewGameWithSeed(42)
	gClassic.Rules.Adversaries = false
	gClassic.GalaxyChart[4][4] = 203 // 2 Klingons, 0 Bases, 3 Stars
	gClassic.PopulateQuadrant(Coord{4, 4}, Coord{1, 1})

	if len(gClassic.CurrentQuad.Klingons) != 2 {
		t.Fatalf("expected 2 Klingons in classic mode, got %d", len(gClassic.CurrentQuad.Klingons))
	}
	if len(gClassic.CurrentQuad.Enemies) != 0 {
		t.Fatalf("expected 0 Enemies in classic mode, got %d", len(gClassic.CurrentQuad.Enemies))
	}

	// 2. When Rules.Adversaries is true in an inner quadrant without a starbase:
	// One enemy should be converted to FactionRomulan (cloaked).
	gRomulan := NewGameWithSeed(42)
	gRomulan.Rules.Adversaries = true
	gRomulan.GalaxyChart[4][4] = 203 // 2 Klingons, 0 Bases, 3 Stars (inner quadrant)
	gRomulan.PopulateQuadrant(Coord{4, 4}, Coord{1, 1})

	if len(gRomulan.CurrentQuad.Enemies) != 2 {
		t.Fatalf("expected 2 Enemies in adversary mode, got %d", len(gRomulan.CurrentQuad.Enemies))
	}
	var foundRomulan bool
	var foundKlingon bool
	for _, enemy := range gRomulan.CurrentQuad.Enemies {
		if enemy.Faction == FactionRomulan {
			foundRomulan = true
			if !enemy.IsCloaked {
				t.Errorf("expected Romulan to be cloaked")
			}
			if gRomulan.CurrentQuad.Grid[enemy.Sector[0]][enemy.Sector[1]] != EntityEmpty {
				t.Errorf("expected cloaked Romulan grid cell to be EntityEmpty, got %v", gRomulan.CurrentQuad.Grid[enemy.Sector[0]][enemy.Sector[1]])
			}
		}
		if enemy.Faction == FactionKlingon {
			foundKlingon = true
		}
	}
	if !foundRomulan || !foundKlingon {
		t.Errorf("expected 1 Romulan and 1 Klingon in inner quad, got romulan=%v, klingon=%v", foundRomulan, foundKlingon)
	}
	// Legacy slice should only contain the Klingon (Romulans omitted from legacy Klingons)
	if len(gRomulan.CurrentQuad.Klingons) != 1 {
		t.Errorf("expected 1 legacy Klingon, got %d", len(gRomulan.CurrentQuad.Klingons))
	}

	// 3. When Rules.Adversaries is true on a border quadrant:
	// One enemy should be converted to FactionTholian.
	gBorder := NewGameWithSeed(42)
	gBorder.Rules.Adversaries = true
	gBorder.GalaxyChart[1][4] = 203 // Border quadrant (row 1)
	gBorder.PopulateQuadrant(Coord{1, 4}, Coord{1, 1})

	var foundTholian bool
	for _, enemy := range gBorder.CurrentQuad.Enemies {
		if enemy.Faction == FactionTholian {
			foundTholian = true
			if gBorder.CurrentQuad.Grid[enemy.Sector[0]][enemy.Sector[1]] != EntityTholian {
				t.Errorf("expected Tholian grid cell to be EntityTholian, got %v", gBorder.CurrentQuad.Grid[enemy.Sector[0]][enemy.Sector[1]])
			}
		}
	}
	if !foundTholian {
		t.Errorf("expected 1 Tholian on border quad")
	}

	// 4. When Rules.Adversaries is true in an inner quadrant with a starbase:
	// One enemy should be converted to FactionTholian (border or starbase defense).
	gStarbase := NewGameWithSeed(42)
	gStarbase.Rules.Adversaries = true
	gStarbase.GalaxyChart[4][4] = 213 // 2 Klingons, 1 Base, 3 Stars
	gStarbase.PopulateQuadrant(Coord{4, 4}, Coord{1, 1})

	foundTholian = false
	for _, enemy := range gStarbase.CurrentQuad.Enemies {
		if enemy.Faction == FactionTholian {
			foundTholian = true
		}
	}
	if !foundTholian {
		t.Errorf("expected 1 Tholian in quadrant with starbase")
	}
}


