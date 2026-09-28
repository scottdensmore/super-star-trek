package config

import (
	"encoding/json"
	"errors"
	"os"
	"path/filepath"
	"strings"
)

// AudioMode represents the supported audio output subsystem.
type AudioMode string

const (
	// AudioModeAuto automatically chooses the best available audio output.
	AudioModeAuto AudioMode = "auto"
	// AudioModeNative uses OS-level native sound player binaries (afplay, aplay, paplay).
	AudioModeNative AudioMode = "native"
	// AudioModeBell uses terminal ASCII/visual bell escapes.
	AudioModeBell AudioMode = "bell"
	// AudioModeOff disables all sound generation.
	AudioModeOff AudioMode = "off"
)

// Config represents persistent user preferences for audio, visual presentation, and UI timing.
type Config struct {
	Volume    int       `json:"volume"`     // Volume clamped to [0, 100], default 80
	Muted     bool      `json:"muted"`      // Whether audio is muted, default false
	AudioMode AudioMode `json:"audio_mode"` // Audio output backend, default "auto"
	Theme     string    `json:"theme"`      // Active visual theme ("modern", "lcars", "crt"), default "modern"
	AnimSpeed string    `json:"anim_speed"` // Animation speed ("fast", "normal", "slow"), default "normal"
}

// DefaultConfig returns the default configuration values.
func DefaultConfig() Config {
	return Config{
		Volume:    80,
		Muted:     false,
		AudioMode: AudioModeAuto,
		Theme:     "modern",
		AnimSpeed: "normal",
	}
}

// Normalize ensures all configuration parameters fall within valid bounds and domains,
// falling back to safe defaults when encountering invalid or unrecognized values.
func (c *Config) Normalize() {
	if c.Volume < 0 {
		c.Volume = 0
	} else if c.Volume > 100 {
		c.Volume = 100
	}

	switch AudioMode(strings.ToLower(strings.TrimSpace(string(c.AudioMode)))) {
	case AudioModeAuto:
		c.AudioMode = AudioModeAuto
	case AudioModeNative:
		c.AudioMode = AudioModeNative
	case AudioModeBell:
		c.AudioMode = AudioModeBell
	case AudioModeOff:
		c.AudioMode = AudioModeOff
	default:
		c.AudioMode = AudioModeAuto
	}

	switch strings.ToLower(strings.TrimSpace(c.Theme)) {
	case "modern":
		c.Theme = "modern"
	case "lcars":
		c.Theme = "lcars"
	case "crt":
		c.Theme = "crt"
	default:
		c.Theme = "modern"
	}

	switch strings.ToLower(strings.TrimSpace(c.AnimSpeed)) {
	case "fast":
		c.AnimSpeed = "fast"
	case "normal":
		c.AnimSpeed = "normal"
	case "slow":
		c.AnimSpeed = "slow"
	default:
		c.AnimSpeed = "normal"
	}
}

// ConfigPath resolves the path to ~/.super-star-trek/config.json.
func ConfigPath() (string, error) {
	home, err := os.UserHomeDir()
	if err != nil || home == "" {
		if h := os.Getenv("HOME"); h != "" {
			return filepath.Join(h, ".super-star-trek", "config.json"), nil
		}
		if err != nil {
			return "", err
		}
		return "", errors.New("user home directory not found")
	}
	return filepath.Join(home, ".super-star-trek", "config.json"), nil
}

// LoadConfig reads the configuration file from ConfigPath.
// If the configuration file does not exist, DefaultConfig() is returned with nil error.
// If the file exists but contains invalid data or fails to be read, an error is returned
// along with DefaultConfig().
func LoadConfig() (Config, error) {
	path, err := ConfigPath()
	if err != nil {
		return DefaultConfig(), err
	}

	data, err := os.ReadFile(path)
	if err != nil {
		if os.IsNotExist(err) {
			return DefaultConfig(), nil
		}
		return DefaultConfig(), err
	}

	cfg := DefaultConfig()
	if err := json.Unmarshal(data, &cfg); err != nil {
		return DefaultConfig(), err
	}

	cfg.Normalize()
	return cfg, nil
}

// SaveConfig atomically writes the configuration to disk at ConfigPath.
// It creates the parent directory if necessary, writes to a temporary file,
// flushes with Sync, closes, and atomically renames the temporary file.
func SaveConfig(cfg Config) error {
	cfg.Normalize()

	path, err := ConfigPath()
	if err != nil {
		return err
	}

	dir := filepath.Dir(path)
	if err := os.MkdirAll(dir, 0755); err != nil {
		return err
	}

	data, err := json.MarshalIndent(cfg, "", "  ")
	if err != nil {
		return err
	}
	data = append(data, '\n')

	tmpFile, err := os.CreateTemp(dir, "config-*.tmp")
	if err != nil {
		return err
	}
	tmpPath := tmpFile.Name()
	defer func() {
		if tmpPath != "" {
			_ = os.Remove(tmpPath)
		}
	}()

	if _, err := tmpFile.Write(data); err != nil {
		_ = tmpFile.Close()
		return err
	}
	if err := tmpFile.Sync(); err != nil {
		_ = tmpFile.Close()
		return err
	}
	if err := tmpFile.Close(); err != nil {
		return err
	}

	if err := os.Rename(tmpPath, path); err != nil {
		return err
	}
	tmpPath = ""

	return nil
}
