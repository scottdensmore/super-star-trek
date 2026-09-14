package engine

import (
	"testing"
)

func TestScore_CalculationMatchingClassicRules(t *testing.T) {
	g := NewGame(12345, SkillGood, LengthMedium)
	g.InitialStardate = 2000.0
	g.Stardate = 2010.0 // 10 stardates elapsed

	g.Metrics.KlingonsKilled = 8        // 8 * 10 = 80
	g.Metrics.CommandersKilled = 2      // 2 * 50 = 100
	g.Metrics.SuperCommandersKilled = 1 // 1 * 200 = 200
	g.Metrics.RomulansKilled = 1        // 1 * 20 = 20
	g.Metrics.RomulansSurrendered = 2   // 2 * 1 = 2
	g.Metrics.StarbasesDestroyed = 1    // -100
	g.Metrics.StarsDestroyed = 1        // -5
	g.Metrics.PlanetsDestroyed = 1      // -10
	g.Metrics.Casualties = 15           // -15
	g.Metrics.HelpCalls = 1             // -45
	g.Metrics.StarshipsLost = 0         // 0

	// 11 total kills / 10 stardates = 1.1 kill rate
	// 500 * 1.1 = 550 kill rate points
	// Won game on SkillGood (3) -> 100 * 3 = 300 win bonus
	// Total expected:
	// + 80 + 100 + 200 + 20 + 2 + 550 + 300 - 100 - 5 - 10 - 15 - 45 = 1077
	score := ComputeScore(g, true)

	if score.KlingonPoints != 80 {
		t.Errorf("expected 80 Klingon points, got %d", score.KlingonPoints)
	}
	if score.CommanderPoints != 100 {
		t.Errorf("expected 100 Commander points, got %d", score.CommanderPoints)
	}
	if score.SuperCommanderPoints != 200 {
		t.Errorf("expected 200 Super-Commander points, got %d", score.SuperCommanderPoints)
	}
	if score.KillRatePoints != 550 {
		t.Errorf("expected 550 kill rate points, got %d (kill rate: %.2f)", score.KillRatePoints, score.KillRate)
	}
	if score.WinBonus != 300 {
		t.Errorf("expected 300 win bonus, got %d", score.WinBonus)
	}
	if score.TotalScore != 1077 {
		t.Errorf("expected 1077 total score, got %d", score.TotalScore)
	}
	if score.RankBadge != "[FADM]" {
		t.Errorf("expected [FADM] rank badge for score 1077, got %s", score.RankBadge)
	}
}

func TestScore_MinimumFiveStardatesClampingWhenLost(t *testing.T) {
	g := NewGame(12345, SkillGood, LengthMedium)
	g.InitialStardate = 2000.0
	g.Stardate = 2001.0 // Only 1 stardate elapsed, but game lost!

	g.Metrics.KlingonsKilled = 2 // 2 kills
	// Elapsed clamped to 5.0 -> kill rate = 2 / 5.0 = 0.4
	// 500 * 0.4 = 200 kill rate points
	// Lost -> 0 win bonus
	// Total: 20 + 200 = 220
	score := ComputeScore(g, false)

	if score.ElapsedStardates != 5.0 {
		t.Errorf("expected elapsed clamped to 5.0, got %.1f", score.ElapsedStardates)
	}
	if score.KillRatePoints != 200 {
		t.Errorf("expected 200 kill rate points, got %d", score.KillRatePoints)
	}
	if score.WinBonus != 0 {
		t.Errorf("expected 0 win bonus when game lost, got %d", score.WinBonus)
	}
	if score.TotalScore != 220 {
		t.Errorf("expected 220 total score, got %d", score.TotalScore)
	}
	if score.RankBadge != "[CDR]" {
		t.Errorf("expected [CDR] for score 220, got %s", score.RankBadge)
	}
}

func TestScore_Ranks(t *testing.T) {
	tests := []struct {
		score int
		badge string
		title string
	}{
		{-10, "[DISHONOR]", "Dishonorable Discharge"},
		{50, "[CADET]", "Starfleet Cadet"},
		{150, "[LT]", "Lieutenant"},
		{250, "[CDR]", "Commander"},
		{400, "[CAPT]", "Captain"},
		{600, "[COMM]", "Commodore"},
		{800, "[RADM]", "Rear Admiral"},
		{1200, "[FADM]", "Fleet Admiral"},
	}

	for _, tt := range tests {
		title, badge := RankForScore(tt.score)
		if badge != tt.badge || title != tt.title {
			t.Errorf("score %d: expected (%s, %s), got (%s, %s)", tt.score, tt.title, tt.badge, title, badge)
		}
	}
}

func TestScore_SkillWinBonuses(t *testing.T) {
	skills := []struct {
		skill SkillLevel
		want  int
	}{
		{SkillNovice, 100},
		{SkillFair, 200},
		{SkillGood, 300},
		{SkillExpert, 400},
		{SkillEmeritus, 500},
	}

	for _, tt := range skills {
		g := NewGame(12345, tt.skill, LengthMedium)
		g.InitialStardate = 2000.0
		g.Stardate = 2010.0
		score := ComputeScore(g, true)
		if score.WinBonus != tt.want {
			t.Errorf("skill %v: expected win bonus %d, got %d", tt.skill, tt.want, score.WinBonus)
		}
	}
}

func TestScore_MetricTrackingInCombatAndActions(t *testing.T) {
	g := NewGame(12345, SkillGood, LengthMedium)
	g.Enterprise.Energy = 5000
	g.Enterprise.Torpedoes = 10
	g.Enterprise.Sector = Coord{1, 1}

	// Place regular Klingon and Commander
	klingon := &Klingon{ID: 1, Sector: Coord{1, 2}, Energy: 200, IsCommander: false}
	commander := &Klingon{ID: 2, Sector: Coord{1, 3}, Energy: 200, IsCommander: true}
	g.CurrentQuad.Klingons = []*Klingon{klingon, commander}
	g.CurrentQuad.Grid[1][2] = EntityKlingon
	g.CurrentQuad.Grid[1][3] = EntityCommander

	// Torpedo hits and destroys regular Klingon
	torp := ActionFireTorpedo{Angle: 0.0} // due East toward [1, 2]
	if _, err := torp.Execute(g); err != nil {
		t.Fatalf("torpedo failed: %v", err)
	}
	if g.Metrics.KlingonsKilled != 1 {
		t.Errorf("expected 1 Klingon killed, got %d", g.Metrics.KlingonsKilled)
	}

	// Phaser destroys Commander
	phaser := ActionFirePhasers{
		ManualAllocation: map[int]float64{2: 1000},
	}
	if _, err := phaser.Execute(g); err != nil {
		t.Fatalf("phaser failed: %v", err)
	}
	if g.Metrics.CommandersKilled != 1 {
		t.Errorf("expected 1 Commander killed, got %d", g.Metrics.CommandersKilled)
	}

	// Torpedo destroys Starbase
	sbCoord := Coord{1, 4}
	g.CurrentQuad.Starbase = &sbCoord
	g.CurrentQuad.Grid[1][4] = EntityStarbase
	if _, err := torp.Execute(g); err != nil {
		t.Fatalf("torpedo at starbase failed: %v", err)
	}
	if g.Metrics.StarbasesDestroyed != 1 {
		t.Errorf("expected 1 starbase destroyed, got %d", g.Metrics.StarbasesDestroyed)
	}

	// Call help increments metric
	if _, err := (ActionCallHelp{}).Execute(g); err != nil {
		t.Fatalf("call help failed: %v", err)
	}
	if g.Metrics.HelpCalls != 1 {
		t.Errorf("expected 1 help call, got %d", g.Metrics.HelpCalls)
	}
}
