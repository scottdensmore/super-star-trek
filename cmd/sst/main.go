package main

import (
	"flag"
	"fmt"
	"io"
	"os"
	"strings"
	"time"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/scottdensmore/super-star-trek/pkg/classic"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

var runProgram = func(m tea.Model, opts ...tea.ProgramOption) error {
	p := tea.NewProgram(m, opts...)
	_, err := p.Run()
	return err
}

func isClassic(args []string) bool {
	for _, arg := range args {
		if arg == "--classic" || arg == "-classic" || strings.HasPrefix(arg, "--classic=") || strings.HasPrefix(arg, "-classic=") {
			return true
		}
	}
	return false
}

func run(args []string, in io.Reader, out, errOut io.Writer) int {
	if isClassic(args) {
		return classic.RunClassicCLI(in, out, args)
	}

	fs := flag.NewFlagSet("sst", flag.ContinueOnError)
	fs.SetOutput(errOut)
	_ = fs.Bool("classic", false, "run in teletype plain mode")
	themeName := fs.String("theme", "modern", "initial theme name (modern, lcars, crt)")
	seed := fs.Int64("seed", 0, "PRNG seed (0 for random)")

	if err := fs.Parse(args); err != nil {
		if err == flag.ErrHelp {
			return 0
		}
		return 1
	}

	s := *seed
	if s == 0 {
		s = time.Now().UnixNano()
	}

	game := engine.NewGame(s, engine.SkillGood, engine.LengthMedium)
	selectedTheme := theme.GetTheme(*themeName)

	p := tui.NewModel(game, selectedTheme)
	if err := runProgram(p, tea.WithAltScreen()); err != nil {
		fmt.Fprintf(errOut, "Error running game: %v\n", err)
		return 1
	}
	return 0
}

func main() {
	if isClassic(os.Args[1:]) {
		os.Exit(classic.RunClassicCLI(os.Stdin, os.Stdout, os.Args[1:]))
	}

	os.Exit(run(os.Args[1:], os.Stdin, os.Stdout, os.Stderr))
}


