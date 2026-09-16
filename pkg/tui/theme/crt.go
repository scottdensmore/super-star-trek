package theme

import "github.com/charmbracelet/lipgloss"

// CrtTheme implements the 1970s monochrome phosphor CRT green aesthetic.
type CrtTheme struct {
	mode ColorMode
}

func (t CrtTheme) Name() string {
	return "crt"
}

func (t CrtTheme) ColorMode() ColorMode {
	if t.mode == "" {
		return ColorModeAuto
	}
	return t.mode
}

func (t CrtTheme) WithColorMode(mode ColorMode) Theme {
	t.mode = mode
	return t
}

func (t CrtTheme) Next() Theme {
	return ModernTheme{mode: t.ColorMode()}
}

func (t CrtTheme) PaletteStyles(isDark bool) Styles {
	return t.Styles()
}

func (t CrtTheme) Styles() Styles {
	border := lipgloss.NormalBorder()

	return Styles{
		Title: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#050B05")).
			Background(lipgloss.Color("#33FF33")).
			Padding(0, 1),

		Panel: lipgloss.NewStyle().
			BorderStyle(border).
			BorderForeground(lipgloss.Color("#33FF33")).
			Background(lipgloss.Color("#050B05")).
			Padding(0, 1),

		PanelTitle: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#33FF33")),

		Border: border,

		Grid: lipgloss.NewStyle().
			BorderStyle(border).
			BorderForeground(lipgloss.Color("#116611")),

		GridHeader: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#33FF33")),

		Enterprise: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#33FF33")).
			Reverse(true),

		Klingon: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#33FF33")),

		Starbase: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#33FF33")),

		Star: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#22CC22")),

		Planet: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#22AA22")),

		BlackHole: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#116611")),

		Empty: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#0D3B0D")),

		ConditionGreen: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#33FF33")),

		ConditionYellow: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#33FF33")).
			Underline(true),

		ConditionRed: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#050B05")).
			Background(lipgloss.Color("#33FF33")),

		ConditionDocked: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#33FF33")).
			Italic(true),

		ProgressBarFilled: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#33FF33")),

		ProgressBarEmpty: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#114411")),

		GaugeLabel: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#22CC22")),

		GaugeValue: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#33FF33")),

		SubsystemNormal: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#33FF33")),

		SubsystemDamaged: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#050B05")).
			Background(lipgloss.Color("#33FF33")),

		Prompt: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#33FF33")),

		CommandText: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#33FF33")),

		LogText: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#22AA22")),
	}
}
