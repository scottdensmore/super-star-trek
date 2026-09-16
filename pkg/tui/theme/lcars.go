package theme

import "github.com/charmbracelet/lipgloss"

// LcarsTheme implements the 24th-century LCARS aesthetic palette.
type LcarsTheme struct {
	mode ColorMode
}

func (t LcarsTheme) Name() string {
	return "lcars"
}

func (t LcarsTheme) ColorMode() ColorMode {
	if t.mode == "" {
		return ColorModeAuto
	}
	return t.mode
}

func (t LcarsTheme) WithColorMode(mode ColorMode) Theme {
	t.mode = mode
	return t
}

func (t LcarsTheme) Next() Theme {
	return CrtTheme{mode: t.ColorMode()}
}

func (t LcarsTheme) PaletteStyles(isDark bool) Styles {
	return t.Styles()
}

func (t LcarsTheme) Styles() Styles {
	border := lipgloss.RoundedBorder()

	return Styles{
		Title: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#000000")).
			Background(lipgloss.Color("#FF9900")).
			Padding(0, 1),

		Panel: lipgloss.NewStyle().
			BorderStyle(border).
			BorderForeground(lipgloss.Color("#FF9900")).
			Background(lipgloss.Color("#000000")).
			Padding(0, 1),

		PanelTitle: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#FFCC66")),

		Border: border,

		Grid: lipgloss.NewStyle().
			BorderStyle(border).
			BorderForeground(lipgloss.Color("#3399CC")),

		GridHeader: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#FFCC66")),

		Enterprise: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#FFCC66")),

		Klingon: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#FF3300")),

		Starbase: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#3399CC")),

		Star: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#FFFFFF")),

		Planet: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#CC6699")),

		BlackHole: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#9966CC")),

		Empty: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#443322")),

		ConditionGreen: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#3399CC")),

		ConditionYellow: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#FF9900")),

		ConditionRed: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#FF3300")),

		ConditionDocked: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#CC6699")),

		ProgressBarFilled: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#FF9900")),

		ProgressBarEmpty: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#332211")),

		GaugeLabel: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#FFCC66")),

		GaugeValue: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#FF9900")),

		SubsystemNormal: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#3399CC")),

		SubsystemDamaged: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#FF3300")),

		Prompt: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#FF9900")),

		CommandText: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#FFCC66")),

		LogText: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#CC9966")),
	}
}
