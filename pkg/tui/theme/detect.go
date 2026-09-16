package theme

import (
	"context"
	"os"
	"os/exec"
	"runtime"
	"strconv"
	"strings"
	"sync"
	"time"

	"github.com/charmbracelet/lipgloss"
)

var (
	osGetenv       = os.Getenv
	runtimeGOOS    = runtime.GOOS
	darwinDetector = detectDarwinTerminalDarkBackground

	darwinCacheMu   sync.Mutex
	darwinLastCheck time.Time
	darwinCachedVal bool
	darwinCacheTTL  = 1 * time.Second
)

// DetectDarkBackground returns true if the terminal has a dark background,
// or false if it has a light background.
//
// It addresses edge cases such as Apple Terminal on macOS, which does not support
// OSC 11 background queries and does not export COLORFGBG (causing termenv and
// lipgloss to falsely default to dark / black).
func DetectDarkBackground() bool {
	// 1. Explicit COLORFGBG environment variable (supported by rxvt, foot, some xterm)
	colorFGBG := osGetenv("COLORFGBG")
	if strings.Contains(colorFGBG, ";") {
		parts := strings.Split(colorFGBG, ";")
		bgStr := strings.TrimSpace(parts[len(parts)-1])
		if bg, err := strconv.Atoi(bgStr); err == nil {
			if bg == 7 || bg >= 11 {
				lipgloss.SetHasDarkBackground(false)
				return false
			}
			lipgloss.SetHasDarkBackground(true)
			return true
		}
	}

	// 2. Darwin & Apple_Terminal: Terminal.app does not support OSC 11 queries.
	if runtimeGOOS == "darwin" && osGetenv("TERM_PROGRAM") == "Apple_Terminal" {
		darwinCacheMu.Lock()
		if darwinCacheTTL > 0 && time.Since(darwinLastCheck) < darwinCacheTTL {
			val := darwinCachedVal
			darwinCacheMu.Unlock()
			lipgloss.SetHasDarkBackground(val)
			return val
		}
		darwinCacheMu.Unlock()

		isDark := darwinDetector()

		darwinCacheMu.Lock()
		darwinCachedVal = isDark
		darwinLastCheck = time.Now()
		darwinCacheMu.Unlock()

		lipgloss.SetHasDarkBackground(isDark)
		return isDark
	}

	// 3. Standard fallback: lipgloss OSC 11 query
	return lipgloss.HasDarkBackground()
}

// detectDarwinTerminalDarkBackground queries Apple Terminal or macOS system appearance.
func detectDarwinTerminalDarkBackground() bool {
	// Query Terminal.app front window background color via AppleScript
	ctx, cancel := context.WithTimeout(context.Background(), 250*time.Millisecond)
	defer cancel()

	cmd := exec.CommandContext(ctx, "osascript", "-e", `tell application "Terminal" to get background color of selected tab of front window`)
	out, err := cmd.Output()
	if err == nil {
		if isDark, ok := parseAppleScriptRGB(string(out)); ok {
			return isDark
		}
	}

	// Fallback: Check macOS system appearance
	ctxSys, cancelSys := context.WithTimeout(context.Background(), 150*time.Millisecond)
	defer cancelSys()

	cmdSys := exec.CommandContext(ctxSys, "defaults", "read", "-g", "AppleInterfaceStyle")
	outSys, errSys := cmdSys.Output()
	if errSys == nil && strings.TrimSpace(string(outSys)) == "Dark" {
		return true
	}

	// Apple Terminal's default profile ("Basic") is light
	return false
}

// parseAppleScriptRGB parses AppleScript 16-bit RGB values (e.g. "65535, 65535, 65535").
func parseAppleScriptRGB(s string) (bool, bool) {
	parts := strings.Split(strings.TrimSpace(s), ",")
	if len(parts) != 3 {
		return false, false
	}
	r, errR := strconv.ParseFloat(strings.TrimSpace(parts[0]), 64)
	g, errG := strconv.ParseFloat(strings.TrimSpace(parts[1]), 64)
	b, errB := strconv.ParseFloat(strings.TrimSpace(parts[2]), 64)
	if errR != nil || errG != nil || errB != nil {
		return false, false
	}
	lum := (0.299*r + 0.587*g + 0.114*b) / 65535.0
	return lum < 0.5, true
}
