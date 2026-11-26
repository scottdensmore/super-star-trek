package main

import (
	"bytes"
	"sort"
	"strconv"
	"strings"

	"github.com/muesli/termenv"
)

type fakeEnv map[string]string

func (e fakeEnv) Environ() []string {
	var out []string
	for k, v := range e {
		out = append(out, k+"="+v)
	}
	sort.Strings(out)
	return out
}
func (e fakeEnv) Getenv(k string) string { return e[k] }

func envString(e fakeEnv) string {
	return strings.Join(e.Environ(), ";")
}

func genTermenv(path string) {
	w := newRecWriter(path)
	defer w.close()

	terms := []string{"", "xterm", "xterm-256color", "screen", "screen-256color", "tmux-256color", "linux", "alacritty",
		"xterm-kitty", "xterm-ghostty", "wezterm", "rio", "contour", "vt100", "ansi", "dumb", "rxvt-unicode-256color", "color"}
	colorterms := []string{"", "truecolor", "24bit", "TrueColor", "yes", "true", "no"}
	extras := []fakeEnv{
		{}, {"NO_COLOR": "1"}, {"CLICOLOR": "0"}, {"CLICOLOR": "0", "CLICOLOR_FORCE": "1"}, {"CLICOLOR_FORCE": "1"},
		{"CLICOLOR_FORCE": "0"}, {"CI": "true"}, {"GOOGLE_CLOUD_SHELL": "true"}, {"TERM_PROGRAM": "tmux"},
	}
	for _, t := range terms {
		for _, ct := range colorterms {
			for _, ex := range extras {
				for _, tty := range []bool{true, false} {
					env := fakeEnv{}
					for k, v := range ex {
						env[k] = v
					}
					if t != "" {
						env["TERM"] = t
					}
					if ct != "" {
						env["COLORTERM"] = ct
					}
					o := termenv.NewOutput(&bytes.Buffer{}, termenv.WithEnvironment(env), termenv.WithTTY(tty))
					w.rec("op", "profile", "env", envString(env), "tty", strconv.FormatBool(tty),
						"profile", strconv.Itoa(int(o.ColorProfile())), "envprofile", strconv.Itoa(int(o.EnvColorProfile())),
						"nocolor", strconv.FormatBool(o.EnvNoColor()))
				}
			}
		}
	}

	for _, fgbg := range []string{"", "15;0", "0;15", "7;0", "0;7", "15;default;0", "12;11", "1;2;3", "x", "15;x", "0;10", "0;8"} {
		for _, tty := range []bool{true, false} {
			for _, t := range []string{"xterm-256color", "screen", "dumb"} {
				env := fakeEnv{"TERM": t}
				if fgbg != "" {
					env["COLORFGBG"] = fgbg
				}
				o := termenv.NewOutput(&bytes.Buffer{}, termenv.WithEnvironment(env), termenv.WithTTY(tty))
				bg := o.BackgroundColor()
				w.rec("op", "background", "env", envString(env), "tty", strconv.FormatBool(tty),
					"bg", colorSeq(bg, true), "dark", strconv.FormatBool(o.HasDarkBackground()))
			}
		}
	}
}
