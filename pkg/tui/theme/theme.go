package theme

import (
	"fmt"
	"strings"

	"github.com/charmbracelet/lipgloss"
)

// ColorMode represents the luminance mode for TUI themes.
type ColorMode string

const (
	// ColorModeAuto dynamically detects terminal background darkness.
	ColorModeAuto ColorMode = "auto"
	// ColorModeDark forces dark mode palettes regardless of terminal query.
	ColorModeDark ColorMode = "dark"
	// ColorModeLight forces light mode palettes regardless of terminal query.
	ColorModeLight ColorMode = "light"
)

// Resolve returns true if the active mode resolves to dark, false for light.
func (m ColorMode) Resolve(hasDarkBg bool) bool {
	switch m {
	case ColorModeDark:
		return true
	case ColorModeLight:
		return false
	case ColorModeAuto:
		return hasDarkBg
	default:
		return hasDarkBg
	}
}

// Next cycles through Auto -> Dark -> Light -> Auto.
func (m ColorMode) Next() ColorMode {
	switch m {
	case ColorModeAuto:
		return ColorModeDark
	case ColorModeDark:
		return ColorModeLight
	case ColorModeLight:
		return ColorModeAuto
	default:
		return ColorModeAuto
	}
}

// ParseColorMode converts a string to a valid ColorMode, returning an error if invalid.
func ParseColorMode(val string) (ColorMode, error) {
	switch strings.ToLower(strings.TrimSpace(val)) {
	case "auto":
		return ColorModeAuto, nil
	case "dark":
		return ColorModeDark, nil
	case "light":
		return ColorModeLight, nil
	default:
		return ColorModeAuto, fmt.Errorf("invalid color mode: %q", val)
	}
}

// Theme defines the interface for TUI styling palettes and cycling.
type Theme interface {
	Name() string
	Styles() Styles
	PaletteStyles(isDark bool) Styles
	ColorMode() ColorMode
	WithColorMode(mode ColorMode) Theme
	Next() Theme
}

// Styles holds Lip Gloss styles for dashboard panels, grid cells,
// progress bars, alert conditions, and command prompt.
type Styles struct {
	// Title banner
	Title lipgloss.Style

	// Panels and layout
	Panel      lipgloss.Style
	PanelTitle lipgloss.Style
	Border     lipgloss.Border

	// Sector grid
	Grid       lipgloss.Style
	GridHeader lipgloss.Style

	// Entity glyphs
	Enterprise lipgloss.Style
	Klingon    lipgloss.Style
	Starbase   lipgloss.Style
	Star       lipgloss.Style
	Planet     lipgloss.Style
	BlackHole  lipgloss.Style
	Empty      lipgloss.Style

	// Condition alert badges
	ConditionGreen  lipgloss.Style
	ConditionYellow lipgloss.Style
	ConditionRed    lipgloss.Style
	ConditionDocked lipgloss.Style

	// Telemetry & Progress Bars
	ProgressBarFilled lipgloss.Style
	ProgressBarEmpty  lipgloss.Style
	GaugeLabel        lipgloss.Style
	GaugeValue        lipgloss.Style

	// Subsystems / Devices
	SubsystemNormal  lipgloss.Style
	SubsystemDamaged lipgloss.Style

	// Command bar & Log
	Prompt      lipgloss.Style
	CommandText lipgloss.Style
	LogText     lipgloss.Style
}

// DefaultTheme returns the default theme (Starfleet Modern) with ColorModeAuto.
func DefaultTheme() Theme {
	return ModernTheme{mode: ColorModeAuto}
}

// GetTheme returns the Theme matching the given name, falling back to DefaultTheme.
func GetTheme(name string) Theme {
	switch strings.ToLower(strings.TrimSpace(name)) {
	case "modern":
		return ModernTheme{mode: ColorModeAuto}
	case "lcars":
		return LcarsTheme{mode: ColorModeAuto}
	case "crt":
		return CrtTheme{mode: ColorModeAuto}
	default:
		return DefaultTheme()
	}
}

