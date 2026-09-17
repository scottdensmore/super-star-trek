package damageschematic

import (
	"strings"
	"testing"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/x/ansi"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

func TestSchematic_NominalDimensionsAndContent(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th, 66, 18)

	var ent engine.EnterpriseState
	ent.Energy = 5000
	ent.Shields = 2500

	m.SetState(ent, "GREEN", false, 1.0)
	view := m.View()

	lines := strings.Split(view, "\n")
	if len(lines) != 18 {
		t.Fatalf("expected 18 lines, got %d", len(lines))
	}

	for i, line := range lines {
		w := ansi.StringWidth(line)
		if w != 66 {
			t.Errorf("line %d width = %d, expected 66 (content: %q)", i, w, line)
		}
	}

	if !strings.Contains(view, "DAMAGE CONTROL SCHEMATIC") {
		t.Errorf("expected header title in view")
	}
	if !strings.Contains(view, "NCC-1701") {
		t.Errorf("expected ship registry in view")
	}
	if !strings.Contains(view, "SRS: OK") {
		t.Errorf("expected nominal SRS status")
	}
	if !strings.Contains(view, "All other primary and tactical systems operational.") {
		t.Errorf("expected nominal summary message when no devices damaged")
	}
}

func TestSchematic_DamagedSubsystems(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th, 66, 18)

	var ent engine.EnterpriseState
	ent.Devices[engine.DeviceComputer] = 2.1
	ent.Devices[engine.DevicePhotonTubes] = 1.4

	m.SetState(ent, "YELLOW", false, 1.0)
	view := m.View()

	lines := strings.Split(view, "\n")
	if len(lines) != 18 {
		t.Fatalf("expected 18 lines, got %d", len(lines))
	}

	if !strings.Contains(view, "COMP: 2.1") {
		t.Errorf("expected damaged computer countdown in wireframe pin")
	}
	if !strings.Contains(view, "TUB: 1.4") {
		t.Errorf("expected damaged photon tubes countdown in wireframe pin")
	}
	if !strings.Contains(view, "Library Computer") || !strings.Contains(view, "No Chart/Nav") {
		t.Errorf("expected library computer tactical impact breakdown")
	}
	if !strings.Contains(view, "Photon Tubes") || !strings.Contains(view, "Tubes Locked") {
		t.Errorf("expected photon tubes tactical impact breakdown")
	}
}

func TestSchematic_RepairMultiplierAndDocked(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th, 66, 18)

	var ent engine.EnterpriseState
	ent.Devices[engine.DeviceWarp] = 2.0

	// 1.5x repair multiplier
	m.SetState(ent, "RED", false, 1.5)
	view := m.View()

	// In-flight: 2.0 * 1.5 = 3.0 SD; Docked: 3.0 * 0.25 = 0.8 SD
	if !strings.Contains(view, "3.0 SD") {
		t.Errorf("expected scaled in-flight repair time 3.0 SD")
	}
	if !strings.Contains(view, "0.8 SD") {
		t.Errorf("expected scaled docked repair time 0.8 SD")
	}
}

func TestSchematic_SetTheme(t *testing.T) {
	m := New(nil, 0, 0)
	m.SetTheme(theme.GetTheme("lcars"))
	view := m.View()
	lines := strings.Split(view, "\n")
	if len(lines) != 18 {
		t.Fatalf("expected 18 lines, got %d", len(lines))
	}
	for i, line := range lines {
		w := ansi.StringWidth(line)
		if w != 66 {
			t.Errorf("line %d width = %d, expected 66", i, w)
		}
	}
}

func TestSchematic_DockedAndOverflowDevices(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th, 66, 18)

	var ent engine.EnterpriseState
	ent.Devices[engine.DeviceWarp] = 1.0
	ent.Devices[engine.DeviceSRSensors] = 1.0
	ent.Devices[engine.DeviceLRSensors] = 1.0
	ent.Devices[engine.DevicePhasers] = 1.0
	ent.Devices[engine.DevicePhotonTubes] = 1.0

	m.SetState(ent, "GREEN", true, 1.0)
	view := m.View()

	if !strings.Contains(view, "ALERT STATUS: DOCKED") {
		t.Errorf("expected DOCKED alert status")
	}
	if !strings.Contains(view, "+2 more damaged subsystems") {
		t.Errorf("expected overflow indicator for 5 damaged subsystems (+2 more)")
	}
}

func TestSchematic_UpdateDismissKeys(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th, 66, 18)

	dismissKeys := []string{"esc", "enter", "q", "Q", "d", "D", " ", "space"}
	for _, k := range dismissKeys {
		var keyMsg tea.KeyMsg
		switch k {
		case "esc":
			keyMsg = tea.KeyMsg{Type: tea.KeyEsc}
		case "enter":
			keyMsg = tea.KeyMsg{Type: tea.KeyEnter}
		case "space":
			keyMsg = tea.KeyMsg{Type: tea.KeySpace}
		case " ":
			keyMsg = tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune(" ")}
		default:
			keyMsg = tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune(k)}
		}

		updated, cmd := m.Update(keyMsg)
		_ = updated
		if cmd == nil {
			t.Errorf("expected CloseModalMsg command on key %q, got nil", k)
			continue
		}
		msg := cmd()
		if _, ok := msg.(CloseModalMsg); !ok {
			t.Errorf("expected CloseModalMsg on key %q, got %T", k, msg)
		}
	}

	// Non-dismissal key should not emit CloseModalMsg
	otherKey := tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune("x")}
	_, cmd := m.Update(otherKey)
	if cmd != nil {
		t.Errorf("expected nil cmd on non-dismiss key 'x', got %v", cmd())
	}

	// Non-key message should not emit CloseModalMsg
	otherMsg := struct{}{}
	_, cmd = m.Update(otherMsg)
	if cmd != nil {
		t.Errorf("expected nil cmd on non-key message, got %v", cmd())
	}
}

func TestSchematic_DamageControlSubsystemImpact(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th, 66, 18)

	var ent engine.EnterpriseState
	ent.Devices[engine.DeviceDamageControl] = 1.0

	m.SetState(ent, "GREEN", false, 1.0)
	view := m.View()

	if !strings.Contains(view, "Repairs Slow") {
		t.Errorf("expected 'Repairs Slow' impact description for Damage Control, got:\n%s", view)
	}

	// Verify all rows still exactly 66 chars wide
	lines := strings.Split(view, "\n")
	for i, line := range lines {
		w := ansi.StringWidth(line)
		if w != 66 {
			t.Errorf("line %d width = %d, expected 66 (content: %q)", i, w, line)
		}
	}
}
