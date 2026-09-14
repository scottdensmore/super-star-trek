package halloffame

import (
	"fmt"
	"strings"
	"time"

	"github.com/charmbracelet/bubbles/textinput"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/charmbracelet/x/ansi"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

const (
	tabTelemetry   = 0
	tabLeaderboard = 1
)

// CloseModalMsg is emitted when the player dismisses the hall of fame modal.
type CloseModalMsg struct{}

// ScoreRecordedMsg is emitted when a player's qualifying score is recorded on the leaderboard.
type ScoreRecordedMsg struct {
	Entry engine.ScoreEntry
}

// Model represents the dual-tab Hall of Fame and Mission Telemetry modal component.
type Model struct {
	width       int
	height      int
	activeTab   int
	score       engine.ScoreBreakdown
	leaderboard *engine.Leaderboard
	promptName  bool
	textInput   textinput.Model
	theme       theme.Theme
	storagePath string
}

type hofStyles struct {
	theme.Styles
	Border lipgloss.Style
}

func getStyles(th theme.Theme) hofStyles {
	if th == nil {
		th = theme.DefaultTheme()
	}
	base := th.Styles()
	borderFg := base.Panel.GetBorderTopForeground()
	bStyle := lipgloss.NewStyle()
	if borderFg != nil {
		bStyle = bStyle.Foreground(borderFg)
	}
	return hofStyles{
		Styles: base,
		Border: bStyle,
	}
}

// New constructs a new Hall of Fame Model initialized with the given dimensions and theme.
func New(th theme.Theme, width, height int, storagePath string) Model {
	if th == nil {
		th = theme.DefaultTheme()
	}
	if width <= 0 {
		width = 66
	}
	if height <= 0 {
		height = 18
	}

	ti := textinput.New()
	ti.Prompt = ""
	ti.Placeholder = "Enter callsign..."
	ti.CharLimit = 20
	ti.Width = 20

	styles := th.Styles()
	ti.PromptStyle = styles.Prompt
	ti.TextStyle = styles.CommandText

	lb := engine.DefaultLeaderboard()
	if storagePath != "" {
		if loaded, err := engine.LoadLeaderboard(storagePath); err == nil && loaded != nil {
			lb = loaded
		}
	}

	return Model{
		width:       width,
		height:      height,
		activeTab:   tabTelemetry,
		leaderboard: lb,
		textInput:   ti,
		theme:       th,
		storagePath: storagePath,
	}
}

// SetTheme updates the active theme and reapplies input styles.
func (m *Model) SetTheme(th theme.Theme) {
	if th == nil {
		th = theme.DefaultTheme()
	}
	m.theme = th
	styles := th.Styles()
	m.textInput.PromptStyle = styles.Prompt
	m.textInput.TextStyle = styles.CommandText
}

// SetState updates the score metrics, active leaderboard reference, and callsign prompt status.
func (m *Model) SetState(score engine.ScoreBreakdown, lb *engine.Leaderboard, promptName bool) {
	m.score = score
	if m.score.RankBadge == "" {
		title, badge := engine.RankForScore(m.score.TotalScore)
		m.score.RankTitle = title
		m.score.RankBadge = badge
	}

	if lb != nil {
		m.leaderboard = lb
	} else if m.leaderboard == nil {
		m.leaderboard = engine.DefaultLeaderboard()
	}

	m.promptName = promptName
	if promptName {
		m.activeTab = tabLeaderboard
		m.textInput.SetValue("")
		m.textInput.Focus()
	} else {
		m.activeTab = tabTelemetry
	}
}

// Update processes keyboard navigation, tab switching, and callsign entry.
func (m Model) Update(msg tea.Msg) (Model, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.KeyMsg:
		if m.promptName {
			switch msg.Type {
			case tea.KeyEsc:
				m.promptName = false
				return m, func() tea.Msg { return CloseModalMsg{} }
			case tea.KeyEnter:
				name := strings.TrimSpace(m.textInput.Value())
				if name == "" {
					name = "Unknown Captain"
				}
				entry := engine.ScoreEntry{
					CaptainName: name,
					Score:       m.score.TotalScore,
					Rank:        m.score.RankBadge,
					Stardate:    m.score.ElapsedStardates,
					Date:        time.Now(),
					GameWon:     m.score.GameWon,
				}
				if m.leaderboard != nil {
					m.leaderboard.Add(entry)
					if m.storagePath != "" {
						_ = m.leaderboard.Save(m.storagePath)
					}
				}
				m.promptName = false
				return m, func() tea.Msg {
					return ScoreRecordedMsg{Entry: entry}
				}
			default:
				var cmd tea.Cmd
				m.textInput, cmd = m.textInput.Update(msg)
				return m, cmd
			}
		}

		switch msg.Type {
		case tea.KeyTab:
			if m.activeTab == tabTelemetry {
				m.activeTab = tabLeaderboard
			} else {
				m.activeTab = tabTelemetry
			}
			return m, nil
		case tea.KeyLeft:
			m.activeTab = tabTelemetry
			return m, nil
		case tea.KeyRight:
			m.activeTab = tabLeaderboard
			return m, nil
		case tea.KeyEsc, tea.KeyEnter:
			return m, func() tea.Msg {
				return CloseModalMsg{}
			}
		}

		switch msg.String() {
		case "tab":
			if m.activeTab == tabTelemetry {
				m.activeTab = tabLeaderboard
			} else {
				m.activeTab = tabTelemetry
			}
			return m, nil
		case "1":
			m.activeTab = tabTelemetry
			return m, nil
		case "2":
			m.activeTab = tabLeaderboard
			return m, nil
		case "esc", "enter", "q", "Q", "h", "H", " ":
			return m, func() tea.Msg {
				return CloseModalMsg{}
			}
		}
	}

	if m.promptName {
		var cmd tea.Cmd
		m.textInput, cmd = m.textInput.Update(msg)
		return m, cmd
	}

	return m, nil
}

// View renders the exact 66x18 bordered modal box.
func (m Model) View() string {
	styles := getStyles(m.theme)
	innerW := m.width - 2
	if innerW < 10 {
		innerW = 64
	}

	formatRow := func(content string) string {
		w := ansi.StringWidth(content)
		pad := innerW - w
		if pad < 0 {
			content = ansi.Truncate(content, innerW, "")
			pad = 0
		}
		return styles.Border.Render("│") + content + strings.Repeat(" ", pad) + styles.Border.Render("│")
	}

	var tab1Title, tab2Title string
	if m.activeTab == tabTelemetry {
		tab1Title = styles.PanelTitle.Render("[1] MISSION DEBRIEF & TELEMETRY")
		tab2Title = styles.LogText.Render("[2] HALL OF FAME")
	} else {
		tab1Title = styles.LogText.Render("[1] MISSION DEBRIEF & TELEMETRY")
		tab2Title = styles.PanelTitle.Render("[2] STARFLEET HALL OF FAME")
	}

	prefixW := 2 + ansi.StringWidth(tab1Title) + 2 + ansi.StringWidth(tab2Title)
	topFill := m.width - prefixW - 1
	if topFill < 0 {
		topFill = 0
	}
	borderTop := styles.Border.Render("┌─") + tab1Title + styles.Border.Render("──") + tab2Title + styles.Border.Render(strings.Repeat("─", topFill)) + styles.Border.Render("┐")

	bottomFill := innerW
	borderBottom := styles.Border.Render("└") + styles.Border.Render(strings.Repeat("─", bottomFill)) + styles.Border.Render("┘")

	var rows []string
	rows = append(rows, borderTop)

	if m.activeTab == tabTelemetry {
		rows = append(rows, m.renderTelemetryRows(styles, innerW, formatRow)...)
	} else {
		rows = append(rows, m.renderLeaderboardRows(styles, innerW, formatRow)...)
	}

	rows = append(rows, borderBottom)
	return strings.Join(rows, "\n")
}

func (m Model) renderTelemetryRows(styles hofStyles, innerW int, formatRow func(string) string) []string {
	leftH := " COMBAT ACHIEVEMENTS"
	rightH := "PENALTIES & LOSSES"
	gapH := innerW - ansi.StringWidth(leftH) - ansi.StringWidth(rightH)
	if gapH < 0 {
		gapH = 0
	}
	row1 := styles.GaugeLabel.Render(leftH) + strings.Repeat(" ", gapH) + styles.GaugeLabel.Render(rightH)

	formatCol := func(lLabel string, lVal, lPts int, rLabel string, rVal, rPts int) string {
		lStr := fmt.Sprintf(" %-21s %2d (+%4d)", lLabel, lVal, lPts)
		rStr := fmt.Sprintf(" %-19s %3d (-%4d)", rLabel, rVal, rPts)
		gap := innerW - ansi.StringWidth(lStr) - ansi.StringWidth(rStr)
		if gap < 0 {
			gap = 0
		}
		return lStr + strings.Repeat(" ", gap) + rStr
	}

	row2 := formatCol("Klingons Destroyed:", m.score.KlingonsKilled, m.score.KlingonPoints, "Casualties:", m.score.Casualties, m.score.CasualtyPenalty)
	row3 := formatCol("Commanders Destroyed:", m.score.CommandersKilled, m.score.CommanderPoints, "Starbases Lost:", m.score.StarbasesLost, m.score.StarbasePenalty)
	row4 := formatCol("Super-Cmdrs Killed:", m.score.SuperCommandersKilled, m.score.SuperCommanderPoints, "Distress Calls:", m.score.HelpCalls, m.score.HelpPenalty)
	row5 := formatCol("Romulans Destroyed:", m.score.RomulansKilled, m.score.RomulanPoints, "Planets Destroyed:", m.score.PlanetsDestroyed, m.score.PlanetPenalty)
	row6 := formatCol("Romulans Surrendered:", m.score.RomulansSurrendered, m.score.SurrenderedPoints, "Stars Destroyed:", m.score.StarsDestroyed, m.score.StarPenalty)

	lRate := fmt.Sprintf(" Kill Rate (%4.2f/SD):      (+%4d)", m.score.KillRate, m.score.KillRatePoints)
	rShips := fmt.Sprintf(" %-19s %3d (-%4d)", "Starships Lost:", m.score.StarshipsLost, m.score.StarshipPenalty)
	gapRate := innerW - ansi.StringWidth(lRate) - ansi.StringWidth(rShips)
	if gapRate < 0 {
		gapRate = 0
	}
	row7 := lRate + strings.Repeat(" ", gapRate) + rShips

	lBonus := fmt.Sprintf(" Mission Victory Bonus:    (+%4d)", m.score.WinBonus)
	rElapsed := fmt.Sprintf(" Stardates Elapsed:       %5.1f", m.score.ElapsedStardates)
	gapBonus := innerW - ansi.StringWidth(lBonus) - ansi.StringWidth(rElapsed)
	if gapBonus < 0 {
		gapBonus = 0
	}
	row8 := lBonus + strings.Repeat(" ", gapBonus) + rElapsed

	divider := styles.Border.Render(" " + strings.Repeat("─", innerW-2))
	row9 := divider

	scorePrefix := " CURRENT NET SCORE:  "
	scoreVal := fmt.Sprintf("%+6d", m.score.TotalScore)
	row10 := scorePrefix + styles.GaugeValue.Render(scoreVal)

	rankPrefix := " STARFLEET RANK:     "
	rankVal := fmt.Sprintf("%-8s %s", m.score.RankBadge, m.score.RankTitle)
	row11 := rankPrefix + styles.PanelTitle.Render(rankVal)

	row12 := fmt.Sprintf(" RATING ASSESSMENT:  %.2f kills/stardate", m.score.KillRate)
	row13 := divider
	row14 := ""
	row15 := " [Tab / 1-2] Switch Tab                 [Esc / Q / H] Dismiss"
	row16 := ""

	return []string{
		formatRow(row1),
		formatRow(row2),
		formatRow(row3),
		formatRow(row4),
		formatRow(row5),
		formatRow(row6),
		formatRow(row7),
		formatRow(row8),
		formatRow(row9),
		formatRow(row10),
		formatRow(row11),
		formatRow(row12),
		formatRow(row13),
		formatRow(row14),
		formatRow(row15),
		formatRow(row16),
	}
}

func (m Model) renderLeaderboardRows(styles hofStyles, innerW int, formatRow func(string) string) []string {
	hStr := fmt.Sprintf(" %2s  %-20s %-9s %6s   %7s  %-10s", "#", "CAPTAIN", "RANK", "SCORE", "STARDATE", "DATE")
	row1 := styles.GaugeLabel.Render(hStr)
	divider := styles.Border.Render(" " + strings.Repeat("─", innerW-2))
	row2 := divider

	var entries []engine.ScoreEntry
	if m.leaderboard != nil {
		entries = m.leaderboard.Entries
	}

	entryRows := make([]string, 10)
	for i := 0; i < 10; i++ {
		if i < len(entries) {
			e := entries[i]
			cName := e.CaptainName
			if ansi.StringWidth(cName) > 20 {
				cName = ansi.Truncate(cName, 20, "…")
			}
			dateStr := e.Date.Format("2006-01-02")
			line := fmt.Sprintf(" %2d. %-20s %-9s %6d   %7.1f  %-10s", i+1, cName, e.Rank, e.Score, e.Stardate, dateStr)
			if i == 0 {
				entryRows[i] = styles.PanelTitle.Render(line)
			} else {
				entryRows[i] = line
			}
		} else {
			entryRows[i] = fmt.Sprintf(" %2d. %-20s %-9s %6s   %7s  %-10s", i+1, "--------------------", "------", "------", "-------", "----------")
		}
	}

	var row13, row14, row15, row16 string
	if m.promptName {
		row13 = " ENTER CALLSIGN: " + m.textInput.View()
		row14 = " [ENTER] Save Score to Hall of Fame     [ESC] Cancel"
		row15 = ""
		row16 = ""
	} else {
		row13 = divider
		row14 = " Starfleet Command honors all officers serving the Federation."
		row15 = " [Tab / 1-2] Switch Tab                 [Esc / Q / H] Dismiss"
		row16 = ""
	}

	result := make([]string, 0, 16)
	result = append(result, formatRow(row1))
	result = append(result, formatRow(row2))
	for _, er := range entryRows {
		result = append(result, formatRow(er))
	}
	result = append(result, formatRow(row13))
	result = append(result, formatRow(row14))
	result = append(result, formatRow(row15))
	result = append(result, formatRow(row16))
	return result
}
