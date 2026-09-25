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

var (
	version = "2.0.0-dev"
	commit  = "none"
	date    = "unknown"
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
	for _, arg := range args {
		if arg == "--help" || arg == "-h" || arg == "-help" {
			fs.SetOutput(out)
			break
		}
	}
	_ = fs.Bool("classic", false, "run in teletype plain mode")
	versionFlag := fs.Bool("version", false, "print version information and exit")
	fs.BoolVar(versionFlag, "v", false, "shorthand for --version")
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
	sound := fs.Bool("sound", true, "enable retro procedural audio and sound FX")
	noSound := fs.Bool("no-sound", false, "disable retro procedural audio and sound FX")
	scenarioFlag := fs.String("scenario", "", "launch specific tactical scenario")
	fs.StringVar(scenarioFlag, "s", "", "shorthand for --scenario")
	listScenariosFlag := fs.Bool("list-scenarios", false, "display available tactical scenarios")
	tourFlag := fs.Bool("tour", false, "Launch in Starfleet Career & Campaign (Patrol Tour) mode")
	fs.BoolVar(tourFlag, "campaign", false, "alias for --tour")

	if err := fs.Parse(args); err != nil {
		if err == flag.ErrHelp {
			return 0
		}
		return 1
	}

	if *versionFlag {
		_, _ = fmt.Fprintf(out, "sst version %s (commit: %s, built at: %s)\n", version, commit, date)
		return 0
	}

	if *listScenariosFlag {
		_, _ = fmt.Fprintln(out, "Available Tactical Scenarios:")
		for _, sc := range engine.ListScenarios() {
			_, _ = fmt.Fprintf(out, "  %-16s [%s] %s - %s\n", sc.ID, sc.Difficulty, sc.Name, sc.Description)
		}
		return 0
	}

	var sc *engine.Scenario
	if *scenarioFlag != "" {
		var ok bool
		sc, ok = engine.GetScenario(engine.ScenarioID(*scenarioFlag))
		if !ok {
			_, _ = fmt.Fprintf(errOut, "Error: unknown scenario %q\n", *scenarioFlag)
			return 1
		}
	}

	parsedMode, err := theme.ParseColorMode(*modeFlag)
	if err != nil {
		_, _ = fmt.Fprintf(errOut, "Error: %v\n", err)
		return 1
	}

	visited := make(map[string]bool)
	fs.Visit(func(f *flag.Flag) {
		visited[f.Name] = true
	})

	if visited["anomalies"] && visited["no-anomalies"] {
		_, _ = fmt.Fprintln(errOut, "Error: cannot specify both --anomalies and --no-anomalies")
		return 1
	}

	if visited["sound"] && visited["no-sound"] {
		_, _ = fmt.Fprintln(errOut, "Error: cannot specify both --sound and --no-sound")
		return 1
	}

	soundEnabled := true
	if visited["sound"] {
		soundEnabled = *sound
	}
	if visited["no-sound"] {
		soundEnabled = !*noSound
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

	selectedTheme := theme.GetTheme(*themeName)
	selectedTheme = selectedTheme.WithColorMode(parsedMode)

	var p tui.Model
	if *tourFlag {
		tour := engine.NewTour(s)
		p = tui.NewModelWithTour(tour, selectedTheme)
	} else {
		var game *engine.GameState
		if sc != nil {
			game = sc.Build(s)
		} else {
			game = engine.NewGameWithOptions(s, engine.SkillGood, engine.LengthMedium, rules)
			game.PopulateQuadrant(game.Enterprise.Quad, game.Enterprise.Sector)
		}
		p = tui.NewModel(game, selectedTheme)
	}
	p.SetSoundEnabled(soundEnabled)
	if err := runProgram(p, tea.WithAltScreen(), tea.WithMouseCellMotion()); err != nil {
		_, _ = fmt.Fprintf(errOut, "Error running game: %v\n", err)
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
