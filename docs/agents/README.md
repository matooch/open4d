# Agent roles and the hand-off workflow

Forge3D is built by three roles. Each works only from files in this repository, so any
model can pick up a role by reading the right documents.

| Role | Who | Does | Never does |
|---|---|---|---|
| **Planner** | the main (cloud) session | picks the next features from [PLAN.md](../PLAN.md), writes observation forms (`docs/parity/<feature>.md`), briefs the observer, reads the golden data, writes **task briefs** (`docs/agents/tasks/`), reviews hand-back reports and diffs, opens PRs | writes production code for features that have a brief (it reviews instead) |
| **Observer** | a session on the owner's Mac that drives the reference app (R25) through the MCP bridge, GUI as fallback ([OBSERVER.md](OBSERVER.md)) | fills observation forms, exports golden cases and `samples.json` to `tests/golden/`, commits locally on the batch branch | touches code, specs or tests; opens the app's install folders or resource files ([CLEANROOM.md](../CLEANROOM.md)) |
| **Implementer** | a local session on the owner's Mac (Claude Sonnet 5.5), **without** the MCP bridge | follows [IMPLEMENTER.md](IMPLEMENTER.md) for one task brief at a time: derive → spec → tests → code → gate → report | edits golden data or observation files; drives the reference app; loosens tolerances; pushes or merges |

## The loop

1. **Planner:** choose a batch from [PLAN.md](../PLAN.md); write forms and briefs; push
   them on the planner's session branch and open a PR. Once it is merged, the batch
   works on `batch-NN` (NN = the batch number in PLAN.md) cut from `main`.
2. **Observer:** fill the forms, export the cases, commit locally, report back.
3. **Planner:** read the data; settle design decisions (crate, node types, how it plugs
   into evaluation); write one brief per task with [TASK_TEMPLATE.md](TASK_TEMPLATE.md)
   into `docs/agents/tasks/<NN>-<slug>.md`; commit. Order tasks so each builds on the last.
4. **Implementer:** take the lowest-numbered brief whose status is `ready`; work it per
   IMPLEMENTER.md on the same branch; commit; write the hand-back report into the brief
   (`## Report`); set its status to `done` or `blocked`.
5. **Planner:** review the report and the diff (spec cites cases, tests pin the
   unsupported/mismatch sets, no tolerance changes, gate passes); fix or send back
   (`status: rework` with notes); when a batch is done, push and open the PR.

## Branches

- Planner sessions push only to their own session branch, then PR to `main`.
- Observer and implementer share one local branch per batch, `batch-NN`, cut from `main`
  after the planner's PR merged (`git fetch origin && git switch -c batch-NN origin/main`).
  The owner pushes it and opens the PR (or asks a planner session to review it).
- Older batches used `observations-N`; that name is retired.

## Status values in a brief

`ready` (implementer may start) · `in-progress` · `done` (report written, gate passed) ·
`blocked` (report says what is missing, e.g. an observation) · `rework` (planner notes
added; implementer resumes).

## Files

- [IMPLEMENTER.md](IMPLEMENTER.md): the implementer's step-by-step playbook, codebase map
  and pitfalls. Read it fully before every task.
- [OBSERVER.md](OBSERVER.md): the observer's MCP playbook (allowed tools, probe scripts
  outside the repo, the smoke test).
- [TASK_TEMPLATE.md](TASK_TEMPLATE.md): the brief format (planner) with the report
  format (implementer) at its end.
- `tasks/`: the briefs, one file per task.
- [../../scripts/obj_dump.sh](../../scripts/obj_dump.sh): print an OBJ with 0-based
  indices. [../../scripts/gate.sh](../../scripts/gate.sh): the full pre-commit gate.
