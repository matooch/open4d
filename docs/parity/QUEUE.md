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
| O16 | Keyframes III (confirm brief 09's fitted rules) | [keyframes-3.md](keyframes-3.md) | `tests/golden/keyframes/` samples | done |
| O25 | Modeling tools II (U4) | [modeling-tools-2.md](modeling-tools-2.md) | `tests/golden/modeling-tools/` meshes | todo |
| O26 | Selection commands (U4) | [selection-ops.md](selection-ops.md) | `tests/golden/selection-ops/` meshes + selection.json | todo |
| O24 | Deformer falloff through fields | [deformer-falloff.md](deformer-falloff.md) | `tests/golden/deformer-falloff/` meshes | todo |
| O23 | Fields III (blend modes, more types) | [fields-3.md](fields-3.md) | `tests/golden/fields/` samples | todo |
| O22 | Effectors III | [effectors-3.md](effectors-3.md) | `tests/golden/cloner/` samples + meshes | todo |
| O21 | Cloner III | [cloner-3.md](cloner-3.md) | `tests/golden/cloner/` samples + meshes | todo |
| O17 | Spline primitives II | [splines-2.md](splines-2.md) | `tests/golden/splines-2/` meshes | todo |
| O18 | Primitives III | [primitives-3.md](primitives-3.md) | `tests/golden/<object>/` meshes | todo |
| O19 | Deformers III | [deformers-3.md](deformers-3.md) | `tests/golden/<deformer>/` meshes | todo |
| O20 | Generators III | [generators-3.md](generators-3.md) | `tests/golden/<generator>/` meshes | todo |
| O27 | Animation II (tracks, tags) | [animation-2.md](animation-2.md) | `tests/golden/animation-2/` samples | todo |
| O28 | UV coordinates and projection | [uv-projection.md](uv-projection.md) | `tests/golden/uv/` meshes | todo |
| O29 | Scene, camera, light, material, render defaults | [scene-defaults.md](scene-defaults.md) | form only | todo |
| O30 | Render references | [render-reference.md](render-reference.md) | `tests/golden/render/` PNG + meshes | todo |
| O31 | Clone-toolset generators II | [clone-generators-2.md](clone-generators-2.md) | meshes + samples | todo |
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
6. **Bridge health first.** Before starting a job, run `ping` **and** an
   `execute_python_script` probe of `1 + 1`. If either fails twice, 30 s apart, the app is
   stuck (often a modal dialog). Then do NOT mark the job partial: leave its row `todo`,
   change nothing in the repo, create the file `/Users/bigman/forge-observer/STOP`
   containing the error text, and stop. The overnight loop checks for that file and ends
   the run, so one stuck app doesn't burn through the whole queue. If the bridge fails in
   the middle of a job, commit and push what is finished, mark the row `partial` with the
   reason, create the STOP file, and stop.
7. **Every case writes output**: a `mesh.obj` or a `samples*.json` (or both). A case with
   neither breaks a test that pins the list of output-less cases. Information without
   output (defaults, tree notes) goes into the form, not into `tests/golden/`. O29 is a
   form-only job and writes no case folders.
8. **A case that can't be done** (the tool can't be driven by script, an option doesn't
   exist): write the reason in the form's Notes, skip it, and carry on. Never stop the
   whole run for one case. A job where most cases fail is `partial`.
9. Folder and key names must stay clear of the reference app's and its maker's names
   (the cleanroom check fails on them). Use the feature names the forms give.
10. When no `todo` row is left: print a summary (jobs done, cases per job, everything marked
   `TODO`, `partial` or `blocked`) and stop.
