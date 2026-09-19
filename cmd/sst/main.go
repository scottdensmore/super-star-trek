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
	modeFlag := fs.String("mode", "auto", "theme color mode (auto, dark, light)")
	seed := fs.Int64("seed", 0, "PRNG seed (0 for random)")
	difficulty := fs.String("difficulty", "normal", "difficulty profile (casual, normal, hardcore, nightmare)")
	surveillance := fs.String("surveillance", "", "surveillance extent (full, classic, local, blackout)")
	sensorDegradation := fs.Bool("sensor-degradation", true, "enable two-tier sensor degradation curve")
	repairMult := fs.Float64("repair-multiplier", 1.0, "subsystem repair duration multiplier")
	klingonCloak := fs.Bool("klingon-cloak", false, "enable Klingon commander tactical cloaking")
	anomalies := fs.Bool("anomalies", false, "enable spatial anomalies & environmental hazards")
	noAnomalies := fs.Bool("no-anomalies", false, "disable spatial anomalies & environmental hazards")

	if err := fs.Parse(args); err != nil {
		if err == flag.ErrHelp {
			return 0
		}
		return 1
	}

	parsedMode, err := theme.ParseColorMode(*modeFlag)
	if err != nil {
		fmt.Fprintf(errOut, "Error: %v\n", err)
		return 1
	}

	visited := make(map[string]bool)
	fs.Visit(func(f *flag.Flag) {
		visited[f.Name] = true
	})

	if visited["anomalies"] && visited["no-anomalies"] {
		fmt.Fprintln(errOut, "Error: cannot specify both --anomalies and --no-anomalies")
		return 1
	}

	rules := engine.DefaultRulesForProfile(engine.DifficultyProfile(*difficulty))
	if visited["surveillance"] {
		rules.Surveillance = engine.SurveillanceMode(*surveillance)
		rules.Profile = engine.ProfileCustom
	}
	if visited["sensor-degradation"] {
		rules.SensorDegradation = *sensorDegradation
		rules.Profile = engine.ProfileCustom
	}
	if visited["repair-multiplier"] {
		rules.RepairMultiplier = *repairMult
		rules.Profile = engine.ProfileCustom
	}
	if visited["klingon-cloak"] {
		rules.KlingonCloak = *klingonCloak
		rules.Profile = engine.ProfileCustom
	}
	if visited["anomalies"] {
		rules.SpatialAnomalies = *anomalies
		rules.Profile = engine.ProfileCustom
	}
	if visited["no-anomalies"] {
		rules.SpatialAnomalies = !*noAnomalies
		rules.Profile = engine.ProfileCustom
	}

	s := *seed
	if s == 0 {
		s = time.Now().UnixNano()
	}

	game := engine.NewGameWithOptions(s, engine.SkillGood, engine.LengthMedium, rules)
	game.PopulateQuadrant(game.Enterprise.Quad, game.Enterprise.Sector)
	selectedTheme := theme.GetTheme(*themeName)
	selectedTheme = selectedTheme.WithColorMode(parsedMode)

	p := tui.NewModel(game, selectedTheme)
	if err := runProgram(p, tea.WithAltScreen(), tea.WithMouseCellMotion()); err != nil {
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
