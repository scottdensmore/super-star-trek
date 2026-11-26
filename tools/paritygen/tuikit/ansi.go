package main

import (
	"strconv"
	"strings"

	"github.com/charmbracelet/lipgloss"
	"github.com/charmbracelet/x/ansi"
	"github.com/charmbracelet/x/cellbuf"
	"github.com/clipperhouse/uax29/v2/graphemes"
	"github.com/rivo/uniseg"
)

// widthInputs are strings exercising ANSI sequences, wide characters,
// grapheme clusters and every non-ASCII rune that pkg/tui prints.
func widthInputs() []string {
	game := "±²·Δπ—•…←↑→↓─│┌┐└┘█░■▲▶▼◀◖◗★⚙⚠️🔇🔊"
	in := []string{
		"",
		"a",
		"hello world",
		game,
		"\x1b[1mbold\x1b[0m",
		"\x1b[38;2;255;0;0mred\x1b[0m and \x1b[48;5;62mbg\x1b[0m",
		"\x1b[1;38;2;0;229;255mCOMMAND> \x1b[0m\x1b[38;2;226;232;240mnav 1 2\x1b[0m",
		"\x1b]8;;https://example.com\x1b\\link\x1b]8;;\x1b\\ tail",
		"\x1b]0;title\a after",
		"tab\there",
		"日本語テキスト",
		"mixed 日本 and ascii",
		"é combining",
		"👍🏽 skin tone",
		"👨‍👩‍👧 family",
		"🇺🇸🇯🇵 flags",
		"⚠️ warning ⚠ plain",
		"❤️❤ hearts",
		"🔊 75% 🔇",
		"★ STARDATE 2265.3 ★",
		"│ ┌──┐ │",
		"█████░░░░░",
		"▲▶▼◀ ◖LCARS◗",
		"Δ ±0.5 π² · …",
		"\x1b[7mreverse\x1b[27m normal",
		"\x1b[31m日本\x1b[0m語",
		"control\x07bell\x00nul",
		" nbsp ",
		"zero​width‍joiner",
		"ﾊﾝｶｸ half-width",
		"Ｆｕｌｌ fullwidth",
		"한국어 hangul",
		"각 jamo",
		"⸺⸻ dashes",
		"\x1b[38;5;196m\x1b[48;2;1;2;3mx\x1b[m",
		"\x1bPq#0;2;0;0;0#1;2;100;100;0\x1b\\dcs",
		"\x1b[?25l\x1b[2Jscreen",
		"\x1b7saved\x1b8",
		"\x1b(Bcharset",
	}
	return in
}

func genAnsi(path string) {
	w := newRecWriter(path)
	defer w.close()

	inputs := widthInputs()
	tails := []string{"", "…", "...", "\x1b[0m>"}
	for _, s := range inputs {
		w.rec("op", "width", "in", s,
			"ansi", strconv.Itoa(ansi.StringWidth(s)),
			"lipgloss", strconv.Itoa(lipgloss.Width(s)),
			"uniseg", strconv.Itoa(uniseg.StringWidth(s)),
			"strip", ansi.Strip(s))

		// Grapheme cluster boundaries (uax29) of the stripped text.
		var cl []string
		g := graphemes.FromString(ansi.Strip(s))
		for g.Next() {
			cl = append(cl, strconv.Itoa(len(g.Value())))
		}
		var ucl []string
		state := -1
		rest := ansi.Strip(s)
		for len(rest) > 0 {
			var c string
			var wd int
			c, rest, wd, state = uniseg.FirstGraphemeClusterInString(rest, state)
			ucl = append(ucl, strconv.Itoa(len(c))+":"+strconv.Itoa(wd))
		}
		w.rec("op", "clusters", "in", ansi.Strip(s), "uax29", strings.Join(cl, ","), "uniseg", strings.Join(ucl, ","))

		sw := ansi.StringWidth(s)
		for n := -1; n <= sw+1; n++ {
			for _, t := range tails {
				w.rec("op", "truncate", "in", s, "n", strconv.Itoa(n), "tail", t, "out", ansi.Truncate(s, n, t))
				w.rec("op", "truncateleft", "in", s, "n", strconv.Itoa(n), "tail", t, "out", ansi.TruncateLeft(s, n, t))
			}
			for m := n; m <= sw+1; m += 5 {
				w.rec("op", "cut", "in", s, "n", strconv.Itoa(n), "m", strconv.Itoa(m), "out", ansi.Cut(s, n, m))
			}
		}
	}

	wrapInputs := append(inputs,
		"The quick brown fox jumps over the lazy dog",
		"supercalifragilisticexpialidocious is a long word",
		"hyphen-ated words-and-more-hyphens here",
		"  leading and trailing spaces  ",
		"multiple   spaces    between",
		"line one\nline two is longer\n\nafter blank",
		"\x1b[1mbold words that wrap around\x1b[0m and plain ones",
		"\x1b[38;2;255;0;0mred text spanning several wrapped lines of output\x1b[0m",
		"\x1b[7mreverse spaces   in here\x1b[0m end",
		"\x1b[4munderlined words keep their style\x1b[24m done",
		"\x1b]8;id=1;https://x.y\x1b\\hyperlinked text wraps too\x1b]8;;\x1b\\",
		"日本語の文章は単語の区切りがありません。",
		"tab\tseparated\twords\tin a line",
		"nbsp joined words stay together",
		"\x1b[1;3;4;5;7;9;38;5;100;48;2;1;2;3mall attrs wrapped here\x1b[m",
		"\x1b[4:3;58;2;10;20;30mcurly underline colored\x1b[m",
		"\x1b[38:2::10:20:30mcolon rgb text wraps\x1b[m",
		"\x1b[2;6;8;22;23;25;27;28;29;39;49;59;90;100;107mreset-ish\x1b[0m",
		"\x1b[38;2;0;229;255m★ COMMAND PALETTE ★\x1b[0m long title",
		"a b c d e f g h i j k l m n o p q r s t u v w x y z",
	)
	for _, s := range wrapInputs {
		for limit := -1; limit <= 30; limit++ {
			w.rec("op", "wrap", "in", s, "n", strconv.Itoa(limit), "out", cellbuf.Wrap(s, limit, ""))
		}
		w.rec("op", "wrap", "in", s, "n", "7", "bp", ",.", "out", cellbuf.Wrap(s, 7, ",."))
	}
}
