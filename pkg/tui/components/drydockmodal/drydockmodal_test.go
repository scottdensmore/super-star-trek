package drydockmodal

import (
	"strings"
	"testing"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

func TestDrydockModal_NavigationAndSelection(t *testing.T) {
	tour := engine.NewTour(404)
	tour.InDrydock = true
	tour.RequisitionPoints = 2000

	m := New(tour, theme.GetTheme(theme.ThemeModern))
	if m.selectedIndex != 0 {
		t.Errorf("expected initial selectedIndex 0, got %d", m.selectedIndex)
	}

	// Move down
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyDown})
	if m.selectedIndex != 1 {
		t.Errorf("expected selectedIndex 1 after down key, got %d", m.selectedIndex)
	}

	// Move up
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyUp})
	if m.selectedIndex != 0 {
		t.Errorf("expected selectedIndex 0 after up key, got %d", m.selectedIndex)
	}

	// Move up beyond top bound
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyUp})
	if m.selectedIndex != 0 {
		t.Errorf("expected selectedIndex to remain 0 at top bound, got %d", m.selectedIndex)
	}

	// Test vim keys j and k
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'j'}})
	if m.selectedIndex != 1 {
		t.Errorf("expected selectedIndex 1 after 'j' key, got %d", m.selectedIndex)
	}

	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'k'}})
	if m.selectedIndex != 0 {
		t.Errorf("expected selectedIndex 0 after 'k' key, got %d", m.selectedIndex)
	}

	// Move down to bottom bound
	for i := 0; i < len(engine.RefitCatalog)+2; i++ {
		m, _ = m.Update(tea.KeyMsg{Type: tea.KeyDown})
	}
	if m.selectedIndex != len(engine.RefitCatalog)-1 {
		t.Errorf("expected selectedIndex to clamp at %d, got %d", len(engine.RefitCatalog)-1, m.selectedIndex)
	}
}

func TestDrydockModal_PurchaseRefit(t *testing.T) {
	tour := engine.NewTour(405)
	tour.InDrydock = true
	tour.RequisitionPoints = 1000 // Enough for Dilithium Core (500)

	m := New(tour, theme.GetTheme(theme.ThemeModern))
	m.selectedIndex = 0 // Dilithium Core

	m, cmd := m.Update(tea.KeyMsg{Type: tea.KeyEnter})
	if cmd == nil {
		t.Fatal("expected command on purchase")
	}
	if tour.InstalledRefits[engine.RefitDilithiumCore] != 1 {
		t.Errorf("expected Dilithium Core tier 1, got %d", tour.InstalledRefits[engine.RefitDilithiumCore])
	}
	if tour.RequisitionPoints != 500 {
		t.Errorf("expected 500 requisition remaining, got %d", tour.RequisitionPoints)
	}

	msg := cmd()
	purchasedMsg, ok := msg.(RefitPurchasedMsg)
	if !ok {
		t.Fatalf("expected RefitPurchasedMsg, got %T", msg)
	}
	if purchasedMsg.RefitID != engine.RefitDilithiumCore {
		t.Errorf("expected refit ID %s, got %s", engine.RefitDilithiumCore, purchasedMsg.RefitID)
	}
	if purchasedMsg.Tier != 1 {
		t.Errorf("expected tier 1, got %d", purchasedMsg.Tier)
	}
}

func TestDrydockModal_PurchaseRefit_InsufficientPoints(t *testing.T) {
	tour := engine.NewTour(407)
	tour.InDrydock = true
	tour.RequisitionPoints = 100 // Dilithium core costs 500

	m := New(tour, theme.GetTheme(theme.ThemeModern))
	m.selectedIndex = 0

	m, cmd := m.Update(tea.KeyMsg{Type: tea.KeyEnter})
	if cmd != nil {
		t.Errorf("expected nil cmd on failed purchase, got %v", cmd)
	}
	if m.errorMessage == "" {
		t.Error("expected error message for insufficient requisition")
	}
	if !strings.Contains(m.errorMessage, "insufficient requisition") {
		t.Errorf("expected error message to contain 'insufficient requisition', got %q", m.errorMessage)
	}
	if tour.InstalledRefits[engine.RefitDilithiumCore] != 0 {
		t.Errorf("expected Dilithium Core to remain tier 0, got %d", tour.InstalledRefits[engine.RefitDilithiumCore])
	}

	// Verify error message is rendered in View()
	viewStr := m.View()
	if !strings.Contains(viewStr, "insufficient requisition") {
		t.Errorf("expected view to contain error message, got:\n%s", viewStr)
	}

	// Moving cursor clears error message
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyDown})
	if m.errorMessage != "" {
		t.Errorf("expected error message cleared on navigation, got %q", m.errorMessage)
	}
}

func TestDrydockModal_PurchaseRefit_MaxTier(t *testing.T) {
	tour := engine.NewTour(408)
	tour.InDrydock = true
	tour.RequisitionPoints = 10000
	tour.InstalledRefits[engine.RefitDilithiumCore] = 3

	m := New(tour, theme.GetTheme(theme.ThemeModern))
	m.selectedIndex = 0

	m, cmd := m.Update(tea.KeyMsg{Type: tea.KeyEnter})
	if cmd != nil {
		t.Errorf("expected nil cmd when refit is already max tier, got %v", cmd)
	}
	if m.errorMessage == "" {
		t.Error("expected error message for max tier")
	}
}

func TestDrydockModal_Disembark(t *testing.T) {
	tour := engine.NewTour(406)
	tour.InDrydock = true

	m := New(tour, theme.GetTheme(theme.ThemeModern))
	_, cmd := m.Update(tea.KeyMsg{Type: tea.KeySpace})
	if cmd == nil {
		t.Fatal("expected DisembarkMsg command on Space key")
	}
	msg := cmd()
	if _, ok := msg.(DisembarkMsg); !ok {
		t.Errorf("expected msg to be DisembarkMsg, got %T", msg)
	}

	// Test 'd' key
	_, cmdD := m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'d'}})
	if cmdD == nil {
		t.Fatal("expected DisembarkMsg command on 'd' key")
	}
	if _, ok := cmdD().(DisembarkMsg); !ok {
		t.Errorf("expected msg to be DisembarkMsg from 'd' key")
	}

	// Test 'D' key
	_, cmdUpperD := m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'D'}})
	if cmdUpperD == nil {
		t.Fatal("expected DisembarkMsg command on 'D' key")
	}
	if _, ok := cmdUpperD().(DisembarkMsg); !ok {
		t.Errorf("expected msg to be DisembarkMsg from 'D' key")
	}
}

func TestDrydockModal_View(t *testing.T) {
	tour := engine.NewTour(409)
	tour.InDrydock = true
	tour.SectorsCompleted = 2
	tour.RequisitionPoints = 1250
	tour.InstalledRefits[engine.RefitDeflectorGrid] = 1

	m := New(tour, theme.GetTheme(theme.ThemeModern))
	view := m.View()

	if !strings.Contains(view, "STARBASE 01 DRYDOCK & REFIT FACILITY") {
		t.Error("view missing title")
	}
	if !strings.Contains(view, "Sector 2 Cleared") {
		t.Error("view missing sector cleared indicator")
	}
	if !strings.Contains(view, "1250 PTS") {
		t.Error("view missing requisition points")
	}
	if !strings.Contains(view, "NCC - 1701") {
		t.Error("view missing ship schematic")
	}
	if !strings.Contains(view, "Modules Installed: 1/6") {
		t.Error("view missing installed modules count")
	}
	if !strings.Contains(view, "Dilithium Core Tuning") {
		t.Error("view missing catalog item name")
	}
	if !strings.Contains(view, "> Dilithium Core Tuning") {
		t.Errorf("expected selected item cursor on Dilithium Core, got:\n%s", view)
	}
	if !strings.Contains(view, "Reinforced Deflectors") {
		t.Error("view missing Reinforced Deflectors item")
	}
	if !strings.Contains(view, "[Tier 1/3]") {
		t.Error("view missing tier indicator for installed refit")
	}
	if !strings.Contains(view, "Disembark") || !strings.Contains(view, "Next Sector") {
		t.Error("view missing footer prompt")
	}
}
