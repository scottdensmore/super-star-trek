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

func (t ModernTheme) Romulan() lipgloss.Style       { return t.Styles().Romulan }
func (t ModernTheme) Tholian() lipgloss.Style       { return t.Styles().Tholian }
func (t ModernTheme) PlasmaTorpedo() lipgloss.Style { return t.Styles().PlasmaTorpedo }
func (t ModernTheme) TholianWeb() lipgloss.Style    { return t.Styles().TholianWeb }
func (t ModernTheme) AlertRed() lipgloss.Style      { return t.Styles().AlertRed }
func (t ModernTheme) AlertYellow() lipgloss.Style   { return t.Styles().AlertYellow }

func (t ModernTheme) Styles() Styles {
	switch t.ColorMode() {
	case ColorModeDark:
		return t.PaletteStyles(true)
	case ColorModeLight:
		return t.PaletteStyles(false)
	default:
		return t.PaletteStyles(DetectDarkBackground())
	}
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

			Romulan: lipgloss.NewStyle().
				Bold(true).
				Foreground(lipgloss.Color("#10B981")),

			Tholian: lipgloss.NewStyle().
				Bold(true).
				Foreground(lipgloss.Color("#00E5FF")),

			PlasmaTorpedo: lipgloss.NewStyle().
				Bold(true).
				Foreground(lipgloss.Color("#FF5722")),

			TholianWeb: lipgloss.NewStyle().
				Bold(true).
				Foreground(lipgloss.Color("#22D3EE")),

			Starbase: lipgloss.NewStyle().
				Bold(true).
				Foreground(lipgloss.Color("#FFD700")),

			Star: lipgloss.NewStyle().
				Foreground(lipgloss.Color("#FFFFFF")),

			Planet: lipgloss.NewStyle().
				Foreground(lipgloss.Color("#00E676")),

			BlackHole: lipgloss.NewStyle().
				Foreground(lipgloss.Color("#A855F7")),

			Wormhole: lipgloss.NewStyle().
				Bold(true).
				Foreground(lipgloss.Color("#04D9FF")),

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

			AlertRed: lipgloss.NewStyle().
				Bold(true).
				Foreground(lipgloss.Color("#FF3344")),

			AlertYellow: lipgloss.NewStyle().
				Bold(true).
				Foreground(lipgloss.Color("#FFD700")),

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

		Romulan: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#059669")),

		Tholian: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#0891B2")),

		PlasmaTorpedo: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#EA580C")),

		TholianWeb: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#06B6D4")),

		Starbase: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#D97706")),

		Star: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#475569")),

		Planet: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#16A34A")),

		BlackHole: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#7C3AED")),

		Wormhole: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#0284C7")),

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

		AlertRed: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#DC2626")),

		AlertYellow: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#D97706")),

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
