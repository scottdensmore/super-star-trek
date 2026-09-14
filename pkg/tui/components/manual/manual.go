package manual

import (
	"fmt"
	"strings"

	"github.com/charmbracelet/lipgloss"
	"github.com/charmbracelet/x/ansi"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

// FocusPane represents which pane currently receives directional navigation keys.
type FocusPane int

const (
	// FocusChapters indicates directional navigation operates on the chapter list.
	FocusChapters FocusPane = iota
	// FocusContent indicates directional navigation operates on the reader content.
	FocusContent
)

// Model encapsulates the Starfleet Technical Manual component state.
type Model struct {
	width         int
	height        int
	theme         theme.Theme
	chapters      []Chapter
	selectedIdx   int
	scrollOffsets []int
	focus         FocusPane
}

type manualStyles struct {
	theme.Styles
	borderFg lipgloss.Style
}

func getStyles(th theme.Theme) manualStyles {
	if th == nil {
		th = theme.DefaultTheme()
	}
	base := th.Styles()
	borderFg := base.Panel.GetBorderTopForeground()
	bStyle := lipgloss.NewStyle()
	if borderFg != nil {
		bStyle = bStyle.Foreground(borderFg)
	}
	return manualStyles{
		Styles:   base,
		borderFg: bStyle,
	}
}

// New constructs a new Technical Manual Model initialized with the given theme and dimensions.
func New(th theme.Theme, width, height int) Model {
	if th == nil {
		th = theme.DefaultTheme()
	}
	if width <= 0 {
		width = 66
	}
	if height <= 0 {
		height = 18
	}

	chs := defaultChapters()
	return Model{
		width:         width,
		height:        height,
		theme:         th,
		chapters:      chs,
		selectedIdx:   0,
		scrollOffsets: make([]int, len(chs)),
		focus:         FocusChapters,
	}
}

// SetTheme updates the active styling theme.
func (m *Model) SetTheme(th theme.Theme) {
	if th == nil {
		th = theme.DefaultTheme()
	}
	m.theme = th
}

// SelectChapter jumps to the specified chapter by ID, numeric string (1-8), or topic alias.
func (m *Model) SelectChapter(topicOrNum string) {
	s := strings.ToLower(strings.TrimSpace(topicOrNum))
	if s == "" {
		return
	}

	// Direct numeric string "1" - "8"
	if len(s) == 1 && s[0] >= '1' && s[0] <= '8' {
		idx := int(s[0] - '1')
		if idx >= 0 && idx < len(m.chapters) {
			m.selectedIdx = idx
			return
		}
	}

	// Exact chapter ID match
	for i, ch := range m.chapters {
		if strings.EqualFold(ch.ID, s) {
			m.selectedIdx = i
			return
		}
	}

	// Topic and command aliases
	aliases := map[string]int{
		"sys":          0,
		"device":       0,
		"devices":      0,
		"subsystems":   0,
		"computer":     0,
		"srs":          0,
		"lrs":          0,
		"navigation":   1,
		"warp":         1,
		"impulse":      1,
		"course":       1,
		"chart":        1,
		"combat":       2,
		"weapons":      2,
		"tor":          2,
		"torpedo":      2,
		"torpedoes":    2,
		"pha":          2,
		"phaser":       2,
		"phasers":      2,
		"target":       2,
		"aim":          2,
		"shield":       3,
		"shields":      3,
		"she":          3,
		"deflector":    3,
		"dam":          3,
		"damage":       3,
		"repair":       3,
		"starbase":     4,
		"starbases":    4,
		"base":         4,
		"bases":        4,
		"doc":          4,
		"dock":         4,
		"docking":      4,
		"surveillance": 4,
		"tactics":      5,
		"tac":          5,
		"klingon":      5,
		"klingons":     5,
		"threat":       5,
		"threats":      5,
		"cloak":        5,
		"cloaking":     5,
		"romulan":      5,
		"romulans":     5,
		"commander":    5,
		"scoring":      6,
		"score":        6,
		"scores":       6,
		"rank":         6,
		"ranks":        6,
		"rating":       6,
		"points":       6,
		"command":      7,
		"commands":     7,
		"cmd":          7,
		"cmds":         7,
		"keys":         7,
		"hotkeys":      7,
		"cheatsheet":   7,
		"help":         7,
		"guide":        7,
	}

	if idx, ok := aliases[s]; ok && idx >= 0 && idx < len(m.chapters) {
		m.selectedIdx = idx
	}
}

// View renders the exact 66-column x 18-line wireframe layout.
func (m Model) View() string {
	styles := getStyles(m.theme)

	w := m.width
	if w <= 0 {
		w = 66
	}

	const (
		sidebarW = 20
		readerW  = 43
	)

	// Top Border (66 cols): ┌─ [F1] STARFLEET TECHNICAL MANUAL & LIBRARY COMPUTER ───────────┐
	titleStr := "[F1] STARFLEET TECHNICAL MANUAL & LIBRARY COMPUTER"
	styledTitle := styles.PanelTitle.Render(titleStr)
	// Border prefix "┌─ " (3) + titleStr (50) + " " (1) = 54 cols.
	// Remaining: 66 - 54 - 1 ("┐") = 11 dashes.
	dashesTop := w - 3 - ansi.StringWidth(titleStr) - 2
	if dashesTop < 0 {
		dashesTop = 0
	}
	borderTop := "┌─ " + styledTitle + " " + styles.borderFg.Render(strings.Repeat("─", dashesTop)) + "┐"

	// Bottom Border (66 cols): └─ [Tab: Pane]  [↑/↓: Move]  [1-8: Jump]  [Esc/Q/F1: Close] ─────┘
	footerStr := "[Tab: Pane]  [↑/↓: Move]  [1-8: Jump]  [Esc/Q/F1: Close]"
	styledFooter := styles.LogText.Render(footerStr)
	// Border prefix "└─ " (3) + footerStr (56) + " " (1) = 60 cols.
	// Remaining: 66 - 60 - 1 ("┘") = 5 dashes.
	dashesBottom := w - 3 - ansi.StringWidth(footerStr) - 2
	if dashesBottom < 0 {
		dashesBottom = 0
	}
	borderBottom := "└─ " + styledFooter + " " + styles.borderFg.Render(strings.Repeat("─", dashesBottom)) + "┘"

	// Build Sidebar (16 rows)
	sidebarRows := m.renderSidebarRows(styles, sidebarW)

	// Build Reader Pane (16 rows)
	readerRows := m.renderReaderRows(styles, readerW)

	// Assemble 16 inner content rows
	rows := make([]string, 0, 18)
	rows = append(rows, borderTop)

	for i := 0; i < 16; i++ {
		sRow := sidebarRows[i]
		rRow := readerRows[i]
		row := "│" + sRow + "│" + rRow + "│"
		rows = append(rows, row)
	}

	rows = append(rows, borderBottom)
	return strings.Join(rows, "\n")
}

func (m Model) renderSidebarRows(styles manualStyles, width int) []string {
	rows := make([]string, 16)

	// Row 0: CHAPTERS header
	rows[0] = padRight(" "+styles.PanelTitle.Render("CHAPTERS"), width)

	// Row 1: Blank
	rows[1] = padRight("", width)

	// Rows 2..9: The 8 chapters
	for i := 0; i < 8; i++ {
		if i >= len(m.chapters) {
			rows[2+i] = padRight("", width)
			continue
		}
		ch := m.chapters[i]
		if i == m.selectedIdx {
			var label string
			if m.focus == FocusChapters {
				label = "▶ " + styles.PanelTitle.Bold(true).Render(ch.ShortTag)
			} else {
				label = "▶ " + styles.PanelTitle.Render(ch.ShortTag)
			}
			rows[2+i] = padRight(label, width)
		} else {
			label := "  " + styles.LogText.Render(ch.ShortTag)
			rows[2+i] = padRight(label, width)
		}
	}

	// Row 10: Blank
	rows[10] = padRight("", width)

	// Row 11: Separator
	rows[11] = padRight(" "+styles.borderFg.Render(strings.Repeat("─", 18)), width)

	// Row 12: Help text
	rows[12] = padRight(" "+styles.LogText.Render("[Tab] Focus Reader"), width)

	// Row 13: Help text
	rows[13] = padRight(" "+styles.LogText.Render("[1-8] Direct Jump"), width)

	// Rows 14..15: Blank
	rows[14] = padRight("", width)
	rows[15] = padRight("", width)

	return rows
}

func (m Model) renderReaderRows(styles manualStyles, width int) []string {
	rows := make([]string, 16)

	if m.selectedIdx < 0 || m.selectedIdx >= len(m.chapters) {
		for i := range rows {
			rows[i] = padRight("", width)
		}
		return rows
	}

	ch := m.chapters[m.selectedIdx]
	totalLines := len(ch.Lines)
	offset := 0
	if m.selectedIdx < len(m.scrollOffsets) {
		offset = m.scrollOffsets[m.selectedIdx]
	}
	maxOffset := totalLines - 14
	if maxOffset < 0 {
		maxOffset = 0
	}
	if offset > maxOffset {
		offset = maxOffset
	}
	if offset < 0 {
		offset = 0
	}

	// Row 0: Chapter title banner with line counter
	counterStr := fmt.Sprintf("[%02d/%02d] ", offset+1, totalLines)
	counterW := ansi.StringWidth(counterStr)
	maxTitleW := width - counterW - 2
	titleText := fmt.Sprintf("CHAPTER %d: %s", ch.Number, strings.ToUpper(ch.Title))
	if ansi.StringWidth(titleText) > maxTitleW {
		titleText = ansi.Truncate(titleText, maxTitleW, "…")
	}
	styledTitle := styles.PanelTitle.Render(" " + titleText)
	styledCounter := styles.PanelTitle.Render(counterStr)
	gap := width - ansi.StringWidth(styledTitle) - counterW
	if gap < 0 {
		gap = 0
	}
	headerRow := styledTitle + strings.Repeat(" ", gap) + styledCounter
	rows[0] = padRight(headerRow, width)

	// Rows 1..14 (14 content rows)
	for i := 0; i < 14; i++ {
		lineIdx := offset + i
		if lineIdx < totalLines {
			rawLine := ch.Lines[lineIdx]
			var lineStr string
			if ansi.StringWidth(rawLine) < width && rawLine != "" {
				lineStr = " " + rawLine
			} else {
				lineStr = rawLine
			}
			rows[1+i] = padRight(styles.LogText.Render(lineStr), width)
		} else {
			rows[1+i] = padRight("", width)
		}
	}

	// Row 15: Scroll indicator hint
	hintText := "▲ [↑/↓, J/K to scroll 14 lines] ▼"
	hintW := ansi.StringWidth(hintText)
	hintGap := width - hintW
	if hintGap < 0 {
		hintGap = 0
	}
	leftPad := hintGap / 2
	rightPad := hintGap - leftPad
	hintRow := strings.Repeat(" ", leftPad) + styles.LogText.Render(hintText) + strings.Repeat(" ", rightPad)
	rows[15] = padRight(hintRow, width)

	return rows
}

func padRight(s string, targetWidth int) string {
	w := ansi.StringWidth(s)
	if w >= targetWidth {
		return ansi.Truncate(s, targetWidth, "")
	}
	return s + strings.Repeat(" ", targetWidth-w)
}
