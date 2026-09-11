package classic

import (
	"flag"
	"fmt"
	"io"

	"github.com/scottdensmore/super-star-trek/pkg/engine"
)

// RunClassicCLI executes the command-line interface with the provided I/O streams and command-line arguments.
// It returns an exit status code (0 for success, non-zero for failure).
func RunClassicCLI(in io.Reader, out io.Writer, args []string) int {
	fs := flag.NewFlagSet("sst", flag.ContinueOnError)
	fs.SetOutput(out)
	classicMode := fs.Bool("classic", false, "run in teletype plain mode")
	seed := fs.Int64("seed", 0, "PRNG seed (0 for random)")
	if err := fs.Parse(args); err != nil {
		return 1
	}

	if *seed == 0 {
		*seed = 12345
	}

	game := engine.NewGame(*seed, engine.SkillGood, engine.LengthMedium)

	if *classicMode {
		sub := NewTeletypeSubscriber(out)
		fmt.Fprintln(out, "Super Star Trek (Go Edition)")
		sub.HandleEvent(engine.EventShieldTransfer{
			NewShields: game.Enterprise.Shields,
			NewEnergy:  game.Enterprise.Energy,
		})
	} else {
		fmt.Fprintln(out, "Charmbracelet TUI placeholder - use --classic for teletype mode.")
	}
	return 0
}
