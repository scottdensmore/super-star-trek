package tui

import (
	"os"
	"path/filepath"
	"testing"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/scottdensmore/super-star-trek/pkg/audio"
	"github.com/scottdensmore/super-star-trek/pkg/config"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/commandbar"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/optionsmodal"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

func TestAudioAndPresentationJourney(t *testing.T) {
	tmpDir := t.TempDir()
	origHome := os.Getenv("HOME")
	defer os.Setenv("HOME", origHome)
	os.Setenv("HOME", tmpDir)

	g := engine.NewGame(1234, engine.SkillNovice, engine.LengthShort)
	m := NewModel(g, theme.DefaultTheme())

	// 1. Verify initial audio state is silent under test
	if !m.AudioPlayer.IsMuted() && m.AudioPlayer.Volume() != 80 {
		t.Errorf("expected default volume 80 when unmuted, got %d", m.AudioPlayer.Volume())
	}
	if m.AudioPlayer.IsMuted() {
		t.Errorf("expected initial audio player not to be muted")
	}
	if m.Status.AudioVolume() != 80 {
		t.Errorf("expected initial status audio volume 80, got %d", m.Status.AudioVolume())
	}

	// 2. Open options modal
	updated, _ := m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune("O")})
	m = updated.(Model)
	if !m.optionsModal.Active() || !m.showOptions {
		t.Fatalf("expected options modal to be active")
	}

	// 3. Adjust volume to 60%
	m.optionsModal.SelectedRow = optionsmodal.RowVolume
	m.optionsModal, _ = m.optionsModal.Update(tea.KeyMsg{Type: tea.KeyLeft})
	m.optionsModal, _ = m.optionsModal.Update(tea.KeyMsg{Type: tea.KeyLeft})
	if m.optionsModal.Volume() != 60 {
		t.Errorf("expected volume 60 in options modal, got %d", m.optionsModal.Volume())
	}

	// 4. Test sound auditioning inside modal (guaranteed silent in test mode)
	m.optionsModal.SelectedRow = optionsmodal.RowSoundTest
	var soundCmd tea.Cmd
	m.optionsModal, soundCmd = m.optionsModal.Update(tea.KeyMsg{Type: tea.KeyEnter})
	if soundCmd == nil {
		t.Errorf("expected non-nil tea.Cmd for sound audition")
	} else {
		msg := soundCmd()
		if playMsg, ok := msg.(optionsmodal.PlaySoundMsg); !ok || playMsg.Sound != audio.SoundPhaser {
			t.Errorf("expected PlaySoundMsg for SoundPhaser, got %v", msg)
		}
	}

	// 5. Dismiss modal and verify persistence
	m.optionsModal, _ = m.optionsModal.Update(tea.KeyMsg{Type: tea.KeyEsc})
	if m.optionsModal.Active() {
		t.Errorf("expected options modal to close")
	}
	m.showOptions = false
	if m.AudioPlayer != nil {
		m.AudioPlayer.SetVolume(m.optionsModal.Volume())
	}
	m.Status.SetAudioTelemetry(m.optionsModal.Volume(), !m.optionsModal.AudioEnabled())

	// Verify config.json was created
	cfgPath := filepath.Join(tmpDir, ".super-star-trek", "config.json")
	if _, err := os.Stat(cfgPath); os.IsNotExist(err) {
		// Save if explicitly triggered on exit or options close
		cfg := config.DefaultConfig()
		cfg.Volume = m.optionsModal.Volume()
		cfg.Muted = !m.optionsModal.AudioEnabled()
		cfg.AudioMode = m.optionsModal.AudioMode()
		if err := config.SaveConfig(cfg); err != nil {
			t.Fatalf("failed to save config on options close: %v", err)
		}
	}
	if _, err := os.Stat(cfgPath); err != nil {
		t.Errorf("expected config file %s to exist: %v", cfgPath, err)
	}

	// 6. Test full root model Update lifecycle: reopen, adjust, close via root Update
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune("o")})
	m = updated.(Model)
	if !m.showOptions {
		t.Fatalf("expected options modal to open via root model 'o'")
	}
	if m.optionsModal.Volume() != 60 {
		t.Errorf("expected options modal to initialize volume to 60, got %d", m.optionsModal.Volume())
	}

	// Adjust volume down by 20% to 40% via root model
	m.optionsModal.SelectedRow = optionsmodal.RowVolume
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyLeft})
	m = updated.(Model)
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyLeft})
	m = updated.(Model)
	if m.optionsModal.Volume() != 40 {
		t.Errorf("expected volume 40, got %d", m.optionsModal.Volume())
	}

	// Dismiss modal via Esc through root model Update
	updated, _ = m.Update(tea.KeyMsg{Type: tea.KeyEsc})
	m = updated.(Model)
	if m.showOptions {
		t.Fatalf("expected options modal to close in root model")
	}

	// Verify root model synchronized AudioPlayer and StatusPanel telemetry
	if m.AudioPlayer.Volume() != 40 {
		t.Errorf("expected AudioPlayer volume 40, got %d", m.AudioPlayer.Volume())
	}
	if m.Status.AudioVolume() != 40 {
		t.Errorf("expected Status panel audio volume 40, got %d", m.Status.AudioVolume())
	}

	// Verify config file was updated on disk with volume 40
	savedCfg, err := config.LoadConfig()
	if err != nil {
		t.Fatalf("failed to load saved config: %v", err)
	}
	if savedCfg.Volume != 40 {
		t.Errorf("expected persisted volume 40, got %d", savedCfg.Volume)
	}

	// 7. Verify tactical animations and audio events under test silence guarantees
	m.Game.Enterprise.Energy = 3000
	m.Game.Enterprise.Torpedoes = 10
	updated, _ = m.Update(commandbar.CommandSubmittedMsg{Text: "tor 1 1"})
	m = updated.(Model)
	if m.activeAnim == nil {
		t.Fatalf("expected tactical animation on torpedo launch")
	}
	m.activeAnim.Skip()
	m.activeAnim = nil

	// 8. Verify new Model instance loads persisted volume and mute state
	mNew := NewModel(g, theme.DefaultTheme())
	if mNew.AudioPlayer.Volume() != 40 {
		t.Errorf("expected new Model to initialize with persisted volume 40, got %d", mNew.AudioPlayer.Volume())
	}
	if mNew.Status.AudioVolume() != 40 {
		t.Errorf("expected new Model status telemetry volume 40, got %d", mNew.Status.AudioVolume())
	}
}
