package audio

import "sync"

// SoundID identifies a distinct procedural retro sound effect.
type SoundID string

const (
	SoundPhaser        SoundID = "phaser"
	SoundTorpedoLaunch SoundID = "torpedo_launch"
	SoundExplosion     SoundID = "explosion"
	SoundRedAlert      SoundID = "red_alert"
	SoundDock          SoundID = "dock"
	SoundWarp          SoundID = "warp"
	SoundDamage        SoundID = "damage"
	SoundShields       SoundID = "shields"
	SoundVictory       SoundID = "victory"
	SoundDefeat        SoundID = "defeat"
)

// Player defines the playback interface for sound effects.
type Player interface {
	Play(sound SoundID)
	SetMuted(muted bool)
	IsMuted() bool
}

// NullPlayer provides a no-op implementation of Player.
type NullPlayer struct {
	mu    sync.RWMutex
	muted bool
}

// NewNullPlayer returns a newly initialized NullPlayer.
func NewNullPlayer() *NullPlayer {
	return &NullPlayer{}
}

// Play does nothing.
func (p *NullPlayer) Play(_ SoundID) {}

// SetMuted sets whether the player is muted.
func (p *NullPlayer) SetMuted(muted bool) {
	p.mu.Lock()
	defer p.mu.Unlock()
	p.muted = muted
}

// IsMuted reports whether the player is muted.
func (p *NullPlayer) IsMuted() bool {
	p.mu.RLock()
	defer p.mu.RUnlock()
	return p.muted
}
