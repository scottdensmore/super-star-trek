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

`CMakePresets.json` is the single source of truth for how this project is
built and tested. Use a preset; do not spell the flags out.

```sh
cmake --preset debug
cmake --build --preset debug
./build/debug/sst
ctest --preset debug            # --output-on-failure is already in the preset
```

Four presets, each with its own build directory under `build/`:

| Preset | Build type | `-Werror` |
| --- | --- | --- |
| `debug` | Debug (adds `-DDEBUG`) | no |
| `release` | Release | no |
| `ci-debug` | Debug | **yes** |
| `ci-release` | Release | **yes** |

The `ci-*` presets are what CI runs, command for command, so
`cmake --build --preset ci-debug` is how you find out what CI will refuse
before it refuses it. The flags themselves live in `CMakeLists.txt`, not in
the presets: `-Wall -Wextra -Wmissing-prototypes` on GCC and Clang, and the
feature flags `-DSCORE -DCAPTURE -DCLOAKING` on every compiler. A preset
decides only the build type and whether warnings are fatal.

Configuring by hand still works, but it is how the recipe drifted in the
first place. Five files wrote it down, three of them identically. Only CI
carried `-DSST_WERROR=ON`, and `verifier.md` — the pass that stands
between a change and CI — omitted it where the omission mattered, and
built somewhere else besides. So the one build meant to predict CI was
the one build that could not. A sixth file, `tests/golden.sh`, named only
the binary's path, which is why a sweep for the recipe missed it. If you
find yourself typing `-DCMAKE_BUILD_TYPE=`, you want a preset. Add one to
`CMakePresets.json` rather than passing flags a reader of this file cannot
see. (`CMakeLists.txt` compares `CMAKE_BUILD_TYPE` directly, so a bare
`cmake -S . -B build` fails outright.)

## TUI mode (`sst -t`)

- All game output must go through `prout`/`proutn`/`proutf`/`prouts` and all
  input through `scan()`/`getch()`/`readinput()` — raw `printf`/`putchar`/
  `fgets` silently bypasses the full-screen mode.
- Plain mode (no `-t`) must not change as an unintended side effect of any
  change; verify with a piped scripted journey (e.g.
  `printf 'regular\nshort\nnovice\nxyz\nsrscan\nquit\nn\n' | ./build/debug/sst`).
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
  sync (`ctest --preset debug -R '^tuifmt$'`).
- Testing the TUI needs a pty: use tmux, and always on your own socket via
  `-L`, with `-f /dev/null` so the config does not follow — `tmux -f /dev/null
  -L sst new-session -d -x 100 -y 30 -s t './build/debug/sst -t; sleep 30'`, drive
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

## The golden recordings

A failing `golden` test means a journey's output changed. Work out why
and say so in the commit -- the fixture diff is the evidence a reviewer
needs. Never re-record with `tests/golden.sh <sst> --update` to turn CI
green without that explanation; the recordings are the only thing in
the suite that checks the game's arithmetic at all.

## Development workflow

Every coding agent working in this repository must follow this workflow.

1. **Inspect before changing anything.** Inspect the repository, current Git
   state, all applicable instruction files, and the issue or roadmap item the
   work comes from, before making changes. Preserve unrelated staged,
   unstaged, and untracked work.

2. **Create a branch first.** Create a dedicated feature, fix, refactor, chore,
   test, or documentation branch before making code changes. Never commit
   directly to `main`, and create the branch from the latest appropriate
   `main` state.

3. **Choose a thin vertical slice.** Before implementing a roadmap item or
   feature, define the smallest end-to-end slice that can be reviewed, tested,
   shipped, and merged independently. Prefer one coherent user-visible or
   operational outcome over a broad horizontal layer.

   The slice inherits whatever the issue assumes, so check what it claims
   before building on it. A claim you can settle — what the code does, what
   `sst.doc` says at a given line — you settle, and where it does not hold
   you say so on the issue in a comment: what it claimed, what is actually
   the case, and the line that shows it. Comment, do not rewrite the issue
   body; a comment keeps straight who said what. A closed issue still
   takes one, and an issue closed by the pull request that got the premise
   wrong is exactly where the correction belongs. Where you cannot comment
   at all — a locked issue, a harness with no access to the tracker — say
   it in the pull request body and the summary of the work, marked as
   unposted.

   What is worth building stays the issue author's judgment. A wrong premise
   voids the claim it supports, not the issue, and the acceptance criteria
   it never touched still bind. Where the facts change what the issue
   ought to ask for, say so and ask — do not swap in a slice of your own.
   Say what was wrong and what the correction changed about the slice, in
   the pull request body and in the summary of the work, in the summary
   alone when there is no pull request. Silence is worse than either:
   building on a mistake you have seen, or building something else and
   leaving the issue describing work nobody did.

   #58 is the example, and it reasoned well from one wrong fact. On its
   main ground it ruled out checking the galaxy's digit packing because
   that packing "appears nowhere in `sst.doc`" — where the manual gives it
   at `sst.doc:436-439`, as the meaning of the long-range scan. The
   instinct was the one step 4 now endorses, to take an expectation from
   somewhere independent of the code; only the fact about where that
   source was silent was wrong. It cost a narrower slice than the facts
   allowed, not the wrong one.

4. **Use test-driven development when behavior or structure is testable.**
   - Add or update a focused test before implementation.
   - Run it and confirm it fails for the expected reason.
   - Implement the smallest appropriate change.
   - Run focused tests while iterating.
   - Refactor only while the relevant tests remain green.

   Run that loop yourself rather than delegating it. "Who runs which
   tests" after this list says why, and which tests belong to the
   `verifier` instead.

   A test written against behavior that already exists — a recording, a
   check of something the game has always done — never has that failing
   first run to show for itself, and that is where tests which assert
   nothing come from. Prove it can fail some other way.

   - Break what it checks, watch the test report it, and write down what
     was broken both in the commit and in a comment beside the assertion.
     The commit is invisible to whoever next edits the test.
   - Where the test compares against a literal in its own file, changing
     the literal proves it in a second. What follows is for tests that run
     the whole game and read what it printed.
   - Break it outside the repository. Copy the working tree — the current
     one, uncommitted test and all — somewhere else, leave `build/`
     behind, and configure a fresh one there: a copied `build/` still
     names the original source directory in its cache, so building in the
     copy compiles the original's sources — your break never reaches the
     binary, and the test passes for the wrong reason. It prints a
     cache-path error and exits 0 while it does it, and a regeneration
     can rewrite the original's build files on the way past. Most tests
     here take the binary as an argument. Run from inside the copy:
     `cd <copy> && tests/tournament.sh build/debug/sst Debug`, or `ctest
     --preset debug -R '^(tuifmt|rules)$'` for the two compiled ones.
     From inside, because for some of them the script decides which tree
     is read and not the binary: `help` and `tui` find `sst.doc` from
     their own path, and `golden` its fixtures. The original's script
     pointed at the copy's binary proves the original's data and comes
     back green — the same wrong-reason pass as the stale `build/`
     above. The two that read the tree instead, `instructions` and
     `score`, take `.` from in there. A `git worktree` will not do: it
     holds `HEAD`, not the test you have just written.
   - Break the game, not the test — and break it somewhere the test's
     expectation does not come from. A break that both sides of the
     comparison descend from cancels out: setup writes the galaxy word
     wrong, the code that reads it back fills the quadrant to match, both
     scans agree, and nothing moves. Rounding, clamping and sensor masking
     swallow a break the same way. Prefer a path where the expected value
     comes from somewhere independent — `sst.doc`, a literal, a subsystem
     that does not share the representation.
   - A green run means one of two things: the test is blind, or the break
     was. Rule out the second before touching the test.
   - This proof, unlike the loop above, is one a sub-agent can run for you.
     "Who runs which tests" says when that is worth doing, and what the
     sub-agent has to report back for the record this asks you to keep.
   - Where a test reaches its subject only on some of the data it runs
     over — one seed, one galaxy, one game that happened to dock — assert
     that the case it needs really occurred. Otherwise the day the data
     stops providing it, the test keeps passing and says nothing.

   Three assertions in #64 passed against builds that broke them: one
   counted rows from scans it was not looking at, one searched for a line
   the game does not print at the skill it ran at, and one multiplied a
   term that was zero in every galaxy the battery then played — a seed
   whose opening quadrant holds a starbase had to be added before that
   assertion meant anything. All three read convincingly.

   **The same holds for what a comment claims, testable change or not.**
   Comments here are read as specification: the review passes take them
   as the reason the code is the way it is, and dispute them on those
   terms. So a comment that states a number, a bound, or what can and
   cannot reach the code should say where that came from — measured on a
   terminal resized to 72x14, counted in `reports.c`, derived from
   `PANELH`, read off the gate in `tui_init()`. A claim carrying its
   source is one the next reader checks in a minute; one without it they
   have to trust or re-derive.

   Not a call for precision in general. It is the sentence that says
   *why* that a later change quietly falsifies, and that a reviewer
   cannot dispute without redoing the work. #156 collects what prompted
   this, from three changes in one session: "the longest line that gets
   here is the 57-character skill question", where 57 was right and the
   reachability wrong — the line is whatever the game last finished; "a
   height shrink takes the pending line whole", stated in three places,
   where a wrapped pair can lose its lower rows and keep its first; and a
   notice described as printing only below 72 columns, where the gate is
   `LINES < 24 || COLS < 72` and it prints on a 100-column screen. Twice
   the wrong claim was inside text written to answer the round before,
   about exactly this.

   That third one is the reason this is a rule and not an aspiration. It
   shipped in #18 and stayed on `main` until #155 — every pass that read
   the file in between took it on trust. A wrong comment can outlive the
   review that should have caught it.

5. **Inspect the complete diff.** Review the branch diff plus all staged,
   unstaged, and untracked files. Remove accidental or unrelated changes while
   preserving work that belongs to the user. What you drop because it was
   unrelated is often worth filing; see step 9.

6. **Run `ui-review` before verification.** After the main agent completes an
   implementation pass, invoke the `ui-review` sub-agent whenever the change
   can alter what the game shows a player or asks of them. The `ui-review`
   sub-agent must act as an expert in games and CLI games. Fix every
   actionable finding, or resolve it explicitly, before moving on — the
   standard that "Answering a finding" below sets for all three passes.
   If fixing one changes what the game shows a player, rerun `ui-review`
   on the result: the fixes made for #46 introduced a doubled echo, a
   duplicated question, a line cut mid-word and a cursor left where the
   next keystroke typed over the answer, each of them found by a later
   round rather than by the one that caused it. Read "The review
   sub-agents" after this list for when this pass binds, what to say when
   it does not, and — if you are not Claude Code — how to get any of
   these three passes at all.

7. **Run `verifier` before code review.** Invoke the `verifier` sub-agent to
   run the builds, static checks, tests, and journey coverage appropriate for
   the change. The verifier must report failures, flakes, missing coverage,
   and environment issues. Fix every actionable finding, or resolve it
   explicitly, before starting code review. If a verifier finding requires a
   code change, rerun the verifier after addressing it.

   The battery is the `verifier`'s work and not the main agent's, which
   is what "Who runs which tests" after this list is about.

8. **Run `code-review` before every commit.** Invoke the `code-review`
   sub-agent against the current branch diff and every staged, unstaged, and
   untracked file. The reviewer must act as an expert in the languages and
   frameworks used by this application. Fix every actionable finding, or
   resolve it explicitly, before committing. If review findings cause changes,
   rerun the appropriate tests and the `verifier`, then obtain a fresh
   `code-review` approval for the changed state.

   Tell it which of the passes above ran, whether the last `ui-review`
   round saw the state being committed, and where `ui-review` did not
   run, why. The middle one is what makes the rerun rule in step 6
   auditable: a fix made after that pass reported is a change it never
   saw, and "it ran" is a true answer that hides it.

   The skip itself is the author's judgment about the author's own
   change — unlike the `verifier`'s scope, which the `verifier` decides
   and reports for itself. `code-review` binds on every commit whatever the
   change touched, so it is the pass certain to see the claim, and to be
   able to dispute it while the work can still change. Left to the commit
   and the pull request, the decision reaches a reader only once the work
   is finished, which is disclosure rather than review.

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
   - That is not license to file what steps 3, 4 and 6-8 require you to
     fix. A premise the slice rests on is corrected under step 3, not
     deferred; a test of your own that cannot fail is fixed under step 4. A
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

10. **Commit once the passes are answered.** Commit only after verification
    and code review are complete, with every finding answered — a finding
    resolved rather than fixed needs no fresh approval, which is why this
    does not say "after approval". Use Conventional Commits:

    ```text
    <type>(<scope>): <imperative summary>
    ```

    Keep the subject at 72 characters or fewer, describe why in the body when
    useful, and do not combine unrelated work.

11. **Create pull requests from the reviewed state.**
    - Confirm that local verification remains valid.
    - Rerun `code-review` only if the reviewed state changed after the
      pre-commit review. A changed state includes code, tests,
      documentation, generated files, conflict resolution, or any other
      staged, unstaged, or untracked content.
    - Do not repeat code review when the already-reviewed diff and
      worktree remain unchanged. The `verifier` follows the same
      condition: what makes either pass run again is a changed state. A
      finding answered by argument alone leaves both passes nothing they
      have not already seen, and a round of findings answered by edits
      takes one rerun over the result rather than one per finding.
    - Push and create the pull request only after local verification and
      any required code review are complete.
    - Open a normal, ready-for-review pull request by default. Do not
      open draft pull requests unless the user explicitly asks for a
      draft.

12. **Merge only clean, passing pull requests.** Merge only after GitHub
    reports a clean merge state and every configured check passes. Never
    bypass a failing or pending required check. Self-merges are allowed
    when these conditions are met.
    Use squash merge for short-lived development branches to keep `main`
    linear, then delete the merged branch.

### Who runs which tests

Steps 4 and 7 both run tests, and they are not running the same ones.

**The main agent runs the test that is the subject of the change. The
`verifier` runs the rest of the battery.** Not everything else: the piped
plain-mode journey above and the 80x24/72x24 TUI check stay with whoever
made the change, since they are how you see your own work. The line does not
fall between unit tests and journeys; it falls on what the change is about.
A change to the pager makes `tests/tui.sh` the focused test and it belongs
in the loop, slow as it is. A change to `tuifmt.c` makes the compiled
`tuifmt` test the focused one, fast as it is, and leaves `tui` and the rest
to the `verifier` — including the golden recordings, which never pass `-t`
and so cannot see that file's panel formatters at all, whatever else in it
they reach.

Step 4's loop cannot be delegated at all, and not because a sub-agent cannot
be trusted with it. Each failure decides the next edit, so the run and the
work are one act: what you would hand over is the whole of the task, and a
report arriving afterwards comes after the moment it was supposed to inform.
The entanglement is the reason, so a faster harness would not change it, and
neither would a focused test cheap enough to look worth handing over.

What the `verifier` owns is everything the loop did not reach: the rest of
`ctest`, the full journey and golden battery, and the presets the main
agent did not build — the two `ci-*` ones, which between them cover both
build types with warnings fatal, and whatever the platform adds. That is
the set `verifier.md` names, and the two files have to keep naming the
same one. It decides that scope for itself: step 7 gives it the criterion, and
step 8 says the decision is the verifier's own. This says only that it
should not be asked to skip what the main agent happened to run, and that
the main agent should not pre-run the battery to save it the trouble.
Running the suite twice buys nothing and costs the whole of it: a full
`ctest` here is about four minutes per configuration.

The `ci-*` presets are not optional for that pass. A warning CI will reject
is invisible to every pass that runs before CI, and a `verifier` reporting
"zero warnings" from a build never asked to treat them as errors is making
the stronger claim by accident. It does not need asking for: the presets
carry `-Werror`, so running the `ci-*` pair is the whole of it.

Both of them, which is what `verifier.md` asks for. Release is not the
same binary with the same checks run twice: without `-DDEBUG` the `debug`
command and the code behind it are not compiled in at all, so the
journeys drive something different, and `-Werror` over optimised code
reaches warnings that Debug never emits — on GCC, `-Wmaybe-uninitialized`
is in `-Wall` and needs the dataflow analysis only optimisation runs.
`tests/tui.sh` is the one test that branches on the configuration and the
block it branches on runs only in Debug, so Release is the thinner run
and needs its own pass rather than less of one.

**Keep what reaches the main agent's context small.** That is the reason
this split is worth stating, and it is mechanical:

- `ctest` prints no test output on a pass at all, and the test presets
  already carry `--output-on-failure`, which costs nothing there while
  saving the second run a failure would otherwise need. What is left to
  add is `-R`, scoping it to the test being worked on — anchor the
  pattern, since it is a regex: `-R tui` picks up `tuifmt` and
  `journey-tui` with it. An anchored pattern that matches nothing is an
  error rather than a pass, since the presets set `--no-tests=error`.
- The shell tests print one line on a pass, so they need no filtering. On a
  failure they print what broke and then dump the evidence behind it — a
  pane, a slice of game output, a diff, depending on the script — in that
  order, so it is the first lines that are worth reading and not the last:
  `2>&1 | head` keeps what broke, where `tail` keeps the dump and loses it.
- Grep a tmux pane for the string being asserted rather than capturing it
  whole. `capture-pane -p` is twenty-odd lines of screen and a TUI check
  makes many; capture the pane when the grep fails, which is when its
  content is worth reading.

**The mutation proof in step 4 can be delegated**, unlike the loop in the
step it belongs to, and for the reason the loop cannot: it is one shot,
asked and answered, and what comes back is checkable. The break has a name,
and step 4 wants that name written down anyway, so a report that does not
give it is visibly incomplete.

Not to one of the three review sub-agents — all of them are read-only by
definition, and a proof has to break something. A general-purpose sub-agent
is the one that can, working on a copy outside the repository, so nothing it
touches is tracked. Step 4 already asks for that copy where the proof runs
the game; ask for it whatever the proof breaks, because a sub-agent editing
the tree you are working in is a surprise nobody needs.

What handing it off buys is context and not time, and the two do not track
each other: a proof that runs over three minutes is no cheaper delegated,
where one that prints twelve hundred lines is. So delegate when the failing
run would bury this context, which is a property of the test and worth
looking up rather than guessing — one line broken in `tui_refresh_panels()`
made `tui.sh` report 47 failures and 1209 lines of panes, `tournament.sh`
dumps up to 3000 bytes for every seed that fails, `golden.sh` a diff per
changed journey. Below that, and most proofs are below it — a broken `ctest
-R '^tuifmt$'` reports itself, with `--output-on-failure`, in a few dozen
lines, a broken `help` journey in sixty — there is nothing to bury, so keep
it.

When you do hand it off, the sub-agent has to report what it broke precisely
enough for the main agent to write that down, since step 4 wants it in the
commit and in a comment beside the assertion.

### The review sub-agents

Steps 6-8 invoke Claude Code sub-agents defined in `.claude/agents/`:
`ui-review`, `verifier` and `code-review`. What matters is not the tool. It
is that the work is judged by a reader who did not write it, and who
reaches its conclusions from the diff rather than from a memory of having
intended something.

Which of the three bind depends on what changed. `code-review` binds on
every commit, without exception — it is the gate step 8 describes.
`verifier` binds as well, with the scope step 7 already gives it: the
checks appropriate for the change, which for a file nothing compiles can
come down to a single test — the sub-agent still runs, even when what it
has to run is one test. `ui-review` binds when the change can alter what
the game shows a player or asks of them. This file, a test's plumbing and
CI configuration usually give it nothing to read; the build system does
the moment it changes what gets compiled in, since `-DSCORE`, `-DCAPTURE`,
`-DCLOAKING` and the `-DDEBUG` that Debug builds add all decide which
commands and messages the game has at all.

When it is unclear whether a change reaches the player, run the pass. A
`ui-review` with nothing to say costs one pass; code that runs while the
game runs reaches the player whether the change meant it to or not, and a
refactor that preserves behavior is exactly where a lost line hides.

Skipping it is a decision, not an omission. Say that it was skipped and
why — to `code-review` when invoking it, per step 8, and in the commit,
the pull request body and the summary of the work. The last three put it
on the record; the first is the one that reaches a reviewer with a
mandate to argue back, and reaches it before there is a commit to argue
about, which is why step 8 asks for it as well. An agent writing that
sentence about a change to the game's own output has skipped the wrong
pass.

An agent that is not Claude Code is bound on the same terms, and cannot
satisfy a pass by reviewing its own work.

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

### Answering a finding

Steps 6, 7 and 8 hold their passes to one standard, in the same words:
fix every actionable finding, or resolve it explicitly. Two ways to
answer, and silence is neither.

**Fixing** is the ordinary one, and most findings deserve it.

**Resolving explicitly** is for the finding you judge wrong, or right
about something this change is not. Say what you think is true instead,
in the words you would use if you expected to be argued with.

Where it goes depends on whether anything else is going to read the
change. A resolution alters no code, so it triggers no rerun: steps 6,
7 and 8 each condition theirs on a change, and step 11 forbids
repeating `code-review` over an unchanged worktree. So put it to the
next pass that runs anyway — step 8 already carries `ui-review` facts
into `code-review`, and a `verifier` finding you resolve is one
`code-review` should hear about. For a `code-review` finding resolved
without a change there is no next pass, and the record is the whole
of it.

Either way it goes in the pull request body and in the summary of the
work — in the summary alone when there is no pull request, on the same
terms steps 3 and 9 already set. A resolution nobody can find
afterwards is the silence this rule exists to stop.

What does not count: fixing something adjacent and calling the finding
handled, agreeing and then forgetting, or "I disagree" with no reason
under it. Filing an issue does not answer a finding about *this*
change — step 9 is explicit that filing is not resolving — but a
finding about code the change did not touch is answered by filing it
and saying so, which is step 9 working as intended rather than an
exception to it.
