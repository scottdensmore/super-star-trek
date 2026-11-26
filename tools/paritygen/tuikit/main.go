// Command tuikit generates parity reference data for crates/sst-tuikit, the
// Rust re-implementation of the Charmbracelet subset used by pkg/tui
// (lipgloss v1.1.0, termenv v0.16.0, x/ansi v0.11.9, x/cellbuf, bubbletea
// v1.3.10, bubbles v1.0.0, sahilm/fuzzy).
//
// Usage (from the repo root):
//
//	RUNEWIDTH_EASTASIAN=0 LANG=C go run ./tools/paritygen/tuikit
//
// It writes:
//
//   - crates/sst-tuikit/src/unicode/tables.rs: per-codepoint width
//     properties as computed by clipperhouse/displaywidth (x/ansi),
//     rivo/uniseg and mattn/go-runewidth, run-length encoded.
//   - testdata/parity/tuikit/*.txt: case records (see writer.go) that the
//     Rust tests replay byte-for-byte.
//
// The generator is committed once for provenance and is removed together
// with the Go code.
package main

import (
	"flag"
	"fmt"
	"os"
	"path/filepath"
)

func main() {
	root := flag.String("root", ".", "repository root")
	only := flag.String("only", "", "only generate this area (tables, ansi, lipgloss, colors, tea, textinput, list, termenv)")
	flag.Parse()

	out := filepath.Join(*root, "testdata", "parity", "tuikit")
	if err := os.MkdirAll(out, 0o755); err != nil {
		panic(err)
	}

	run := func(name string, f func()) {
		if *only != "" && *only != name {
			return
		}
		fmt.Fprintln(os.Stderr, "generating", name)
		f()
	}

	run("tables", func() { genTables(filepath.Join(*root, "crates", "sst-tuikit", "src", "unicode", "tables.rs")) })
	run("ansi", func() { genAnsi(filepath.Join(out, "ansi.txt")) })
	run("colors", func() { genColors(filepath.Join(out, "colors.txt")) })
	run("lipgloss", func() { genLipgloss(filepath.Join(out, "lipgloss.txt")) })
	run("tea", func() { genTea(filepath.Join(out, "tea.txt")) })
	run("textinput", func() { genTextinput(filepath.Join(out, "textinput.txt")) })
	run("list", func() { genList(filepath.Join(out, "list.txt")) })
	run("termenv", func() { genTermenv(filepath.Join(out, "termenv.txt")) })
}
