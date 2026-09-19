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
	osGetenv                 = os.Getenv
	runtimeGOOS              = runtime.GOOS
	darwinDetector           = detectDarwinTerminalDarkBackground
	oscDetector              = queryOSCDarkBackground
	detectDarkBackgroundImpl = detectDarkBackgroundInternal

	detectCacheMu   sync.Mutex
	detectLastCheck time.Time
	detectCachedVal bool
	detectHasCached bool
	detectCacheTTL  = 2 * time.Second

	// Preserved for backward compatibility with existing tests
	darwinCacheMu   sync.Mutex
	darwinLastCheck time.Time
	darwinCachedVal bool
	darwinCacheTTL  = 1 * time.Second
)

// DetectDarkBackground returns true if the terminal has a dark background,
// or false if it has a light background.
//
// It addresses edge cases such as Apple Terminal on macOS, which does not support
// OSC 11 background queries and does not export COLORFGBG, and avoids blocking
// 5-second OSC timeouts in tmux or remote SSH sessions.
func DetectDarkBackground() bool {
	return detectDarkBackgroundImpl()
}

// ResetDetectionCache clears the global background detection cache.
func ResetDetectionCache() {
	detectCacheMu.Lock()
	detectHasCached = false
	detectLastCheck = time.Time{}
	detectCacheMu.Unlock()
}

func detectDarkBackgroundInternal() bool {
	detectCacheMu.Lock()
	if detectHasCached && detectCacheTTL > 0 && time.Since(detectLastCheck) < detectCacheTTL {
		val := detectCachedVal
		detectCacheMu.Unlock()
		lipgloss.SetHasDarkBackground(val)
		return val
	}
	detectCacheMu.Unlock()

	val := detectDarkBackgroundUncached()

	detectCacheMu.Lock()
	detectCachedVal = val
	detectHasCached = true
	detectLastCheck = time.Now()
	detectCacheMu.Unlock()

	lipgloss.SetHasDarkBackground(val)
	return val
}

func detectDarkBackgroundUncached() bool {
	// 1. Explicit COLORFGBG environment variable (supported by rxvt, foot, some xterm)
	colorFGBG := osGetenv("COLORFGBG")
	if strings.Contains(colorFGBG, ";") {
		parts := strings.Split(colorFGBG, ";")
		bgStr := strings.TrimSpace(parts[len(parts)-1])
		if bg, err := strconv.Atoi(bgStr); err == nil {
			if bg == 7 || bg >= 11 {
				return false
			}
			return true
		}
	}

	// 2. Darwin & Apple_Terminal: Terminal.app does not support OSC 11 queries.
	if runtimeGOOS == "darwin" && osGetenv("TERM_PROGRAM") == "Apple_Terminal" {
		darwinCacheMu.Lock()
		if darwinCacheTTL > 0 && time.Since(darwinLastCheck) < darwinCacheTTL {
			val := darwinCachedVal
			darwinCacheMu.Unlock()
			return val
		}
		darwinCacheMu.Unlock()

		isDark := darwinDetector()

		darwinCacheMu.Lock()
		darwinCachedVal = isDark
		darwinLastCheck = time.Now()
		darwinCacheMu.Unlock()

		return isDark
	}

	// 3. Fast OSC 11 terminal query (50ms timeout)
	if isDark, ok := oscDetector(50 * time.Millisecond); ok {
		return isDark
	}

	// 4. Heuristics when OSC 11 is unsupported or unanswered (e.g. Apple Terminal over SSH, tmux without passthrough)
	if osGetenv("TERM_PROGRAM") == "Apple_Terminal" {
		return false
	}

	// Standard default: dark background
	return true
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
