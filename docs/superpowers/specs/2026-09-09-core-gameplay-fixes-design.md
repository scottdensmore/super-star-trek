# Design Spec: Core Gameplay Fixes & Polish (Sub-Project 2)

**Issue**: [#210](https://github.com/scottdensmore/super-star-trek/issues/210)  
**Date**: 2026-09-09  
**Status**: Approved (Brainstorming Complete)

---

## 1. Overview & Goals

This specification details the design for **Sub-Project 2: Core Gameplay Fixes & Polish**, addressing high-priority gameplay bugs, I/O formatting consistency, score column validation, and documentation pagination across Super Star Trek.

### Key Goals
1. **Formatted I/O Consistency ([#162](https://github.com/scottdensmore/super-star-trek/issues/162))**:
   - Introduce `void proutfn(const char *fmt, ...);` for formatted open prompts (matching `proutn`).
   - Standardize `proutf(fmt, ...)` as a complete-line printer that guarantees a trailing newline.
   - Separate end-of-game questions (`Do you want your score recorded?`, `Do you want to play again?`) so they never run together on piped input or typeahead.
2. **Save/Freeze Rule Alignment ([#160](https://github.com/scottdensmore/super-star-trek/issues/160))**:
   - Align `sst.doc` with existing game code regarding filename rules: 9-character limit, `.trk` extension added only when no dot is present, and letter-start constraint.
   - Replace generic `huh()` parser failures with helpful Spock guidance when a filename does not start with an alphabetic letter.
3. **Score Sheet C-Level Verification ([#76](https://github.com/scottdensmore/super-star-trek/issues/76))**:
   - Replace the fragile source-scraping awk script (`tests/score.sh`) with native compiled C unit test assertions in `tests/test_rules.c`.
   - Test all 12 dual-form score lines (singular and plural), 5 skill winning bonuses, penalties, and totals against the 47-column right-alignment invariant.
4. **Documentation Pagination ([#121](https://github.com/scottdensmore/super-star-trek/issues/121))**:
   - Split `MODIFICATIONS` in `sst.doc` across form feeds so pages remain within the 47–67 line standard.
   - Renumber following sections and update the Table of Contents on page 2 (`sst.doc:54-110`).
5. **Local Verification**:
   - Update golden fixtures with `tests/golden.sh` to capture only intentional prompt spacing.
   - Pass all local CI gates (`ci-debug`, `ci-release`, and `lineendings`).

---

## 2. Technical Specification

### 2.1 Formatted Output Architecture (`sst.h` / `sst.c`)
- **`proutfn(const char *fmt, ...)`**:
  - Takes format arguments and formats into an internal buffer via `vsnprintf`.
  - Breaks lines at embedded `\n` to advance screen paging properly.
  - Leaves the trailing line open without an extra newline, designed for interactive prompts.
- **`proutf(const char *fmt, ...)`**:
  - Guarantees that the formatted text concludes with a newline. If the formatted output does not end with `\n`, calls `skip(1)`.
- **End-of-Game Prompt Formatting (`sst.c:665-675`)**:
  - Update `score(0)` prompt:
    ```c
    if (alldone) {
        score(0);
        skip(1);
        proutn("Do you want your score recorded? ");
        if (ja()) {
            chew();
            freeze(FALSE);
        }
    }
    skip(1);
    proutn("Do you want to play again? ");
    if (!ja()) break;
    ```

### 2.2 Freeze / Thaw Alignment (`setup.c` & `sst.doc`)
- In `setup.c` (`freeze()` and `thaw()`):
  - Check `key != IHALPHA`:
    ```c
    if (key != IHALPHA) {
        prout("Spock- \"Captain, file names must begin with an alphabetic letter (A-Z).\"");
        return;
    }
    ```
  - Preserve the 9-character truncation and the dot preservation rule (`if (strchr(citem, '.') == NULL) strcat(citem, ".trk");`).
- In `sst.doc` (`FREEZE`, `THAW`, and `COMMAND SUMMARY` sections):
  - Document that filenames must begin with a letter, retain up to 9 characters, and automatically append `.TRK` only when no dot is present in the input.

### 2.3 Score Sheet Column Alignment (`finish.c` & `tests/test_rules.c`)
- Hoist or declare score format strings/helpers in `finish.c` and expose via `finish.h`.
- In `tests/test_rules.c`:
  - Implement `check_score_column(const char *line)` asserting that formatting ends at column 47.
  - Exercise all dual-form format strings with singular (`1`) and plural (`2`) arguments.
  - Exercise all 5 skill winning bonuses (`Novice`, `Fair`, `Good`, `Expert`, `Emeritus`).
  - Exercise penalty (`alive == 0`) and total score lines.
- Remove `tests/score.sh` and drop the `score` test target from `CMakeLists.txt`.

### 2.4 Documentation Pagination (`sst.doc`)
- Split page 26 (`MODIFICATIONS`) at line 1540 with `\f` and header for page 27.
- Split page 27 if needed to keep pages <= 67 lines.
- Renumber subsequent page headers (`ACKNOWLEDGMENTS` and `REFERENCES` become page 28).
- Update Table of Contents at `sst.doc:106-110`.

---

## 3. Testing & Gate Strategy

1. **Unit Tests**:
   - `ctest --preset debug -R '^rules$'` to verify all score column assertions pass in compiled C code.
2. **Integration Tests**:
   - `ctest --preset debug -R '^help$'` to confirm `sst.doc` navigation and topics remain intact after pagination.
   - `ctest --preset debug -R '^journey$'` and `ctest --preset debug -R '^tui$'` to verify terminal and prompt interactions.
3. **Golden Fixtures**:
   - Re-record golden fixtures: `tests/golden.sh ./build/debug/sst --update`.
   - Inspect diffs to ensure only end-of-game prompt spacing changes.
4. **Local Gates**:
   - `cmake --preset ci-debug && cmake --build --preset ci-debug && ctest --preset ci-debug`
   - `cmake --preset ci-release && cmake --build --preset ci-release && ctest --preset ci-release`
   - `ctest --preset debug -R '^lineendings$'`
