package main

import (
	"strings"
	"testing"

	"github.com/scottdensmore/super-star-trek/pkg/engine"
)

func TestFormatSRS(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	g.PopulateQuadrant(g.Enterprise.Quad, g.Enterprise.Sector)

	out := FormatSRS(g)
	if !strings.Contains(out, "<E>") {
		t.Errorf("expected Enterprise glyph <E> in SRS, got:\n%s", out)
	}
	if !strings.Contains(out, "Stardate:") || !strings.Contains(out, "Energy:") {
		t.Errorf("expected telemetry metrics in SRS, got:\n%s", out)
	}
	if !strings.Contains(out, "\r\n") {
		t.Errorf("expected CR LF line endings for xterm.js compatibility")
	}
	if !strings.Contains(out, "CONDITION:") {
		t.Errorf("expected CONDITION in SRS, got:\n%s", out)
	}
	// Verify dim ANSI styling for row coordinates
	if !strings.Contains(out, "\x1b[2m") {
		t.Errorf("expected dim ANSI styling for row coordinates")
	}
}

func TestFormatSRS_Conditions(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	g.PopulateQuadrant(g.Enterprise.Quad, g.Enterprise.Sector)

	conditions := []struct {
		cond     engine.ConditionType
		wantWord string
	}{
		{engine.ConditionGreen, "GREEN"},
		{engine.ConditionYellow, "YELLOW"},
		{engine.ConditionRed, "RED"},
		{engine.ConditionDocked, "DOCKED"},
	}

	for _, tc := range conditions {
		g.Enterprise.Condition = tc.cond
		out := FormatSRS(g)
		if !strings.Contains(out, tc.wantWord) {
			t.Errorf("condition %v: expected %q in SRS output, got:\n%s", tc.cond, tc.wantWord, out)
		}
	}
}

func TestFormatSRS_Glyphs(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	g.PopulateQuadrant(g.Enterprise.Quad, g.Enterprise.Sector)

	// Explicitly place entities on the quadrant grid
	klingonCoord := engine.Coord{2, 3}
	starbaseCoord := engine.Coord{4, 5}
	starCoord := engine.Coord{6, 7}

	g.CurrentQuad.Grid[klingonCoord.Row()][klingonCoord.Col()] = engine.EntityKlingon
	g.CurrentQuad.Grid[starbaseCoord.Row()][starbaseCoord.Col()] = engine.EntityStarbase
	g.CurrentQuad.Grid[starCoord.Row()][starCoord.Col()] = engine.EntityStar

	out := FormatSRS(g)
	if !strings.Contains(out, "+K+") {
		t.Errorf("expected Klingon glyph +K+ in SRS, got:\n%s", out)
	}
	if !strings.Contains(out, ">B<") {
		t.Errorf("expected Starbase glyph >B< in SRS, got:\n%s", out)
	}
	if !strings.Contains(out, " * ") {
		t.Errorf("expected Star glyph ' * ' in SRS, got:\n%s", out)
	}
}

func TestFormatLRS(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	out := FormatLRS(g)
	if !strings.Contains(out, "LONG RANGE SENSOR SCAN") {
		t.Errorf("expected LRS header, got:\n%s", out)
	}
	if !strings.Contains(out, "-------------------") {
		t.Errorf("expected grid dividers in LRS, got:\n%s", out)
	}
	if !strings.Contains(out, ": ") {
		t.Errorf("expected cell separators ': ' in LRS, got:\n%s", out)
	}
}

func TestFormatChart(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	g.Enterprise.Quad = engine.Coord{4, 4}
	g.GalaxyChart[4][4] = 105
	g.ChartDiscovered[2][2] = true
	g.GalaxyChart[2][2] = 203
	g.ChartKnownBases[3][3] = true

	out := FormatChart(g)
	if !strings.Contains(out, "=== GALACTIC STAR CHART ===") {
		t.Errorf("expected chart header, got:\n%s", out)
	}
	if !strings.Contains(out, "105") {
		t.Errorf("expected current quad value 105 in chart, got:\n%s", out)
	}
	if !strings.Contains(out, "203") {
		t.Errorf("expected discovered quad value 203 in chart, got:\n%s", out)
	}
	if !strings.Contains(out, ".B.") {
		t.Errorf("expected known starbase marker .B. in chart, got:\n%s", out)
	}
	if !strings.Contains(out, "...") {
		t.Errorf("expected undiscovered marker ... in chart, got:\n%s", out)
	}
}

func TestFormatDamages(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	g.Enterprise.Devices[engine.DeviceWarp] = 3.5

	out := FormatDamages(g)
	if !strings.Contains(out, "=== DAMAGE CONTROL REPORT ===") {
		t.Errorf("expected damage report header, got:\n%s", out)
	}
	if !strings.Contains(out, "Warp Engines") {
		t.Errorf("expected Warp Engines in report, got:\n%s", out)
	}
	if !strings.Contains(out, "DAMAGED (Repair in 3.5 stardates)") {
		t.Errorf("expected damaged status with repair time, got:\n%s", out)
	}
	if !strings.Contains(out, "OPERATIONAL") {
		t.Errorf("expected OPERATIONAL status for undamaged systems, got:\n%s", out)
	}
}

func TestFormatCombatEvents(t *testing.T) {
	events := []engine.Event{
		engine.EventTorpedoFired{Origin: engine.Coord{3, 3}, Angle: 0.0},
		engine.EventTorpedoHit{Target: engine.Coord{3, 7}, Damage: 350, Destroyed: true},
	}
	out := FormatCombatEvents(events)
	if !strings.Contains(out, "TORPEDO TRACK") || !strings.Contains(out, "DESTROYED") {
		t.Errorf("expected torpedo tracking and destruction message, got:\n%s", out)
	}
}

func TestFormatCombatEvents_AnomalyEvents(t *testing.T) {
	events := []engine.Event{
		engine.EventHazardTriggered{
			HazardType:  "ion_storm_drift",
			Description: "Ion storm turbulence deflected course",
		},
		engine.EventSingularityAbsorption{
			Sector: engine.Coord{4, 4},
			Weapon: "torpedo",
		},
		engine.EventWormholeJump{
			FromQuad: engine.Coord{1, 1},
			ToQuad:   engine.Coord{5, 6},
			ToSector: engine.Coord{3, 3},
		},
	}
	output := FormatCombatEvents(events)
	if !strings.Contains(output, "turbulence deflected course") {
		t.Errorf("expected ion storm message, got: %s", output)
	}
	if !strings.Contains(output, "absorbed into event horizon") {
		t.Errorf("expected singularity absorption message, got: %s", output)
	}
	if !strings.Contains(output, "Wormhole transit completed") {
		t.Errorf("expected wormhole transit message, got: %s", output)
	}
}

func TestFormatCombatEvents_AnomalyDiscovered(t *testing.T) {
	events := []engine.Event{
		engine.EventAnomalyDiscovered{
			Quad: engine.Coord{2, 3},
			Env:  engine.EnvNebula,
		},
		engine.EventAnomalyDiscovered{
			Quad: engine.Coord{4, 5},
			Env:  engine.EnvIonStorm,
		},
	}
	output := FormatCombatEvents(events)
	if !strings.Contains(output, "Entering Mutara Nebula") {
		t.Errorf("expected nebula discovery message, got: %s", output)
	}
	if !strings.Contains(output, "Entering Ion Storm") {
		t.Errorf("expected ion storm discovery message, got: %s", output)
	}
}

func TestFormatSRS_Wormhole(t *testing.T) {
	g := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	g.PopulateQuadrant(g.Enterprise.Quad, g.Enterprise.Sector)
	g.CurrentQuad.Grid[3][3] = engine.EntityWormhole
	out := FormatSRS(g)
	if !strings.Contains(out, ">W<") {
		t.Errorf("expected Wormhole glyph '>W<' in SRS, got:\n%s", out)
	}
}

func TestFormatCombatEvents_AllTypes(t *testing.T) {
	events := []engine.Event{
		engine.EventPhaserFired{Energy: 400},
		engine.EventPhaserHit{Target: engine.Coord{2, 2}, Damage: 250, Destroyed: false},
		engine.EventPhaserHit{Target: engine.Coord{2, 3}, Damage: 500, Destroyed: true},
		engine.EventShieldTransfer{NewShields: 1500, NewEnergy: 3500},
		engine.EventShipMoved{FromQuad: engine.Coord{1, 1}, ToQuad: engine.Coord{1, 1}, FromSector: engine.Coord{2, 2}, ToSector: engine.Coord{4, 4}, Warp: 1.0},
		engine.EventShipMoved{FromQuad: engine.Coord{1, 1}, ToQuad: engine.Coord{2, 2}, FromSector: engine.Coord{4, 4}, ToSector: engine.Coord{1, 1}, Warp: 2.0},
		engine.EventObstacleEncountered{Sector: engine.Coord{5, 5}, Entity: engine.EntityStar},
		engine.EventDocked{Starbase: engine.Coord{3, 3}},
		engine.EventKlingonCounterAttack{EnemyID: 1, Damage: 120},
		engine.EventSubsystemDamaged{Device: engine.DeviceShields, RepairTime: 2.4},
		engine.EventSubsystemDamaged{Device: -1, RepairTime: 1.0},
		engine.EventSubsystemRepaired{Device: engine.DeviceShields},
		engine.EventSubsystemRepaired{Device: 99},
		engine.EventGameOver{Reason: engine.GameOverWon, Score: 1250},
		engine.EventGameOver{Reason: engine.GameOverDestroyed, Score: 450},
	}

	out := FormatCombatEvents(events)
	expectedSubstrings := []string{
		"[PHASERS FIRED]",
		"Allocated 400 units",
		"Phaser beam hit target at [2,2]: 250 units",
		"KLINGON DESTROYED BY PHASER FIRE",
		"Deflector Shields: 1500  |  Total energy: 3500",
		"Arrived at sector [4,4]",
		"Arrived at quadrant [2,2] sector [1,1]",
		"[NAVIGATION HAZARD]",
		"[STARBASE DOCKING]",
		"[RETURN FIRE]",
		"[DAMAGE]",
		"Shield Control damaged! Repair in 2.4 stardates",
		"Subsystem damaged! Repair in 1.0 stardates",
		"[REPAIR]",
		"Shield Control has been repaired",
		"Subsystem has been repaired",
		"*** FEDERATION MISSION ACCOMPLISHED ***",
		"*** MISSION TERMINATED ***",
	}

	for _, sub := range expectedSubstrings {
		if !strings.Contains(out, sub) {
			t.Errorf("expected combat output to contain %q, got:\n%s", sub, out)
		}
	}
}

func TestFormatters_NilAndEmpty(t *testing.T) {
	if out := FormatSRS(nil); out != "" {
		t.Errorf("expected empty string for nil SRS, got: %q", out)
	}
	if out := FormatLRS(nil); out != "" {
		t.Errorf("expected empty string for nil LRS, got: %q", out)
	}
	if out := FormatChart(nil); out != "" {
		t.Errorf("expected empty string for nil Chart, got: %q", out)
	}
	if out := FormatDamages(nil); out != "" {
		t.Errorf("expected empty string for nil Damages, got: %q", out)
	}
	if out := FormatCombatEvents(nil); out != "" {
		t.Errorf("expected empty string for nil events, got: %q", out)
	}
	if out := FormatCombatEvents([]engine.Event{}); out != "" {
		t.Errorf("expected empty string for empty events, got: %q", out)
	}
}
