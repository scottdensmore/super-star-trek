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

## Re-recording after an intentional change

Fix the behaviour first, with a regression test that fails before the fix.
Then re-record only the affected transcripts; the replay harnesses rewrite
just the cases whose output changed and copy every other case byte for byte:

| Transcript | Command |
| :--- | :--- |
| `parity/engine/<file>.txt` | `SST_PARITY_RECORD=/tmp/<file>.txt cargo test --release -p sst-engine --test parity_<harness>` |
| `parity/tui/*.txt` | `SST_PARITY_RECORD=/tmp/tui cargo test --release -p sst-tui --test tui_parity` (writes every file into the directory) |
| `parity/wasm/sessions.txt` | `SST_PARITY_RECORD=/tmp/sessions.txt cargo test --release -p sst-wasm --test parity parity_sessions` |

Copy the result over the reference, check with `git diff` that only the
cases you expect moved, and add an entry below. (`SST_PARITY_RECORD_FULL=1`
additionally writes the engine game payloads in full instead of hashed, which
helps to read a changed game; do not commit that output.) The CLI transcript
`parity/cli/cli.txt` stores byte lengths, so edit it with a script that
rewrites the lengths; its `--version` output is stored with `{VERSION}`,
`{COMMIT}` and `{DATE}` placeholders that the test fills in from the build.

## Intentional changes from the Go reference

The Go edition had bugs that the port first reproduced for parity. These were
fixed afterwards; each fix has regression tests in
`crates/sst-engine/tests/bug_fixes.rs` (or the crate named), and the
transcripts below were re-recorded for exactly the cases whose behaviour the
fix changes. Every other case is still the original Go recording.

1. **A won game clears a patrol-tour sector.** `EvaluateSector` only looked
   at the hostile counters, so a `GameOver(Won)` in a tour (for example the
   new victory below) did not clear the sector, and the console opened the
   Hall of Fame. Now a won game clears the sector and the console goes
   straight to the drydock.
2. **Torpedoes hit cloaked Romulans.** A cloaked Romulan leaves its sector
   empty on the grid, so torpedoes flew through it and the "decloak on hit"
   code in `ActionFireTorpedo` could never run; a Romulan could only be hit in
   the single turn it was visible, and its decloak event reported Romulan ID
   0. Torpedo traces now treat cloaked Romulans as solid (as a cloaked Klingon
   commander, which keeps its grid cell, already was): blind fire along its
   line hits it, decloaks it with its real ID, applies shields-then-hull damage
   and can destroy it. Target lock still refuses cloaked ships.
3. **Klingon damage and kills survive the adversary turn.** Phasers,
   torpedoes and `KlingonTurn` act on the legacy Klingon list while the
   adversary turn acts on `Enemies` and then rebuilds the legacy list from it,
   so with adversaries enabled every turn restored damaged Klingons to full
   energy and brought destroyed ones back (they kept firing). The legacy list
   is now written back to `Enemies` before the adversary turn, and kills
   remove the Klingon from both lists. Focused phasers (`pha E r,c`) on a
   Klingon in `Enemies` now update the legacy list too, report a `PhaserHit`
   and count the kill (Go only counted Romulan kills there).
4. **The adversary turn can destroy the Enterprise.** Romulan, plasma and
   pack fire drove the ship's energy below zero without ending the game; the
   adversary turn now ends it (`GameOver(Lost)`) the way `KlingonTurn` does.
5. **Destroying the last Klingon wins a normal game.** Games without a
   scenario had no victory check, so play went on after the last kill. A turn
   that destroys the last hostile now emits `GameOver(Won)` with the classic
   score (including the skill win bonus); scenarios keep their own rules.
6. **A loaded tour save has a working PRNG.** `LoadTourGame` only rebuilt the
   generator when it was nil, but the envelope's `"RNG": {}` decodes into a
   PRNG with a nil source, so the next random draw panicked. Tour loads now
   reseed from the stardate exactly like `LoadGame`, so both loaders return
   the same state for the same file.
7. **No "Go" in user-visible text.** The `--classic` banner reads
   "Super Star Trek" (was "Super Star Trek (Go Edition)"), `--classic=false`
   no longer names the Go TUI library, `--version` reports the Cargo package
   version (was Go's hard-coded `2.0.0-dev`; `SST_VERSION`, `SST_COMMIT` and
   `SST_BUILD_DATE` still override it for release builds), and JSON decoding
   errors say "into struct field ..." / "into value of type ..." instead of
   "into Go struct field ..." / "into Go value of type ...".

Re-recorded cases:

| Transcript | Cases | Cause |
| :--- | :--- | :--- |
| `engine/games.txt` | 63 of the 64 TUI games with adversaries (1, 4, 5, 7, 10, 13, 16, 19-22, 25, 28, 29, 31, 34, 36, 37, 40, 43-46, 49, 52, 53, 55, 58, 60, 61, 64, 67-70, 73, 76, 77, 79, 82, 84, 85, 88, 91-94, 97, 100, 101, 103, 106, 108, 109, 112, 115-118, 121, 124, 125, 127) | 3, 4 |
| `engine/games.txt` | 25 of the 26 tour games (178-185, 187-203) | 1-5 |
| `engine/games.txt` | 11 browser sessions on the expert/emeritus profiles, which enable adversaries (208, 209, 216, 217, 225, 233, 241, 248, 249, 256, 257) | 3, 4 |
| `engine/save.txt` | `loadtour` hash of every envelope save (now equal to the `load` hash) and the two tour-envelope `case` loads (`R` line) | 6 |
| `engine/save.txt`, `engine/score.txt` | JSON type-error messages | 7 |
| `tui/games.txt` | `game_adversaries`, `game_click_targets` (both profiles; the latter showed a destroyed Klingon returning fire) | 3 |
| `tui/tour.txt`, `tui/mash.txt` | `tour_win`, `mash_2` (both profiles): a `gameover 0` in a tour now opens the drydock instead of the Hall of Fame | 1 |
| `wasm/sessions.txt` | `smart-11-normal`, `smart-4242-casual`, `smart-4242-normal`, `smart-8-normal`, `smart-77-casual`: the final kill now ends with "FEDERATION MISSION ACCOMPLISHED" | 5 |
| `cli/cli.txt` | the 4 `--version` cases (7, 8, 9, 14; now placeholders), the 6 `--classic` banner cases (44-47, 55, 56) and the 3 placeholder-message cases (48, 49, 111) | 7 |

All 66 games without adversaries, all 48 scenario games and the other 49
browser sessions in `engine/games.txt`, the other 52 console sessions, the
other 174 browser sessions in `wasm/sessions.txt`, and all component,
toolkit, theme, audio and formatter references are unchanged.
