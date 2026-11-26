package main

import (
	"fmt"
	"strconv"
	"strings"

	"github.com/charmbracelet/lipgloss"
	"github.com/muesli/termenv"
)

// Style specs: ops separated by ';', each "name=arg,arg". The Rust test
// parses the same specs (tests/lipgloss_parity.rs).

func parseColor(s string) lipgloss.TerminalColor {
	switch {
	case s == "none":
		return lipgloss.NoColor{}
	case strings.HasPrefix(s, "a:"):
		p := strings.Split(s[2:], "|")
		return lipgloss.AdaptiveColor{Light: p[0], Dark: p[1]}
	case strings.HasPrefix(s, "c:"):
		p := strings.Split(s[2:], "|")
		return lipgloss.CompleteColor{TrueColor: p[0], ANSI256: p[1], ANSI: p[2]}
	case strings.HasPrefix(s, "ca:"):
		lr := strings.Split(s[3:], "/")
		l := strings.Split(lr[0], "|")
		d := strings.Split(lr[1], "|")
		return lipgloss.CompleteAdaptiveColor{
			Light: lipgloss.CompleteColor{TrueColor: l[0], ANSI256: l[1], ANSI: l[2]},
			Dark:  lipgloss.CompleteColor{TrueColor: d[0], ANSI256: d[1], ANSI: d[2]},
		}
	case strings.HasPrefix(s, "u:"):
		n, _ := strconv.Atoi(s[2:])
		return lipgloss.ANSIColor(n)
	}
	return lipgloss.Color(s)
}

func borderByName(n string) lipgloss.Border {
	switch n {
	case "normal":
		return lipgloss.NormalBorder()
	case "rounded":
		return lipgloss.RoundedBorder()
	case "block":
		return lipgloss.BlockBorder()
	case "outer":
		return lipgloss.OuterHalfBlockBorder()
	case "inner":
		return lipgloss.InnerHalfBlockBorder()
	case "thick":
		return lipgloss.ThickBorder()
	case "double":
		return lipgloss.DoubleBorder()
	case "hidden":
		return lipgloss.HiddenBorder()
	case "markdown":
		return lipgloss.MarkdownBorder()
	case "ascii":
		return lipgloss.ASCIIBorder()
	case "custom":
		return lipgloss.Border{Top: "=-", Bottom: "日", Left: "|:", Right: "", TopLeft: "<>", TopRight: "", BottomLeft: "*", BottomRight: "★"}
	}
	return lipgloss.Border{}
}

func atoi(s string) int {
	n, err := strconv.Atoi(s)
	if err != nil {
		panic(err)
	}
	return n
}

func pos(s string) lipgloss.Position {
	f, err := strconv.ParseFloat(s, 64)
	if err != nil {
		panic(err)
	}
	return lipgloss.Position(f)
}

func applySpec(spec string) lipgloss.Style {
	st := lipgloss.NewStyle()
	if spec == "" {
		return st
	}
	for _, op := range strings.Split(spec, ";") {
		name, arg, _ := strings.Cut(op, "=")
		args := strings.Split(arg, ",")
		b := arg == "1"
		var ints []int
		var colors []lipgloss.TerminalColor
		for _, a := range args {
			if n, err := strconv.Atoi(a); err == nil {
				ints = append(ints, n)
			}
			colors = append(colors, parseColor(a))
		}
		switch name {
		case "bold":
			st = st.Bold(b)
		case "italic":
			st = st.Italic(b)
		case "underline":
			st = st.Underline(b)
		case "strike":
			st = st.Strikethrough(b)
		case "reverse":
			st = st.Reverse(b)
		case "blink":
			st = st.Blink(b)
		case "faint":
			st = st.Faint(b)
		case "fg":
			st = st.Foreground(parseColor(arg))
		case "bg":
			st = st.Background(parseColor(arg))
		case "marginbg":
			st = st.MarginBackground(parseColor(arg))
		case "width":
			st = st.Width(atoi(arg))
		case "height":
			st = st.Height(atoi(arg))
		case "maxw":
			st = st.MaxWidth(atoi(arg))
		case "maxh":
			st = st.MaxHeight(atoi(arg))
		case "tabw":
			st = st.TabWidth(atoi(arg))
		case "align":
			var ps []lipgloss.Position
			for _, a := range args {
				ps = append(ps, pos(a))
			}
			st = st.Align(ps...)
		case "alignh":
			st = st.AlignHorizontal(pos(arg))
		case "alignv":
			st = st.AlignVertical(pos(arg))
		case "pad":
			st = st.Padding(ints...)
		case "padl":
			st = st.PaddingLeft(atoi(arg))
		case "padr":
			st = st.PaddingRight(atoi(arg))
		case "padt":
			st = st.PaddingTop(atoi(arg))
		case "padb":
			st = st.PaddingBottom(atoi(arg))
		case "margin":
			st = st.Margin(ints...)
		case "marginl":
			st = st.MarginLeft(atoi(arg))
		case "marginr":
			st = st.MarginRight(atoi(arg))
		case "margint":
			st = st.MarginTop(atoi(arg))
		case "marginb":
			st = st.MarginBottom(atoi(arg))
		case "border":
			var sides []bool
			for _, a := range args[1:] {
				sides = append(sides, a == "1")
			}
			st = st.Border(borderByName(args[0]), sides...)
		case "borderstyle":
			st = st.BorderStyle(borderByName(arg))
		case "bordertop":
			st = st.BorderTop(b)
		case "borderright":
			st = st.BorderRight(b)
		case "borderbottom":
			st = st.BorderBottom(b)
		case "borderleft":
			st = st.BorderLeft(b)
		case "borderfg":
			st = st.BorderForeground(colors...)
		case "borderbg":
			st = st.BorderBackground(colors...)
		case "btfg":
			st = st.BorderTopForeground(parseColor(arg))
		case "blfg":
			st = st.BorderLeftForeground(parseColor(arg))
		case "bbbg":
			st = st.BorderBottomBackground(parseColor(arg))
		case "inline":
			st = st.Inline(b)
		case "ulspaces":
			st = st.UnderlineSpaces(b)
		case "stspaces":
			st = st.StrikethroughSpaces(b)
		case "colorws":
			st = st.ColorWhitespace(b)
		case "setstring":
			st = st.SetString(args...)
		case "transform":
			st = st.Transform(strings.ToUpper)
		case "unsetbold":
			st = st.UnsetBold()
		case "unsetpad":
			st = st.UnsetPadding()
		case "unsetborderstyle":
			st = st.UnsetBorderStyle()
		case "inheritfrom":
			// Inherit from a small fixed style.
			st = st.Inherit(lipgloss.NewStyle().Bold(true).Underline(true).Foreground(lipgloss.Color("#00ff00")).Background(lipgloss.Color("#000080")).Padding(1).Margin(2).Width(7))
		default:
			panic("unknown op " + name)
		}
	}
	return st
}

func colorName(c lipgloss.TerminalColor) string {
	switch v := c.(type) {
	case lipgloss.NoColor:
		return "NoColor"
	case lipgloss.Color:
		return "Color(" + string(v) + ")"
	case lipgloss.ANSIColor:
		return fmt.Sprintf("ANSIColor(%d)", uint(v))
	case lipgloss.AdaptiveColor:
		return "Adaptive(" + v.Light + "|" + v.Dark + ")"
	case lipgloss.CompleteColor:
		return "Complete(" + v.TrueColor + "|" + v.ANSI256 + "|" + v.ANSI + ")"
	case lipgloss.CompleteAdaptiveColor:
		return "CompleteAdaptive"
	case nil:
		return "nil"
	}
	return "?"
}

func getters(st lipgloss.Style) string {
	t, r, b, l := st.GetPadding()
	mt, mr, mb, ml := st.GetMargin()
	_, bt, br, bb, bl := st.GetBorder()
	return fmt.Sprintf("pad=%d,%d,%d,%d margin=%d,%d,%d,%d border=%v,%v,%v,%v hb=%d vb=%d hp=%d vp=%d hm=%d vm=%d hf=%d vf=%d bts=%d brs=%d bbs=%d bls=%d w=%d h=%d mw=%d mh=%d tw=%d a=%v av=%v bold=%v italic=%v ul=%v rev=%v inline=%v cws=%v uls=%v fg=%s bg=%s btfg=%s blfg=%s bbbg=%s val=%q",
		t, r, b, l, mt, mr, mb, ml, bt, br, bb, bl,
		st.GetHorizontalBorderSize(), st.GetVerticalBorderSize(), st.GetHorizontalPadding(), st.GetVerticalPadding(),
		st.GetHorizontalMargins(), st.GetVerticalMargins(), st.GetHorizontalFrameSize(), st.GetVerticalFrameSize(),
		st.GetBorderTopSize(), st.GetBorderRightSize(), st.GetBorderBottomSize(), st.GetBorderLeftSize(),
		st.GetWidth(), st.GetHeight(), st.GetMaxWidth(), st.GetMaxHeight(), st.GetTabWidth(),
		float64(st.GetAlign()), float64(st.GetAlignVertical()),
		st.GetBold(), st.GetItalic(), st.GetUnderline(), st.GetReverse(), st.GetInline(), st.GetColorWhitespace(), st.GetUnderlineSpaces(),
		colorName(st.GetForeground()), colorName(st.GetBackground()), colorName(st.GetBorderTopForeground()),
		colorName(st.GetBorderLeftForeground()), colorName(st.GetBorderBottomBackground()), st.Value())
}

type cfg struct {
	p    termenv.Profile
	dark bool
}

func (c cfg) key() string {
	d := "l"
	if c.dark {
		d = "d"
	}
	return "o" + strconv.Itoa(int(c.p)) + d
}

func allCfgs() []cfg {
	var out []cfg
	for _, p := range profiles {
		for _, d := range []bool{true, false} {
			out = append(out, cfg{p, d})
		}
	}
	return out
}

func setCfg(c cfg) {
	lipgloss.SetColorProfile(c.p)
	lipgloss.SetHasDarkBackground(c.dark)
}

func lipglossInputs() []string {
	return []string{
		"",
		"hello",
		"hello world",
		"line one\nline two\nthree",
		"The quick brown fox jumps over the lazy dog near the riverbank",
		"日本語 wide ★ chars",
		"\x1b[1mpre\x1b[0m styled \x1b[38;2;255;0;0mred\x1b[0m text that is long",
		"tabs\there\tok",
		"  padded  ",
		"⚠️ alert 🔊 75%",
		"crlf\r\nnext",
		"a\n\nb",
		"COMMAND> nav 1 2",
		"supercalifragilisticexpialidocious",
	}
}

func genLipgloss(path string) {
	w := newRecWriter(path)
	defer w.close()
	cfgs := allCfgs()

	gotGetters := map[string]bool{}
	// Outputs identical to an earlier configuration's are stored as
	// "\x00same:<key>" to keep the file small.
	renderAll := func(spec, in string, cs []cfg, withGetters bool) {
		kv := []string{"op", "render", "spec", spec, "in", in}
		seen := map[string]string{}
		for _, c := range cs {
			setCfg(c)
			out := applySpec(spec).Render(in)
			if k, ok := seen[out]; ok {
				kv = append(kv, c.key(), "\x00same:"+k)
				continue
			}
			seen[out] = c.key()
			kv = append(kv, c.key(), out)
		}
		w.rec(kv...)
		if withGetters && !gotGetters[spec] {
			gotGetters[spec] = true
			setCfg(cfg{termenv.Ascii, true})
			w.rec("op", "getters", "spec", spec, "get", getters(applySpec(spec)))
		}
	}

	single := []string{
		"", "bold=1", "italic=1", "underline=1", "strike=1", "reverse=1", "blink=1", "faint=1", "bold=0",
		"bold=1;italic=1;underline=1;strike=1;reverse=1;blink=1;faint=1",
		"fg=#FF0000", "fg=#00E5FF", "fg=#0f0", "fg=9", "fg=240", "fg=none", "fg=", "fg=bogus",
		"fg=a:#1a1a1a|#dddddd", "fg=c:#ff0000|196|9", "fg=ca:#ff0000|196|9/#00ff00|46|10", "fg=u:62",
		"bg=#0B0F19", "bg=62", "bg=3", "fg=230;bg=62;pad=0,1",
		"underline=1;ulspaces=0", "strike=1;stspaces=0", "underline=1;fg=#ff0000;bg=#000000",
		"ulspaces=1", "stspaces=1;fg=#abcdef", "reverse=1;bg=#123456;pad=1,2", "reverse=1;fg=#123456;pad=0,2;width=20",
		"colorws=0;bg=#123456;pad=1,2;width=20", "colorws=1;bg=#123456;pad=1,2;width=20",
		"width=10", "width=1", "width=0", "width=-3", "width=25;align=0.5", "width=25;align=1", "width=25;align=0.25",
		"width=25;alignh=0.5;alignv=1;height=6", "height=5", "height=5;alignv=0.5", "height=5;alignv=1", "height=2",
		"height=4;align=0,0.5", "maxw=5", "maxh=2", "maxw=8;maxh=1;border=rounded", "inline=1",
		"inline=1;width=10;border=normal;pad=1;margin=1", "tabw=0", "tabw=-1", "tabw=-5", "tabw=2",
		"pad=1", "pad=1,2", "pad=1,2,3", "pad=1,2,3,4", "pad=1,2,3,4,5", "pad=-1", "padl=3", "padr=2;padt=1;padb=2",
		"margin=1,2", "margin=1,2,3,4;marginbg=#ff0000", "marginbg=#ff0000;margin=1", "marginl=2;marginr=1;margint=1;marginb=1",
		"border=normal", "border=rounded;borderfg=#1E293B", "border=thick;borderfg=#f00,#0f0,#00f,#ff0;borderbg=#111,#222",
		"border=double;borderbg=4", "border=normal,0,0,0,1", "border=normal,1,0", "border=normal,1,1,0", "border=normal,0,1,0,1",
		"border=normal,1,0,1,0", "border=normal,0,0,1,0", "border=normal,1,1,1,1,1",
		"borderstyle=rounded", "borderstyle=rounded;bordertop=1", "borderstyle=block;borderleft=1;borderright=1",
		"borderstyle=normal;bordertop=0", "bordertop=1;borderbottom=1",
		"border=hidden", "border=outer", "border=inner", "border=markdown", "border=ascii", "border=none", "border=custom",
		"border=normal;borderfg=none;borderbg=none", "border=normal;borderfg=#ff0000;borderbg=#00ff00",
		"border=normal;btfg=#ff0000;blfg=9;bbbg=a:#ffffff|#000000", "border=rounded;borderfg=a:#F793FF|#AD58B4",
		"setstring=VALUE", "setstring=two,words;bold=1", "transform=upper;fg=#ff00ff",
		"bold=1;unsetbold", "pad=2;unsetpad", "border=normal;unsetborderstyle",
		"inheritfrom", "bold=0;fg=#ff0000;pad=3;inheritfrom", "border=normal;inheritfrom;width=12",
		"fg=#00E5FF;bg=#0B0F19;pad=0,1;border=rounded;borderfg=#1E293B;width=30;height=4;margin=1",
		"bold=1;fg=#00E5FF;bg=#0B0F19;pad=0,1",
		"border=normal,0,0,0,1;borderfg=#00E5FF;pad=0,0,0,1;bold=1;fg=#00E5FF",
	}
	inputs := lipglossInputs()
	hasColor := func(spec string) bool {
		for _, k := range []string{"fg=", "bg=", "borderfg=", "borderbg=", "marginbg=", "btfg=", "blfg=", "bbbg=", "inheritfrom"} {
			if strings.Contains(spec, k) {
				return true
			}
		}
		return false
	}
	for _, spec := range single {
		cs := cfgs
		if !hasColor(spec) {
			cs = []cfg{{termenv.Ascii, true}, {termenv.TrueColor, true}, {termenv.ANSI, false}}
		}
		for _, in := range inputs[:10] {
			renderAll(spec, in, cs, true)
		}
	}

	borders := []string{"", "border=normal", "border=rounded", "border=normal,0,0,0,1", "border=thick,1,0"}
	pads := []string{"", "pad=0,1", "pad=1,2,0,3"}
	margins := []string{"", "margin=1", "margin=0,2"}
	widths := []string{"", "width=12", "width=24"}
	heights := []string{"", "height=4"}
	aligns := []string{"", "align=1,0.5"}
	colorSets := []string{"", "fg=#00E5FF;bg=#0B0F19", "bold=1;fg=62;borderfg=#1E293B"}
	n := 0
	for _, b := range borders {
		for _, p := range pads {
			for _, m := range margins {
				for _, wd := range widths {
					for _, h := range heights {
						for _, a := range aligns {
							for ci, col := range colorSets {
								var parts []string
								for _, x := range []string{b, p, m, wd, h, a, col} {
									if x != "" {
										parts = append(parts, x)
									}
								}
								spec := strings.Join(parts, ";")
								in := inputs[n%len(inputs)]
								n++
								cs := []cfg{{termenv.Ascii, true}, {termenv.TrueColor, true}}
								if ci != 0 {
									cs = cfgs
								}
								renderAll(spec, in, cs, false)
							}
						}
					}
				}
			}
		}
	}

	// Join / Place.
	setCfg(cfg{termenv.Ascii, true})
	box := applySpec("border=rounded;pad=0,1").Render("box\ncontent")
	blocks := []string{"a", "bb\nbbb", "c\nc\nc\nc", box, "日本\nx", "", "\x1b[1mbold\x1b[0m\nline"}
	positions := []string{"0", "0.5", "1", "0.25", "0.75", "0.3333", "-1", "2"}
	for _, ps := range positions {
		p := pos(ps)
		for i := range blocks {
			for j := range blocks {
				w.rec("op", "joinh", "pos", ps, "a", blocks[i], "b", blocks[j], "out", lipgloss.JoinHorizontal(p, blocks[i], blocks[j]))
				w.rec("op", "joinv", "pos", ps, "a", blocks[i], "b", blocks[j], "out", lipgloss.JoinVertical(p, blocks[i], blocks[j]))
			}
		}
		w.rec("op", "joinh3", "pos", ps, "out", lipgloss.JoinHorizontal(p, blocks[0], blocks[3], blocks[2]))
		w.rec("op", "joinv3", "pos", ps, "out", lipgloss.JoinVertical(p, blocks[1], blocks[3], blocks[4]))
	}
	w.rec("op", "joinh0", "out", lipgloss.JoinHorizontal(lipgloss.Top))
	w.rec("op", "joinh1", "out", lipgloss.JoinHorizontal(lipgloss.Top, "solo\nx"))

	placeStrs := []string{"x", "two\nlines", box, "日本語"}
	wsOpts := []string{"", "chars=.", "chars=日", "chars=ab", "fg=#ff0000", "bg=#0000ff;chars=-"}
	for _, ws := range wsOpts {
		for _, s := range placeStrs {
			for _, wd := range []int{0, 5, 20} {
				for _, h := range []int{0, 4, 8} {
					for _, hp := range []string{"0", "1", "0.3"} {
						for _, vp := range []string{"0", "0.5", "0.7"} {
							var opts []lipgloss.WhitespaceOption
							for _, o := range strings.Split(ws, ";") {
								if o == "" {
									continue
								}
								k, v, _ := strings.Cut(o, "=")
								switch k {
								case "chars":
									opts = append(opts, lipgloss.WithWhitespaceChars(v))
								case "fg":
									opts = append(opts, lipgloss.WithWhitespaceForeground(lipgloss.Color(v)))
								case "bg":
									opts = append(opts, lipgloss.WithWhitespaceBackground(lipgloss.Color(v)))
								}
							}
							kv := []string{"op", "place", "ws", ws, "in", s, "w", strconv.Itoa(wd), "h", strconv.Itoa(h), "hp", hp, "vp", vp}
							cs := []cfg{{termenv.Ascii, true}}
							if strings.Contains(ws, "fg") || strings.Contains(ws, "bg") {
								cs = append(cs, cfg{termenv.TrueColor, true}, cfg{termenv.ANSI, true})
							}
							for _, c := range cs {
								setCfg(c)
								kv = append(kv, c.key(), lipgloss.Place(wd, h, pos(hp), pos(vp), s, opts...))
							}
							w.rec(kv...)
							if h == 0 {
								setCfg(cfg{termenv.Ascii, true})
								w.rec("op", "placeh", "ws", ws, "in", s, "w", strconv.Itoa(wd), "hp", hp,
									"out", lipgloss.PlaceHorizontal(wd, pos(hp), s, opts...))
								w.rec("op", "placev", "ws", ws, "in", s, "h", strconv.Itoa(wd), "vp", vp,
									"out", lipgloss.PlaceVertical(wd, pos(vp), s, opts...))
							}
						}
					}
				}
			}
		}
	}

	// Size helpers and StyleRunes / StyleRanges.
	for _, s := range append(blocks, inputs...) {
		wd, h := lipgloss.Size(s)
		w.rec("op", "size", "in", s, "out", fmt.Sprintf("%d %d %d %d", lipgloss.Width(s), lipgloss.Height(s), wd, h))
	}
	for _, c := range cfgs {
		setCfg(c)
		m := lipgloss.NewStyle().Underline(true).Foreground(lipgloss.Color("#ff0000"))
		u := lipgloss.NewStyle().Foreground(lipgloss.Color("#00ff00"))
		w.rec("op", "stylerunes", "cfg", c.key(), "out", lipgloss.StyleRunes("Hello, 世界! nav", []int{0, 1, 4, 7, 8, 9, 13, 14}, m, u))
		w.rec("op", "styleranges", "cfg", c.key(), "out", lipgloss.StyleRanges("\x1b[1mbold text\x1b[0m and more", lipgloss.NewRange(0, 4, m), lipgloss.NewRange(10, 13, u)))
	}
	setCfg(cfg{termenv.Ascii, true})
}
