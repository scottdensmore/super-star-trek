package theme

import (
	"context"
	"errors"
	"sync"
	"testing"
	"time"
)

func TestParseAppleScriptRGB(t *testing.T) {
	tests := []struct {
		input        string
		expectedDark bool
		expectedOk   bool
	}{
		{"65535, 65535, 65535", false, true}, // Pure White
		{"0, 0, 0", true, true},              // Pure Black
		{"50000, 50000, 50000", false, true}, // Light gray
		{"10000, 10000, 10000", true, true},  // Dark gray
		{"65535, 65535", false, false},       // Too few parts
		{"a, b, c", false, false},            // Non-numeric
		{"", false, false},                   // Empty
	}

	for _, tt := range tests {
		dark, ok := parseAppleScriptRGB(tt.input)
		if ok != tt.expectedOk {
			t.Errorf("parseAppleScriptRGB(%q) ok = %v, expected %v", tt.input, ok, tt.expectedOk)
		}
		if ok && dark != tt.expectedDark {
			t.Errorf("parseAppleScriptRGB(%q) dark = %v, expected %v", tt.input, dark, tt.expectedDark)
		}
	}
}

func TestDetectDarkBackground_COLORFGBG(t *testing.T) {
	origGetenv := osGetenv
	origGOOS := runtimeGOOS
	origTTL := detectCacheTTL
	defer func() {
		osGetenv = origGetenv
		runtimeGOOS = origGOOS
		detectCacheTTL = origTTL
		ResetDetectionCache()
	}()

	runtimeGOOS = "linux"
	detectCacheTTL = 0
	ResetDetectionCache()

	tests := []struct {
		colorfgbg string
		expected  bool
	}{
		{"15;0", true},       // Black background
		{"0;15", false},      // Bright white background
		{"15;7", false},      // Light gray background
		{"15;8", true},       // Dark gray (bright black) background
		{"15;1", true},       // Red background
		{"15;11", false},     // Bright yellow background
		{"0;default", false}, // fallback to default
	}

	for _, tt := range tests {
		osGetenv = func(k string) string {
			if k == "COLORFGBG" {
				return tt.colorfgbg
			}
			return ""
		}
		if tt.colorfgbg == "0;default" {
			continue
		}
		got := DetectDarkBackground()
		if got != tt.expected {
			t.Errorf("COLORFGBG=%q: expected DetectDarkBackground()=%v, got %v", tt.colorfgbg, tt.expected, got)
		}
	}
}

func TestDetectDarkBackground_DarwinAppleTerminal(t *testing.T) {
	origGetenv := osGetenv
	origGOOS := runtimeGOOS
	origDetector := darwinDetector
	origTTL := darwinCacheTTL
	origGlobalTTL := detectCacheTTL

	defer func() {
		osGetenv = origGetenv
		runtimeGOOS = origGOOS
		darwinDetector = origDetector
		darwinCacheTTL = origTTL
		detectCacheTTL = origGlobalTTL
		darwinCacheMu.Lock()
		darwinLastCheck = time.Time{}
		darwinCacheMu.Unlock()
		ResetDetectionCache()
	}()

	runtimeGOOS = "darwin"
	darwinCacheTTL = 0 // Disable cache for unit test
	detectCacheTTL = 0
	ResetDetectionCache()

	osGetenv = func(k string) string {
		if k == "TERM_PROGRAM" {
			return "Apple_Terminal"
		}
		return ""
	}

	// Test light terminal (e.g. white background)
	darwinDetector = func() bool { return false }
	if got := DetectDarkBackground(); got != false {
		t.Errorf("expected light terminal to return false, got %v", got)
	}

	// Test dark terminal (e.g. black background)
	darwinDetector = func() bool { return true }
	if got := DetectDarkBackground(); got != true {
		t.Errorf("expected dark terminal to return true, got %v", got)
	}
}

func TestDetectDarkBackground_DarwinCache(t *testing.T) {
	origGetenv := osGetenv
	origGOOS := runtimeGOOS
	origDetector := darwinDetector
	origTTL := darwinCacheTTL
	origGlobalTTL := detectCacheTTL

	defer func() {
		osGetenv = origGetenv
		runtimeGOOS = origGOOS
		darwinDetector = origDetector
		darwinCacheTTL = origTTL
		detectCacheTTL = origGlobalTTL
		darwinCacheMu.Lock()
		darwinLastCheck = time.Time{}
		darwinCacheMu.Unlock()
		ResetDetectionCache()
	}()

	runtimeGOOS = "darwin"
	darwinCacheTTL = 10 * time.Second
	detectCacheTTL = 0
	ResetDetectionCache()

	osGetenv = func(k string) string {
		if k == "TERM_PROGRAM" {
			return "Apple_Terminal"
		}
		return ""
	}

	callCount := 0
	var mu sync.Mutex
	darwinDetector = func() bool {
		mu.Lock()
		callCount++
		mu.Unlock()
		return false
	}

	// First call should invoke detector
	if got := DetectDarkBackground(); got != false {
		t.Errorf("expected false, got %v", got)
	}
	// Second and third calls within TTL should use cached value without calling detector again
	_ = DetectDarkBackground()
	_ = DetectDarkBackground()

	mu.Lock()
	if callCount != 1 {
		t.Errorf("expected detector called exactly once due to caching, called %d times", callCount)
	}
	mu.Unlock()
}

func TestTheme_LazyColorModeResolution(t *testing.T) {
	origDetector := detectDarkBackgroundImpl
	defer func() {
		detectDarkBackgroundImpl = origDetector
	}()

	called := false
	detectDarkBackgroundImpl = func() bool {
		called = true
		t.Fatalf("DetectDarkBackground should NOT be called when ColorMode is explicitly dark or light")
		return false
	}

	themes := []Theme{
		ModernTheme{mode: ColorModeLight},
		ModernTheme{mode: ColorModeDark},
		LcarsTheme{mode: ColorModeLight},
		LcarsTheme{mode: ColorModeDark},
		CrtTheme{mode: ColorModeLight},
		CrtTheme{mode: ColorModeDark},
	}

	for _, th := range themes {
		_ = th.Styles()
		if called {
			break
		}
	}
}

func TestParseOSC11Response(t *testing.T) {
	tests := []struct {
		input        string
		expectedDark bool
		expectedOk   bool
	}{
		{"\x1b]11;rgb:ffff/ffff/ffff\x1b\\", false, true}, // White
		{"\x1b]11;rgb:0000/0000/0000\x07", true, true},    // Black
		{"\x1b]11;rgb:1234/1234/1234\x1b\\", true, true},  // Dark gray
		{"\x1b]11;rgb:eeee/eeee/eeee\x07", false, true},   // Light gray
		{"\x1b]11;rgb:ff/ff/ff\x1b\\", false, true},       // 2-digit white
		{"\x1b]11;rgb:00/00/00\x1b\\", true, true},        // 2-digit black
		{"invalid response", false, false},
		{"\x1b]11;rgb:zz/yy/xx\x1b\\", false, false},
	}
	for _, tt := range tests {
		dark, ok := parseOSC11Response(tt.input)
		if ok != tt.expectedOk {
			t.Errorf("parseOSC11Response(%q) ok = %v, expected %v", tt.input, ok, tt.expectedOk)
		}
		if ok && dark != tt.expectedDark {
			t.Errorf("parseOSC11Response(%q) dark = %v, expected %v", tt.input, dark, tt.expectedDark)
		}
	}
}

func TestDetectDarkBackground_GlobalCache(t *testing.T) {
	origGetenv := osGetenv
	origGOOS := runtimeGOOS
	origOSC := oscDetector
	origTTL := detectCacheTTL

	defer func() {
		osGetenv = origGetenv
		runtimeGOOS = origGOOS
		oscDetector = origOSC
		detectCacheTTL = origTTL
		detectCacheMu.Lock()
		detectHasCached = false
		detectLastCheck = time.Time{}
		detectCacheMu.Unlock()
	}()

	runtimeGOOS = "linux"
	detectCacheTTL = 5 * time.Second
	osGetenv = func(k string) string { return "" }

	detectCacheMu.Lock()
	detectHasCached = false
	detectCacheMu.Unlock()

	callCount := 0
	oscDetector = func(timeout time.Duration) (bool, bool) {
		callCount++
		return false, true // detected light background
	}

	res1 := DetectDarkBackground()
	if res1 != false {
		t.Errorf("expected false, got %v", res1)
	}
	res2 := DetectDarkBackground()
	if res2 != false {
		t.Errorf("expected false, got %v", res2)
	}

	if callCount != 1 {
		t.Errorf("expected detector to be called exactly once due to global cache, got %d", callCount)
	}
}

func TestDetectDarkBackground_OSCFallback(t *testing.T) {
	origGetenv := osGetenv
	origGOOS := runtimeGOOS
	origOSC := oscDetector
	origTTL := detectCacheTTL

	defer func() {
		osGetenv = origGetenv
		runtimeGOOS = origGOOS
		oscDetector = origOSC
		detectCacheTTL = origTTL
		detectCacheMu.Lock()
		detectHasCached = false
		detectLastCheck = time.Time{}
		detectCacheMu.Unlock()
	}()

	runtimeGOOS = "linux"
	detectCacheTTL = 0 // disable cache
	detectCacheMu.Lock()
	detectHasCached = false
	detectCacheMu.Unlock()

	// 1. OSC fails in Apple_Terminal -> fallback to light (false)
	osGetenv = func(k string) string {
		if k == "TERM_PROGRAM" {
			return "Apple_Terminal"
		}
		return ""
	}
	oscDetector = func(timeout time.Duration) (bool, bool) {
		return false, false // timeout / unhandled
	}
	if got := DetectDarkBackground(); got != false {
		t.Errorf("expected fallback to light for Apple_Terminal, got %v", got)
	}

	// 2. OSC fails in standard terminal -> fallback to dark (true)
	osGetenv = func(k string) string { return "" }
	if got := DetectDarkBackground(); got != true {
		t.Errorf("expected fallback to dark for standard terminal, got %v", got)
	}
}

func TestIsAppleScriptDefaultWhite(t *testing.T) {
	tests := []struct {
		input    string
		expected bool
	}{
		{"65535, 65535, 65535", true},
		{"65530, 65530, 65530", true},
		{"65535, 65535, 65535\n", true},
		{" 65535 , 65535 , 65535 ", true},
		{"0, 0, 0", false},
		{"55372, 54249, 46908", false}, // Novel profile
		{"65520, 65535, 65535", false},
		{"65535, 65535", false},
		{"invalid, 1, 2", false},
		{"", false},
	}

	for _, tt := range tests {
		got := isAppleScriptDefaultWhite(tt.input)
		if got != tt.expected {
			t.Errorf("isAppleScriptDefaultWhite(%q) = %v, expected %v", tt.input, got, tt.expected)
		}
	}
}

func TestDetectDarwinTerminalDarkBackground_Permutations(t *testing.T) {
	origAppleScript := appleScriptDetector
	origSystemDark := darwinSystemDarkDetector
	defer func() {
		appleScriptDetector = origAppleScript
		darwinSystemDarkDetector = origSystemDark
	}()

	tests := []struct {
		name         string
		scriptOutput string
		scriptErr    error
		systemDark   bool
		expectedDark bool
	}{
		{
			name:         "Default white profile in Dark Mode (macOS auto dark)",
			scriptOutput: "65535, 65535, 65535",
			scriptErr:    nil,
			systemDark:   true,
			expectedDark: true,
		},
		{
			name:         "Default white profile in Light Mode (macOS light)",
			scriptOutput: "65535, 65535, 65535",
			scriptErr:    nil,
			systemDark:   false,
			expectedDark: false,
		},
		{
			name:         "Explicit dark profile (Pro) in Light Mode",
			scriptOutput: "0, 0, 0",
			scriptErr:    nil,
			systemDark:   false,
			expectedDark: true,
		},
		{
			name:         "Explicit dark profile (Pro) in Dark Mode",
			scriptOutput: "0, 0, 0",
			scriptErr:    nil,
			systemDark:   true,
			expectedDark: true,
		},
		{
			name:         "Explicit light profile (Novel) in Dark Mode",
			scriptOutput: "55372, 54249, 46908",
			scriptErr:    nil,
			systemDark:   true,
			expectedDark: false,
		},
		{
			name:         "Explicit light profile (Novel) in Light Mode",
			scriptOutput: "55372, 54249, 46908",
			scriptErr:    nil,
			systemDark:   false,
			expectedDark: false,
		},
		{
			name:         "AppleScript error fallback to Dark Mode",
			scriptOutput: "",
			scriptErr:    errors.New("osascript execution failed"),
			systemDark:   true,
			expectedDark: true,
		},
		{
			name:         "AppleScript error fallback to Light Mode",
			scriptOutput: "",
			scriptErr:    errors.New("osascript execution failed"),
			systemDark:   false,
			expectedDark: false,
		},
		{
			name:         "Invalid AppleScript response fallback to Dark Mode",
			scriptOutput: "unrecognized output",
			scriptErr:    nil,
			systemDark:   true,
			expectedDark: true,
		},
		{
			name:         "Invalid AppleScript response fallback to Light Mode",
			scriptOutput: "unrecognized output",
			scriptErr:    nil,
			systemDark:   false,
			expectedDark: false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			appleScriptDetector = func(ctx context.Context) ([]byte, error) {
				return []byte(tt.scriptOutput), tt.scriptErr
			}
			darwinSystemDarkDetector = func() bool {
				return tt.systemDark
			}

			got := detectDarwinTerminalDarkBackground()
			if got != tt.expectedDark {
				t.Errorf("detectDarwinTerminalDarkBackground() = %v, expected %v", got, tt.expectedDark)
			}
		})
	}
}

func TestDetectDarkBackground_LiveDarwinIntegration(t *testing.T) {
	if runtimeGOOS != "darwin" {
		t.Skip("skipping Darwin live integration test on non-darwin")
	}
	ResetDetectionCache()
	isDark := DetectDarkBackground()
	t.Logf("Live Darwin background detection: isDark=%v", isDark)

	// Test specifically with TERM_PROGRAM=Apple_Terminal on Darwin
	origGetenv := osGetenv
	origCacheTTL := darwinCacheTTL
	origGlobalTTL := detectCacheTTL
	defer func() {
		osGetenv = origGetenv
		darwinCacheTTL = origCacheTTL
		detectCacheTTL = origGlobalTTL
		ResetDetectionCache()
	}()

	darwinCacheTTL = 0
	detectCacheTTL = 0
	ResetDetectionCache()

	osGetenv = func(k string) string {
		if k == "TERM_PROGRAM" {
			return "Apple_Terminal"
		}
		return origGetenv(k)
	}

	gotAppleTerminal := DetectDarkBackground()
	t.Logf("Live Darwin Apple_Terminal background detection: isDark=%v", gotAppleTerminal)
	// On this macOS system, AppleInterfaceStyle is "Dark", so Apple_Terminal must detect as dark.
	if isDarwinSystemDark() && !gotAppleTerminal {
		t.Errorf("expected Darwin Apple_Terminal to detect dark mode when AppleInterfaceStyle is Dark, got false")
	}
}


