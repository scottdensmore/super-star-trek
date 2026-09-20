package tui

import (
	"fmt"
	"strings"
	"testing"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/commandbar"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/scenariomodal"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

// sendTestCommand simulates a user typing a command into the command bar and pressing Enter.
func sendTestCommand(m Model, cmdText string) (Model, tea.Cmd) {
	updated, cmd := m.Update(commandbar.CommandSubmittedMsg{Text: cmdText})
	return updated.(Model), cmd
}

// TestTUIJourney_Moving tests both sub-light sector maneuvering and inter-quadrant warp jumps.
func TestTUIJourney_Moving(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	m := NewModel(g, theme.DefaultTheme())
	updated, _ := m.Update(tea.WindowSizeMsg{Width: 120, Height: 30})
	m = updated.(Model)

	startQuad := g.Enterprise.Quad
	startSector := g.Enterprise.Sector
	initialEnergy := g.Enterprise.Energy

	// 1. Sector navigation to an empty adjacent sector
	targetSector := engine.Coord{startSector[0] + 1, startSector[1]}
	if targetSector[0] > 8 {
		targetSector[0] = startSector[0] - 1
	}
	// Ensure target sector is empty on grid
	g.CurrentQuad.Grid[targetSector[0]][targetSector[1]] = engine.EntityEmpty

	m, _ = sendTestCommand(m, fmt.Sprintf("nav s %d %d", targetSector[0], targetSector[1]))

	if g.Enterprise.Sector != targetSector {
		t.Fatalf("expected Enterprise sector to be %v, got %v", targetSector, g.Enterprise.Sector)
	}
	if g.Enterprise.Energy >= initialEnergy {
		t.Errorf("expected energy consumption after sector move, energy = %.0f", g.Enterprise.Energy)
	}
	if g.CurrentQuad.Grid[targetSector[0]][targetSector[1]] != engine.EntityEnterprise {
		t.Errorf("expected grid at %v to contain EntityEnterprise", targetSector)
	}

	// 2. Inter-quadrant warp jump
	destQuad := engine.Coord{startQuad[0] + 1, startQuad[1]}
	if destQuad[0] > 8 {
		destQuad[0] = startQuad[0] - 1
	}
	prevStardate := g.Stardate

	m, _ = sendTestCommand(m, fmt.Sprintf("nav q %d %d", destQuad[0], destQuad[1]))

	if g.Enterprise.Quad != destQuad {
		t.Fatalf("expected Enterprise quadrant to be %v, got %v", destQuad, g.Enterprise.Quad)
	}
	if g.Stardate <= prevStardate {
		t.Errorf("expected stardate to advance after warp, prev=%.1f curr=%.1f", prevStardate, g.Stardate)
	}
	if !g.ChartDiscovered[destQuad[0]][destQuad[1]] {
		t.Errorf("expected new quadrant %v to be marked discovered in galaxy chart", destQuad)
	}

	// 3. Verify View reflects the updated quadrant and sector
	view := m.View()
	if !strings.Contains(view, "<E>") {
		t.Errorf("expected view to contain Enterprise glyph <E>")
	}
	if !strings.Contains(view, fmt.Sprintf("[%d,%d]", destQuad[0], destQuad[1])) {
		t.Errorf("expected view to display current quadrant coordinates [%d,%d]", destQuad[0], destQuad[1])
	}
}

// TestTUIJourney_Fighting tests shield transfers, phaser firing, torpedo targeting,
// Klingon counter-attacks, and quadrant clearance.
func TestTUIJourney_Fighting(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	m := NewModel(g, theme.DefaultTheme())
	updated, _ := m.Update(tea.WindowSizeMsg{Width: 120, Height: 30})
	m = updated.(Model)

	// Ensure Enterprise is at [4, 4] with full reserves and 0 initial shields
	g.Enterprise.Sector = engine.Coord{4, 4}
	g.Enterprise.Energy = 5000
	g.Enterprise.Shields = 0
	g.Enterprise.Torpedoes = 10
	g.Enterprise.Condition = engine.ConditionRed

	// Set up two Klingons: one weak at [4, 7] and one at [6, 2]
	k1 := &engine.Klingon{ID: 1, Sector: engine.Coord{4, 7}, Energy: 200}
	k2 := &engine.Klingon{ID: 2, Sector: engine.Coord{6, 2}, Energy: 350}
	g.CurrentQuad.Klingons = []*engine.Klingon{k1, k2}
	g.CurrentQuad.Grid[4][7] = engine.EntityKlingon
	g.CurrentQuad.Grid[6][2] = engine.EntityKlingon

	// 1. Raise Shields
	m, _ = sendTestCommand(m, "she 2000")
	if g.Enterprise.Energy > 3000 {
		t.Errorf("expected energy to decrease by 2000 for shields, got energy=%.0f", g.Enterprise.Energy)
	}
	// Surviving Klingons returned fire; shields absorbed the hit
	if g.Enterprise.Shields <= 0 || g.Enterprise.Shields >= 2000 {
		t.Errorf("expected shields between 0 and 2000 after absorbing counter-attack, got %.0f", g.Enterprise.Shields)
	}

	// 2. Fire Phasers
	prevK1Energy := k1.Energy
	m, _ = sendTestCommand(m, "pha 400")
	if k1.Energy >= prevK1Energy && k2.Energy >= 350 {
		t.Errorf("expected phasers to damage at least one Klingon vessel")
	}

	// 3. Fire Torpedo directly at Klingon 1
	k1Sector := engine.Coord{4, 7}
	m, _ = sendTestCommand(m, fmt.Sprintf("tor %d %d", k1Sector[0], k1Sector[1]))

	if g.Enterprise.Torpedoes != 9 {
		t.Errorf("expected torpedoes to decrease to 9, got %d", g.Enterprise.Torpedoes)
	}
	if g.CurrentQuad.Grid[k1Sector[0]][k1Sector[1]] == engine.EntityKlingon {
		t.Errorf("expected Klingon at %v to be destroyed and cleared from grid", k1Sector)
	}

	// 4. Destroy remaining Klingon 2 with torpedo
	k2Sector := engine.Coord{6, 2}
	m, _ = sendTestCommand(m, fmt.Sprintf("tor %d %d", k2Sector[0], k2Sector[1]))
	if g.CurrentQuad.Grid[k2Sector[0]][k2Sector[1]] == engine.EntityKlingon {
		// If high damage didn't destroy on first hit, finish with second torpedo
		m, _ = sendTestCommand(m, fmt.Sprintf("tor %d %d", k2Sector[0], k2Sector[1]))
	}

	// Verify all Klingons in quadrant destroyed
	if len(g.CurrentQuad.Klingons) != 0 {
		t.Errorf("expected 0 surviving Klingons in current quadrant, got %d", len(g.CurrentQuad.Klingons))
	}
	if g.CurrentQuad.Grid[k1Sector[0]][k1Sector[1]] != engine.EntityEmpty || g.CurrentQuad.Grid[k2Sector[0]][k2Sector[1]] != engine.EntityEmpty {
		t.Errorf("expected destroyed Klingons cleared from grid")
	}
}

// TestTUIJourney_Docking tests navigation to a starbase, docking, replenishing energy and ammo,
// and repairing all damaged subsystems.
func TestTUIJourney_Docking(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	m := NewModel(g, theme.DefaultTheme())
	updated, _ := m.Update(tea.WindowSizeMsg{Width: 120, Height: 30})
	m = updated.(Model)

	// Set up Starbase at [3, 4] and place Enterprise adjacent at [3, 5]
	sbCoord := engine.Coord{3, 4}
	entCoord := engine.Coord{3, 5}
	g.CurrentQuad.Starbase = &sbCoord
	g.CurrentQuad.Grid[sbCoord[0]][sbCoord[1]] = engine.EntityStarbase
	g.CurrentQuad.Grid[entCoord[0]][entCoord[1]] = engine.EntityEnterprise
	g.Enterprise.Sector = entCoord

	// Deplete ship reserves and induce device damages
	g.Enterprise.Energy = 1450
	g.Enterprise.Torpedoes = 2
	g.Enterprise.Shields = 1000
	g.Enterprise.Devices[engine.DeviceWarp] = 4.2
	g.Enterprise.Devices[engine.DeviceComputer] = 1.8
	g.Enterprise.Devices[engine.DevicePhotonTubes] = 2.5

	// Execute docking command
	m, _ = sendTestCommand(m, "doc")

	if g.Enterprise.Condition != engine.ConditionDocked {
		t.Fatalf("expected ConditionDocked, got %v", g.Enterprise.Condition)
	}
	if g.Enterprise.Energy != 5000 {
		t.Errorf("expected energy replenished to 5000, got %.0f", g.Enterprise.Energy)
	}
	if g.Enterprise.Torpedoes != 10 {
		t.Errorf("expected torpedoes replenished to 10, got %d", g.Enterprise.Torpedoes)
	}

	// Verify all damaged devices fully repaired
	for dev := engine.DeviceID(0); dev < engine.NumDevices; dev++ {
		if g.Enterprise.Devices[dev] != 0 {
			t.Errorf("expected device %d to be fully repaired (0.0), got %.2f", dev, g.Enterprise.Devices[dev])
		}
	}

	// Verify docked status rendered in view
	view := m.View()
	if !strings.Contains(view, "DOCKED") {
		t.Errorf("expected view to display DOCKED condition")
	}
}

// TestTUIJourney_Scenario_KobayashiMaru tests the unwinnable scenario: neutral zone trap,
// wave reinforcement after eliminating enemies, and tactical commendation ranking.
func TestTUIJourney_Scenario_KobayashiMaru(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	m := NewModel(g, theme.DefaultTheme())
	updated, _ := m.Update(tea.WindowSizeMsg{Width: 120, Height: 30})
	m = updated.(Model)

	// Launch Kobayashi Maru scenario
	updated, _ = m.Update(scenariomodal.MsgLaunchScenario{ScenarioID: engine.ScenarioKobayashiMaru})
	m = updated.(Model)

	if m.Game == nil || m.Game.Scenario != engine.ScenarioKobayashiMaru {
		t.Fatalf("expected scenario to be KobayashiMaru, got %v", m.Game.Scenario)
	}
	if m.Game.Enterprise.Quad != (engine.Coord{4, 4}) {
		t.Errorf("expected starting quad [4,4], got %v", m.Game.Enterprise.Quad)
	}
	if len(m.Game.CurrentQuad.Klingons) != 3 {
		t.Fatalf("expected 3 initial Klingons in Kobayashi Maru, got %d", len(m.Game.CurrentQuad.Klingons))
	}

	// Destroy the first Klingon with a torpedo
	targetK := m.Game.CurrentQuad.Klingons[0]
	targetSec := targetK.Sector
	targetK.Energy = 10 // ensure one-shot destruction

	m, _ = sendTestCommand(m, fmt.Sprintf("tor %d %d", targetSec[0], targetSec[1]))
	if m.Game.Metrics.KlingonsKilled < 1 {
		m, _ = sendTestCommand(m, "pha 500")
	}

	if m.Game.Metrics.KlingonsKilled < 1 {
		t.Fatalf("expected at least 1 Klingon killed, got %d", m.Game.Metrics.KlingonsKilled)
	}

	// Wave reinforcement check: in quadrant [4,4], Klingons reinforce back to 3!
	if len(m.Game.CurrentQuad.Klingons) != 3 {
		t.Errorf("expected wave reinforcement to restore fleet to 3 Klingons, got %d", len(m.Game.CurrentQuad.Klingons))
	}

	// Test defeat and commendation score calculation
	m.Game.Enterprise.Energy = 0
	done, won, reason := engine.EvaluateKobayashiMaru(m.Game)
	if !done || won || reason != engine.GameOverLost {
		t.Errorf("expected defeat in Kobayashi Maru when energy depleted")
	}

	score := engine.ComputeScoreKobayashiMaru(m.Game, false)
	if !strings.Contains(score.RankTitle, "Commendation") && !strings.Contains(score.RankTitle, "Citation") {
		t.Errorf("expected Starfleet Tactical Commendation title, got %q", score.RankTitle)
	}
}

// TestTUIJourney_Scenario_MutaraNebula tests the nebula duel: shield ionization blackout,
// engaging the cloaked commander, and mission victory or abandonment.
func TestTUIJourney_Scenario_MutaraNebula(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	m := NewModel(g, theme.DefaultTheme())
	updated, _ := m.Update(tea.WindowSizeMsg{Width: 120, Height: 30})
	m = updated.(Model)

	// Launch Mutara Nebula scenario
	updated, _ = m.Update(scenariomodal.MsgLaunchScenario{ScenarioID: engine.ScenarioMutaraNebula})
	m = updated.(Model)

	if m.Game == nil || m.Game.Scenario != engine.ScenarioMutaraNebula {
		t.Fatalf("expected scenario to be MutaraNebula, got %v", m.Game.Scenario)
	}
	if m.Game.QuadrantEnv[5][5] != engine.EnvNebula {
		t.Errorf("expected quadrant [5,5] environment to be EnvNebula")
	}
	if m.Game.Enterprise.Shields != 0 {
		t.Errorf("expected 0 shields inside Mutara Nebula, got %.0f", m.Game.Enterprise.Shields)
	}

	// Attempting to raise shields in nebula should fail or drop back to 0
	m, _ = sendTestCommand(m, "she 1000")
	if m.Game.Enterprise.Shields != 0 {
		t.Errorf("expected shields to remain 0 in Mutara Nebula, got %.0f", m.Game.Enterprise.Shields)
	}

	// Locate the cloaked Super-Commander and destroy it
	if len(m.Game.CurrentQuad.Klingons) == 0 {
		t.Fatalf("expected Super-Commander in Mutara Nebula")
	}
	cmdKlingon := m.Game.CurrentQuad.Klingons[0]
	cmdSec := cmdKlingon.Sector
	cmdKlingon.Energy = 10 // ensure one-shot destruction

	m, _ = sendTestCommand(m, fmt.Sprintf("tor %d %d", cmdSec[0], cmdSec[1]))
	if len(m.Game.CurrentQuad.Klingons) > 0 {
		m, _ = sendTestCommand(m, "pha 500")
	}

	// Evaluate victory condition: destroying the commander wins the scenario!
	done, won, reason := engine.EvaluateMutaraNebula(m.Game)
	if !done || !won || reason != engine.GameOverWon {
		t.Fatalf("expected victory upon destroying commander in Mutara Nebula, done=%v won=%v reason=%v", done, won, reason)
	}
}

// TestTUIJourney_Scenario_StarbaseSiege tests the siege defense: warping to quadrant [4,4],
// defending Starbase 12, resupplying, and destroying all 3 siege cruisers for victory.
func TestTUIJourney_Scenario_StarbaseSiege(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	m := NewModel(g, theme.DefaultTheme())
	updated, _ := m.Update(tea.WindowSizeMsg{Width: 120, Height: 30})
	m = updated.(Model)

	// Launch Starbase Siege scenario
	updated, _ = m.Update(scenariomodal.MsgLaunchScenario{ScenarioID: engine.ScenarioStarbaseSiege})
	m = updated.(Model)

	if m.Game == nil || m.Game.Scenario != engine.ScenarioStarbaseSiege {
		t.Fatalf("expected scenario to be StarbaseSiege, got %v", m.Game.Scenario)
	}
	if m.Game.Enterprise.Quad != (engine.Coord{2, 2}) {
		t.Errorf("expected starting quad [2,2], got %v", m.Game.Enterprise.Quad)
	}

	// 1. Warp to besieged Starbase quadrant [4, 4] at Warp 6
	m, _ = sendTestCommand(m, "nav q 4 4 6")
	if m.Game.Enterprise.Quad != (engine.Coord{4, 4}) {
		t.Fatalf("expected arrived in quad [4,4], got %v", m.Game.Enterprise.Quad)
	}
	if m.Game.CurrentQuad.Starbase == nil {
		t.Fatalf("expected Starbase 12 in quadrant [4,4]")
	}
	if len(m.Game.CurrentQuad.Klingons) != 3 {
		t.Fatalf("expected 3 siege Klingons in quadrant [4,4], got %d", len(m.Game.CurrentQuad.Klingons))
	}

	// 2. Eliminate besieging Klingon cruisers with phasers to prevent friendly fire on Starbase 12
	for _, k := range m.Game.CurrentQuad.Klingons {
		k.Energy = 10
	}
	m.Game.Enterprise.Energy = 3000
	m, _ = sendTestCommand(m, "pha 2000")

	// 3. Verify Starbase Siege victory
	done, won, reason := engine.EvaluateStarbaseSiege(m.Game)
	if !done || !won || reason != engine.GameOverWon {
		t.Fatalf("expected Starbase Siege victory, got done=%v won=%v reason=%v (RemSB=%d, SBHit=%d, Chart=%d, Time=%.2f, Energy=%.0f, Kills=%d, RemK=%d, QuadK=%d)",
			done, won, reason, m.Game.RemainingStarbases, m.Game.Metrics.StarbasesDestroyed, m.Game.GalaxyChart[4][4], m.Game.TimeRemaining, m.Game.Enterprise.Energy,
			m.Game.Metrics.KlingonsKilled+m.Game.Metrics.CommandersKilled+m.Game.Metrics.SuperCommandersKilled, m.Game.RemainingKlingons, len(m.Game.CurrentQuad.Klingons))
	}
}
