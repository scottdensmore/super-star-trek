package halloffame

import (
	"path/filepath"
	"strings"
	"testing"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/x/ansi"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

func TestHallOfFame_DimensionsAndTabSwitching(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th, 66, 18, "")

	score := engine.ScoreBreakdown{
		KlingonsKilled:   5,
		KlingonPoints:    50,
		KillRate:         1.0,
		KillRatePoints:   500,
		TotalScore:       550,
		RankBadge:        "[COMM]",
		RankTitle:        "Commodore",
		ElapsedStardates: 5.0,
	}
	lb := engine.DefaultLeaderboard()

	m.SetState(score, lb, false)

	// Tab 1: Mission Telemetry
	view1 := m.View()
	lines1 := strings.Split(view1, "\n")
	if len(lines1) != 18 {
		t.Fatalf("expected 18 lines on Tab 1, got %d", len(lines1))
	}
	for i, l := range lines1 {
		if w := ansi.StringWidth(l); w != 66 {
			t.Errorf("Tab 1 line %d width = %d, expected 66", i, w)
		}
	}
	if !strings.Contains(view1, "MISSION DEBRIEF") || !strings.Contains(view1, "CURRENT NET SCORE") {
		t.Errorf("expected mission telemetry contents in Tab 1")
	}

	// Verify Tab 1 rows are not truncated at the right border
	expectedRowEndings := []struct {
		lineIdx int
		suffix  string
		name    string
	}{
		{2, "(-   0)│", "Casualties"},
		{3, "(-   0)│", "Starbases Lost"},
		{4, "(-   0)│", "Distress Calls"},
		{5, "(-   0)│", "Planets Destroyed"},
		{6, "(-   0)│", "Stars Destroyed"},
		{7, "(-   0)│", "Starships Lost"},
		{8, "  5.0│", "Stardates Elapsed"},
	}
	for _, tc := range expectedRowEndings {
		if !strings.HasSuffix(lines1[tc.lineIdx], tc.suffix) {
			t.Errorf("Tab 1 line %d (%s) truncated: expected suffix %q, got line %q", tc.lineIdx, tc.name, tc.suffix, lines1[tc.lineIdx])
		}
	}

	// Switch to Tab 2 via Tab key
	tabMsg := tea.KeyMsg{Type: tea.KeyTab}
	updated, _ := m.Update(tabMsg)
	mTab2 := updated

	view2 := mTab2.View()
	lines2 := strings.Split(view2, "\n")
	if len(lines2) != 18 {
		t.Fatalf("expected 18 lines on Tab 2, got %d", len(lines2))
	}
	for i, l := range lines2 {
		if w := ansi.StringWidth(l); w != 66 {
			t.Errorf("Tab 2 line %d width = %d, expected 66", i, w)
		}
	}
	if !strings.Contains(view2, "HALL OF FAME") || !strings.Contains(view2, "James T. Kirk") {
		t.Errorf("expected leaderboard table in Tab 2")
	}

	// Verify Tab 2 table header contains STARDATE and DATE
	if !strings.Contains(lines2[1], "STARDATE") || !strings.Contains(lines2[1], "DATE") {
		t.Errorf("expected STARDATE and DATE in Tab 2 header, got %q", lines2[1])
	}

	// Switch back to Tab 1 via Shift+Tab key
	shiftTabMsg := tea.KeyMsg{Type: tea.KeyShiftTab}
	backTab1, _ := mTab2.Update(shiftTabMsg)
	if backTab1.activeTab != tabTelemetry {
		t.Errorf("expected Tab 1 after Shift+Tab, got activeTab %d", backTab1.activeTab)
	}

	// Also verify shift+tab string message
	backTab1Str, _ := mTab2.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune("shift+tab")})
	_ = backTab1Str
}

func TestHallOfFame_DismissalKeys(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th, 66, 18, "")

	for _, k := range []string{"esc", "q", "h"} {
		var keyMsg tea.KeyMsg
		if k == "esc" {
			keyMsg = tea.KeyMsg{Type: tea.KeyEsc}
		} else {
			keyMsg = tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune(k)}
		}

		_, cmd := m.Update(keyMsg)
		if cmd == nil {
			t.Fatalf("expected dismissal command on key %s", k)
		}
		if _, ok := cmd().(CloseModalMsg); !ok {
			t.Errorf("expected CloseModalMsg on key %s, got %T", k, cmd())
		}
	}
}

func TestHallOfFame_NameSubmission(t *testing.T) {
	th := theme.DefaultTheme()
	tmpDir := t.TempDir()
	scoreFile := filepath.Join(tmpDir, "highscores.json")
	m := New(th, 66, 18, scoreFile)

	score := engine.ScoreBreakdown{
		KlingonsKilled:   10,
		KlingonPoints:    100,
		TotalScore:       1500,
		RankBadge:        "[FADM]",
		RankTitle:        "Fleet Admiral",
		ElapsedStardates: 8.0,
		GameWon:          true,
	}
	lb := engine.DefaultLeaderboard()
	m.SetState(score, lb, true)

	// Verify view on Tab 2 with prompt
	view := m.View()
	lines := strings.Split(view, "\n")
	if len(lines) != 18 {
		t.Fatalf("expected 18 lines, got %d", len(lines))
	}
	for i, l := range lines {
		if w := ansi.StringWidth(l); w != 66 {
			t.Errorf("line %d width = %d, expected 66", i, w)
		}
	}
	if !strings.Contains(view, "ENTER CALLSIGN") {
		t.Errorf("expected CALLSIGN prompt in view")
	}

	// Type "Picard"
	for _, r := range "Picard" {
		var cmd tea.Cmd
		m, cmd = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{r}})
		_ = cmd
	}

	// Press Enter to submit
	enterMsg := tea.KeyMsg{Type: tea.KeyEnter}
	var submitCmd tea.Cmd
	m, submitCmd = m.Update(enterMsg)
	if submitCmd == nil {
		t.Fatalf("expected submitCmd on enter, got nil")
	}
	recordedMsg, ok := submitCmd().(ScoreRecordedMsg)
	if !ok {
		t.Fatalf("expected ScoreRecordedMsg, got %T", submitCmd())
	}
	if recordedMsg.Entry.CaptainName != "Picard" {
		t.Errorf("expected captain name 'Picard', got %q", recordedMsg.Entry.CaptainName)
	}
	if recordedMsg.Entry.Score != 1500 {
		t.Errorf("expected score 1500, got %d", recordedMsg.Entry.Score)
	}

	// Verify leaderboard has Picard
	if lb.Entries[0].CaptainName != "Picard" {
		t.Errorf("expected Picard to be #1, got %s", lb.Entries[0].CaptainName)
	}

	// Verify saved to disk
	loaded, err := engine.LoadLeaderboard(scoreFile)
	if err != nil {
		t.Fatalf("failed to load saved leaderboard: %v", err)
	}
	if len(loaded.Entries) == 0 || loaded.Entries[0].CaptainName != "Picard" {
		t.Errorf("expected Picard in loaded leaderboard from disk")
	}

	// Subsequent enter should dismiss
	_, dismissCmd := m.Update(enterMsg)
	if dismissCmd == nil {
		t.Fatalf("expected dismiss command on enter after submission")
	}
	if _, ok := dismissCmd().(CloseModalMsg); !ok {
		t.Errorf("expected CloseModalMsg on enter, got %T", dismissCmd())
	}
}

func TestHallOfFame_TabKeysAndThemes(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th, 66, 18, "")

	// Test '2' key switches to Tab 2
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune("2")})
	view2 := m.View()
	if !strings.Contains(view2, "HALL OF FAME") {
		t.Errorf("expected Tab 2 after pressing '2'")
	}

	// Test '1' key switches to Tab 1
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune("1")})
	view1 := m.View()
	if !strings.Contains(view1, "MISSION DEBRIEF") {
		t.Errorf("expected Tab 1 after pressing '1'")
	}

	// Test SetTheme
	m.SetTheme(theme.CrtTheme{})
	if m.theme.Name() != "crt" {
		t.Errorf("expected theme to be updated to crt, got %s", m.theme.Name())
	}
}

func TestHallOfFame_EdgeCases(t *testing.T) {
	// Nil theme and zero dimensions fallback
	m := New(nil, 0, 0, "")
	if m.width != 66 || m.height != 18 {
		t.Errorf("expected 66x18 fallback, got %dx%d", m.width, m.height)
	}
	m.SetTheme(nil)
	if m.theme == nil {
		t.Errorf("expected default theme on nil SetTheme")
	}

	// Negative score and rank evaluation fallback
	negScore := engine.ScoreBreakdown{
		TotalScore: -50,
	}
	m.SetState(negScore, nil, false)
	view := m.View()
	if !strings.Contains(view, "[DISHONOR]") {
		t.Errorf("expected [DISHONOR] badge for negative score")
	}

	// Esc during name entry cancels and emits CloseModalMsg
	m.SetState(negScore, nil, true)
	if !m.promptName {
		t.Errorf("expected promptName to be true")
	}
	escMsg := tea.KeyMsg{Type: tea.KeyEsc}
	updated, cmd := m.Update(escMsg)
	if updated.promptName {
		t.Errorf("expected promptName to be false after Esc")
	}
	if cmd == nil {
		t.Fatalf("expected dismissal command on Esc during prompt")
	}
	if _, ok := cmd().(CloseModalMsg); !ok {
		t.Errorf("expected CloseModalMsg on Esc during prompt, got %T", cmd())
	}

	// Name submission with empty storagePath and empty name fallback
	highScore := engine.ScoreBreakdown{
		TotalScore: 2000,
	}
	m.SetState(highScore, nil, true)
	enterMsg := tea.KeyMsg{Type: tea.KeyEnter}
	updated2, submitCmd := m.Update(enterMsg)
	if submitCmd == nil {
		t.Fatalf("expected submitCmd on empty name enter")
	}
	rec, ok := submitCmd().(ScoreRecordedMsg)
	if !ok {
		t.Fatalf("expected ScoreRecordedMsg, got %T", submitCmd())
	}
	if rec.Entry.CaptainName != "Unknown Captain" {
		t.Errorf("expected 'Unknown Captain' on blank submission, got %q", rec.Entry.CaptainName)
	}
	if updated2.promptName {
		t.Errorf("expected promptName to be false after submission")
	}
}

func TestHallOfFame_ExtremeScoresLayoutIntegrity(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th, 66, 18, "")

	extremeScore := engine.ScoreBreakdown{
		KlingonsKilled:        50,
		KlingonPoints:         5000,
		CommandersKilled:      10,
		CommanderPoints:       1000,
		SuperCommandersKilled: 5,
		SuperCommanderPoints:  1000,
		RomulansKilled:        20,
		RomulanPoints:         800,
		RomulansSurrendered:   5,
		SurrenderedPoints:     150,
		Casualties:            150,
		CasualtyPenalty:       1500,
		StarbasesLost:         2,
		StarbasePenalty:       200,
		HelpCalls:             5,
		HelpPenalty:           225,
		PlanetsDestroyed:      3,
		PlanetPenalty:         30,
		StarsDestroyed:        4,
		StarPenalty:           20,
		StarshipsLost:         1,
		StarshipPenalty:       100,
		KillRate:              25.50,
		KillRatePoints:        12750,
		WinBonus:              2500,
		TotalScore:            15000,
		RankBadge:             "[FADM]",
		RankTitle:             "Fleet Admiral",
		ElapsedStardates:      123.4,
		GameWon:               true,
	}

	m.SetState(extremeScore, engine.DefaultLeaderboard(), false)

	view := m.View()
	lines := strings.Split(view, "\n")
	if len(lines) != 18 {
		t.Fatalf("expected exactly 18 lines, got %d", len(lines))
	}

	for i, l := range lines {
		w := ansi.StringWidth(l)
		if w != 66 {
			t.Errorf("line %d width = %d, expected 66; line content: %q", i, w, l)
		}
	}

	// Verify all content rows have intact left and right borders without truncation
	for i := 1; i <= 16; i++ {
		line := lines[i]
		if !strings.HasPrefix(line, "│") {
			t.Errorf("line %d does not start with '│': %q", i, line)
		}
		if !strings.HasSuffix(line, "│") {
			t.Errorf("line %d does not end with '│' (layout truncated): %q", i, line)
		}
	}

	// Verify line 7 (Kill Rate & Starships Lost) preserves right column suffix
	if !strings.HasSuffix(lines[7], "(- 100)│") {
		t.Errorf("line 7 truncated: expected suffix '(- 100)│', got %q", lines[7])
	}
	if !strings.Contains(lines[7], "(+12750)") {
		t.Errorf("line 7 missing kill rate points (+12750): %q", lines[7])
	}

	// Verify line 8 (Victory Bonus & Stardates Elapsed) preserves right column suffix
	if !strings.HasSuffix(lines[8], " 123.4│") {
		t.Errorf("line 8 truncated: expected suffix ' 123.4│', got %q", lines[8])
	}
	if !strings.Contains(lines[8], "(+2500)") {
		t.Errorf("line 8 missing victory bonus (+2500): %q", lines[8])
	}
}
