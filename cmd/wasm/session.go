package main

import (
	"encoding/json"
	"fmt"
	"strings"

	"github.com/scottdensmore/super-star-trek/pkg/audio"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/parser"
)

// Session manages a single player's game session.
type Session struct {
	game       *engine.GameState
	history    []string
	seed       int64
	dispatcher *audio.Dispatcher
}

// NewSession creates and initializes a new Session with the specified seed and difficulty.
func NewSession(seed int64, difficulty ...engine.DifficultyProfile) *Session {
	if seed == 0 {
		seed = 12345
	}
	diff := engine.ProfileNormal
	if len(difficulty) > 0 {
		diff = difficulty[0]
	}
	rules := engine.DefaultRulesForProfile(diff)
	game := engine.NewGameWithOptions(seed, engine.SkillGood, engine.LengthMedium, rules)
	game.PopulateQuadrant(game.Enterprise.Quad, game.Enterprise.Sector)
	return &Session{
		game:       game,
		history:    make([]string, 0),
		seed:       seed,
		dispatcher: audio.NewDispatcher(audio.NewNullPlayer()),
	}
}

// Game returns the underlying GameState.
func (s *Session) Game() *engine.GameState {
	return s.game
}

// SetAudioPlayer configures the audio player for the session.
func (s *Session) SetAudioPlayer(player audio.Player) {
	s.dispatcher = audio.NewDispatcher(player)
}

// AudioDispatcher returns the session's audio event dispatcher.
func (s *Session) AudioDispatcher() *audio.Dispatcher {
	return s.dispatcher
}

// Execute processes a user input string and returns formatted teletype output.
func (s *Session) Execute(input string) string {
	trimmed := strings.TrimSpace(input)
	if trimmed == "" {
		return ""
	}
	s.history = append(s.history, trimmed)

	tokens := strings.Fields(trimmed)
	cmd := strings.ToLower(tokens[0])

	switch cmd {
	case "help", "?":
		return "COMMANDS: nav, srs, lrs, pha, tor, she, dam, chart, com, scenario [list|<id>], save [slot], load [slot], help, quit\r\n"
	case "srs", "srscan", "status":
		return FormatSRS(s.game)
	case "lrs", "lrscan":
		return FormatLRS(s.game)
	case "chart":
		return FormatChart(s.game)
	case "dam", "damages":
		return FormatDamages(s.game)
	case "scenario":
		if len(tokens) == 1 || strings.ToLower(tokens[1]) == "list" {
			return FormatScenarios()
		}
		id := tokens[1]
		if len(tokens) > 2 {
			id = strings.Join(tokens[1:], "-")
		}
		sc, ok := engine.GetScenario(engine.ScenarioID(id))
		if !ok {
			return fmt.Sprintf("Unknown scenario: %s\r\n", id)
		}
		s.game = sc.Build(s.seed)
		return FormatScenarioBriefing(sc)
	case "quit", "exit", "q":
		return "Session terminated.\r\n"
	}

	parsed := parser.ParseCommand(trimmed)
	if parsed.Error != nil {
		return fmt.Sprintf("Error: %v\r\n", parsed.Error)
	}

	if parsed.Special != "" {
		switch strings.ToLower(parsed.Special) {
		case "quit":
			return "Session terminated.\r\n"
		default:
			return fmt.Sprintf("Special command: %s\r\n", parsed.Special)
		}
	}

	if parsed.Action != nil {
		events, err := s.game.Dispatch(parsed.Action)
		if err != nil {
			return fmt.Sprintf("Cannot execute: %v\r\n", err)
		}

		// Surviving Klingons counter-attack on turn-consuming actions
		gameOver := false
		for _, ev := range events {
			if _, ok := ev.(engine.EventGameOver); ok {
				gameOver = true
				break
			}
		}
		if !gameOver && len(s.game.CurrentQuad.Klingons) > 0 && s.game.Enterprise.Condition != engine.ConditionDocked {
			shouldAttack := false
			switch parsed.Action.(type) {
			case engine.ActionFireTorpedo, engine.ActionTorpedoDirect, engine.ActionFirePhasers, engine.ActionShields:
				shouldAttack = true
			case engine.ActionMove:
				isInterQuad := false
				for _, ev := range events {
					if moveEv, ok := ev.(engine.EventShipMoved); ok && moveEv.FromQuad != moveEv.ToQuad {
						isInterQuad = true
						break
					}
				}
				if !isInterQuad {
					shouldAttack = true
				}
			}
			if shouldAttack {
				kEvents := engine.KlingonTurn(s.game)
				events = append(events, kEvents...)
			}
		}

		if s.dispatcher != nil {
			s.dispatcher.DispatchEvents(events)
		}
		var out strings.Builder
		out.WriteString(FormatCombatEvents(events))
		if _, ok := parsed.Action.(engine.ActionMove); ok {
			out.WriteString(FormatSRS(s.game))
		}
		return out.String()
	}

	return "Invalid command.\r\n"
}

// Save serializes the game state to a JSON string.
func (s *Session) Save() (string, error) {
	data, err := json.Marshal(s.game)
	if err != nil {
		return "", err
	}
	return string(data), nil
}

// Load restores the game state from a JSON string.
func (s *Session) Load(data string) error {
	var g engine.GameState
	if err := json.Unmarshal([]byte(data), &g); err != nil {
		return err
	}
	s.game = &g
	return nil
}
