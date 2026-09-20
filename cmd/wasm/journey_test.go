package main

import (
	"fmt"
	"strings"
	"testing"

	"github.com/scottdensmore/super-star-trek/pkg/audio"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
)

// TestWASMJourney_Moving tests sector maneuvering and inter-quadrant warp jumps via the WASM session.
func TestWASMJourney_Moving(t *testing.T) {
	s := NewSession(12345, engine.ProfileNormal)
	mock := &mockAudioPlayer{}
	s.SetAudioPlayer(mock)

	g := s.Game()
	startSector := g.Enterprise.Sector
	startQuad := g.Enterprise.Quad
	initialEnergy := g.Enterprise.Energy

	// 1. Sector navigation
	targetSector := engine.Coord{startSector[0] + 1, startSector[1]}
	if targetSector[0] > 8 {
		targetSector[0] = startSector[0] - 1
	}
	g.CurrentQuad.Grid[targetSector[0]][targetSector[1]] = engine.EntityEmpty

	mock.Clear()
	out := s.Execute(fmt.Sprintf("nav s %d %d", targetSector[0], targetSector[1]))

	if g.Enterprise.Sector != targetSector {
		t.Fatalf("expected Enterprise sector %v, got %v", targetSector, g.Enterprise.Sector)
	}
	if g.Enterprise.Energy >= initialEnergy {
		t.Errorf("expected energy consumption, got %.0f", g.Enterprise.Energy)
	}
	if !strings.Contains(out, "<E>") {
		t.Errorf("expected SRS grid with <E> in move output, got: %s", out)
	}

	// 2. Inter-quadrant warp jump
	destQuad := engine.Coord{startQuad[0] + 1, startQuad[1]}
	if destQuad[0] > 8 {
		destQuad[0] = startQuad[0] - 1
	}

	mock.Clear()
	warpOut := s.Execute(fmt.Sprintf("nav q %d %d 6", destQuad[0], destQuad[1]))

	if g.Enterprise.Quad != destQuad {
		t.Fatalf("expected Enterprise quad %v, got %v", destQuad, g.Enterprise.Quad)
	}
	if !mock.HasSound(audio.SoundWarp) {
		t.Errorf("expected SoundWarp dispatched on warp move")
	}
	if !strings.Contains(warpOut, "[WARP ENGINES ENGAGED]") {
		t.Errorf("expected warp confirmation in output, got: %s", warpOut)
	}
}

// TestWASMJourney_Fighting tests shields, phasers, torpedoes, and Klingon counter-attacks in WASM.
func TestWASMJourney_Fighting(t *testing.T) {
	s := NewSession(12345, engine.ProfileNormal)
	mock := &mockAudioPlayer{}
	s.SetAudioPlayer(mock)

	g := s.Game()
	g.Enterprise.Sector = engine.Coord{4, 4}
	g.Enterprise.Energy = 5000
	g.Enterprise.Shields = 0
	g.Enterprise.Torpedoes = 10
	g.Enterprise.Condition = engine.ConditionRed

	// Place two Klingons
	k1 := &engine.Klingon{ID: 1, Sector: engine.Coord{4, 7}, Energy: 200}
	k2 := &engine.Klingon{ID: 2, Sector: engine.Coord{6, 2}, Energy: 350}
	g.CurrentQuad.Klingons = []*engine.Klingon{k1, k2}
	g.CurrentQuad.Grid[4][7] = engine.EntityKlingon
	g.CurrentQuad.Grid[6][2] = engine.EntityKlingon

	// 1. Raise Shields
	mock.Clear()
	sheOut := s.Execute("she 2000")
	if !strings.Contains(sheOut, "Deflector Shields:") {
		t.Errorf("expected shield output, got: %s", sheOut)
	}
	if !mock.HasSound(audio.SoundShields) {
		t.Errorf("expected SoundShields event")
	}
	// Surviving Klingons returned fire
	if !strings.Contains(sheOut, "[RETURN FIRE]") {
		t.Errorf("expected Klingon return fire in output, got: %s", sheOut)
	}

	// 2. Fire Phasers
	mock.Clear()
	phaOut := s.Execute("pha 400")
	if !strings.Contains(phaOut, "[PHASERS FIRED]") {
		t.Errorf("expected phasers fired in output, got: %s", phaOut)
	}
	if !mock.HasSound(audio.SoundPhaser) {
		t.Errorf("expected SoundPhaser event")
	}

	// 3. Fire Torpedo at Klingon 1
	mock.Clear()
	tor1Out := s.Execute("tor 4 7")
	if !mock.HasSound(audio.SoundTorpedoLaunch) {
		t.Errorf("expected SoundTorpedoLaunch event")
	}
	if !strings.Contains(tor1Out, "DESTROYED") {
		t.Errorf("expected Klingon destroyed in output, got: %s", tor1Out)
	}
	if g.CurrentQuad.Grid[4][7] == engine.EntityKlingon {
		t.Errorf("expected Klingon cleared from sector [4,7]")
	}

	// 4. Destroy Klingon 2 with Torpedo
	s.Execute("tor 6 2")
	if g.CurrentQuad.Grid[6][2] == engine.EntityKlingon {
		s.Execute("tor 6 2")
	}

	if len(g.CurrentQuad.Klingons) != 0 {
		t.Errorf("expected all Klingons in quadrant destroyed, remaining: %d", len(g.CurrentQuad.Klingons))
	}
}

// TestWASMJourney_Docking tests starbase resupply, repairs, and audio dispatch in WASM.
func TestWASMJourney_Docking(t *testing.T) {
	s := NewSession(12345, engine.ProfileNormal)
	mock := &mockAudioPlayer{}
	s.SetAudioPlayer(mock)

	g := s.Game()
	sbCoord := engine.Coord{3, 4}
	entCoord := engine.Coord{3, 5}
	g.CurrentQuad.Starbase = &sbCoord
	g.CurrentQuad.Grid[sbCoord[0]][sbCoord[1]] = engine.EntityStarbase
	g.CurrentQuad.Grid[entCoord[0]][entCoord[1]] = engine.EntityEnterprise
	g.Enterprise.Sector = entCoord

	g.Enterprise.Energy = 1200
	g.Enterprise.Torpedoes = 2
	g.Enterprise.Devices[engine.DeviceWarp] = 3.5
	g.Enterprise.Devices[engine.DevicePhasers] = 2.0

	mock.Clear()
	dockOut := s.Execute("doc")

	if !mock.HasSound(audio.SoundDock) {
		t.Errorf("expected SoundDock event")
	}
	if !strings.Contains(dockOut, "[STARBASE DOCKING]") {
		t.Errorf("expected docking confirmation in output, got: %s", dockOut)
	}
	if g.Enterprise.Condition != engine.ConditionDocked {
		t.Fatalf("expected ConditionDocked, got %v", g.Enterprise.Condition)
	}
	if g.Enterprise.Energy != 5000 {
		t.Errorf("expected energy replenished to 5000, got %.0f", g.Enterprise.Energy)
	}
	if g.Enterprise.Torpedoes != 10 {
		t.Errorf("expected torpedoes replenished to 10, got %d", g.Enterprise.Torpedoes)
	}
	if g.Enterprise.Devices[engine.DeviceWarp] != 0 || g.Enterprise.Devices[engine.DevicePhasers] != 0 {
		t.Errorf("expected devices fully repaired after docking")
	}
}

// TestWASMJourney_Scenario_KobayashiMaru tests launching Kobayashi Maru in WASM,
// wave reinforcement, and commendation evaluation.
func TestWASMJourney_Scenario_KobayashiMaru(t *testing.T) {
	s := NewSession(12345)
	out := s.Execute("scenario kobayashi-maru")

	if !strings.Contains(out, "KOBAYASHI MARU") {
		t.Fatalf("expected Kobayashi Maru briefing in output, got: %s", out)
	}
	if s.Game().Scenario != engine.ScenarioKobayashiMaru {
		t.Fatalf("expected scenario to be KobayashiMaru, got %v", s.Game().Scenario)
	}
	if len(s.Game().CurrentQuad.Klingons) != 3 {
		t.Fatalf("expected 3 initial Klingons, got %d", len(s.Game().CurrentQuad.Klingons))
	}

	// Destroy first Klingon
	k := s.Game().CurrentQuad.Klingons[0]
	sec := k.Sector
	k.Energy = 10
	s.Execute(fmt.Sprintf("tor %d %d", sec[0], sec[1]))

	// Wave reinforcement restores fleet to 3 Klingons
	if len(s.Game().CurrentQuad.Klingons) != 3 {
		t.Errorf("expected wave reinforcement to restore fleet to 3 Klingons, got %d", len(s.Game().CurrentQuad.Klingons))
	}

	// Drain energy to evaluate commendation
	s.Game().Enterprise.Energy = 0
	done, won, reason := engine.EvaluateKobayashiMaru(s.Game())
	if !done || won || reason != engine.GameOverLost {
		t.Errorf("expected defeat when energy depleted in Kobayashi Maru")
	}
	score := engine.ComputeScoreKobayashiMaru(s.Game(), false)
	if !strings.Contains(score.RankTitle, "Commendation") && !strings.Contains(score.RankTitle, "Citation") {
		t.Errorf("expected commendation title, got %q", score.RankTitle)
	}
}

// TestWASMJourney_Scenario_MutaraNebula tests Mutara Nebula duel in WASM:
// zero shields enforcement and victory upon eliminating the Super-Commander.
func TestWASMJourney_Scenario_MutaraNebula(t *testing.T) {
	s := NewSession(12345)
	out := s.Execute("scenario mutara")

	if !strings.Contains(out, "MUTARA NEBULA") {
		t.Fatalf("expected Mutara briefing in output, got: %s", out)
	}
	if s.Game().Scenario != engine.ScenarioMutaraNebula {
		t.Fatalf("expected scenario to be MutaraNebula, got %v", s.Game().Scenario)
	}
	if s.Game().Enterprise.Shields != 0 {
		t.Errorf("expected 0 shields in Mutara Nebula, got %.0f", s.Game().Enterprise.Shields)
	}

	// Shields cannot be raised
	s.Execute("she 1000")
	if s.Game().Enterprise.Shields != 0 {
		t.Errorf("expected shields to remain 0 in Mutara Nebula, got %.0f", s.Game().Enterprise.Shields)
	}

	// Eliminate Super-Commander
	if len(s.Game().CurrentQuad.Klingons) == 0 {
		t.Fatalf("expected Super-Commander in Mutara Nebula")
	}
	cmdKlingon := s.Game().CurrentQuad.Klingons[0]
	cmdSec := cmdKlingon.Sector
	cmdKlingon.Energy = 10

	torOut := s.Execute(fmt.Sprintf("tor %d %d", cmdSec[0], cmdSec[1]))
	if !strings.Contains(torOut, "MISSION ACCOMPLISHED") {
		t.Errorf("expected mission accomplished in combat output, got: %s", torOut)
	}

	done, won, reason := engine.EvaluateMutaraNebula(s.Game())
	if !done || !won || reason != engine.GameOverWon {
		t.Fatalf("expected victory after destroying commander, done=%v won=%v reason=%v", done, won, reason)
	}
}

// TestWASMJourney_Scenario_StarbaseSiege tests warping to Starbase 12 at Warp 6 and
// breaking the siege in WASM.
func TestWASMJourney_Scenario_StarbaseSiege(t *testing.T) {
	s := NewSession(12345)
	out := s.Execute("scenario siege")

	if !strings.Contains(out, "STARBASE UNDER SIEGE") {
		t.Fatalf("expected Starbase Siege briefing, got: %s", out)
	}
	if s.Game().Scenario != engine.ScenarioStarbaseSiege {
		t.Fatalf("expected scenario to be StarbaseSiege, got %v", s.Game().Scenario)
	}

	// Warp to [4, 4] at Warp 6
	s.Execute("nav q 4 4 6")
	if s.Game().Enterprise.Quad != (engine.Coord{4, 4}) {
		t.Fatalf("expected arrived in quad [4,4], got %v", s.Game().Enterprise.Quad)
	}
	if s.Game().CurrentQuad.Starbase == nil {
		t.Fatalf("expected Starbase 12 in quadrant [4,4]")
	}

	// Eliminate besieging fleet using torpedoes or phasers
	for i := 0; i < 5 && len(s.Game().CurrentQuad.Klingons) > 0; i++ {
		k := s.Game().CurrentQuad.Klingons[0]
		k.Energy = 10
		s.Game().Enterprise.Torpedoes = 10
		s.Execute(fmt.Sprintf("tor %d %d", k.Sector[0], k.Sector[1]))
	}
	// Finish any obstacle-shielded Klingons with phasers
	if len(s.Game().CurrentQuad.Klingons) > 0 {
		s.Game().Enterprise.Energy = 3000
		s.Execute("pha 500")
	}

	done, won, reason := engine.EvaluateStarbaseSiege(s.Game())
	if !done || !won || reason != engine.GameOverWon {
		t.Fatalf("expected Starbase Siege victory, got done=%v won=%v reason=%v", done, won, reason)
	}
}

// TestWASMJourney_SaveAndLoad tests state serialization and restoration in WASM.
func TestWASMJourney_SaveAndLoad(t *testing.T) {
	s1 := NewSession(12345, engine.ProfileNormal)
	s1.Execute("she 800")
	s1.Execute("nav 1 1")

	savedData, err := s1.Save()
	if err != nil {
		t.Fatalf("Save() failed: %v", err)
	}

	s2 := NewSession(99999, engine.ProfileHardcore)
	if err := s2.Load(savedData); err != nil {
		t.Fatalf("Load() failed: %v", err)
	}

	if s2.Game().Enterprise.Shields != s1.Game().Enterprise.Shields {
		t.Errorf("shields mismatch after load: got %.0f, want %.0f", s2.Game().Enterprise.Shields, s1.Game().Enterprise.Shields)
	}
	if s2.Game().Enterprise.Sector != s1.Game().Enterprise.Sector {
		t.Errorf("sector mismatch after load: got %v, want %v", s2.Game().Enterprise.Sector, s1.Game().Enterprise.Sector)
	}
}
