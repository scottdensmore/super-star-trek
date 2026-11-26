package main

import (
	"strconv"
	"strings"
	"time"

	"github.com/charmbracelet/bubbles/cursor"
	"github.com/charmbracelet/bubbles/textinput"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/muesli/termenv"
)

// tiConfig builds a textinput configured like one of the game's uses (or a
// variation). The Rust test has the same table (tests/textinput_parity.rs).
func tiConfig(name string) textinput.Model {
	ti := textinput.New()
	ti.Cursor.BlinkSpeed = time.Millisecond
	switch name {
	case "default":
	case "commandbar":
		ti.Prompt = "COMMAND> "
		ti.PromptStyle = lipgloss.NewStyle().Bold(true).Foreground(lipgloss.Color("#00E5FF"))
		ti.TextStyle = lipgloss.NewStyle().Foreground(lipgloss.Color("#E2E8F0"))
	case "commandbar_w40":
		ti.Prompt = "COMMAND> "
		ti.PromptStyle = lipgloss.NewStyle().Bold(true).Foreground(lipgloss.Color("#00E5FF"))
		ti.TextStyle = lipgloss.NewStyle().Foreground(lipgloss.Color("#E2E8F0"))
		ti.Width = 40
	case "narrow":
		ti.Prompt = "> "
		ti.Width = 8
	case "halloffame":
		ti.Prompt = ""
		ti.Placeholder = "Enter callsign..."
		ti.CharLimit = 20
		ti.Width = 20
		ti.PromptStyle = lipgloss.NewStyle().Bold(true).Foreground(lipgloss.Color("#FFD700"))
		ti.TextStyle = lipgloss.NewStyle().Foreground(lipgloss.Color("#E2E8F0"))
	case "placeholder_nowidth":
		ti.Placeholder = "Type a command"
	case "placeholder_short":
		ti.Placeholder = "x"
	case "password":
		ti.EchoMode = textinput.EchoPassword
		ti.Width = 12
	case "echonone":
		ti.EchoMode = textinput.EchoNone
	case "limit5":
		ti.CharLimit = 5
	case "suggest":
		ti.ShowSuggestions = true
		ti.SetSuggestions([]string{"navigate", "nav", "phasers", "photon", "status"})
	case "static":
		ti.SetCursorMode(textinput.CursorStatic)
	case "hidden":
		ti.SetCursorMode(textinput.CursorHide)
	case "wide":
		ti.Width = 6
		ti.Prompt = "★ "
	}
	return ti
}

// runCmds executes commands synchronously (batches flattened) and returns
// their messages.
func runCmds(cmds []tea.Cmd) []tea.Msg {
	var out []tea.Msg
	for _, c := range cmds {
		if c == nil {
			continue
		}
		m := c()
		if b, ok := m.(tea.BatchMsg); ok {
			out = append(out, runCmds(b)...)
			continue
		}
		if m != nil {
			out = append(out, m)
		}
	}
	return out
}

func tiStepMsg(step string) tea.Msg {
	kind, arg, _ := strings.Cut(step, ":")
	switch kind {
	case "r":
		return tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune(arg)}
	case "ra":
		return tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune(arg), Alt: true}
	case "p":
		return tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune(arg), Paste: true}
	case "k", "ka":
		t, _ := strconv.Atoi(arg)
		return tea.KeyMsg{Type: tea.KeyType(t), Alt: kind == "ka"}
	case "sp":
		return tea.KeyMsg{Type: tea.KeySpace, Runes: []rune{' '}}
	case "focusmsg":
		return tea.FocusMsg{}
	case "blurmsg":
		return tea.BlurMsg{}
	}
	return nil
}

func genTextinput(path string) {
	w := newRecWriter(path)
	defer w.close()

	K := func(t tea.KeyType) string { return "k:" + strconv.Itoa(int(t)) }
	KA := func(t tea.KeyType) string { return "ka:" + strconv.Itoa(int(t)) }
	scenarios := []string{
		"focus|r:nav 1 2|blink|blink|" + K(tea.KeyLeft) + "|" + K(tea.KeyLeft) + "|r:X|blink|" + K(tea.KeyHome) + "|" + K(tea.KeyDelete) + "|" + K(tea.KeyEnd) + "|" + K(tea.KeyBackspace),
		"focus|r:hello world foo|" + K(tea.KeyCtrlW) + "|" + KA(tea.KeyLeft) + "|" + K(tea.KeyCtrlK) + "|r:abc|" + K(tea.KeyCtrlA) + "|" + KA(tea.KeyRight) + "|" + K(tea.KeyCtrlU),
		"focus|r:one two  three|" + K(tea.KeyCtrlLeft) + "|" + K(tea.KeyCtrlLeft) + "|ra:d|" + K(tea.KeyCtrlRight) + "|" + KA(tea.KeyBackspace) + "|" + KA(tea.KeyDelete),
		"r:ignored|focus|r:a|sp|r:b|" + K(tea.KeyTab) + "|" + K(tea.KeyCtrlB) + "|" + K(tea.KeyCtrlF) + "|" + K(tea.KeyCtrlE) + "|" + K(tea.KeyCtrlH) + "|" + K(tea.KeyCtrlD),
		"focus|p:pasted\ttext\nnewline|blink|" + K(tea.KeyHome) + "|blink|blink",
		"focus|r:日本語テキスト|" + K(tea.KeyLeft) + "|" + K(tea.KeyLeft) + "|r:★|blink",
		"focus|r:abcdefghijklmnopqrstuvwxyz0123456789|" + K(tea.KeyHome) + "|" + K(tea.KeyRight) + "|" + K(tea.KeyRight) + "|" + K(tea.KeyEnd) + "|" + K(tea.KeyLeft) + "|" + K(tea.KeyCtrlA) + "|" + K(tea.KeyCtrlE),
		"focus|blur|r:x|focus|r:y|blurmsg|r:z|focusmsg|blink",
		"focus|r:na|" + K(tea.KeyDown) + "|" + K(tea.KeyTab) + "|" + K(tea.KeyCtrlW) + "|r:ph|" + K(tea.KeyUp) + "|" + K(tea.KeyTab) + "|r:Q",
		"focus|setvalue:status report|cursorstart|r:>|setcursor:3|blink|reset|r:z",
		"focus|r:" + strings.Repeat("w", 30) + "|" + K(tea.KeyHome) + "|" + K(tea.KeyEnd),
		"focus|r:Spock|blink|blink|blink|" + K(tea.KeyBackspace) + "|" + K(tea.KeyBackspace),
		"focus|r:\x07bell\x7f|r:tab\there|" + K(tea.KeyEsc) + "|" + K(tea.KeyEnter),
	}
	configs := []string{"default", "commandbar", "commandbar_w40", "narrow", "halloffame", "placeholder_nowidth",
		"placeholder_short", "password", "echonone", "limit5", "suggest", "static", "hidden", "wide"}

	views := func(ti textinput.Model) (string, string) {
		lipgloss.SetColorProfile(termenv.Ascii)
		a := ti.View()
		lipgloss.SetColorProfile(termenv.TrueColor)
		t := ti.View()
		lipgloss.SetColorProfile(termenv.Ascii)
		return a, t
	}

	for _, cfg := range configs {
		for si, sc := range scenarios {
			ti := tiConfig(cfg)
			var pending []tea.Cmd
			kv := []string{"config", cfg, "steps", sc}
			a, t := views(ti)
			kv = append(kv, "init_a", a, "init_t", t)
			for i, step := range strings.Split(sc, "|") {
				kind, arg, _ := strings.Cut(step, ":")
				switch kind {
				case "focus":
					pending = append(pending, ti.Focus())
				case "blur":
					ti.Blur()
				case "blink":
					msgs := runCmds(pending)
					pending = nil
					for _, m := range msgs {
						var c tea.Cmd
						ti, c = ti.Update(m)
						pending = append(pending, c)
					}
				case "setvalue":
					ti.SetValue(arg)
				case "cursorstart":
					ti.CursorStart()
				case "setcursor":
					n, _ := strconv.Atoi(arg)
					ti.SetCursor(n)
				case "reset":
					ti.Reset()
				default:
					var c tea.Cmd
					ti, c = ti.Update(tiStepMsg(step))
					pending = append(pending, c)
				}
				a, t := views(ti)
				p := strconv.Itoa(i)
				kv = append(kv, "a"+p, a, "t"+p, t, "val"+p, ti.Value(), "pos"+p, strconv.Itoa(ti.Position()),
					"blink"+p, strconv.FormatBool(ti.Cursor.Blink), "focused"+p, strconv.FormatBool(ti.Focused()),
					"sug"+p, strings.Join(ti.MatchedSuggestions(), ",")+"#"+strconv.Itoa(ti.CurrentSuggestionIndex()))
			}
			_ = si
			w.rec(kv...)
		}
	}
	_ = cursor.CursorBlink
}
