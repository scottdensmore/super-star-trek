package targetlock

import (
	"math"
	"strings"
	"testing"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

func TestTargetLock_BallisticsMath(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th)

	// Known sector geometry: Enterprise at [4, 4], Klingon at [4, 7]
	// Due East: distance = 3.0, bearing = 1.0, hit prob = 1.0 - 3.0/15.0 = 0.80
	entSector := engine.Coord{4, 4}
	klingonE := &engine.Klingon{ID: 1, Sector: engine.Coord{4, 7}, Energy: 400}

	m.SetState(entSector, 5000, 10, []*engine.Klingon{klingonE}, engine.Coord{})

	target := m.CurrentTarget()
	if target == nil {
		t.Fatalf("expected active target, got nil")
	}

	if math.Abs(target.Distance-3.0) > 1e-4 {
		t.Errorf("expected distance 3.0, got %f", target.Distance)
	}
	if math.Abs(target.Bearing-1.0) > 1e-4 {
		t.Errorf("expected bearing 1.0 (East), got %f", target.Bearing)
	}
	if math.Abs(target.HitProbability-0.80) > 1e-4 {
		t.Errorf("expected hit probability 0.80, got %f", target.HitProbability)
	}

	// Additional cardinal and intercardinal bearings
	tests := []struct {
		name            string
		klingonPos      engine.Coord
		expectedDist    float64
		expectedBearing float64
	}{
		{"Due North", engine.Coord{1, 4}, 3.0, 3.0},
		{"Due West", engine.Coord{4, 1}, 3.0, 5.0},
		{"Due South", engine.Coord{7, 4}, 3.0, 7.0},
		{"North-East", engine.Coord{1, 7}, math.Hypot(3, 3), 2.0},
		{"North-West", engine.Coord{1, 1}, math.Hypot(3, 3), 4.0},
		{"South-West", engine.Coord{7, 1}, math.Hypot(3, 3), 6.0},
		{"South-East", engine.Coord{7, 7}, math.Hypot(3, 3), 8.0},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			k := &engine.Klingon{ID: 2, Sector: tc.klingonPos, Energy: 400}
			m.SetState(entSector, 5000, 10, []*engine.Klingon{k}, engine.Coord{})
			cur := m.CurrentTarget()
			if cur == nil {
				t.Fatalf("expected target for %s, got nil", tc.name)
			}
			if math.Abs(cur.Distance-tc.expectedDist) > 1e-4 {
				t.Errorf("%s: expected dist %f, got %f", tc.name, tc.expectedDist, cur.Distance)
			}
			if math.Abs(cur.Bearing-tc.expectedBearing) > 1e-4 {
				t.Errorf("%s: expected bearing %f, got %f", tc.name, tc.expectedBearing, cur.Bearing)
			}
		})
	}

	// Test hit probability clamping bounds: 0.10 <= P <= 0.95
	// Distance 0 -> 1.0 - 0 = 1.0 clamped to 0.95
	kZero := &engine.Klingon{ID: 3, Sector: entSector, Energy: 400}
	m.SetState(entSector, 5000, 10, []*engine.Klingon{kZero}, engine.Coord{})
	if cur := m.CurrentTarget(); cur != nil {
		if cur.HitProbability != 0.95 {
			t.Errorf("expected max clamped hit probability 0.95, got %f", cur.HitProbability)
		}
	}

	// Distance large (e.g. Enterprise [1,1] to [8,8] dist ~9.9 -> 1.0 - 9.9/15 = 0.34, or simulated far distance)
	// Even at sector distance 15+, hit prob should clamp to 0.10
	kFar := &engine.Klingon{ID: 4, Sector: engine.Coord{16, 16}, Energy: 400}
	m.SetState(engine.Coord{1, 1}, 5000, 10, []*engine.Klingon{kFar}, engine.Coord{})
	if cur := m.CurrentTarget(); cur != nil {
		if cur.HitProbability != 0.10 {
			t.Errorf("expected min clamped hit probability 0.10, got %f", cur.HitProbability)
		}
	}
}

func TestTargetLock_TargetCycling(t *testing.T) {
	m := New(theme.DefaultTheme())
	entSector := engine.Coord{4, 4}

	// 3 Klingons at varying distances:
	// K1: [4, 6] -> dist 2.0
	// K2: [4, 7] -> dist 3.0
	// K3: [1, 1] -> dist ~4.24
	k1 := &engine.Klingon{ID: 10, Sector: engine.Coord{4, 6}, Energy: 300}
	k2 := &engine.Klingon{ID: 20, Sector: engine.Coord{4, 7}, Energy: 400}
	k3 := &engine.Klingon{ID: 30, Sector: engine.Coord{1, 1}, Energy: 500}

	// Pass unsorted order: k3, k1, k2
	m.SetState(entSector, 5000, 10, []*engine.Klingon{k3, k1, k2}, engine.Coord{})

	targets := m.Targets()
	if len(targets) != 3 {
		t.Fatalf("expected 3 targets, got %d", len(targets))
	}

	// Verify targets are sorted by distance ascending: K1 (2.0), K2 (3.0), K3 (~4.24)
	if targets[0].KlingonID != 10 || targets[1].KlingonID != 20 || targets[2].KlingonID != 30 {
		t.Fatalf("expected targets sorted by distance (10, 20, 30), got IDs: %d, %d, %d",
			targets[0].KlingonID, targets[1].KlingonID, targets[2].KlingonID)
	}

	// Default active target is closest: index 0 (K1)
	if cur := m.CurrentTarget(); cur == nil || cur.KlingonID != 10 {
		t.Fatalf("expected initial current target K1 (10), got %v", cur)
	}

	// Cycle forward with Tab
	var cmd tea.Cmd
	m, cmd = m.Update(tea.KeyMsg{Type: tea.KeyTab})
	if cmd != nil {
		t.Errorf("expected nil cmd on Tab, got %v", cmd)
	}
	if cur := m.CurrentTarget(); cur == nil || cur.KlingonID != 20 {
		t.Errorf("after Tab: expected target K2 (20), got %v", cur)
	}

	// Cycle forward with Right arrow
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRight})
	if cur := m.CurrentTarget(); cur == nil || cur.KlingonID != 30 {
		t.Errorf("after Right: expected target K3 (30), got %v", cur)
	}

	// Wrap around forward with Tab -> should be K1 (10)
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyTab})
	if cur := m.CurrentTarget(); cur == nil || cur.KlingonID != 10 {
		t.Errorf("after wrap Tab: expected target K1 (10), got %v", cur)
	}

	// Cycle backward with Left arrow -> should be K3 (30)
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyLeft})
	if cur := m.CurrentTarget(); cur == nil || cur.KlingonID != 30 {
		t.Errorf("after Left: expected target K3 (30), got %v", cur)
	}

	// Cycle backward with Shift+Tab -> should be K2 (20)
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyShiftTab})
	if cur := m.CurrentTarget(); cur == nil || cur.KlingonID != 20 {
		t.Errorf("after Shift+Tab: expected target K2 (20), got %v", cur)
	}

	// Test initialTarget selection: pass initialTarget = K2's sector [4, 7]
	m.SetState(entSector, 5000, 10, []*engine.Klingon{k1, k2, k3}, engine.Coord{4, 7})
	if cur := m.CurrentTarget(); cur == nil || cur.KlingonID != 20 {
		t.Errorf("expected initialTarget to select K2 (20), got %v", cur)
	}
}

func TestTargetLock_FireTorpedo(t *testing.T) {
	m := New(theme.DefaultTheme())
	entSector := engine.Coord{4, 4}
	klingon := &engine.Klingon{ID: 1, Sector: engine.Coord{4, 7}, Energy: 400}

	m.SetState(entSector, 5000, 8, []*engine.Klingon{klingon}, engine.Coord{})

	// Press Enter to fire torpedo
	m, cmd := m.Update(tea.KeyMsg{Type: tea.KeyEnter})
	if cmd == nil {
		t.Fatalf("expected non-nil cmd when firing torpedo with torpedoes available")
	}

	msg := cmd()
	fireMsg, ok := msg.(FireTorpedoMsg)
	if !ok {
		t.Fatalf("expected FireTorpedoMsg, got %T: %v", msg, msg)
	}

	if fireMsg.Target != (engine.Coord{4, 7}) {
		t.Errorf("expected target [4, 7], got %v", fireMsg.Target)
	}
	if math.Abs(fireMsg.Bearing-1.0) > 1e-4 {
		t.Errorf("expected bearing 1.0, got %f", fireMsg.Bearing)
	}
	if m.WarningMessage() != "" {
		t.Errorf("expected no warning message, got %q", m.WarningMessage())
	}
}

func TestTargetLock_ZeroTorpedoesWarning(t *testing.T) {
	m := New(theme.DefaultTheme())
	entSector := engine.Coord{4, 4}
	klingon := &engine.Klingon{ID: 1, Sector: engine.Coord{4, 7}, Energy: 400}

	// 0 torpedoes remaining
	m.SetState(entSector, 5000, 0, []*engine.Klingon{klingon}, engine.Coord{})

	m, cmd := m.Update(tea.KeyMsg{Type: tea.KeyEnter})
	if cmd != nil {
		t.Errorf("expected nil cmd when 0 torpedoes remain, got %v", cmd)
	}

	warning := m.WarningMessage()
	if !strings.Contains(warning, "NO TORPEDOES") {
		t.Errorf("expected warning containing 'NO TORPEDOES', got %q", warning)
	}

	// View should also display the warning
	view := m.View()
	if !strings.Contains(view, "NO TORPEDOES") {
		t.Errorf("expected View to contain warning, got:\n%s", view)
	}
}

func TestTargetLock_FirePhasersPrompt(t *testing.T) {
	m := New(theme.DefaultTheme())
	entSector := engine.Coord{4, 4}
	klingon := &engine.Klingon{ID: 1, Sector: engine.Coord{4, 7}, Energy: 400}

	m.SetState(entSector, 5000, 10, []*engine.Klingon{klingon}, engine.Coord{})

	// Initially not inputting phasers
	if m.InputtingPhaser() {
		t.Fatalf("expected InputtingPhaser to be false initially")
	}

	// Press 'P' to activate phaser prompt
	m, cmd := m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'p'}})
	if cmd != nil {
		t.Errorf("expected nil cmd when toggling phaser prompt, got %v", cmd)
	}
	if !m.InputtingPhaser() {
		t.Fatalf("expected InputtingPhaser to be true after pressing 'p'")
	}

	// View should reflect phaser prompt
	view := m.View()
	if !strings.Contains(view, "PHASER") {
		t.Errorf("expected View to show phaser prompt, got:\n%s", view)
	}

	// Type digits: '3', '5', '0'
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'3'}})
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'5'}})
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'0'}})

	// Press Enter to fire phasers
	m, cmd = m.Update(tea.KeyMsg{Type: tea.KeyEnter})
	if cmd == nil {
		t.Fatalf("expected non-nil cmd when firing phasers with valid amount")
	}

	msg := cmd()
	phaserMsg, ok := msg.(FirePhasersMsg)
	if !ok {
		t.Fatalf("expected FirePhasersMsg, got %T: %v", msg, msg)
	}
	if phaserMsg.Energy != 350.0 {
		t.Errorf("expected energy 350.0, got %f", phaserMsg.Energy)
	}
	if m.InputtingPhaser() {
		t.Errorf("expected InputtingPhaser to be false after firing")
	}

	// Test Backspace in phaser input
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'P'}})
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'5'}})
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'0'}})
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyBackspace})
	m, cmd = m.Update(tea.KeyMsg{Type: tea.KeyEnter})
	if cmd == nil {
		t.Fatalf("expected non-nil cmd after backspace and enter")
	}
	msg = cmd()
	if pm, ok := msg.(FirePhasersMsg); !ok || pm.Energy != 5.0 {
		t.Errorf("expected energy 5.0 after backspace, got %v", msg)
	}

	// Test toggle off with 'P'
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'p'}})
	if !m.InputtingPhaser() {
		t.Fatalf("expected InputtingPhaser true")
	}
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'p'}})
	if m.InputtingPhaser() {
		t.Fatalf("expected InputtingPhaser false after second 'p'")
	}
}

func TestTargetLock_PhaserWarningClearedOnTyping(t *testing.T) {
	m := New(theme.DefaultTheme())
	entSector := engine.Coord{4, 4}
	klingon := &engine.Klingon{ID: 1, Sector: engine.Coord{4, 7}, Energy: 400}

	m.SetState(entSector, 5000, 10, []*engine.Klingon{klingon}, engine.Coord{})

	// Activate phaser mode
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'p'}})

	// Press Enter with empty input -> invalid amount warning
	m, cmd := m.Update(tea.KeyMsg{Type: tea.KeyEnter})
	if cmd != nil {
		t.Errorf("expected nil cmd on invalid input")
	}
	if !strings.Contains(m.WarningMessage(), "INVALID ENERGY") {
		t.Fatalf("expected warning message to contain 'INVALID ENERGY', got %q", m.WarningMessage())
	}
	if !strings.Contains(m.View(), "INVALID ENERGY") {
		t.Errorf("expected View to show warning message")
	}

	// Typing a digit must clear the warning and display the input buffer
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'2'}})
	if m.WarningMessage() != "" {
		t.Errorf("expected warning to be cleared after typing digit, got %q", m.WarningMessage())
	}
	view := m.View()
	if strings.Contains(view, "INVALID ENERGY") {
		t.Errorf("expected View to no longer show warning message")
	}
	if !strings.Contains(view, "PHASER ENERGY> 2") {
		t.Errorf("expected View to show input buffer 'PHASER ENERGY> 2', got:\n%s", view)
	}

	// Trigger warning again with invalid input (e.g. empty)
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyBackspace}) // removes '2'
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyEnter})
	if m.WarningMessage() == "" {
		t.Fatalf("expected warning message")
	}

	// Backspace must also clear warning
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyBackspace})
	if m.WarningMessage() != "" {
		t.Errorf("expected warning to be cleared on backspace, got %q", m.WarningMessage())
	}
}

func TestTargetLock_Close(t *testing.T) {
	m := New(theme.DefaultTheme())
	m.SetState(engine.Coord{4, 4}, 5000, 10, nil, engine.Coord{})

	m, cmd := m.Update(tea.KeyMsg{Type: tea.KeyEsc})
	if cmd == nil {
		t.Fatalf("expected non-nil cmd on Esc")
	}

	msg := cmd()
	if _, ok := msg.(CloseHUDMsg); !ok {
		t.Fatalf("expected CloseHUDMsg, got %T: %v", msg, msg)
	}
}

func TestTargetLock_ViewDimensionsAndLayout(t *testing.T) {
	m := New(theme.DefaultTheme())
	entSector := engine.Coord{4, 4}
	klingon := &engine.Klingon{ID: 1, Sector: engine.Coord{3, 6}, Energy: 280}

	m.SetState(entSector, 3450, 8, []*engine.Klingon{klingon}, engine.Coord{})

	view := m.View()
	// Check required text elements in View
	expectedElements := []string{
		"TACTICAL TARGET LOCK",
		"KLINGON",
		"Sector [3, 6]",
		"Range:",
		"Bearing:",
		"Hit Prob",
		"[TORP: 8/10]",
		"[ENERGY: 3450]",
		"[Enter]",
		"[P]",
		"[Tab]",
		"[Esc]",
	}

	for _, elem := range expectedElements {
		if !strings.Contains(view, elem) {
			t.Errorf("expected View to contain %q, view was:\n%s", elem, view)
		}
	}

	// Check dimensions: 52 columns wide x 12 rows tall
	lines := strings.Split(strings.TrimRight(view, "\n"), "\n")
	if len(lines) != 12 {
		t.Errorf("expected View height 12 rows, got %d rows:\n%s", len(lines), view)
	}
	for i, line := range lines {
		w := lipgloss.Width(line)
		if w != 52 {
			t.Errorf("expected line %d width 52, got %d: %q", i, w, line)
		}
	}
}

func TestTargetLock_NoTargets(t *testing.T) {
	m := New(theme.DefaultTheme())
	m.SetState(engine.Coord{4, 4}, 5000, 10, nil, engine.Coord{})

	if cur := m.CurrentTarget(); cur != nil {
		t.Fatalf("expected nil current target with no Klingons, got %v", cur)
	}

	view := m.View()
	if !strings.Contains(view, "NO HOSTILE") {
		t.Errorf("expected view to indicate no hostile targets, got:\n%s", view)
	}

	// Pressing Enter when no target should not panic or emit fire message
	m, cmd := m.Update(tea.KeyMsg{Type: tea.KeyEnter})
	if cmd != nil {
		t.Errorf("expected nil cmd when no targets exist, got %v", cmd)
	}

	// Esc should still close
	m, cmd = m.Update(tea.KeyMsg{Type: tea.KeyEsc})
	if cmd == nil {
		t.Fatalf("expected non-nil cmd on Esc")
	}
	if _, ok := cmd().(CloseHUDMsg); !ok {
		t.Errorf("expected CloseHUDMsg")
	}
}

func TestTargetLock_SetTheme(t *testing.T) {
	m := New(theme.DefaultTheme())
	m.SetTheme(theme.GetTheme("lcars"))
	v1 := m.View()
	m.SetTheme(theme.GetTheme("crt"))
	v2 := m.View()

	if v1 == "" || v2 == "" {
		t.Errorf("expected non-empty views across themes")
	}
}

