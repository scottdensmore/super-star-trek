# Sub-Project 4 Implementation Plan: Issue Hygiene, CI & Test Suite Hardening

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete Sub-Project 4 by hardening test scripts, integrating shellcheck, verifying trap consistency, tuning static analysis documentation, hardening TUI wrapping-answer assertions, and triaging all obsolete issues.

**Architecture:** Extend `tests/workflow.sh` to lint shell scripts and assert trap consistency; make byte counting locale-independent in `tests/lineendings.sh`; harden assertion verification in `tests/tui.sh`; calibrate static analysis comments in `tests/analyze.sh`; align `CMakeLists.txt` comments; triage 18 obsolete issues.

**Tech Stack:** POSIX sh, CMake, ctest, shellcheck, GCC -fanalyzer, tmux, C17.

**Spec:** `docs/superpowers/specs/2026-09-10-issue-hygiene-ci-hardening-design.md`

## Global Constraints

- Never run a bare `tmux kill-server` or `tmux kill-session`. Any tmux commands must specify `-L <socket>`.
- Remote GitHub Actions are disabled; all CI gates (`ci-debug`, `ci-release`, `lineendings`) must pass locally.
- Zero changes to core combat, damage, or stardate game arithmetic.
- Preserve line endings: LF on all modified files (`tests/*.sh`, `CMakeLists.txt`, `README.md`).
- Superpowers directories (`.superpowers/` and `docs/superpowers/`) are git-ignored and must NOT be committed to git.

---

### Task 1: Portable Byte Counting & Invalid UTF-8 Self-Test in `tests/lineendings.sh` (#205)

**Files:**
- Modify: `tests/lineendings.sh`

**Interfaces:**
- Consumes: `LC_ALL=C tr -cd`
- Produces: Portable byte-level CR and LF counting invariant across GNU and BSD `tr`.

- [ ] **Step 1: Write failing self-test for invalid UTF-8 counting**
Add a self-test check in `tests/lineendings.sh` that pipes a synthetic invalid UTF-8 sequence (`printf '\xff\xfe\r\n'`) into the counting logic and asserts it counts exactly 1 CR and 1 LF. Run it under a strict UTF-8 locale to verify behavior.

- [ ] **Step 2: Update byte-counting commands in `tests/lineendings.sh`**
Prefix the counting commands with `LC_ALL=C`:
```sh
cr=$(LC_ALL=C tr -cd '\r' < "$path" | wc -c | tr -d ' ')
lf=$(LC_ALL=C tr -cd '\n' < "$path" | wc -c | tr -d ' ')
```

- [ ] **Step 3: Run lineendings test**
Run: `ctest --preset debug -R '^lineendings$'`
Expected: 100% tests passed.

- [ ] **Step 4: Commit**
```bash
git add tests/lineendings.sh
git commit -m "fix(test): make lineendings byte counting portable with LC_ALL=C and test invalid utf8 (#205)"
```

---

### Task 2: Shell Script Linting & Verification with `shellcheck` (#106)

**Files:**
- Modify: `tests/lineendings.sh:133`
- Modify: `tests/tui.sh:662`
- Modify: `tests/workflow.sh`

**Interfaces:**
- Consumes: `/usr/bin/shellcheck`
- Produces: Clean `shellcheck -s sh tests/*.sh` verification in `tests/workflow.sh`.

- [ ] **Step 1: Fix existing shellcheck warnings**
In `tests/lineendings.sh:133`:
Add `# shellcheck disable=SC2086` above `for f in $(git -C "$src" ls-files -- $globs); do`.
In `tests/tui.sh:662`:
Replace `[ "${4:-}" = "" -a "$#" -ge 4 ]` with `[ "${4:-}" = "" ] && [ "$#" -ge 4 ]`.

- [ ] **Step 2: Add shellcheck execution to `tests/workflow.sh`**
In `tests/workflow.sh`, add a check that lints all `tests/*.sh`:
```sh
if command -v shellcheck >/dev/null 2>&1; then
    if ! shellcheck -s sh "$root"/tests/*.sh; then
        echo "FAIL: shellcheck faulted tests/*.sh" >&2
        exit 1
    fi
elif [ -n "${CI:-}" ] && [ "$(uname -s)" = Linux ]; then
    echo "FAIL: no shellcheck on Linux CI" >&2
    exit 1
fi
```

- [ ] **Step 3: Run workflow test**
Run: `ctest --preset debug -R '^workflow$'`
Expected: PASS (or skip if actionlint absent locally). Run `shellcheck -s sh tests/*.sh` directly to verify exit code 0.

- [ ] **Step 4: Commit**
```bash
git add tests/lineendings.sh tests/tui.sh tests/workflow.sh
git commit -m "ci(lint): fix shellcheck warnings and wire shellcheck into workflow test (#106)"
```

---

### Task 3: Test Trap Set Consistency Verification (#132)

**Files:**
- Modify: `tests/workflow.sh`

**Interfaces:**
- Consumes: Structural check over `tests/*.sh`
- Produces: Enforcement that every test script setting traps registers all 5 signals (`EXIT`, `INT`, `TERM`, `HUP`, `PIPE`).

- [ ] **Step 1: Add trap set consistency check in `tests/workflow.sh`**
In `tests/workflow.sh`, add a verification loop:
For each `f` in `tests/*.sh`:
If `grep -q '^trap ' "$f"`; then verify that all 5 signals appear in traps (`EXIT`, `INT`, `TERM`, `HUP`, `PIPE`).
If any is missing, report `FAIL: $f is missing required signal traps` and exit 1.

- [ ] **Step 2: Run verification**
Run: `ctest --preset debug -R '^workflow$'` or `sh tests/workflow.sh .`
Expected: PASS.

- [ ] **Step 3: Commit**
```bash
git add tests/workflow.sh
git commit -m "test(ci): enforce complete 5-signal trap sets across test scripts (#132)"
```

---

### Task 4: Static Analysis Complexity Documentation (#171)

**Files:**
- Modify: `tests/analyze.sh`

**Interfaces:**
- Consumes: GCC `-fanalyzer` dial calibration from PR #173
- Produces: Explicit documentation of non-fatal `-Wanalyzer-too-complex` rationale.

- [ ] **Step 1: Update comments in `tests/analyze.sh`**
Clarify the rationale in `tests/analyze.sh`:
Document why `-Wanalyzer-too-complex` is intentionally kept non-fatal (diagnostic counts increase with search depth), and how `--param=analyzer-bb-explosion-factor=30` provides verified coverage across `sst.c` and `battle.c`.

- [ ] **Step 2: Run analyze test**
Run: `cmake --build --preset debug && ctest --preset debug -R '^analyze$'`
Expected: PASS (`analyze OK`).

- [ ] **Step 3: Commit**
```bash
git add tests/analyze.sh
git commit -m "docs(analyze): document non-fatal -Wanalyzer-too-complex behavior and explosion factor (#171)"
```

---

### Task 5: Multi-line Wrapped Answer Hardening in `tests/tui.sh` (#115)

**Files:**
- Modify: `tests/tui.sh`

**Interfaces:**
- Consumes: Real PTY window resizes in `tests/tui.sh`
- Produces: Assertions that typed multi-line answers survive resize operations.

- [ ] **Step 1: Update wrapped answer test arm in `tests/tui.sh`**
In `tests/tui.sh:2697`:
Change the typed answer string from `regularbbb...` to use unique character `z` (e.g. `regularzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz`).
In `tests/tui.sh:2708-2719`:
Assert that `screen | tr -cd 'z' | wc -c` matches the expected number of `z`s (58) across each resize step (`72x30`, `72x36`, `72x42`).
In `tests/tui.sh:2843` ("wrapping answer to an ended prompt"):
Assert that the typed answer survives cleanly across 80x23/24 resize steps.

- [ ] **Step 2: Run tui test**
Run: `cmake --build --preset debug && ctest --preset debug -R '^tui$'`
Expected: PASS.

- [ ] **Step 3: Commit**
```bash
git add tests/tui.sh
git commit -m "test(tui): assert typed answer characters survive across resize in wrapping answer test (#115)"
```

---

### Task 6: Documentation and Comment Alignment (#192, #130, #127)

**Files:**
- Modify: `tests/tui.sh:slept_drag`
- Modify: `CMakeLists.txt:68-74`
- Modify: `CMakeLists.txt:112-114`

**Interfaces:**
- Consumes: Accurate rationale from issues #192, #130, #127
- Produces: Clean, accurate code comments.

- [ ] **Step 1: Update comments**
- In `tests/tui.sh:slept_drag`: explain POSIX orphaned process group cleanup and that `stopped_pid` is platform insurance (#192).
- In `CMakeLists.txt:68-74`: update `journey` `WORKING_DIRECTORY` rationale (#130).
- In `CMakeLists.txt:112-114`: update `tournament` timing description (~50-70s) (#127).

- [ ] **Step 2: Run verification**
Run: `ctest --preset debug -R '^(journey|tournament|lineendings)$'`
Expected: PASS.

- [ ] **Step 3: Commit**
```bash
git add tests/tui.sh CMakeLists.txt
git commit -m "docs: correct comments for slept_drag, journey working directory, and tournament runtime (#192, #130, #127)"
```

---

### Task 7: Full Local CI Verification & Obsolete Issue Triage

**Files:**
- None (verification and issue triage)

**Interfaces:**
- Consumes: GitHub CLI `gh issue close`
- Produces: 100% CI pass and closure of all 18 obsolete/investigated issues (#197, #196, #188, #145, #138, #110, #109, #93, #86, #85, #84, #81, #65, #126, #105, #129, #98, #184).

- [ ] **Step 1: Run complete CI suites**
Run:
```bash
cmake --preset ci-debug && ctest --preset ci-debug
cmake --preset ci-release && ctest --preset ci-release
ctest --preset debug -R '^lineendings$'
ctest --preset debug -R '^golden$'
```
Expected: 100% tests passed.

- [ ] **Step 2: Triage and close obsolete issues via gh CLI**
Close issues #197, #196, #188, #145, #138, #110, #109, #93, #86, #85, #84, #81, #65, #126, #105, #129, #98, #184 with explicit explanations.
