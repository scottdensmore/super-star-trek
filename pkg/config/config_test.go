package config

import (
	"os"
	"path/filepath"
	"testing"
)

func TestDefaultConfig(t *testing.T) {
	cfg := DefaultConfig()
	if cfg.Volume != 80 {
		t.Errorf("expected default volume 80, got %d", cfg.Volume)
	}
	if cfg.Muted {
		t.Errorf("expected default muted false")
	}
	if cfg.AudioMode != AudioModeAuto {
		t.Errorf("expected default audio mode 'auto', got %s", cfg.AudioMode)
	}
	if cfg.Theme != "modern" {
		t.Errorf("expected default theme 'modern', got %s", cfg.Theme)
	}
	if cfg.AnimSpeed != "normal" {
		t.Errorf("expected default anim speed 'normal', got %s", cfg.AnimSpeed)
	}
}

func TestSaveAndLoadRoundtrip(t *testing.T) {
	tmpDir := t.TempDir()
	origHome := os.Getenv("HOME")
	defer os.Setenv("HOME", origHome)
	os.Setenv("HOME", tmpDir)

	cfg := Config{
		Volume:    65,
		Muted:     true,
		AudioMode: AudioModeNative,
		Theme:     "lcars",
		AnimSpeed: "fast",
	}

	if err := SaveConfig(cfg); err != nil {
		t.Fatalf("failed to save config: %v", err)
	}

	loaded, err := LoadConfig()
	if err != nil {
		t.Fatalf("failed to load config: %v", err)
	}
	if loaded.Volume != 65 || !loaded.Muted || loaded.AudioMode != AudioModeNative || loaded.Theme != "lcars" || loaded.AnimSpeed != "fast" {
		t.Errorf("loaded config does not match saved config: %+v", loaded)
	}
}

func TestLoadConfig_MissingFileReturnsDefault(t *testing.T) {
	tmpDir := t.TempDir()
	origHome := os.Getenv("HOME")
	defer os.Setenv("HOME", origHome)
	os.Setenv("HOME", tmpDir)

	loaded, err := LoadConfig()
	if err != nil {
		t.Fatalf("expected no error for missing config file, got %v", err)
	}
	if loaded.Volume != 80 {
		t.Errorf("expected default volume 80, got %d", loaded.Volume)
	}
	if loaded.Theme != "modern" {
		t.Errorf("expected default theme 'modern', got %s", loaded.Theme)
	}
	if loaded.AudioMode != AudioModeAuto {
		t.Errorf("expected default audio mode 'auto', got %s", loaded.AudioMode)
	}
}

func TestConfigValidationBounds(t *testing.T) {
	cfg := Config{
		Volume: 150, // out of range
	}
	cfg.Normalize()
	if cfg.Volume != 100 {
		t.Errorf("expected volume clamped to 100, got %d", cfg.Volume)
	}
	if cfg.AudioMode != AudioModeAuto {
		t.Errorf("expected audio mode normalized to auto, got %s", cfg.AudioMode)
	}
	if cfg.Theme != "modern" {
		t.Errorf("expected theme normalized to modern, got %s", cfg.Theme)
	}
	if cfg.AnimSpeed != "normal" {
		t.Errorf("expected anim speed normalized to normal, got %s", cfg.AnimSpeed)
	}

	cfg.Volume = -20
	cfg.AudioMode = "invalid_mode"
	cfg.Theme = "unknown_theme"
	cfg.AnimSpeed = "ultra_hyper"
	cfg.Normalize()
	if cfg.Volume != 0 {
		t.Errorf("expected volume clamped to 0, got %d", cfg.Volume)
	}
	if cfg.AudioMode != AudioModeAuto {
		t.Errorf("expected invalid audio mode fallback to auto, got %s", cfg.AudioMode)
	}
	if cfg.Theme != "modern" {
		t.Errorf("expected invalid theme fallback to modern, got %s", cfg.Theme)
	}
	if cfg.AnimSpeed != "normal" {
		t.Errorf("expected invalid anim speed fallback to normal, got %s", cfg.AnimSpeed)
	}
}

func TestConfigPath(t *testing.T) {
	tmpDir := t.TempDir()
	origHome := os.Getenv("HOME")
	defer os.Setenv("HOME", origHome)
	os.Setenv("HOME", tmpDir)

	p, err := ConfigPath()
	if err != nil {
		t.Fatalf("unexpected error getting config path: %v", err)
	}
	expected := filepath.Join(tmpDir, ".super-star-trek", "config.json")
	if p != expected {
		t.Errorf("expected %s, got %s", expected, p)
	}
}

func TestLoadConfig_CorruptedFileReturnsDefaultWithError(t *testing.T) {
	tmpDir := t.TempDir()
	origHome := os.Getenv("HOME")
	defer os.Setenv("HOME", origHome)
	os.Setenv("HOME", tmpDir)

	cfgDir := filepath.Join(tmpDir, ".super-star-trek")
	if err := os.MkdirAll(cfgDir, 0755); err != nil {
		t.Fatalf("failed to create temp config dir: %v", err)
	}
	badFile := filepath.Join(cfgDir, "config.json")
	if err := os.WriteFile(badFile, []byte("{invalid json corrupt"), 0644); err != nil {
		t.Fatalf("failed to write corrupted config file: %v", err)
	}

	loaded, err := LoadConfig()
	if err == nil {
		t.Fatalf("expected error for corrupted config file, got nil")
	}
	if loaded.Volume != 80 {
		t.Errorf("expected default volume 80 on corruption, got %d", loaded.Volume)
	}
}
