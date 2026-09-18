package main

import (
	"testing"
)

func TestBridgeCompilationAndMain(t *testing.T) {
	// Verify that registerBridge compiles and runs without panic on native arch
	registerBridge()
}
