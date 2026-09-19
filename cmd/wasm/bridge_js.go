//go:build js && wasm

package main

import (
	"sync"
	"syscall/js"

	"github.com/scottdensmore/super-star-trek/pkg/audio"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
)

// jsAudioPlayer implements audio.Player by delegating playback to window.sstPlaySound.
type jsAudioPlayer struct {
	mu    sync.RWMutex
	muted bool
}

func (p *jsAudioPlayer) Play(sound audio.SoundID) {
	p.mu.RLock()
	muted := p.muted
	p.mu.RUnlock()
	if muted {
		return
	}
	fn := js.Global().Get("sstPlaySound")
	if fn.Type() == js.TypeFunction {
		fn.Invoke(string(sound))
	}
}

func (p *jsAudioPlayer) SetMuted(muted bool) {
	p.mu.Lock()
	defer p.mu.Unlock()
	p.muted = muted
}

func (p *jsAudioPlayer) IsMuted() bool {
	p.mu.RLock()
	defer p.mu.RUnlock()
	return p.muted
}

var (
	activeSession     *Session
	activeAudioPlayer = &jsAudioPlayer{}
)

func registerBridge() {
	activeSession = NewSession(12345, engine.ProfileNormal)
	activeSession.SetAudioPlayer(activeAudioPlayer)

	js.Global().Set("sstInit", js.FuncOf(func(this js.Value, args []js.Value) any {
		seed := int64(0)
		diff := engine.ProfileNormal
		if len(args) > 0 && !args[0].IsNull() && !args[0].IsUndefined() {
			seed = int64(args[0].Int())
		}
		if len(args) > 1 && !args[1].IsNull() && !args[1].IsUndefined() {
			diff = engine.DifficultyProfile(args[1].String())
		}
		activeSession = NewSession(seed, diff)
		activeSession.SetAudioPlayer(activeAudioPlayer)
		return FormatSRS(activeSession.Game())
	}))

	js.Global().Set("sstCommand", js.FuncOf(func(this js.Value, args []js.Value) any {
		if len(args) == 0 {
			return ""
		}
		return activeSession.Execute(args[0].String())
	}))

	js.Global().Set("sstSave", js.FuncOf(func(this js.Value, args []js.Value) any {
		s, err := activeSession.Save()
		if err != nil {
			return ""
		}
		return s
	}))

	js.Global().Set("sstLoad", js.FuncOf(func(this js.Value, args []js.Value) any {
		if len(args) == 0 {
			return false
		}
		err := activeSession.Load(args[0].String())
		return err == nil
	}))

	js.Global().Set("sstReset", js.FuncOf(func(this js.Value, args []js.Value) any {
		activeSession = NewSession(0, engine.ProfileNormal)
		activeSession.SetAudioPlayer(activeAudioPlayer)
		return FormatSRS(activeSession.Game())
	}))
}
