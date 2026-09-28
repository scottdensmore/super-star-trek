package theme

import (
	"fmt"
	"strings"

	"github.com/charmbracelet/lipgloss"
)

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

func (t CrtTheme) Romulan() lipgloss.Style       { return t.Styles().Romulan }
func (t CrtTheme) Tholian() lipgloss.Style       { return t.Styles().Tholian }
func (t CrtTheme) PlasmaTorpedo() lipgloss.Style { return t.Styles().PlasmaTorpedo }
func (t CrtTheme) TholianWeb() lipgloss.Style    { return t.Styles().TholianWeb }
func (t CrtTheme) AlertRed() lipgloss.Style      { return t.Styles().AlertRed }
func (t CrtTheme) AlertYellow() lipgloss.Style   { return t.Styles().AlertYellow }

// FormatHeader formats the top status banner for CRT theme with phosphor text and scanline spacing.
func (t CrtTheme) FormatHeader(title, themeInfo string, width int) string {
	styles := t.Styles()
	frame := styles.Title.GetHorizontalFrameSize()
	contentWidth := width - frame
	gap := contentWidth - lipgloss.Width(title) - lipgloss.Width(themeInfo)
	if gap < 2 {
		gap = 2
	}
	headerLine := title + strings.Repeat(" ", gap) + themeInfo
	return styles.Title.Width(width).Render(headerLine)
}

// FormatAudioBadge formats the HUD audio volume indicator badge for CRT theme.
func (t CrtTheme) FormatAudioBadge(volume int, muted bool) string {
	styles := t.Styles()
	if muted || volume <= 0 {
		return styles.Empty.Render("[SND: OFF]")
	}
	return styles.Prompt.Render(fmt.Sprintf("[SND: %d%%]", volume))
}

func (t CrtTheme) Styles() Styles {
	switch t.ColorMode() {
	case ColorModeDark:
		return t.PaletteStyles(true)
	case ColorModeLight:
		return t.PaletteStyles(false)
	default:
		return t.PaletteStyles(DetectDarkBackground())
	}
}

func (t CrtTheme) PaletteStyles(isDark bool) Styles {
	border := lipgloss.NormalBorder()

	if isDark {
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
				BorderForeground(lipgloss.Color("#116611")).
				Background(lipgloss.Color("#050B05")),

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

			Romulan: lipgloss.NewStyle().
				Bold(true).
				Foreground(lipgloss.Color("#33FF33")),

			Tholian: lipgloss.NewStyle().
				Bold(true).
				Foreground(lipgloss.Color("#FFB000")),

			PlasmaTorpedo: lipgloss.NewStyle().
				Bold(true).
				Foreground(lipgloss.Color("#33FF33")),

			TholianWeb: lipgloss.NewStyle().
				Bold(true).
				Foreground(lipgloss.Color("#66FF66")),

			Starbase: lipgloss.NewStyle().
				Bold(true).
				Foreground(lipgloss.Color("#33FF33")),

			Star: lipgloss.NewStyle().
				Foreground(lipgloss.Color("#22CC22")),

			Planet: lipgloss.NewStyle().
				Foreground(lipgloss.Color("#22AA22")),

			BlackHole: lipgloss.NewStyle().
				Foreground(lipgloss.Color("#116611")),

			Wormhole: lipgloss.NewStyle().
				Bold(true).
				Foreground(lipgloss.Color("#33FF33")),

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

			AlertRed: lipgloss.NewStyle().
				Bold(true).
				Foreground(lipgloss.Color("#050B05")).
				Background(lipgloss.Color("#33FF33")),

			AlertYellow: lipgloss.NewStyle().
				Bold(true).
				Foreground(lipgloss.Color("#33FF33")).
				Underline(true),

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

	return Styles{
		Title: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#F0FDF4")).
			Background(lipgloss.Color("#14532D")).
			Padding(0, 1),

		Panel: lipgloss.NewStyle().
			BorderStyle(border).
			BorderForeground(lipgloss.Color("#166534")).
			Background(lipgloss.Color("#F0FDF4")).
			Padding(0, 1),

		PanelTitle: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#14532D")),

		Border: border,

		Grid: lipgloss.NewStyle().
			BorderStyle(border).
			BorderForeground(lipgloss.Color("#86EFAC")).
			Background(lipgloss.Color("#F0FDF4")),

		GridHeader: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#14532D")),

		Enterprise: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#14532D")).
			Reverse(true),

		Klingon: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#14532D")),

		Romulan: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#14532D")),

		Tholian: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#854D0E")),

		PlasmaTorpedo: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#14532D")),

		TholianWeb: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#15803D")),

		Starbase: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#14532D")),

		Star: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#15803D")),

		Planet: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#15803D")),

		BlackHole: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#86EFAC")),

		Wormhole: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#166534")),

		Empty: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#86EFAC")),

		ConditionGreen: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#14532D")),

		ConditionYellow: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#14532D")).
			Underline(true),

		ConditionRed: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#F0FDF4")).
			Background(lipgloss.Color("#14532D")),

		ConditionDocked: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#14532D")).
			Italic(true),

		AlertRed: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#F0FDF4")).
			Background(lipgloss.Color("#14532D")),

		AlertYellow: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#14532D")).
			Underline(true),

		ProgressBarFilled: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#14532D")),

		ProgressBarEmpty: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#DCFCE7")),

		GaugeLabel: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#15803D")),

		GaugeValue: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#14532D")),

		SubsystemNormal: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#14532D")),

		SubsystemDamaged: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#F0FDF4")).
			Background(lipgloss.Color("#14532D")),

		Prompt: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#14532D")),

		CommandText: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#14532D")),

		LogText: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#15803D")),
	}
}
