# AGENTS.md

Shared project instructions for every coding agent. `CLAUDE.md` imports this
file, and `GEMINI.md` points to it; keep project rules here so all tools use
the same source of truth.

## Project overview

Super Star Trek is a classic terminal space-strategy game written in C17 and
built with CMake. The shipped executable is `sst`; player documentation is in
`sst.doc`.

- **UI domain:** Terminal/CLI. Plain line-oriented mode and the optional
  ncurses full-screen mode (`sst -t`) are both player-facing.
- **Supported hosts:** Linux and macOS. There is currently no working Windows
  build because the game depends on ncurses.
- **Base branch:** `main`.

## Repo Map

| Area | Location and ownership |
|---|---|
| Entry point and global storage | `sst.c`; it defines `INCLUDED` before `sst.h` to instantiate shared globals |
| Shared declarations and game state | `sst.h` |
| Game logic | Root C files such as `ai.c`, `battle.c`, `events.c`, `moving.c`, `reports.c`, `rules.c`, and `setup.c` |
| Full-screen display | `tui.c`, `tui.h`, and `tuifmt.c` |
| Platform boundary | `osx.c` and `osx.h` |
| Player documentation | `sst.doc` |
| Tests and recordings | `tests/`; compiled tests are `tests/test_*.c`, shell journeys are `tests/*.sh`, and golden fixtures are `tests/golden/*.txt` |
| Build and test definition | `CMakeLists.txt` and `CMakePresets.json` |
| CI gate | `.github/workflows/ci.yml` |
| Generated output — never edit | `build/<preset>/`; regenerate with the matching CMake preset |

The build declares no generated or vendored source directory.

## Development Commands

`CMakePresets.json` is the single source of truth. Run commands from the
repository root and use a preset; do not restate its flags by configuring
CMake by hand.

| Purpose | Command | A green result proves |
|---|---|---|
| Configure everyday Debug build | `cmake --preset debug` | CMake generated `build/debug/` with the declared dependencies |
| Build everyday Debug binary | `cmake --build --preset debug` | `build/debug/sst` compiled; warnings are visible but not fatal |
| Run the game | `./build/debug/sst` or `./build/debug/sst -t` | The selected display starts interactively |
| Run all debug tests | `ctest --preset debug` | The complete debug test suite passed |
| Run one focused test | `ctest --preset debug -R '^<test-name>$'` | The named registered test passed; the anchored filter cannot silently select neighbors |
| CI Debug gate | `cmake --preset ci-debug && cmake --build --preset ci-debug && ctest --preset ci-debug` | Debug compiled with warnings fatal and the complete Debug suite passed, apart from explicitly reported platform skips |
| CI Release gate | `cmake --preset ci-release && cmake --build --preset ci-release && ctest --preset ci-release` | Optimized Release compiled with warnings fatal and the complete Release suite passed, apart from explicitly reported platform skips |

Both CI gates must pass. CI itself runs that pair on Linux and macOS.
A passing local gate does not cover a test reported as skipped; name skips
rather than folding them into a pass.

For a plain-mode smoke journey:

```sh
printf 'regular\nshort\nnovice\nxyz\nsrscan\nquit\nn\n' | ./build/debug/sst
```

For TUI work, `tests/tui.sh build/debug/sst Debug` is the focused real-PTY
journey. It covers the required 80x24 and 72x24 layouts and uses an isolated
tmux socket.

## Local Setup

- CMake 3.21 or newer, a C17 compiler, and ncurses are required.
  Debian/Ubuntu package the latter as `libncurses-dev`; macOS already provides
  it for the supported build.
- `tmux` is required to exercise curses. Without it, the local `tui` test
  skips; Linux CI treats its absence as a failure.
- `gcc -fanalyzer` enables the `analyze` test. It may skip when unavailable
  and on macOS unless an analyzer-capable compiler was selected deliberately.
- `actionlint` 1.7.12 plus `shellcheck` enables the workflow lint locally.
  Linux CI installs both and treats missing coverage as a failure.
- No services, environment files, credentials, seed step, or package install
  are required.

## Architecture & Conventions

- Global game state uses the `EXTERN`/`INCLUDED` mechanism in `sst.h`.
  Exactly one translation unit per binary defines `INCLUDED` before including
  the header: `sst.c` for the game and the owning `tests/test_*.c` file for a
  test binary that does not link `sst.c`.
- Display state (`tui_active`, `tui_ingame`) is plain `extern` state in
  `tui.h`, defined in `tui.c`. Add display flags there, not in `sst.h`.
- `osx.c` cannot include `sst.h`: the game declaration `pause(int)` collides
  with POSIX `pause(void)`. Functions provided by `osx.c` belong in `osx.h`;
  functions it calls come from `tui.h`. This also keeps
  `-Wmissing-prototypes` effective.
- All game output goes through `prout`, `proutn`, `proutf`, or `prouts`; all
  game input goes through `scan()`, `getch()`, or `readinput()`. Raw stdio
  bypasses the full-screen display.
- `tuifmt.c` panel formatters mirror `srscan()` in `reports.c`, including
  sensor-damage masking and `-f` coordinate transposition (x is the column).
  Keep `tests/test_tuifmt.c` synchronized.
- Comments that justify code with a number, bound, or reachability claim must
  cite the source or measurement that establishes it.

## Gotchas & Troubleshooting

- `CMakeLists.txt` reads `CMAKE_BUILD_TYPE` at configure time, so a bare
  `cmake -S . -B build` is not a supported substitute for a preset.
- Debug adds `-DDEBUG`; every build adds `SCORE`, `CAPTURE`, and `CLOAKING`.
  The `ci-*` presets also enable `-Werror`. Debug and Release therefore need
  separate verification.
- Plain mode must not change accidentally during TUI work. Run the scripted
  plain journey above as well as the focused TUI journey.
- Never drive TUI checks on the default tmux server and never issue a bare
  `tmux kill-server`. Use `tests/tui.sh`; for manual work use `-f /dev/null`
  and a dedicated `-L` socket, and clean up only that socket.
- Game text must not end in carriage return. `sst.doc` uses CRLF, so strip
  both line endings when reading it. The deliberate carriage return in
  `pause()` is the sole exception.
- A `golden` failure means game output or arithmetic changed. Explain the
  fixture diff; never use `tests/golden.sh <sst> --update` merely to make the
  gate green.
- Full-game mutation proofs must use a fresh copy of the current tree without
  `build/`, configure inside that copy, and run its script from that copy.
  Copied CMake caches and scripts invoked from the original tree can test the
  wrong source while appearing green.
- The `tui`, `analyze`, `workflow`, and `lineendings` tests may skip locally
  when their tools are absent -- `lineendings` needs `git` and a work tree.
  A skip is an environment gap, not a passing check.

## Verification Map

Use this map after a fix to select the affected checks. Both complete Debug and
Release CI gates must still run before completing changes.

| A fix touches | Focused check before the complete gate |
|---|---|
| `tuifmt.c`, `tui.h`, or panel formatting | `ctest --preset debug -R '^tuifmt$'` and `ctest --preset debug -R '^lineendings$'`; also run the plain journey and `tests/tui.sh build/debug/sst Debug` when player-visible |
| Other TUI behavior in `tui.c` | `ctest --preset debug -R '^tui$'` and `ctest --preset debug -R '^lineendings$'`, plus the plain journey |
| Rules in `rules.c` or `rules.h` | `ctest --preset debug -R '^rules$'` and `ctest --preset debug -R '^lineendings$'` |
| Other game C or header files | The narrowest registered journey or compiled test that reaches the behavior, and `ctest --preset debug -R '^lineendings$'`; then both complete gates |
| `sst.doc` or help behavior | `ctest --preset debug -R '^help$'` and `ctest --preset debug -R '^lineendings$'` |
| Golden fixtures or output arithmetic | `ctest --preset debug -R '^golden$'` and inspect every fixture diff |
| Documentation or agent instructions (`README.md`, `AGENTS.md`, etc.) | `ctest --preset debug -R '^lineendings$'` |
| `.github/workflows/**` or `tests/workflow.sh` | `ctest --preset debug -R '^workflow$'`; absence of `actionlint`/`shellcheck` is a skip (77) locally |
| `CMakeLists.txt` or `CMakePresets.json` | Both complete CI gates |
| `.gitattributes` or `tests/lineendings.sh` | `ctest --preset debug -R '^lineendings$'`; without `git` it exits 77 (skipped), not a pass |
| Any path not listed | Both complete CI gates |

**`lineendings` is named in every source row, not only its own.** Rows match
first to last, so a row selecting the guard when the *guard* changes says
nothing about the case it exists for: a line-ending flip in `sst.doc` matches
the help row, in `tui.c` the TUI row, and neither would have run it. The five
source rows above name it because a flip is invisible to every other check --
the compiler, the journeys, the golden recordings and `analyze` all pass -- so
the one test that can see it has to be selected by the files it guards, not by
itself. Adding it to a row costs about half a second: a 30-file shell loop,
observed between 0.48s and 0.56s across both presets. The rows are
alternatives, so following one runs it once, not five times.

Each names it as a second `-R` rather than one alternation, because a `|` in a
table cell has to be escaped and `-R '^(tui\|lineendings)$'` is what a reader
copies out: measured, that exits 8 with `No tests were found!!!`, since the
backslash is literal to ctest. A command in this table has to survive being
pasted.

## Git

- **Branching:** Never commit directly to `main`. Create a dedicated branch
  from `origin/main` (e.g. `<owner>/<type>/<short-description>`).
- **Self-merges:** Self-merges are allowed in this repository. An agent may
  squash-merge its own pull request without requesting separate approval once
  all of these are true:
  - the pull request head is the exact locally reviewed and verified commit;
  - GitHub reports the pull request clean and mergeable;
  - every required check has completed successfully;
  - no unresolved review threads or required changes remain; and
  - a final readback confirms the base branch, head SHA, and clean local
    worktree.
- **Merge policy:** Squash merge is required to maintain a linear commit graph;
  merge commits and rebase merges are disabled. Delete the merged branch after
  merging. Never bypass a pending or failing check, merge a different head than
  the one reviewed, or treat approval for one pull request as approval for another.
