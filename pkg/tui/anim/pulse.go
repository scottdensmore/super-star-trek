package anim

import (
	"github.com/charmbracelet/lipgloss"
)

// RedAlertBadgeStyle returns an oscillating high-intensity crimson or dimmed red lipgloss
// style for the Condition RED badge based on cycle.
func RedAlertBadgeStyle(cycle int) lipgloss.Style {
	if cycle%2 == 0 {
		return lipgloss.NewStyle().Bold(true).Foreground(lipgloss.Color("#FF1744"))
	}
	return lipgloss.NewStyle().Foreground(lipgloss.Color("#881B2B"))
}
