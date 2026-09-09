# SDD ledger — plan: docs/superpowers/plans/2026-09-09-command-modernization.md

## Pre-flight Plan Scan
| Task Pair / Task | Produces vs Consumes | Findings & Rulings |
|---|---|---|
| Task 1 (`cmdtab` core) & Task 2 (`cmdtab_suggest`) | Task 1 declares `cmdtab_lookup`; Task 2 adds `cmdtab_suggest` | No conflict. Compatible. |
| Task 2 & Task 3 (`topic` lookup) | Task 3 adds `cmdtab_topic_lookup` | No conflict. Compatible. |
| Tasks 1-3 & Task 4 (`sst.c` dispatch) | Task 4 consumes all `cmdtab` functions | No conflict. Task 4 replaces `commands[]` with `cmdtab`. |
| Task 4 & Task 5 (`helpme()` overhaul) | Task 5 consumes `cmdtab_topic_lookup` and `cmdtab_suggest` | No conflict. Resolves #120. |
| Task 6 (interactive prompts) & Tasks 1-5 | Task 6 touches `moving.c`, `battle.c`, `setup.c` | Independent of `cmdtab.c` / `sst.c`. Can run in parallel. |
| Task 7 & Tasks 1-6 | Task 7 re-records golden fixtures | Depends on Tasks 4, 5, 6 prompt changes. Must run after Tasks 1-6. |
| Task 8 & All | Complete local CI verification | Final gate. |

Ruling: Tasks 1-3 form the core `cmdtab` library. Task 6 modernizes interactive prompts in `moving.c` and `battle.c`. Since their file footprints are completely disjoint, they can proceed in parallel.
