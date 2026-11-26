package main

import (
	"fmt"
	"strconv"
	"strings"
	"time"

	"github.com/charmbracelet/bubbles/list"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/muesli/termenv"
)

type paletteItem struct {
	title, desc string
}

func (p paletteItem) Title() string       { return p.title }
func (p paletteItem) Description() string { return p.desc }
func (p paletteItem) FilterValue() string { return p.title + " " + p.desc }

// The command palette catalog of pkg/tui/components/commandpalette.
var paletteCatalog = []paletteItem{
	{"TOR", "Fire photon torpedo along bearing or sector (tor <course|r c>)"},
	{"PHA", "Fire ship phaser bank with energy (pha <energy>)"},
	{"SHE", "Transfer energy between warp and shields (she <amount>)"},
	{"TARGET", "Open Tactical Target Lock HUD"},
	{"NAV", "Maneuver: nav q <r c> (quad), nav s <r c> (sector), or nav <course warp>"},
	{"DOC", "Dock with adjacent Starbase for fuel & repair"},
	{"SRSCAN", "Refresh short-range sensor tactical grid"},
	{"LRSCAN", "Scan adjacent 3x3 quadrant cluster"},
	{"STATUS", "Review ship status, condition, and stardate"},
	{"DAM", "Review subsystem device damage repair times"},
	{"CHART", "Interactive 8x8 galactic star chart and warp planner (Ctrl+G)"},
	{"SAVES", "Browse, inspect, and load saved missions (Ctrl+O)"},
	{"SCENARIOS", "Browse and launch tactical challenge scenarios"},
	{"THEME: Modern", "Switch theme to Starfleet Modern (Electric Cyan/Navy)"},
	{"THEME: LCARS", "Switch theme to 24th Century LCARS (Gold/Purple)"},
	{"THEME: CRT", "Switch theme to Phosphor Monochrome CRT (Green/Amber)"},
	{"THEME MODE AUTO", "Set theme adaptation to automatic terminal background detection"},
	{"THEME MODE DARK", "Force dark theme palette"},
	{"THEME MODE LIGHT", "Force light theme palette"},
	{"HELP", "Display tactical command reference summary"},
	{"HELP NAV", "Navigation reference: quadrant warping, course angles, and impulse"},
	{"QUIT", "Abandon mission and exit to shell"},
}

var extraItems = []paletteItem{
	{"Ünïcödé títle with a very long name that must be truncated", "desc line one\nsecond line hidden"},
	{"日本語のタイトル", "説明文 ★ wide"},
	{"⚠️ Alert", "🔊 sound on"},
	{"short", ""},
	{"camelCaseItem", "under_score-sep.dot/slash"},
}

func listItems(set string) []list.Item {
	var src []paletteItem
	switch set {
	case "palette":
		src = paletteCatalog
	case "mixed":
		src = append(append([]paletteItem{}, paletteCatalog[:6]...), extraItems...)
	case "few":
		src = paletteCatalog[:3]
	case "empty":
	}
	out := make([]list.Item, len(src))
	for i, it := range src {
		out[i] = it
	}
	return out
}

// newPaletteList reproduces commandpalette.New/updateStyles/SetSize with
// the Modern (dark) theme styles.
func newPaletteList(items []list.Item, w, h int) list.Model {
	d := list.NewDefaultDelegate()
	d.ShowDescription = true
	d.SetSpacing(0)
	l := list.New(items, d, w, h)
	l.Title = "COMMAND PALETTE"
	l.SetShowTitle(true)
	l.SetShowStatusBar(false)
	l.SetShowPagination(false)
	l.SetShowHelp(false)
	l.DisableQuitKeybindings()

	commandText := lipgloss.NewStyle().Foreground(lipgloss.Color("#E2E8F0"))
	gaugeLabel := lipgloss.NewStyle().Foreground(lipgloss.Color("#E2E8F0"))
	prompt := lipgloss.NewStyle().Bold(true).Foreground(lipgloss.Color("#00E5FF"))
	panelTitle := lipgloss.NewStyle().Bold(true).Foreground(lipgloss.Color("#00E5FF"))
	condRed := lipgloss.NewStyle().Bold(true).Foreground(lipgloss.Color("#FF3344"))

	d2 := list.NewDefaultDelegate()
	d2.ShowDescription = true
	d2.SetSpacing(0)
	d2.Styles.NormalTitle = commandText.Padding(0, 0, 0, 2)
	d2.Styles.NormalDesc = gaugeLabel.Padding(0, 0, 0, 2)
	accent := prompt.GetForeground()
	d2.Styles.SelectedTitle = prompt.Border(lipgloss.NormalBorder(), false, false, false, true).BorderForeground(accent).Padding(0, 0, 0, 1)
	d2.Styles.SelectedDesc = gaugeLabel.Border(lipgloss.NormalBorder(), false, false, false, true).BorderForeground(accent).Padding(0, 0, 0, 1)
	d2.Styles.FilterMatch = lipgloss.NewStyle().Underline(true)
	l.SetDelegate(d2)
	l.Styles.Title = panelTitle.Padding(0, 1)
	l.Styles.NoItems = condRed.Padding(0, 0, 0, 2)
	l.SetSize(w, h)
	return l
}

func newDefaultList(items []list.Item, w, h int, variant string) list.Model {
	d := list.NewDefaultDelegate()
	switch variant {
	case "nodesc":
		d.ShowDescription = false
	case "spacing0":
		d.SetSpacing(0)
	case "height3":
		d.SetHeight(3)
	}
	l := list.New(items, d, w, h)
	l.FilterInput.Cursor.BlinkSpeed = time.Millisecond
	if variant == "infinite" {
		l.InfiniteScrolling = true
	}
	if variant == "noquit" {
		l.DisableQuitKeybindings()
	}
	if variant == "nofilter" {
		l.SetFilteringEnabled(false)
	}
	if variant == "names" {
		l.SetStatusBarItemName("command", "commands")
	}
	return l
}

func listState(l list.Model) string {
	var titles []string
	for _, it := range l.VisibleItems() {
		titles = append(titles, it.(paletteItem).title)
	}
	sel := "nil"
	if s := l.SelectedItem(); s != nil {
		sel = s.(paletteItem).title
	}
	return fmt.Sprintf("index=%d global=%d cursor=%d page=%d/%d per=%d state=%s filter=%q sel=%q visible=%q",
		l.Index(), l.GlobalIndex(), l.Cursor(), l.Paginator.Page, l.Paginator.TotalPages, l.Paginator.PerPage,
		l.FilterState().String(), l.FilterValue(), sel, strings.Join(titles, "|"))
}

// applyPaletteQuery mirrors commandpalette.applyFilter.
func applyPaletteQuery(l *list.Model, items []list.Item, query string) {
	if query == "" {
		l.Title = "COMMAND PALETTE"
		l.SetItems(items)
		l.FilterInput.Reset()
		l.Select(0)
		return
	}
	l.Title = "COMMAND PALETTE: " + query
	l.FilterInput.SetValue(query)
	targets := make([]string, len(items))
	for i, it := range items {
		targets[i] = it.FilterValue()
	}
	ranks := list.DefaultFilter(query, targets)
	filtered := make([]list.Item, len(ranks))
	for i, r := range ranks {
		filtered[i] = items[r.Index]
	}
	l.SetItems(filtered)
	l.Select(0)
}

func listKey(step string) tea.Msg {
	kind, arg, _ := strings.Cut(step, ":")
	switch kind {
	case "r":
		return tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune(arg)}
	case "k":
		t, _ := strconv.Atoi(arg)
		return tea.KeyMsg{Type: tea.KeyType(t)}
	case "sp":
		return tea.KeyMsg{Type: tea.KeySpace, Runes: []rune{' '}}
	case "ws":
		return tea.WindowSizeMsg{Width: 80, Height: 24}
	}
	panic("bad step " + step)
}

func genList(path string) {
	w := newRecWriter(path)
	defer w.close()

	// DefaultFilter ranks (sahilm/fuzzy + double sort.Stable).
	var targets []string
	for _, it := range append(append([]paletteItem{}, paletteCatalog...), extraItems...) {
		targets = append(targets, it.FilterValue())
	}
	queries := []string{"t", "th", "theme", "nav", "n", "e", "scan", "ch", "hp", "THEME m", "x", "zzz", "a", "s", "mode", "dock", "st", "re", "help n",
		"日", "ü", "Ü", "cc", "cci", "us", "/", " ", "r c", "ctrl+", "q"}
	for _, q := range queries {
		var sb strings.Builder
		for _, r := range list.DefaultFilter(q, targets) {
			sb.WriteString(fmt.Sprintf("%d:%v;", r.Index, r.MatchedIndexes))
		}
		w.rec("op", "filter", "q", q, "out", sb.String())
		var sb2 strings.Builder
		for _, r := range list.UnsortedFilter(q, targets) {
			sb2.WriteString(fmt.Sprintf("%d:%v;", r.Index, r.MatchedIndexes))
		}
		w.rec("op", "unsorted", "q", q, "out", sb2.String())
	}

	K := func(t tea.KeyType) string { return "k:" + strconv.Itoa(int(t)) }
	runs := []struct {
		p    termenv.Profile
		dark bool
		tag  string
	}{{termenv.Ascii, true, "a"}, {termenv.TrueColor, true, "t"}, {termenv.ANSI256, false, "l"}}

	// Palette scenarios: queries and navigation like commandpalette.Update.
	paletteScenarios := []string{
		"down|down|down|up|end|home|pgdown|pgdown|pgup",
		"q:t|q:th|q:the|q:them|q:theme|down|q:theme m|q:theme mo|q:|down",
		"q:nav|down|q:zzz|q:z|q:|end",
		"sel:5|sel:12|sel:21|sel:0|left|right|right|key:" + K(tea.KeyTab) + "|key:" + K(tea.KeyLeft),
		"q:s|down|down|down|down|down|down|down|down|down|pgdown|pgup",
		"q:help n|q:dock|q:ch",
	}
	paletteSizes := [][2]int{{54, 14}, {30, 6}, {10, 3}}
	for _, run := range runs {
		for szi, sz := range paletteSizes {
			for _, set := range []string{"palette", "mixed", "empty"} {
				for sci, sc := range paletteScenarios {
					if run.tag != "a" && szi != 0 {
						continue
					}
					if set == "empty" && sci > 1 {
						continue
					}
					lipgloss.SetColorProfile(run.p)
					lipgloss.SetHasDarkBackground(run.dark)
					items := listItems(set)
					l := newPaletteList(items, sz[0], sz[1])
					kv := []string{"op", "palette", "run", run.tag, "set", set, "w", strconv.Itoa(sz[0]), "h", strconv.Itoa(sz[1]), "steps", sc,
						"v_init", l.View(), "s_init", listState(l)}
					for i, step := range strings.Split(sc, "|") {
						kind, arg, _ := strings.Cut(step, ":")
						switch kind {
						case "down":
							l.CursorDown()
						case "up":
							l.CursorUp()
						case "pgdown":
							l.NextPage()
						case "pgup":
							l.PrevPage()
						case "home":
							l.Select(0)
						case "end":
							if n := len(l.VisibleItems()); n > 0 {
								l.Select(n - 1)
							}
						case "sel":
							// Selecting past the end makes Go's View panic;
							// such steps are skipped.
							n, _ := strconv.Atoi(arg)
							if n < len(l.VisibleItems()) {
								l.Select(n)
							}
						case "q":
							applyPaletteQuery(&l, items, arg)
						case "left", "right":
							t := tea.KeyLeft
							if kind == "right" {
								t = tea.KeyRight
							}
							l, _ = l.Update(tea.KeyMsg{Type: t})
						case "key":
							l, _ = l.Update(listKey(arg))
						}
						p := strconv.Itoa(i)
						kv = append(kv, "v"+p, l.View(), "s"+p, listState(l))
					}
					w.rec(kv...)
				}
			}
		}
	}

	// Default list scenarios driven through Update (filtering included).
	defaultScenarios := []string{
		K(tea.KeyDown) + "|" + K(tea.KeyDown) + "|r:j|r:k|r:l|r:h|r:G|r:g|r:?|r:?|" + K(tea.KeyPgDown) + "|" + K(tea.KeyEnd) + "|" + K(tea.KeyHome),
		"r:/|r:t|r:h|r:e|" + K(tea.KeyDown) + "|r:x|" + K(tea.KeyBackspace) + "|" + K(tea.KeyEnter) + "|" + K(tea.KeyDown) + "|" + K(tea.KeyEsc),
		"r:/|r:zz|" + K(tea.KeyEnter) + "|r:/|" + K(tea.KeyEsc) + "|r:/|r:n|" + K(tea.KeyTab) + "|r:q",
		K(tea.KeyUp) + "|" + K(tea.KeyUp) + "|r:d|r:d|r:d|r:u|r:b|r:f|" + K(tea.KeyCtrlC),
		"r:/|r:ü|" + K(tea.KeyEnter) + "|ws|r:/|" + K(tea.KeyEnter),
	}
	defaultSizes := [][2]int{{40, 12}, {20, 10}, {60, 5}, {0, 0}}
	variants := []string{"", "nodesc", "spacing0", "height3", "infinite", "noquit", "nofilter", "names"}
	for _, run := range runs {
		for _, sz := range defaultSizes {
			for vi, variant := range variants {
				for si, sc := range defaultScenarios {
					if vi > 1 && sz[0] != 40 {
						continue
					}
					set := "palette"
					if (vi+si)%3 == 1 {
						set = "mixed"
					}
					if vi == 2 && si == 4 {
						set = "few"
					}
					if run.tag != "a" && (vi > 1 || sz[0] != 40) {
						continue // keep the colour runs smaller
					}
					lipgloss.SetColorProfile(run.p)
					lipgloss.SetHasDarkBackground(run.dark)
					l := newDefaultList(listItems(set), sz[0], sz[1], variant)
					kv := []string{"op", "default", "run", run.tag, "set", set, "variant", variant, "w", strconv.Itoa(sz[0]), "h", strconv.Itoa(sz[1]), "steps", sc,
						"v_init", l.View(), "s_init", listState(l)}
					for i, step := range strings.Split(sc, "|") {
						var cmd tea.Cmd
						l, cmd = l.Update(listKey(step))
						quit := false
						// One round of commands, like a program would run
						// them; further commands are dropped.
						for _, m := range runCmds([]tea.Cmd{cmd}) {
							if _, ok := m.(tea.QuitMsg); ok {
								quit = true
								continue
							}
							l, _ = l.Update(m)
						}
						p := strconv.Itoa(i)
						kv = append(kv, "v"+p, l.View(), "s"+p, listState(l), "quit"+p, strconv.FormatBool(quit))
					}
					w.rec(kv...)
				}
			}
		}
	}
	lipgloss.SetColorProfile(termenv.Ascii)
	lipgloss.SetHasDarkBackground(true)
}
