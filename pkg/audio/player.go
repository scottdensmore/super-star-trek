package audio

import (
	"context"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"sync"
	"testing"
	"time"
)

// TerminalBellPlayer emits an ASCII bell character (\a) to an io.Writer for combat
// and alert sounds, with an optional visual bell callback.
type TerminalBellPlayer struct {
	mu         sync.RWMutex
	w          io.Writer
	visualBell func()
	muted      bool
	volume     int
}

// NewTerminalBellPlayer creates a TerminalBellPlayer that writes \a to w and invokes
// visualBell (if non-nil) on combat/alert sounds.
func NewTerminalBellPlayer(w io.Writer, visualBell func()) *TerminalBellPlayer {
	return &TerminalBellPlayer{
		w:          w,
		visualBell: visualBell,
		volume:     80,
	}
}

// Play triggers an ASCII bell and visual bell for combat and alert sound IDs.
func (p *TerminalBellPlayer) Play(sound SoundID) {
	p.mu.Lock()
	defer p.mu.Unlock()
	if p.muted || p.volume == 0 {
		return
	}

	switch sound {
	case SoundRedAlert, SoundDamage, SoundExplosion, SoundPhaser, SoundTorpedoLaunch:
		if p.w != nil && (!testing.Testing() || (p.w != os.Stdout && p.w != os.Stderr)) {
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

// SetVolume sets the volume (0-100) for this player.
func (p *TerminalBellPlayer) SetVolume(vol int) {
	p.mu.Lock()
	defer p.mu.Unlock()
	if vol < 0 {
		vol = 0
	} else if vol > 100 {
		vol = 100
	}
	p.volume = vol
}

// Volume reports the current volume (0-100) of this player.
func (p *TerminalBellPlayer) Volume() int {
	p.mu.RLock()
	defer p.mu.RUnlock()
	return p.volume
}

var (
	wavFileCache   = make(map[wavCacheKey]string)
	wavFileCacheMu sync.RWMutex
)

func synthesizeOSPlaybackWav(sound SoundID, vol int) []byte {
	if vol < 0 {
		vol = 0
	} else if vol > 100 {
		vol = 100
	}
	raw := rawSamples(sound)
	// Pad 60ms (480 samples at 8kHz) of trailing silence so OS audio players
	// (macOS afplay / CoreAudio AudioQueue) drain the full waveform before closing.
	tailPad := sampleRate * 60 / 1000
	scaled := make([]int16, len(raw)+tailPad)
	scale := float64(vol) / 100.0
	for i, s := range raw {
		v := float64(s) * scale
		if v > 32767.0 {
			v = 32767.0
		} else if v < -32768.0 {
			v = -32768.0
		}
		scaled[i] = int16(v)
	}
	return encodeWAV(scaled)
}

func getOrWriteWavFile(sound SoundID, vol int) (string, bool, error) {
	key := wavCacheKey{id: sound, volume: vol}
	wavFileCacheMu.RLock()
	if path, ok := wavFileCache[key]; ok {
		if _, err := os.Stat(path); err == nil {
			wavFileCacheMu.RUnlock()
			return path, false, nil
		}
	}
	wavFileCacheMu.RUnlock()

	wavData := synthesizeOSPlaybackWav(sound, vol)
	tmpFile, err := os.CreateTemp("", "sst-audio-*.wav")
	if err != nil {
		return "", false, err
	}
	tmpPath := tmpFile.Name()
	if _, err := tmpFile.Write(wavData); err != nil {
		_ = tmpFile.Close()
		_ = os.Remove(tmpPath)
		return "", false, err
	}
	if err := tmpFile.Close(); err != nil {
		_ = os.Remove(tmpPath)
		return "", false, err
	}

	wavFileCacheMu.Lock()
	if existing, ok := wavFileCache[key]; ok {
		if _, err := os.Stat(existing); err == nil {
			wavFileCacheMu.Unlock()
			_ = os.Remove(tmpPath)
			return existing, false, nil
		}
	}
	wavFileCache[key] = tmpPath
	wavFileCacheMu.Unlock()

	return tmpPath, false, nil
}

func isPrimaryActionSound(sound SoundID) bool {
	switch sound {
	case SoundWarp, SoundPhaser, SoundTorpedoLaunch, SoundShields, SoundDock, SoundCommChime, SoundComputerBeep:
		return true
	default:
		return false
	}
}

// NativeOSPlayer streams in-memory WAV buffers to OS audio commands (afplay, paplay, aplay)
// sequentially in a background worker so consecutive turn sounds (phaser -> explosion -> Klingon return fire)
// play in strict order without overlapping.
type NativeOSPlayer struct {
	mu         sync.RWMutex
	muted      bool
	volume     int
	cmdPath    string
	fallback   Player
	lastSound  SoundID
	lastTime   time.Time
	queue      chan SoundID
	workerOnce sync.Once
}

// NewNativeOSPlayer initializes a NativeOSPlayer with automatic OS command detection
// and a TerminalBellPlayer fallback.
func NewNativeOSPlayer(w io.Writer, visualBell func()) *NativeOSPlayer {
	var fallback Player = NewTerminalBellPlayer(w, visualBell)
	if testing.Testing() {
		fallback = NewNullPlayer()
	}
	p := newNativeOSPlayerWithCmd(fallback, findOSAudioPlayer())
	if !testing.Testing() && isOSAudioBinary(p.cmdPath) {
		go func() {
			for _, id := range []SoundID{SoundWarp, SoundPhaser, SoundTorpedoLaunch, SoundShields, SoundExplosion, SoundDamage} {
				_, _, _ = getOrWriteWavFile(id, 80)
			}
		}()
	}
	return p
}

func newNativeOSPlayerWithCmd(fallback Player, cmdPath string) *NativeOSPlayer {
	return &NativeOSPlayer{
		fallback: fallback,
		cmdPath:  cmdPath,
		volume:   80,
		queue:    make(chan SoundID, 8),
	}
}

func isOSAudioBinary(cmd string) bool {
	base := filepath.Base(cmd)
	return base == "afplay" || base == "aplay" || base == "paplay"
}

func (p *NativeOSPlayer) startWorker() {
	go func() {
		for sound := range p.queue {
			p.playOne(sound)
			if len(p.queue) > 0 {
				time.Sleep(45 * time.Millisecond)
			}
		}
	}()
}

func (p *NativeOSPlayer) playOne(sound SoundID) {
	p.mu.RLock()
	if p.muted || p.volume == 0 {
		p.mu.RUnlock()
		return
	}
	vol := p.volume
	cmd := p.cmdPath
	fallback := p.fallback
	p.mu.RUnlock()

	tmpPath, removeAfter, err := getOrWriteWavFile(sound, vol)
	if err != nil {
		if fallback != nil {
			fallback.Play(sound)
		}
		return
	}
	if removeAfter {
		defer func() {
			_ = os.Remove(tmpPath)
		}()
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
}

// Play streams the synthesized WAV sound to the OS player in sequential order.
func (p *NativeOSPlayer) Play(sound SoundID) {
	p.mu.RLock()
	muted := p.muted
	volume := p.volume
	cmd := p.cmdPath
	fallback := p.fallback
	p.mu.RUnlock()

	if muted || volume == 0 {
		return
	}

	if cmd == "" {
		if fallback != nil {
			fallback.Play(sound)
		}
		return
	}

	// Never execute OS audio commands (afplay, aplay, paplay) in automated unit/e2e tests.
	// Only human interactive play should execute audio binaries.
	if testing.Testing() && isOSAudioBinary(cmd) {
		return
	}

	if !testing.Testing() && isOSAudioBinary(cmd) {
		now := time.Now()
		p.mu.Lock()
		// Avoid stacking duplicate plays for the exact same sound within the same turn
		if sound == p.lastSound && now.Sub(p.lastTime) < 80*time.Millisecond {
			p.mu.Unlock()
			return
		}
		p.lastSound = sound
		p.lastTime = now
		// If the player initiates a new primary action, clear any stale queued backlog from a prior turn
		if isPrimaryActionSound(sound) {
			for len(p.queue) > 0 {
				select {
				case <-p.queue:
				default:
				}
			}
		}
		p.mu.Unlock()

		p.workerOnce.Do(p.startWorker)
		select {
		case p.queue <- sound:
		default:
		}
		return
	}

	go p.playOne(sound)
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

// SetVolume sets the volume (0-100) for the player and its fallback.
func (p *NativeOSPlayer) SetVolume(vol int) {
	if vol < 0 {
		vol = 0
	} else if vol > 100 {
		vol = 100
	}
	p.mu.Lock()
	p.volume = vol
	fb := p.fallback
	p.mu.Unlock()

	if fb != nil {
		fb.SetVolume(vol)
	}
}

// Volume reports the current volume of the player.
func (p *NativeOSPlayer) Volume() int {
	p.mu.RLock()
	defer p.mu.RUnlock()
	return p.volume
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
