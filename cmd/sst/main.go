package main

import (
	"flag"
	"fmt"
	"os"
	"time"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/scottdensmore/super-star-trek/pkg/classic"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

func main() {
	fs := flag.NewFlagSet("sst", flag.ContinueOnError)
	classicMode := fs.Bool("classic", false, "run in teletype plain mode")
	themeName := fs.String("theme", "modern", "initial theme name (modern, lcars, crt)")
	seed := fs.Int64("seed", 0, "PRNG seed (0 for random)")

	if err := fs.Parse(os.Args[1:]); err != nil {
		if err == flag.ErrHelp {
			os.Exit(0)
		}
		os.Exit(1)
	}

	if *classicMode {
		os.Exit(classic.RunClassicCLI(os.Stdin, os.Stdout, os.Args[1:]))
	}

	s := *seed
	if s == 0 {
		s = time.Now().UnixNano()
	}

	game := engine.NewGame(s, engine.SkillGood, engine.LengthMedium)
	selectedTheme := theme.GetTheme(*themeName)

	p := tea.NewProgram(tui.NewModel(game, selectedTheme), tea.WithAltScreen())
	if _, err := p.Run(); err != nil {
		fmt.Fprintf(os.Stderr, "Error running game: %v\n", err)
		os.Exit(1)
	}
}

