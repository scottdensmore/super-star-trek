package theme

import (
	"strings"

	"github.com/charmbracelet/lipgloss"
)

// Theme defines the interface for TUI styling palettes and cycling.
type Theme interface {
	Name() string
	Styles() Styles
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

// DefaultTheme returns the default theme (Starfleet Modern).
func DefaultTheme() Theme {
	return ModernTheme{}
}

// GetTheme returns the Theme matching the given name, falling back to DefaultTheme.
func GetTheme(name string) Theme {
	switch strings.ToLower(strings.TrimSpace(name)) {
	case "modern":
		return ModernTheme{}
	case "lcars":
		return LcarsTheme{}
	case "crt":
		return CrtTheme{}
	default:
		return DefaultTheme()
	}
}
