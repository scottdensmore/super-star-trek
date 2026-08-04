# AGENTS.md

Shared instructions for every coding agent working in this repository
(Claude Code, Codex, Cursor, Copilot, and others). `CLAUDE.md` imports this
file; keep all rules here so every agent sees the same thing.

## Project

Super Star Trek — a classic terminal (CLI) space strategy game written in C17,
built with CMake. The executable is `sst`. Game documentation lives in
`sst.doc`; shared declarations live in `sst.h`.

Global game state uses the `EXTERN`/`INCLUDED` trick in `sst.h`: exactly one
translation unit per binary defines `INCLUDED` before including `sst.h` to
instantiate the globals (`sst.c` for the game; a test's own file, e.g.
`tests/test_tuifmt.c`, for test binaries that don't link `sst.c`).

## Build and run

```sh
cmake -S . -B build -DCMAKE_BUILD_TYPE=Debug   # build type is required; Debug adds -DDEBUG
cmake --build build
./build/sst
```

Note: `CMakeLists.txt` compares `CMAKE_BUILD_TYPE` directly, so configuring
without `-DCMAKE_BUILD_TYPE=...` fails. `-Wall -Wextra` is always on; CI
also configures with `-DSST_WERROR=ON`, so pass that locally to see what
CI will refuse. Feature flags `-DSCORE -DCAPTURE
-DCLOAKING` are always enabled.

## TUI mode (`sst -t`)

- All game output must go through `prout`/`proutn`/`proutf`/`prouts` and all
  input through `scan()`/`getch()`/`readinput()` — raw `printf`/`putchar`/
  `fgets` silently bypasses the full-screen mode.
- Plain mode (no `-t`) must not change as an unintended side effect of any
  change; verify with a piped scripted journey (e.g.
  `printf 'regular\nshort\nnovice\nxyz\nsrscan\nquit\nn\n' | ./build/sst`).
  If a change alters plain-mode output on purpose, say so in the commit
  and explain why.
- Text the game prints must not end in a carriage return. `sst.doc` is
  CRLF, so anything read from it needs both stripped: under curses a
  trailing `\r` returns the cursor to column 0 and the newline then
  erases the line. `pause()` writes a CR deliberately, to wipe its own
  prompt in place -- the one exception, and the reason the test scripts
  filter that shape before checking.
- The panel formatters in `tuifmt.c` must mirror `srscan()` in `reports.c`
  exactly — it is the spec, including sensor-damage masking and the `-f`
  coordinate transposition (x is the column). Keep `tests/test_tuifmt.c` in
  sync (`ctest --test-dir build`).
- Testing the TUI needs a pty: use tmux, and always on your own socket via
  `-L`, with `-f /dev/null` so the config does not follow — `tmux -f /dev/null
  -L sst new-session -d -x 100 -y 30 -s t './build/sst -t; sleep 30'`, drive
  with `tmux -f /dev/null -L sst send-keys -t t 'cmd' Enter` (input is
  line-based; keys need Enter), inspect with `capture-pane -t t -p`, and clean
  up with `kill-server` on that same socket. Without `-f /dev/null` a setting
  as ordinary as `base-index 1` renames the pane out from under you and every
  capture comes back empty. The default socket is where the
  human running you keeps their own work: on 2026-08-01 a review agent tidied up
  after a TUI check with a bare `tmux kill-server` and destroyed a week-old
  session of eight panes, including the agent's own parent. `-L` is what makes
  that mistake impossible rather than merely discouraged — never issue a bare
  `tmux kill-server`, and never `kill-session` a session you did not create.
  Never redirect the game's stdout in the session — it breaks curses rendering.
  `tests/tui.sh` does all of this automatically and is the place to add
  coverage; it skips without tmux, but fails rather than skipping when `$CI` is
  set.
- Check the TUI at 80x24 and 72x24, not just at a comfortable size. 72x24 is
  the smallest it accepts and 80x24 is what most terminals open at; the
  message window is only ten lines at both, which is where the paging bugs
  live.

## Development workflow

Every coding agent working in this repository must follow this workflow.

1. **Inspect before changing anything.** Inspect the repository, current Git
   state, and all applicable instruction files before making changes. Preserve
   unrelated staged, unstaged, and untracked work.

2. **Create a branch first.** Create a dedicated feature, fix, refactor, chore,
   test, or documentation branch before making code changes. Never commit
   directly to `main`, and create the branch from the latest appropriate
   `main` state.

3. **Choose a thin vertical slice.** Before implementing a roadmap item or
   feature, define the smallest end-to-end slice that can be reviewed, tested,
   shipped, and merged independently. Prefer one coherent user-visible or
   operational outcome over a broad horizontal layer.

4. **Use test-driven development when behavior or structure is testable.**
   - Add or update a focused test before implementation.
   - Run it and confirm it fails for the expected reason.
   - Implement the smallest appropriate change.
   - Run focused tests while iterating.
   - Refactor only while the relevant tests remain green.

5. **Inspect the complete diff.** Review the branch diff plus all staged,
   unstaged, and untracked files. Remove accidental or unrelated changes while
   preserving work that belongs to the user.

6. **Run `ui-review` before verification.** After the main agent completes an
   implementation pass, invoke the `ui-review` sub-agent. The `ui-review`
   sub-agent must act as an expert in games and CLI games.

7. **Run `verifier` before code review.** Invoke the `verifier` sub-agent to
   run the builds, static checks, tests, and journey coverage appropriate for
   the change. The verifier must report failures, flakes, missing coverage,
   and environment issues. Fix or explicitly resolve every actionable finding
   before starting code review. If a verifier finding requires a code change,
   rerun the verifier after addressing it.

8. **Run `code-review` before every commit.** Invoke the `code-review`
   sub-agent against the current branch diff and every staged, unstaged, and
   untracked file. The reviewer must act as an expert in the languages and
   frameworks used by this application. Address every actionable finding
   before committing. If review findings cause changes, rerun the appropriate
   tests and the `verifier`, then obtain a fresh `code-review` approval for
   the changed state.

9. **Commit after approval.** Commit only after verification and code review
   are complete. Use Conventional Commits:

   ```text
   <type>(<scope>): <imperative summary>
   ```

   Keep the subject at 72 characters or fewer, describe why in the body when
   useful, and do not combine unrelated work.

10. **Create pull requests from the reviewed state.**
    - Confirm that local verification remains valid.
    - Rerun `code-review` only if the reviewed state changed after the
      pre-commit review.
    - A changed state includes code, tests, documentation, generated files,
      conflict resolution, or any other staged, unstaged, or untracked
      content.
    - Do not repeat code review when the already-reviewed diff and worktree
      remain unchanged.
    - Push and create the pull request only after local verification and any
      required code review are complete.
    - Open a normal, ready-for-review pull request by default. Do not open
      draft pull requests unless the user explicitly asks for a draft.

11. **Merge only clean, passing pull requests.** Merge only after GitHub
    reports a clean merge state and every configured check passes. Never
    bypass a failing or pending required check. Self-merges are allowed when
    these conditions are met. Use squash merge for short-lived development
    branches to keep `main` linear, then delete the merged branch.

Note: steps 6-8 name Claude Code sub-agents defined in `.claude/agents/`.
Agents without sub-agent support should perform the equivalent review,
verification, and code-review passes themselves before committing.
