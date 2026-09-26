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

