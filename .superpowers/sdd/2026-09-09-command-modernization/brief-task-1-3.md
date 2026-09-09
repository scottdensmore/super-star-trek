# Task Brief: Tasks 1, 2, and 3 - Command Registry Module, Typo Suggestions & Topic Catalog

## Scope & Target Files
- Create: `cmdtab.h`
- Create: `cmdtab.c`
- Create: `tests/test_cmdtab.c`
- Modify: `CMakeLists.txt` (add `cmdtab.c` to `sst` sources, add `test_cmdtab` target and test)

## Requirements
Follow the implementation plan `docs/superpowers/plans/2026-09-09-command-modernization.md` (Tasks 1, 2, 3) and spec `docs/superpowers/specs/2026-09-09-command-modernization-design.md`:

### 1. Data Structures (`cmdtab.h`)
- Categories: `CMD_CAT_NAV`, `CMD_CAT_COMBAT`, `CMD_CAT_SENSORS`, `CMD_CAT_SHIP`, `CMD_CAT_SYSTEM`.
- `command_def_t` containing:
  - `const char *name;` (canonical name, e.g. "move", "phasers")
  - `const char *syntax;` (e.g. "MOVE [manual|automatic] <course> <distance>")
  - `const char *summary;` (concise 1-line description)
  - `const char *example;` (concrete example, e.g. "MOVE manual 1 3 (Course 1=East, distance 3)")
  - `const char *doc_key;` (key in sst.doc, e.g. "  Mnemonic:  MOVE")
  - `cmd_category_t category;`
  - `int allow_abbrev;` (1 for classic 29 commands, 0 for commands requiring exact match)
  - `int enabled;` (1 if active; respects SCORE, CLOAKING, CAPTURE, DEBUG compile options)
  - `int id;` (0..36 matching classic command IDs in sst.c)
- `topic_def_t` containing:
  - `const char *name;` ("scoring", "tui", "notes", "abbrev", "modifications")
  - `const char *title;` (display title)
  - `const char *doc_header;` (anchor pattern in sst.doc)
  - `const char *doc_end;` (terminator marker)

### 2. Registry Functions (`cmdtab.c` and `cmdtab.h`)
- `void cmdtab_init(void)`: registers all 37 commands with full metadata, sets `enabled` per compile definitions.
- `int cmdtab_count(void)`: returns total number of registered commands.
- `const command_def_t *cmdtab_get(int index)`: returns command by index.
- `const command_def_t *cmdtab_lookup(const char *input)`: case-insensitive match; checks exact match first, then prefix match if `allow_abbrev` is set.
- `const command_def_t *cmdtab_suggest(const char *input)`: Levenshtein distance check (distance <= 2) against enabled commands, returns closest match or NULL if none.
- `int cmdtab_topic_count(void)`: returns count of manual topics.
- `const topic_def_t *cmdtab_topic_get(int index)`: returns topic by index.
- `const topic_def_t *cmdtab_topic_lookup(const char *input)`: case-insensitive topic lookup.
- `void cmdtab_print_categories(void)`: prints commands grouped by category with summaries (using `prout`/`proutn` or formatted stdout helper).
- `void cmdtab_print_topics(void)`: prints available topics with titles.

### 3. Testing & Verification
- Test file `tests/test_cmdtab.c` must test:
  1. Exact lookup across commands.
  2. Prefix matching where allowed, rejection of prefixes where disallowed.
  3. Fuzzy suggestions on typos (`mvoe` -> `move`, `phaser` -> `phasers`, `shild` -> `shields`, `xyzzy` -> NULL).
  4. Topic lookup for `scoring`, `tui`, `notes`, `abbrev`.
  5. Assertion that all enabled commands have non-empty syntax, summary, and example.
- Register `test_cmdtab` in `CMakeLists.txt`:
  ```cmake
  add_executable(test_cmdtab tests/test_cmdtab.c cmdtab.c)
  add_test(NAME cmdtab COMMAND test_cmdtab)
  ```
- Make sure `cmdtab.c` is also added to `add_executable(sst ...)` in `CMakeLists.txt:35`.
- Verify:
  `cmake --preset debug`
  `cmake --build --preset debug`
  `ctest --preset debug -R '^cmdtab$'`
  `ctest --preset debug -R '^lineendings$'`
- Commit with message: `feat(cmdtab): implement command and topic registry with tests`
