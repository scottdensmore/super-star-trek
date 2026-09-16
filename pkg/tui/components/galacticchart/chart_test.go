package galacticchart

import (
	"regexp"
	"strings"
	"testing"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

func TestGalacticChart_CursorNavigationAndBounds(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th, 64, 18)
	var chart [9][9]int
	var discovered [9][9]bool
	var knownBases [9][9]bool
	m.SetState(engine.Coord{3, 3}, chart, discovered, knownBases, false)

	if m.Cursor() != (engine.Coord{3, 3}) {
		t.Fatalf("expected cursor initialized to enterprise quad [3,3], got %v", m.Cursor())
	}

	// Move Up (k)
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'k'}})
	if m.Cursor() != (engine.Coord{2, 3}) {
		t.Fatalf("expected cursor [2,3] after Up, got %v", m.Cursor())
	}

	// Move Left (h)
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'h'}})
	if m.Cursor() != (engine.Coord{2, 2}) {
		t.Fatalf("expected cursor [2,2] after Left, got %v", m.Cursor())
	}

	// Move beyond row 1 (clamp)
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'k'}})
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'k'}})
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'k'}})
	if m.Cursor()[0] != 1 {
		t.Fatalf("expected cursor row clamped to 1, got %d", m.Cursor()[0])
	}

	// Move beyond col 1 (clamp)
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'h'}})
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'h'}})
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'h'}})
	if m.Cursor()[1] != 1 {
		t.Fatalf("expected cursor col clamped to 1, got %d", m.Cursor()[1])
	}

	// Move Down with arrow key
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyDown})
	if m.Cursor() != (engine.Coord{2, 1}) {
		t.Fatalf("expected cursor [2,1] after KeyDown, got %v", m.Cursor())
	}

	// Move Right with arrow key
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRight})
	if m.Cursor() != (engine.Coord{2, 2}) {
		t.Fatalf("expected cursor [2,2] after KeyRight, got %v", m.Cursor())
	}

	// Move Down with 'j' beyond row 8 (clamp)
	for i := 0; i < 10; i++ {
		m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'j'}})
	}
	if m.Cursor()[0] != 8 {
		t.Fatalf("expected cursor row clamped to 8, got %d", m.Cursor()[0])
	}

	// Move Right with 'l' beyond col 8 (clamp)
	for i := 0; i < 10; i++ {
		m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'l'}})
	}
	if m.Cursor()[1] != 8 {
		t.Fatalf("expected cursor col clamped to 8, got %d", m.Cursor()[1])
	}
}

func TestGalacticChart_EnterAndEscMessages(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th, 64, 18)
	var chart [9][9]int
	var discovered [9][9]bool
	var knownBases [9][9]bool
	m.SetState(engine.Coord{3, 3}, chart, discovered, knownBases, false)

	// Move to [2, 3]
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyUp})

	// Enter emits WarpToQuadrantMsg
	m, cmd := m.Update(tea.KeyMsg{Type: tea.KeyEnter})
	if cmd == nil {
		t.Fatalf("expected command on Enter, got nil")
	}
	msg := cmd()
	warpMsg, ok := msg.(WarpToQuadrantMsg)
	if !ok || warpMsg.DestQuad != (engine.Coord{2, 3}) {
		t.Fatalf("expected WarpToQuadrantMsg with [2,3], got %v", msg)
	}
	if warpMsg.Warp != 1.0 {
		t.Fatalf("expected Warp 1.0, got %v", warpMsg.Warp)
	}

	// Esc emits CloseChartMsg
	m, cmd = m.Update(tea.KeyMsg{Type: tea.KeyEsc})
	if cmd == nil {
		t.Fatalf("expected command on Esc, got nil")
	}
	if _, ok := cmd().(CloseChartMsg); !ok {
		t.Fatalf("expected CloseChartMsg on Esc, got %T", cmd())
	}
}

func TestGalacticChart_ViewDimensionsAndLayout(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th, 64, 18)
	var chart [9][9]int
	var discovered [9][9]bool
	chart[3][3] = 3
	discovered[3][3] = true
	chart[2][5] = 105
	discovered[2][5] = true
	var knownBases [9][9]bool
	m.SetState(engine.Coord{3, 3}, chart, discovered, knownBases, false)

	view := m.View()
	if !strings.Contains(view, "GALACTIC STAR CHART") {
		t.Fatalf("expected title in View, got:\n%s", view)
	}
	if !strings.Contains(view, "105") {
		t.Fatalf("expected discovered quad 105 in View, got:\n%s", view)
	}

	lines := strings.Split(strings.TrimRight(view, "\n"), "\n")
	if len(lines) != 18 {
		t.Fatalf("expected height 18 rows, got %d", len(lines))
	}
	for i, line := range lines {
		w := lipgloss.Width(line)
		if w != 64 {
			t.Fatalf("line %d width %d != 64: %q", i, w, line)
		}
	}
}

func TestGalacticChart_TelemetryFormatting(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th, 64, 18)
	var chart [9][9]int
	var discovered [9][9]bool
	var knownBases [9][9]bool
	m.SetState(engine.Coord{3, 3}, chart, discovered, knownBases, false)

	// When cursor is on enterprise quad:
	vCurrent := m.View()
	if !strings.Contains(vCurrent, "Current Position") {
		t.Errorf("expected View to show 'Current Position' when cursor is on enterprise quad, got:\n%s", vCurrent)
	}

	// Move to remote quad [2, 3] (North, negative ΔR)
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyUp})
	vRemote := m.View()
	if !strings.Contains(vRemote, "Target: Quad [2, 3]") {
		t.Errorf("expected Target: Quad [2, 3] in View, got:\n%s", vRemote)
	}
	if !strings.Contains(vRemote, "Dist: 1.0 quads (ΔR: -1, ΔC: 0)") {
		t.Errorf("expected Dist: 1.0 quads (ΔR: -1, ΔC: 0) in View, got:\n%s", vRemote)
	}
	if !strings.Contains(vRemote, "Course:") || !strings.Contains(vRemote, "North") {
		t.Errorf("expected Course with North in View, got:\n%s", vRemote)
	}
	if !strings.Contains(vRemote, "Warp: 1.0") {
		t.Errorf("expected Warp: 1.0 in View, got:\n%s", vRemote)
	}

	// Move to remote quad [5, 6] (South-East, positive ΔR and ΔC)
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyDown})  // back to [3, 3]
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyDown})  // [4, 3]
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyDown})  // [5, 3]
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRight}) // [5, 4]
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRight}) // [5, 5]
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRight}) // [5, 6]
	vSE := m.View()
	if !strings.Contains(vSE, "Target: Quad [5, 6]") {
		t.Errorf("expected Target: Quad [5, 6] in View, got:\n%s", vSE)
	}
	if !strings.Contains(vSE, "ΔR: +2, ΔC: +3") {
		t.Errorf("expected positive deltas ΔR: +2, ΔC: +3 in View, got:\n%s", vSE)
	}
	if !strings.Contains(vSE, "South-East") {
		t.Errorf("expected South-East in View, got:\n%s", vSE)
	}
}

func TestGalacticChart_ThemeAndLifecycle(t *testing.T) {
	m := New(nil, 0, 0)
	if m.Theme() == nil {
		t.Fatal("expected non-nil default theme")
	}
	if cmd := m.Init(); cmd != nil {
		t.Errorf("expected nil from Init, got %v", cmd)
	}

	lcars := theme.GetTheme("lcars")
	m.SetTheme(lcars)
	if m.Theme().Name() != "lcars" {
		t.Fatalf("expected theme lcars, got %s", m.Theme().Name())
	}

	crt := theme.GetTheme("crt")
	m.SetTheme(crt)
	viewCRT := m.View()
	if !strings.Contains(viewCRT, "GALACTIC STAR CHART") {
		t.Fatalf("expected view with CRT theme to render title")
	}

	m.SetTheme(nil)
	if m.Theme() == nil {
		t.Fatal("expected fallback default theme on nil")
	}

	m.SetSize(80, 24)
	// Safely ignore WindowSizeMsg
	m, cmd := m.Update(tea.WindowSizeMsg{Width: 100, Height: 50})
	if cmd != nil {
		t.Errorf("expected nil cmd on WindowSizeMsg, got %v", cmd)
	}

	// Unknown non-key message safely returns nil cmd
	type customMsg struct{}
	m, cmd = m.Update(customMsg{})
	if cmd != nil {
		t.Errorf("expected nil cmd on custom message, got %v", cmd)
	}
}

var ansiRegex = regexp.MustCompile(`\x1b\[[0-9;]*[a-zA-Z]`)

func stripAnsi(s string) string {
	return ansiRegex.ReplaceAllString(s, "")
}

func TestGalacticChart_ColumnHeaderAlignment(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th, 64, 18)
	var chart [9][9]int
	var discovered [9][9]bool
	var knownBases [9][9]bool
	for c := 1; c <= 8; c++ {
		chart[1][c] = c * 10
		discovered[1][c] = true
	}
	m.SetState(engine.Coord{1, 1}, chart, discovered, knownBases, false)

	view := m.View()
	lines := strings.Split(strings.TrimRight(view, "\n"), "\n")

	var headerLine string
	var headerLineIdx int
	for idx, line := range lines {
		stripped := stripAnsi(line)
		if strings.Contains(stripped, "1") && strings.Contains(stripped, "8") && strings.Contains(stripped, "2") && strings.Contains(stripped, "3") {
			headerLine = stripped
			headerLineIdx = idx
			break
		}
	}
	if headerLine == "" {
		t.Fatalf("could not find column header line in View:\n%s", view)
	}

	row1Line := stripAnsi(lines[headerLineIdx+1])
	headerRunes := []rune(headerLine)
	rowRunes := []rune(row1Line)

	for col := 1; col <= 8; col++ {
		digitRune := rune('0' + col)
		headerColIdx := -1
		for rIdx, r := range headerRunes {
			if r == digitRune {
				headerColIdx = rIdx
				break
			}
		}
		if headerColIdx == -1 {
			t.Fatalf("digit %d not found in header line: %q", col, headerLine)
		}

		expectedCenterRune := digitRune
		if headerColIdx >= len(rowRunes) {
			t.Fatalf("headerColIdx %d exceeds row line length %d", headerColIdx, len(rowRunes))
		}
		if rowRunes[headerColIdx] != expectedCenterRune {
			t.Errorf("column %d digit %c at index %d does not align with cell center in row 1 (found %c, expected %c):\nheader: %s\nrow 1:  %s",
				col, digitRune, headerColIdx, rowRunes[headerColIdx], expectedCenterRune, headerLine, row1Line)
		}
	}
}

func TestGalacticChart_SetStateClampsEnterpriseQuad(t *testing.T) {
	m := New(nil, 64, 18)
	var chart [9][9]int
	var discovered [9][9]bool
	var knownBases [9][9]bool
	m.SetState(engine.Coord{0, 10}, chart, discovered, knownBases, false)
	if m.enterpriseQuad != (engine.Coord{1, 8}) {
		t.Errorf("expected enterpriseQuad clamped to [1, 8], got %v", m.enterpriseQuad)
	}
	if m.Cursor() != (engine.Coord{1, 8}) {
		t.Errorf("expected cursor clamped to [1, 8], got %v", m.Cursor())
	}
}

func TestGalacticChart_KnownBaseRendering(t *testing.T) {
	th := theme.ModernTheme{}
	m := New(th, 64, 18)
	var chart [9][9]int
	var disc [9][9]bool
	var knownBases [9][9]bool

	chart[2][3] = 15 // base at [2,3]
	knownBases[2][3] = true
	// Not yet discovered!

	m.SetState(engine.Coord{4, 4}, chart, disc, knownBases, false)
	view := m.View()

	if !strings.Contains(view, ".1.") {
		t.Errorf("expected view to contain '.1.' for known base quadrant, got:\n%s", view)
	}
}

func TestGalacticChart_ComputerDamagedTelemetryAndLockout(t *testing.T) {
	th := theme.ModernTheme{}
	m := New(th, 64, 18)
	var chart [9][9]int
	var disc [9][9]bool
	var knownBases [9][9]bool

	m.SetState(engine.Coord{4, 4}, chart, disc, knownBases, true) // Computer damaged!
	view := m.View()

	if !strings.Contains(view, "[CALC OFFLINE]") {
		t.Errorf("expected '[CALC OFFLINE]' in footer, got:\n%s", view)
	}
	if !strings.Contains(view, "Target: ") {
		t.Errorf("expected 'Target: ' in footer, got:\n%s", view)
	}
	if !strings.Contains(view, "Quad [") {
		t.Errorf("expected 'Quad [' in footer, got:\n%s", view)
	}
	if !strings.Contains(view, "Disabled (Comp Offline)") {
		t.Errorf("expected action hint to indicate Enter disabled, got:\n%s", view)
	}

	// Test Enter lockout
	m.cursor = engine.Coord{6, 6}
	_, cmd := m.Update(tea.KeyMsg{Type: tea.KeyEnter})
	if cmd == nil {
		t.Fatalf("expected command emitting WarpBlockedMsg, got nil")
	}
	msg := cmd()
	blocked, ok := msg.(WarpBlockedMsg)
	if !ok {
		t.Fatalf("expected WarpBlockedMsg, got %T", msg)
	}
	if !strings.Contains(blocked.Reason, "COMPUTER DAMAGED") {
		t.Errorf("expected reason to contain 'COMPUTER DAMAGED', got: %s", blocked.Reason)
	}
}

func TestGalacticChart_ComputerDamagedDimensions(t *testing.T) {
	th := theme.ModernTheme{}
	m := New(th, 64, 18)
	var chart [9][9]int
	var disc [9][9]bool
	var knownBases [9][9]bool

	m.SetState(engine.Coord{4, 4}, chart, disc, knownBases, true)
	view := m.View()

	lines := strings.Split(strings.TrimRight(view, "\n"), "\n")
	if len(lines) != 18 {
		t.Fatalf("expected height 18 rows when computer damaged, got %d", len(lines))
	}
	for i, line := range lines {
		w := lipgloss.Width(line)
		if w != 64 {
			t.Fatalf("line %d width %d != 64 when computer damaged: %q", i, w, line)
		}
	}
}

func TestGalacticChart_LegendRendering(t *testing.T) {
	th := theme.ModernTheme{}
	m := New(th, 64, 18)
	var chart [9][9]int
	var disc [9][9]bool
	var knownBases [9][9]bool

	m.SetState(engine.Coord{3, 3}, chart, disc, knownBases, false)
	view := m.View()

	if !strings.Contains(view, "Legend:") || !strings.Contains(view, "··· Unexplored") {
		t.Errorf("expected view to contain legend with '··· Unexplored', got:\n%s", view)
	}
	if !strings.Contains(view, ".1. Base") {
		t.Errorf("expected view to contain '.1. Base', got:\n%s", view)
	}
	if !strings.Contains(view, "KBS (Klingon/Base/Star)") {
		t.Errorf("expected view to contain 'KBS (Klingon/Base/Star)', got:\n%s", view)
	}
	if !strings.Contains(view, "[?] Help") {
		t.Errorf("expected view to contain '[?] Help', got:\n%s", view)
	}

	lines := strings.Split(strings.TrimRight(view, "\n"), "\n")
	if len(lines) != 18 {
		t.Fatalf("expected height 18 rows, got %d", len(lines))
	}
	for i, line := range lines {
		w := lipgloss.Width(line)
		if w != 64 {
			t.Fatalf("line %d width %d != 64: %q", i, w, line)
		}
	}
}

func TestGalacticChart_HelpToggle(t *testing.T) {
	th := theme.ModernTheme{}
	m := New(th, 64, 18)
	var chart [9][9]int
	var disc [9][9]bool
	var knownBases [9][9]bool

	m.SetState(engine.Coord{3, 3}, chart, disc, knownBases, false)
	if m.ShowingHelp() {
		t.Fatal("expected ShowingHelp() to be false initially")
	}

	// Press '?' to toggle help on
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'?'}})
	if !m.ShowingHelp() {
		t.Fatal("expected ShowingHelp() to be true after pressing '?'")
	}

	helpView := m.View()
	if !strings.Contains(helpView, "GALACTIC STAR CHART GUIDE") {
		t.Errorf("expected help view to contain title 'GALACTIC STAR CHART GUIDE', got:\n%s", helpView)
	}
	if !strings.Contains(helpView, "Unexplored quadrant") {
		t.Errorf("expected help view to explain unexplored quadrant, got:\n%s", helpView)
	}
	if !strings.Contains(helpView, "K (hundreds) = Klingon battlecruisers") {
		t.Errorf("expected help view to explain Klingon hundreds digit, got:\n%s", helpView)
	}
	if !strings.Contains(helpView, "B (tens)     = Federation starbases") {
		t.Errorf("expected help view to explain Starbases tens digit, got:\n%s", helpView)
	}
	if !strings.Contains(helpView, "S (units)    = Stars") {
		t.Errorf("expected help view to explain Stars units digit, got:\n%s", helpView)
	}

	lines := strings.Split(strings.TrimRight(helpView, "\n"), "\n")
	if len(lines) != 18 {
		t.Fatalf("expected help view height 18 rows, got %d", len(lines))
	}
	for i, line := range lines {
		w := lipgloss.Width(line)
		if w != 64 {
			t.Fatalf("help line %d width %d != 64: %q", i, w, line)
		}
	}

	// Press '?' again to toggle help off
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'?'}})
	if m.ShowingHelp() {
		t.Fatal("expected ShowingHelp() to be false after pressing '?' again")
	}

	// Toggle on and press Esc to toggle off
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'?'}})
	if !m.ShowingHelp() {
		t.Fatal("expected ShowingHelp() true")
	}
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyEsc})
	if m.ShowingHelp() {
		t.Fatal("expected ShowingHelp() false after Esc")
	}

	// SetShowingHelp programmatic control
	m.SetShowingHelp(true)
	if !m.ShowingHelp() {
		t.Fatal("expected ShowingHelp() true after SetShowingHelp(true)")
	}
}
