package engine

import (
	"testing"
)

func TestRefits_ApplyEnergyAndTorpedoScaling(t *testing.T) {
	g := NewGameWithSeed(100)
	refits := map[RefitID]int{
		RefitDilithiumCore: 2, // Tier 2: 3000 + 1000 = 4000
		RefitTorpedoBays:   1, // Tier 1: 10 + 4 = 14
	}

	ApplyRefits(g, refits)

	if g.Enterprise.MaxEnergy != 4000.0 {
		t.Errorf("expected MaxEnergy 4000, got %f", g.Enterprise.MaxEnergy)
	}
	if g.Enterprise.Energy != 4000.0 {
		t.Errorf("expected Energy 4000, got %f", g.Enterprise.Energy)
	}
	if g.Enterprise.MaxTorpedoes != 14 {
		t.Errorf("expected MaxTorpedoes 14, got %d", g.Enterprise.MaxTorpedoes)
	}
	if g.Enterprise.Torpedoes != 14 {
		t.Errorf("expected Torpedoes 14, got %d", g.Enterprise.Torpedoes)
	}
}

func TestRefits_ApplyRefits_CapsInitialEnergy(t *testing.T) {
	// When GameState has 5000 energy initially (e.g. from NewGame),
	// ApplyRefits with no dilithium core should set MaxEnergy=3000 and cap Energy to 3000.
	g := NewGame(1234, SkillGood, LengthMedium)
	if g.Enterprise.Energy != 5000.0 {
		t.Fatalf("expected initial energy 5000, got %f", g.Enterprise.Energy)
	}

	ApplyRefits(g, map[RefitID]int{})
	if g.Enterprise.MaxEnergy != 3000.0 {
		t.Errorf("expected MaxEnergy 3000, got %f", g.Enterprise.MaxEnergy)
	}
	if g.Enterprise.Energy != 3000.0 {
		t.Errorf("expected Energy capped to 3000, got %f", g.Enterprise.Energy)
	}

	// Now with Tier 1 Dilithium Core (3500 max)
	g.Enterprise.Energy = 5000.0
	ApplyRefits(g, map[RefitID]int{RefitDilithiumCore: 1})
	if g.Enterprise.MaxEnergy != 3500.0 {
		t.Errorf("expected MaxEnergy 3500, got %f", g.Enterprise.MaxEnergy)
	}
	if g.Enterprise.Energy != 3500.0 {
		t.Errorf("expected Energy capped to 3500, got %f", g.Enterprise.Energy)
	}
}

func TestRefits_PurchaseWorkflowAndConstraints(t *testing.T) {
	tour := NewTour(555)
	tour.RequisitionPoints = 1000

	// Purchase Tier 1 Dilithium Core (cost 500)
	err := PurchaseRefit(tour, RefitDilithiumCore)
	if err != nil {
		t.Fatalf("unexpected error purchasing refit: %v", err)
	}
	if tour.InstalledRefits[RefitDilithiumCore] != 1 {
		t.Errorf("expected Dilithium Core tier 1, got %d", tour.InstalledRefits[RefitDilithiumCore])
	}
	if tour.RequisitionPoints != 500 {
		t.Errorf("expected 500 requisition points remaining, got %d", tour.RequisitionPoints)
	}

	// Attempting Tier 2 costs 1000, but only 500 remaining -> should error
	err = PurchaseRefit(tour, RefitDilithiumCore)
	if err == nil {
		t.Errorf("expected error for insufficient funds, got nil")
	}

	// Test max tier constraint
	tour.RequisitionPoints = 5000
	err = PurchaseRefit(tour, RefitDilithiumCore) // tier 2
	if err != nil {
		t.Fatalf("unexpected error buying tier 2: %v", err)
	}
	err = PurchaseRefit(tour, RefitDilithiumCore) // tier 3
	if err != nil {
		t.Fatalf("unexpected error buying tier 3: %v", err)
	}
	err = PurchaseRefit(tour, RefitDilithiumCore) // beyond tier 3 -> error
	if err == nil {
		t.Errorf("expected error purchasing beyond tier 3, got nil")
	}

	// Unknown refit
	err = PurchaseRefit(tour, RefitID("unknown_module"))
	if err == nil {
		t.Errorf("expected error for unknown refit ID, got nil")
	}

	// Nil tour
	err = PurchaseRefit(nil, RefitDilithiumCore)
	if err == nil {
		t.Errorf("expected error for nil tour, got nil")
	}
}

func TestRefits_TorpedoDamageScaling(t *testing.T) {
	g := NewGameWithSeed(200)
	// Base damage
	baseYield := CalculateTorpedoDamage(g, 100.0)

	// Apply High Yield Tier 2 (+50%)
	g.ActiveRefits = map[RefitID]int{RefitTorpedoCasings: 2}
	boostedYield := CalculateTorpedoDamage(g, 100.0)

	expectedYield := baseYield * 1.5
	if boostedYield != expectedYield {
		t.Errorf("expected boosted yield %f, got %f", expectedYield, boostedYield)
	}
}

func TestRefits_ShieldAbsorptionReduction(t *testing.T) {
	g := NewGameWithSeed(300)
	incomingRaw := 200.0

	// Without refits
	unshieldedLoss := CalculateShieldDamageAbsorption(g, incomingRaw)

	// With Tier 2 Reinforced Deflectors (-30% shield drain)
	g.ActiveRefits = map[RefitID]int{RefitDeflectorGrid: 2}
	reinforcedLoss := CalculateShieldDamageAbsorption(g, incomingRaw)

	if reinforcedLoss >= unshieldedLoss {
		t.Errorf("expected reinforced loss %f < unshielded loss %f", reinforcedLoss, unshieldedLoss)
	}

	expectedLoss := incomingRaw * 0.70
	if mathAbs(reinforcedLoss-expectedLoss) > 0.001 {
		t.Errorf("expected reinforced loss %f, got %f", expectedLoss, reinforcedLoss)
	}
}

func TestRefits_RepairNanitesWarpMovement(t *testing.T) {
	g := NewGameWithSeed(400)
	g.ActiveRefits = map[RefitID]int{RefitDamageNanites: 2} // Tier 2: +1.0d repair per warp move
	g.Enterprise.Damage[DeviceWarp] = 2.0
	g.Enterprise.Damage[DevicePhasers] = 0.8

	act := ActionMove{Warp: 1.0, Course: 0}
	_, err := act.Execute(g)
	if err != nil {
		t.Fatalf("warp move failed: %v", err)
	}

	// Warp damage reduced from 2.0 by 1.0 -> 1.0
	if g.Enterprise.Damage[DeviceWarp] != 1.0 {
		t.Errorf("expected warp damage 1.0, got %f", g.Enterprise.Damage[DeviceWarp])
	}
	// Phaser damage reduced from 0.8 by 1.0 -> clamped to 0
	if g.Enterprise.Damage[DevicePhasers] != 0.0 {
		t.Errorf("expected phaser damage 0.0, got %f", g.Enterprise.Damage[DevicePhasers])
	}
}

func TestRefits_GetRefitDefinition(t *testing.T) {
	def := GetRefitDefinition(RefitDilithiumCore)
	if def == nil {
		t.Fatalf("expected definition for RefitDilithiumCore, got nil")
	}
	if def.Name != "Dilithium Core Tuning" {
		t.Errorf("unexpected name: %s", def.Name)
	}

	invalidDef := GetRefitDefinition(RefitID("nonexistent"))
	if invalidDef != nil {
		t.Errorf("expected nil for nonexistent refit, got %+v", invalidDef)
	}
}

func mathAbs(x float64) float64 {
	if x < 0 {
		return -x
	}
	return x
}
