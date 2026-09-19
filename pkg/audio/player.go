package audio

import (
	"context"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"sync"
	"time"
)

// TerminalBellPlayer emits an ASCII bell character (\a) to an io.Writer for combat
// and alert sounds, with an optional visual bell callback.
type TerminalBellPlayer struct {
	mu         sync.RWMutex
	w          io.Writer
	visualBell func()
	muted      bool
}

// NewTerminalBellPlayer creates a TerminalBellPlayer that writes \a to w and invokes
// visualBell (if non-nil) on combat/alert sounds.
func NewTerminalBellPlayer(w io.Writer, visualBell func()) *TerminalBellPlayer {
	return &TerminalBellPlayer{
		w:          w,
		visualBell: visualBell,
	}
}

// Play triggers an ASCII bell and visual bell for combat and alert sound IDs.
func (p *TerminalBellPlayer) Play(sound SoundID) {
	p.mu.RLock()
	muted := p.muted
	p.mu.RUnlock()
	if muted {
		return
	}

	switch sound {
	case SoundRedAlert, SoundDamage, SoundExplosion, SoundPhaser, SoundTorpedoLaunch:
		if p.w != nil {
			_, _ = p.w.Write([]byte("\a"))
		}
		if p.visualBell != nil {
			p.visualBell()
		}
	}
}

// SetMuted toggles audio output for this player.
func (p *TerminalBellPlayer) SetMuted(muted bool) {
	p.mu.Lock()
	defer p.mu.Unlock()
	p.muted = muted
}

// IsMuted reports whether this player is muted.
func (p *TerminalBellPlayer) IsMuted() bool {
	p.mu.RLock()
	defer p.mu.RUnlock()
	return p.muted
}

// NativeOSPlayer streams in-memory WAV buffers to OS audio commands (afplay, paplay, aplay)
// in non-blocking background goroutines, falling back to TerminalBellPlayer if no command is available.
type NativeOSPlayer struct {
	mu       sync.RWMutex
	muted    bool
	cmdPath  string
	fallback Player
}

// NewNativeOSPlayer initializes a NativeOSPlayer with automatic OS command detection
// and a TerminalBellPlayer fallback.
func NewNativeOSPlayer(w io.Writer, visualBell func()) *NativeOSPlayer {
	fallback := NewTerminalBellPlayer(w, visualBell)
	return newNativeOSPlayerWithCmd(fallback, findOSAudioPlayer())
}

func newNativeOSPlayerWithCmd(fallback Player, cmdPath string) *NativeOSPlayer {
	return &NativeOSPlayer{
		fallback: fallback,
		cmdPath:  cmdPath,
	}
}

// Play streams the synthesized WAV sound to the OS player in a detached goroutine.
func (p *NativeOSPlayer) Play(sound SoundID) {
	p.mu.RLock()
	muted := p.muted
	cmd := p.cmdPath
	fallback := p.fallback
	p.mu.RUnlock()

	if muted {
		return
	}

	if cmd == "" {
		if fallback != nil {
			fallback.Play(sound)
		}
		return
	}

	go func() {
		p.mu.RLock()
		if p.muted {
			p.mu.RUnlock()
			return
		}
		p.mu.RUnlock()

		wavData := SynthesizeWav(sound)
		tmpFile, err := os.CreateTemp("", "sst-audio-*.wav")
		if err != nil {
			if fallback != nil {
				fallback.Play(sound)
			}
			return
		}
		tmpPath := tmpFile.Name()
		defer func() {
			_ = os.Remove(tmpPath)
		}()

		if _, err := tmpFile.Write(wavData); err != nil {
			_ = tmpFile.Close()
			return
		}
		if err := tmpFile.Close(); err != nil {
			return
		}

		ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
		defer cancel()

		var execCmd *exec.Cmd
		base := filepath.Base(cmd)
		if base == "aplay" {
			execCmd = exec.CommandContext(ctx, cmd, "-q", tmpPath)
		} else {
			execCmd = exec.CommandContext(ctx, cmd, tmpPath)
		}

		if err := execCmd.Run(); err != nil && ctx.Err() == nil {
			if fallback != nil {
				fallback.Play(sound)
			}
		}
	}()
}

// SetMuted sets the mute state for the player and its fallback.
func (p *NativeOSPlayer) SetMuted(muted bool) {
	p.mu.Lock()
	p.muted = muted
	fb := p.fallback
	p.mu.Unlock()

	if fb != nil {
		fb.SetMuted(muted)
	}
}

// IsMuted reports whether the player is currently muted.
func (p *NativeOSPlayer) IsMuted() bool {
	p.mu.RLock()
	defer p.mu.RUnlock()
	return p.muted
}

func findOSAudioPlayer() string {
	if runtime.GOOS == "darwin" {
		if p, err := exec.LookPath("afplay"); err == nil {
			return p
		}
	}
	for _, name := range []string{"paplay", "aplay"} {
		if p, err := exec.LookPath(name); err == nil {
			return p
		}
	}
	return ""
}
