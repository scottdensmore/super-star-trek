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

Display state (`tui_active`, `tui_ingame`) deliberately sits outside that
mechanism — plain `extern` in `tui.h`, defined in `tui.c` — so `tui.h`
stays self-contained for `osx.c`, which cannot include `sst.h`. Put new
display flags there, not in `sst.h`.

`osx.c` is the file that cannot include `sst.h`: `sst.h` declares
`pause(int)`, which collides with the POSIX `pause(void)` that `osx.c`
needs. So what `osx.c` *provides* is declared in `osx.h` (included by
both `osx.c` and `sst.h`), and what `osx.c` *calls* it gets from
`tui.h`. Put new platform functions in `osx.h`, not `sst.h`, so the
definition stays checked against the declaration — `-Wmissing-prototypes`
is on and CI builds with `-Werror`.

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

A failing `golden` test means a journey's output changed. Work out why
and say so in the commit -- the fixture diff is the evidence a reviewer
needs. Never re-record with `tests/golden.sh <sst> --update` to turn CI
green without that explanation; the recordings are the only thing in
the suite that checks the game's arithmetic at all.

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
   preserving work that belongs to the user. What you drop because it was
   unrelated is often worth filing; see step 9.

6. **Run `ui-review` before verification.** After the main agent completes an
   implementation pass, invoke the `ui-review` sub-agent. The `ui-review`
   sub-agent must act as an expert in games and CLI games. Address every
   actionable finding before moving on, as with the two steps below. If you
   are not Claude Code, read "The review sub-agents" after this list before
   running this step or the two that follow.

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

9. **File what you found outside the slice.** Work turns up problems that
   belong to no slice in particular: a defect the change did not cause, a
   test that cannot fail, a comment that is no longer true. Open a GitHub
   issue for each as you find it, without waiting to be asked — a finding
   that lives only in a session summary is a finding nobody will act on.
   Standing here at step 9 makes this the last check that nothing was left
   unfiled, not the moment to begin. The main agent files them; the review
   sub-agents only report.

   - Search the open issues first. If it is already filed, add what you
     learned to that issue instead of opening a second one.
   - File a real defect — anything that would mislead a player or a
     maintainer — and a coverage gap, meaning a test that cannot fail or
     that asserts less than it appears to. Leave style preferences and
     speculative ideas out; they belong in the pull request body. When it
     is genuinely unclear, file it: a wrong issue costs one close.
   - **Do not fix it in this slice.** An unrelated fix buried in an
     unrelated diff is how both the fix and the finding go unreviewed. Fix
     it here only if it belongs to the slice.
   - That is not license to file what steps 6-8 require you to fix. A
     `ui-review`, `verifier` or `code-review` finding about *this* change is
     resolved here; filing it is not resolving it. What gets filed is what
     those reviews turn up about code the change did not touch, and
     whatever you meet while reading for some other purpose.
   - Give the issue what the next person needs: what is wrong, why it
     matters, where it was found, and how to see it, citing `sst.doc` or a
     file and line where one applies.
   - Where the work depends on the unfixed behavior, name the issue in a
     comment beside it. Write that comment as part of the implementation,
     so it reaches the reviewer rather than arriving after approval.
   - Say which issues were filed, in the pull request body and in the
     summary of the work — in the summary alone when there is no pull
     request. An agent that cannot open issues says so plainly and lists
     the findings in the same places, marked as unfiled, so a human can
     file them. Silence is the one outcome this step exists to prevent.

10. **Commit after approval.** Commit only after verification and code review
    are complete. Use Conventional Commits:

    ```text
    <type>(<scope>): <imperative summary>
    ```

    Keep the subject at 72 characters or fewer, describe why in the body when
    useful, and do not combine unrelated work.

11. **Create pull requests from the reviewed state.**
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

12. **Merge only clean, passing pull requests.** Merge only after GitHub
    reports a clean merge state and every configured check passes. Never
    bypass a failing or pending required check. Self-merges are allowed when
    these conditions are met. Use squash merge for short-lived development
    branches to keep `main` linear, then delete the merged branch.

### The review sub-agents

Steps 6-8 invoke Claude Code sub-agents defined in `.claude/agents/`:
`ui-review`, `verifier` and `code-review`. What matters is not the tool. It
is that the work is judged by a reader who did not write it, and who
reaches its conclusions from the diff rather than from a memory of having
intended something.

An agent that is not Claude Code gets the same three passes, and does not
skip them.

- **A reviewer is a fresh session holding the reviewer's instructions, the
  change, and nothing else.** A sub-agent where the harness has them; a
  second non-interactive invocation of your own CLI — `codex exec` and its
  equivalents — where it does not. A new session reading the change cold
  satisfies this. The same session in a later turn does not.
- **Point at the definitions here rather than restating them.**
  `.claude/agents/ui-review.md`, `verifier.md` and `code-review.md` are the
  specifications. A per-tool definition should say to read the matching
  file and follow it, and carry only the wiring its own harness needs: the
  name, and that the reviewer reports findings and never edits. Copying the
  text is how three tools come to review three different things, which is
  the whole reason this file exists.
- **Keep the three names.** The workflow above, the commit history and the
  issue tracker all refer to them by name.
- **Land the definition as its own change,** on its own branch and commit,
  before the work it will review — not beside a feature, which step 10
  forbids and step 3 argues against. It belongs in the repository rather
  than in someone's local configuration for the same reason this file
  does: the next session in that tool then inherits a reviewer instead of
  improvising one, and an improvised reviewer drifts. That commit is also
  the one place self-review cannot be avoided, since step 8 cannot be
  satisfied by the commit that creates the reviewer. Say so in the commit,
  and disclose it as below.
- **Confirm the harness actually loads what you wrote.** A definition in a
  directory the tool never reads is inert, and from the outside it looks
  exactly like a step that was carried out.
- **Self-review is the last resort, not the default.** Taking it means
  saying which mechanism you looked for and why it was not there — or, for
  the commit that creates the reviewer, that it did not yet exist. Say it
  in the pull request body and in the summary of the work — in the summary
  alone when there is no pull request. A reviewer who shares the author's
  memory of what was intended is the one thing this section exists to prevent.
