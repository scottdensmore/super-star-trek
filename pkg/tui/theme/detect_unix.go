//go:build !windows && !js && !wasm

package theme

import (
	"os"
	"strconv"
	"strings"
	"time"

	"github.com/charmbracelet/x/term"
	"golang.org/x/sys/unix"
)

// queryOSCDarkBackground sends an OSC 11 background color query to the terminal
// with a short timeout. It returns (isDark, true) on successful response,
// or (false, false) if the terminal does not support OSC 11 or timed out.
func queryOSCDarkBackground(timeout time.Duration) (bool, bool) {
	f, err := os.OpenFile("/dev/tty", os.O_RDWR, 0)
	if err != nil {
		return false, false
	}
	defer func() {
		_ = f.Close()
	}()

	fd := int(f.Fd())

	// Save terminal state and temporarily set raw mode so we can read the response
	oldState, err := term.MakeRaw(uintptr(fd))
	if err != nil {
		return false, false
	}
	defer func() {
		_ = term.Restore(uintptr(fd), oldState)
	}()

	// Send OSC 11 query
	if _, err := f.WriteString("\x1b]11;?\x1b\\"); err != nil {
		return false, false
	}

	tv := unix.NsecToTimeval(int64(timeout))
	var readfds unix.FdSet
	readfds.Set(fd)

	n, err := unix.Select(fd+1, &readfds, nil, nil, &tv)
	if err != nil || n <= 0 {
		return false, false
	}

	buf := make([]byte, 128)
	nr, err := f.Read(buf)
	if err != nil || nr == 0 {
		return false, false
	}

	return parseOSC11Response(string(buf[:nr]))
}

// parseOSC11Response parses an OSC 11 terminal response like "\x1b]11;rgb:rrrr/gggg/bbbb\x1b\\".
func parseOSC11Response(s string) (bool, bool) {
	idx := strings.Index(s, "rgb:")
	if idx == -1 {
		return false, false
	}
	s = s[idx+4:]
	s = strings.TrimRight(s, "\x1b\\\a\r\n ")

	parts := strings.Split(s, "/")
	if len(parts) != 3 {
		return false, false
	}

	r, errR := parseHexComponent(parts[0])
	g, errG := parseHexComponent(parts[1])
	b, errB := parseHexComponent(parts[2])
	if errR != nil || errG != nil || errB != nil {
		return false, false
	}

	lum := 0.299*r + 0.587*g + 0.114*b
	return lum < 0.5, true
}

func parseHexComponent(hexStr string) (float64, error) {
	if len(hexStr) == 0 {
		return 0, strconv.ErrSyntax
	}
	val, err := strconv.ParseUint(hexStr, 16, 64)
	if err != nil {
		return 0, err
	}
	maxVal := float64((uint64(1) << (len(hexStr) * 4)) - 1)
	if maxVal <= 0 {
		return 0, strconv.ErrSyntax
	}
	return float64(val) / maxVal, nil
}
