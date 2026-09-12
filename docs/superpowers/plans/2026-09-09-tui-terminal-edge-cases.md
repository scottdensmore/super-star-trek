# Sub-Project 3: TUI & Terminal Edge Cases Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Harden Super Star Trek's curses TUI, terminal signal handling, window layout clamps, and line reconstruction invariants against edge cases.

**Architecture:** Implement targeted fixes organized into three modular clusters: (1) Signal & IOCTL consolidation in `osx.c` and `tui.c`, (2) Geometry clamps & game-state synchronization in `tui.c` and `moving.c`, and (3) `restore_curline()` height grow/shrink differentiation and string wrapping invariants in `tui.c`. Verify through compiled C unit tests in `test_tuifmt.c` and automated tmux tests in `tests/tui.sh`.

**Tech Stack:** C99, POSIX signals & termios, ncurses, CMake, CTest, Bash, tmux.

**Spec:** `docs/superpowers/specs/2026-09-09-tui-terminal-edge-cases-design.md`

## Global Constraints

- Remote GitHub Actions are disabled; all verification gates must pass locally (`ci-debug`, `ci-release`, `lineendings`).
- Zero changes to core combat, damage, or stardate game arithmetic.
- Preserve CRLF line endings convention on modified C source files (`osx.c`, `tui.c`, `moving.c`, `tui.h`) and `README.md`.
- Preserve LF line endings on test scripts (`tests/tui.sh`) and compiled tests (`tests/test_tuifmt.c`).
- Superpowers workflow artifacts (`.superpowers/` and `docs/superpowers/`) are ignored by git; do not commit them.

---

### Task 1: Keystroke Reader Suspend/Resume Spacebar Recovery in `osx.c:getch()` (#190)

**Files:**
- Modify: `osx.c:40-89`
- Test: `tests/tui.sh`

**Interfaces:**
- Consumes: `tcsetattr(0, TCSANOW, &newstate)`, POSIX `read(0, ...)`
- Produces: `int getch(void)` with immediate Space responsiveness after `SIGTSTP`/`SIGCONT`

- [ ] **Step 1: Write failing test in `tests/tui.sh`**

Add a test arm in `tests/tui.sh` that pauses at `[HIT SPACE BAR TO CONTINUE]` in plain mode, sends `SIGTSTP`, sends `SIGCONT`, sends a Space key, and asserts the game continues without requiring Enter:

```bash
# In tests/tui.sh:
start 80 24 "tournament 7 short novice pw" ""
# trigger pause: e.g., request instructions or help topic with pause
```

Verify the existing code swallows Space after resume.

- [ ] **Step 2: Run test to verify it fails**

Run: `ctest --preset debug -R '^tui$'`
Expected: FAIL on the new suspend-resume spacebar test arm.

- [ ] **Step 3: Implement `tcsetattr` re-application in `osx.c:getch()`**

In `osx.c`:
```c
	do {
		tcsetattr(0, TCSANOW, &newstate);
		n = read(0, &chbuf, 1);
	} while (n < 0 && errno == EINTR);
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cmake --build --preset debug && ctest --preset debug -R '^tui$'`
Expected: PASS

- [ ] **Step 5: Verify line endings and commit**

```bash
ctest --preset debug -R '^lineendings$'
git add osx.c tests/tui.sh
git commit -m "fix(tui): re-apply termios in getch retry loop to prevent swallowing space after suspend (#190)"
```

---

### Task 2: Immediate Curses Suspend-Resize Repaint in `tui.c` (#191)

**Files:**
- Modify: `tui.c:1640-1830`, `tui.c:2040-2130`
- Test: `tests/tui.sh`

**Interfaces:**
- Consumes: `sync_size()`, `tui_refresh_panels()`
- Produces: `on_sigcont` signal handler and `sigcont_pending` detection

- [ ] **Step 1: Write failing test in `tests/tui.sh`**

Add a test arm in `tests/tui.sh`:
Launch full-screen game (`sst -t`) at 100x30, suspend (`SIGTSTP`), resize tmux window to 90x26, resume (`SIGCONT`), capture pane immediately before sending any keypress, and assert frame width matches 90 columns without stale 100-column text.

- [ ] **Step 2: Run test to verify it fails**

Run: `ctest --preset debug -R '^tui$'`
Expected: FAIL (frame remains un-repaired until a key is sent).

- [ ] **Step 3: Implement `SIGCONT` handling and immediate repaint in `tui.c`**

In `tui.c`:
1. Declare:
   ```c
   static volatile sig_atomic_t sigcont_pending = 0;
   static void on_sigcont(int sig) {
       (void)sig;
       sigcont_pending = 1;
   }
   ```
2. In `tui_init()`, install `on_sigcont` for `SIGCONT` (without `SA_RESTART`).
3. In `tui_getch()` and `tui_readline()`, check `sigcont_pending`. When non-zero, reset flag, call `sync_size()`, and repaint panels and window immediately before reading.

- [ ] **Step 4: Run test to verify it passes**

Run: `cmake --build --preset debug && ctest --preset debug -R '^tui$'`
Expected: PASS

- [ ] **Step 5: Verify line endings and commit**

```bash
ctest --preset debug -R '^lineendings$'
git add tui.c tests/tui.sh
git commit -m "fix(tui): handle SIGCONT to immediately relayout and repaint on resume (#191)"
```

---

### Task 3: IOCTL Consolidation & Cross-Source Comparison Fix (#183, #175, #176)

**Files:**
- Modify: `tui.c:1430-1505`, `tui.c:1770-1785`
- Test: `tests/tui.sh:285-300`

**Interfaces:**
- Consumes: `term_size(int *rows, int *cols)`
- Produces: Normalized `axis_moved()`, unified `term_size()` call sites

- [ ] **Step 1: Write failing test / verify `tests/tui.sh` assertion**

In `tests/tui.sh`, update the `env size` arm to assert that `scrollback_count "LINES/COLUMNS make it"` does not increase on the retry.
Verify that `tests/tui.sh` passes or flags the regression.

- [ ] **Step 2: Unify `TIOCGWINSZ` call sites into `term_size()` and fix `axis_moved()`**

In `tui.c`:
1. In `tui_size_changed_since_refusal()`:
   ```c
   int r, c;
   term_size(&r, &c);
   if (r == 0) return FALSE;
   if (axis_moved("LINES", r, refusedlines, refusedtermlines)) return TRUE;
   if (axis_moved("COLUMNS", c, refusedcols, refusedtermcols)) return TRUE;
   return FALSE;
   ```
2. In `tui_init()`:
   ```c
   int r, c;
   term_size(&r, &c);
   haveterm = (r > 0);
   if (haveterm) {
       winsz.ws_row = r;
       winsz.ws_col = c;
       resize_term(pinned("LINES") ? LINES : r,
                   pinned("COLUMNS") ? COLS : c);
   }
   ```
3. In `axis_moved()`:
   ```c
   static int axis_moved(const char *name, int now, int refusedcurses,
                         int refusedterm) {
       (void)name;
       (void)refusedcurses;
       return refusedterm != 0 && now != refusedterm;
   }
   ```

- [ ] **Step 3: Run test to verify it passes**

Run: `cmake --build --preset debug && ctest --preset debug -R '^tui$'`
Expected: PASS

- [ ] **Step 4: Verify line endings and commit**

```bash
ctest --preset debug -R '^lineendings$'
git add tui.c tests/tui.sh
git commit -m "refactor(tui): unify TIOCGWINSZ into term_size, fix free axis moved comparison, and update tui.sh (#183, #175, #176)"
```

---

### Task 4: Window Geometry & Layout Boundary Clamps (#141, #182)

**Files:**
- Modify: `tui.c:350-385`
- Test: `tests/tui.sh`

**Interfaces:**
- Consumes: `layout_size(&rows, &cols, ...)`
- Produces: Clamped `statw`, bounded `mvwin(wmsg, panelh, 1)`

- [ ] **Step 1: Write failing tests in `tests/tui.sh`**

1. Add a test arm resizing mid-game to 30x8: assert no 1-column sliver border glyphs appear on the right.
2. Add a test arm running with `LINES=30`, resizing to 3 rows: assert the bottom border is clean and `COMMAND>` is not rendered over the border.

- [ ] **Step 2: Run test to verify it fails**

Run: `ctest --preset debug -R '^tui$'`
Expected: FAIL on the 30-column sliver and 3-row pinned prompt overlap.

- [ ] **Step 3: Implement geometry clamps in `make_windows()`**

In `tui.c`:
1. Status panel width clamp:
   ```c
   if (cols <= QUADW + 1) {
       statw = 0;
       quadw = cols;
   } else {
       statw = cols - QUADW;
       quadw = QUADW;
   }
   ```
2. Message window boundary check against layout rows:
   ```c
   if (panelh < rows) {
       if (wmsg == NULL) {
           wmsg = newwin(msgh, msgw, panelh, 1);
       } else {
           wresize(wmsg, msgh, msgw);
           mvwin(wmsg, panelh, 1);
       }
   }
   ```

- [ ] **Step 4: Run test to verify it passes**

Run: `cmake --build --preset debug && ctest --preset debug -R '^tui$'`
Expected: PASS

- [ ] **Step 5: Verify line endings and commit**

```bash
ctest --preset debug -R '^lineendings$'
git add tui.c tests/tui.sh
git commit -m "fix(tui): suppress status sliver at <=30 cols and guard message window on <=3 rows under pin (#141, #182)"
```

---

### Task 5: Atomic Quadrant Warp Move Transition & Status Line Fit Assertion (#148, #89)

**Files:**
- Modify: `moving.c:120-140`
- Test: `tests/test_tuifmt.c`, `tests/tui.sh`

**Interfaces:**
- Consumes: `quadx`, `quady`, `newqad(0)`, `fmt_status_line()`
- Produces: Atomic quadrant transition, `test_status_panel_width_bounds()`

- [ ] **Step 1: Write failing unit test in `tests/test_tuifmt.c`**

Add `test_status_panel_width_bounds()` asserting:
- For every status line `i = 1..10`, `strlen(buf) <= 40`.
- For every quadrant line `i = 0..10`, `strlen(buf) <= 26`.
Inject a temporarily widened status line to confirm failure (RED), then revert to normal.

- [ ] **Step 2: Add mid-move warp pause test in `tests/tui.sh`**

Add a test arm at 72x14 running `move 1 5`, pausing during quadrant entry, and verifying the quadrant title matches the displayed grid.

- [ ] **Step 3: Implement atomic quadrant state transition in `moving.c`**

In `moving.c`:
```c
int newquadx = (ix+9)/10;
int newquady = (iy+9)/10;
int newsectx = ix - 10*(newquadx-1);
int newsecty = iy - 10*(newquady-1);
if (newquadx != oldquadx || newquady != oldquady) {
    proutn("\nEntering");
    cramlc(1, newquadx, newquady);
} else {
    prout("(Negative energy barrier disturbs quadrant.)");
}
skip(1);
quadx = newquadx;
quady = newquady;
sectx = newsectx;
secty = newsecty;
quad[sectx][secty] = ship;
newqad(0);
return;
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cmake --build --preset debug && ctest --preset debug -R '^(tuifmt|tui)$'`
Expected: PASS

- [ ] **Step 5: Verify line endings and commit**

```bash
ctest --preset debug -R '^lineendings$'
git add moving.c tests/test_tuifmt.c tests/tui.sh
git commit -m "fix(tui): synchronize quadrant transition during warp move and pin 72-column status bounds (#148, #89)"
```

---

### Task 6: Prompt Line Reconstruction Grow vs Shrink & Boundary Invariants (#114, #134, #135, #136, #151, #101, #122, #123)

**Files:**
- Modify: `tui.c:760-1140`, `tui.c:1310-1325`, `README.md:30-55`, `tests/tui.sh`
- Test: `tests/tui.sh`

**Interfaces:**
- Consumes: `oldmsgh`, `oldmsgw`
- Produces: Differentiated height grow vs shrink in `restore_curline(int oldmsgw, int oldmsgh)`

- [ ] **Step 1: Write failing test in `tests/tui.sh` for height grow**

Add a test arm in `tests/tui.sh`:
With a pending prompt, shrink from 24 to 16 rows, then grow back to 24 rows.
Verify the prompt sits cleanly without duplicate prompts or blank line gaps above it.

- [ ] **Step 2: Run test to verify it fails**

Run: `ctest --preset debug -R '^tui$'`
Expected: FAIL on the height grow prompt restoration.

- [ ] **Step 3: Update `restore_curline()` to distinguish height grow vs shrink**

1. Update signature: `static void restore_curline(int oldmsgw, int oldmsgh);`
2. In `sync_size()`:
   Pass `oldmsgh` as `oldlines - PANELH > 1 ? oldlines - PANELH : 1;`.
3. In `restore_curline()`:
   When `oldmsgw == 0`:
   - If `maxy > oldmsgh` (grow): erase back from `last` to clear the old stump.
   - If `maxy < oldmsgh` (shrink): execute `wscrl(wmsg, 1)` and move to `maxy - 1`.
4. Audit exact-boundary checks `linelen % width == 0` to prevent extraneous row offsets.
5. Align timeout comment in `tests/tui.sh` (30s).
6. Align resize documentation in `README.md` and message-window comments in `tui.c`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cmake --build --preset debug && ctest --preset debug -R '^tui$'`
Expected: PASS

- [ ] **Step 5: Verify line endings and commit**

```bash
ctest --preset debug -R '^lineendings$'
git add tui.c README.md tests/tui.sh
git commit -m "fix(tui): differentiate height grow vs shrink in restore_curline and polish docs (#114, #134, #135, #136, #151, #101, #122, #123)"
```

---

### Task 7: Full Local CI Gates & Golden Fixture Verification

**Files:**
- Test: All CTest targets across `ci-debug` and `ci-release`

- [ ] **Step 1: Run Debug CI Gate**

Run: `cmake --preset ci-debug && cmake --build --preset ci-debug && ctest --preset ci-debug`
Expected: 100% pass across all tests.

- [ ] **Step 2: Run Release CI Gate**

Run: `cmake --preset ci-release && cmake --build --preset ci-release && ctest --preset ci-release`
Expected: 100% pass across all tests.

- [ ] **Step 3: Verify Line Endings Gate**

Run: `ctest --preset debug -R '^lineendings$'`
Expected: 100% pass.

- [ ] **Step 4: Verify Golden Fixture Parity**

Run: `ctest --preset debug -R '^golden$'`
Expected: 100% byte-for-byte pass.
