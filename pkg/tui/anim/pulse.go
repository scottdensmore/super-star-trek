package anim

import (
	"math"

	"github.com/charmbracelet/lipgloss"
)

// RedAlertBadgeStyle returns an oscillating high-intensity crimson or dimmed red lipgloss
// style for the Condition RED badge based on cycle.
func RedAlertBadgeStyle(cycle int) lipgloss.Style {
	// Cosine oscillator: alternates high and low intensity phases.
	if math.Cos(float64(cycle)*math.Pi) >= 0 {
		return lipgloss.NewStyle().Bold(true).Foreground(lipgloss.Color("#FF1744"))
	}
	return lipgloss.NewStyle().Foreground(lipgloss.Color("#881B2B"))
}
