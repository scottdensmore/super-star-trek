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

func (t LcarsTheme) Styles() Styles {
	return t.PaletteStyles(t.ColorMode().Resolve(lipgloss.HasDarkBackground()))
}

func (t LcarsTheme) PaletteStyles(isDark bool) Styles {
	border := lipgloss.RoundedBorder()

	if isDark {
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

	return Styles{
		Title: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#FEF9EF")).
			Background(lipgloss.Color("#C2410C")).
			Padding(0, 1),

		Panel: lipgloss.NewStyle().
			BorderStyle(border).
			BorderForeground(lipgloss.Color("#C2410C")).
			Background(lipgloss.Color("#FEF9EF")).
			Padding(0, 1),

		PanelTitle: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#9A3412")),

		Border: border,

		Grid: lipgloss.NewStyle().
			BorderStyle(border).
			BorderForeground(lipgloss.Color("#0E7490")),

		GridHeader: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#9A3412")),

		Enterprise: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#C2410C")),

		Klingon: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#B91C1C")),

		Starbase: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#0E7490")),

		Star: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#57534E")),

		Planet: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#9D174D")),

		BlackHole: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#6B21A8")),

		Empty: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#A8A29E")),

		ConditionGreen: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#0E7490")),

		ConditionYellow: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#C2410C")),

		ConditionRed: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#B91C1C")),

		ConditionDocked: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#9D174D")),

		ProgressBarFilled: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#C2410C")),

		ProgressBarEmpty: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#E7E5E4")),

		GaugeLabel: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#1C1917")),

		GaugeValue: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#0E7490")),

		SubsystemNormal: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#0E7490")),

		SubsystemDamaged: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#B91C1C")),

		Prompt: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#C2410C")),

		CommandText: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#1C1917")),

		LogText: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#78350F")),
	}
}
