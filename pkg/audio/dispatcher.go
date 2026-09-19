package audio

import "github.com/scottdensmore/super-star-trek/pkg/engine"

// Dispatcher maps game engine domain events to retro audio SoundIDs.
type Dispatcher struct {
	player Player
}

// NewDispatcher creates a new audio event Dispatcher.
func NewDispatcher(player Player) *Dispatcher {
	if player == nil {
		player = NewNullPlayer()
	}
	return &Dispatcher{player: player}
}

// DispatchEvents processes a slice of engine events and plays corresponding sounds.
func (d *Dispatcher) DispatchEvents(events []engine.Event) {
	if d.player == nil || d.player.IsMuted() {
		return
	}

	for _, event := range events {
		switch e := event.(type) {
		case engine.EventPhaserFired:
			d.player.Play(SoundPhaser)

		case engine.EventTorpedoFired:
			d.player.Play(SoundTorpedoLaunch)

		case engine.EventPhaserHit:
			if e.Destroyed || e.Damage > 0 {
				d.player.Play(SoundExplosion)
			}

		case engine.EventTorpedoHit:
			if e.Destroyed || e.Damage > 0 {
				d.player.Play(SoundExplosion)
			}

		case engine.EventKlingonCounterAttack:
			if e.Damage > 0 {
				d.player.Play(SoundDamage)
			}

		case engine.EventSubsystemDamaged:
			d.player.Play(SoundDamage)

		case engine.EventShieldTransfer:
			d.player.Play(SoundShields)

		case engine.EventDocked:
			d.player.Play(SoundDock)

		case engine.EventConditionChanged:
			if e.To == engine.ConditionRed {
				d.player.Play(SoundRedAlert)
			}

		case engine.EventAnomalyDiscovered:
			d.player.Play(SoundRedAlert)

		case engine.EventWormholeJump:
			d.player.Play(SoundWarp)

		case engine.EventShipMoved:
			d.player.Play(SoundWarp)

		case engine.EventGameOver:
			if e.Reason == engine.GameOverWon {
				d.player.Play(SoundVictory)
			} else {
				d.player.Play(SoundDefeat)
			}
		}
	}
}
