package anim

// Parity reference generator for the Rust port (crates/sst-tui/src/anim).
// Run with: SST_PARITYGEN=1 go test ./pkg/tui/anim -run TestParityGen -count=1
// Writes testdata/parity/anim/anim.txt at the repository root. Every line is
// "<key>\t<%q of value>". It records FrameDuration, BresenhamLine for every
// pair of sector coordinates, RedAlertBadgeStyle renders, and for every
// animation constructor with representative inputs and all speeds: the
// precomputed frames, then a Step-by-Step playback (state before/after each
// step and the returned frame), and Skip. Override styles are rendered under
// every colour profile.

import (
	"bufio"
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"testing"
	"time"

	"github.com/charmbracelet/lipgloss"
	"github.com/muesli/termenv"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
)

var pgProfiles = []termenv.Profile{termenv.Ascii, termenv.ANSI, termenv.ANSI256, termenv.TrueColor}

func pgRender(s lipgloss.Style, text string) string {
	parts := make([]string, 0, len(pgProfiles))
	for _, p := range pgProfiles {
		lipgloss.SetColorProfile(p)
		parts = append(parts, s.Render(text))
	}
	return strings.Join(parts, "|")
}

func pgFrame(f Frame) string {
	var b strings.Builder
	fmt.Fprintf(&b, "dur=%d n=%d", int64(f.Duration), len(f.Overrides))
	keys := make([]engine.Coord, 0, len(f.Overrides))
	for k := range f.Overrides {
		keys = append(keys, k)
	}
	sort.Slice(keys, func(i, j int) bool {
		if keys[i][0] != keys[j][0] {
			return keys[i][0] < keys[j][0]
		}
		return keys[i][1] < keys[j][1]
	})
	for _, k := range keys {
		ov := f.Overrides[k]
		fmt.Fprintf(&b, " [%d,%d]%q=%s", k[0], k[1], ov.Glyph, pgRender(ov.Style, ov.Glyph))
	}
	return b.String()
}

type pgExtra func() string

func pgAnim(w func(k, v string), key string, a Animation, extra pgExtra) {
	frames := a.Frames()
	w(key+".total", fmt.Sprintf("%d frames=%d finished=%v", int64(a.TotalDuration()), len(frames), a.IsFinished()))
	for i, f := range frames {
		w(fmt.Sprintf("%s.frame%d", key, i), pgFrame(f))
	}
	for i := 0; i <= len(frames)+1; i++ {
		pre := ""
		if extra != nil {
			pre = extra()
		}
		before := a.IsFinished()
		f := a.Step()
		w(fmt.Sprintf("%s.step%d", key, i), fmt.Sprintf("before=%v after=%v %s %s", before, a.IsFinished(), pre, pgFrame(f)))
	}
}

func pgAnimSkip(w func(k, v string), key string, a Animation, steps int, extra pgExtra) {
	for i := 0; i < steps; i++ {
		a.Step()
	}
	f := a.Skip()
	post := ""
	if extra != nil {
		post = extra()
	}
	w(fmt.Sprintf("%s.skip%d", key, steps), fmt.Sprintf("finished=%v %s %s", a.IsFinished(), post, pgFrame(f)))
	f = a.Step()
	w(fmt.Sprintf("%s.skip%d.step", key, steps), fmt.Sprintf("finished=%v %s", a.IsFinished(), pgFrame(f)))
}

func TestParityGen(t *testing.T) {
	if os.Getenv("SST_PARITYGEN") != "1" {
		t.Skip("set SST_PARITYGEN=1 to regenerate parity data")
	}
	out := filepath.Join("..", "..", "..", "testdata", "parity", "anim")
	if err := os.MkdirAll(out, 0o755); err != nil {
		t.Fatal(err)
	}
	f, err := os.Create(filepath.Join(out, "anim.txt"))
	if err != nil {
		t.Fatal(err)
	}
	defer f.Close()
	bw := bufio.NewWriter(f)
	defer bw.Flush()
	w := func(k, v string) { fmt.Fprintf(bw, "%s\t%q\n", k, v) }
	defer lipgloss.SetColorProfile(termenv.Ascii)
	lipgloss.SetHasDarkBackground(true)

	speeds := []int{-1, 0, 1, 2, 3, 4}
	for _, s := range []int{-5, -1, 0, 1, 2, 3, 4, 99} {
		w(fmt.Sprintf("frameduration.%d", s), fmt.Sprint(int64(FrameDuration(s))))
	}

	// Bresenham for every pair of in-quadrant coordinates, plus some off-grid.
	pts := []engine.Coord{}
	for r := 1; r <= 8; r++ {
		for c := 1; c <= 8; c++ {
			pts = append(pts, engine.Coord{r, c})
		}
	}
	for _, a := range pts {
		var b strings.Builder
		for _, e := range pts {
			line := BresenhamLine(a, e)
			for _, p := range line {
				fmt.Fprintf(&b, "%d%d", p[0], p[1])
			}
			b.WriteByte(' ')
		}
		w(fmt.Sprintf("bresenham.%d.%d", a[0], a[1]), b.String())
	}
	odd := [][2]engine.Coord{{{0, 0}, {10, 3}}, {{-2, 5}, {7, -4}}, {{12, 12}, {1, 1}}, {{0, 0}, {0, 0}}}
	for i, pr := range odd {
		w(fmt.Sprintf("bresenham.odd%d", i), fmt.Sprint(BresenhamLine(pr[0], pr[1])))
	}

	for c := -1; c <= 4; c++ {
		s := RedAlertBadgeStyle(c)
		w(fmt.Sprintf("redalert.%d", c), fmt.Sprintf("bold=%v %s", s.GetBold(), pgRender(s, "RED")))
	}

	// Phaser.
	phaserCases := []struct {
		start   engine.Coord
		targets []engine.Coord
		hits    []bool
	}{
		{engine.Coord{3, 1}, []engine.Coord{{3, 5}}, []bool{true}},
		{engine.Coord{1, 4}, []engine.Coord{{5, 4}}, []bool{true}},
		{engine.Coord{1, 1}, []engine.Coord{{4, 4}}, []bool{false}},
		{engine.Coord{1, 4}, []engine.Coord{{4, 1}}, []bool{true}},
		{engine.Coord{4, 4}, []engine.Coord{{4, 7}, {1, 4}, {7, 7}}, []bool{true, false, true}},
		{engine.Coord{4, 4}, []engine.Coord{{4, 5}, {5, 5}, {8, 1}, {2, 8}, {1, 2}}, []bool{true, true}},
		{engine.Coord{5, 5}, []engine.Coord{{5, 5}}, []bool{true}},
		{engine.Coord{2, 2}, []engine.Coord{{8, 5}, {5, 8}, {8, 3}}, []bool{false, true, true, true}},
		{engine.Coord{4, 4}, nil, nil},
		{engine.Coord{4, 4}, []engine.Coord{{4, 8}, {4, 8}}, []bool{false, true}},
	}
	for i, pc := range phaserCases {
		for _, sp := range speeds {
			key := fmt.Sprintf("phaser.%d.s%d", i, sp)
			pgAnim(w, key, NewMultiPhaserAnimation(pc.start, pc.targets, pc.hits, sp), nil)
			pgAnimSkip(w, key, NewMultiPhaserAnimation(pc.start, pc.targets, pc.hits, sp), 1, nil)
		}
		if len(pc.targets) > 0 {
			pgAnim(w, fmt.Sprintf("phaser1.%d", i), NewPhaserAnimation(pc.start, pc.targets[0], len(pc.hits) > 0 && pc.hits[0], 2), nil)
		}
	}

	// Torpedo.
	torpCases := [][2]engine.Coord{{{1, 1}, {1, 4}}, {{1, 1}, {1, 2}}, {{3, 3}, {3, 3}}, {{8, 8}, {1, 1}}, {{2, 7}, {6, 1}}, {{4, 4}, {5, 6}}}
	entities := []engine.EntityType{engine.EntityKlingon, engine.EntityRomulan, engine.EntityTholian, engine.EntityStarbase,
		engine.EntityStar, engine.EntityPlanet, engine.EntityBlackHole, engine.EntityPlasmaTorpedo, engine.EntityTholianWeb,
		engine.EntityEmpty, engine.EntityCommander, engine.EntityWormhole}
	for i, tc := range torpCases {
		for _, hit := range []bool{true, false} {
			for _, sp := range speeds {
				key := fmt.Sprintf("torpedo.%d.%v.s%d", i, hit, sp)
				pgAnim(w, key, NewTorpedoAnimation(tc[0], tc[1], hit, sp), nil)
				pgAnimSkip(w, key, NewTorpedoAnimation(tc[0], tc[1], hit, sp), 2, nil)
			}
			for _, e := range entities {
				key := fmt.Sprintf("torpedoE.%d.%v.e%d", i, hit, int(e))
				pgAnim(w, key, NewTorpedoAnimationWithEntity(tc[0], tc[1], hit, e, 2), nil)
			}
		}
	}

	// Cloak.
	for _, cloaking := range []bool{true, false} {
		coord := engine.Coord{3, 6}
		for _, sp := range append([]int{-3, 7}, speeds...) {
			key := fmt.Sprintf("cloak.%v.s%d", cloaking, sp)
			c := NewCloakAnimationWithSpeed(coord, cloaking, sp)
			ex := func() string {
				return fmt.Sprintf("glyph=%q done=%v coord=%v cloaking=%v", c.CurrentGlyph(), c.Done(), c.Coord(), c.IsCloaking())
			}
			pgAnim(w, key, c, ex)
			c2 := NewCloakAnimationWithSpeed(coord, cloaking, sp)
			pgAnimSkip(w, key, c2, 1, func() string { return fmt.Sprintf("glyph=%q done=%v", c2.CurrentGlyph(), c2.Done()) })
		}
		c := NewCloakAnimation(engine.Coord{1, 8}, cloaking)
		pgAnim(w, fmt.Sprintf("cloakdef.%v", cloaking), c, func() string { return fmt.Sprintf("glyph=%q", c.CurrentGlyph()) })
		ct := NewCloakAnimation(engine.Coord{2, 2}, cloaking)
		for i := 0; i < 6; i++ {
			fr := ct.Tick()
			w(fmt.Sprintf("cloaktick.%v.%d", cloaking, i), fmt.Sprintf("%q %v %s", ct.CurrentGlyph(), ct.Done(), pgFrame(fr)))
		}
	}

	// Web.
	for _, action := range []WebAction{WebActionWeave, WebActionBreach, WebAction(7)} {
		w(fmt.Sprintf("webaction.%d", int(action)), action.String())
		coord := engine.Coord{6, 2}
		for _, sp := range append([]int{-3, 7}, speeds...) {
			key := fmt.Sprintf("web.%d.s%d", int(action), sp)
			wa := NewWebAnimationWithSpeed(coord, action, sp)
			ex := func() string {
				return fmt.Sprintf("glyph=%q done=%v coord=%v action=%s", wa.CurrentGlyph(), wa.Done(), wa.Coord(), wa.Action())
			}
			pgAnim(w, key, wa, ex)
			w2 := NewWebAnimationWithSpeed(coord, action, sp)
			pgAnimSkip(w, key, w2, 2, func() string { return fmt.Sprintf("glyph=%q done=%v", w2.CurrentGlyph(), w2.Done()) })
		}
		wd := NewWebAnimation(engine.Coord{8, 8}, action)
		pgAnim(w, fmt.Sprintf("webdef.%d", int(action)), wd, func() string { return fmt.Sprintf("glyph=%q", wd.CurrentGlyph()) })
	}
	w(fmt.Sprintf("webaction.%d", -1), WebAction(-1).String())

	// Plasma.
	plasmaCases := [][2]engine.Coord{{{2, 2}, {4, 4}}, {{1, 1}, {1, 3}}, {{5, 5}, {5, 5}}, {{8, 1}, {1, 7}}, {{3, 7}, {6, 2}}}
	durs := []time.Duration{-time.Second, -1, 0, 1, 50 * time.Millisecond, 100 * time.Millisecond, 240 * time.Millisecond}
	for i, pc := range plasmaCases {
		for di, d := range durs {
			key := fmt.Sprintf("plasma.%d.d%d", i, di)
			p := NewPlasmaAnimation(pc[0], pc[1], d)
			ex := func() string {
				return fmt.Sprintf("cur=%v glyph=%q done=%v from=%v to=%v path=%v", p.CurrentCoord(), p.CurrentGlyph(), p.Done(), p.From(), p.To(), p.Path())
			}
			pgAnim(w, key, p, ex)
			p2 := NewPlasmaAnimation(pc[0], pc[1], d)
			pgAnimSkip(w, key, p2, 1, func() string { return fmt.Sprintf("cur=%v done=%v", p2.CurrentCoord(), p2.Done()) })
		}
	}
}
