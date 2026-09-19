package engine

import (
	"math"
	"strings"
	"testing"
)

func TestPhaserAttenuation(t *testing.T) {
	// Phaser damage falls off inversely with distance
	d1 := ComputePhaserDamage(1000, 1.0)
	d2 := ComputePhaserDamage(1000, 2.0)
	if d2 >= d1 {
		t.Fatalf("expected phaser damage to fall off with distance: d1=%f, d2=%f", d1, d2)
	}

	// Proportionality: 2000 at dist 2.0 equals 1000 at dist 1.0
	d3 := ComputePhaserDamage(2000, 2.0)
	if math.Abs(d3-d1) > 0.0001 {
		t.Fatalf("expected d3 == d1, got d3=%f, d1=%f", d3, d1)
	}

	// Zero or negative distance clamps to 0.1
	dZero := ComputePhaserDamage(100, 0)
	dNeg := ComputePhaserDamage(100, -1.0)
	dExpected := 100 * (1.0 / 0.1)
	if math.Abs(dZero-dExpected) > 0.0001 {
		t.Fatalf("expected zero dist to clamp to 0.1, got %f", dZero)
	}
	if math.Abs(dNeg-dExpected) > 0.0001 {
		t.Fatalf("expected negative dist to clamp to 0.1, got %f", dNeg)
	}

	// Zero or negative energy yields zero damage
	if d := ComputePhaserDamage(0, 5.0); d != 0 {
		t.Fatalf("expected 0 damage for 0 energy, got %f", d)
	}
	if d := ComputePhaserDamage(-50, 5.0); d != 0 {
		t.Fatalf("expected 0 damage for negative energy, got %f", d)
	}
}

func TestTorpedoPathTracing(t *testing.T) {
	t.Run("hit directly right east", func(t *testing.T) {
		quad := &QuadrantState{}
		quad.Grid[4][7] = EntityKlingon
		origin := Coord{4, 1}

		// Trajectory pointing directly right (angle = 0)
		hitCoord, entity, hit := TraceTorpedoPath(origin, 0.0, quad)
		if !hit || entity != EntityKlingon || hitCoord != (Coord{4, 7}) {
			t.Fatalf("expected torpedo to hit Klingon at [4, 7], got %v %v %v", hitCoord, entity, hit)
		}
	})

	t.Run("hit directly north up", func(t *testing.T) {
		quad := &QuadrantState{}
		quad.Grid[2][3] = EntityStar
		origin := Coord{6, 3}

		// Angle Pi/2 points North
		hitCoord, entity, hit := TraceTorpedoPath(origin, math.Pi/2, quad)
		if !hit || entity != EntityStar || hitCoord != (Coord{2, 3}) {
			t.Fatalf("expected torpedo to hit Star at [2, 3], got %v %v %v", hitCoord, entity, hit)
		}
	})

	t.Run("hit directly west left", func(t *testing.T) {
		quad := &QuadrantState{}
		quad.Grid[3][2] = EntityStarbase
		origin := Coord{3, 6}

		// Angle Pi points West
		hitCoord, entity, hit := TraceTorpedoPath(origin, math.Pi, quad)
		if !hit || entity != EntityStarbase || hitCoord != (Coord{3, 2}) {
			t.Fatalf("expected torpedo to hit Starbase at [3, 2], got %v %v %v", hitCoord, entity, hit)
		}
	})

	t.Run("hit directly south down", func(t *testing.T) {
		quad := &QuadrantState{}
		quad.Grid[7][5] = EntityPlanet
		origin := Coord{2, 5}

		// Angle 3*Pi/2 points South
		hitCoord, entity, hit := TraceTorpedoPath(origin, 3*math.Pi/2, quad)
		if !hit || entity != EntityPlanet || hitCoord != (Coord{7, 5}) {
			t.Fatalf("expected torpedo to hit Planet at [7, 5], got %v %v %v", hitCoord, entity, hit)
		}
	})

	t.Run("hit northeast diagonal", func(t *testing.T) {
		quad := &QuadrantState{}
		quad.Grid[2][5] = EntityCommander
		origin := Coord{5, 2}

		angle := Bearing(origin, Coord{2, 5})
		hitCoord, entity, hit := TraceTorpedoPath(origin, angle, quad)
		if !hit || entity != EntityCommander || hitCoord != (Coord{2, 5}) {
			t.Fatalf("expected torpedo to hit Commander at [2, 5], got %v %v %v", hitCoord, entity, hit)
		}
	})

	t.Run("obstacle in path hit first", func(t *testing.T) {
		quad := &QuadrantState{}
		quad.Grid[4][4] = EntityStar
		quad.Grid[4][7] = EntityKlingon
		origin := Coord{4, 1}

		hitCoord, entity, hit := TraceTorpedoPath(origin, 0.0, quad)
		if !hit || entity != EntityStar || hitCoord != (Coord{4, 4}) {
			t.Fatalf("expected torpedo to hit Star at [4, 4] first, got %v %v %v", hitCoord, entity, hit)
		}
	})

	t.Run("miss exits quadrant", func(t *testing.T) {
		quad := &QuadrantState{}
		origin := Coord{4, 1}

		hitCoord, entity, hit := TraceTorpedoPath(origin, 0.0, quad)
		if hit || entity != EntityEmpty || hitCoord != (Coord{}) {
			t.Fatalf("expected torpedo to miss and exit quadrant, got %v %v %v", hitCoord, entity, hit)
		}
	})

	t.Run("nil quadrant or out of bounds", func(t *testing.T) {
		hitCoord, entity, hit := TraceTorpedoPath(Coord{4, 1}, 0.0, nil)
		if hit || entity != EntityEmpty || hitCoord != (Coord{}) {
			t.Fatalf("expected nil quadrant to return false, got %v %v %v", hitCoord, entity, hit)
		}

		quad := &QuadrantState{}
		invalidOrigins := []Coord{
			{0, 4},
			{9, 4},
			{4, 0},
			{4, 9},
		}
		for _, o := range invalidOrigins {
			hitCoord, entity, hit = TraceTorpedoPath(o, 0.0, quad)
			if hit || entity != EntityEmpty || hitCoord != (Coord{}) {
				t.Fatalf("expected out of bounds origin %v to return false, got %v %v %v", o, hitCoord, entity, hit)
			}
		}
	})
}

func TestResolveShieldHit(t *testing.T) {
	t.Run("shields absorb all damage", func(t *testing.T) {
		ent := &Enterprise{
			Energy:  3000,
			Shields: 1000,
		}
		shieldDmg, hullDmg := ResolveShieldHit(ent, 400)
		if shieldDmg != 400 || hullDmg != 0 {
			t.Fatalf("expected (400, 0), got (%f, %f)", shieldDmg, hullDmg)
		}
		if ent.Shields != 600 || ent.Energy != 3000 {
			t.Fatalf("expected shields=600, energy=3000, got shields=%f, energy=%f", ent.Shields, ent.Energy)
		}
	})

	t.Run("shields absorb exact damage", func(t *testing.T) {
		ent := &Enterprise{
			Energy:  2500,
			Shields: 500,
		}
		shieldDmg, hullDmg := ResolveShieldHit(ent, 500)
		if shieldDmg != 500 || hullDmg != 0 {
			t.Fatalf("expected (500, 0), got (%f, %f)", shieldDmg, hullDmg)
		}
		if ent.Shields != 0 || ent.Energy != 2500 {
			t.Fatalf("expected shields=0, energy=2500, got shields=%f, energy=%f", ent.Shields, ent.Energy)
		}
	})

	t.Run("partial shield absorption remainder to energy", func(t *testing.T) {
		ent := &Enterprise{
			Energy:  2000,
			Shields: 300,
		}
		shieldDmg, hullDmg := ResolveShieldHit(ent, 500)
		if shieldDmg != 300 || hullDmg != 200 {
			t.Fatalf("expected (300, 200), got (%f, %f)", shieldDmg, hullDmg)
		}
		if ent.Shields != 0 || ent.Energy != 1800 {
			t.Fatalf("expected shields=0, energy=1800, got shields=%f, energy=%f", ent.Shields, ent.Energy)
		}
	})

	t.Run("zero shields takes full hull damage", func(t *testing.T) {
		ent := &Enterprise{
			Energy:  1000,
			Shields: 0,
		}
		shieldDmg, hullDmg := ResolveShieldHit(ent, 250)
		if shieldDmg != 0 || hullDmg != 250 {
			t.Fatalf("expected (0, 250), got (%f, %f)", shieldDmg, hullDmg)
		}
		if ent.Shields != 0 || ent.Energy != 750 {
			t.Fatalf("expected shields=0, energy=750, got shields=%f, energy=%f", ent.Shields, ent.Energy)
		}
	})

	t.Run("negative shields clamped to zero", func(t *testing.T) {
		ent := &Enterprise{
			Energy:  1000,
			Shields: -50,
		}
		shieldDmg, hullDmg := ResolveShieldHit(ent, 100)
		if shieldDmg != 0 || hullDmg != 100 || ent.Shields != 0 || ent.Energy != 900 {
			t.Fatalf("expected negative shields to clamp to 0 and take 100 hull damage, got shieldDmg=%f, hullDmg=%f, shields=%f, energy=%f", shieldDmg, hullDmg, ent.Shields, ent.Energy)
		}
	})

	t.Run("zero or negative damage", func(t *testing.T) {
		ent := &Enterprise{
			Energy:  1000,
			Shields: 500,
		}
		shieldDmg, hullDmg := ResolveShieldHit(ent, 0)
		if shieldDmg != 0 || hullDmg != 0 || ent.Shields != 500 || ent.Energy != 1000 {
			t.Fatalf("expected no damage for 0 damage, got (%f, %f)", shieldDmg, hullDmg)
		}

		shieldDmg, hullDmg = ResolveShieldHit(ent, -50)
		if shieldDmg != 0 || hullDmg != 0 || ent.Shields != 500 || ent.Energy != 1000 {
			t.Fatalf("expected no damage for negative damage, got (%f, %f)", shieldDmg, hullDmg)
		}
	})

	t.Run("nil enterprise", func(t *testing.T) {
		shieldDmg, hullDmg := ResolveShieldHit(nil, 500)
		if shieldDmg != 0 || hullDmg != 0 {
			t.Fatalf("expected (0, 0) for nil enterprise, got (%f, %f)", shieldDmg, hullDmg)
		}
	})
}

func TestKlingonCloaking_LockLockoutAndDecloak(t *testing.T) {
	rules := DefaultRulesForProfile(ProfileHardcore)
	g := NewGameWithOptions(100, SkillGood, LengthMedium, rules)
	klingon := &Klingon{
		ID:          1,
		Sector:      Coord{4, 4},
		Energy:      400,
		IsCommander: true,
		IsCloaked:   true,
	}
	g.CurrentQuad.Klingons = []*Klingon{klingon}
	g.CurrentQuad.Grid[4][4] = EntityCommander

	// Firing direct torpedo at cloaked commander must fail target lock
	action := ActionTorpedoDirect{TargetSector: Coord{4, 4}}
	_, err := action.Execute(g)
	if err == nil || !strings.Contains(err.Error(), "CLOAKED") {
		t.Errorf("expected error containing 'CLOAKED', got %v", err)
	}

	// Phaser sweep or direct hit decloaks commander
	events := DecloakKlingon(g, klingon)
	if klingon.IsCloaked {
		t.Errorf("klingon should be decloaked")
	}
	if len(events) == 0 {
		t.Errorf("expected EventKlingonCloakState event")
	}
	if ev, ok := events[0].(EventKlingonCloakState); !ok || ev.Cloaked || ev.KlingonID != 1 {
		t.Errorf("expected EventKlingonCloakState with Cloaked=false, KlingonID=1, got %+v", events[0])
	}
	if events[0].EventType() != "KlingonCloakState" {
		t.Errorf("expected EventType 'KlingonCloakState', got %q", events[0].EventType())
	}

	// Decloaking already uncloaked Klingon produces nil events
	events2 := DecloakKlingon(g, klingon)
	if events2 != nil {
		t.Errorf("expected nil events when already uncloaked, got %v", events2)
	}

	// Direct torpedo on uncloaked Klingon should proceed without cloaked vessel error
	_, err = action.Execute(g)
	if err != nil && strings.Contains(err.Error(), "CLOAKED") {
		t.Errorf("unexpected cloaked vessel error when uncloaked: %v", err)
	}
}

func TestRepairMultiplier(t *testing.T) {
	rules := GameRules{RepairMultiplier: 2.0}
	g := NewGameWithOptions(100, SkillGood, LengthMedium, rules)
	g.Enterprise.Devices[DeviceWarp] = 4.0

	// Advance turn by 1.0 stardate unit
	// With 2.0x multiplier, repaired amount is 1.0 / 2.0 = 0.5 units
	AdvanceRepairs(g, 1.0)
	expected := 3.5
	if g.Enterprise.Devices[DeviceWarp] != expected {
		t.Errorf("expected device warp damage %f, got %f", expected, g.Enterprise.Devices[DeviceWarp])
	}

	// Repair exceeding remaining damage clamps to zero
	AdvanceRepairs(g, 10.0)
	if g.Enterprise.Devices[DeviceWarp] != 0 {
		t.Errorf("expected device warp damage clamped to 0, got %f", g.Enterprise.Devices[DeviceWarp])
	}

	// Zero or negative elapsed time does nothing
	g.Enterprise.Devices[DeviceWarp] = 2.0
	AdvanceRepairs(g, 0.0)
	AdvanceRepairs(g, -1.0)
	if g.Enterprise.Devices[DeviceWarp] != 2.0 {
		t.Errorf("expected device warp damage unchanged at 2.0, got %f", g.Enterprise.Devices[DeviceWarp])
	}

	// Zero or negative repair multiplier defaults to 1.0
	g.Rules.RepairMultiplier = 0
	AdvanceRepairs(g, 0.5)
	if g.Enterprise.Devices[DeviceWarp] != 1.5 {
		t.Errorf("expected device warp damage 1.5 with default multiplier, got %f", g.Enterprise.Devices[DeviceWarp])
	}

	// nil game state does not crash
	AdvanceRepairs(nil, 1.0)
}

func TestCloakKlingon_SuccessAndGuards(t *testing.T) {
	rules := GameRules{KlingonCloak: true}
	g := NewGameWithOptions(100, SkillGood, LengthMedium, rules)

	// Commander cloaking succeeds
	cmd := &Klingon{ID: 1, IsCommander: true, IsCloaked: false}
	events := CloakKlingon(g, cmd)
	if !cmd.IsCloaked {
		t.Fatalf("expected commander to be cloaked")
	}
	if len(events) != 1 {
		t.Fatalf("expected 1 event, got %d", len(events))
	}
	ev, ok := events[0].(EventKlingonCloakState)
	if !ok || !ev.Cloaked || ev.KlingonID != 1 {
		t.Fatalf("expected EventKlingonCloakState with Cloaked=true, ID=1, got %+v", events[0])
	}

	// Already cloaked returns nil
	events2 := CloakKlingon(g, cmd)
	if events2 != nil {
		t.Errorf("expected nil events when already cloaked, got %v", events2)
	}

	// Non-commander cannot cloak
	regular := &Klingon{ID: 2, IsCommander: false, IsCloaked: false}
	regEvents := CloakKlingon(g, regular)
	if regEvents != nil || regular.IsCloaked {
		t.Errorf("expected regular Klingon not to cloak, got events=%v, cloaked=%v", regEvents, regular.IsCloaked)
	}

	// Rules disabled prevents cloaking
	gNoCloak := NewGameWithOptions(100, SkillGood, LengthMedium, GameRules{KlingonCloak: false})
	cmd2 := &Klingon{ID: 3, IsCommander: true, IsCloaked: false}
	eventsNoCloak := CloakKlingon(gNoCloak, cmd2)
	if eventsNoCloak != nil || cmd2.IsCloaked {
		t.Errorf("expected commander not to cloak when rules disabled, got events=%v, cloaked=%v", eventsNoCloak, cmd2.IsCloaked)
	}

	// nil Klingon returns nil
	if evNil := CloakKlingon(g, nil); evNil != nil {
		t.Errorf("expected nil for nil Klingon, got %v", evNil)
	}
}

func TestKlingonCounterAttack_DecloaksAttacker(t *testing.T) {
	rules := GameRules{KlingonCloak: true}
	g := NewGameWithOptions(100, SkillGood, LengthMedium, rules)
	g.Enterprise.Shields = 500
	g.Enterprise.Energy = 3000

	cmd := &Klingon{
		ID:          1,
		IsCommander: true,
		IsCloaked:   true,
		Energy:      400,
	}

	// Attacking decloaks cloaked commander and damages Enterprise
	events := KlingonCounterAttack(g, cmd, 150)
	if cmd.IsCloaked {
		t.Errorf("expected commander to decloak upon counter-attack")
	}
	if len(events) != 2 {
		t.Fatalf("expected 2 events (decloak + counter-attack), got %d: %+v", len(events), events)
	}
	cloakEv, ok := events[0].(EventKlingonCloakState)
	if !ok || cloakEv.Cloaked || cloakEv.KlingonID != 1 {
		t.Errorf("expected EventKlingonCloakState with Cloaked=false, got %+v", events[0])
	}
	atkEv, ok := events[1].(EventKlingonCounterAttack)
	if !ok || atkEv.EnemyID != 1 || atkEv.Damage != 150 {
		t.Errorf("expected EventKlingonCounterAttack with EnemyID=1, Damage=150, got %+v", events[1])
	}
	if g.Enterprise.Shields != 350 {
		t.Errorf("expected shields 350, got %f", g.Enterprise.Shields)
	}

	// Subsequent attack when already uncloaked does not emit decloak event
	events2 := KlingonCounterAttack(g, cmd, 100)
	if len(events2) != 1 {
		t.Fatalf("expected 1 event for uncloaked attack, got %d", len(events2))
	}
	if _, ok := events2[0].(EventKlingonCounterAttack); !ok {
		t.Errorf("expected EventKlingonCounterAttack, got %+v", events2[0])
	}
}

func TestKlingonCombat_ZeroShieldsInNebula(t *testing.T) {
	g := NewGame(10, SkillGood, LengthMedium)
	g.QuadrantEnv[g.Enterprise.Quad[0]][g.Enterprise.Quad[1]] = EnvNebula
	klingon := &Klingon{ID: 1, Sector: Coord{4, 5}, Energy: 400, Shields: 100}
	g.CurrentQuad.Klingons = []*Klingon{klingon}
	g.CurrentQuad.Grid[4][5] = EntityKlingon

	// Symmetrical damage: in nebula, 100 energy phaser hit does full direct damage with no shield mitigation
	act := ActionFirePhasers{Energy: 200}
	_, err := act.Execute(g)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if klingon.Energy >= 400 {
		t.Errorf("expected Klingon to take full damage in nebula, remaining: %f", klingon.Energy)
	}
	if klingon.Shields != 0 {
		t.Errorf("expected Klingon shields to collapse to 0 in nebula, got %f", klingon.Shields)
	}
}

func TestKlingonCombat_AbsorbedByBlackHole(t *testing.T) {
	g := NewGame(10, SkillGood, LengthMedium)
	g.CurrentQuad.Grid[4][4] = EntityBlackHole
	klingon := &Klingon{ID: 1, Sector: Coord{4, 3}, Energy: 400}
	g.CurrentQuad.Klingons = []*Klingon{klingon}
	g.CurrentQuad.Grid[4][3] = EntityKlingon

	// Move/push Klingon into black hole at [4, 4]
	events := MoveKlingon(g, klingon, Coord{4, 4})

	foundAbsorption := false
	for _, e := range events {
		if sa, ok := e.(EventSingularityAbsorption); ok {
			foundAbsorption = true
			if sa.Target != EntityKlingon {
				t.Errorf("expected target EntityKlingon, got %v", sa.Target)
			}
			if sa.Sector != (Coord{4, 4}) {
				t.Errorf("expected sector [4, 4], got %v", sa.Sector)
			}
		}
	}
	if !foundAbsorption {
		t.Errorf("expected EventSingularityAbsorption when Klingon enters black hole")
	}
	if len(g.CurrentQuad.Klingons) != 0 {
		t.Errorf("expected Klingon to be eliminated from quadrant, remaining: %d", len(g.CurrentQuad.Klingons))
	}
	if g.CurrentQuad.Grid[4][3] != EntityEmpty {
		t.Errorf("expected origin cell [4, 3] to be empty, got %v", g.CurrentQuad.Grid[4][3])
	}
}

func TestKlingonTurn_SymmetryAndBlackHole(t *testing.T) {
	g := NewGame(10, SkillGood, LengthMedium)
	g.QuadrantEnv[g.Enterprise.Quad[0]][g.Enterprise.Quad[1]] = EnvNebula
	klingon := &Klingon{ID: 1, Sector: Coord{4, 3}, Energy: 400, Shields: 200}
	g.CurrentQuad.Klingons = []*Klingon{klingon}
	g.CurrentQuad.Grid[4][3] = EntityKlingon

	KlingonTurn(g)

	if klingon.Shields != 0 {
		t.Errorf("expected Klingon shields to collapse to 0 in nebula, got %f", klingon.Shields)
	}
}

func TestKlingonCombat_CommanderAbsorbedByBlackHole(t *testing.T) {
	g := NewGame(10, SkillGood, LengthMedium)
	g.CurrentQuad.Grid[5][5] = EntityBlackHole
	cmd := &Klingon{ID: 1, Sector: Coord{5, 4}, Energy: 600, IsCommander: true}
	g.CurrentQuad.Klingons = []*Klingon{cmd}
	g.CurrentQuad.Grid[5][4] = EntityCommander

	events := MoveKlingon(g, cmd, Coord{5, 5})
	if len(events) != 1 {
		t.Fatalf("expected 1 event, got %d", len(events))
	}
	sa, ok := events[0].(EventSingularityAbsorption)
	if !ok {
		t.Fatalf("expected EventSingularityAbsorption, got %T", events[0])
	}
	if sa.Target != EntityCommander {
		t.Errorf("expected target EntityCommander, got %v", sa.Target)
	}
	if g.Metrics.CommandersKilled != 1 {
		t.Errorf("expected CommandersKilled 1, got %d", g.Metrics.CommandersKilled)
	}
	if len(g.CurrentQuad.Klingons) != 0 {
		t.Errorf("expected Commander eliminated from CurrentQuad.Klingons")
	}
}

func TestKlingonTurn_BlackHoleAbsorption(t *testing.T) {
	g := NewGame(10, SkillGood, LengthMedium)
	g.CurrentQuad.Grid[4][4] = EntityBlackHole
	klingon := &Klingon{ID: 1, Sector: Coord{4, 4}, Energy: 400}
	g.CurrentQuad.Klingons = []*Klingon{klingon}

	events := KlingonTurn(g)
	if len(events) != 1 {
		t.Fatalf("expected 1 event, got %d", len(events))
	}
	if sa, ok := events[0].(EventSingularityAbsorption); !ok || sa.Sector != (Coord{4, 4}) {
		t.Errorf("unexpected event: %+v", events[0])
	}
	if len(g.CurrentQuad.Klingons) != 0 {
		t.Errorf("expected Klingon absorbed and removed")
	}
}

func TestMoveKlingon_RejectsOccupiedSector(t *testing.T) {
	g := NewGame(10, SkillGood, LengthMedium)
	k := &Klingon{ID: 1, Sector: Coord{4, 3}, Energy: 400}
	g.CurrentQuad.Klingons = []*Klingon{k}
	g.CurrentQuad.Grid[4][3] = EntityKlingon
	g.CurrentQuad.Grid[4][4] = EntityStar // Destination occupied by star

	events := MoveKlingon(g, k, Coord{4, 4})
	if events != nil {
		t.Errorf("expected MoveKlingon to return nil when moving into occupied cell, got %v", events)
	}
	if k.Sector != (Coord{4, 3}) {
		t.Errorf("expected Klingon sector unchanged at [4, 3], got %v", k.Sector)
	}
	if g.CurrentQuad.Grid[4][4] != EntityStar {
		t.Errorf("expected destination cell [4, 4] to remain EntityStar, got %v", g.CurrentQuad.Grid[4][4])
	}
}

func TestMoveKlingon_PreservesSuperCommander(t *testing.T) {
	g := NewGame(10, SkillGood, LengthMedium)
	sc := &Klingon{ID: 1, Sector: Coord{3, 3}, Energy: 1500, IsCommander: true}
	g.CurrentQuad.Klingons = []*Klingon{sc}
	g.CurrentQuad.Grid[3][3] = EntitySuperCommander

	// Move to empty cell [3, 4]
	events := MoveKlingon(g, sc, Coord{3, 4})
	if events != nil {
		t.Errorf("expected no events on empty move, got %v", events)
	}
	if g.CurrentQuad.Grid[3][4] != EntitySuperCommander {
		t.Errorf("expected destination cell [3, 4] to be EntitySuperCommander, got %v", g.CurrentQuad.Grid[3][4])
	}
	if g.CurrentQuad.Grid[3][3] != EntityEmpty {
		t.Errorf("expected origin cell [3, 3] to be EntityEmpty, got %v", g.CurrentQuad.Grid[3][3])
	}

	// Move into black hole at [3, 5]
	g.CurrentQuad.Grid[3][5] = EntityBlackHole
	events2 := MoveKlingon(g, sc, Coord{3, 5})
	if len(events2) != 1 {
		t.Fatalf("expected 1 event on black hole move, got %d", len(events2))
	}
	sa, ok := events2[0].(EventSingularityAbsorption)
	if !ok || sa.Target != EntitySuperCommander {
		t.Errorf("expected absorption target EntitySuperCommander, got %v", events2[0])
	}
	if g.Metrics.SuperCommandersKilled != 1 {
		t.Errorf("expected SuperCommandersKilled 1, got %d", g.Metrics.SuperCommandersKilled)
	}
}



