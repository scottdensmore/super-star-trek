# Sub-Project 4 Design: Issue Hygiene, CI & Test Suite Hardening

## Overview & Goals

This document specifies the technical design for **Sub-Project 4: Issue Hygiene, CI & Test Suite Hardening** (tracking issue [#212](https://github.com/scottdensmore/super-star-trek/issues/212)).
Sub-Project 4 is the final phase of the Super Star Trek modernization and hardening roadmap. Its objectives are:
1. Triage and resolve all 18 obsolete or previously investigated issues relating to retired agent prompts and instruction scripts.
2. Integrate `shellcheck` into the test/CI verification suite for all `tests/*.sh` scripts ([#106](https://github.com/scottdensmore/super-star-trek/issues/106)).
3. Guarantee portable byte counting across platforms in `tests/lineendings.sh` ([#205](https://github.com/scottdensmore/super-star-trek/issues/205)).
4. Enforce consistent signal trap sets (`EXIT`, `INT`, `TERM`, `HUP`, `PIPE`) across all test harness scripts ([#132](https://github.com/scottdensmore/super-star-trek/issues/132)).
5. Finalize `gcc -fanalyzer` static analysis complexity handling and documentation ([#171](https://github.com/scottdensmore/super-star-trek/issues/171)).
6. Harden multi-line wrapped answer assertions in `tests/tui.sh` ([#115](https://github.com/scottdensmore/super-star-trek/issues/115)).
7. Align comments and documentation in `tests/tui.sh` and `CMakeLists.txt` ([#192](https://github.com/scottdensmore/super-star-trek/issues/192), [#130](https://github.com/scottdensmore/super-star-trek/issues/130), [#127](https://github.com/scottdensmore/super-star-trek/issues/127)).

Upon completion of Sub-Project 4, every open issue in the repository will be closed.

---

## 1. Obsolete Issue Triage

The following 18 issues will be closed on GitHub with clear rationales citing the relevant commits:

### Agent-Workflow & Retired Prompt Issues (13 issues)
- **#197**: `verifier.md tells the verifier to re-read files it just wrote` — obsolete following prompt retirement in Sub-Project 1 (#208).
- **#196**: `Nothing checks that a workflow commit message mentions an issue` — obsolete following prompt retirement (#208).
- **#188**: `AGENTS.md says a full ctest is about 2 minutes; it is about 7` — `AGENTS.md` was rewritten in #208 and no longer makes this outdated claim.
- **#145**: `AGENTS.md warns about tmux kill-server; the harness does not run it` — warning in `AGENTS.md` is now concise and accurate.
- **#138**: `AGENTS.md calls the message window lines 10..LINES-1` — resolved by removing hardcoded layout tables from `AGENTS.md` in #208.
- **#110**: `No floor under "Answering a finding is always more code"` — obsolete following prompt retirement (#208).
- **#109**: `Nothing checks that a quoted "Section" pointer has a matching heading` — obsolete following removal of rigid instruction validation (#202).
- **#93**: `A commit body that names an issue without Closes leaves it open` — obsolete prompt guidance.
- **#86**: `code-review's brief quotes sst.doc page 26` — obsolete code-review prompt; sst.doc was paginated in #121.
- **#85**: `The parts of a session's injected prompt do not say which file they came from` — obsolete prompt injection harness.
- **#84**: `ui-review's description points into the retired prompt directory` — obsolete prompt directory.
- **#81**: `Nothing checks that the agent definitions parse as YAML frontmatter` — obsolete agent definitions directory.
- **#65**: `Nothing checks that AGENTS.md's workflow matches the prompt templates` — obsolete prompt templates.

### `tests/instructions.sh` Retired Issues (4 issues)
- **#126**, **#105**, **#129**, **#98**: All related to regex matching, pointer bounds, and unvalidated checks in `tests/instructions.sh`. Commit `13a7fb8` (#202) retired `tests/instructions.sh` completely because instruction files should not be policed by test suites.

### Layout Invariant Investigation (1 issue)
- **#184**: `sync_size()'s repaint has no test asserting it runs doupdate()`. Investigated in detail by repository owner; confirmed that comments and code in `tui.c` and `tests/tui.sh` accurately document the invariants.

---

## 2. Technical Design & Component Changes

### Component 1: Portable Byte Counting in `tests/lineendings.sh` (#205)
- **Problem**: `tr -cd '\r'` and `tr -cd '\n'` in `tests/lineendings.sh` are locale-sensitive in BSD `tr` under UTF-8 locales when encountering invalid UTF-8 byte sequences.
- **Changes**:
  - In `tests/lineendings.sh:42-43`:
    ```sh
    cr=$(LC_ALL=C tr -cd '\r' < "$path" | wc -c | tr -d ' ')
    lf=$(LC_ALL=C tr -cd '\n' < "$path" | wc -c | tr -d ' ')
    ```
  - Add an internal self-test check in `tests/lineendings.sh` that validates a synthetic byte sequence containing invalid UTF-8 (e.g., `printf '\xff\xfe\r\n'`) is correctly counted as 1 CR and 1 LF.
- **Files**: `tests/lineendings.sh`

### Component 2: Shell Script Linting & Verification (`shellcheck`) (#106)
- **Problem**: Shell scripts under `tests/` are not linted in CI or local ctest runs.
- **Changes**:
  - Fix existing shellcheck findings:
    - In `tests/lineendings.sh:133`: add `# shellcheck disable=SC2086` to document intentional word splitting on `$globs`.
    - In `tests/tui.sh:662`: change `[ "${4:-}" = "" -a "$#" -ge 4 ]` to `[ "${4:-}" = "" ] && [ "$#" -ge 4 ]` (SC2166).
  - In `tests/workflow.sh`:
    - Add a step running `shellcheck -s sh "$root"/tests/*.sh`.
    - Handle tool availability cleanly: if `shellcheck` is absent, skip (77) locally or fail in Linux CI, matching the existing `actionlint` convention.
- **Files**: `tests/lineendings.sh`, `tests/tui.sh`, `tests/workflow.sh`

### Component 3: Trap Set Consistency Verification (#132)
- **Problem**: Test scripts creating temporary directories or background processes must reliably trap all exit and termination signals (`EXIT`, `INT`, `TERM`, `HUP`, `PIPE`) to prevent leaking processes or temp state.
- **Changes**:
  - Audit all `tests/*.sh`. All 7 scripts with temporary state (`analyze.sh`, `eof.sh`, `golden.sh`, `help.sh`, `journey.sh`, `tournament.sh`, `tui.sh`) already register all 5 traps.
  - In `tests/workflow.sh`: add a structural verification loop asserting that every `tests/*.sh` script creating temporary directories or setting traps registers all 5 signals (`EXIT`, `INT`, `TERM`, `HUP`, `PIPE`).
- **Files**: `tests/workflow.sh`

### Component 4: Static Analysis (`gcc -fanalyzer`) Complexity Tuning (#171)
- **Problem**: GCC `-fanalyzer` emits `-Wanalyzer-too-complex` warnings when the explosion factor increases, which cannot be made fatal.
- **Changes**:
  - Update `tests/analyze.sh` documentation to clearly specify that `-Wanalyzer-too-complex` is intentionally kept non-fatal and that `bb-explosion-factor=30` represents the calibrated depth providing full analysis coverage across `sst.c` and `battle.c`.
- **Files**: `tests/analyze.sh`

### Component 5: Wrapping Answer Verification in `tests/tui.sh` (#115)
- **Problem**: The multi-line wrapped answer test arms in `tests/tui.sh` asserted that the prompt question remained on screen once, but omitted asserting the presence of the typed answer because `b` appeared in the startup banner.
- **Changes**:
  - In `tests/tui.sh:2697`: change the test typed answer to use a unique character (e.g. `z` repeated 58 times: `regularzzzz...`).
  - In `tests/tui.sh:2708-2719`: add an assertion checking that the run of `z`s appears on screen across each resize step (`72x30`, `72x36`, `72x42`).
  - In `tests/tui.sh:2843` ("wrapping answer to an ended prompt"): apply the same pattern to assert the typed answer survives.
- **Files**: `tests/tui.sh`

### Component 6: Comments & Documentation Polish (#192, #130, #127)
- **Changes**:
  - In `tests/tui.sh:slept_drag()`: update comment to reflect that on Linux, POSIX orphaned process group rules reap stopped processes via SIGHUP/SIGCONT, and the `stopped_pid` registration is safety insurance for other platforms (e.g. macOS) (#192).
  - In `CMakeLists.txt:68-74`: correct `journey` `WORKING_DIRECTORY` comment to state that running from the source tree matches standard user invocation without claiming `sst.doc` read dependency (#130).
  - In `CMakeLists.txt:112-114`: correct `tournament` test timing comment to state realistic duration (~50-70s) without claiming it is the slowest test by four times (#127).
- **Files**: `tests/tui.sh`, `CMakeLists.txt`

---

## 3. Verification Strategy & Invariants

1. **Local CI Gates**:
   - `cmake --preset ci-debug && ctest --preset ci-debug` (all 13 tests passing/skipped as designed)
   - `cmake --preset ci-release && ctest --preset ci-release`
   - `ctest --preset debug -R '^lineendings$'`
   - `ctest --preset debug -R '^golden$'`
   - `ctest --preset debug -R '^workflow$'`
2. **Line Endings Conventions**:
   - Shell scripts (`tests/*.sh`) and `CMakeLists.txt` must maintain LF line endings.
   - Core C files (`sst.c`, `sst.h`, `moving.c`, etc.) must maintain CRLF line endings.
3. **No Gameplay Math Changes**:
   - Zero changes to core combat, damage, or stardate game logic.
