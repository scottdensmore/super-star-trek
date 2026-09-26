package tui

import (
	"strings"
	"testing"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/commandbar"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/sectorgrid"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/statuspanel"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

func TestTUI_AdversaryGlyphsRendering(t *testing.T) {
	themes := []string{theme.ThemeModern, theme.ThemeLcars, theme.ThemeCrt}

	for _, thName := range themes {
		t.Run(thName, func(t *testing.T) {
			th := theme.GetTheme(thName)
			grid := sectorgrid.New(th)

			g := engine.NewGameWithSeed(100)
			g.CurrentQuad.Grid[1][1] = engine.EntityRomulan
			g.CurrentQuad.Grid[2][2] = engine.EntityTholian
			g.CurrentQuad.Grid[3][3] = engine.EntityPlasmaTorpedo
			g.CurrentQuad.Grid[4][4] = engine.EntityTholianWeb

			view := grid.ViewWithGame(g)
			if !strings.Contains(view, "+R+") {
				t.Errorf("[%s] expected Romulan glyph '+R+' in grid view", thName)
			}
			if !strings.Contains(view, "<T>") {
				t.Errorf("[%s] expected Tholian glyph '<T>' in grid view", thName)
			}
			if !strings.Contains(view, "*P*") {
				t.Errorf("[%s] expected Plasma Torpedo glyph '*P*' in grid view", thName)
			}
			if !strings.Contains(view, ":::") {
				t.Errorf("[%s] expected Tholian Web glyph ':::' in grid view", thName)
			}
		})
	}
}

func TestTUI_AdversaryGlyphsSelectedReticle(t *testing.T) {
	th := theme.GetTheme(theme.ThemeModern)
	grid := sectorgrid.New(th)

	g := engine.NewGameWithSeed(100)
	g.CurrentQuad.Grid[1][1] = engine.EntityRomulan
	g.CurrentQuad.Grid[2][2] = engine.EntityTholian
	g.CurrentQuad.Grid[3][3] = engine.EntityPlasmaTorpedo
	g.CurrentQuad.Grid[4][4] = engine.EntityTholianWeb

	view1 := grid.View(&g.CurrentQuad, g.Enterprise.Sector, engine.Coord{1, 1})
	if !strings.Contains(view1, "[R]") {
		t.Errorf("expected selected Romulan reticle '[R]', view:\n%s", view1)
	}

	view2 := grid.View(&g.CurrentQuad, g.Enterprise.Sector, engine.Coord{2, 2})
	if !strings.Contains(view2, "[T]") {
		t.Errorf("expected selected Tholian reticle '[T]', view:\n%s", view2)
	}

	view3 := grid.View(&g.CurrentQuad, g.Enterprise.Sector, engine.Coord{3, 3})
	if !strings.Contains(view3, "[P]") {
		t.Errorf("expected selected Plasma Torpedo reticle '[P]', view:\n%s", view3)
	}

	view4 := grid.View(&g.CurrentQuad, g.Enterprise.Sector, engine.Coord{4, 4})
	if !strings.Contains(view4, "[:]") {
		t.Errorf("expected selected Tholian Web reticle '[:]', view:\n%s", view4)
	}
}

func TestTUI_StatusPanel_HazardAlerts(t *testing.T) {
	th := theme.GetTheme(theme.ThemeModern)
	panel := statuspanel.New(th)

	g := engine.NewGameWithSeed(200)

	// 1. Plasma Torpedo Alert
	g.CurrentQuad.PlasmaTorpedoes = []*engine.PlasmaTorpedo{
		{ID: 1001, Sector: engine.Coord{3, 4}, Energy: 850.0},
	}
	view := panel.View(g)
	if !strings.Contains(view, "⚠️ INCOMING PLASMA TORPEDO TRACKING") {
		t.Errorf("expected status panel to contain incoming plasma torpedo alert, got:\n%s", view)
	}

	// 2. Tholian Web Alert
	g.CurrentQuad.PlasmaTorpedoes = nil
	g.CurrentQuad.WebSegments = []*engine.TholianWebSegment{
		{Coord: engine.Coord{2, 2}, Strength: 100.0},
		{Coord: engine.Coord{2, 3}, Strength: 100.0},
	}
	view = panel.View(g)
	if !strings.Contains(view, "⚠️ THOLIAN WEB ENCLOSURE:") {
		t.Errorf("expected status panel to contain web enclosure alert, got:\n%s", view)
	}

	// 3. Crossfire Bracket Alert
	g.CurrentQuad.WebSegments = nil
	g.Enterprise.Sector = engine.Coord{4, 4}
	g.CurrentQuad.Enemies = []*engine.EnemyVessel{
		{ID: 1, Faction: engine.FactionKlingon, Sector: engine.Coord{1, 4}, Energy: 500},
		{ID: 2, Faction: engine.FactionKlingon, Sector: engine.Coord{4, 7}, Energy: 500},
	}
	engine.SyncQuadrantEnemies(&g.CurrentQuad)
	view = panel.View(g)
	if !strings.Contains(view, "⚠️ CROSSFIRE BRACKET ACTIVE (+35% DMG)") {
		t.Errorf("expected status panel to contain crossfire bracket alert, got:\n%s", view)
	}
}

func TestTUI_HostileTurn_UnifiedAdversaryTurn(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	g.Rules.Adversaries = true
	m := NewModel(g, theme.DefaultTheme())

	// Place an in-flight plasma torpedo that should advance during the hostile turn
	m.Game.CurrentQuad.PlasmaTorpedoes = []*engine.PlasmaTorpedo{
		{
			ID:            1001,
			Sector:        engine.Coord{2, 4},
			TargetSector:  m.Game.Enterprise.Sector,
			Energy:        1000.0,
			TurnsInFlight: 1,
		},
	}
	m.Game.CurrentQuad.Grid[2][4] = engine.EntityPlasmaTorpedo

	// Player action that triggers hostile turn
	var events []engine.Event
	m.executeKlingonTurnIfActive(engine.ActionShields{Amount: 100}, &events)

	// In-flight plasma torpedo should have moved (no longer at 2, 4)
	if m.Game.CurrentQuad.Grid[2][4] == engine.EntityPlasmaTorpedo {
		t.Errorf("expected plasma torpedo at 2,4 to advance via UnifiedAdversaryTurn")
	}
}

func TestEngine_ActionFireTorpedo_DirectCollisions_Adversaries(t *testing.T) {
	// 1. Collision with Romulan
	{
		g := engine.NewGameWithSeed(300)
		g.Enterprise.Sector = engine.Coord{4, 1}
		g.Enterprise.Torpedoes = 5
		g.CurrentQuad.Grid[4][5] = engine.EntityRomulan
		romulan := &engine.EnemyVessel{
			ID:      201,
			Faction: engine.FactionRomulan,
			Sector:  engine.Coord{4, 5},
			Energy:  200,
			Shields: 0,
		}
		g.CurrentQuad.Enemies = []*engine.EnemyVessel{romulan}

		events, err := engine.ActionFireTorpedo{Direction: 1.0}.Execute(g)
		if err != nil {
			t.Fatalf("unexpected torpedo error: %v", err)
		}
		if g.CurrentQuad.Grid[4][5] != engine.EntityEmpty {
			t.Errorf("expected Romulan sector to be emptied after direct torpedo hit")
		}
		hitFound := false
		for _, ev := range events {
			if th, ok := ev.(engine.EventTorpedoHit); ok && th.Entity == engine.EntityRomulan && th.Destroyed {
				hitFound = true
				break
			}
		}
		if !hitFound {
			t.Errorf("expected EventTorpedoHit for EntityRomulan Destroyed=true")
		}
	}

	// 2. Collision with Tholian
	{
		g := engine.NewGameWithSeed(301)
		g.Enterprise.Sector = engine.Coord{4, 1}
		g.Enterprise.Torpedoes = 5
		g.CurrentQuad.Grid[4][5] = engine.EntityTholian
		tholian := &engine.EnemyVessel{
			ID:      301,
			Faction: engine.FactionTholian,
			Sector:  engine.Coord{4, 5},
			Energy:  200,
			Shields: 0,
		}
		g.CurrentQuad.Enemies = []*engine.EnemyVessel{tholian}

		events, err := engine.ActionFireTorpedo{Direction: 1.0}.Execute(g)
		if err != nil {
			t.Fatalf("unexpected torpedo error: %v", err)
		}
		if g.CurrentQuad.Grid[4][5] != engine.EntityEmpty {
			t.Errorf("expected Tholian sector to be emptied after direct torpedo hit")
		}
		hitFound := false
		for _, ev := range events {
			if th, ok := ev.(engine.EventTorpedoHit); ok && th.Entity == engine.EntityTholian && th.Destroyed {
				hitFound = true
				break
			}
		}
		if !hitFound {
			t.Errorf("expected EventTorpedoHit for EntityTholian Destroyed=true")
		}
	}

	// 3. Collision with Plasma Torpedo (interception)
	{
		g := engine.NewGameWithSeed(302)
		g.Enterprise.Sector = engine.Coord{4, 1}
		g.Enterprise.Torpedoes = 5
		g.CurrentQuad.Grid[4][5] = engine.EntityPlasmaTorpedo
		g.CurrentQuad.PlasmaTorpedoes = []*engine.PlasmaTorpedo{
			{ID: 1001, Sector: engine.Coord{4, 5}, Energy: 800.0},
		}

		events, err := engine.ActionFireTorpedo{Direction: 1.0}.Execute(g)
		if err != nil {
			t.Fatalf("unexpected torpedo error: %v", err)
		}
		if g.CurrentQuad.Grid[4][5] != engine.EntityEmpty {
			t.Errorf("expected Plasma Torpedo sector to be cleared after interception")
		}
		if len(g.CurrentQuad.PlasmaTorpedoes) != 0 {
			t.Errorf("expected plasma torpedo slice to be empty, got %d", len(g.CurrentQuad.PlasmaTorpedoes))
		}
		hitFound := false
		for _, ev := range events {
			if th, ok := ev.(engine.EventTorpedoHit); ok && th.Entity == engine.EntityPlasmaTorpedo && th.Destroyed {
				hitFound = true
				break
			}
		}
		if !hitFound {
			t.Errorf("expected EventTorpedoHit for EntityPlasmaTorpedo Destroyed=true")
		}
	}

	// 4. Collision with Tholian Web (breach)
	{
		g := engine.NewGameWithSeed(303)
		g.Enterprise.Sector = engine.Coord{4, 1}
		g.Enterprise.Torpedoes = 5
		g.CurrentQuad.Grid[4][5] = engine.EntityTholianWeb
		g.CurrentQuad.WebSegments = []*engine.TholianWebSegment{
			{Coord: engine.Coord{4, 5}, Strength: 150.0},
		}

		events, err := engine.ActionFireTorpedo{Direction: 1.0}.Execute(g)
		if err != nil {
			t.Fatalf("unexpected torpedo error: %v", err)
		}
		if g.CurrentQuad.Grid[4][5] != engine.EntityEmpty {
			t.Errorf("expected Tholian Web sector to be breached and emptied")
		}
		if len(g.CurrentQuad.WebSegments) != 0 {
			t.Errorf("expected web segments to be empty, got %d", len(g.CurrentQuad.WebSegments))
		}
		hitFound := false
		for _, ev := range events {
			if th, ok := ev.(engine.EventTorpedoHit); ok && th.Entity == engine.EntityTholianWeb && th.Destroyed {
				hitFound = true
				break
			}
		}
		if !hitFound {
			t.Errorf("expected EventTorpedoHit for EntityTholianWeb Destroyed=true")
		}
	}
}

func TestTUI_TargetLock_AdversaryCycling(t *testing.T) {
	g := engine.NewGameWithSeed(500)
	g.Enterprise.Sector = engine.Coord{4, 4}
	g.CurrentQuad.Enemies = []*engine.EnemyVessel{
		{ID: 201, Faction: engine.FactionRomulan, Sector: engine.Coord{2, 4}, Energy: 400},
		{ID: 202, Faction: engine.FactionRomulan, Sector: engine.Coord{2, 5}, Energy: 400, IsCloaked: true}, // Cloaked, must be excluded
		{ID: 301, Faction: engine.FactionTholian, Sector: engine.Coord{6, 4}, Energy: 500},
	}
	g.CurrentQuad.PlasmaTorpedoes = []*engine.PlasmaTorpedo{
		{ID: 1001, Sector: engine.Coord{3, 4}, Energy: 850.0},
	}
	g.CurrentQuad.WebSegments = []*engine.TholianWebSegment{
		{Coord: engine.Coord{5, 4}, Strength: 120.0},
	}

	m := NewModel(g, theme.DefaultTheme())
	m.TargetLock.SetStateFromGame(g, engine.Coord{})

	// Cycle through all targets and collect names
	var names []string
	for i := 0; i < 4; i++ {
		cur := m.TargetLock.CurrentTarget()
		if cur != nil {
			names = append(names, cur.Name)
		}
		// update with tab key
		m.TargetLock, _ = m.TargetLock.Update(tea.KeyMsg{Type: tea.KeyTab})
	}

	joined := strings.Join(names, " | ")
	if !strings.Contains(joined, "ROMULAN RAIDER") {
		t.Errorf("expected Romulan Raider in target list: %s", joined)
	}
	if !strings.Contains(joined, "THOLIAN SPINNER") {
		t.Errorf("expected Tholian Spinner in target list: %s", joined)
	}
	if !strings.Contains(joined, "PLASMA TORPEDO") {
		t.Errorf("expected Plasma Torpedo in target list: %s", joined)
	}
	if !strings.Contains(joined, "THOLIAN WEB FILAMENT") {
		t.Errorf("expected Tholian Web Filament in target list: %s", joined)
	}
	if strings.Contains(joined, "#202") {
		t.Errorf("cloaked Romulan #202 should NOT be in target list: %s", joined)
	}
}

func TestTUI_PointDefense_ParserAndExecution(t *testing.T) {
	// 1. Direct phaser parsing
	parsed1 := ParseCommand("pha 400 3 5")
	if parsed1.Error != nil {
		t.Fatalf("unexpected error parsing 'pha 400 3 5': %v", parsed1.Error)
	}
	direct1, ok := parsed1.Action.(engine.ActionPhaserDirect)
	if !ok || direct1.Energy != 400 || direct1.TargetSector != (engine.Coord{3, 5}) {
		t.Fatalf("mismatched parsed action: %+v", parsed1.Action)
	}

	parsed2 := ParseCommand("pha 400 3,5")
	if parsed2.Error != nil {
		t.Fatalf("unexpected error parsing 'pha 400 3,5': %v", parsed2.Error)
	}
	direct2, ok := parsed2.Action.(engine.ActionPhaserDirect)
	if !ok || direct2.Energy != 400 || direct2.TargetSector != (engine.Coord{3, 5}) {
		t.Fatalf("mismatched parsed action: %+v", parsed2.Action)
	}

	// 2. Direct phaser execution against in-flight plasma torpedo
	g := engine.NewGameWithSeed(600)
	g.Enterprise.Sector = engine.Coord{4, 4}
	g.Enterprise.Energy = 4000
	g.CurrentQuad.Grid[3][4] = engine.EntityPlasmaTorpedo
	g.CurrentQuad.PlasmaTorpedoes = []*engine.PlasmaTorpedo{
		{ID: 1001, Sector: engine.Coord{3, 4}, Energy: 800.0},
	}

	m := NewModel(g, theme.DefaultTheme())
	// Send command submitted message
	updated, _ := m.Update(commandbar.CommandSubmittedMsg{Text: "pha 800 3,4"})
	mod := updated.(Model)
	if mod.Game.CurrentQuad.Grid[3][4] != engine.EntityEmpty {
		t.Errorf("expected plasma torpedo to be intercepted and cell cleared")
	}
	if len(mod.Game.CurrentQuad.PlasmaTorpedoes) != 0 {
		t.Errorf("expected plasma torpedo slice to be empty, got %d", len(mod.Game.CurrentQuad.PlasmaTorpedoes))
	}
}

