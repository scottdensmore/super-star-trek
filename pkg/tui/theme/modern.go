package theme

import "github.com/charmbracelet/lipgloss"

// ModernTheme implements the Starfleet Modern high-contrast 24-bit TrueColor theme.
type ModernTheme struct{}

func (t ModernTheme) Name() string {
	return "modern"
}

func (t ModernTheme) Next() Theme {
	return LcarsTheme{}
}

func (t ModernTheme) Styles() Styles {
	border := lipgloss.RoundedBorder()

	return Styles{
		Title: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#00E5FF")).
			Background(lipgloss.Color("#0B0F19")).
			Padding(0, 1),

		Panel: lipgloss.NewStyle().
			BorderStyle(border).
			BorderForeground(lipgloss.Color("#1E293B")).
			Background(lipgloss.Color("#0B0F19")).
			Padding(0, 1),

		PanelTitle: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#00E5FF")),

		Border: border,

		Grid: lipgloss.NewStyle().
			BorderStyle(lipgloss.NormalBorder()).
			BorderForeground(lipgloss.Color("#1E293B")),

		GridHeader: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#64748B")),

		Enterprise: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#00E5FF")),

		Klingon: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#FF3344")),

		Starbase: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#FFD700")),

		Star: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#FFFFFF")),

		Planet: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#00E676")),

		BlackHole: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#A855F7")),

		Empty: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#334155")),

		ConditionGreen: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#00E676")),

		ConditionYellow: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#FFD700")),

		ConditionRed: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#FF3344")),

		ConditionDocked: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#00E5FF")),

		ProgressBarFilled: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#00E5FF")),

		ProgressBarEmpty: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#1E293B")),

		GaugeLabel: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#E2E8F0")),

		GaugeValue: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#00E5FF")),

		SubsystemNormal: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#00E676")),

		SubsystemDamaged: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#FF3344")),

		Prompt: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#00E5FF")),

		CommandText: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#E2E8F0")),

		LogText: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#94A3B8")),
	}
}
