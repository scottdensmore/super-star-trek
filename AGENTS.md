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
     can rewrite the original's build files on the way past. Every test
     here takes the binary as an argument, so the run is
     `tests/tournament.sh <copy>/build/sst Debug`, or `ctest --test-dir
     <copy>/build` for the compiled ones. A `git worktree` will not do: it
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
      pre-commit review.
    - A changed state includes code, tests, documentation, generated files,
      conflict resolution, or any other staged, unstaged, or untracked
      content.
    - Do not repeat code review when the already-reviewed diff and worktree
      remain unchanged. The `verifier` follows the same condition, and so
      does answering a Codex comment: what makes either pass run again is
      a changed state, not the arrival of a comment. A finding answered by
      argument alone leaves both passes nothing they have not already
      seen, and a round of findings answered by edits takes one rerun over
      the result rather than one per finding.
    - Push and create the pull request only after local verification and any
      required code review are complete.
    - Open a normal, ready-for-review pull request by default. Do not open
      draft pull requests unless the user explicitly asks for a draft.
    - Opening a ready-for-review pull request normally starts a Codex
      review, and a push may start another. "The Codex review" below is
      what to do with what it says.

12. **Merge only clean, passing pull requests.** Merge only after GitHub
    reports a clean merge state and every configured check passes. Never
    bypass a failing or pending required check. Wait as well for the
    Codex review to come back 👍 with a `created_at` later than your last
    push, which is what says it read the state being merged. It is not a
    check and nothing enforces it — "The Codex review" below says how to
    read one and when to stop waiting. Where you stop, say so in the
    pull request body and in the summary of the work. Self-merges are
    allowed when these conditions are met.
    Use squash merge for short-lived development branches to keep `main`
    linear, then delete the merged branch.

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

### The Codex review

`chatgpt-codex-connector[bot]` reviews pull requests in this repository,
and nobody has to invoke it: opening a ready-for-review pull request
normally starts a review, which is what its documentation promises where
automatic reviews are turned on. It is not the `codex exec` of "The
review sub-agents", and it is not the other bot that comments on pull
requests here.

It answers with a reaction on the pull request itself. 👀 while it is
working, and 👍 when it has finished with nothing to say. 👍 is not
approval — it is one reader reporting that this pass found nothing.
Otherwise it posts review comments. Every review it had filed here as
of 2026-08-09, when #104 was written, was a plain commenting review
rather than an approval or a request for changes — two of them, on #99
and #104. A count like that goes stale the next time it reviews
anything, which is why it is dated. `gh api
repos/:owner/:repo/pulls/<n>/reviews` is what brings it forward.

A review it posts says which state it read, in its own body: **Reviewed
commit:** and a short sha. That settles the question outright and with
no arithmetic, so where there are comments, read that rather than
comparing times. It is only the silent pass that needs the rest of
this, since a pass with nothing to say files no review to carry a sha.

Those comments bind as `code-review`'s do: fix, or resolve explicitly,
on the standard "Answering a finding" sets, with the record going where
that section says. Reply on the thread as well — that is where a
disagreement reaches the reviewer, and where the next reader looks —
and resolve the thread once it is answered. Where the answer was a fix,
push it and let the review run again on what you pushed. Where it was a
resolution, there is nothing to push and no further review to wait for.

Reading the answer takes one API call. The bot swaps its own reaction
as it goes rather than accumulating them: 👀 when a review starts, 👍
when that review finishes with nothing to say, and where it has
something to say it takes the 👀 away and posts comments instead. So the
reaction tracks the latest review, and the question worth asking is not
whether a 👍 is there but whether it is newer than your last push. That
is the one thing the web interface does not show. `gh api
repos/:owner/:repo/issues/<n>/reactions` gives `created_at`, with
`content` reading `eyes` and `+1` rather than the emoji.

Filter it on both fields, or step 12's gate opens on things that are
not an answer. That endpoint returns every user's reactions, and a
human thumbs-up carries the same `+1` as the bot's, so `content` cannot
tell them apart — a maintainer adding one to their own agent's pull
request, an ordinary thing to do and likelier the more this workflow
gets used, would clear the gate with no Codex pass having run. And the
bot's own 👀 is a reaction too, newer than your push by five or ten
seconds, so filtering on the user alone hands you a review that is
still running as though it had answered. Both, then:

    --jq '.[] | select(.user.login == "chatgpt-codex-connector[bot]"
                       and .content == "+1")'

The push time it is compared against takes a second call: `gh api
--paginate repos/:owner/:repo/events`, and the newest `created_at`
among the entries naming your branch. Newest, not first — the feed is
not strictly ordered by time. And two event types, not one: a branch's
first push files no `PushEvent` at all, only a `CreateEvent` whose
`payload.ref` is the bare branch name where a `PushEvent` spells it
`refs/heads/<branch>`. Match only the second and the ordinary case —
branch, one push, open, merge — comes back empty, which is every branch
in this repository that was never pushed to twice. `repos/:owner/:repo`
carries a `pushed_at` as well, but that one is repo-wide and names no
branch: all three of the pushes timed below that filed a `PushEvent`
shared their second with a push to another branch, so `pushed_at` could
not have told you which one it was reporting even where it agreed with
them. No other push shared the second of the two that filed a
`CreateEvent`, which is luck rather than a rule.

That feed is not built for real time. The vendor puts its lag at
anywhere from thirty seconds to six hours, against the few minutes a
review takes, so nothing found three minutes after a push means the
feed is behind rather than the push missing. It also ends: ninety days
or three hundred events, whichever comes first, and three hundred is
what this repository holds today. Drop the `--paginate` and you get the
first thirty of those, which here is about a day — long enough to work
on every branch you try it on and fail on the one that sat open over a
weekend. Against all three — the lag, the limits, the flag — the
fallback is the same and costs nothing: note the time as you push, and
none of them can reach you. Note it *after* the push returns, though.
A time taken before it is a lower bound, so anything the bot does in
the gap reads as having come after your push when it came before — and
the gap is not small enough to ignore: the note taken here went in six
seconds early, because the shell read the clock before `git push` had
finished sending. The measurements below are timed off the feed for
that reason, poll times included. What will not do at all is a commit's
own date, which is a lower bound by more: the commit behind the
06:34:17Z push was made twenty-four seconds before it reached the
remote, and a commit written before a rebase or a review round is off
by however long that took. (Shas are avoided here on purpose. Step 12
squash-merges and deletes the branch, so a commit named on it stops
resolving in a fresh clone of `main` — dates and pull request numbers
survive that, and `payload.head` in the feed gives the sha back.)

A pass that finds nothing files no review at all — no review object, no
comments, only the reaction changing. So an unchanged comment count is
not evidence that nothing ran, and the reaction is the whole of the
answer.

The wait is short, and its range is known. Five runs are timed here and
they span two minutes twenty-two to five minutes thirty-nine: #99
answered 2m22s after the activity at 00:57:22Z, #104's two pushes drew
answers in 2m24s and 3m50s with 👀 up within ten seconds on both,
#104's opening review took 3m39s from the pull request opening at
05:46:08Z to its comments, and #102 — the slowest, and no draft —
waited 5m39s from 04:39:34Z to a 👍 at 04:45:13Z. Note that the two
opening runs are timed from the pull request opening, not from the
branch push, which is the event the paragraph above counts: a second
earlier for #104, and the same second for #102. So four minutes is not
the ceiling it looks like from #104 alone. Give it ten; past that,
treat the answer as one that is not coming and take the paragraph below
on saying so.

Two things to know about it.

**It is a service on infrastructure this repository does not control.**
Step 12 asks for its answer; it does not ask anyone to wait indefinitely
for one, and there are ordinary states where no new answer can arrive —
a finding resolved rather than fixed changes no code, so there is no
push to review. Where you want one anyway, a comment saying
`@codex review` is the documented way to ask. Where the answer still
does not come, say so in the pull request body and in the summary of the
work, and let whoever merges decide.

**It reads this file.** Its comments here cited `AGENTS.md` by line
range, at rules rather than at the diff, so it is better informed than a
stranger and worse informed than the passes in steps 6-8: it has the
pull request conversation but not the rounds behind it, which happen in
sessions that leave no artifact, and not the reason a rule is worded the
way it is. It found a defect in this workflow's own test that the passes
before it had missed, which is the argument for reading it closely. It
is not an argument for doing what it says without judging it.

What is measured here rather than read, and by which pull request.

That a push started a fresh review: on #104, a push at 06:34:17Z drew
👀 five seconds later with nothing else touched on the pull request, and
👍 at 06:36:41Z. The next push to the same pull request did it again,
ten seconds and three minutes fifty, again with nothing else touched
between the push and the answer. #99 looks like a third — a push at
00:57:22Z, 👍 at 00:59:44Z — but two thread replies of mine went in at
00:57:22Z and 00:57:23Z, so which of the three that review answered is
not something anyone watched. A reply is not a documented trigger, but
neither is a push: the whole point of this entry is that the published
list is short, so it cannot be used to acquit one unlisted candidate
and convict another. Two clean instances rather than a promise, which
is why step 11 says a push *may* start one — the bot's own note
lists its triggers without a push among them. A trigger list that does
include every push is documented for Codex Security Review, a different
feature and not evidence about this one.

That the reaction cycles rather than accumulates: that 👀 was a re-add.
The same reaction had gone three quarters of an hour earlier, when the
review before it had comments to post at 05:49:47Z. GitHub keeps no
history of a reaction taken back, so that withdrawal is dated from
those comments rather than watched, and is already unrecoverable —
which is the reason to have written it down.

That it withdrew a standing 👍 rather than leaving it in place, once,
watched: on the push that carried the prediction this entry replaces.
#104 was carrying a 👍 from 06:36:41Z when that push landed at
23:35:58Z. A poll at 23:36:05Z, seven seconds later, still found that
👍; the next at 23:36:17Z, nineteen seconds in, found it gone and 👀 in
its place carrying a `created_at` of 23:36:08Z. Nothing looked in
between, so the withdrawal happened somewhere in those twelve seconds,
and the ten-second figure is the 👀's arrival rather than the 👍's
departure — they are the same instant only if the two never overlapped,
which is exactly the inference the entry above refuses to make. At
23:39:48Z — three minutes and fifty seconds after the push — 👀 gave
way to a 👍 of its own, with no review and no comments filed. The
paragraph this entry replaces called the case unwatched and named the
push that would settle it, which makes this the only experiment here
the file called in advance.

That a 👍 can arrive where none stood: the 06:34 run, and what step 12's
rule first rested on. #104 was carrying nothing at all when that push
landed — its opening review had taken the 👀 away when it posted
comments — so what the run disproved was the *conclusion* an earlier
draft had drawn, that no 👍 could ever postdate your push and the rule
was therefore unwaitable. What that run could not reach is the claim
underneath, that a 👍 already standing cannot be replaced by a newer
one; the 23:36 run reached that and disproved it. The draft rested both
on a supposed platform rule, one reaction to a user per issue, which is
not a rule — the limit is one reaction of each *type*, which is how a 👀
and a 👍 come to sit on a pull request together. What swaps them here is
the bot, as behaviour, and it is measured above rather than promised
anywhere.

That a pass finding nothing files no review at all: the same run once
more. The 👍 arrived with no review object and no comments behind it,
which is what the paragraph on comment counts above rests on. #99 shows
it too, and #102 as well, and those two are where a reader can still
check: both are merged with their branches gone, so nothing will ever
be pushed to them again and their 👍s are frozen where they landed.
#104's from 06:36:41Z is gone, withdrawn by the cycle the next push
started, and whichever 👍 stands on it as you read this will go the same
way the moment anything else is pushed. A reaction is evidence only
until the next push, which is why the two frozen ones carry this and
#104 has to be taken on trust.

None of those is on the documentation page. That 👍 means finished with
nothing to say *is* published, though not there: it is in the collapsed
note at the foot of every review it posts.

What was still unwatched as of 2026-08-09, and what would settle each.
These are the entries most likely to be out of date by the time you
read them, since the thing that settles one is an ordinary review.

A commenting run that starts from a standing 👍: both commenting runs
to that date began from no reaction, each being the review a pull
request draws on opening, so whether a 👍 goes the way a 👀 does when
the review has something to say is untested. #104 was carrying one, so
the next round of findings on it settles this — quite possibly the
review of the push that landed this paragraph.

A 👀 withdrawal watched rather than inferred: none of the three here
was. The opening review's is dated from the comments that replaced it
at 05:49:47Z, the 06:34 one from the 👍 that followed at 06:36:41Z, and
the 23:36 one from its 👍 at 23:39:48Z — each from the thing that came
next, not from watching the reaction go. Even the 23:35 run, which
polled closely enough to bracket the 👍's departure, stopped polling
once 👀 was up and so watched its own 👀 no better than the others. A
commenting run polled the whole way through would date one directly,
and the same review settles this and the entry above it.

Whether `@codex review` starts anything here: documented, and never
once used in this repository to that date. The advice above, on what to
do when no answer comes, tells an agent to use it — so the first one
who does ends this entry, and should say what happened.

None of this changes what you do, which is to compare `created_at`
against the push and read the time rather than the presence. It matters
to the reader who glances at the page instead: the stale 👍 was still
up seven seconds after the push and gone by nineteen, so for something
like the first twenty seconds the reaction on screen is the last
review's and looks exactly like this one's. That bracket is one run,
the only one anyone here has watched, and nineteen seconds is a
measurement rather than a promise — which is the argument for reading
the timestamp and never the icon.
