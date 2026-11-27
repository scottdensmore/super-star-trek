//! A single player's game session (port of `cmd/wasm/session.go`).

use std::rc::Rc;

use sst_audio::{Dispatcher, NullPlayer, PlayerRef};
use sst_engine::json::JsonError;
use sst_engine::parser::{fields, parse_command, trim_space};
use sst_engine::{
    decode_load_game, default_rules_for_profile, encode_session, get_scenario,
    session_counter_attack, Action, DifficultyProfile, GameLength, GameState, ScenarioID,
    SkillLevel,
};

use crate::formatter::*;

/// Go `Session`.
pub struct Session {
    game: GameState,
    history: Vec<String>,
    seed: i64,
    dispatcher: Dispatcher,
}

impl Session {
    /// Go `NewSession(seed, difficulty...)`; `None` means the variadic was omitted
    /// (`ProfileNormal`). A zero seed becomes 12345.
    pub fn new(seed: i64, difficulty: Option<DifficultyProfile>) -> Session {
        let seed = if seed == 0 { 12345 } else { seed };
        let diff = difficulty.unwrap_or_else(DifficultyProfile::normal);
        let rules = default_rules_for_profile(&diff);
        let mut game =
            GameState::new_game_with_options(seed, SkillLevel::Good, GameLength::Medium, rules);
        let (q, s) = (game.enterprise.quad, game.enterprise.sector);
        game.populate_quadrant(q, s);
        Session {
            game,
            history: Vec::new(),
            seed,
            dispatcher: Dispatcher::new(Some(Rc::new(NullPlayer::new()))),
        }
    }

    /// Go `Game()`.
    pub fn game(&self) -> &GameState {
        &self.game
    }

    /// Mutable access to the game (Go callers mutate through the `*GameState`).
    pub fn game_mut(&mut self) -> &mut GameState {
        &mut self.game
    }

    /// The trimmed, non-empty inputs executed so far.
    pub fn history(&self) -> &[String] {
        &self.history
    }

    /// Go `SetAudioPlayer`.
    pub fn set_audio_player(&mut self, player: PlayerRef) {
        self.dispatcher = Dispatcher::new(Some(player));
    }

    /// Go `AudioDispatcher()`.
    pub fn audio_dispatcher(&self) -> &Dispatcher {
        &self.dispatcher
    }

    /// Go `Execute`: processes one input line and returns teletype output.
    pub fn execute(&mut self, input: &str) -> String {
        let trimmed = trim_space(input);
        if trimmed.is_empty() {
            return String::new();
        }
        self.history.push(trimmed.to_string());

        let tokens = fields(trimmed);
        let cmd = tokens[0].to_lowercase();
        match cmd.as_str() {
            "help" | "?" => {
                return "COMMANDS: nav, srs, lrs, pha, tor, she, dam, chart, com, scenario [list|<id>], save [slot], load [slot], help, quit\r\n".to_string();
            }
            "srs" | "srscan" | "status" => return format_srs(Some(&self.game)),
            "lrs" | "lrscan" => return format_lrs(Some(&self.game)),
            "chart" => return format_chart(Some(&self.game)),
            "dam" | "damages" => return format_damages(Some(&self.game)),
            "scenario" => {
                if tokens.len() == 1 || tokens[1].to_lowercase() == "list" {
                    return format_scenarios();
                }
                let id = if tokens.len() > 2 {
                    tokens[1..].join("-")
                } else {
                    tokens[1].to_string()
                };
                let Some(sc) = get_scenario(&ScenarioID(id.clone())) else {
                    return format!("Unknown scenario: {id}\r\n");
                };
                self.game = sc.build(self.seed);
                return format_scenario_briefing(Some(&sc));
            }
            "quit" | "exit" | "q" => return "Session terminated.\r\n".to_string(),
            _ => {}
        }

        let parsed = parse_command(trimmed);
        if let Some(err) = parsed.error {
            return format!("Error: {err}\r\n");
        }
        if !parsed.special.is_empty() {
            return match parsed.special.to_lowercase().as_str() {
                "quit" => "Session terminated.\r\n".to_string(),
                _ => format!("Special command: {}\r\n", parsed.special),
            };
        }
        if let Some(action) = parsed.action {
            let mut events = match self.game.dispatch(&action) {
                Ok(ev) => ev,
                // A refusal may carry a hint on a second line.
                Err(err) => return format!("Cannot execute: {}\r\n", err.0.replace('\n', "\r\n")),
            };
            // Surviving Klingons counter-attack on turn-consuming actions.
            session_counter_attack(&mut self.game, &action, &mut events);
            self.dispatcher.dispatch_events(&events);
            let mut out = format_combat_events(&events);
            if matches!(action, Action::Move(_)) {
                out.push_str(&format_srs(Some(&self.game)));
            }
            return out;
        }
        "Invalid command.\r\n".to_string()
    }

    /// Go `Save`: compact JSON of the game state.
    pub fn save(&self) -> Result<String, JsonError> {
        encode_session(&self.game)
    }

    /// Go `Load`: replaces the game with the decoded JSON state.
    ///
    /// Runs the terminal loader's post-load fix-ups ([`decode_load_game`]):
    /// rules default to the normal profile when missing, the PRNG is reseeded
    /// from the stardate, and a session saved before format 3 gets its
    /// quadrant's Klingons rebuilt. (Go's `Session.Load` skipped all of
    /// them: the PRNG kept a nil source, so the next random draw panicked,
    /// and the Klingons in the current quadrant were lost.)
    pub fn load(&mut self, data: &str) -> Result<(), JsonError> {
        let g = decode_load_game(data.as_bytes())?;
        self.game = g.borrow().clone();
        Ok(())
    }
}
