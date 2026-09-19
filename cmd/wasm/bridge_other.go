//go:build !(js && wasm)

package main

// registerBridge is a no-op when compiled for non-js/wasm architectures.
// Audio player bridging is handled via standard audio players or NullPlayer on non-JS targets.
func registerBridge() {}
