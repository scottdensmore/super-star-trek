package tui

import "github.com/scottdensmore/super-star-trek/pkg/tui/parser"

// ParsedCommand is an alias for parser.ParsedCommand for backward compatibility.
type ParsedCommand = parser.ParsedCommand

// ParseCommand wraps parser.ParseCommand for backward compatibility.
func ParseCommand(input string) ParsedCommand {
	return parser.ParseCommand(input)
}
