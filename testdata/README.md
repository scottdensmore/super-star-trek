# Reference transcripts

These files were recorded from the original Go implementation of Super Star
Trek and are replayed by the Rust test suite, which must reproduce them byte
for byte.

| Directory | Contents | Replayed by |
| :--- | :--- | :--- |
| `parity/engine` | PRNG streams, galaxy setup, scenarios, tours, scoring, save files, 264 scripted games | `crates/sst-engine/tests/parity_*.rs` |
| `parity/tuikit` | Styled rendering, widths, colour degradation, key decoding, text input, lists | `crates/sst-tuikit/tests/*_parity.rs` |
| `parity/theme`, `parity/anim` | Theme styles under every colour profile, animation frames | `crates/sst-tui/src/{theme,anim}` |
| `parity/components` | Every console component's rendered view across states and themes | `crates/sst-tui/src/components/*/parity_tests.rs` |
| `parity/tui` | 60 scripted bridge console sessions (frame hash per step, plus full frames) | `crates/sst-tui/tests/tui_parity.rs` |
| `parity/cli` | 124 `sst` invocations: stdout, stderr, exit code, initial view | `crates/sst/tests/cli_parity.rs` |
| `parity/wasm` | Browser session transcripts, formatter output | `crates/sst-wasm/tests/parity.rs`, `tests/wasm_parity_replay.js` |
| `parity/audio`, `parity/config` | Synthesized WAV hashes, sound dispatch, config files | `crates/sst-audio`, `crates/sst-tui` |
| `golden/tui` | 80x24 console goldens | `crates/sst-tui/tests/tui_golden.rs` |

The Go generators that produced them (`tools/paritygen/` and the
`zz_paritygen_test.go` files) were removed along with the Go sources; they are
in the history at commit `fc960a2`. Do not regenerate by hand: a change here
means the game's behaviour changed.
