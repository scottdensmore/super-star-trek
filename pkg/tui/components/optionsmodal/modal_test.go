package optionsmodal

import (
	"strings"
	"testing"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/scottdensmore/super-star-trek/pkg/audio"
	"github.com/scottdensmore/super-star-trek/pkg/config"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

func TestOptionsModal_NavigationAndCycle(t *testing.T) {
	th := theme.DefaultTheme()
	rules := engine.DefaultRulesForProfile(engine.ProfileNormal)
	m := New(th, rules)

	// Default row is 0 (Difficulty Preset)
	if m.SelectedRow != 0 {
		t.Fatalf("expected initial SelectedRow 0, got %d", m.SelectedRow)
	}

	// Press right arrow to cycle preset to Hardcore
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRight})
	if m.Rules().Profile != engine.ProfileHardcore {
		t.Errorf("expected ProfileHardcore after cycling right, got %s", m.Rules().Profile)
	}
	if m.Rules().Surveillance != engine.SurveillanceLocal {
		t.Errorf("expected SurveillanceLocal from preset cascade, got %s", m.Rules().Surveillance)
	}

	// Navigate down to Surveillance row (row 1)
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyDown})
	if m.SelectedRow != 1 {
		t.Fatalf("expected SelectedRow 1, got %d", m.SelectedRow)
	}

	// Cycle surveillance to Blackout
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRight})
	if m.Rules().Surveillance != engine.SurveillanceBlackout {
		t.Errorf("expected SurveillanceBlackout, got %s", m.Rules().Surveillance)
	}
	// Manual adjustment should tag profile as Custom
	if m.Rules().Profile != engine.ProfileCustom {
		t.Errorf("expected ProfileCustom after manual setting change, got %s", m.Rules().Profile)
	}
}

func TestOptionsModal_RenderLayout(t *testing.T) {
	th := theme.DefaultTheme()
	rules := engine.DefaultRulesForProfile(engine.ProfileNormal)
	m := New(th, rules)

	view := m.View()
	expectedStrings := []string{
		"STARFLEET CONFIGURATION & RULES",
		"Difficulty Profile",
		"Starbase Surveillance",
		"Sensor Degradation",
		"Repair Multiplier",
		"Klingon Cloaking",
		"Combat Animations",
		"Color Mode",
		"Sound FX",
		"Audio Mode",
		"Audio Volume",
		"Sound Test",
		"[■■■■■■■■··] 80%",
	}
	for _, exp := range expectedStrings {
		if !strings.Contains(view, exp) {
			t.Errorf("expected modal view to contain %q, view:\n%s", exp, view)
		}
	}
}

func TestOptionsModal_CloseAndActive(t *testing.T) {
	th := theme.DefaultTheme()
	rules := engine.DefaultRulesForProfile(engine.ProfileNormal)
	m := New(th, rules)

	if !m.Active() {
		t.Errorf("expected modal to be active initially")
	}
	if m.Closed {
		t.Errorf("expected modal not to be closed initially")
	}

	// Press esc to close
	mEsc, _ := m.Update(tea.KeyMsg{Type: tea.KeyEsc})
	if !mEsc.Closed || mEsc.Active() {
		t.Errorf("expected modal to be closed after Esc, got Closed=%v, Active=%v", mEsc.Closed, mEsc.Active())
	}

	// Press 'q' to close
	mQ, _ := m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'q'}})
	if !mQ.Closed || mQ.Active() {
		t.Errorf("expected modal to be closed after 'q', got Closed=%v, Active=%v", mQ.Closed, mQ.Active())
	}

	// Navigate to RowDone and press Enter
	mDone := m
	mDone.SelectedRow = RowDone
	mDone, _ = mDone.Update(tea.KeyMsg{Type: tea.KeyEnter})
	if !mDone.Closed || mDone.Active() {
		t.Errorf("expected modal to be closed after Enter on RowDone, got Closed=%v, Active=%v", mDone.Closed, mDone.Active())
	}
}

func TestOptionsModal_CycleAllOptions(t *testing.T) {
	th := theme.DefaultTheme()
	rules := engine.DefaultRulesForProfile(engine.ProfileNormal)
	m := New(th, rules)

	// Sensor degradation toggle (RowSensorDegradation)
	m.SelectedRow = RowSensorDegradation
	origSensor := m.Rules().SensorDegradation
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRight})
	if m.Rules().SensorDegradation == origSensor {
		t.Errorf("expected SensorDegradation to toggle")
	}
	if m.Rules().Profile != engine.ProfileCustom {
		t.Errorf("expected ProfileCustom after toggling SensorDegradation")
	}

	// Repair multiplier cycle (RowRepairMultiplier)
	m.SelectedRow = RowRepairMultiplier
	origRepair := m.Rules().RepairMultiplier
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRight})
	if m.Rules().RepairMultiplier == origRepair {
		t.Errorf("expected RepairMultiplier to change")
	}

	// Klingon cloak toggle (RowKlingonCloak)
	m.SelectedRow = RowKlingonCloak
	origCloak := m.Rules().KlingonCloak
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRight})
	if m.Rules().KlingonCloak == origCloak {
		t.Errorf("expected KlingonCloak to toggle")
	}

	// Time margin cycle (RowTimeMargin)
	m.SelectedRow = RowTimeMargin
	origTime := m.Rules().TimeMargin
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRight})
	if m.Rules().TimeMargin == origTime {
		t.Errorf("expected TimeMargin to change")
	}

	// SetRules
	customRules := engine.DefaultRulesForProfile(engine.ProfileCasual)
	m.SetRules(customRules)
	if m.Rules().Profile != engine.ProfileCasual {
		t.Errorf("expected ProfileCasual after SetRules, got %s", m.Rules().Profile)
	}
}

func TestOptionsModal_NavigationWrapping(t *testing.T) {
	th := theme.DefaultTheme()
	rules := engine.DefaultRulesForProfile(engine.ProfileNormal)
	m := New(th, rules)

	// Up from 0 should wrap to NumRows - 1 (RowDone)
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyUp})
	if m.SelectedRow != RowDone {
		t.Errorf("expected RowDone after wrapping up from 0, got %d", m.SelectedRow)
	}

	// Down from RowDone should wrap to 0 (RowProfile)
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyDown})
	if m.SelectedRow != RowProfile {
		t.Errorf("expected RowProfile after wrapping down from RowDone, got %d", m.SelectedRow)
	}
}

func TestOptionsModal_ColorModeRow(t *testing.T) {
	th := theme.DefaultTheme().WithColorMode(theme.ColorModeAuto)
	m := New(th, engine.DefaultRulesForProfile(engine.ProfileNormal))

	// Set selected row to RowColorMode
	m.SelectedRow = RowColorMode
	if m.ColorMode() != theme.ColorModeAuto {
		t.Fatalf("expected initial ColorMode to be Auto, got %s", m.ColorMode())
	}

	// Press Right arrow to cycle to Dark
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRight})
	if m.ColorMode() != theme.ColorModeDark {
		t.Fatalf("expected ColorMode to be Dark after Right arrow, got %s", m.ColorMode())
	}

	// Press Right arrow again to cycle to Light
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRight})
	if m.ColorMode() != theme.ColorModeLight {
		t.Fatalf("expected ColorMode to be Light after Right arrow, got %s", m.ColorMode())
	}

	// Press Left arrow to cycle back to Dark
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyLeft})
	if m.ColorMode() != theme.ColorModeDark {
		t.Fatalf("expected ColorMode to be Dark after Left arrow, got %s", m.ColorMode())
	}

	// Space key cycles forward to Light
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeySpace})
	if m.ColorMode() != theme.ColorModeLight {
		t.Fatalf("expected ColorMode to be Light after Space, got %s", m.ColorMode())
	}

	// SetColorMode
	m.SetColorMode(theme.ColorModeAuto)
	if m.ColorMode() != theme.ColorModeAuto {
		t.Fatalf("expected ColorMode to be Auto after SetColorMode, got %s", m.ColorMode())
	}

	// View rendering contains Color Mode and AUTO
	view := m.View()
	if !strings.Contains(view, "Color Mode") {
		t.Errorf("expected view to contain 'Color Mode', got:\n%s", view)
	}
	if !strings.Contains(view, "AUTO") {
		t.Errorf("expected view to contain 'AUTO', got:\n%s", view)
	}
}

func TestOptionsModal_AnimSpeedRow(t *testing.T) {
	th := theme.DefaultTheme()
	rules := engine.DefaultRulesForProfile(engine.ProfileNormal)
	m := New(th, rules)

	// Default AnimSpeed is Normal (2)
	if m.Rules().AnimSpeed != 2 {
		t.Fatalf("expected initial AnimSpeed to be 2 (Normal), got %d", m.Rules().AnimSpeed)
	}

	m.SelectedRow = RowAnimSpeed

	// Right arrow cycles 2 -> 3 (Cinematic)
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRight})
	if m.Rules().AnimSpeed != 3 {
		t.Fatalf("expected AnimSpeed 3 after right arrow, got %d", m.Rules().AnimSpeed)
	}

	// Right arrow cycles 3 -> 0 (Off)
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRight})
	if m.Rules().AnimSpeed != 0 {
		t.Fatalf("expected AnimSpeed 0 after right arrow, got %d", m.Rules().AnimSpeed)
	}

	// Space cycles 0 -> 1 (Fast)
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeySpace})
	if m.Rules().AnimSpeed != 1 {
		t.Fatalf("expected AnimSpeed 1 after space, got %d", m.Rules().AnimSpeed)
	}

	// Left arrow cycles 1 -> 0 (Off)
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyLeft})
	if m.Rules().AnimSpeed != 0 {
		t.Fatalf("expected AnimSpeed 0 after left arrow, got %d", m.Rules().AnimSpeed)
	}

	// Left arrow wraps 0 -> 3 (Cinematic)
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyLeft})
	if m.Rules().AnimSpeed != 3 {
		t.Fatalf("expected AnimSpeed 3 after left wrap, got %d", m.Rules().AnimSpeed)
	}

	// View rendering checks
	view := m.View()
	if !strings.Contains(view, "Combat Animations") {
		t.Errorf("expected view to contain 'Combat Animations', got:\n%s", view)
	}
	if !strings.Contains(view, "CINEMATIC") {
		t.Errorf("expected view to contain 'CINEMATIC', got:\n%s", view)
	}
}

func TestOptionsModal_AudioRow(t *testing.T) {
	th := theme.DefaultTheme()
	rules := engine.DefaultRulesForProfile(engine.ProfileNormal)
	m := New(th, rules)

	// Default state is AudioEnabled() == true
	if !m.AudioEnabled() {
		t.Fatalf("expected initial AudioEnabled to be true, got %v", m.AudioEnabled())
	}

	m.SelectedRow = RowAudio

	// Toggle with Right arrow: true -> false
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRight})
	if m.AudioEnabled() {
		t.Fatalf("expected AudioEnabled to be false after Right arrow, got %v", m.AudioEnabled())
	}

	// Toggle with Left arrow: false -> true
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyLeft})
	if !m.AudioEnabled() {
		t.Fatalf("expected AudioEnabled to be true after Left arrow, got %v", m.AudioEnabled())
	}

	// Toggle with Space: true -> false
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeySpace})
	if m.AudioEnabled() {
		t.Fatalf("expected AudioEnabled to be false after Space, got %v", m.AudioEnabled())
	}

	// Toggle with Enter: false -> true
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyEnter})
	if !m.AudioEnabled() {
		t.Fatalf("expected AudioEnabled to be true after Enter, got %v", m.AudioEnabled())
	}

	// View rendering when ENABLED
	viewEnabled := m.View()
	if !strings.Contains(viewEnabled, "Sound FX") {
		t.Errorf("expected view to contain 'Sound FX', got:\n%s", viewEnabled)
	}
	if !strings.Contains(viewEnabled, "[ENABLED]") {
		t.Errorf("expected view to contain '[ENABLED]', got:\n%s", viewEnabled)
	}

	// View rendering when DISABLED
	m.SetAudioEnabled(false)
	if m.AudioEnabled() {
		t.Fatalf("expected AudioEnabled to be false after SetAudioEnabled(false)")
	}
	viewDisabled := m.View()
	if !strings.Contains(viewDisabled, "[DISABLED]") {
		t.Errorf("expected view to contain '[DISABLED]', got:\n%s", viewDisabled)
	}

	// Navigation: Down from RowAudio should be RowAudioMode
	m.SelectedRow = RowAudio
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyDown})
	if m.SelectedRow != RowAudioMode {
		t.Errorf("expected SelectedRow to be RowAudioMode after Down from RowAudio, got %v", m.SelectedRow)
	}

	// Up from RowAudioMode should be RowAudio
	m.SelectedRow = RowAudioMode
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyUp})
	if m.SelectedRow != RowAudio {
		t.Errorf("expected SelectedRow to be RowAudio after Up from RowAudioMode, got %v", m.SelectedRow)
	}

	// Up from RowAudio should be RowColorMode
	m.SelectedRow = RowAudio
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyUp})
	if m.SelectedRow != RowColorMode {
		t.Errorf("expected SelectedRow to be RowColorMode after Up from RowAudio, got %v", m.SelectedRow)
	}

	// Down from RowColorMode should be RowAudio
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyDown})
	if m.SelectedRow != RowAudio {
		t.Errorf("expected SelectedRow to be RowAudio after Down from RowColorMode, got %v", m.SelectedRow)
	}
}

func TestOptionsModal_AudioNavigation(t *testing.T) {
	th := theme.DefaultTheme()
	rules := engine.DefaultRulesForProfile(engine.ProfileNormal)
	m := New(th, rules)

	m.SelectedRow = RowColorMode
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyDown})
	if m.SelectedRow != RowAudio {
		t.Errorf("expected RowAudio down from RowColorMode, got %v", m.SelectedRow)
	}
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyDown})
	if m.SelectedRow != RowAudioMode {
		t.Errorf("expected RowAudioMode down from RowAudio, got %v", m.SelectedRow)
	}
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyDown})
	if m.SelectedRow != RowVolume {
		t.Errorf("expected RowVolume down from RowAudioMode, got %v", m.SelectedRow)
	}
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyDown})
	if m.SelectedRow != RowSoundTest {
		t.Errorf("expected RowSoundTest down from RowVolume, got %v", m.SelectedRow)
	}
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyDown})
	if m.SelectedRow != RowDone {
		t.Errorf("expected RowDone down from RowSoundTest, got %v", m.SelectedRow)
	}
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyUp})
	if m.SelectedRow != RowSoundTest {
		t.Errorf("expected RowSoundTest up from RowDone, got %v", m.SelectedRow)
	}
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyUp})
	if m.SelectedRow != RowVolume {
		t.Errorf("expected RowVolume up from RowSoundTest, got %v", m.SelectedRow)
	}
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyUp})
	if m.SelectedRow != RowAudioMode {
		t.Errorf("expected RowAudioMode up from RowVolume, got %v", m.SelectedRow)
	}
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyUp})
	if m.SelectedRow != RowAudio {
		t.Errorf("expected RowAudio up from RowAudioMode, got %v", m.SelectedRow)
	}
}

type mockAudioPlayer struct {
	audio.NullPlayer
	playedSound audio.SoundID
	playCount   int
	volume      int
}

func (m *mockAudioPlayer) Play(s audio.SoundID) {
	m.playedSound = s
	m.playCount++
}

func (m *mockAudioPlayer) SetVolume(v int) {
	m.volume = v
}

func TestOptionsModalAudioRows(t *testing.T) {
	th := theme.DefaultTheme()
	rules := engine.DefaultRulesForProfile(engine.ProfileNormal)
	m := New(th, rules)
	m.SetVolume(70)
	if m.Volume() != 70 {
		t.Errorf("expected volume 70, got %d", m.Volume())
	}

	// Test adjusting volume via Right arrow on RowVolume
	m.SelectedRow = RowVolume
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRight})
	if m.Volume() != 80 {
		t.Errorf("expected volume 80 after KeyRight, got %d", m.Volume())
	}

	// Test adjusting volume via Left arrow on RowVolume
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyLeft})
	if m.Volume() != 70 {
		t.Errorf("expected volume 70 after KeyLeft, got %d", m.Volume())
	}

	// Test Volume boundary clamping at 100
	m.SetVolume(95)
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRight})
	if m.Volume() != 100 {
		t.Errorf("expected volume clamped to 100, got %d", m.Volume())
	}
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRight})
	if m.Volume() != 100 {
		t.Errorf("expected volume still 100, got %d", m.Volume())
	}

	// Test Volume boundary clamping at 0
	m.SetVolume(5)
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyLeft})
	if m.Volume() != 0 {
		t.Errorf("expected volume clamped to 0, got %d", m.Volume())
	}
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyLeft})
	if m.Volume() != 0 {
		t.Errorf("expected volume still 0, got %d", m.Volume())
	}

	// Test SetAudioMode and AudioMode getter
	m.SetAudioMode(config.AudioModeNative)
	if m.AudioMode() != config.AudioModeNative {
		t.Errorf("expected audio mode Native, got %s", m.AudioMode())
	}

	// Test AudioMode cycling on RowAudioMode: Auto -> Native -> Bell -> Off -> Auto
	m.SelectedRow = RowAudioMode
	m.SetAudioMode(config.AudioModeAuto)
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRight})
	if m.AudioMode() != config.AudioModeNative {
		t.Errorf("expected AudioModeNative after KeyRight, got %s", m.AudioMode())
	}
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRight})
	if m.AudioMode() != config.AudioModeBell {
		t.Errorf("expected AudioModeBell after KeyRight, got %s", m.AudioMode())
	}
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRight})
	if m.AudioMode() != config.AudioModeOff {
		t.Errorf("expected AudioModeOff after KeyRight, got %s", m.AudioMode())
	}
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRight})
	if m.AudioMode() != config.AudioModeAuto {
		t.Errorf("expected AudioModeAuto after KeyRight wrap, got %s", m.AudioMode())
	}
	// Left arrow cycles backward
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyLeft})
	if m.AudioMode() != config.AudioModeOff {
		t.Errorf("expected AudioModeOff after KeyLeft, got %s", m.AudioMode())
	}

	// Test Sound Test navigation and Enter key
	m.SelectedRow = RowSoundTest
	testSound := m.SelectedSound()
	if testSound == "" {
		t.Errorf("expected non-empty selected sound")
	}
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRight})
	if m.SelectedSound() == testSound {
		t.Errorf("expected cycling to next sound on KeyRight")
	}

	// Test auditioning with Enter key
	currSound := m.SelectedSound()
	var cmd tea.Cmd
	m, cmd = m.Update(tea.KeyMsg{Type: tea.KeyEnter})
	if cmd == nil {
		t.Fatalf("expected non-nil tea.Cmd on KeyEnter for RowSoundTest")
	}
	msg := cmd()
	soundMsg, ok := msg.(PlaySoundMsg)
	if !ok {
		t.Fatalf("expected PlaySoundMsg from audition cmd, got %T", msg)
	}
	if soundMsg.Sound != currSound {
		t.Errorf("expected PlaySoundMsg.Sound=%s, got %s", currSound, soundMsg.Sound)
	}

	// Test auditioning with Space key
	m, cmd = m.Update(tea.KeyMsg{Type: tea.KeySpace})
	if cmd == nil {
		t.Fatalf("expected non-nil tea.Cmd on KeySpace for RowSoundTest")
	}
	msg = cmd()
	soundMsg, ok = msg.(PlaySoundMsg)
	if !ok || soundMsg.Sound != currSound {
		t.Errorf("expected PlaySoundMsg with %s on KeySpace", currSound)
	}

	// Test Injected Player
	mock := &mockAudioPlayer{}
	m.SetPlayer(mock)
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyEnter})
	if mock.playedSound != currSound {
		t.Errorf("expected injected player to play %s, got %s", currSound, mock.playedSound)
	}
}

func TestOptionsModal_AuditionVolumeSync(t *testing.T) {
	th := theme.DefaultTheme()
	rules := engine.DefaultRulesForProfile(engine.ProfileNormal)
	m := New(th, rules)
	mock := &mockAudioPlayer{}
	m.SetPlayer(mock)

	// Set a volume and test audition
	m.SetVolume(50)
	m.SelectedRow = RowSoundTest
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyEnter})
	if mock.volume != 50 {
		t.Errorf("expected mock volume 50 on audition, got %d", mock.volume)
	}

	// Change volume via keys
	m.SelectedRow = RowVolume
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRight}) // +10 = 60
	if mock.volume != 60 {
		t.Errorf("expected mock volume 60 on cycle, got %d", mock.volume)
	}

	// Audition again
	m.SelectedRow = RowSoundTest
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyEnter})
	if mock.volume != 60 {
		t.Errorf("expected mock volume 60 on audition, got %d", mock.volume)
	}
}
