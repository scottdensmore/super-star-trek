package config

// Parity reference generator for the Rust port (crates/sst-tui/src/config.rs).
// Run with: SST_PARITYGEN=1 go test ./pkg/config -run TestParityGen -count=1
// Writes testdata/parity/config/ at the repository root:
//   decode.txt  hex-encoded JSON inputs and the LoadConfig-equivalent result
//               ("ok <volume> <muted> <audio_mode> <theme> <anim_speed>" or "err")
//   encode.txt  hex-encoded SaveConfig file contents for various configs
//   paths.txt   ConfigPath results for various HOME values

import (
	"encoding/hex"
	"encoding/json"
	"fmt"
	"math/rand/v2"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func decodeLikeLoad(data []byte) string {
	cfg := DefaultConfig()
	if err := json.Unmarshal(data, &cfg); err != nil {
		return "err"
	}
	cfg.Normalize()
	return fmt.Sprintf("ok %d %v %s %s %s", cfg.Volume, cfg.Muted, cfg.AudioMode, cfg.Theme, cfg.AnimSpeed)
}

func TestParityGen(t *testing.T) {
	if os.Getenv("SST_PARITYGEN") != "1" {
		t.Skip("set SST_PARITYGEN=1 to regenerate parity data")
	}
	out := filepath.Join("..", "..", "testdata", "parity", "config")
	if err := os.MkdirAll(out, 0o755); err != nil {
		t.Fatal(err)
	}

	inputs := []string{
		``, ` `, `{}`, `null`, ` null `, `[]`, `1`, `"x"`, `true`, `{"volume":65}`,
		`{"volume":65,"muted":true,"audio_mode":"native","theme":"lcars","anim_speed":"fast"}`,
		`{"VOLUME":10,"Muted":true,"AUDIO_MODE":"BELL","Theme":" CRT ","ANIM_SPEED":"Slow"}`,
		`{"volume":1,"VOLUME":2}`, `{"volume":1,"volume":3,"Volume":2}`, `{"volume":"10"}`,
		`{"volume":1.5}`, `{"volume":1e2}`, `{"volume":-0}`, `{"volume":-5}`, `{"volume":500}`,
		`{"volume":9223372036854775807}`, `{"volume":9223372036854775808}`, `{"volume":-9223372036854775808}`,
		`{"volume":3000000000}`, `{"volume":null}`, `{"muted":null}`, `{"theme":null}`, `{"muted":1}`,
		`{"muted":"true"}`, `{"theme":5}`, `{"theme":["lcars"]}`, `{"theme":{"a":1}}`, `{"volume":[1]}`,
		`{"unknown":1e400,"theme":"crt"}`, `{"unknown":[1,2,{"x":null}],"theme":"crt"}`,
		`{"theme":"lcArs"}`, `{"theme":"LCARS"}`, `{"theme":"crt\ud800"}`,
		`{"audio_mode":"natİve"}`, `{"audio_mode":"natİve"}`, `{"anim_ſpeed":"slow"}`,
		`{"anim_speed":" slow　"}`, `{"anim_speed":"\tslow\n"}`, `{"anim_speed":"slow\u0085"}`,
		`{"anim_speed":"slow​"}`, `{"theme":"modern"} x`, `{"theme":"modern"}{}`, `{"theme":"modern",}`,
		`{"theme" "modern"}`, `{theme:"modern"}`, `{'theme':'modern'}`, `{"theme":"mod` + "\n" + `ern"}`,
		`{"theme":"\x"}`, `{"theme":"\/crt"}`, `{"volume":01}`, `{"volume":-}`, `{"volume":1.}`,
		`{"volume":.5}`, `{"volume":1e}`, `{"volume":+1}`, `{"muted":tru}`, `{"muted":True}`, `{"muted":false}`,
		"\xef\xbb\xbf{}", "{\"theme\":\"lcars\xff\"}", "{\"the\xffme\":\"lcars\"}", "{\"theme\":\"lcars\"}\x00",
		`{"invalid json corrupt`, `{invalid json corrupt`, "{\"theme\":\"crt\"}\r\n\t ",
		`{"volume":80,"muted":false,"audio_mode":"auto","theme":"modern","anim_speed":"normal"}`,
		`{"Volume":0,"muted":true}`, `{"audio_mode":"OFF"}`, `{"audio_mode":""}`, `{"audio_mode":"  native  "}`,
		`{"volume":1E1}`, `{"volume":0.0}`, `{"volume":1e-1}`, `{"x":-01}`, `{"x":1.e5}`, `{"x":"\u12"}`,
		`{"x":"😀"}`, `{"x":[}`, `{"x":]}`, `{"x":{}}`, `{"x":[[[[[]]]]]}`, `[{"theme":"crt"}]`,
		strings.Repeat("[", 10001) + strings.Repeat("]", 10001),
		`{"x":` + strings.Repeat("[", 10000) + strings.Repeat("]", 10000) + `}`,
		`{"x":` + strings.Repeat("[", 9999) + strings.Repeat("]", 9999) + `}`,
	}
	r := rand.New(rand.NewPCG(7, 8))
	seeds := inputs[:]
	alphabet := []byte("{}[]:,\"\\ \t\n0123456789-+.eEtrufalsn\x00\xffuU/abcdbfnrtvolumetheme_")
	var gen []string
	for i := 0; i < 4000; i++ {
		s := []byte(seeds[r.IntN(len(seeds))])
		if len(s) > 200 {
			continue
		}
		m := 1 + r.IntN(3)
		for j := 0; j < m; j++ {
			switch r.IntN(3) {
			case 0:
				if len(s) > 0 {
					k := r.IntN(len(s))
					s = append(s[:k], s[k+1:]...)
				}
			case 1:
				k := r.IntN(len(s) + 1)
				c := alphabet[r.IntN(len(alphabet))]
				s = append(s[:k], append([]byte{c}, s[k:]...)...)
			default:
				if len(s) > 0 {
					s[r.IntN(len(s))] = alphabet[r.IntN(len(alphabet))]
				}
			}
		}
		gen = append(gen, string(s))
	}
	var db strings.Builder
	for _, in := range append(inputs, gen...) {
		fmt.Fprintf(&db, "%s %s\n", hex.EncodeToString([]byte(in)), decodeLikeLoad([]byte(in)))
	}
	if err := os.WriteFile(filepath.Join(out, "decode.txt"), []byte(db.String()), 0o644); err != nil {
		t.Fatal(err)
	}

	// Encoding: SaveConfig output (MarshalIndent + newline) for normalized configs.
	var eb strings.Builder
	cfgs := []Config{
		DefaultConfig(),
		{Volume: 65, Muted: true, AudioMode: AudioModeNative, Theme: "lcars", AnimSpeed: "fast"},
		{Volume: 0, Muted: false, AudioMode: AudioModeOff, Theme: "crt", AnimSpeed: "slow"},
		{Volume: 100, Muted: true, AudioMode: AudioModeBell, Theme: "<b>&", AnimSpeed: "x"},
		{Volume: -3, AudioMode: "weird"},
		{Volume: 1000},
	}
	for i, c := range cfgs {
		home := t.TempDir()
		t.Setenv("HOME", home)
		if err := SaveConfig(c); err != nil {
			t.Fatal(err)
		}
		p, _ := ConfigPath()
		data, err := os.ReadFile(p)
		if err != nil {
			t.Fatal(err)
		}
		st, _ := os.Stat(p)
		fmt.Fprintf(&eb, "%d %o %s\n", i, st.Mode().Perm(), hex.EncodeToString(data))
	}
	// Raw MarshalIndent (no Normalize) to pin string escaping.
	weird := []string{"<b>&amp;", "a\u2028b\u2029c", "\x00\x01\x1f\x7f", "q\"b\\s/", "\b\f\n\r\t", "bad\xff\xfeutf8", "héllo ☃ 😀", ""}
	for _, w := range weird {
		data, err := json.MarshalIndent(Config{Volume: -7, Muted: true, AudioMode: AudioMode(w), Theme: w, AnimSpeed: w}, "", "  ")
		if err != nil {
			t.Fatal(err)
		}
		fmt.Fprintf(&eb, "raw %s %s\n", hex.EncodeToString([]byte(w)), hex.EncodeToString(data))
	}
	if err := os.WriteFile(filepath.Join(out, "encode.txt"), []byte(eb.String()), 0o644); err != nil {
		t.Fatal(err)
	}

	// Paths.
	var pb strings.Builder
	for _, h := range []string{"", "/home/u", "/home/u/", "/a//b/./c/../d", "rel/dir", ".", "/", "~", "/x y/z"} {
		t.Setenv("HOME", h)
		p, err := ConfigPath()
		if err != nil {
			fmt.Fprintf(&pb, "%q err %s\n", h, err)
		} else {
			fmt.Fprintf(&pb, "%q ok %s\n", h, p)
		}
	}
	if err := os.WriteFile(filepath.Join(out, "paths.txt"), []byte(pb.String()), 0o644); err != nil {
		t.Fatal(err)
	}
}
