//go:build windows || js || wasm

package theme

import "time"

func queryOSCDarkBackground(timeout time.Duration) (bool, bool) {
	return false, false
}
