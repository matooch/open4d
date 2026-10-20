# NN · <Task title>

Status: ready · Branch: `observations-N` · Planner: <session> · Implementer: <model>

## Goal
One or two sentences: what exists when this is done.

## Inputs
- Observation: `docs/parity/<feature>.md`
- Golden cases: `tests/golden/<feature>/` (list them, or "all")
- Earlier specs to follow: `docs/specs/<...>.md` §…

## Design decisions (planner)
Where the code goes (crate, file), node type ids, how it plugs into evaluation, which
parity test to extend, and anything the implementer must not decide alone.

## Hints (planner)
What the planner already saw in the data: likely rules, the case to start with, known
traps. Hints are hypotheses: verify every one.

## Done when
- [ ] Spec section `docs/specs/<SPEC>.md` §… written, citing cases.
- [ ] Parity test covers every case; `UNSUPPORTED` / `KNOWN_MISMATCH` lists explained.
- [ ] `sh scripts/gate.sh` prints `GATE: all passed`.
- [ ] `docs/parity/STATUS.md` rows updated.
- [ ] Committed on the branch with the trailer below; report written.

Commit trailer (last line of every commit message):
`Co-Authored-By: <your model name> <noreply@example.com>`

## Out of scope
What not to touch in this task.

## Report (implementer fills in)
- Result: N/M cases exact; list of unsupported / known mismatch with one-line reasons.
- Rules derived (one line each, with the proving case).
- Parity-pending rules.
- Files changed.
- Open questions for the planner (data that would settle them).
- Gate output (the last two lines of `scripts/gate.sh`).
