package commandpalette

import (
	"github.com/charmbracelet/bubbles/list"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

// PaletteItem represents an entry in the command palette catalog.
type PaletteItem struct {
	title         string
	desc          string
	prefix        string
	parameterized bool
}

// Title returns the command label for display.
func (p PaletteItem) Title() string {
	return p.title
}

// Description returns the command help summary.
func (p PaletteItem) Description() string {
	return p.desc
}

// FilterValue returns the string used for fuzzy filtering against user queries.
func (p PaletteItem) FilterValue() string {
	return p.title + " " + p.desc
}

// Prefix returns the command prefix string to inject into the command bar.
func (p PaletteItem) Prefix() string {
	return p.prefix
}

// Parameterized reports whether the command requires additional user arguments.
func (p PaletteItem) Parameterized() bool {
	return p.parameterized
}

// CommandSelectedMsg is emitted when a command is selected from the palette.
type CommandSelectedMsg struct {
	CommandPrefix string
	Parameterized bool
}

// ClosePaletteMsg is emitted when dismissing the command palette without selecting.
type ClosePaletteMsg struct{}

// Model represents the Spock Command Palette modal component.
type Model struct {
	theme   theme.Theme
	list    list.Model
	catalog []PaletteItem
	query   string
	width   int
	height  int
}

// defaultCatalog returns the canonical Super Star Trek command palette catalog.
func defaultCatalog() []PaletteItem {
	return []PaletteItem{
		// Tactical & Combat
		{
			title:         "TOR",
			desc:          "Fire photon torpedo along bearing or sector (tor <course|r c>)",
			prefix:        "tor ",
			parameterized: true,
		},
		{
			title:         "PHA",
			desc:          "Fire ship phaser bank with energy (pha <energy>)",
			prefix:        "pha ",
			parameterized: true,
		},
		{
			title:         "SHE",
			desc:          "Transfer energy between warp and shields (she <amount>)",
			prefix:        "she ",
			parameterized: true,
		},
		{
			title:         "TARGET",
			desc:          "Open Tactical Target Lock HUD",
			prefix:        "target",
			parameterized: false,
		},

		// Navigation
		{
			title:         "NAV",
			desc:          "Impulse / Warp maneuver (nav <course> <warp>)",
			prefix:        "nav ",
			parameterized: true,
		},
		{
			title:         "DOC",
			desc:          "Dock with adjacent Starbase for fuel & repair",
			prefix:        "doc",
			parameterized: false,
		},
		{
			title:         "SRSCAN",
			desc:          "Refresh short-range sensor tactical grid",
			prefix:        "srscan",
			parameterized: false,
		},
		{
			title:         "LRSCAN",
			desc:          "Scan adjacent 3x3 quadrant cluster",
			prefix:        "lrscan",
			parameterized: false,
		},

		// Reports
		{
			title:         "STATUS",
			desc:          "Review ship status, condition, and stardate",
			prefix:        "status",
			parameterized: false,
		},
		{
			title:         "DAM",
			desc:          "Review subsystem device damage repair times",
			prefix:        "dam",
			parameterized: false,
		},
		{
			title:         "CHART",
			desc:          "Display explored galaxy quadrant chart",
			prefix:        "chart",
			parameterized: false,
		},
		{
			title:         "SAVES",
			desc:          "Browse, inspect, and load saved missions (Ctrl+O)",
			prefix:        "saves",
			parameterized: false,
		},

		// Settings & Help
		{
			title:         "THEME: Modern",
			desc:          "Switch theme to Starfleet Modern (Electric Cyan/Navy)",
			prefix:        "theme modern",
			parameterized: false,
		},
		{
			title:         "THEME: LCARS",
			desc:          "Switch theme to 24th Century LCARS (Gold/Purple)",
			prefix:        "theme lcars",
			parameterized: false,
		},
		{
			title:         "THEME: CRT",
			desc:          "Switch theme to Phosphor Monochrome CRT (Green/Amber)",
			prefix:        "theme crt",
			parameterized: false,
		},
		{
			title:         "HELP",
			desc:          "Display tactical command reference summary",
			prefix:        "help",
			parameterized: false,
		},
		{
			title:         "QUIT",
			desc:          "Abandon mission and exit to shell",
			prefix:        "quit",
			parameterized: false,
		},
	}
}

// New creates a new Model initialized with the given theme and dimensions.
func New(th theme.Theme, width, height int) Model {
	if th == nil {
		th = theme.DefaultTheme()
	}
	if width <= 0 {
		width = 56
	}
	if height <= 0 {
		height = 16
	}

	catalog := defaultCatalog()
	listItems := make([]list.Item, len(catalog))
	for i, it := range catalog {
		listItems[i] = it
	}

	delegate := list.NewDefaultDelegate()
	delegate.ShowDescription = true
	delegate.SetSpacing(0)

	l := list.New(listItems, delegate, width, height)
	l.Title = "COMMAND PALETTE"
	l.SetShowTitle(true)
	l.SetShowStatusBar(false)
	l.SetShowPagination(false)
	l.SetShowHelp(false)
	l.DisableQuitKeybindings()

	m := Model{
		theme:   th,
		list:    l,
		catalog: catalog,
		width:   width,
		height:  height,
	}

	m.updateStyles()
	m.SetSize(width, height)
	return m
}

// SetTheme updates the active styling theme.
func (m *Model) SetTheme(th theme.Theme) {
	if th == nil {
		th = theme.DefaultTheme()
	}
	m.theme = th
	m.updateStyles()
	m.SetSize(m.width, m.height)
}

// Theme returns the currently active theme.
func (m Model) Theme() theme.Theme {
	if m.theme == nil {
		return theme.DefaultTheme()
	}
	return m.theme
}

// SetSize updates the width and height of the modal dialog and list component.
func (m *Model) SetSize(width, height int) {
	if width <= 0 {
		width = 56
	}
	if height <= 0 {
		height = 16
	}
	m.width = width
	m.height = height

	th := m.theme
	if th == nil {
		th = theme.DefaultTheme()
	}
	styles := th.Styles()
	panelStyle := styles.Panel.Border(styles.Border, true).Padding(0, 0)
	borderH := panelStyle.GetHorizontalBorderSize()
	if borderH == 0 {
		borderH = 2
	}
	borderV := panelStyle.GetVerticalBorderSize()
	if borderV == 0 {
		borderV = 2
	}

	innerWidth := width - borderH
	if innerWidth < 10 {
		innerWidth = 10
	}
	innerHeight := height - borderV
	if innerHeight < 1 {
		innerHeight = 1
	}

	m.list.SetSize(innerWidth, innerHeight)
}

// updateStyles applies the active theme's styling to the list delegate and container.
func (m *Model) updateStyles() {
	th := m.theme
	if th == nil {
		th = theme.DefaultTheme()
	}
	styles := th.Styles()

	d := list.NewDefaultDelegate()
	d.ShowDescription = true
	d.SetSpacing(0)

	d.Styles.NormalTitle = styles.CommandText.Padding(0, 0, 0, 2)
	d.Styles.NormalDesc = styles.GaugeLabel.Padding(0, 0, 0, 2)

	accentColor := styles.Prompt.GetForeground()
	if accentColor == nil {
		accentColor = lipgloss.Color("6")
	}

	d.Styles.SelectedTitle = styles.Prompt.
		Border(lipgloss.NormalBorder(), false, false, false, true).
		BorderForeground(accentColor).
		Padding(0, 0, 0, 1)
	d.Styles.SelectedDesc = styles.GaugeLabel.
		Border(lipgloss.NormalBorder(), false, false, false, true).
		BorderForeground(accentColor).
		Padding(0, 0, 0, 1)

	d.Styles.FilterMatch = lipgloss.NewStyle().Underline(true)

	m.list.SetDelegate(d)
	m.list.Styles.Title = styles.PanelTitle.Padding(0, 1)
	m.list.Styles.NoItems = styles.ConditionRed.Padding(0, 0, 0, 2)
}

// Reset clears any active filter query and resets selection to the top item.
func (m *Model) Reset() {
	m.query = ""
	m.applyFilter()
	m.list.Select(0)
}

// applyFilter updates the list items according to the current query.
func (m *Model) applyFilter() {
	if m.query == "" {
		m.list.Title = "COMMAND PALETTE"
		items := make([]list.Item, len(m.catalog))
		for i, it := range m.catalog {
			items[i] = it
		}
		m.list.SetItems(items)
		m.list.FilterInput.Reset()
		m.list.Select(0)
		return
	}

	m.list.Title = "COMMAND PALETTE: " + m.query
	m.list.FilterInput.SetValue(m.query)

	targets := make([]string, len(m.catalog))
	for i, it := range m.catalog {
		targets[i] = it.FilterValue()
	}

	ranks := list.DefaultFilter(m.query, targets)
	filtered := make([]list.Item, len(ranks))
	for i, r := range ranks {
		filtered[i] = m.catalog[r.Index]
	}
	m.list.SetItems(filtered)
	m.list.Select(0)
}

// Items returns a copy of all catalog items.
func (m Model) Items() []PaletteItem {
	out := make([]PaletteItem, len(m.catalog))
	copy(out, m.catalog)
	return out
}

// VisibleItems returns the items currently matching the filter.
func (m Model) VisibleItems() []PaletteItem {
	items := m.list.VisibleItems()
	out := make([]PaletteItem, 0, len(items))
	for _, it := range items {
		if pi, ok := it.(PaletteItem); ok {
			out = append(out, pi)
		}
	}
	return out
}

// SelectedItem returns the active selected item, or nil if none.
func (m Model) SelectedItem() *PaletteItem {
	sel := m.list.SelectedItem()
	if pi, ok := sel.(PaletteItem); ok {
		return &pi
	}
	return nil
}

// FilterValue returns the current filter query string.
func (m Model) FilterValue() string {
	return m.query
}

// Update handles keyboard navigation, filter typing, selection, and dismissal.
func (m Model) Update(msg tea.Msg) (Model, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.KeyMsg:
		keyStr := msg.String()

		// Esc dismisses the palette
		if msg.Type == tea.KeyEsc || keyStr == "esc" {
			return m, func() tea.Msg {
				return ClosePaletteMsg{}
			}
		}

		// Enter selects the current item
		if msg.Type == tea.KeyEnter || keyStr == "enter" {
			sel := m.list.SelectedItem()
			if item, ok := sel.(PaletteItem); ok {
				return m, func() tea.Msg {
					return CommandSelectedMsg{
						CommandPrefix: item.prefix,
						Parameterized: item.parameterized,
					}
				}
			}
			return m, nil
		}

		// Cursor navigation
		switch {
		case msg.Type == tea.KeyUp || keyStr == "up":
			m.list.CursorUp()
			return m, nil
		case msg.Type == tea.KeyDown || keyStr == "down":
			m.list.CursorDown()
			return m, nil
		case msg.Type == tea.KeyPgUp || keyStr == "pgup":
			m.list.PrevPage()
			return m, nil
		case msg.Type == tea.KeyPgDown || keyStr == "pgdown":
			m.list.NextPage()
			return m, nil
		case msg.Type == tea.KeyHome || keyStr == "home":
			m.list.Select(0)
			return m, nil
		case msg.Type == tea.KeyEnd || keyStr == "end":
			if len(m.list.VisibleItems()) > 0 {
				m.list.Select(len(m.list.VisibleItems()) - 1)
			}
			return m, nil
		}

		// Filter editing: Backspace / Delete
		if msg.Type == tea.KeyBackspace || msg.Type == tea.KeyDelete || keyStr == "backspace" || keyStr == "delete" {
			runes := []rune(m.query)
			if len(runes) > 0 {
				m.query = string(runes[:len(runes)-1])
				m.applyFilter()
			}
			return m, nil
		}

		// Filter editing: Space
		if msg.Type == tea.KeySpace || keyStr == " " {
			m.query += " "
			m.applyFilter()
			return m, nil
		}

		// Filter editing: Runes / Character input
		if msg.Type == tea.KeyRunes && len(msg.Runes) > 0 {
			m.query += string(msg.Runes)
			m.applyFilter()
			return m, nil
		}
		if len(keyStr) == 1 && keyStr[0] >= 32 && keyStr[0] <= 126 {
			m.query += keyStr
			m.applyFilter()
			return m, nil
		}

	case tea.WindowSizeMsg:
		// Safely ignore WindowSizeMsg to preserve fixed dialog dimensions (e.g. 56x16)
		return m, nil
	}

	var cmd tea.Cmd
	m.list, cmd = m.list.Update(msg)
	return m, cmd
}

// View renders the styled modal dialog containing the list.
func (m Model) View() string {
	th := m.theme
	if th == nil {
		th = theme.DefaultTheme()
	}
	styles := th.Styles()

	targetWidth := m.width
	if targetWidth <= 0 {
		targetWidth = 56
	}
	targetHeight := m.height
	if targetHeight <= 0 {
		targetHeight = 16
	}

	panelStyle := styles.Panel.Border(styles.Border, true).Padding(0, 0)
	borderH := panelStyle.GetHorizontalBorderSize()
	if borderH == 0 {
		borderH = 2
	}
	borderV := panelStyle.GetVerticalBorderSize()
	if borderV == 0 {
		borderV = 2
	}

	innerWidth := targetWidth - borderH
	if innerWidth < 10 {
		innerWidth = 10
	}
	innerHeight := targetHeight - borderV
	if innerHeight < 1 {
		innerHeight = 1
	}

	listView := m.list.View()

	return panelStyle.Width(innerWidth).Height(innerHeight).Render(listView)
}
