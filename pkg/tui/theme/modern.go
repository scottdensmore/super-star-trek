package theme

import "github.com/charmbracelet/lipgloss"

// ModernTheme implements the Starfleet Modern high-contrast 24-bit TrueColor theme.
type ModernTheme struct {
	mode ColorMode
}

func (t ModernTheme) Name() string {
	return "modern"
}

func (t ModernTheme) ColorMode() ColorMode {
	if t.mode == "" {
		return ColorModeAuto
	}
	return t.mode
}

func (t ModernTheme) WithColorMode(mode ColorMode) Theme {
	t.mode = mode
	return t
}

func (t ModernTheme) Next() Theme {
	return LcarsTheme{mode: t.ColorMode()}
}

func (t ModernTheme) Styles() Styles {
	return t.PaletteStyles(t.ColorMode().Resolve(DetectDarkBackground()))
}

func (t ModernTheme) PaletteStyles(isDark bool) Styles {
	border := lipgloss.RoundedBorder()

	if isDark {
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
				BorderForeground(lipgloss.Color("#1E293B")).
				Background(lipgloss.Color("#0B0F19")),

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

	return Styles{
		Title: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#0284C7")).
			Background(lipgloss.Color("#F8FAFC")).
			Padding(0, 1),

		Panel: lipgloss.NewStyle().
			BorderStyle(border).
			BorderForeground(lipgloss.Color("#CBD5E1")).
			Background(lipgloss.Color("#F8FAFC")).
			Padding(0, 1),

		PanelTitle: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#0284C7")),

		Border: border,

		Grid: lipgloss.NewStyle().
			BorderStyle(lipgloss.NormalBorder()).
			BorderForeground(lipgloss.Color("#CBD5E1")).
			Background(lipgloss.Color("#F8FAFC")),

		GridHeader: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#475569")),

		Enterprise: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#0284C7")),

		Klingon: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#DC2626")),

		Starbase: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#D97706")),

		Star: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#475569")),

		Planet: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#16A34A")),

		BlackHole: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#7C3AED")),

		Empty: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#94A3B8")),

		ConditionGreen: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#16A34A")),

		ConditionYellow: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#D97706")),

		ConditionRed: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#DC2626")),

		ConditionDocked: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#0284C7")),

		ProgressBarFilled: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#0284C7")),

		ProgressBarEmpty: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#E2E8F0")),

		GaugeLabel: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#334155")),

		GaugeValue: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#0F172A")),

		SubsystemNormal: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#16A34A")),

		SubsystemDamaged: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#DC2626")),

		Prompt: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#0284C7")),

		CommandText: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#0F172A")),

		LogText: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#0F172A")),
	}
}
