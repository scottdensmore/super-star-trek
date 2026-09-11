package engine

import (
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"unicode"
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
	firstRune := rune(stem[0])
	if !unicode.IsLetter(firstRune) {
		return fmt.Errorf("Spock- \"Captain, file names must begin with an alphabetic letter (A-Z).\"")
	}
	return nil
}

// Save serializes the GameState to a JSON file. If path lacks the .TRK extension, it is appended.
func (g *GameState) Save(path string) error {
	dir := filepath.Dir(path)
	base := filepath.Base(path)
	if err := ValidateSaveFilename(base); err != nil {
		return err
	}

	if !strings.HasSuffix(strings.ToUpper(path), ".TRK") {
		path = filepath.Join(dir, base+".TRK")
	}

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
