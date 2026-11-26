//! Save-file formats (pure parts of `pkg/engine/save.go`; file I/O lives in
//! [`crate::persist`]). Every function here works on bytes so it can be used
//! from the WASM bridge as well.

use crate::actions::EngineError;
use crate::campaign::{TourRef, TourState};
use crate::gopath;
use crate::gotime::GoTime;
use crate::json::{self, JsonError};
use crate::prng::PRNG;
use crate::refits::apply_refits;
use crate::rules::{default_rules_for_profile, DifficultyProfile};
use crate::state::{game_ref, sync_quadrant_enemies, GameRef, GameState};
use crate::types::{ConditionType, SkillLevel};

/// Go `ValidateSaveFilename`.
pub fn validate_save_filename(filename: &str) -> Result<(), EngineError> {
    let trimmed = filename.trim();
    if trimmed.is_empty() {
        return Err(EngineError::new("file name cannot be empty"));
    }
    let base = gopath::base(trimmed);
    let ext = gopath::ext(&base);
    let stem = base.strip_suffix(ext.as_str()).unwrap_or(&base);
    if stem.is_empty() {
        return Err(EngineError::new("file name cannot be empty"));
    }
    if stem.len() > 9 {
        return Err(EngineError::new("file name cannot exceed 9 characters"));
    }
    let first = stem.as_bytes()[0];
    if !first.is_ascii_alphabetic() {
        return Err(EngineError::new(
            "spock- \"Captain, file names must begin with an alphabetic letter (A-Z).\"",
        ));
    }
    Ok(())
}

/// Resolves the path written by Go `GameState.Save` (validates and appends
/// `.TRK` when missing).
pub fn resolve_save_path(path: &str) -> Result<String, EngineError> {
    let path = path.trim();
    if path.is_empty() {
        return Err(EngineError::new("file name cannot be empty"));
    }
    let dir = gopath::dir(path);
    let mut base = gopath::base(path);
    validate_save_filename(&base)?;
    if !base.to_uppercase().ends_with(".TRK") {
        base.push_str(".TRK");
    }
    Ok(gopath::join(&[&dir, &base]))
}

/// Go `SaveEnvelope`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SaveEnvelope {
    pub version: i32,
    pub game_state: Option<GameRef>,
    pub tour_state: Option<TourRef>,
}

crate::go_json_struct!(SaveEnvelope, "SaveEnvelope", "engine.SaveEnvelope" {
    version => "version",
    game_state => "game_state",
    tour_state => "tour_state" omitempty,
});

/// Bytes written by Go `GameState.Save` (bare state, `MarshalIndent`).
pub fn encode_legacy_save(g: &GameState) -> Result<Vec<u8>, JsonError> {
    json::marshal_indent(g, "", "  ")
}

/// Bytes written by Go `SaveGame` (envelope version 2).
pub fn encode_save_game(g: &GameRef) -> Result<Vec<u8>, EngineError> {
    let env = SaveEnvelope {
        version: 2,
        game_state: Some(g.clone()),
        tour_state: None,
    };
    json::marshal_indent(&env, "", "  ")
        .map_err(|e| EngineError(format!("failed to serialize game save: {}", e)))
}

/// Bytes written by Go `SaveTourGame`.
pub fn encode_tour_save(g: Option<&GameRef>, t: Option<&TourRef>) -> Result<Vec<u8>, EngineError> {
    let env = SaveEnvelope {
        version: 2,
        game_state: g.cloned(),
        tour_state: t.cloned(),
    };
    json::marshal_indent(&env, "", "  ")
        .map_err(|e| EngineError(format!("failed to serialize tour save: {}", e)))
}

fn finish_loaded(state: &mut GameState) {
    if state.rules.profile.as_str().is_empty() {
        state.rules = default_rules_for_profile(&DifficultyProfile::normal());
    }
    state.rng = Some(PRNG::new(state.stardate as i64));
    sync_quadrant_enemies(&mut state.current_quad);
}

/// Decoding half of Go `LoadGame` (envelope first, then legacy bare state).
pub fn decode_load_game(data: &[u8]) -> Result<GameRef, JsonError> {
    let mut env = SaveEnvelope::default();
    if json::unmarshal(data, &mut env).is_ok() {
        if let Some(state) = env.game_state.clone() {
            if let Some(t) = &env.tour_state {
                let refits = t.borrow().installed_refits.clone();
                apply_refits(&mut state.borrow_mut(), refits.as_ref());
            }
            finish_loaded(&mut state.borrow_mut());
            return Ok(state);
        }
    }
    let mut state = GameState::default();
    json::unmarshal(data, &mut state)?;
    finish_loaded(&mut state);
    Ok(game_ref(state))
}

/// Go `SaveMetadata`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SaveMetadata {
    pub path: String,
    pub filename: String,
    pub mod_time: GoTime,
    pub skill: SkillLevel,
    pub stardate: f64,
    pub time_remaining: f64,
    pub condition: ConditionType,
    pub klingons_left: i32,
}

/// Decoding half of Go `InspectSave` (`path` is only used for names/messages).
pub fn decode_inspect_save(
    data: &[u8],
    path: &str,
    mod_time: GoTime,
) -> Result<SaveMetadata, EngineError> {
    let mut env = SaveEnvelope::default();
    let state: GameState = match json::unmarshal(data, &mut env) {
        Ok(()) if env.game_state.is_some() || env.tour_state.is_some() => match &env.game_state {
            Some(g) => g.borrow().clone(),
            None => GameState::default(),
        },
        _ => {
            let mut legacy = GameState::default();
            if let Err(e) = json::unmarshal(data, &mut legacy) {
                return Err(EngineError(format!(
                    "corrupted save file {}: {}",
                    gopath::base(path),
                    e
                )));
            }
            legacy
        }
    };
    Ok(SaveMetadata {
        path: path.to_string(),
        filename: gopath::base(path),
        mod_time,
        skill: state.skill,
        stardate: state.stardate,
        time_remaining: state.time_remaining,
        condition: state.enterprise.condition,
        klingons_left: state.remaining_klingons,
    })
}

/// Decoding half of Go `LoadTourGame`.
///
/// Note (Go behaviour, preserved): the envelope's `"RNG": {}` decodes into a
/// non-nil PRNG with a nil source, so `RNG == nil` is false and the PRNG is
/// *not* rebuilt; drawing from it later panics.
pub fn decode_tour_game(
    data: &[u8],
    filename: &str,
) -> Result<(Option<GameRef>, Option<TourRef>), EngineError> {
    let mut env = SaveEnvelope::default();
    if let Err(e) = json::unmarshal(data, &mut env) {
        return Err(EngineError(format!(
            "failed to deserialize save file: {}",
            e
        )));
    }
    if env.game_state.is_none() && env.tour_state.is_none() {
        let mut legacy = GameState::default();
        if json::unmarshal(data, &mut legacy).is_ok()
            && (legacy.enterprise.energy > 0.0
                || legacy.stardate > 0.0
                || legacy.enterprise.max_energy > 0.0)
        {
            finish_loaded(&mut legacy);
            return Ok((Some(game_ref(legacy)), None));
        }
        return Err(EngineError(format!(
            "invalid save file {}: neither game state nor tour state found",
            filename
        )));
    }
    if let (Some(g), Some(t)) = (&env.game_state, &env.tour_state) {
        let refits = t.borrow().installed_refits.clone();
        apply_refits(&mut g.borrow_mut(), refits.as_ref());
        t.borrow_mut().current_game_state = Some(g.clone());
    }
    if let Some(g) = &env.game_state {
        let mut gs = g.borrow_mut();
        if gs.rules.profile.as_str().is_empty() {
            gs.rules = default_rules_for_profile(&DifficultyProfile::normal());
        }
        if gs.rng.is_none() {
            let sd = gs.stardate as i64;
            gs.rng = Some(PRNG::new(sd));
        }
        sync_quadrant_enemies(&mut gs.current_quad);
    }
    Ok((env.game_state, env.tour_state))
}

/// WASM `Session.Save`: compact `json.Marshal` of the state.
pub fn encode_session(g: &GameState) -> Result<String, JsonError> {
    json::marshal(g).map(|b| String::from_utf8(b).expect("json is utf-8"))
}

/// WASM `Session.Load`: unmarshal into a fresh state, no fix-ups (Go
/// behaviour: the PRNG source is nil and the legacy Klingon list is empty).
pub fn decode_session(data: &str) -> Result<GameState, JsonError> {
    let mut g = GameState::default();
    json::unmarshal(data.as_bytes(), &mut g)?;
    Ok(g)
}

/// Convenience: a `TourState` handle.
pub fn tour_ref(t: TourState) -> TourRef {
    std::rc::Rc::new(std::cell::RefCell::new(t))
}
