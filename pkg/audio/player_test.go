package audio

import (
	"bytes"
	"sync"
	"testing"
)

func TestNullPlayer(t *testing.T) {
	p := NewNullPlayer()
	if p.IsMuted() {
		t.Errorf("expected NullPlayer to initially be unmuted")
	}
	p.Play(SoundPhaser)
	p.SetMuted(true)
	if !p.IsMuted() {
		t.Errorf("expected NullPlayer to be muted")
	}
	p.Play(SoundExplosion)
}

func TestTerminalBellPlayer_Sounds(t *testing.T) {
	bellSounds := []SoundID{
		SoundRedAlert,
		SoundDamage,
		SoundExplosion,
		SoundPhaser,
		SoundTorpedoLaunch,
	}

	for _, s := range bellSounds {
		t.Run(string(s), func(t *testing.T) {
			var buf bytes.Buffer
			calledVisual := false
			p := NewTerminalBellPlayer(&buf, func() {
				calledVisual = true
			})

			p.Play(s)

			if buf.String() != "\a" {
				t.Errorf("expected \\a in buffer, got %q", buf.String())
			}
			if !calledVisual {
				t.Errorf("expected visual bell callback to be invoked for %s", s)
			}
		})
	}
}

func TestTerminalBellPlayer_IgnoredSounds(t *testing.T) {
	nonBellSounds := []SoundID{
		SoundDock,
		SoundWarp,
		SoundShields,
		SoundVictory,
		SoundDefeat,
	}

	for _, s := range nonBellSounds {
		t.Run(string(s), func(t *testing.T) {
			var buf bytes.Buffer
			calledVisual := false
			p := NewTerminalBellPlayer(&buf, func() {
				calledVisual = true
			})

			p.Play(s)

			if buf.Len() != 0 {
				t.Errorf("expected empty buffer for non-bell sound %s, got %q", s, buf.String())
			}
			if calledVisual {
				t.Errorf("visual bell should not be invoked for %s", s)
			}
		})
	}
}

func TestTerminalBellPlayer_Muted(t *testing.T) {
	var buf bytes.Buffer
	calledVisual := false
	p := NewTerminalBellPlayer(&buf, func() {
		calledVisual = true
	})

	p.SetMuted(true)
	if !p.IsMuted() {
		t.Errorf("expected player to be muted")
	}

	p.Play(SoundRedAlert)
	if buf.Len() != 0 {
		t.Errorf("expected no bell output when muted")
	}
	if calledVisual {
		t.Errorf("expected no visual bell when muted")
	}
}

func TestNewNativeOSPlayer(t *testing.T) {
	var buf bytes.Buffer
	p := NewNativeOSPlayer(&buf, nil)
	if p == nil {
		t.Fatal("expected non-nil NativeOSPlayer")
	}
	if p.IsMuted() {
		t.Errorf("expected player not to be muted initially")
	}
	p.Play(SoundRedAlert)
}

func TestNativeOSPlayer_FallbackWhenNoCmd(t *testing.T) {
	var buf bytes.Buffer
	calledVisual := false
	fallback := NewTerminalBellPlayer(&buf, func() {
		calledVisual = true
	})

	// Create NativeOSPlayer with an empty/invalid command to force fallback
	p := newNativeOSPlayerWithCmd(fallback, "")

	if p.IsMuted() {
		t.Errorf("expected NativeOSPlayer to be unmuted by default")
	}

	p.Play(SoundRedAlert)

	if buf.String() != "\a" {
		t.Errorf("expected fallback player to trigger \\a, got %q", buf.String())
	}
	if !calledVisual {
		t.Errorf("expected fallback visual bell callback to trigger")
	}

	// Mute both
	p.SetMuted(true)
	if !p.IsMuted() {
		t.Errorf("expected NativeOSPlayer to be muted")
	}
	buf.Reset()
	calledVisual = false
	p.Play(SoundRedAlert)
	if buf.Len() != 0 || calledVisual {
		t.Errorf("expected no sound when NativeOSPlayer is muted")
	}
}

func TestNativeOSPlayer_CustomCmd(t *testing.T) {
	var buf bytes.Buffer
	fallback := NewTerminalBellPlayer(&buf, nil)

	// Use "true" which is available on all POSIX platforms and immediately exits 0
	p := newNativeOSPlayerWithCmd(fallback, "true")
	p.Play(SoundPhaser)

	// Verify mute
	p.SetMuted(true)
	p.Play(SoundPhaser)
}

func TestNativeOSPlayer_Concurrency(t *testing.T) {
	var buf bytes.Buffer
	fallback := NewTerminalBellPlayer(&buf, nil)
	p := newNativeOSPlayerWithCmd(fallback, "true")

	var wg sync.WaitGroup
	for i := 0; i < 20; i++ {
		wg.Add(3)
		go func(idx int) {
			defer wg.Done()
			p.Play(SoundPhaser)
			p.Play(SoundExplosion)
		}(i)
		go func(idx int) {
			defer wg.Done()
			p.SetMuted(idx%2 == 0)
			_ = p.IsMuted()
		}(i)
		go func(idx int) {
			defer wg.Done()
			p.SetVolume(idx * 5)
			_ = p.Volume()
		}(i)
	}
	wg.Wait()
}

func TestPlayerVolumeInterface(t *testing.T) {
	np := NewNullPlayer()
	if np.Volume() != 100 {
		t.Errorf("expected default NullPlayer volume 100, got %d", np.Volume())
	}
	np.SetVolume(60)
	if np.Volume() != 60 {
		t.Errorf("expected volume 60, got %d", np.Volume())
	}

	var buf bytes.Buffer
	tb := NewTerminalBellPlayer(&buf, nil)
	if tb.Volume() != 80 {
		t.Errorf("expected default TerminalBellPlayer volume 80, got %d", tb.Volume())
	}
	tb.SetVolume(40)
	if tb.Volume() != 40 {
		t.Errorf("expected volume 40, got %d", tb.Volume())
	}

	nos := NewNativeOSPlayer(&buf, nil)
	if nos.Volume() != 80 {
		t.Errorf("expected default NativeOSPlayer volume 80, got %d", nos.Volume())
	}
	nos.SetVolume(50)
	if nos.Volume() != 50 {
		t.Errorf("expected volume 50, got %d", nos.Volume())
	}
}

func TestAutomatedTestSilenceGuarantees(t *testing.T) {
	// In test mode, NativeOSPlayer should never execute external OS binaries
	var buf bytes.Buffer
	p := NewNativeOSPlayer(&buf, nil)
	p.Play(SoundRedAlert)
	// buffer should remain clean
	if buf.Len() > 0 {
		t.Errorf("expected 0 bytes written to buffer in test mode, got %d bytes", buf.Len())
	}
}

func TestPlayerVolumeClamping(t *testing.T) {
	var buf bytes.Buffer
	players := []Player{
		NewNullPlayer(),
		NewTerminalBellPlayer(&buf, nil),
		NewNativeOSPlayer(&buf, nil),
	}

	for _, p := range players {
		p.SetVolume(-10)
		if p.Volume() != 0 {
			t.Errorf("%T: expected volume clamped to 0, got %d", p, p.Volume())
		}
		p.SetVolume(150)
		if p.Volume() != 100 {
			t.Errorf("%T: expected volume clamped to 100, got %d", p, p.Volume())
		}
	}
}

func TestTerminalBellPlayer_ZeroVolumeSilence(t *testing.T) {
	var buf bytes.Buffer
	calledVisual := false
	tb := NewTerminalBellPlayer(&buf, func() {
		calledVisual = true
	})
	tb.SetVolume(0)
	tb.Play(SoundRedAlert)

	if buf.Len() != 0 {
		t.Errorf("expected zero bytes in buffer when volume is 0, got %q", buf.String())
	}
	if calledVisual {
		t.Errorf("visual bell should not be called when volume is 0")
	}
}
