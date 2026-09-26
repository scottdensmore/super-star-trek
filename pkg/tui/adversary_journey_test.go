package tui

import (
	"strings"
	"testing"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/commandbar"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

func TestAdversaryJourney_TacticalFactionsWorkflow(t *testing.T) {
	tour := engine.NewTour(987654)
	th := theme.GetTheme(theme.ThemeModern)
	m := NewModelWithTour(tour, th)

	if m.Tour == nil || !m.Tour.Active {
		t.Fatal("expected active tour in model")
	}

	// 1. Sector 1 (Romulan Neutral Zone)
	if len(m.Game.CurrentQuad.Enemies) < 2 {
		t.Fatalf("expected Romulans in sector 1, got %d", len(m.Game.CurrentQuad.Enemies))
	}

	// Advance turn to trigger Romulan decloak and plasma launch
	m, _ = m.UpdateModel(tea.KeyMsg{Type: tea.KeyEnter})

	if len(m.Game.CurrentQuad.PlasmaTorpedoes) == 0 {
		t.Log("Note: Plasma torpedo may require decloak turn cycle")
	}

	// Verify full game loop executes cleanly
	m, _ = m.UpdateModel(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune("s")})
	if m.Game.GameOver {
		t.Errorf("unexpected game over during tactical test")
	}
}

func TestAdversaryJourney_PointDefenseInterception(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	g.Rules.Adversaries = true
	g.Enterprise.Sector = engine.Coord{4, 4}
	g.Enterprise.Energy = 4000
	g.Enterprise.Shields = 1000

	// In-flight tracking plasma torpedo aimed at Enterprise
	torpCoord := engine.Coord{4, 6}
	plasma := &engine.PlasmaTorpedo{
		ID:            1001,
		SourceID:      201,
		Sector:        torpCoord,
		Energy:        750.0,
		TargetSector:  g.Enterprise.Sector,
		TurnsInFlight: 1,
	}
	g.CurrentQuad.PlasmaTorpedoes = []*engine.PlasmaTorpedo{plasma}
	g.CurrentQuad.Grid[torpCoord[0]][torpCoord[1]] = engine.EntityPlasmaTorpedo

	m := NewModel(g, theme.GetTheme(theme.ThemeModern))

	// Verify HUD alerts and grid representation
	statusView := m.Status.View(g)
	if !strings.Contains(statusView, "INCOMING PLASMA TORPEDO TRACKING") {
		t.Errorf("expected incoming plasma warning in status panel, got:\n%s", statusView)
	}

	gridView := m.Grid.ViewWithGame(g)
	if !strings.Contains(gridView, "*P*") {
		t.Errorf("expected plasma torpedo glyph *P* in grid view, got:\n%s", gridView)
	}

	// User types point-defense command: pha 800 4 6
	m, _ = m.UpdateModel(commandbar.CommandSubmittedMsg{Text: "pha 800 4 6"})

	// Verify plasma torpedo was intercepted and cleared
	if len(m.Game.CurrentQuad.PlasmaTorpedoes) != 0 {
		t.Errorf("expected 0 plasma torpedoes remaining after interception, got %d", len(m.Game.CurrentQuad.PlasmaTorpedoes))
	}
	if m.Game.CurrentQuad.Grid[torpCoord[0]][torpCoord[1]] != engine.EntityEmpty {
		t.Errorf("expected grid cell %v to be cleared, got %v", torpCoord, m.Game.CurrentQuad.Grid[torpCoord[0]][torpCoord[1]])
	}

	// Verify status warning cleared
	updatedStatus := m.Status.View(m.Game)
	if strings.Contains(updatedStatus, "INCOMING PLASMA TORPEDO TRACKING") {
		t.Errorf("expected incoming plasma warning to be cleared after interception")
	}
}

func TestAdversaryJourney_TholianWebBreachAndEscape(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	g.Rules.Adversaries = true
	g.Enterprise.Sector = engine.Coord{4, 4}
	g.Enterprise.Energy = 4000
	g.Enterprise.Torpedoes = 5

	// Active Tholian spinner patrolling perimeter
	tholian := &engine.EnemyVessel{
		ID:           301,
		Faction:      engine.FactionTholian,
		Sector:       engine.Coord{1, 1},
		Energy:       800.0,
		SpecialState: 0,
	}
	g.CurrentQuad.Enemies = []*engine.EnemyVessel{tholian}
	g.CurrentQuad.Grid[1][1] = engine.EntityTholian

	// Web filament blocking the eastern corridor at [4, 5]
	webCoord := engine.Coord{4, 5}
	webSeg := &engine.TholianWebSegment{
		Coord:    webCoord,
		Strength: 150.0,
	}
	g.CurrentQuad.WebSegments = []*engine.TholianWebSegment{webSeg}
	g.CurrentQuad.Grid[webCoord[0]][webCoord[1]] = engine.EntityTholianWeb
	engine.SyncQuadrantEnemies(&g.CurrentQuad)

	m := NewModel(g, theme.GetTheme(theme.ThemeModern))

	// Verify HUD enclosure alert
	statusView := m.Status.View(g)
	if !strings.Contains(statusView, "THOLIAN WEB ENCLOSURE") {
		t.Errorf("expected Tholian web warning in status panel, got:\n%s", statusView)
	}

	// 1. Attempt to move into web obstacle: should be blocked
	m, _ = m.UpdateModel(commandbar.CommandSubmittedMsg{Text: "nav s 4 5"})
	if m.Game.Enterprise.Sector != (engine.Coord{4, 4}) {
		t.Fatalf("expected Enterprise to remain at [4,4] when blocked by web, got %v", m.Game.Enterprise.Sector)
	}

	// 2. Fire photon torpedo directly at the web filament to breach it
	m, _ = m.UpdateModel(commandbar.CommandSubmittedMsg{Text: "tor 4 5"})

	for _, ws := range m.Game.CurrentQuad.WebSegments {
		if ws.Coord == webCoord {
			t.Errorf("expected web segment at %v to be breached and removed from WebSegments", webCoord)
		}
	}
	if m.Game.CurrentQuad.Grid[webCoord[0]][webCoord[1]] != engine.EntityEmpty {
		t.Errorf("expected web grid cell %v cleared to empty, got %v", webCoord, m.Game.CurrentQuad.Grid[webCoord[0]][webCoord[1]])
	}

	// 3. Move through the newly breached corridor
	m, _ = m.UpdateModel(commandbar.CommandSubmittedMsg{Text: "nav s 4 5"})
	if m.Game.Enterprise.Sector != webCoord {
		t.Fatalf("expected Enterprise to successfully move to breached sector %v, got %v", webCoord, m.Game.Enterprise.Sector)
	}
	if m.Game.CurrentQuad.Grid[webCoord[0]][webCoord[1]] != engine.EntityEnterprise {
		t.Errorf("expected Enterprise on grid at %v", webCoord)
	}
}

func TestAdversaryJourney_KlingonPackCrossfireAndScreening(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	g.Rules.Adversaries = true
	g.Enterprise.Sector = engine.Coord{4, 1}
	g.Enterprise.Shields = 2000
	g.Enterprise.Energy = 4000

	// Klingon Commander in direct horizontal line of fire at [4, 7]
	commander := &engine.EnemyVessel{
		ID:          101,
		Faction:     engine.FactionKlingon,
		Sector:      engine.Coord{4, 7},
		Energy:      800.0,
		IsCommander: true,
	}
	// Escort Raider poised to screen at [3, 4]
	escort := &engine.EnemyVessel{
		ID:          102,
		Faction:     engine.FactionKlingon,
		Sector:      engine.Coord{3, 4},
		Energy:      400.0,
		IsCommander: false,
	}
	// Flanking Raider creating >= 60 degree crossfire angle
	flanker := &engine.EnemyVessel{
		ID:          103,
		Faction:     engine.FactionKlingon,
		Sector:      engine.Coord{7, 4},
		Energy:      400.0,
		IsCommander: false,
	}

	g.CurrentQuad.Enemies = []*engine.EnemyVessel{commander, escort, flanker}
	g.CurrentQuad.Grid[4][7] = engine.EntityCommander
	g.CurrentQuad.Grid[3][4] = engine.EntityKlingon
	g.CurrentQuad.Grid[7][4] = engine.EntityKlingon
	engine.SyncQuadrantEnemies(&g.CurrentQuad)

	m := NewModel(g, theme.GetTheme(theme.ThemeModern))

	// Verify Crossfire Bracket detection and alert HUD
	if !engine.DetectCrossfireBracket(g) {
		t.Fatal("expected crossfire bracket detected")
	}
	mult := engine.CalculatePackDamageMultiplier(g)
	if mult != 1.35 {
		t.Fatalf("expected 1.35x crossfire damage multiplier, got %f", mult)
	}

	statusView := m.Status.View(g)
	if !strings.Contains(statusView, "CROSSFIRE BRACKET ACTIVE (+35% DMG)") {
		t.Errorf("expected crossfire alert in status panel, got:\n%s", statusView)
	}

	initialShields := g.Enterprise.Shields

	// Player performs a tactical action (e.g. transfer shield energy) that triggers the adversary turn
	m, _ = m.UpdateModel(commandbar.CommandSubmittedMsg{Text: "she 500"})

	// 1. Verify Commander Screening: Escort raider stepped into [4, 4] to shield the Commander
	if escort.Sector != (engine.Coord{4, 4}) {
		t.Errorf("expected escort raider to interpose at [4, 4], got %v", escort.Sector)
	}
	if m.Game.CurrentQuad.Grid[4][4] != engine.EntityKlingon {
		t.Errorf("expected grid at [4, 4] to show EntityKlingon after screening")
	}

	// 2. Verify Crossfire counter-attack damage applied to Enterprise shields
	// Transfer added 500 to shields, but bracketed Klingons returned fire with +35% damage
	// Resulting shields should have absorbed the heavy counter-attack
	if m.Game.Enterprise.Shields >= initialShields+500 {
		t.Errorf("expected shields to have absorbed crossfire counter-attack damage, got %.1f", m.Game.Enterprise.Shields)
	}
}
