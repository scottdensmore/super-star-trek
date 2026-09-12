# Design Specification: Sub-Project 3 — TUI & Terminal Edge Cases

**Date:** 2026-09-09  
**Tracking Issue:** [#211](https://github.com/scottdensmore/super-star-trek/issues/211)  
**Related Issues Closed:** #190, #191, #183, #175, #176, #182, #141, #148, #89, #114, #134, #135, #136, #151, #101, #122, #123

---

## 1. Overview & Objectives

Sub-Project 3 hardens Super Star Trek's full-screen curses interface (`sst -t`), classic terminal I/O, terminal signal handling, and layout geometry against edge cases. It addresses:
1. **Terminal Signals & Job Control:** Ensuring pauses do not swallow keystrokes after suspension, curses recovers immediately from resizes while suspended, duplicate `TIOCGWINSZ` ioctl zero-guards are consolidated, and terminal-to-terminal size comparisons are like-for-like.
2. **Layout Geometry & State Synchronization:** Eliminating degenerate 1-column status panel rendering at narrow widths ($\le 30$ columns), preventing prompt stamping across panel borders at $\le 3$ rows under an exported `LINES` pin, keeping quadrant titles and grid synchronized during mid-move pauses, and pinning the 72-column status line fit invariant.
3. **Prompt Line Reconstruction:** Distinguishing height grow vs shrink in `restore_curline()` to avoid unwanted gaps or lost conversation, and resolving exact-fit boundary conditions.
4. **Documentation & Comment Alignment:** Correcting timeout assertions, README resize documentation, and message-window height comments.

---

## 2. Architecture & Detailed Component Design

### Cluster 1: Terminal Signals, Suspend/Resume & IOCTL Hardening

#### 1.1 Keystroke Reader Suspend/Resume (`osx.c:getch()`) — #190
- **Root Cause:** When `sst` is paused at `[HIT SPACE BAR TO CONTINUE]` via `getch()` in plain mode or fallback mode, the terminal is put into non-canonical, no-echo mode (`tcsetattr(0, TCSANOW, &newstate)`). When the user presses `Ctrl-Z` (`SIGTSTP`), the shell suspends the job and restores the terminal to canonical mode. Upon resumption (`fg`, delivering `SIGCONT`), the kernel restarts or completes the interrupted `read(0, &chbuf, 1)` with `EINTR`. Because `tcsetattr` was only called before the loop, the restarted read occurs while the terminal remains in canonical mode. The line discipline holds the Space keystroke until the user presses Enter.
- **Implementation:** In `osx.c:getch()`, re-apply `tcsetattr(0, TCSANOW, &newstate)` on every pass inside the retry loop:
  ```c
  do {
      tcsetattr(0, TCSANOW, &newstate);
      n = read(0, &chbuf, 1);
  } while (n < 0 && errno == EINTR);
  ```
  Whenever an interrupted read resumes, the terminal line discipline is guaranteed to be non-canonical and no-echo before the next byte is read.

#### 1.2 Curses Suspend-Resize Immediate Repaint (`tui.c`) — #191
- **Root Cause:** In full-screen mode, when suspended by `SIGTSTP`, curses switches to shell mode. If the terminal is resized while suspended, curses' resume hook runs `reset_prog_mode()` and `doupdate()` using its stale pre-suspend cached dimensions. The display remains misaligned until the next user keypress reaches `wgetch()`, which finally returns `KEY_RESIZE`.
- **Implementation:**
  - Install a `SIGCONT` handler in `tui.c`:
    ```c
    static volatile sig_atomic_t sigcont_pending = 0;
    static void on_sigcont(int sig) {
        (void)sig;
        sigcont_pending = 1;
    }
    ```
  - Register `on_sigcont` via `sigaction(SIGCONT, ...)` without `SA_RESTART`, so blocking reads are interrupted when `SIGCONT` arrives.
  - In `tui_getch()` and `tui_readline()`, check `sigcont_pending`. If set:
    - Clear `sigcont_pending = 0;`.
    - Run `sync_size()`.
    - Refresh panels and message window immediately.
  - The frame repaints instantly on `fg` with zero delay or user interaction required.

#### 1.3 Consolidated `TIOCGWINSZ` Zero Guards (`tui.c`) — #183
- **Root Cause:** Three sites in `tui.c` repeat the identical ioctl check:
  `ioctl(fileno(stdout), TIOCGWINSZ, &ws) != 0 || ws.ws_row <= 0 || ws.ws_col <= 0`.
- **Implementation:**
  - `term_size(int *rows, int *cols)` is already the canonical implementation, assigning `*rows = 0; *cols = 0;` whenever the ioctl fails or reports non-positive dimensions.
  - Update `tui_size_changed_since_refusal()` to call `term_size(&r, &c)` and return `FALSE` if `r == 0`.
  - Update `tui_init()` to call `term_size(&winsz.ws_row, &winsz.ws_col)` and derive `haveterm = (winsz.ws_row > 0)`.
  - Remove duplicate comments and explanations across the call sites.

#### 1.4 Like-for-Like Comparison on Free Axis (`tui.c:axis_moved()`) — #175
- **Root Cause:** In `axis_moved()`, an unpinned axis compares `now` (from `TIOCGWINSZ`) against `refusedcurses` (which came from terminfo if `refusedtermlines == 0`). If the refusal occurred when no terminal size was available (0x0 pty), this compares ioctl against terminfo, spuriously claiming the terminal moved.
- **Implementation:**
  - In `axis_moved()`, check `refusedterm == 0`. If `refusedterm == 0`, return `FALSE` (the refusal had no terminal size, so no movement can be determined).
  - For both pinned and unpinned axes, compare `now != refusedterm`. Both sides are guaranteed to come from the terminal ioctl.

#### 1.5 Unreachable Test Assertion Cleanup (`tests/tui.sh`) — #176
- **Root Cause:** The `env size` arm runs in a 100x30 pane where `smallwindow` is false. `staying classic` only prints when `smallwindow` is true.
- **Implementation:** Update `tests/tui.sh` to count instances of `LINES/COLUMNS make it` using `scrollback_count`, ensuring the test can fail on regressions.

---

### Cluster 2: Window Layout, Mid-Move State & Geometry Clamps

#### 2.1 Degenerate Status Panel Sliver Clamp (`tui.c:make_windows()`) — #141
- **Root Cause:** `statw = cols - QUADW > 1 ? cols - QUADW : 1;`. When `cols <= 30`, `statw` is clamped to 1. `box(wstat, 0, 0)` renders left and right borders in the same column, creating a 1-column vertical glyph sliver with 0 usable interior columns.
- **Implementation:**
  - In `make_windows()`, evaluate whether sufficient columns exist for a status panel:
    ```c
    if (cols <= QUADW + 1) {
        statw = 0;
        quadw = cols;
    } else {
        statw = cols - QUADW;
        quadw = QUADW;
    }
    ```
  - If `statw == 0`, do not draw `box(wstat)` or status content; expand `wquad` to `cols` so the quadrant panel occupies the full terminal width and remains legible.

#### 2.2 Layout Boundary Check on $\le 3$ Rows Under `LINES` Pin (`tui.c:make_windows()`) — #182
- **Root Cause:** When `rows <= 3`, `panelh` is clamped to `PANELMIN` (3). `wmsg` origin is placed at row `panelh` (row 3). Unpinned, `stdscr` has 3 rows, so `mvwin(wmsg, 3, 1)` fails with `ERR` and leaves the border clean. Under an exported `LINES=24` or `LINES=30` pin, `stdscr` has 24–30 rows, so `mvwin()` succeeds and renders `COMMAND>` across the bottom border of the quadrant and status panels.
- **Implementation:**
  - In `make_windows()`, check `panelh >= rows` (where `rows` is the clipped layout height from `layout_size()`).
  - If `panelh >= rows`, skip moving or displaying `wmsg`. The message window is treated as off-screen, ensuring pinned and unpinned terminals behave identically.

#### 2.3 Atomic Quadrant Transition During Warp Move (`moving.c`) — #148
- **Root Cause:** In `moving.c:121-136`, `quadx` and `quady` are assigned immediately before printing `\nEntering Quadrant X - Y`. If that output triggers a pager pause, the panels repaint with the *new* quadrant coordinates in the title and position line, but the 8x8 grid still holds the *old* quadrant data.
- **Implementation:**
  - Keep `quadx` and `quady` unchanged during the announcement:
    ```c
    int newquadx = (ix + 9) / 10;
    int newquady = (iy + 9) / 10;
    int newsectx = ix - 10 * (newquadx - 1);
    int newsecty = iy - 10 * (newquady - 1);
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
    ```
  - The panels remain consistent with the ship's actual position at all times.

#### 2.4 Status Panel 72-Column Fit Invariant (`tests/test_tuifmt.c`) — #89
- **Implementation:**
  - Add `test_status_panel_width_bounds()` in `tests/test_tuifmt.c`.
  - Assert that across all simulated states (damaged shields, low energy, docked, stars, enemies), `strlen(buf) <= 40` for every status field `1..10`.
  - Assert `strlen(buf) <= 26` for all quadrant panel lines.

---

### Cluster 3: Line Reconstruction & `restore_curline()` Invariants

#### 3.1 Differentiate Height Grow vs. Shrink (`tui.c:restore_curline()`) — #114
- **Root Cause:** When width is unchanged (`oldmsgw == 0`), `restore_curline()` executes `wscrl(wmsg, 1)` and `wmove(wmsg, maxy - 1, 0)`. On a shrink, this preserves truncated conversation. On a grow, the old stump is still on screen, and the scroll moves it up and leaves an unwanted blank gap above the restored prompt.
- **Implementation:**
  - Update `restore_curline(int oldmsgw, int oldmsgh)`.
  - In `sync_size()`, pass `oldlines - PANELH` (or previous message height).
  - When `oldmsgw == 0`:
    - If `maxy > oldmsgh` (grow): erase back from `last` to clear the old stump without scrolling.
    - If `maxy < oldmsgh` (shrink): scroll `wscrl(wmsg, 1)` to keep live conversation.

#### 3.2 Exact-Fit and Multi-Line Wrap Boundaries (#134, #135, #136, #151)
- **Implementation:**
  - Standardize exact-fit modulo arithmetic: when `linelen % width == 0`, avoid inserting spurious newline or advancing cursor beyond window boundaries.
  - In multi-line wrapped search, ensure `start = r - (wantlen - 1) / width - arows` correctly scans all lines when both question and answer wrap.
  - Add explicit commentary documenting buffer size limits (128/160/512 bytes) and scroll guard preconditions.

#### 3.3 Documentation, Comment & Test Polish (#101, #122, #123)
- Fix timeout comment in `tests/tui.sh` (30s).
- Clarify resize behavior and environment variable references in `README.md`.
- Align 15/14/13-row message-window layout comments in `tui.c`.

---

## 3. Testing & Verification Plan

1. **Compiled Unit Tests (`tests/test_tuifmt.c`)**:
   - Status line width assertions ($\le 40$ cols).
   - Quadrant line width assertions ($\le 26$ cols).
2. **TMUX Automated Integration Tests (`tests/tui.sh`)**:
   - Suspend/resume Space bar responsiveness (#190).
   - Suspend-resize immediate screen healing on `fg` (#191).
   - $\le 30$-column width resize (sliver eliminated, #141).
   - $\le 3$-row height resize under pin (clean bottom border, #182).
   - Mid-move warp pause quadrant synchronization (#148).
   - Height-only grow/shrink prompt restoration (#114).
   - Corrected `env size` count assertion (#176).
3. **Local CI Gates**:
   - `cmake --preset ci-debug && ctest --preset ci-debug`
   - `cmake --preset ci-release && ctest --preset ci-release`
   - `ctest --preset debug -R '^lineendings$'`
