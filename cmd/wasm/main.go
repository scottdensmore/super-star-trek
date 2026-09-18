package main

func main() {
	registerBridge()

	// Keep runtime active in WebAssembly
	select {}
}
