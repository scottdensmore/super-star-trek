package engine

import (
	"testing"
)

func TestNewTour_InitializesFourSectors(t *testing.T) {
	tour := NewTour(42)
	if tour == nil {
		t.Fatal("expected non-nil TourState")
	}
	if !tour.Active {
		t.Errorf("expected tour.Active to be true, got %v", tour.Active)
	}
	if len(tour.Sectors) != 4 {
		t.Fatalf("expected 4 sectors in tour, got %d", len(tour.Sectors))
	}
	if tour.CurrentSectorIndex != 0 {
		t.Errorf("expected CurrentSectorIndex 0, got %d", tour.CurrentSectorIndex)
	}
	if tour.RequisitionPoints != 0 {
		t.Errorf("expected initial 0 RequisitionPoints, got %d", tour.RequisitionPoints)
	}
	if tour.Sectors[0].Objective != ObjectiveBorderPatrol {
		t.Errorf("expected sector 1 objective %s, got %s", ObjectiveBorderPatrol, tour.Sectors[0].Objective)
	}
}

func TestTourState_SectorProgressionAndBounty(t *testing.T) {
	tour := NewTour(12345)
	state := tour.StartCurrentSector()
	if state == nil {
		t.Fatal("expected non-nil GameState for sector 1")
	}
	if tour.InDrydock {
		t.Errorf("expected InDrydock false during active sector, got true")
	}

	// Eliminate hostiles to satisfy sector 1 objective
	for qx := 0; qx < GalaxySize; qx++ {
		for qy := 0; qy < GalaxySize; qy++ {
			q := state.Galaxy[qx][qy]
			q.Klingons = 0
			q.Commanders = 0
			q.SuperCommanders = 0
		}
	}
	state.KlingonsRemaining = 0

	cleared, failed, bounty := tour.EvaluateSector()
	if !cleared {
		t.Errorf("expected sector to be cleared, got %v", cleared)
	}
	if failed {
		t.Errorf("expected sector not failed, got %v", failed)
	}
	if bounty <= 0 {
		t.Errorf("expected positive bounty payout, got %d", bounty)
	}

	tour.AdvanceToDrydock(bounty)
	if !tour.InDrydock {
		t.Errorf("expected InDrydock true after advancing, got false")
	}
	if tour.RequisitionPoints != bounty {
		t.Errorf("expected RequisitionPoints == bounty (%d), got %d", bounty, tour.RequisitionPoints)
	}
	if tour.SectorsCompleted != 1 {
		t.Errorf("expected SectorsCompleted 1, got %d", tour.SectorsCompleted)
	}

	// Disembark to sector 2
	nextState, err := tour.DisembarkToNextSector()
	if err != nil {
		t.Fatalf("unexpected error disembarking: %v", err)
	}
	if nextState == nil {
		t.Fatal("expected non-nil nextState")
	}
	if tour.InDrydock {
		t.Errorf("expected InDrydock false after disembarking")
	}
	if tour.CurrentSectorIndex != 1 {
		t.Errorf("expected CurrentSectorIndex 1, got %d", tour.CurrentSectorIndex)
	}
}

func TestTourState_PermadeathOnLoss(t *testing.T) {
	tour := NewTour(999)
	state := tour.StartCurrentSector()

	// Enterprise destroyed
	state.GameOver = true
	state.GameOverReason = GameOverDestroyed

	cleared, failed, _ := tour.EvaluateSector()
	if cleared {
		t.Errorf("expected cleared false on destroyed ship")
	}
	if !failed {
		t.Errorf("expected failed true on destroyed ship")
	}
	if tour.Active {
		t.Errorf("expected tour.Active false on permadeath")
	}
	if !tour.Failed {
		t.Errorf("expected tour.Failed true on permadeath")
	}
}

func TestNewTour_ZeroSeedUsesTime(t *testing.T) {
	tour := NewTour(0)
	if tour == nil {
		t.Fatal("expected non-nil TourState")
	}
	if tour.Seed == 0 {
		t.Errorf("expected non-zero seed when passing 0")
	}
	if tour.ID == "" {
		t.Errorf("expected non-empty Tour ID")
	}
}

func TestTourState_CurrentSector_Bounds(t *testing.T) {
	tour := NewTour(42)
	tour.CurrentSectorIndex = -1
	if tour.CurrentSector() != nil {
		t.Errorf("expected nil for negative CurrentSectorIndex")
	}
	if tour.StartCurrentSector() != nil {
		t.Errorf("expected nil StartCurrentSector for negative CurrentSectorIndex")
	}

	tour.CurrentSectorIndex = 10
	if tour.CurrentSector() != nil {
		t.Errorf("expected nil for out-of-bounds CurrentSectorIndex")
	}
	if tour.StartCurrentSector() != nil {
		t.Errorf("expected nil StartCurrentSector for out-of-bounds CurrentSectorIndex")
	}
}

func TestTourState_EvaluateSector_NilState(t *testing.T) {
	tour := NewTour(42)
	cleared, failed, bounty := tour.EvaluateSector()
	if cleared || failed || bounty != 0 {
		t.Errorf("expected false, false, 0 for nil CurrentGameState, got %v, %v, %d", cleared, failed, bounty)
	}
}

func TestTourState_EvaluateSector_FlawlessBonus(t *testing.T) {
	tour := NewTour(100)
	state := tour.StartCurrentSector()
	state.KlingonsRemaining = 0

	// Case 1: No damage -> flawless bonus (250) included
	cleared, failed, bountyNoDamage := tour.EvaluateSector()
	if !cleared || failed {
		t.Fatalf("expected cleared without fail, got cleared=%v failed=%v", cleared, failed)
	}

	// Case 2: Damage on subsystem -> no flawless bonus
	state.Enterprise.Damage[0] = 2.0
	_, _, bountyWithDamage := tour.EvaluateSector()
	if bountyNoDamage-bountyWithDamage != 250 {
		t.Errorf("expected flawless bonus difference of 250, got %d vs %d (diff=%d)",
			bountyNoDamage, bountyWithDamage, bountyNoDamage-bountyWithDamage)
	}
}

func TestTourState_DisembarkToNextSector_NotDocked(t *testing.T) {
	tour := NewTour(100)
	tour.StartCurrentSector()
	// tour.InDrydock is false
	_, err := tour.DisembarkToNextSector()
	if err == nil {
		t.Fatal("expected error when disembarking while not docked")
	}
}

func TestTourState_CompleteTourSequence(t *testing.T) {
	tour := NewTour(100)
	for i := 0; i < len(tour.Sectors); i++ {
		state := tour.StartCurrentSector()
		if state == nil {
			t.Fatalf("expected valid state for sector %d", i+1)
		}
		state.KlingonsRemaining = 0
		cleared, failed, bounty := tour.EvaluateSector()
		if !cleared || failed {
			t.Fatalf("sector %d evaluation failed: cleared=%v, failed=%v", i+1, cleared, failed)
		}
		tour.AdvanceToDrydock(bounty)
		nextState, err := tour.DisembarkToNextSector()
		if err != nil {
			t.Fatalf("unexpected error on sector %d disembark: %v", i+1, err)
		}
		if i == len(tour.Sectors)-1 {
			// Finished final sector
			if nextState != nil {
				t.Errorf("expected nil nextState after final sector")
			}
			if !tour.Completed {
				t.Errorf("expected tour.Completed true")
			}
			if tour.Active {
				t.Errorf("expected tour.Active false")
			}
			if tour.InDrydock {
				t.Errorf("expected tour.InDrydock false")
			}
		} else {
			if nextState == nil {
				t.Fatalf("expected non-nil nextState for sector %d", i+2)
			}
		}
	}
	if tour.SectorsCompleted != 4 {
		t.Errorf("expected 4 sectors completed, got %d", tour.SectorsCompleted)
	}
}

