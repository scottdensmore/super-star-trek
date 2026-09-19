//go:build js && wasm

package main

import (
	"syscall/js"
	"testing"

	"github.com/scottdensmore/super-star-trek/pkg/audio"
)

func TestJSAudioPlayer_FallbackWithoutWindowFunctions(t *testing.T) {
	player := &jsAudioPlayer{}

	// Initially unmuted
	if player.IsMuted() {
		t.Errorf("expected initially unmuted")
	}

	player.SetMuted(true)
	if !player.IsMuted() {
		t.Errorf("expected muted after SetMuted(true)")
	}

	player.SetMuted(false)
	if player.IsMuted() {
		t.Errorf("expected unmuted after SetMuted(false)")
	}
}

func TestJSAudioPlayer_SyncWithWindowMuteFunctions(t *testing.T) {
	player := &jsAudioPlayer{}

	jsMuted := false
	setMutedCalledWith := false

	setMutedFn := js.FuncOf(func(this js.Value, args []js.Value) any {
		if len(args) > 0 {
			jsMuted = args[0].Bool()
			setMutedCalledWith = jsMuted
		}
		return nil
	})
	defer setMutedFn.Release()

	isMutedFn := js.FuncOf(func(this js.Value, args []js.Value) any {
		return jsMuted
	})
	defer isMutedFn.Release()

	js.Global().Set("sstSetMuted", setMutedFn)
	js.Global().Set("sstIsMuted", isMutedFn)
	defer func() {
		js.Global().Delete("sstSetMuted")
		js.Global().Delete("sstIsMuted")
	}()

	// SetMuted should invoke window.sstSetMuted
	player.SetMuted(true)
	if !setMutedCalledWith || !jsMuted {
		t.Errorf("expected sstSetMuted called with true, got %v (jsMuted: %v)", setMutedCalledWith, jsMuted)
	}
	if !player.IsMuted() {
		t.Errorf("expected IsMuted() to reflect sstIsMuted true")
	}

	// External change to jsMuted should be reflected by player.IsMuted()
	jsMuted = false
	if player.IsMuted() {
		t.Errorf("expected player.IsMuted() to return false when sstIsMuted returns false")
	}

	player.SetMuted(false)
	if player.IsMuted() {
		t.Errorf("expected player.IsMuted() false")
	}
}

func TestJSAudioPlayer_PlayDelegation(t *testing.T) {
	player := &jsAudioPlayer{}

	var playedSounds []string
	playSoundFn := js.FuncOf(func(this js.Value, args []js.Value) any {
		if len(args) > 0 {
			playedSounds = append(playedSounds, args[0].String())
		}
		return nil
	})
	defer playSoundFn.Release()

	js.Global().Set("sstPlaySound", playSoundFn)
	defer js.Global().Delete("sstPlaySound")

	player.Play(audio.SoundPhaser)
	if len(playedSounds) != 1 || playedSounds[0] != string(audio.SoundPhaser) {
		t.Fatalf("expected sstPlaySound called with %s, got: %v", audio.SoundPhaser, playedSounds)
	}

	// When muted, Play should not invoke sstPlaySound
	player.SetMuted(true)
	player.Play(audio.SoundExplosion)
	if len(playedSounds) != 1 {
		t.Fatalf("expected no additional sounds played when muted, got: %v", playedSounds)
	}
}
