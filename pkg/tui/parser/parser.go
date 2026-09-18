package parser

import (
	"errors"
	"fmt"
	"strconv"
	"strings"

	"github.com/scottdensmore/super-star-trek/pkg/engine"
)

// ParsedCommand represents the structured result of parsing player input.
// It contains either an executable engine.Action, a special TUI command string,
// or an error if the input was unrecognized or malformed.
type ParsedCommand struct {
	Action  engine.Action
	Special string
	Error   error
}

// ParseCommand translates a raw player input string into a ParsedCommand.
// Supported commands:
//   - nav <course> <warp>: moves Enterprise along course vector with warp speed
//   - nav <r> <c>: moves Enterprise directly to destination sector
//   - tor <course>: fires photon torpedo along angle/course
//   - tor <r> <c>: fires photon torpedo at target sector coordinates
//   - pha <energy>: discharges phasers with specified energy
//   - she <amount>: transfers energy to/from shields
//   - doc: docks with adjacent Starbase
//   - theme [name]: switches visual theme
//   - help / ?: displays command reference
//   - quit / exit: exits the game
func ParseCommand(input string) ParsedCommand {
	trimmed := strings.TrimSpace(input)
	if trimmed == "" {
		return ParsedCommand{Error: errors.New("no command entered")}
	}

	tokens := strings.Fields(trimmed)
	cmd := strings.ToLower(tokens[0])
	args := tokens[1:]

	switch cmd {
	case "nav", "move", "warp":
		return parseNav(args)
	case "tor", "torpedo":
		return parseTorpedo(args)
	case "pha", "phaser", "phasers":
		return parsePhasers(args)
	case "she", "shield", "shields", "def":
		return parseShields(args)
	case "doc", "dock":
		return ParsedCommand{Action: engine.ActionDock{}}
	case "opts", "options", "settings":
		return ParsedCommand{Special: "options"}
	case "theme":
		if len(args) > 0 {
			return ParsedCommand{Special: "theme " + strings.ToLower(args[0])}
		}
		return ParsedCommand{Special: "theme"}
	case "help", "?", "commands":
		return parseHelp(args)
	case "quit", "exit", "q":
		return ParsedCommand{Special: "quit"}
	default:
		return ParsedCommand{Error: fmt.Errorf("unknown command: %q (type 'help' for commands)", tokens[0])}
	}
}

func parseNav(args []string) ParsedCommand {
	usageErr := errors.New("usage: nav q <r> <c> (quadrant), nav s <r> <c> (sector), or nav <course> <warp>. Type 'help nav' for guide.")

	if len(args) < 2 {
		return ParsedCommand{Error: usageErr}
	}

	// 3 or 4 arguments with explicit prefix
	if len(args) == 3 || len(args) == 4 {
		prefix := strings.ToLower(args[0])
		if prefix == "q" || prefix == "quad" || prefix == "quadrant" {
			r, err1 := strconv.Atoi(args[1])
			c, err2 := strconv.Atoi(args[2])
			if err1 != nil || err2 != nil {
				return ParsedCommand{Error: errors.New("invalid quadrant parameters: expected integer row and column")}
			}
			if r < 1 || r > 8 || c < 1 || c > 8 {
				return ParsedCommand{Error: errors.New("quadrant coordinates must be between 1 and 8")}
			}
			warp := 1.0
			if len(args) == 4 {
				w, errW := strconv.ParseFloat(args[3], 64)
				if errW != nil || w <= 0 {
					return ParsedCommand{Error: errors.New("warp factor must be positive")}
				}
				warp = w
			}
			return ParsedCommand{Action: engine.ActionMove{DestQuad: engine.Coord{r, c}, Warp: warp}}
		}

		if len(args) == 3 {
			if prefix == "s" || prefix == "sec" || prefix == "sector" {
				r, err1 := strconv.Atoi(args[1])
				c, err2 := strconv.Atoi(args[2])
				if err1 != nil || err2 != nil {
					return ParsedCommand{Error: errors.New("invalid sector parameters: expected integer row and column")}
				}
				if r < 1 || r > 8 || c < 1 || c > 8 {
					return ParsedCommand{Error: errors.New("sector coordinates must be between 1 and 8")}
				}
				return ParsedCommand{Action: engine.ActionMove{DestSector: engine.Coord{r, c}, Warp: 1.0}}
			}
			if prefix == "c" || prefix == "course" {
				course, err1 := strconv.ParseFloat(args[1], 64)
				warp, err2 := strconv.ParseFloat(args[2], 64)
				if err1 != nil || err2 != nil {
					return ParsedCommand{Error: errors.New("invalid course/warp parameters: expected numbers")}
				}
				if warp <= 0 {
					return ParsedCommand{Error: errors.New("warp factor must be positive")}
				}
				return ParsedCommand{Action: engine.ActionMove{Course: course, Warp: warp}}
			}
		}

		return ParsedCommand{Error: usageErr}
	}

	// 2 arguments: distinguish nav <course> <warp> vs nav <r> <c>
	hasDot := strings.Contains(args[0], ".") || strings.Contains(args[1], ".")
	if !hasDot {
		r, err1 := strconv.Atoi(args[0])
		c, err2 := strconv.Atoi(args[1])
		if err1 == nil && err2 == nil && r >= 1 && r <= 8 && c >= 1 && c <= 8 {
			return ParsedCommand{Action: engine.ActionMove{DestSector: engine.Coord{r, c}, Warp: 1.0}}
		}
	}

	course, err1 := strconv.ParseFloat(args[0], 64)
	warp, err2 := strconv.ParseFloat(args[1], 64)
	if err1 != nil || err2 != nil {
		return ParsedCommand{Error: errors.New("invalid navigation parameters: expected numbers")}
	}
	if warp <= 0 {
		return ParsedCommand{Error: errors.New("warp factor must be positive")}
	}

	return ParsedCommand{Action: engine.ActionMove{Course: course, Warp: warp}}
}

func parseHelp(args []string) ParsedCommand {
	if len(args) == 0 {
		return ParsedCommand{Special: "help"}
	}
	topic := strings.ToLower(args[0])
	switch topic {
	case "nav", "move", "warp":
		return ParsedCommand{Special: "help nav"}
	case "tor", "torpedo", "torpedoes", "target", "reticle":
		return ParsedCommand{Special: "help tor"}
	case "pha", "phaser", "phasers":
		return ParsedCommand{Special: "help pha"}
	case "she", "shield", "shields", "def":
		return ParsedCommand{Special: "help she"}
	case "doc", "dock":
		return ParsedCommand{Special: "help doc"}
	case "chart", "map":
		return ParsedCommand{Special: "help chart"}
	case "saves", "thaw", "freeze":
		return ParsedCommand{Special: "help saves"}
	case "opts", "options", "settings":
		return ParsedCommand{Special: "help options"}
	default:
		return ParsedCommand{Special: "help " + topic}
	}
}

func parseTorpedo(args []string) ParsedCommand {
	if len(args) == 1 {
		angle, err := strconv.ParseFloat(args[0], 64)
		if err != nil {
			return ParsedCommand{Error: fmt.Errorf("invalid torpedo course: %s", args[0])}
		}
		return ParsedCommand{Action: engine.ActionFireTorpedo{Angle: angle}}
	}

	if len(args) == 2 {
		r, err1 := strconv.Atoi(args[0])
		c, err2 := strconv.Atoi(args[1])
		if err1 != nil || err2 != nil {
			return ParsedCommand{Error: fmt.Errorf("invalid torpedo target: %s %s", args[0], args[1])}
		}
		if r < 1 || r > 8 || c < 1 || c > 8 {
			return ParsedCommand{Error: errors.New("torpedo target sector coordinates must be between 1 and 8")}
		}
		return ParsedCommand{Action: engine.ActionFireTorpedo{Target: engine.Coord{r, c}}}
	}

	return ParsedCommand{Error: errors.New("usage: tor <course> or tor <row> <col>")}
}

func parsePhasers(args []string) ParsedCommand {
	if len(args) != 1 {
		return ParsedCommand{Error: errors.New("usage: pha <energy>")}
	}
	energy, err := strconv.ParseFloat(args[0], 64)
	if err != nil {
		return ParsedCommand{Error: fmt.Errorf("invalid phaser energy: %s", args[0])}
	}
	if energy <= 0 {
		return ParsedCommand{Error: errors.New("phaser energy must be positive")}
	}
	return ParsedCommand{Action: engine.ActionFirePhasers{Energy: energy}}
}

func parseShields(args []string) ParsedCommand {
	if len(args) != 1 {
		return ParsedCommand{Error: errors.New("usage: she <amount>")}
	}
	amount, err := strconv.ParseFloat(args[0], 64)
	if err != nil {
		return ParsedCommand{Error: fmt.Errorf("invalid shield amount: %s", args[0])}
	}
	return ParsedCommand{Action: engine.ActionShields{Amount: amount}}
}
