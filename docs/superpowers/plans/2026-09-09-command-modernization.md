# Command System Modernization & Interactive Guidance Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Modernize Super Star Trek's command system by replacing fragile index-based command dispatch with a modular table-driven registry, providing built-in syntax and examples, fuzzy typo suggestions, reachable topic manual help (resolving issue #120), and helpful interactive prompts.

**Architecture:** A new `cmdtab.c` / `cmdtab.h` module defines structured command and topic registries, prefix matching, and Levenshtein fuzzy distance matching. `sst.c` dispatches through `cmdtab` without magic integers. `helpme()` streams both command documentation and manual topics from `sst.doc` while always guaranteeing built-in quick references. Interactive prompts in `moving.c`, `battle.c`, and `setup.c` are updated with concrete examples and guidance.

**Tech Stack:** C17, CMake 3.21+, CTest, POSIX shell test harnesses, ncurses.

**Spec:** `docs/superpowers/specs/2026-09-09-command-modernization-design.md`

## Global Constraints

- C17 standard compliance with `-Wall -Wextra -Wmissing-prototypes` and `-Werror` under CI presets.
- Presets: `debug`, `release`, `ci-debug`, `ci-release`. No manual CMake flags.
- Dual display support: Line-oriented terminal mode and full-screen ncurses mode (`sst -t`).
- Line endings: Game output and files must not contain trailing `\r` carriage returns; `lineendings` test must pass.
- Golden fixture integrity: Combat, damage, and navigation arithmetic must remain 100% identical; fixture diffs must be restricted to prompt text changes.
- Local gate verification: GitHub Actions are disabled; all verification gates (`ci-debug`, `ci-release`, `lineendings`) must pass locally before completion.

---

### Task 1: Command Registry Data Structures & Lookup (`cmdtab.h`, `cmdtab.c`, `tests/test_cmdtab.c`)

**Files:**
- Create: `cmdtab.h`
- Create: `cmdtab.c`
- Create: `tests/test_cmdtab.c`
- Modify: `CMakeLists.txt:35-59`

**Interfaces:**
- Consumes: Standard C library (`string.h`, `ctype.h`, `stdio.h`)
- Produces:
  ```c
  typedef enum {
      CMD_CAT_NAV,
      CMD_CAT_COMBAT,
      CMD_CAT_SENSORS,
      CMD_CAT_SHIP,
      CMD_CAT_SYSTEM
  } cmd_category_t;

  typedef struct {
      const char *name;
      const char *syntax;
      const char *summary;
      const char *example;
      const char *doc_key;
      cmd_category_t category;
      int allow_abbrev;
      int enabled;
      int id;
  } command_def_t;

  void cmdtab_init(void);
  int cmdtab_count(void);
  const command_def_t *cmdtab_get(int index);
  const command_def_t *cmdtab_lookup(const char *input);
  ```

- [ ] **Step 1: Write the failing unit test**

Create `tests/test_cmdtab.c`:
```c
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>
#include "cmdtab.h"

int main(void) {
    cmdtab_init();
    assert(cmdtab_count() >= 36);

    /* Test exact lookup */
    const command_def_t *cmd = cmdtab_lookup("move");
    assert(cmd != NULL);
    assert(strcmp(cmd->name, "move") == 0);
    assert(cmd->syntax != NULL && strlen(cmd->syntax) > 0);
    assert(cmd->example != NULL && strlen(cmd->example) > 0);

    /* Test abbreviation lookup */
    const command_def_t *abbrev_cmd = cmdtab_lookup("m");
    assert(abbrev_cmd != NULL);
    assert(strcmp(abbrev_cmd->name, "move") == 0);

    /* Test non-abbreviated command rejects prefix */
    assert(cmdtab_lookup("fre") == NULL);
    assert(cmdtab_lookup("freeze") != NULL);

    printf("PASS: cmdtab lookup tests\n");
    return 0;
}
```

- [ ] **Step 2: Add test and sources to `CMakeLists.txt` and verify it fails**

In `CMakeLists.txt`, add `cmdtab.c` to `sst` sources, and add `test_cmdtab`:
```cmake
add_executable(test_cmdtab tests/test_cmdtab.c cmdtab.c)
add_test(NAME cmdtab COMMAND test_cmdtab)
```
Run: `cmake --build --preset debug --target test_cmdtab`
Expected: FAIL (missing `cmdtab.h` / `cmdtab.c`)

- [ ] **Step 3: Implement `cmdtab.h` and `cmdtab.c`**

Create `cmdtab.h` with the public enum, struct, and prototypes.
Create `cmdtab.c` declaring the full table of 37 commands:
`srscan`, `lrscan`, `phasers`, `photons`, `move`, `shields`, `dock`, `damages`, `chart`, `impulse`, `rest`, `warp`, `status`, `sensors`, `orbit`, `transport`, `mine`, `crystals`, `shuttle`, `planets`, `request`, `report`, `computer`, `commands`, `emexit`, `probe`, `cloak`, `capture`, `score`, `abandon`, `destruct`, `freeze`, `deathray`, `debug`, `call`, `quit`, `help`.

Include canonical syntax, one-line summary, concrete example, and category for each entry.
Implement `cmdtab_lookup` checking exact match first, then prefix match if `allow_abbrev` is true.

- [ ] **Step 4: Run test to verify it passes**

Run: `cmake --build --preset debug --target test_cmdtab && ctest --preset debug -R '^cmdtab$'`
Expected: PASS

- [ ] **Step 5: Check line endings and commit**

```bash
ctest --preset debug -R '^lineendings$'
git add cmdtab.h cmdtab.c tests/test_cmdtab.c CMakeLists.txt
git commit -m "feat(cmdtab): introduce table-driven command registry with tests"
```

---

### Task 2: Fuzzy Command Suggestions (`cmdtab_suggest`)

**Files:**
- Modify: `cmdtab.h`
- Modify: `cmdtab.c`
- Modify: `tests/test_cmdtab.c`

**Interfaces:**
- Consumes: `cmdtab.h`
- Produces:
  ```c
  const command_def_t *cmdtab_suggest(const char *input);
  ```

- [ ] **Step 1: Write the failing unit test**

In `tests/test_cmdtab.c`, add tests for typo suggestions:
```c
/* Test typo suggestions */
const command_def_t *sug = cmdtab_suggest("mvoe");
assert(sug != NULL);
assert(strcmp(sug->name, "move") == 0);

sug = cmdtab_suggest("phaser");
assert(sug != NULL);
assert(strcmp(sug->name, "phasers") == 0);

sug = cmdtab_suggest("shild");
assert(sug != NULL);
assert(strcmp(sug->name, "shields") == 0);

/* Unrelated input produces no suggestion */
assert(cmdtab_suggest("xyzzy99") == NULL);
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cmake --build --preset debug --target test_cmdtab`
Expected: FAIL (symbol `cmdtab_suggest` not defined)

- [ ] **Step 3: Implement Levenshtein distance and `cmdtab_suggest` in `cmdtab.c`**

Implement `levenshtein_distance(const char *s1, const char *s2)`:
Iterate over enabled commands, compute distance, and return the candidate with minimum distance if $\le 2$.

- [ ] **Step 4: Run test to verify it passes**

Run: `cmake --build --preset debug --target test_cmdtab && ctest --preset debug -R '^cmdtab$'`
Expected: PASS

- [ ] **Step 5: Check line endings and commit**

```bash
ctest --preset debug -R '^lineendings$'
git add cmdtab.h cmdtab.c tests/test_cmdtab.c
git commit -m "feat(cmdtab): add fuzzy command typo suggestion matcher"
```

---

### Task 3: Topic Registry & `sst.doc` Navigation (Resolving #120)

**Files:**
- Modify: `cmdtab.h`
- Modify: `cmdtab.c`
- Modify: `tests/test_cmdtab.c`

**Interfaces:**
- Consumes: `cmdtab.h`
- Produces:
  ```c
  typedef struct {
      const char *name;
      const char *title;
      const char *doc_header;
      const char *doc_end;
  } topic_def_t;

  int cmdtab_topic_count(void);
  const topic_def_t *cmdtab_topic_get(int index);
  const topic_def_t *cmdtab_topic_lookup(const char *input);
  ```

- [ ] **Step 1: Write the failing unit test**

In `tests/test_cmdtab.c`, add tests for topic lookup:
```c
assert(cmdtab_topic_count() >= 4);

const topic_def_t *top = cmdtab_topic_lookup("scoring");
assert(top != NULL);
assert(strcmp(top->name, "scoring") == 0);
assert(strstr(top->doc_header, "SCORING") != NULL);

top = cmdtab_topic_lookup("tui");
assert(top != NULL);
assert(strstr(top->title, "Full-Screen") != NULL);

top = cmdtab_topic_lookup("unknown_topic");
assert(top == NULL);
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cmake --build --preset debug --target test_cmdtab`
Expected: FAIL (missing topic types and functions)

- [ ] **Step 3: Implement topic catalog in `cmdtab.c` and `cmdtab.h`**

Define topics:
- `scoring`: title "Game Scoring System", search pattern `"       SCORING"`, terminator `"******"`.
- `tui`: title "Full-Screen TUI Mode & Resizing", search pattern `"A full-screen interface"`, terminator `"******"`.
- `notes`: title "Miscellaneous Game Notes", search pattern `"       MISCELLANEOUS NOTES"`, terminator `"******"`.
- `abbrev`: title "Command Abbreviations", search pattern `"COMMAND ABBREVIATIONS"`, terminator `"******"`.
- `modifications`: title "Game Modifications", search pattern `"       MODIFICATIONS"`, terminator `"******"`.

Implement `cmdtab_topic_lookup` with case-insensitive matching.

- [ ] **Step 4: Run test to verify it passes**

Run: `cmake --build --preset debug --target test_cmdtab && ctest --preset debug -R '^cmdtab$'`
Expected: PASS

- [ ] **Step 5: Check line endings and commit**

```bash
ctest --preset debug -R '^lineendings$'
git add cmdtab.h cmdtab.c tests/test_cmdtab.c
git commit -m "feat(cmdtab): add topic catalog resolving unreachable documentation (#120)"
```

---

### Task 4: Integrate `cmdtab` into `sst.c` Command Dispatch & Error Guidance

**Files:**
- Modify: `sst.c:58-250`
- Modify: `sst.h`

**Interfaces:**
- Consumes: `cmdtab.h`
- Produces: Updated command loop in `makemoves()` and modernized `listCommands()` in `sst.c`.

- [ ] **Step 1: Write a test verifying unrecognized command guidance**

In `tests/test_cmdtab.c`, verify that formatting functions for categorized command listings and suggestion lines produce the expected output strings.

- [ ] **Step 2: Run test to verify it fails**

Run: `cmake --build --preset debug --target test_cmdtab`
Expected: FAIL if helper output formatting is missing.

- [ ] **Step 3: Update `sst.c` to use `cmdtab`**

1. Include `"cmdtab.h"` in `sst.c`.
2. In `main()`, call `cmdtab_init()`.
3. In `makemoves()`:
   - Call `const command_def_t *cmd = cmdtab_lookup(citem);`
   - If matched, dispatch to the appropriate case or handler using `cmd->id`.
   - If not matched:
     - Check `const command_def_t *sug = cmdtab_suggest(citem);`
     - If `sug != NULL`:
       ```c
       proutf("UNRECOGNIZED COMMAND '%s'. Did you mean '%s'?\n", citem, sug->name);
       proutf("Example: %s\n", sug->example);
       proutf("Type 'HELP %s' for details or 'COMMANDS' for a list.\n", sug->name);
       ```
     - If `sug == NULL`:
       ```c
       proutf("UNRECOGNIZED COMMAND '%s'. Type 'COMMANDS' to list all commands or 'HELP <cmd>'.\n", citem);
       ```
4. Replace `listCommands()` to print commands categorized by subsystem with summaries.

- [ ] **Step 4: Verify compilation and tests**

Run: `cmake --build --preset debug && ctest --preset debug -R '^(cmdtab|rules|journey|journey-tui)$'`
Expected: PASS

- [ ] **Step 5: Check line endings and commit**

```bash
ctest --preset debug -R '^lineendings$'
git add sst.c sst.h tests/test_cmdtab.c
git commit -m "refactor(sst): dispatch commands through cmdtab and show suggestions on typos"
```

---

### Task 5: Overhaul In-Game Help (`helpme()` in `sst.c`, Resolving #120)

**Files:**
- Modify: `sst.c:128-202`
- Modify: `tests/help.sh`

**Interfaces:**
- Consumes: `cmdtab.h`, `sst.doc`
- Produces: Enhanced `helpme()` handling commands, topics, built-in examples, and `tests/help.sh` coverage.

- [ ] **Step 1: Write the failing test in `tests/help.sh`**

Add tests to `tests/help.sh`:
1. Query `help scoring` against `sst.doc` and assert output contains `SCORING`.
2. Query `help tui` against `sst.doc` and assert output contains `full-screen`.
3. Query `help move` without `sst.doc` present and assert output contains `Syntax:` and `Example:`.
4. Check that no trailing `\r` exists in any help output.

- [ ] **Step 2: Run `tests/help.sh` to verify it fails**

Run: `tests/help.sh ./build/debug/sst`
Expected: FAIL (unreachable topics in `sst.doc` and missing built-in fallback)

- [ ] **Step 3: Implement enhanced `helpme()` in `sst.c`**

1. If no command/topic requested:
   - Call `cmdtab_print_categories();`
   - Print note: `"Type 'HELP <command>' for specific command syntax or 'HELP TOPICS' for manual topics."`
2. If argument is `"topics"`:
   - Call `cmdtab_print_topics();`
3. Check `cmdtab_topic_lookup(citem)`:
   - If matched: open `sst.doc`, search for `topic->doc_header`, stream lines until `topic->doc_end`, strip `\r`, and print.
4. Check `cmdtab_lookup(citem)`:
   - If matched:
     - Print built-in syntax, summary, and example:
       ```text
       Spock- "Captain, command specifications:"
         Syntax:   <syntax>
         Example:  <example>
       ```
     - If `sst.doc` is available: scan for `cmd->doc_key` and stream lines until `******`, stripping `\r`.
     - If `sst.doc` is missing: print note that extended manual is missing but specifications are displayed above.
5. If neither matches:
   - Check `cmdtab_suggest(citem)` and print suggestion.

- [ ] **Step 4: Run `tests/help.sh` to verify it passes**

Run: `tests/help.sh ./build/debug/sst && ctest --preset debug -R '^help$'`
Expected: PASS

- [ ] **Step 5: Check line endings and commit**

```bash
ctest --preset debug -R '^lineendings$'
git add sst.c tests/help.sh
git commit -m "feat(help): unlock manual topics and embed syntax examples in help (#120)"
```

---

### Task 6: Modernize Interactive Prompts with Guidance & Examples

**Files:**
- Modify: `moving.c`
- Modify: `battle.c`
- Modify: `setup.c`

**Interfaces:**
- Consumes: Standard game I/O (`prout`, `proutn`, `scan`, `ja`)
- Produces: Updated prompts with guidance and examples.

- [ ] **Step 1: Write tests verifying prompt strings**

Write a journey test or test harness verification ensuring that when commands like `move`, `phasers`, `shields`, and `warp` are given empty inputs, the prompt text contains the expected guidance tokens (e.g. `1=E, 3=N`, `e.g.`).

- [ ] **Step 2: Run test to verify it fails**

Expected: FAIL against current prompt strings.

- [ ] **Step 3: Update interactive prompts**

1. In `moving.c` (`getcd()`):
   - Mode prompt: `Manual or automatic navigation (e.g. 'manual 1 3')- `
   - Course prompt: `Course (1-9; 1=E, 3=N, 5=W, 7=S) [e.g. 1]- `
   - Distance prompt: `Distance in quadrants or sectors [e.g. 2.5]- `
2. In `battle.c` (`phasers()`):
   - `Units to fire (Energy: %d available, e.g. 500, 0 to cancel)- `
3. In `battle.c` (`photon()`):
   - `Torpedo course (1-9; 1=E, 3=N, 5=W, 7=S) [e.g. 3]- `
4. In `battle.c` (`sheild()`):
   - `Units to transfer (+ to raise, - to drop, e.g. +300)- `
5. In `moving.c` (`setwrp()`):
   - `Warp factor (1.0 to 10.0, e.g. 6.0)- `
6. In `moving.c` (`probe()`):
   - `Arm NOVAMAX warhead (detonates at target)? (Y/N): `

- [ ] **Step 4: Run build and journey tests**

Run: `cmake --build --preset debug && ctest --preset debug -R '^(journey|journey-tui)$'`
Expected: PASS

- [ ] **Step 5: Check line endings and commit**

```bash
ctest --preset debug -R '^lineendings$'
git add moving.c battle.c setup.c
git commit -m "feat(ui): add examples and directional guidance to interactive prompts"
```

---

### Task 7: Golden Fixtures Re-recording & Verification

**Files:**
- Modify: `tests/golden/*.txt`

**Interfaces:**
- Consumes: `tests/golden.sh`
- Produces: Updated golden recordings matching the modernized prompts.

- [ ] **Step 1: Run `golden.sh` to inspect diffs**

Run: `tests/golden.sh ./build/debug/sst`
Expected: Diff detected on prompt lines.

- [ ] **Step 2: Update golden fixtures**

Run: `tests/golden.sh ./build/debug/sst --update`

- [ ] **Step 3: Inspect fixture diffs**

Run: `git diff tests/golden/`
Verify line by line that ONLY the prompt wording changed (e.g. `Manual or automatic- ` $\to$ `Manual or automatic navigation (e.g. 'manual 1 3')- `). Confirm that no combat outcomes, energy counts, or star dates changed.

- [ ] **Step 4: Run golden ctest**

Run: `ctest --preset debug -R '^golden$'`
Expected: PASS

- [ ] **Step 5: Check line endings and commit**

```bash
ctest --preset debug -R '^lineendings$'
git add tests/golden/
git commit -m "test(golden): re-record golden fixtures for modernized prompt strings"
```

---

### Task 8: Complete Local CI Gates & Verification

**Files:** None (verification step)

**Interfaces:**
- CMake Presets: `ci-debug`, `ci-release`, `debug`

- [ ] **Step 1: Run complete `ci-debug` gate locally**

Run: `cmake --preset ci-debug && cmake --build --preset ci-debug && ctest --preset ci-debug`
Expected: 100% tests pass (skips only documented platform skips).

- [ ] **Step 2: Run complete `ci-release` gate locally**

Run: `cmake --preset ci-release && cmake --build --preset ci-release && ctest --preset ci-release`
Expected: 100% tests pass.

- [ ] **Step 3: Run lineendings test**

Run: `ctest --preset debug -R '^lineendings$'`
Expected: PASS

- [ ] **Step 4: Verify git status is clean**

Run: `git status`
Expected: Clean working tree on `scottdensmore/feat/command-modernization`.
