# AGENTS.md

Shared project instructions for every coding agent. `CLAUDE.md` imports this
file, and `GEMINI.md` points to it; keep project rules here so all tools use
the same source of truth.

## Project overview

Super Star Trek is a classic terminal space-strategy game written in C17 and
built with CMake. The shipped executable is `sst`; player documentation is in
`sst.doc`.

### UI Domain

The UI domain is Terminal/CLI. Plain line-oriented mode and the optional
ncurses full-screen mode (`sst -t`) are both player-facing.

UI-review applicability is based on the effect of a change, not only its file
type. A build- or configuration-only diff still requires UI review when it
changes compiled sources, `DEBUG`, `SCORE`, `CAPTURE`, `CLOAKING`, or another
choice that can alter player commands, prompts, or messages. It is N/A only
when the build/configuration change cannot alter player-visible behavior (for
example, comments, warning policy, or CI plumbing). This repository-specific
contract overrides the generated `ui-reviewer` file-type shortcut.

- **Supported hosts:** Linux and macOS. There is currently no working Windows
  build because the game depends on ncurses.
- **Base branch:** `main`.

## Repo Map

| Area | Location and ownership |
|---|---|
| Entry point and global storage | `sst.c`; it defines `INCLUDED` before `sst.h` to instantiate shared globals |
| Shared declarations and game state | `sst.h` |
| Game logic | Root C files such as `ai.c`, `battle.c`, `events.c`, `moving.c`, `reports.c`, `rules.c`, and `setup.c` |
| Full-screen display | `tui.c`, `tui.h`, and `tuifmt.c` |
| Platform boundary | `osx.c` and `osx.h` |
| Player documentation | `sst.doc` |
| Tests and recordings | `tests/`; compiled tests are `tests/test_*.c`, shell journeys are `tests/*.sh`, and golden fixtures are `tests/golden/*.txt` |
| Build and test definition | `CMakeLists.txt` and `CMakePresets.json` |
| CI gate | `.github/workflows/ci.yml` |
| Generated output — never edit | `build/<preset>/`; regenerate with the matching CMake preset |
| Installed agent workflow | `.agents/` is the canonical installed bundle; tool-specific mirrors live under `.claude/`, `.codex/`, `.cursor/`, and `.github/agents/` |

The build declares no generated or vendored source directory. The generated
agent and skill mirrors are installer-owned; update them by rerunning the
installer that wrote `.agents/agent-skills.json`, not by editing one mirror
independently.

## Development Commands

`CMakePresets.json` is the single source of truth. Run commands from the
repository root and use a preset; do not restate its flags by configuring
CMake by hand.

| Purpose | Command | A green result proves |
|---|---|---|
| Configure everyday Debug build | `cmake --preset debug` | CMake generated `build/debug/` with the declared dependencies |
| Build everyday Debug binary | `cmake --build --preset debug` | `build/debug/sst` compiled; warnings are visible but not fatal |
| Run the game | `./build/debug/sst` or `./build/debug/sst -t` | The selected display starts interactively |
| Run one focused test | `ctest --preset debug -R '^<test-name>$'` | The named registered test passed; the anchored filter cannot silently select neighbors |
| Check shared instructions directly | `tests/instructions.sh .` | The project profile, managed workflow, agent set, and `CLAUDE.md` pointer have the required structure |
| Check installed workflow, when the sibling source checkout exists | `python3 ../agent-skills/scripts/adopt.py --dry-run --keep-existing .` | The managed block and every generated skill/agent mirror match the current installer; otherwise report this check as NOT RUN |
| CI Debug gate | `cmake --preset ci-debug && cmake --build --preset ci-debug && ctest --preset ci-debug` | Debug compiled with warnings fatal and the complete Debug suite passed, apart from explicitly reported platform skips |
| CI Release gate | `cmake --preset ci-release && cmake --build --preset ci-release && ctest --preset ci-release` | Optimized Release compiled with warnings fatal and the complete Release suite passed, apart from explicitly reported platform skips |

The verifier runs both CI gates. CI itself runs that pair on Linux and macOS.
A passing local gate does not cover a test reported as skipped; name skips
rather than folding them into a pass.

For a plain-mode smoke journey:

```sh
printf 'regular\nshort\nnovice\nxyz\nsrscan\nquit\nn\n' | ./build/debug/sst
```

For TUI work, `tests/tui.sh build/debug/sst Debug` is the focused real-PTY
journey. It covers the required 80x24 and 72x24 layouts and uses an isolated
tmux socket.

## Local Setup

- CMake 3.21 or newer, a C17 compiler, and ncurses are required.
  Debian/Ubuntu package the latter as `libncurses-dev`; macOS already provides
  it for the supported build.
- `tmux` is required to exercise curses. Without it, the local `tui` test
  skips; Linux CI treats its absence as a failure.
- `gcc -fanalyzer` enables the `analyze` test. It may skip when unavailable
  and on macOS unless an analyzer-capable compiler was selected deliberately.
- `actionlint` 1.7.12 plus `shellcheck` enables the workflow lint locally.
  Linux CI installs both and treats missing coverage as a failure.
- No services, environment files, credentials, seed step, or package install
  are required.

## Architecture & Conventions

- Global game state uses the `EXTERN`/`INCLUDED` mechanism in `sst.h`.
  Exactly one translation unit per binary defines `INCLUDED` before including
  the header: `sst.c` for the game and the owning `tests/test_*.c` file for a
  test binary that does not link `sst.c`.
- Display state (`tui_active`, `tui_ingame`) is plain `extern` state in
  `tui.h`, defined in `tui.c`. Add display flags there, not in `sst.h`.
- `osx.c` cannot include `sst.h`: the game declaration `pause(int)` collides
  with POSIX `pause(void)`. Functions provided by `osx.c` belong in `osx.h`;
  functions it calls come from `tui.h`. This also keeps
  `-Wmissing-prototypes` effective.
- All game output goes through `prout`, `proutn`, `proutf`, or `prouts`; all
  game input goes through `scan()`, `getch()`, or `readinput()`. Raw stdio
  bypasses the full-screen display.
- `tuifmt.c` panel formatters mirror `srscan()` in `reports.c`, including
  sensor-damage masking and `-f` coordinate transposition (x is the column).
  Keep `tests/test_tuifmt.c` synchronized.
- Comments that justify code with a number, bound, or reachability claim must
  cite the source or measurement that establishes it.

## Gotchas & Troubleshooting

- `CMakeLists.txt` reads `CMAKE_BUILD_TYPE` at configure time, so a bare
  `cmake -S . -B build` is not a supported substitute for a preset.
- Debug adds `-DDEBUG`; every build adds `SCORE`, `CAPTURE`, and `CLOAKING`.
  The `ci-*` presets also enable `-Werror`. Debug and Release therefore need
  separate verification.
- Plain mode must not change accidentally during TUI work. Run the scripted
  plain journey above as well as the focused TUI journey.
- Never drive TUI checks on the default tmux server and never issue a bare
  `tmux kill-server`. Use `tests/tui.sh`; for manual work use `-f /dev/null`
  and a dedicated `-L` socket, and clean up only that socket.
- Game text must not end in carriage return. `sst.doc` uses CRLF, so strip
  both line endings when reading it. The deliberate carriage return in
  `pause()` is the sole exception.
- A `golden` failure means game output or arithmetic changed. Explain the
  fixture diff; never use `tests/golden.sh <sst> --update` merely to make the
  gate green.
- Full-game mutation proofs must use a fresh copy of the current tree without
  `build/`, configure inside that copy, and run its script from that copy.
  Copied CMake caches and scripts invoked from the original tree can test the
  wrong source while appearing green.
- The `tui`, `analyze`, and `workflow` tests may skip locally when their tools
  are absent. A skip is an environment gap, not a passing check.
- The managed triage table decides which workflow stages apply. The
  `slice-and-pr` instruction to commit only after UI review, verification, and
  code review have passed means every gate selected for the current track; it
  does not re-enable a stage the table skips. Record each skipped or
  not-applicable stage and its reason.

## Verification Map

Use this map after a fix to select the affected checks. The complete Debug and
Release CI gates must still both run at least once on the state entering code
review.

| A fix touches | Focused check before the complete gate |
|---|---|
| `tuifmt.c`, `tui.h`, or panel formatting | `ctest --preset debug -R '^tuifmt$'`; also run the plain journey and `tests/tui.sh build/debug/sst Debug` when player-visible |
| Other TUI behavior in `tui.c` | `ctest --preset debug -R '^tui$'` plus the plain journey |
| Rules in `rules.c` or `rules.h` | `ctest --preset debug -R '^rules$'` |
| Other game C or header files | The narrowest registered journey or compiled test that reaches the behavior; then both complete gates |
| `sst.doc` or help behavior | `ctest --preset debug -R '^help$'` |
| Golden fixtures or output arithmetic | `ctest --preset debug -R '^golden$'` and inspect every fixture diff |
| `AGENTS.md`, `CLAUDE.md`, `GEMINI.md`, or installed agent/skill files | `tests/instructions.sh .` and, when `../agent-skills/` exists, `python3 ../agent-skills/scripts/adopt.py --dry-run --keep-existing .` |
| `.github/workflows/**` or `tests/workflow.sh` | `ctest --preset debug -R '^workflow$'`; absence of `actionlint`/`shellcheck` is NOT RUN locally |
| `CMakeLists.txt` or `CMakePresets.json` | Both complete CI gates |
| Any path not listed | Both complete CI gates |

## Git

Self-merges are allowed in this repository. An agent may squash-merge its own
pull request without requesting separate approval once all of these are true:

- the pull request head is the exact locally reviewed and verified commit;
- GitHub reports the pull request clean and mergeable;
- every required check has completed successfully;
- no unresolved review threads or required changes remain; and
- a final readback confirms the base branch, head SHA, and clean local
  worktree.

Use squash merge and delete the merged branch. This standing project policy
overrides the managed workflow's general requirement to ask for merge approval.
Never bypass a pending or failing check, merge a different head than the one
reviewed, or treat approval for one pull request as approval for another.

<!-- agent-skills:begin workflow 185672e4 — managed block, edits here are overwritten -->
## Development Workflow

Follow these stages in order (governed by the global `agent-workflow-skills`). Scale the pipeline to the
size of the change using the triage table — skipping a stage is a decision to
state out loud, never a shortcut taken silently. A stage in parentheses applies
only when its own entry says it does.

| Track | When | Stages |
|---|---|---|
| **Trivial** | Docs, comments, typos, config with no logic change | 1 → 6 → 9 |
| **Single fix** | One bug or small change with a clear, contained cause | 1 → 2 → 5 → 6 → (7) → 8 → 9 |
| **Feature** | New behavior, several files, or an architectural choice | All stages; repeat 5–8 per slice |

**Division of labor.** The main agent runs only focused checks — the single test
it just wrote, a formatter over the files it just touched. Whole suites, builds,
dependency audits, and repository-wide lint go to the **`verifier`** subagent;
reviews go to **`code-reviewer`** and **`ui-reviewer`**. Each follows the skill
of the same job (`verifier`, `code-review`, `ui-review`), reads this file for
what the project's commands and criteria are, and is declared without
file-editing tools — a read-only sandbox where the host supports one. This is
not ceremony: it keeps routine command output out of the implementation context,
and it means each gate is read by something that has not already convinced
itself the change is correct. If a subagent is unavailable, run the stage
inline against the same skill and say that you did.

**Stages end.** Every delegated stage returns a verdict, and a verdict is acted
on once. Fix what came back, then rerun only the stage whose inputs your fix
touched. If the same finding survives two attempts, stop and report it with what
you tried — do not loop. Never rerun a stage against a state it has already
seen; an unchanged tree yields an unchanged verdict.

**Preserve what you did not change.** A worktree may hold work that is not yours.
Never stage, revert, or "clean up" a change you did not make; when something
unrelated is in the way, name it and leave it alone.

**Claim only what you observed.** A gate licenses a statement about exactly
what it measured and nothing more: a green build says the code compiles, not
that the feature works; a passing test says that test passed, not that the bug
is gone. If you did not run it, say you did not. "I believe this fixes it" is a
usable sentence; "fixed and verified" without a command and its output is not.

**Say what you assumed.** When a choice would change what gets delivered and the
request does not settle it, ask before building rather than after. When it is
too small to be worth asking, decide, and write the assumption where a reviewer
will see it. An assumption nobody can see is indistinguishable from a mistake.

**Instructions are part of the change.** When a command, a behavior, or a
constraint changes, the file that documents it changes in the same commit —
`AGENTS.md`, the Verification Map, the README, whichever is now wrong. Stale
instructions are worse than missing ones, because the next agent follows them
confidently.

1. **Inspect & Branch**: Inspect `git status`, the current branch, and every
   applicable instruction file before touching anything. Note unrelated staged,
   unstaged, and untracked work so you can preserve it. Fetch the base branch
   (`git fetch origin main`) and create a dedicated branch:
   `git checkout -b <owner>/<type>/<short-description> origin/main`.
   `<owner>` is your GitHub login (`gh api user --jq .login`); `<type>` is one of
   `feat`, `fix`, `refactor`, `chore`, `test`, `docs`. Never commit to `main`.
2. **Plan & Slice (`plan-and-prototype`)**:
   - **Read before you plan.** Open the code the change will touch, its tests, and
     its call sites. A plan written without reading them is a guess about a
     codebase rather than a plan for this one.
   - Formulate a clear step-by-step plan before writing code. Define the smallest
     end-to-end slice that can be reviewed, tested, and shipped independently; if
     the work is too large for one pull request, order the slices and complete only
     the current one.
   - **A slice is vertical, not horizontal.** It goes through every layer of one
     narrow thing and ends in something you can actually verify: "add the new field
     end to end, with tests" is a slice; "rename the field everywhere" is a sweep.
     One concern per branch — if a change spans unrelated concerns, that is two
     branches.
   - **A new dependency is an architectural decision, not an implementation
     detail.** Say what it replaces, why writing that yourself is the worse option,
     and what its license and maintenance status are. Adding one silently is how a
     project acquires a liability nobody chose.
3. **Prototype Options (if needed)**: When facing architectural choices, unfamiliar
   APIs, or UX alternatives, spike lightweight prototypes and compare trade-offs
   before committing to an approach.
4. **Track Bugs & Follow-ups**: When bugs, edge cases, technical debt, or follow-up
   tasks surface mid-change, file them immediately (`gh issue create`, the project's
   tracker, or `ISSUES.md` when none is configured) instead of expanding the current
   slice.
5. **Test-Driven Development (`tdd-workflow`)**:
   - Write/update a focused test first → confirm it fails for the expected reason →
     minimal implementation → iterate until passing → refactor. A test that passes
     before the code exists is testing the wrong thing.
   - **When the change replaces an existing contract, find the tests pinning the old
     one first.** A new failing test proves the new behavior is missing; it says
     nothing about tests still asserting the behavior being removed. Search for
     assertions on the symbol, attribute, label, or role being changed and update
     them inside the same red/green loop. Skipping this is silently safe — the new
     test goes green, the loop looks complete, and the contradiction only surfaces a
     full gate cycle later.
   - **A test that has never failed is not evidence of anything.** When you add a
     regression detector, break the thing it guards and confirm it catches it, then
     put it back. A detector that cannot be shown to fire is decoration.
   - Run only the test you authored or changed, filtered by file and name. Whole
     suites are stage 6's job.
   - Pure logic (calculations, state machines, business rules) must be unit-tested.
     Non-testable areas (rendering, audio) must be visually/interactively verified.
6. **Verification (`verifier` subagent → `verifier` skill)**:
   - Run the project's full gate: lint, type-check, test suites, build. Focused runs
     from stage 5 do not substitute for it.
   - **Know what green looked like before you started.** If you do not know the
     gate passed on the state you began from, establish that first. Without it a
     failure is ambiguous — you cannot tell what you broke from what you inherited,
     and every later decision rests on that distinction.
   - **Measure the thing you ship, not a proxy for it.** A gate that checks part of
     the output, or a stand-in for it, reads exactly like one that checks all of it
     — and certifies the rest by silence. If a command covers less than it appears
     to, say what it left out.
   - The subagent runs and reports; fixing is yours. Resolve every actionable
     finding before code review. When a fix changes code, rerun the affected focused
     tests, then ask for only the gate commands whose inputs the fix touched — see
     **Verification Map** below if this project defines one. The complete gate must
     run in full at least once on the state that enters code review.
   - Some findings are environmental and no code change resolves them (browsers that
     will not install, no network, a missing credential). Resolving those means
     naming them precisely — what ran, what did not, and why — not retrying them.
7. **UI Review (`ui-reviewer` → `ui-review`)**:
   - Runs after verification, so the tree builds before anyone looks at it.
   - **Check whether this stage applies before delegating.** It applies only when
     the change can alter something a person sees or interacts with. A change
     confined to documentation, comments, configuration, build scripts, CI, tests,
     or code with no rendered output does not qualify — skip the stage, record one
     line saying which of those it was, and move on. A docs-only or test-only diff
     never needs a UI review.
   - When it does apply, audit layout, visual hierarchy, contrast (WCAG AA),
     interaction states, and accessibility according to the project's UI domain.
   - A project whose UI domain is headless or backend skips this stage every time.
   - Never invent findings to justify the stage, and never describe an appearance
     that was not observed running.
8. **Code Review (`code-reviewer` → `code-review`)**:
   - The reviewer reads the complete change: `git diff origin/main...HEAD`,
     plus staged and unstaged edits (`git diff HEAD`) and untracked files (`git
     status --porcelain`). It reports; it does not edit. **You** remove the
     accidental or unrelated edits it names, and preserve anything that is the
     user's.
   - Enforce architectural boundaries, language idioms, defensive error handling,
     and zero committed secrets.
   - Do not repeat this review on an unchanged state. Rerun it only when the
     reviewed content actually changed.
9. **Commit & PR Lifecycle (`slice-and-pr`)**:
   - **Close the loop against the request.** Re-read what was actually asked for,
     and state how this change satisfies it — and what it deliberately does not.
     Every gate above proves the code works; none of them prove it is the thing
     that was wanted. A green pipeline on the wrong feature is the most expensive
     outcome available.
   - Commit using Conventional Commits (`<type>(<scope>): <summary>`). Stage files
     explicitly; never `git add -A` when unrelated work is present.
   - **Match the stopping point to the request.** A request that only asks to
     commit stops after the local commit. A request that asks to use, follow, or
     complete the workflow—including "commit based on the workflow"—includes the
     reversible remote steps: push the branch, open the PR, and watch its checks.
     It does not authorize a merge or any action named under **Stop there and
     report**.
   - Open the PR with `gh pr create` and watch CI with `gh pr checks --watch`.
   - **The description carries the evidence.** Say why the change exists, what it
     changes grouped by concern rather than by file, and how it was tested — the
     command you actually ran and its actual result. "Should work" is not a test
     result. If a test was added, say what it would have caught.
   - **Stop there and report.** Anything you cannot take back needs explicit
     approval from the user in the current conversation: merging (`gh pr merge`),
     force-pushing, rewriting shared history, deleting a branch or tag, dropping
     or migrating data, removing files wholesale, and publishing or deploying.
     Approval for one of them is not approval for the next.
   - **Squash, unless this project says otherwise.** One reviewed slice lands as
     one commit on the base branch. The false starts, the fixups and the "address
     review" commits are how the work got made, not what it is; keeping them turns
     the base branch's history into a diary and makes a revert an archaeology
     exercise. Because the PR description is what survives, it has to carry the
     reasoning — see above. A project that requires merge commits or a rebase says
     so in its own section, and that wins.
   - **A merge takes its branch with it.** Once a merge is approved and done,
     delete that branch — remote and local, in the same step. It is the one
     deletion the merge approval covers, because it is the merge finishing rather
     than a separate act; no other branch is included. A merged branch left
     behind is a decoy: it looks like work in flight, and the next person cannot
     tell it from the real thing without checking.
   - Verify before deleting, and be aware of the squash case: a squash merge
     writes a new commit rather than joining histories, so git sees no ancestry
     and `git branch -d` refuses a branch whose every line is already merged.
     Confirm with `git diff <base> <branch>` — empty output means nothing is
     lost — and then `-D` is correct rather than reckless. If that diff is *not*
     empty, stop: something did not make it in.
<!-- agent-skills:end workflow -->
