package audio

import (
	"testing"

	"github.com/scottdensmore/super-star-trek/pkg/engine"
)

type mockPlayer struct {
	played []SoundID
	muted  bool
}

func (m *mockPlayer) Play(s SoundID) {
	if !m.muted {
		m.played = append(m.played, s)
	}
}
func (m *mockPlayer) SetMuted(muted bool) { m.muted = muted }
func (m *mockPlayer) IsMuted() bool       { return m.muted }

func TestDispatcher_EventMapping(t *testing.T) {
	p := &mockPlayer{}
	d := NewDispatcher(p)

	events := []engine.Event{
		engine.EventPhaserFired{Energy: 500},
		engine.EventTorpedoFired{Origin: engine.Coord{1, 1}, Angle: 1.57},
		engine.EventPhaserHit{Target: engine.Coord{2, 2}, Damage: 100, Destroyed: false},
		engine.EventTorpedoHit{Target: engine.Coord{3, 3}, Damage: 200, Destroyed: true},
		engine.EventKlingonCounterAttack{EnemyID: 1, Damage: 50},
		engine.EventSubsystemDamaged{Device: engine.DeviceShields, RepairTime: 2.0},
		engine.EventShieldTransfer{NewShields: 500, NewEnergy: 1000},
		engine.EventDocked{Starbase: engine.Coord{4, 4}},
		engine.EventConditionChanged{From: engine.ConditionGreen, To: engine.ConditionRed},
		engine.EventAnomalyDiscovered{Quad: engine.Coord{5, 5}, Env: engine.EnvNebula},
		engine.EventWormholeJump{FromQuad: engine.Coord{1, 1}, ToQuad: engine.Coord{8, 8}},
		engine.EventShipMoved{FromQuad: engine.Coord{1, 1}, ToQuad: engine.Coord{2, 2}, Warp: 2.0},
		engine.EventGameOver{Reason: engine.GameOverWon, Score: 5000},
		engine.EventGameOver{Reason: engine.GameOverDestroyed, Score: 100},
	}

	d.DispatchEvents(events)

	expected := []SoundID{
		SoundPhaser,
		SoundTorpedoLaunch,
		SoundExplosion,
		SoundExplosion,
		SoundDamage,
		SoundDamage,
		SoundShields,
		SoundDock,
		SoundRedAlert,
		SoundRedAlert,
		SoundWarp,
		SoundWarp,
		SoundVictory,
		SoundDefeat,
	}

	if len(p.played) != len(expected) {
		t.Fatalf("expected %d sounds, got %d: %v", len(expected), len(p.played), p.played)
	}
	for i, exp := range expected {
		if p.played[i] != exp {
			t.Errorf("sound %d: expected %s, got %s", i, exp, p.played[i])
		}
	}
}

func TestDispatcher_NilPlayer(t *testing.T) {
	d := NewDispatcher(nil)
	// Should not panic on nil player
	d.DispatchEvents([]engine.Event{
		engine.EventPhaserFired{Energy: 500},
	})
}

func TestDispatcher_MutedPlayer(t *testing.T) {
	p := &mockPlayer{muted: true}
	d := NewDispatcher(p)

	d.DispatchEvents([]engine.Event{
		engine.EventPhaserFired{Energy: 500},
		engine.EventDocked{Starbase: engine.Coord{1, 1}},
	})

	if len(p.played) != 0 {
		t.Errorf("expected 0 sounds when muted, got %d", len(p.played))
	}
}

func TestDispatcher_IgnoredEvents(t *testing.T) {
	p := &mockPlayer{}
	d := NewDispatcher(p)

	d.DispatchEvents([]engine.Event{
		engine.EventSubsystemRepaired{Device: engine.DeviceShields},
		engine.EventConditionChanged{From: engine.ConditionYellow, To: engine.ConditionGreen},
		engine.EventPhaserHit{Target: engine.Coord{1, 1}, Damage: 0, Destroyed: false},
		engine.EventTorpedoHit{Target: engine.Coord{1, 1}, Damage: 0, Destroyed: false},
		engine.EventKlingonCounterAttack{EnemyID: 1, Damage: 0},
	})

	if len(p.played) != 0 {
		t.Errorf("expected 0 sounds for non-audio events, got %d: %v", len(p.played), p.played)
	}
}
