package theme

import (
	"fmt"
	"strings"

	"github.com/charmbracelet/lipgloss"
)

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

func (t LcarsTheme) Romulan() lipgloss.Style       { return t.Styles().Romulan }
func (t LcarsTheme) Tholian() lipgloss.Style       { return t.Styles().Tholian }
func (t LcarsTheme) PlasmaTorpedo() lipgloss.Style { return t.Styles().PlasmaTorpedo }
func (t LcarsTheme) TholianWeb() lipgloss.Style    { return t.Styles().TholianWeb }
func (t LcarsTheme) AlertRed() lipgloss.Style      { return t.Styles().AlertRed }
func (t LcarsTheme) AlertYellow() lipgloss.Style   { return t.Styles().AlertYellow }

// PillCaps returns the LCARS curved pill caps ◖ and ◗.
func (t LcarsTheme) PillCaps() (string, string) {
	return "◖", "◗"
}

// FormatHeader formats the top status banner using LCARS curved pill caps and color blocking.
func (t LcarsTheme) FormatHeader(title, themeInfo string, width int) string {
	isDark := true
	switch t.ColorMode() {
	case ColorModeDark:
		isDark = true
	case ColorModeLight:
		isDark = false
	default:
		isDark = DetectDarkBackground()
	}

	var primaryColor, secondaryColor, textColor, panelBg lipgloss.Color
	if isDark {
		primaryColor = lipgloss.Color("#FF9900")
		secondaryColor = lipgloss.Color("#3399CC")
		textColor = lipgloss.Color("#000000")
		panelBg = lipgloss.Color("#000000")
	} else {
		primaryColor = lipgloss.Color("#C2410C")
		secondaryColor = lipgloss.Color("#0E7490")
		textColor = lipgloss.Color("#FEF9EF")
		panelBg = lipgloss.Color("#FEF9EF")
	}

	capLeft := lipgloss.NewStyle().Foreground(primaryColor).Background(panelBg).Render("◖")
	titleBlock := lipgloss.NewStyle().Bold(true).Foreground(textColor).Background(primaryColor).Render(" " + title + " ")
	themeBlock := lipgloss.NewStyle().Bold(true).Foreground(textColor).Background(secondaryColor).Render(" " + themeInfo + " ")
	capRight := lipgloss.NewStyle().Foreground(secondaryColor).Background(panelBg).Render("◗")

	fixedWidth := lipgloss.Width(capLeft) + lipgloss.Width(titleBlock) + lipgloss.Width(themeBlock) + lipgloss.Width(capRight)
	gap := width - fixedWidth
	if gap < 1 {
		gap = 1
	}
	gapStr := lipgloss.NewStyle().Background(panelBg).Render(strings.Repeat(" ", gap))

	return capLeft + titleBlock + gapStr + themeBlock + capRight
}

// FormatAudioBadge formats the HUD audio volume indicator badge for LCARS theme.
func (t LcarsTheme) FormatAudioBadge(volume int, muted bool) string {
	styles := t.Styles()
	if muted || volume <= 0 {
		return styles.Empty.Render("🔇 MUTED")
	}
	return styles.Prompt.Render(fmt.Sprintf("🔊 %d%%", volume))
}

func (t LcarsTheme) Styles() Styles {
	switch t.ColorMode() {
	case ColorModeDark:
		return t.PaletteStyles(true)
	case ColorModeLight:
		return t.PaletteStyles(false)
	default:
		return t.PaletteStyles(DetectDarkBackground())
	}
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
				BorderForeground(lipgloss.Color("#3399CC")).
				Background(lipgloss.Color("#000000")),

			GridHeader: lipgloss.NewStyle().
				Bold(true).
				Foreground(lipgloss.Color("#FFCC66")),

			Enterprise: lipgloss.NewStyle().
				Bold(true).
				Foreground(lipgloss.Color("#FFCC66")),

			Klingon: lipgloss.NewStyle().
				Bold(true).
				Foreground(lipgloss.Color("#FF3300")),

			Romulan: lipgloss.NewStyle().
				Bold(true).
				Foreground(lipgloss.Color("#CC6699")),

			Tholian: lipgloss.NewStyle().
				Bold(true).
				Foreground(lipgloss.Color("#FFD700")),

			PlasmaTorpedo: lipgloss.NewStyle().
				Bold(true).
				Foreground(lipgloss.Color("#FF6600")),

			TholianWeb: lipgloss.NewStyle().
				Bold(true).
				Foreground(lipgloss.Color("#FFCC66")),

			Starbase: lipgloss.NewStyle().
				Bold(true).
				Foreground(lipgloss.Color("#3399CC")),

			Star: lipgloss.NewStyle().
				Foreground(lipgloss.Color("#FFFFFF")),

			Planet: lipgloss.NewStyle().
				Foreground(lipgloss.Color("#CC6699")),

			BlackHole: lipgloss.NewStyle().
				Foreground(lipgloss.Color("#9966CC")),

			Wormhole: lipgloss.NewStyle().
				Bold(true).
				Foreground(lipgloss.Color("#00CCCC")),

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

			AlertRed: lipgloss.NewStyle().
				Bold(true).
				Foreground(lipgloss.Color("#FF3300")),

			AlertYellow: lipgloss.NewStyle().
				Bold(true).
				Foreground(lipgloss.Color("#FF9900")),

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
			BorderForeground(lipgloss.Color("#0E7490")).
			Background(lipgloss.Color("#FEF9EF")),

		GridHeader: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#9A3412")),

		Enterprise: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#C2410C")),

		Klingon: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#B91C1C")),

		Romulan: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#86198F")),

		Tholian: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#A16207")),

		PlasmaTorpedo: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#C2410C")),

		TholianWeb: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#D97706")),

		Starbase: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#0E7490")),

		Star: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#57534E")),

		Planet: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#9D174D")),

		BlackHole: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#6B21A8")),

		Wormhole: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#008888")),

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

		AlertRed: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#B91C1C")),

		AlertYellow: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#C2410C")),

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
