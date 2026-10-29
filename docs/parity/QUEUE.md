# Observation queue (run by the observer, unattended)

The planner adds jobs here; the observer works through them **top to bottom** without
waiting for anyone. Playbook: [../agents/OBSERVER.md](../agents/OBSERVER.md). Each job is one
form in this folder. Status values: `todo`, `in-progress`, `done`, `partial` (some cases
missing, reason in the form), `blocked` (nothing could be done, reason in the form),
`gui-only` (needs a human at the GUI: skip it in unattended runs).

| # | Job | Form | Output | Status |
|---|---|---|---|---|
| O2 | Keyframe interpolation | [keyframes.md](keyframes.md) | `tests/golden/keyframes/` samples | done |
| O3 | Make Editable / Current State to Object | [make-editable.md](make-editable.md) | `tests/golden/make-editable/` meshes + tree notes | done |
| O5 | Re-checks from O1/O4 | [rechecks.md](rechecks.md) | form only (+ 2 meshes) | done |
| O6 | Modeling tools (U4) | [modeling-tools.md](modeling-tools.md) | `tests/golden/modeling-tools/` meshes | done |
| O7 | Fields: values in space (P2) | [fields.md](fields.md) | `tests/golden/fields/` samples | done |
| O8 | Cloner + basic effectors (P3) | [cloner.md](cloner.md) | `tests/golden/cloner/` samples | done |
| O9 | Deformers II (P1) | [deformers-2.md](deformers-2.md) | `tests/golden/<deformer>/` meshes | done |
| O11 | Follow-ups to O5–O9 | [followups-1.md](followups-1.md) | new cases in fields/, cloner/, wrap/, squash-stretch/, modeling-tools/, rechecks/ | done |
| O12 | Keyframes II (tangent rule) | [keyframes-2.md](keyframes-2.md) | `tests/golden/keyframes/` samples | done |
| O13 | Fields II (more types, layers) | [fields-2.md](fields-2.md) | `tests/golden/fields/` samples | done |
| O14 | Effectors II | [effectors-2.md](effectors-2.md) | `tests/golden/cloner/` samples | done |
| O15 | Re-checks II (sweep caps, extrude bevel) | [rechecks-2.md](rechecks-2.md) | `tests/golden/rechecks/` meshes | done |
| O16 | Keyframes III (confirm brief 09's fitted rules) | [keyframes-3.md](keyframes-3.md) | `tests/golden/keyframes/` samples | todo |
| O10 | Selection and tool behaviour in the GUI | (form comes with batch 10) | form only | gui-only |

## Rules for the unattended loop

1. **One job at a time, in table order.** Skip `done`, `gui-only` and `blocked` rows.
2. Set the row to `in-progress`, commit nothing yet. Do the job exactly as its form says.
3. **Never invent data.** A value you could not read stays `TODO` with the reason. A case
   that fails goes in the form's notes with the error text; carry on with the next case.
4. After the job: fill the form header (app version, OS, observer, date), run
   `cargo run -q -p forge-parity-harness --bin golden -- check` and
   `cargo run -q -p forge-cleanroom`; both must pass. Set the row to `done` or `partial`.
5. **Commit and push after every job** (only `docs/parity/` and `tests/golden/`):
   `Observation: <job name> (<O#>, N cases)`, then `git push origin HEAD`. Data is safe the
   moment it is pushed; a stopped run loses at most one job.
6. If the bridge stops answering (`ping` fails twice, 30 s apart): commit and push what is
   finished, mark the current row `partial` with the reason, and stop.
7. When no `todo` row is left: print a summary (jobs done, cases per job, everything marked
   `TODO`, `partial` or `blocked`) and stop.
