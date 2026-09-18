//go:build !(js && wasm)

package main

// registerBridge is a no-op when compiled for non-js/wasm architectures.
func registerBridge() {}
