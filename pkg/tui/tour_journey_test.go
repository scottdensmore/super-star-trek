package tui

import (
	"testing"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/drydockmodal"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

func TestTourJourney_MultiSectorAndRefitWorkflow(t *testing.T) {
	tour := engine.NewTour(98765)
	th := theme.GetTheme(theme.ThemeModern)
	m := NewModelWithTour(tour, th)

	if m.Tour == nil || !m.Tour.Active {
		t.Fatal("expected active tour in model")
	}

	// 1. Clear Sector 1 hostiles
	g := m.Game
	for qx := 0; qx < engine.GalaxySize; qx++ {
		for qy := 0; qy < engine.GalaxySize; qy++ {
			g.Galaxy[qx][qy].Klingons = 0
			g.Galaxy[qx][qy].Commanders = 0
		}
	}
	g.KlingonsRemaining = 0

	// Trigger update loop turn
	m, _ = m.UpdateModel(tea.KeyMsg{Type: tea.KeyEnter})

	if !m.Tour.InDrydock {
		t.Fatalf("expected Tour to transition to Drydock, got InDrydock=%v", m.Tour.InDrydock)
	}
	if m.Tour.RequisitionPoints <= 0 {
		t.Errorf("expected positive RequisitionPoints, got %d", m.Tour.RequisitionPoints)
	}

	// 2. Buy Refit at Drydock (Dilithium Core)
	m.Drydock.Tour = m.Tour
	m, _ = m.UpdateModel(tea.KeyMsg{Type: tea.KeyEnter}) // Purchases highlighted item (Dilithium Core)

	if m.Tour.InstalledRefits[engine.RefitDilithiumCore] != 1 {
		t.Fatalf("expected Dilithium Core tier 1, got %d", m.Tour.InstalledRefits[engine.RefitDilithiumCore])
	}

	// 3. Disembark to Sector 2
	m, _ = m.UpdateModel(drydockmodal.DisembarkMsg{})

	if m.Tour.InDrydock {
		t.Errorf("expected InDrydock false after disembarking")
	}
	if m.Tour.CurrentSectorIndex != 1 {
		t.Errorf("expected CurrentSectorIndex 1, got %d", m.Tour.CurrentSectorIndex)
	}

	// 4. Verify refit stat boost transferred to Sector 2
	if m.Game.Enterprise.MaxEnergy != 3500.0 {
		t.Errorf("expected MaxEnergy 3500 in sector 2, got %f", m.Game.Enterprise.MaxEnergy)
	}
}
