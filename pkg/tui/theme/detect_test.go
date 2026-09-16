package theme

import (
	"sync"
	"testing"
	"time"
)

func TestParseAppleScriptRGB(t *testing.T) {
	tests := []struct {
		input       string
		expectedDark bool
		expectedOk   bool
	}{
		{"65535, 65535, 65535", false, true}, // Pure White
		{"0, 0, 0", true, true},             // Pure Black
		{"50000, 50000, 50000", false, true}, // Light gray
		{"10000, 10000, 10000", true, true},  // Dark gray
		{"65535, 65535", false, false},       // Too few parts
		{"a, b, c", false, false},             // Non-numeric
		{"", false, false},                    // Empty
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
	defer func() {
		osGetenv = origGetenv
		runtimeGOOS = origGOOS
	}()

	runtimeGOOS = "linux"

	tests := []struct {
		colorfgbg string
		expected  bool
	}{
		{"15;0", true},   // Black background
		{"0;15", false},  // Bright white background
		{"15;7", false},  // Light gray background
		{"15;8", true},   // Dark gray (bright black) background
		{"15;1", true},   // Red background
		{"15;11", false}, // Bright yellow background
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

	defer func() {
		osGetenv = origGetenv
		runtimeGOOS = origGOOS
		darwinDetector = origDetector
		darwinCacheTTL = origTTL
		darwinCacheMu.Lock()
		darwinLastCheck = time.Time{}
		darwinCacheMu.Unlock()
	}()

	runtimeGOOS = "darwin"
	darwinCacheTTL = 0 // Disable cache for unit test

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

	defer func() {
		osGetenv = origGetenv
		runtimeGOOS = origGOOS
		darwinDetector = origDetector
		darwinCacheTTL = origTTL
		darwinCacheMu.Lock()
		darwinLastCheck = time.Time{}
		darwinCacheMu.Unlock()
	}()

	runtimeGOOS = "darwin"
	darwinCacheTTL = 10 * time.Second

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
