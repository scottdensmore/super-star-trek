package theme

// Parity reference generator for the Rust port (crates/sst-tui/src/theme).
// Run with: SST_PARITYGEN=1 go test ./pkg/tui/theme -run TestParityGen -count=1
// Writes testdata/parity/theme/theme.txt at the repository root. Every line is
// "<key>\t<%q of value>". For every theme x {dark, light} x colour profile it
// records: theme getters, every Styles field rendered with sample text plus
// its colour getters, FormatHeader at several widths, FormatAudioBadge cases,
// the Romulan()/Tholian()/... accessors, and the LCARS pill caps.

import (
	"bufio"
	"fmt"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"

	"github.com/charmbracelet/lipgloss"
	"github.com/muesli/termenv"
)

func pgColor(c lipgloss.TerminalColor) string {
	switch v := c.(type) {
	case nil:
		return "nil"
	case lipgloss.NoColor:
		return "NoColor"
	case lipgloss.Color:
		return "Color(" + string(v) + ")"
	default:
		return fmt.Sprintf("%T(%v)", c, c)
	}
}

func pgBorder(b lipgloss.Border) string {
	return strings.Join([]string{b.Top, b.Bottom, b.Left, b.Right, b.TopLeft, b.TopRight,
		b.BottomLeft, b.BottomRight, b.MiddleLeft, b.MiddleRight, b.Middle, b.MiddleTop, b.MiddleBottom}, "|")
}

var pgSamples = []string{"USS Enterprise ★ 1701", "line one\nline two", ""}

func pgStyle(w func(k, v string), key string, s lipgloss.Style) {
	for i, sample := range pgSamples {
		w(fmt.Sprintf("%s.render%d", key, i), s.Render(sample))
	}
	w(key+".fg", pgColor(s.GetForeground()))
	w(key+".bg", pgColor(s.GetBackground()))
	w(key+".border_top_fg", pgColor(s.GetBorderTopForeground()))
	w(key+".flags", fmt.Sprintf("bold=%v italic=%v underline=%v reverse=%v hframe=%d vframe=%d",
		s.GetBold(), s.GetItalic(), s.GetUnderline(), s.GetReverse(),
		s.GetHorizontalFrameSize(), s.GetVerticalFrameSize()))
}

func pgStyles(w func(k, v string), prefix string, st Styles) {
	v := reflect.ValueOf(st)
	tp := v.Type()
	for i := 0; i < tp.NumField(); i++ {
		f := tp.Field(i)
		switch val := v.Field(i).Interface().(type) {
		case lipgloss.Style:
			pgStyle(w, prefix+f.Name, val)
		case lipgloss.Border:
			w(prefix+f.Name, pgBorder(val))
		}
	}
}

func TestParityGen(t *testing.T) {
	if os.Getenv("SST_PARITYGEN") != "1" {
		t.Skip("set SST_PARITYGEN=1 to regenerate parity data")
	}
	out := filepath.Join("..", "..", "..", "testdata", "parity", "theme")
	if err := os.MkdirAll(out, 0o755); err != nil {
		t.Fatal(err)
	}
	f, err := os.Create(filepath.Join(out, "theme.txt"))
	if err != nil {
		t.Fatal(err)
	}
	defer f.Close()
	bw := bufio.NewWriter(f)
	defer bw.Flush()
	w := func(k, v string) { fmt.Fprintf(bw, "%s\t%q\n", k, v) }

	// Detection must never run: every theme below has an explicit mode.
	origImpl := detectDarkBackgroundImpl
	detectDarkBackgroundImpl = func() bool { panic("detection called") }
	defer func() { detectDarkBackgroundImpl = origImpl }()

	// Colour-mode helpers.
	for _, m := range []ColorMode{"", ColorModeAuto, ColorModeDark, ColorModeLight} {
		w("colormode/"+string(m)+"/next", string(m.Next()))
		w("colormode/"+string(m)+"/resolve", fmt.Sprintf("%v %v", m.Resolve(true), m.Resolve(false)))
	}
	for _, in := range []string{"auto", "AUTO", " dark ", "Light", "", "invalid", "darkk", "\tlight\n"} {
		m, err := ParseColorMode(in)
		es := "nil"
		if err != nil {
			es = err.Error()
		}
		w("parse/"+fmt.Sprintf("%q", in), string(m)+" "+es)
	}
	for _, in := range []string{"modern", "MODERN", "Lcars", "  crt  ", "", "unknown"} {
		th := GetTheme(in)
		w("get/"+fmt.Sprintf("%q", in), th.Name()+" "+string(th.ColorMode()))
	}
	w("default", DefaultTheme().Name()+" "+string(DefaultTheme().ColorMode()))

	profiles := []struct {
		name string
		p    termenv.Profile
	}{{"Ascii", termenv.Ascii}, {"ANSI", termenv.ANSI}, {"ANSI256", termenv.ANSI256}, {"TrueColor", termenv.TrueColor}}
	titles := []string{"★ SUPER STAR TREK ★  USS ENTERPRISE NCC-1701", "SST", ""}
	tags := []string{"[Theme: MODERN (F2)]", ""}
	widths := []int{0, 1, 10, 20, 40, 60, 80, 100, 132}
	badges := []struct {
		vol   int
		muted bool
	}{{80, false}, {0, false}, {0, true}, {100, true}, {-5, false}, {1, false}, {100, false}, {150, false}}

	lipgloss.SetHasDarkBackground(true)
	defer lipgloss.SetColorProfile(termenv.Ascii)
	for _, name := range []string{ThemeModern, ThemeLcars, ThemeCrt} {
		for _, mode := range []ColorMode{ColorModeDark, ColorModeLight} {
			for _, pr := range profiles {
				lipgloss.SetColorProfile(pr.p)
				th := GetTheme(name).WithColorMode(mode)
				p := fmt.Sprintf("%s/%s/%s/", name, mode, pr.name)
				w(p+"name", th.Name())
				w(p+"color_mode", string(th.ColorMode()))
				w(p+"next", th.Next().Name()+" "+string(th.Next().ColorMode()))
				w(p+"with_auto", string(th.WithColorMode(ColorModeAuto).ColorMode()))
				pgStyles(w, p+"styles.", th.Styles())
				pgStyle(w, p+"get.Romulan", th.Romulan())
				pgStyle(w, p+"get.Tholian", th.Tholian())
				pgStyle(w, p+"get.PlasmaTorpedo", th.PlasmaTorpedo())
				pgStyle(w, p+"get.TholianWeb", th.TholianWeb())
				pgStyle(w, p+"get.AlertRed", th.AlertRed())
				pgStyle(w, p+"get.AlertYellow", th.AlertYellow())
				for ti, title := range titles {
					for gi, tag := range tags {
						for _, width := range widths {
							w(fmt.Sprintf("%sheader.%d.%d.%d", p, ti, gi, width), th.FormatHeader(title, tag, width))
						}
					}
				}
				for _, b := range badges {
					w(fmt.Sprintf("%sbadge.%d.%v", p, b.vol, b.muted), th.FormatAudioBadge(b.vol, b.muted))
				}
				if lc, ok := th.(LcarsTheme); ok {
					l, r := lc.PillCaps()
					w(p+"pillcaps", l+r)
				}
			}
		}
	}
	// PaletteStyles directly, independent of mode (TrueColor only).
	lipgloss.SetColorProfile(termenv.TrueColor)
	for _, th := range []Theme{ModernTheme{}, LcarsTheme{}, CrtTheme{}} {
		for _, dark := range []bool{true, false} {
			pgStyles(w, fmt.Sprintf("palette/%s/%v/", th.Name(), dark), th.PaletteStyles(dark))
		}
	}
}
