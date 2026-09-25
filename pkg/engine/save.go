package engine

import (
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"time"
)

// ValidateSaveFilename checks that the filename conforms to Spock's constraints:
// non-empty, max 9 characters (excluding extension), and starts with an alphabetic letter (A-Z).
func ValidateSaveFilename(filename string) error {
	trimmed := strings.TrimSpace(filename)
	if trimmed == "" {
		return errors.New("file name cannot be empty")
	}

	base := filepath.Base(trimmed)
	ext := filepath.Ext(base)
	stem := strings.TrimSuffix(base, ext)

	if len(stem) == 0 {
		return errors.New("file name cannot be empty")
	}
	if len(stem) > 9 {
		return errors.New("file name cannot exceed 9 characters")
	}
	firstByte := stem[0]
	if (firstByte < 'A' || firstByte > 'Z') && (firstByte < 'a' || firstByte > 'z') {
		return fmt.Errorf("spock- \"Captain, file names must begin with an alphabetic letter (A-Z).\"")
	}
	return nil
}

// Save serializes the GameState to a JSON file. If path lacks the .TRK extension, it is appended.
func (g *GameState) Save(path string) error {
	path = strings.TrimSpace(path)
	if path == "" {
		return errors.New("file name cannot be empty")
	}
	dir := filepath.Dir(path)
	base := filepath.Base(path)
	if err := ValidateSaveFilename(base); err != nil {
		return err
	}

	if !strings.HasSuffix(strings.ToUpper(base), ".TRK") {
		base = base + ".TRK"
	}
	path = filepath.Join(dir, base)

	data, err := json.MarshalIndent(g, "", "  ")
	if err != nil {
		return err
	}
	return os.WriteFile(path, data, 0644)
}

// LoadGame reads and unmarshals GameState from the specified file and reconstructs the PRNG.
func LoadGame(path string) (*GameState, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, err
	}

	var envelope SaveEnvelope
	if err := json.Unmarshal(data, &envelope); err == nil && envelope.GameState != nil {
		state := envelope.GameState
		if envelope.TourState != nil {
			ApplyRefits(state, envelope.TourState.InstalledRefits)
		}
		if state.Rules.Profile == "" {
			state.Rules = DefaultRulesForProfile(ProfileNormal)
		}
		state.RNG = NewPRNG(int64(state.Stardate))
		return state, nil
	}

	var state GameState
	if err := json.Unmarshal(data, &state); err != nil {
		return nil, err
	}
	if state.Rules.Profile == "" {
		state.Rules = DefaultRulesForProfile(ProfileNormal)
	}
	state.RNG = NewPRNG(int64(state.Stardate))
	return &state, nil
}

// SaveMetadata holds summary attributes extracted from a saved game file.
type SaveMetadata struct {
	Path          string
	Filename      string
	ModTime       time.Time
	Skill         SkillLevel
	Stardate      float64
	TimeRemaining float64
	Condition     ConditionType
	KlingonsLeft  int
}

// InspectSave reads a .TRK file and extracts summary metadata without mutating any active game state.
func InspectSave(path string) (*SaveMetadata, error) {
	info, err := os.Stat(path)
	if err != nil {
		return nil, err
	}

	data, err := os.ReadFile(path)
	if err != nil {
		return nil, err
	}

	var state *GameState
	var envelope SaveEnvelope
	if err := json.Unmarshal(data, &envelope); err == nil && (envelope.GameState != nil || envelope.TourState != nil) {
		if envelope.GameState != nil {
			state = envelope.GameState
		} else {
			state = &GameState{}
		}
	} else {
		var legacy GameState
		if err := json.Unmarshal(data, &legacy); err != nil {
			return nil, fmt.Errorf("corrupted save file %s: %w", filepath.Base(path), err)
		}
		state = &legacy
	}

	return &SaveMetadata{
		Path:          path,
		Filename:      filepath.Base(path),
		ModTime:       info.ModTime(),
		Skill:         state.Skill,
		Stardate:      state.Stardate,
		TimeRemaining: state.TimeRemaining,
		Condition:     state.Enterprise.Condition,
		KlingonsLeft:  state.RemainingKlingons,
	}, nil
}

// SaveEnvelope wraps GameState and TourState for dual-state campaign persistence.
type SaveEnvelope struct {
	Version   int        `json:"version"`
	GameState *GameState `json:"game_state"`
	TourState *TourState `json:"tour_state,omitempty"`
}

// SaveTourGame serializes both the current sector GameState and the active TourState.
func SaveTourGame(filename string, g *GameState, t *TourState) error {
	envelope := SaveEnvelope{
		Version:   2,
		GameState: g,
		TourState: t,
	}
	data, err := json.MarshalIndent(envelope, "", "  ")
	if err != nil {
		return fmt.Errorf("failed to serialize tour save: %w", err)
	}
	return os.WriteFile(filename, data, 0o600)
}

// LoadTourGame deserializes a tour save file, applies installed refits, and reconstructs the PRNG.
func LoadTourGame(filename string) (*GameState, *TourState, error) {
	data, err := os.ReadFile(filename)
	if err != nil {
		return nil, nil, fmt.Errorf("failed to read save file: %w", err)
	}
	var envelope SaveEnvelope
	if err := json.Unmarshal(data, &envelope); err != nil {
		return nil, nil, fmt.Errorf("failed to deserialize save file: %w", err)
	}

	// Backward compatibility: If it's a legacy save without SaveEnvelope structure
	if envelope.GameState == nil && envelope.TourState == nil {
		var legacy GameState
		if err := json.Unmarshal(data, &legacy); err == nil && (legacy.Enterprise.Energy > 0 || legacy.Stardate > 0 || legacy.Enterprise.MaxEnergy > 0) {
			if legacy.Rules.Profile == "" {
				legacy.Rules = DefaultRulesForProfile(ProfileNormal)
			}
			legacy.RNG = NewPRNG(int64(legacy.Stardate))
			return &legacy, nil, nil
		}
	}

	if envelope.GameState != nil && envelope.TourState != nil {
		ApplyRefits(envelope.GameState, envelope.TourState.InstalledRefits)
		if envelope.TourState.CurrentGameState == nil {
			envelope.TourState.CurrentGameState = envelope.GameState
		}
	}
	if envelope.GameState != nil {
		if envelope.GameState.Rules.Profile == "" {
			envelope.GameState.Rules = DefaultRulesForProfile(ProfileNormal)
		}
		if envelope.GameState.RNG == nil {
			envelope.GameState.RNG = NewPRNG(int64(envelope.GameState.Stardate))
		}
	}
	return envelope.GameState, envelope.TourState, nil
}
