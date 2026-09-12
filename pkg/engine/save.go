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
	if !((firstByte >= 'A' && firstByte <= 'Z') || (firstByte >= 'a' && firstByte <= 'z')) {
		return fmt.Errorf("Spock- \"Captain, file names must begin with an alphabetic letter (A-Z).\"")
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

	var state GameState
	if err := json.Unmarshal(data, &state); err != nil {
		return nil, err
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

	var state GameState
	if err := json.Unmarshal(data, &state); err != nil {
		return nil, fmt.Errorf("corrupted save file %s: %w", filepath.Base(path), err)
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
