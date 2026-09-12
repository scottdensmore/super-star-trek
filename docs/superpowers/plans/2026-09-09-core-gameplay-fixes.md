# Core Gameplay Fixes & Polish (Sub-Project 2) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement high-priority gameplay fixes, I/O formatting consistency (`proutfn`/`proutf`), save/freeze Spock guidance, compiled C score column assertions, and manual pagination.

**Architecture:** Introduce `proutfn()` in `sst.h`/`sst.c` for open-line formatting and make `proutf()` guarantee a newline; enhance filename validation in `setup.c` with clear Spock messages; hoist score formats in `finish.c`/`finish.h` and assert 47-column alignment in `tests/test_rules.c`, deleting `tests/score.sh`; split `MODIFICATIONS` in `sst.doc` across form feeds and renumber the Table of Contents.

**Tech Stack:** C99, CMake, CTest, Curses/TUI, POSIX shell.

**Spec:** `docs/superpowers/specs/2026-09-09-core-gameplay-fixes-design.md`

## Global Constraints
- Strictly avoid checking in superpowers workflow/planning artifacts into git (`.superpowers/` and `docs/superpowers/` are ignored in `.gitignore`).
- Preserve CRLF / LF line endings convention across files (checked by `ctest --preset debug -R '^lineendings$'`).
- Zero changes to core combat, damage, or stardate game arithmetic; golden fixture diffs must only reflect intentional prompt and spacing updates.
- All gates must pass locally: `ci-debug`, `ci-release`, and `lineendings`.

---

### Task 1: Formatted Output Architecture (`proutfn()` & `proutf()` Line Completion) (#162)

**Files:**
- Modify: `sst.h:450-460`
- Modify: `sst.c:665-675, 1070-1095`
- Test: `tests/test_tuifmt.c`

**Interfaces:**
- Produces: `void proutfn(const char *fmt, ...);`
- Modifies: `void proutf(const char *fmt, ...);` to guarantee line completion.

- [ ] **Step 1: Write failing test in `tests/test_tuifmt.c` for open vs closed line formatting**
Add test case exercising `proutfn` vs `proutf` behavior and checking that `proutfn` does not emit a trailing newline while `proutf` does.

- [ ] **Step 2: Run test to verify it fails**
Run: `ctest --preset debug -R '^tuifmt$'`
Expected: FAIL (compilation error: `proutfn` undeclared).

- [ ] **Step 3: Declare `proutfn()` in `sst.h` and implement in `sst.c`**
In `sst.h`:
```c
void proutfn(const char *fmt, ...);
```
In `sst.c`:
Implement `proutfn()` using `vsnprintf`, outputting embedded newlines via `proutn` + `skip(1)`, and printing the trailing segment with `proutn()` without `skip(1)`.
Update `proutf()` to check if the formatted string ends with `\n`; if not, invoke `skip(1)`.

- [ ] **Step 4: Fix end-of-game questions in `sst.c`**
In `sst.c:665-675`:
Ensure `Do you want your score recorded? ` and `Do you want to play again? ` are preceded by `skip(1)` and use `proutn()` with a space before `ja()`.

- [ ] **Step 5: Run tests to verify they pass**
Run: `ctest --preset debug -R '^tuifmt$'`
Expected: PASS.

- [ ] **Step 6: Commit**
```bash
git add sst.h sst.c tests/test_tuifmt.c
git commit -m "feat(io): add proutfn for open prompts and guarantee newline in proutf (#162)"
```

---

### Task 2: Freeze / Thaw Filename Guidance and Documentation (#160)

**Files:**
- Modify: `setup.c:40-55, 85-98`
- Modify: `cmdtab.c:165-175`
- Modify: `sst.doc:1100-1115, 1420-1425`
- Test: `tests/help.sh`

**Interfaces:**
- Consumes: `prout()`, `cmdtab`
- Produces: Updated error handling for `freeze` and `thaw` commands.

- [ ] **Step 1: Add test in `tests/help.sh` checking non-alphabetic freeze error**
In `tests/help.sh`, add a test case running `freeze 123` and asserting that the output contains `Spock- "Captain, file names must begin with an alphabetic letter (A-Z)."`.

- [ ] **Step 2: Run test to verify it fails**
Run: `ctest --preset debug -R '^help$'`
Expected: FAIL (output contains `Beg your pardon, Captain?` instead of Spock guidance).

- [ ] **Step 3: Implement Spock filename guidance in `setup.c`**
In `freeze()` and `thaw()`:
```c
if (key != IHALPHA) {
    prout("Spock- \"Captain, file names must begin with an alphabetic letter (A-Z).\"");
    return;
}
```

- [ ] **Step 4: Update `cmdtab.c` and `sst.doc` documentation**
In `cmdtab.c`: update `FREEZE` and `THAW` example strings.
In `sst.doc`: update `FREEZE` (line 1103) and `COMMAND SUMMARY` (line 1420) to document:
- 9-character cap
- Initial letter constraint
- Automatic `.TRK` appending only when no dot is present.

- [ ] **Step 5: Run test to verify it passes**
Run: `ctest --preset debug -R '^help$'`
Expected: PASS.

- [ ] **Step 6: Commit**
```bash
git add setup.c cmdtab.c sst.doc tests/help.sh
git commit -m "fix(save): provide clear Spock guidance on invalid filename and align sst.doc (#160)"
```

---

### Task 3: Score Sheet C-Level Verification in `test_rules.c` & Awk Retirement (#76)

**Files:**
- Modify: `finish.h`
- Modify: `finish.c:345-415`
- Modify: `tests/test_rules.c`
- Delete: `tests/score.sh`
- Modify: `CMakeLists.txt:125-135`

**Interfaces:**
- Produces: `score_line_fmt()` or exposed score format array in `finish.h`
- Consumes: `score_compute()` from `rules.h`

- [ ] **Step 1: Write failing C assertions in `tests/test_rules.c` for score column alignment**
In `tests/test_rules.c`:
Add `test_score_sheet_columns()` validating all 27 score row formats and 5 skill bonus combinations formatted via `snprintf`, asserting `strlen(buf) == 47`.

- [ ] **Step 2: Run test to verify it fails**
Run: `ctest --preset debug -R '^rules$'`
Expected: FAIL (symbols/declarations not yet linked).

- [ ] **Step 3: Expose score format structures in `finish.h` / `finish.c`**
Structure the format strings in `finish.c` and declare their test interface in `finish.h` (or a dedicated rules/scoring header) so `tests/test_rules.c` can iterate through each format string directly.

- [ ] **Step 4: Run test to verify it passes**
Run: `ctest --preset debug -R '^rules$'`
Expected: PASS.

- [ ] **Step 5: Delete `tests/score.sh` and remove from `CMakeLists.txt`**
Remove `tests/score.sh` and drop `add_test(NAME score ...)` from `CMakeLists.txt`.

- [ ] **Step 6: Verify all tests build and run**
Run: `cmake --build --preset debug && ctest --preset debug -R '^rules$'`
Expected: PASS (and `score` test is gone).

- [ ] **Step 7: Commit**
```bash
git rm tests/score.sh
git add finish.h finish.c tests/test_rules.c CMakeLists.txt
git commit -m "test(score): replace awk text-scraping score.sh with compiled C column assertions in test_rules (#76)"
```

---

### Task 4: Documentation Pagination & Table of Contents Renumbering (#121)

**Files:**
- Modify: `sst.doc:54-110, 1470-1670`

- [ ] **Step 1: Verify current page lengths with awk script**
Run: `awk '/\f/{if (prev) printf "%d lines before the break at %d\n", NR-prev, NR; prev=NR}' sst.doc`
Observe page 26 line count (190 lines).

- [ ] **Step 2: Insert form feed and header for page 27 in `sst.doc`**
At line 1540 (before `In 2026 I added an optional full-screen interface...`), insert:
```text
                  ********MODIFICATIONS********                       27
```
And if needed, split subsequent section so neither page exceeds 67 lines.

- [ ] **Step 3: Renumber subsequent section headers**
Update `ACKNOWLEDGMENTS` and `REFERENCES` to page 28 (or 29 depending on split).

- [ ] **Step 4: Update Table of Contents on page 2 (`sst.doc:105-110`)**
Reflect the new page numbers for `ACKNOWLEDGMENTS` and `REFERENCES`.

- [ ] **Step 5: Run awk verification and test suite**
Run: `awk '/\f/{if (prev) printf "%d lines before the break at %d\n", NR-prev, NR; prev=NR}' sst.doc`
Confirm all page lengths fall within the 47-67 line standard.
Run: `ctest --preset debug -R '^help$'`
Expected: PASS.

- [ ] **Step 6: Commit**
```bash
git add sst.doc
git commit -m "docs(manual): split MODIFICATIONS across form feeds and renumber Table of Contents (#121)"
```

---

### Task 5: Golden Fixtures Re-recording & Local Gate Verification

**Files:**
- Modify: `tests/golden/*.txt` (as needed for prompt spacing)

- [ ] **Step 1: Re-record golden fixtures**
Run: `tests/golden.sh ./build/debug/sst --update`
Run: `git diff tests/golden/`
Inspect line-by-line: confirm ONLY the end-of-game prompt lines changed.

- [ ] **Step 2: Commit updated golden recordings if modified**
```bash
git add tests/golden/
git commit -m "test(golden): update recordings for end-of-game prompt newline separation"
```

- [ ] **Step 3: Run local CI gate: `ci-debug`**
Run: `cmake --preset ci-debug && cmake --build --preset ci-debug && ctest --preset ci-debug`
Expected: 100% tests passed.

- [ ] **Step 4: Run local CI gate: `ci-release`**
Run: `cmake --preset ci-release && cmake --build --preset ci-release && ctest --preset ci-release`
Expected: 100% tests passed.

- [ ] **Step 5: Run local CI gate: `lineendings`**
Run: `ctest --preset debug -R '^lineendings$'`
Expected: PASS.
