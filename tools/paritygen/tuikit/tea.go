package main

import (
	"bytes"
	"fmt"
	"io"
	"strconv"
	"strings"

	tea "github.com/charmbracelet/bubbletea"
)

// describeMsg renders a decoded input message in a canonical form the Rust
// test reproduces.
func describeMsg(m tea.Msg) string {
	switch v := m.(type) {
	case tea.KeyMsg:
		return fmt.Sprintf("key type=%d runes=%q alt=%v paste=%v str=%q", int(v.Type), string(v.Runes), v.Alt, v.Paste, v.String())
	case tea.MouseMsg:
		return fmt.Sprintf("mouse x=%d y=%d shift=%v alt=%v ctrl=%v action=%d button=%d type=%d str=%q",
			v.X, v.Y, v.Shift, v.Alt, v.Ctrl, int(v.Action), int(v.Button), int(v.Type), v.String())
	case tea.FocusMsg:
		return "focus"
	case tea.BlurMsg:
		return "blur"
	case fmt.Stringer:
		return fmt.Sprintf("%T %s", v, v.String())
	}
	return fmt.Sprintf("%T", m)
}

// recorder is a model that records every input message until it sees the
// ctrl+] sentinel.
type recorder struct{ got []string }

func (r recorder) Init() tea.Cmd { return nil }
func (r recorder) Update(m tea.Msg) (tea.Model, tea.Cmd) {
	if k, ok := m.(tea.KeyMsg); ok && k.Type == tea.KeyCtrlCloseBracket && !k.Alt {
		return r, tea.Quit
	}
	switch m.(type) {
	case tea.WindowSizeMsg:
		return r, nil
	}
	r.got = append(r.got, describeMsg(m))
	return r, nil
}
func (r recorder) View() string { return "" }

// chunkReader returns each chunk from a separate Read call.
type chunkReader struct{ chunks [][]byte }

func (c *chunkReader) Read(p []byte) (int, error) {
	if len(c.chunks) == 0 {
		select {} // block forever; the program quits on the sentinel
	}
	n := copy(p, c.chunks[0])
	if n < len(c.chunks[0]) {
		c.chunks[0] = c.chunks[0][n:]
	} else {
		c.chunks = c.chunks[1:]
	}
	return n, nil
}

func decodeViaProgram(input []byte) []string {
	r := &chunkReader{chunks: [][]byte{input, []byte("\x1d")}}
	p := tea.NewProgram(recorder{}, tea.WithInput(r), tea.WithOutput(io.Discard), tea.WithoutRenderer(), tea.WithoutSignalHandler())
	m, err := p.Run()
	if err != nil {
		panic(err)
	}
	return m.(recorder).got
}

func genTea(path string) {
	w := newRecWriter(path)
	defer w.close()

	// Key.String for every key type, with and without alt.
	var types []int
	for i := 0; i <= 31; i++ {
		types = append(types, i)
	}
	types = append(types, 127, 32, 100, -54, -100)
	for i := -1; i >= -53; i-- {
		types = append(types, i)
	}
	for _, t := range types {
		for _, alt := range []bool{false, true} {
			k := tea.KeyMsg{Type: tea.KeyType(t), Alt: alt}
			w.rec("op", "keystring", "type", strconv.Itoa(t), "alt", strconv.FormatBool(alt), "out", k.String(), "typestr", tea.KeyType(t).String())
		}
	}
	for _, runes := range []string{"a", "Z", "ab", " ", "日本", "", "★", "q"} {
		for _, alt := range []bool{false, true} {
			for _, paste := range []bool{false, true} {
				k := tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune(runes), Alt: alt, Paste: paste}
				w.rec("op", "keyrunes", "runes", runes, "alt", strconv.FormatBool(alt), "paste", strconv.FormatBool(paste), "out", k.String())
			}
		}
	}
	// KeySpace with runes, as produced by the reader.
	w.rec("op", "keyspace", "out", tea.KeyMsg{Type: tea.KeySpace, Runes: []rune{' '}}.String())

	// MouseEvent.String for all combinations.
	for b := 0; b <= 12; b++ {
		for a := 0; a <= 3; a++ {
			for mods := 0; mods < 8; mods++ {
				m := tea.MouseMsg{X: 3, Y: 4, Button: tea.MouseButton(b), Action: tea.MouseAction(a), Ctrl: mods&1 != 0, Alt: mods&2 != 0, Shift: mods&4 != 0}
				w.rec("op", "mousestring", "b", strconv.Itoa(b), "a", strconv.Itoa(a), "mods", strconv.Itoa(mods), "out", m.String())
			}
		}
	}

	// Input decoding through a real Program reading from a custom reader.
	inputs := []string{
		"a", "abc", "hello world", " ", "  ", "\r", "\n", "\t", "\x7f", "\x08", "\x03", "\x00", "\x1b",
		"\x1b\x1b", "\x1ba", "\x1b ", "\x1b\r", "\x1b\x7f", "\x1b[A", "\x1b[B", "\x1b[C", "\x1b[D", "\x1b[1;5C",
		"\x1b[Z", "\x1b[3~", "\x1b[5~", "\x1b[6~", "\x1b[H", "\x1b[F", "\x1b[1~", "\x1b[4~", "\x1bOP", "\x1bOQ",
		"\x1b[15~", "\x1b[24~", "\x1b[1;2Q", "\x1b[1;3A", "\x1b\x1b[A", "\x1b[200~pasted text\x1b[201~",
		"\x1b[200~multi\nline\tpaste\x1b[201~", "\x1b[I", "\x1b[O", "\x1b[<0;10;5M", "\x1b[<0;10;5m",
		"\x1b[<64;1;1M", "\x1b[<65;2;3M", "\x1b[<32;5;5M", "\x1b[<35;7;8M", "\x1b[<2;1;1M", "\x1b[<1;3;3M",
		"\x1b[<16;4;4M", "\x1b[<8;4;4M", "\x1b[<4;4;4M", "\x1b[<128;1;1M", "\x1b[<129;1;1m",
		"\x1b[M #!", "\x1b[M@$%", "\x1b[Ma b", "\x1b[99x", "\x1b[?1;2c", "日本語", "★x", "é",
		"abc\x1b[Adef", "a\x03b", "x\x1b[<0;1;1My", "\x1b[1;5A\x1b[1;5B", "\xff", "\xc3", "a\xffb", "\x1b\xff",
		"\x1bOA", "\x1b[a", "\x1b[7^", "\x1b[8@", "\x1b[[A", "\x1b[2~", "\x1b[3;3~", "\x1c", "\x1f", "\x1d",
		"\x1b[<0;12;34",
	}
	for _, in := range inputs {
		got := decodeViaProgram([]byte(in))
		w.rec("op", "decode", "in", in, "out", strings.Join(got, "\n"))
	}
	// A 256-byte read boundary: the reader is called with a full buffer.
	long := bytes.Repeat([]byte("x"), 255)
	long = append(long, 0x1b)
	got := decodeViaProgram(append(long, []byte("[A")...))
	w.rec("op", "decode", "in", string(append(long, []byte("[A")...)), "out", strings.Join(got, "\n"))
}
