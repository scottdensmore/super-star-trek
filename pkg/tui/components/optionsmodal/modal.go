package optionsmodal

import (
	"fmt"
	"strings"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/scottdensmore/super-star-trek/pkg/audio"
	"github.com/scottdensmore/super-star-trek/pkg/config"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

// Row identifies a configurable option or action row in the modal.
type Row int

const (
	RowProfile Row = iota
	RowSurveillance
	RowSensorDegradation
	RowRepairMultiplier
	RowKlingonCloak
	RowTimeMargin
	RowAnimSpeed
	RowColorMode
	RowAudio
	RowAudioMode
	RowVolume
	RowSoundTest
	RowDone
	NumRows
)

// PlaySoundMsg is emitted when a sound test audition is triggered.
type PlaySoundMsg struct {
	Sound audio.SoundID
}

var defaultTestSounds = []audio.SoundID{
	audio.SoundPhaser,
	audio.SoundTorpedoLaunch,
	audio.SoundExplosion,
	audio.SoundRedAlert,
	audio.SoundDock,
	audio.SoundWarp,
	audio.SoundDamage,
	audio.SoundShields,
	audio.SoundVictory,
	audio.SoundDefeat,
	audio.SoundCloak,
	audio.SoundDecloak,
	audio.SoundPlasmaLaunch,
	audio.SoundPlasmaImpact,
	audio.SoundTholianWeb,
	audio.SoundWebBreached,
	audio.SoundPointDefense,
	audio.SoundCommChime,
	audio.SoundComputerBeep,
}

// Model represents the interactive options modal overlay component.
type Model struct {
	Theme        theme.Theme
	rules        engine.GameRules
	colorMode    theme.ColorMode
	audioEnabled bool
	audioMode    config.AudioMode
	volume       int
	testSoundIdx int
	testSounds   []audio.SoundID
	player       audio.Player
	SelectedRow  Row
	Closed       bool
	active       bool
}

// New creates a new options modal initialized with the given theme and rules.
func New(th theme.Theme, rules engine.GameRules) Model {
	if th == nil {
		th = theme.DefaultTheme()
	}
	testSounds := make([]audio.SoundID, len(defaultTestSounds))
	copy(testSounds, defaultTestSounds)
	return Model{
		Theme:        th,
		rules:        rules,
		colorMode:    th.ColorMode(),
		audioEnabled: true,
		audioMode:    config.AudioModeAuto,
		volume:       80,
		testSoundIdx: 0,
		testSounds:   testSounds,
		SelectedRow:  RowProfile,
		Closed:       false,
		active:       true,
	}
}

// AudioEnabled returns whether audio sound FX are enabled in the modal.
func (m Model) AudioEnabled() bool {
	return m.audioEnabled
}

// SetAudioEnabled updates the audio enabled state inside the modal.
func (m *Model) SetAudioEnabled(enabled bool) {
	m.audioEnabled = enabled
}

// Volume returns the audio playback volume (0-100).
func (m Model) Volume() int {
	return m.volume
}

// SetVolume updates the audio playback volume clamped to [0, 100].
func (m *Model) SetVolume(vol int) {
	if vol < 0 {
		vol = 0
	} else if vol > 100 {
		vol = 100
	}
	m.volume = vol
}

// AudioMode returns the configured audio subsystem mode.
func (m Model) AudioMode() config.AudioMode {
	return m.audioMode
}

// SetAudioMode updates the audio subsystem mode.
func (m *Model) SetAudioMode(mode config.AudioMode) {
	switch mode {
	case config.AudioModeAuto, config.AudioModeNative, config.AudioModeBell, config.AudioModeOff:
		m.audioMode = mode
	default:
		m.audioMode = config.AudioModeAuto
	}
}

// SelectedSound returns the currently selected sound ID for testing.
func (m Model) SelectedSound() audio.SoundID {
	if len(m.testSounds) == 0 {
		return ""
	}
	idx := m.testSoundIdx % len(m.testSounds)
	if idx < 0 {
		idx += len(m.testSounds)
	}
	return m.testSounds[idx]
}

// SetSelectedSound sets the selected test sound if found in the palette.
func (m *Model) SetSelectedSound(s audio.SoundID) {
	for i, sound := range m.testSounds {
		if sound == s {
			m.testSoundIdx = i
			return
		}
	}
}

// SetPlayer injects an audio player for direct sound auditioning.
func (m *Model) SetPlayer(p audio.Player) {
	m.player = p
}

// Player returns the injected audio player, if any.
func (m Model) Player() audio.Player {
	return m.player
}

// Rules returns the current game rules configured in the modal.
func (m Model) Rules() engine.GameRules {
	return m.rules
}

// SetRules updates the game rules inside the modal.
func (m *Model) SetRules(r engine.GameRules) {
	m.rules = r
}

// ColorMode returns the configured color mode in the modal.
func (m Model) ColorMode() theme.ColorMode {
	return m.colorMode
}

// SetColorMode updates the color mode inside the modal.
func (m *Model) SetColorMode(mode theme.ColorMode) {
	m.colorMode = mode
}

// SetTheme updates the active styling theme and syncs color mode.
func (m *Model) SetTheme(th theme.Theme) {
	if th == nil {
		th = theme.DefaultTheme()
	}
	m.Theme = th
	m.colorMode = th.ColorMode()
}

// Active reports whether the modal is currently open and accepting input.
func (m Model) Active() bool {
	return !m.Closed && m.active
}

// SetActive sets whether the modal is actively open and accepting input.
func (m *Model) SetActive(active bool) {
	m.active = active
	if active {
		m.Closed = false
	}
}

// Update handles keyboard navigation and option cycling.
func (m Model) Update(msg tea.Msg) (Model, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.KeyMsg:
		switch msg.String() {
		case "up", "k":
			if m.SelectedRow > 0 {
				m.SelectedRow--
			} else {
				m.SelectedRow = NumRows - 1
			}
		case "down", "j":
			if m.SelectedRow < NumRows-1 {
				m.SelectedRow++
			} else {
				m.SelectedRow = 0
			}
		case "left", "h":
			m.cycleOption(-1)
		case "right", "l":
			m.cycleOption(1)
		case " ", "space":
			if m.SelectedRow == RowSoundTest {
				return m, m.auditionSound()
			}
			m.cycleOption(1)
		case "enter":
			if m.SelectedRow == RowDone {
				m.Closed = true
				m.active = false
			} else if m.SelectedRow == RowSoundTest {
				return m, m.auditionSound()
			} else {
				m.cycleOption(1)
			}
		case "esc", "q":
			m.Closed = true
			m.active = false
		}
	}
	return m, nil
}

func (m Model) auditionSound() tea.Cmd {
	sound := m.SelectedSound()
	if sound == "" {
		return nil
	}
	if m.player != nil {
		m.player.Play(sound)
	}
	return func() tea.Msg {
		return PlaySoundMsg{Sound: sound}
	}
}

func (m *Model) cycleOption(dir int) {
	profiles := []engine.DifficultyProfile{engine.ProfileCasual, engine.ProfileNormal, engine.ProfileHardcore, engine.ProfileNightmare, engine.ProfileExpert, engine.ProfileEmeritus, engine.ProfileCustom}
	survModes := []engine.SurveillanceMode{engine.SurveillanceFull, engine.SurveillanceClassic, engine.SurveillanceLocal, engine.SurveillanceBlackout}
	repMults := []float64{0.75, 1.00, 1.50, 2.00}
	timeMargins := []float64{1.25, 1.00, 0.80, 0.60}

	switch m.SelectedRow {
	case RowProfile:
		idx := 0
		for i, p := range profiles {
			if p == m.rules.Profile {
				idx = i
				break
			}
		}
		newIdx := (idx + dir + len(profiles)) % len(profiles)
		if profiles[newIdx] != engine.ProfileCustom {
			m.rules = engine.DefaultRulesForProfile(profiles[newIdx])
		} else {
			m.rules.Profile = engine.ProfileCustom
		}
	case RowSurveillance:
		idx := 0
		for i, s := range survModes {
			if s == m.rules.Surveillance {
				idx = i
				break
			}
		}
		m.rules.Surveillance = survModes[(idx+dir+len(survModes))%len(survModes)]
		m.rules.Profile = engine.ProfileCustom
	case RowSensorDegradation:
		m.rules.SensorDegradation = !m.rules.SensorDegradation
		m.rules.Profile = engine.ProfileCustom
	case RowRepairMultiplier:
		idx := 0
		for i, rm := range repMults {
			if rm == m.rules.RepairMultiplier {
				idx = i
				break
			}
		}
		m.rules.RepairMultiplier = repMults[(idx+dir+len(repMults))%len(repMults)]
		m.rules.Profile = engine.ProfileCustom
	case RowKlingonCloak:
		m.rules.KlingonCloak = !m.rules.KlingonCloak
		m.rules.Profile = engine.ProfileCustom
	case RowTimeMargin:
		idx := 0
		for i, tm := range timeMargins {
			if tm == m.rules.TimeMargin {
				idx = i
				break
			}
		}
		m.rules.TimeMargin = timeMargins[(idx+dir+len(timeMargins))%len(timeMargins)]
		m.rules.Profile = engine.ProfileCustom
	case RowAnimSpeed:
		m.rules.AnimSpeed = (m.rules.AnimSpeed + dir + 4) % 4
	case RowColorMode:
		colorModes := []theme.ColorMode{theme.ColorModeAuto, theme.ColorModeDark, theme.ColorModeLight}
		idx := 0
		for i, cm := range colorModes {
			if cm == m.colorMode {
				idx = i
				break
			}
		}
		m.colorMode = colorModes[(idx+dir+len(colorModes))%len(colorModes)]
	case RowAudio:
		m.audioEnabled = !m.audioEnabled
	case RowAudioMode:
		audioModes := []config.AudioMode{config.AudioModeAuto, config.AudioModeNative, config.AudioModeBell, config.AudioModeOff}
		idx := 0
		for i, am := range audioModes {
			if am == m.audioMode {
				idx = i
				break
			}
		}
		newIdx := (idx + dir) % len(audioModes)
		if newIdx < 0 {
			newIdx += len(audioModes)
		}
		m.audioMode = audioModes[newIdx]
	case RowVolume:
		newVol := m.volume + dir*10
		if newVol < 0 {
			newVol = 0
		} else if newVol > 100 {
			newVol = 100
		}
		m.volume = newVol
	case RowSoundTest:
		if len(m.testSounds) > 0 {
			newIdx := (m.testSoundIdx + dir) % len(m.testSounds)
			if newIdx < 0 {
				newIdx += len(m.testSounds)
			}
			m.testSoundIdx = newIdx
		}
	}
}

// View renders the options modal overlay dialog.
func (m Model) View() string {
	th := m.Theme
	if th == nil {
		th = theme.DefaultTheme()
	}
	styles := th.Styles()

	boxStyle := styles.Panel.
		Padding(1, 2).
		Width(66)

	titleStyle := styles.PanelTitle.
		Bold(true).
		Align(lipgloss.Center)

	var rows []string
	rows = append(rows, titleStyle.Render("⚙ STARFLEET CONFIGURATION & RULES"), "")

	renderRow := func(r Row, label, val string) string {
		prefix := "  "
		style := styles.LogText
		if m.SelectedRow == r {
			prefix = "▶ "
			style = styles.GaugeValue
		}
		return prefix + style.Render(fmt.Sprintf("%-30s ◀ %s ▶", label, val))
	}

	rows = append(rows, renderRow(RowProfile, "Difficulty Profile", strings.ToUpper(string(m.rules.Profile))))
	rows = append(rows, renderRow(RowSurveillance, "Starbase Surveillance", strings.ToUpper(string(m.rules.Surveillance))))
	degStr := "DISABLED"
	if m.rules.SensorDegradation {
		degStr = "ENABLED"
	}
	rows = append(rows, renderRow(RowSensorDegradation, "Sensor Degradation", degStr))
	rows = append(rows, renderRow(RowRepairMultiplier, "Repair Multiplier", fmt.Sprintf("%.2fx", m.rules.RepairMultiplier)))
	cloakStr := "DISABLED"
	if m.rules.KlingonCloak {
		cloakStr = "ENABLED"
	}
	rows = append(rows, renderRow(RowKlingonCloak, "Klingon Cloaking", cloakStr))
	rows = append(rows, renderRow(RowTimeMargin, "Stardate Time Margin", fmt.Sprintf("%.0f%%", m.rules.TimeMargin*100)))
	animSpeedStr := "NORMAL"
	switch m.rules.AnimSpeed {
	case 0:
		animSpeedStr = "OFF"
	case 1:
		animSpeedStr = "FAST"
	case 2:
		animSpeedStr = "NORMAL"
	case 3:
		animSpeedStr = "CINEMATIC"
	}
	rows = append(rows, renderRow(RowAnimSpeed, "Combat Animations", animSpeedStr))
	rows = append(rows, renderRow(RowColorMode, "Color Mode", strings.ToUpper(string(m.colorMode))))
	audioStr := "[DISABLED]"
	if m.audioEnabled {
		audioStr = "[ENABLED]"
	}
	rows = append(rows, renderRow(RowAudio, "Sound FX", audioStr))

	modeStr := "AUTO"
	switch m.audioMode {
	case config.AudioModeAuto:
		modeStr = "AUTO"
	case config.AudioModeNative:
		modeStr = "NATIVE OS"
	case config.AudioModeBell:
		modeStr = "TERMINAL BELL"
	case config.AudioModeOff:
		modeStr = "OFF"
	default:
		modeStr = strings.ToUpper(string(m.audioMode))
	}
	rows = append(rows, renderRow(RowAudioMode, "Audio Mode", modeStr))

	volBar := fmt.Sprintf("[%s%s] %d%%", strings.Repeat("■", m.volume/10), strings.Repeat("·", 10-m.volume/10), m.volume)
	rows = append(rows, renderRow(RowVolume, "Audio Volume", volBar))

	soundName := strings.ToUpper(strings.ReplaceAll(string(m.SelectedSound()), "_", " "))
	rows = append(rows, renderRow(RowSoundTest, "Sound Test", soundName))

	doneStyle := styles.LogText
	prefix := "  "
	if m.SelectedRow == RowDone {
		prefix = "▶ "
		doneStyle = styles.GaugeValue
	}
	rows = append(rows, "", prefix+doneStyle.Render("[ Done / Resume Mission ]"), "")

	footerStyle := styles.LogText.Italic(true)
	rows = append(rows, footerStyle.Render("↑/↓: Navigate • ←/→/Space: Change • Esc/q: Close"))

	return boxStyle.Render(strings.Join(rows, "\n"))
}
