package main

import (
	"fmt"
	"hash/fnv"
	"runtime"
	"strconv"
	"strings"
	"sync"

	"github.com/charmbracelet/lipgloss"
	"github.com/muesli/termenv"
)

var profiles = []termenv.Profile{termenv.TrueColor, termenv.ANSI256, termenv.ANSI, termenv.Ascii}

func colorSeq(c termenv.Color, bg bool) string {
	if c == nil {
		return "nil"
	}
	return fmt.Sprintf("%T:%s", c, c.Sequence(bg))
}

// safeSeq reports "panic" where termenv itself panics (ANSI256 indices
// above 255 converted to the ANSI profile).
func safeSeq(p termenv.Profile, s string, bg bool) (out string) {
	defer func() {
		if recover() != nil {
			out = "panic"
		}
	}()
	return colorSeq(p.Color(s), bg)
}

func genColors(path string) {
	w := newRecWriter(path)
	defer w.close()

	// Exhaustive #rrggbb -> ANSI256 degradation, hashed per block of 65536
	// colours (FNV-1a 64 over one byte per colour). The ANSI (16-colour)
	// result is ansi256ToANSIColor of that value, checked via the table below.
	blocks := make([][]byte, 256)
	var wg sync.WaitGroup
	sem := make(chan struct{}, runtime.NumCPU())
	for blk := 0; blk < 256; blk++ {
		wg.Add(1)
		go func(blk int) {
			defer wg.Done()
			sem <- struct{}{}
			defer func() { <-sem }()
			buf := make([]byte, 65536)
			for i := 0; i < 65536; i++ {
				hex := fmt.Sprintf("#%06x", blk<<16|i)
				buf[i] = byte(termenv.ANSI256.Color(hex).(termenv.ANSI256Color))
			}
			blocks[blk] = buf
		}(blk)
	}
	wg.Wait()
	for blk := 0; blk < 256; blk++ {
		h := fnv.New64a()
		h.Write(blocks[blk])
		w.rec("op", "block", "r", strconv.Itoa(blk), "ansi256", fmt.Sprintf("%016x", h.Sum64()))
		if blk%32 == 0 {
			var sb strings.Builder
			for i := 0; i < 65536; i += 97 {
				sb.WriteString(strconv.Itoa(int(blocks[blk][i])))
				sb.WriteByte(',')
			}
			w.rec("op", "sample", "r", strconv.Itoa(blk), "ansi256", sb.String())
		}
	}

	// 256-colour -> 16-colour table and "#rgb" short forms.
	var sb strings.Builder
	for i := 0; i < 256; i++ {
		sb.WriteString(colorSeq(termenv.ANSI.Color(strconv.Itoa(i)), false))
		sb.WriteByte('\n')
	}
	w.rec("op", "ansi256to16", "out", sb.String())
	sb.Reset()
	for i := 0; i < 4096; i++ {
		hex := fmt.Sprintf("#%03x", i)
		for _, p := range profiles {
			sb.WriteString(colorSeq(p.Color(hex), i%2 == 0))
			sb.WriteByte(' ')
		}
		sb.WriteByte('\n')
	}
	h := fnv.New64a()
	h.Write([]byte(sb.String()))
	w.rec("op", "short", "hash", fmt.Sprintf("%016x", h.Sum64()), "head", sb.String()[:4096])

	inputs := []string{"", "0", "7", "8", "15", "16", "62", "230", "240", "255", "256", "1000", "-1", "-20", "+5", "abc", "1e2", " 5",
		"#fff", "#FFF", "#ffffff", "#FFFFFF", "#00E5FF", "#0B0F19", "#1E293B", "#ggg", "#12345", "#1234567", "#", "fff", "#ffg000"}
	for _, s := range inputs {
		for _, p := range profiles {
			w.rec("op", "profilecolor", "in", s, "p", strconv.Itoa(int(p)),
				"fg", safeSeq(p, s, false), "bg", safeSeq(p, s, true))
		}
	}

	// lipgloss TerminalColor.RGBA (uses the default renderer).
	lipgloss.SetColorProfile(termenv.TrueColor)
	lipgloss.SetHasDarkBackground(true)
	for _, s := range inputs {
		if n, err := strconv.Atoi(s); err == nil && (n > 255 || n < 0) {
			continue // termenv.ConvertToRGB panics
		}
		r, g, b, a := lipgloss.Color(s).RGBA()
		w.rec("op", "rgba", "in", s, "out", fmt.Sprintf("%d %d %d %d", r, g, b, a))
	}
	for _, dark := range []bool{true, false} {
		lipgloss.SetHasDarkBackground(dark)
		r, g, b, a := lipgloss.AdaptiveColor{Light: "#102030", Dark: "#405060"}.RGBA()
		w.rec("op", "rgba_adaptive", "dark", strconv.FormatBool(dark), "out", fmt.Sprintf("%d %d %d %d", r, g, b, a))
	}
	r, g, b, a := lipgloss.NoColor{}.RGBA()
	w.rec("op", "rgba_nocolor", "out", fmt.Sprintf("%d %d %d %d", r, g, b, a))
	lipgloss.SetColorProfile(termenv.Ascii)
}
