//go:build js && wasm

package main

import (
	"syscall/js"

	"github.com/scottdensmore/super-star-trek/pkg/engine"
)

var activeSession *Session

func registerBridge() {
	activeSession = NewSession(12345, engine.ProfileNormal)

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
		return FormatSRS(activeSession.Game())
	}))
}
