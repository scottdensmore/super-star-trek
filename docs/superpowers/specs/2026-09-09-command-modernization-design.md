# Command System Modernization & Interactive Guidance Design Spec

- **Author**: Scott Densmore & Antigravity
- **Date**: 2026-09-09
- **Status**: Draft (Approved in Brainstorming)
- **Sub-Project**: Sub-Project 1: Command System & In-Game Help Modernization
- **Related Issues**: [#120 (Unreachable sst.doc sections)](https://github.com/scottdensmore/super-star-trek/issues/120)

---

## 1. Background & Motivation

Super Star Trek is a classic terminal space-strategy game originally written in BASIC (1978) and ported to C17. It supports both a line-oriented scrolling mode and an ncurses full-screen dashboard mode (`sst -t`).

Currently, command handling in `sst.c` relies on an array of raw strings (`commands[]`) coupled to magic integer indices (0 to 36) in `makemoves()`, `listCommands()`, and `helpme()`. A comment in `sst.c:58-60` has stood for decades noting:
> *"I don't like the way this is done, relying on an index. But I don't want to invest the time to make this nice and table driven."*

Furthermore:
1. **Unhelpful Prompts & Errors**: When a player makes a typo or enters an unrecognized command, the game responds tersely with `UNRECOGNIZED COMMAND.` or `Beg your pardon, Captain?` without suggestions. Sub-prompts (e.g. `Manual or automatic- ` or `Course (1-9) - `) offer no examples, leaving new players guessing at the coordinate system.
2. **Unreachable Manual Content ([#120](https://github.com/scottdensmore/super-star-trek/issues/120))**: In-game `HELP` only scans `sst.doc` for `  Mnemonic:  <COMMAND>` markers up to `******`. More than 80% of `sst.doc`—including `MISCELLANEOUS NOTES`, `SCORING`, `MODIFICATIONS` (which details full-screen `-t` mode and window resizing), and game mechanics—is unreachable from inside the game.

---

## 2. Goals & Non-Goals

### Goals
- **Structured Command Registry**: Replace the index-based array with a modular table-driven registry in `cmdtab.h` / `cmdtab.c`.
- **Built-in Quick Reference**: Embed syntax, one-line summary, category, and concrete usage examples for every command directly in the binary.
- **Fuzzy "Did You Mean?" Suggestions**: Provide Levenshtein distance suggestions and pointers to `HELP <command>` on typos instead of uninformative error messages.
- **Modernized Interactive Prompts**: Augment sub-prompts in `moving.c`, `battle.c`, and `setup.c` with concise guidance and examples (e.g. course directions `1=E, 3=N, 5=W, 7=S` and syntax formats).
- **Comprehensive Help & Topic Catalog ([#120](https://github.com/scottdensmore/super-star-trek/issues/120))**: Allow players to query manual topics like `HELP SCORING`, `HELP TUI`, `HELP NOTES`, and `HELP TOPICS`.
- **Hybrid Help Delivery**: If `sst.doc` is present in the working directory, stream detailed narrative text; if `sst.doc` is absent, still provide built-in syntax and examples.
- **Local-Only Gate Verification**: Because GitHub Actions are disabled, all CI checks (`ci-debug`, `ci-release`, and `lineendings`) must be verified locally without requiring remote CI jobs.

### Non-Goals
- Altering core combat arithmetic, random seed algorithms, or game balance formulas.
- Redesigning the ncurses panel layouts or screen borders.
- Replacing the classic turn-based game loop.

---

## 3. Architecture & Data Structures

### 3.1 Module Separation
A new translation unit `cmdtab.c` and header `cmdtab.h` are added to the build target in `CMakeLists.txt`.

### 3.2 Types & Definitions
```c
typedef enum {
    CMD_CAT_NAV,       /* Navigation: MOVE, IMPULSE, WARP, REST */
    CMD_CAT_COMBAT,    /* Weapons & Tactical: PHASERS, PHOTONS, SHIELDS, CLOAK */
    CMD_CAT_SENSORS,   /* Scans & Intelligence: SRSCAN, LRSCAN, CHART, SENSORS, PROBE */
    CMD_CAT_SHIP,      /* Ship Operations: STATUS, DAMAGES, DOCK, ORBIT, TRANSPORT, MINE, CRYSTALS, SHUTTLE, DEATHRAY, CALL, CAPTURE */
    CMD_CAT_SYSTEM     /* Game & System: COMPUTER, COMMANDS, REPORT, PLANETS, SCORE, FREEZE, ABANDON, DESTRUCT, QUIT, HELP, DEBUG, EMEXIT */
} cmd_category_t;

typedef struct {
    const char *name;          /* Canonical name, e.g. "move" */
    const char *syntax;        /* Usage syntax, e.g. "MOVE [manual|automatic] <course> <distance>" */
    const char *summary;       /* Short description, e.g. "Navigate Enterprise using warp engines" */
    const char *example;       /* Concrete example, e.g. "MOVE manual 1 3 (Course 1=East, distance 3)" */
    const char *doc_key;       /* Key in sst.doc, e.g. "  Mnemonic:  MOVE" */
    cmd_category_t category;   /* Subsystem classification */
    int allow_abbrev;          /* 1 if unique prefix matching is allowed (classic 29 commands) */
    int enabled;               /* 1 if command is active in current build (SCORE, CLOAKING, etc.) */
    int id;                    /* Canonical command ID */
    void (*handler)(void);     /* Function pointer to command handler */
} command_def_t;

typedef struct {
    const char *name;          /* Topic key, e.g. "scoring", "tui", "notes", "abbrev" */
    const char *title;         /* Display title, e.g. "Game Scoring System" */
    const char *doc_header;    /* Header match in sst.doc */
    const char *doc_end;       /* Terminator delimiter in sst.doc */
} topic_def_t;
```

### 3.3 Public API in `cmdtab.h`
- `void cmdtab_init(void)`: Initializes the command table and enables/disables optional commands based on compile configuration.
- `const command_def_t *cmdtab_lookup(const char *input)`: Matches input against canonical command names and abbreviations.
- `const command_def_t *cmdtab_suggest(const char *input)`: Returns closest command using Levenshtein distance ($\le 2$), or `NULL` if none.
- `const topic_def_t *cmdtab_topic_lookup(const char *input)`: Matches input against documented manual topics.
- `void cmdtab_print_categories(void)`: Displays commands grouped by category with concise descriptions.
- `void cmdtab_print_topics(void)`: Displays available manual topics.

---

## 4. Interactive Prompts & Help Integration

### 4.1 Unrecognized Command Handling
When input does not match any command in `makemoves()`:
```text
UNRECOGNIZED COMMAND 'mvoe'.
Did you mean 'MOVE'?
Example: MOVE manual 1 3 (Course 1 is East, distance 3)
Type 'HELP MOVE' for details or 'COMMANDS' for a list.
```
If no close match is found:
```text
UNRECOGNIZED COMMAND 'xyz'.
Type 'COMMANDS' to list all commands or 'HELP <command>'.
```

### 4.2 Modernized Interactive Prompts
When a command is entered without arguments or requires interactive input, prompts include concise guidance and examples:
- **MOVE**:
  - Mode prompt: `Manual or automatic navigation (e.g. 'manual 1 3')- `
  - Course prompt: `Course (1-9; 1=E, 3=N, 5=W, 7=S) [e.g. 1]- `
  - Distance prompt: `Distance in quadrants or sectors [e.g. 2.5]- `
- **PHASERS**:
  - `Units to fire (Energy: %d available, e.g. 500, 0 to cancel)- `
- **PHOTONS**:
  - `Torpedo course (1-9; 1=E, 3=N, 5=W, 7=S) [e.g. 3]- `
- **SHIELDS**:
  - `Units to transfer (+ to raise, - to drop, e.g. +300)- `
- **WARP**:
  - `Warp factor (1.0 to 10.0, e.g. 6.0)- `
- **PROBE**:
  - `Arm NOVAMAX warhead (detonates at target)? (Y/N): `

### 4.3 In-Game Help (`HELP [arg]`)
- **`HELP` (no arguments)**: Lists all commands grouped by subsystem categories, with one-line summaries and notes pointing to `HELP TOPICS`.
- **`HELP TOPICS`**: Lists browsable game topics (`SCORING`, `TUI`, `NOTES`, `ABBREV`).
- **`HELP <topic>`**: Opens `sst.doc`, locates the topic header, and streams narrative text until the section terminator, stripping any `\r` carriage returns.
- **`HELP <command>`**:
  1. Displays built-in quick reference (Syntax, Summary, Example).
  2. If `sst.doc` is found, displays narrative text from `sst.doc`.
  3. If `sst.doc` is missing, Spock notes: `Spock- "Captain, detailed documentation in SST.DOC is missing, but standard specifications are shown above."`

---

## 5. Testing & Verification Plan

### 5.1 Unit Tests (`test_cmdtab`)
- Red-Green-Refactor implementation using a standalone unit test binary `tests/test_cmdtab.c`.
- Tests:
  - Exact command lookup across all commands.
  - Abbreviation matching rules (only first 29 commands allow prefixes; remainder require exact strings).
  - Levenshtein fuzzy distance matching and suggestion boundary conditions.
  - Verification that every registered command contains non-empty syntax and example strings.
  - Topic lookup matching and topic list completeness.

### 5.2 Integration Tests
- **`tests/help.sh`**:
  - Test `help scoring`, `help tui`, and `help notes` with `sst.doc`.
  - Test command help behavior both when `sst.doc` is present and when absent.
  - Verify absence of trailing carriage returns (`\r`) in help output.
- **`tests/journey.sh`**:
  - Validate smoke run in plain mode and TUI fallback mode with updated prompt strings.

### 5.3 Golden Fixture Management
- Running `tests/golden.sh` will report diffs corresponding to updated prompt strings.
- Re-record golden fixtures:
  ```sh
  tests/golden.sh ./build/debug/sst --update
  ```
- Review the diff line-by-line to confirm only prompt text was altered, with zero impact on combat damage, energy calculations, or game logic.

### 5.4 Local Gate Verification
Because remote GitHub Actions are disabled, all gates are executed locally:
1. `cmake --preset ci-debug && cmake --build --preset ci-debug && ctest --preset ci-debug`
2. `cmake --preset ci-release && cmake --build --preset ci-release && ctest --preset ci-release`
3. `ctest --preset debug -R '^lineendings$'`
All tests must pass locally before declaring completion.
