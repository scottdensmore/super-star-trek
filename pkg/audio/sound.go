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

	SoundCloak         SoundID = "cloak"
	SoundDecloak       SoundID = "decloak"
	SoundPlasmaLaunch  SoundID = "plasma_launch"
	SoundPlasmaImpact  SoundID = "plasma_impact"
	SoundTholianWeb    SoundID = "tholian_web"
	SoundWebBreached   SoundID = "web_breached"
	SoundPointDefense  SoundID = "point_defense"
	SoundCommChime     SoundID = "comm_chime"
	SoundComputerBeep  SoundID = "computer_beep"
)

// Player defines the playback interface for sound effects.
type Player interface {
	Play(sound SoundID)
	SetMuted(muted bool)
	IsMuted() bool
	SetVolume(vol int)
	Volume() int
}

// NullPlayer provides a no-op implementation of Player.
type NullPlayer struct {
	mu     sync.RWMutex
	muted  bool
	volume int
}

// NewNullPlayer returns a newly initialized NullPlayer.
func NewNullPlayer() *NullPlayer {
	return &NullPlayer{
		volume: 100,
	}
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

// SetVolume sets the playback volume (clamped to 0-100).
func (p *NullPlayer) SetVolume(vol int) {
	p.mu.Lock()
	defer p.mu.Unlock()
	if vol < 0 {
		vol = 0
	} else if vol > 100 {
		vol = 100
	}
	p.volume = vol
}

// Volume reports the current playback volume (0-100).
func (p *NullPlayer) Volume() int {
	p.mu.RLock()
	defer p.mu.RUnlock()
	return p.volume
}
