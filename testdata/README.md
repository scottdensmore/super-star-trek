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

## Second round of fixes

These fixes change behaviour that the first round still kept from Go, so
more transcripts moved; each case below lost its Go byte-parity on purpose.
Regression tests are in `crates/sst-engine/tests/bug_fixes.rs` unless noted.

8. **Saves keep the Klingons in the current quadrant (save format 3).** The
   legacy Klingon list (`QuadrantState.klingons`) was tagged `json:"-"` and
   the loaders rebuilt it from `Enemies`, which is empty in a game without
   adversaries: after a load the quadrant's Klingons sat on the grid, but
   phasers reported "no enemy ships", any torpedo hit destroyed one outright
   and they never fired. (With adversaries the rebuild also discarded
   damage not yet copied to `Enemies`.) The list is now saved as
   `CurrentQuad.klingons` (omitted when empty) and envelopes are written as
   version 3; a version-3 load keeps the list exactly as saved. Older saves
   (version-2 envelopes, bare legacy saves, browser sessions) are migrated
   on load by `migrate_quadrant_klingons`: Klingons recorded in `Enemies`
   are taken from there, and every other Klingon or commander on the grid
   gets the entry a fresh quadrant entry would give it (next free ID,
   `300 + 150 * rand + 25 * skill` energy, `950 + 400 * rand + 50 * skill`
   for a commander, drawn from the reseeded PRNG). Their battle damage was
   never saved and cannot be recovered. `testdata/saves/` holds old saves
   that must keep loading: the `go-*` files are byte-for-byte Go output (the
   two legacy files are the hex dumps recorded in `engine/save.txt`; the
   envelope's hash equals the recorded Go `envhash`), the `prev-*` files were
   written by the previous release (whose encoders matched Go byte for
   byte). The engine parity harnesses hash a state's JSON without the new
   field (the list was already hashed on its own), so a case moved only
   where a save's bytes or a loaded list actually differ.
9. **The browser loader runs the terminal loader's fix-ups.** `Session.Load`
   unmarshalled the state and stopped there (the port already reseeded a
   nil PRNG to avoid Go's panic). It now goes through `decode_load_game`:
   default rules for a save without a profile, the PRNG reseeded from the
   stardate, and the Klingon migration above, so a browser load and a
   terminal load of the same file give the same state
   (`crates/sst-wasm/tests/go_tests.rs`).

10. **Focused phasers hit Klingons in every game.** `ActionPhaserDirect`
    (`pha E r,c`) looked for its target only in `Enemies`, which is empty
    without adversaries, so focused phasers at a Klingon drained the energy
    and did nothing. The target is now any visible ship at the sector: a
    Klingon on the legacy list (damage and kills are applied there and copied
    to `Enemies`), or a Romulan or Tholian. Every kill clears the sector,
    updates the counters and the galaxy chart (Go never decremented the chart
    for a focused Romulan or Tholian kill, so the ship came back when the
    quadrant was re-entered) and runs the victory check.
11. **Spread phasers hit Romulans and Tholians.** `ActionFirePhasers` only
    divided its energy between the legacy Klingon list, so Romulans and
    Tholians were immune to spread fire, and phasers refused to fire at all
    ("no enemy ships in quadrant") once only they were left. The energy is
    now split evenly between every uncloaked Klingon, Romulan and Tholian and
    each share is applied with the classic rules (divided by distance,
    shields first, shields down in a nebula). `PhaserHit` gained a `Faction`
    field (omitted for Klingons, so a Klingon hit encodes as before) and the
    console and browser name the ship that was hit. Cloaked ships are not
    targeted (Go hit and decloaked cloaked commanders); with only a cloaked
    commander in the quadrant the refusal reads "phasers cannot lock on to a
    cloaked vessel". A manual allocation, which names Klingons by ID, still
    reaches cloaked Klingons as in Go. Balance: see the autoplay numbers in
    the commit message (`cargo run --release -p sst-engine --example
    autoplay`).
12. **No weapon locks on to a cloaked ship.** Focused phasers hit cloaked
    Romulans although target lock refused cloaked ships, and target lock
    (`ActionTorpedoDirect`) itself only checked cloaked Klingons. Both now
    refuse any cloaked ship at the target sector with "TARGET LOCK FAILED:
    CLOAKED VESSEL" before spending energy or a torpedo; blind torpedo fire
    along a line still hits a cloaked ship (fix 2).

13. **Torpedo hits report the damage actually dealt.** A torpedo hit that
    did not destroy its target reported the full 500 (Go never subtracted
    what the target's shields absorbed). `TorpedoHit.Damage` is now the
    damage that got through (for a kill, the energy the ship had left) and
    the new `ShieldDamage` field (omitted when 0) what the shields took; the
    console ("Hit on [r,c]: 100 units damage (shields absorbed 400)"), the
    browser and the teletype stream show both, and a hit absorbed entirely
    by shields still sounds the explosion. The browser's torpedo lines also
    name the target instead of calling every ship (and a starbase) a
    Klingon warship, and the browser event transcript gained a record mode
    (`SST_PARITY_RECORD=/tmp/events.txt cargo test --release -p sst-wasm
    --test parity parity_combat_events`).

14. **`--classic=false` launches the bridge console.** `isClassic` sent
    every `--classic=...` argument down the teletype path, which then printed
    "Teletype mode is off - run sst without --classic ..." and exited. The
    last `--classic` flag now decides, read like Go's `strconv.ParseBool`
    (`false`, `0`, `f`, ... launch the console; an invalid value stays on the
    teletype path, which reports it) (`crates/sst/tests/main_test.rs`).

Re-recorded cases:

| Transcript | Cases | Cause |
| :--- | :--- | :--- |
| `engine/save.txt` | every `envhash` and envelope dump (`version` is now 3); the `compact`/`legacy` bytes and `load`/`loadtour` hashes of the three sources with Klingons in the quadrant (Kobayashi Maru, Mutara Nebula, tour) | 8 |
| `wasm/sessions.txt` | the `save` output of 58 sessions that save with Klingons in the quadrant (`random-*`, `scenario-kobayashi-maru-1`, `scenario-mutara-4/5`, `smart-*`, `oddload-*`); in 9 of them (`random-2-custom`, `random-7-casual`, `random-7-expert`, `random-42-hardcore`, `random-777-casual`, `random-2024-hardcore`, `random--12345-casual`, `random-1099511627776-expert`, `random-1099511627776-custom`) the commands after a `load` also changed, because the loaded Klingons are still there | 8, 9 |
| `engine/games.txt` | 115 of the 130 TUI games: 37 of the 41 without adversaries or cloaking (0, 2, 6, 8, 9, 14, 15, 23, 24, 27, 30, 33, 38, 39, 41, 42, 48, 54, 56, 63, 65, 71, 80, 81, 86, 89, 95, 96, 104, 105, 107, 110, 113, 119, 120, 128, 129), 22 of the 25 with cloaking only (3, 11, 17, 26, 32, 47, 50, 51, 57, 59, 62, 66, 72, 74, 75, 83, 87, 90, 98, 99, 114, 123), 20 of the 25 with adversaries (1, 12, 16, 25, 31, 37, 40, 46, 49, 55, 70, 73, 77, 79, 88, 92, 94, 117, 118, 121) and 36 of the 39 with both (5, 7, 10, 19-22, 28, 29, 34, 36, 43-45, 53, 58, 60, 68, 69, 76, 84, 85, 91, 93, 97, 100, 101, 106, 108, 109, 112, 115, 116, 124, 125, 127) | 10-12 |
| `engine/games.txt` | 16 of the 48 scenario games (130, 131, 133, 135, 137, 139, 140, 145, 146, 149, 151, 152, 155, 157, 160, 161), 23 of the 26 tour games (180-198, 200-203) and 47 of the 60 browser sessions (204, 205, 208-211, 213, 214, 216-218, 220-223, 225-237, 240-242, 244, 245, 248, 249, 251-262) | 10-12 |
| `tui/scenarios.txt` | `scenario_mutara-nebula` (both profiles): spread phasers no longer hit the cloaked commander | 11 |
| `wasm/sessions.txt` | `fixed-casual`, `fixed-normal`, `fixed-hardcore`, `random-7-expert`, `random-42-normal`, `random-777-emeritus`, `random--1-expert`, `scenario-mutara-2`, `smart-scenario-mutara-0` | 10-12 |
| `engine/games.txt` | 51 TUI games with adversaries (1, 4, 5, 7, 10, 12, 13, 16, 19, 20, 22, 25, 29, 31, 34, 36, 37, 40, 43-46, 49, 52, 53, 55, 60, 61, 67-70, 73, 76, 77, 79, 82, 84, 91-94, 97, 101, 103, 109, 115, 117, 118, 124, 127), 24 of the 26 tour games (178-185, 187, 189-203) and 9 browser sessions (208, 209, 216, 232, 233, 241, 248, 249, 257): hits on shielded ships | 13 |
| `wasm/sessions.txt` | `random-2-expert`, `random-7-emeritus`, `smart-scenario-siege-1` | 13 |
| `wasm/events.txt` | 24 of the 600 synthetic event lists (torpedo hits on non-Klingon targets) | 13 |
| `cli/cli.txt` | 48 `classic-eq-false` (now a console launch) and 49 `classic-eq-false-seed` (now the seeded console launch, the same facts and view as 58 `tui-seed`) | 14 |
