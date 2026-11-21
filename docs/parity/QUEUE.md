# Observation queue (run by the observer, unattended)

The planner and reviewers add jobs here; the observer works through them **top to bottom**
without waiting for anyone. Each run starts with `git pull origin main` and pushes to the
`observations` branch ([../agents/OBSERVER.md](../agents/OBSERVER.md) §6). The order follows the implementation waves in
[../roadmap/README.md](../roadmap/README.md). Playbook: [../agents/OBSERVER.md](../agents/OBSERVER.md). Each job is one
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
| O14 | Effectors II | [effectors-2.md](effectors-2.md) | `tests/golden/cloner/` samples | partial |
| O15 | Re-checks II (sweep caps, extrude bevel) | [rechecks-2.md](rechecks-2.md) | `tests/golden/rechecks/` meshes | done |
| O16 | Keyframes III (confirm brief 09's fitted rules) | [keyframes-3.md](keyframes-3.md) | `tests/golden/keyframes/` samples | done |
| O25 | Modeling tools II (U4) | [modeling-tools-2.md](modeling-tools-2.md) | `tests/golden/modeling-tools/` meshes | partial |
| O26 | Selection commands (U4) | [selection-ops.md](selection-ops.md) | `tests/golden/selection-ops/` meshes + selection.json | partial (8 of 11 baseline cases; loop-edge, loop-poly and ring-edge are interactive tools - see the form) |
| O24 | Deformer falloff through fields | [deformer-falloff.md](deformer-falloff.md) | `tests/golden/deformer-falloff/` meshes | done |
| O23 | Fields III (blend modes, more types) | [fields-3.md](fields-3.md) | `tests/golden/fields/` samples | done |
| O22 | Effectors III | [effectors-3.md](effectors-3.md) | `tests/golden/cloner/` samples + meshes | done |
| O21 | Cloner III | [cloner-3.md](cloner-3.md) | `tests/golden/cloner/` samples + meshes | done (the three `child` cells were still TODO; filled 2026-11-21 from mesh.obj — per-clone child read for all three order cases) |
| O17 | Spline primitives II | [splines-2.md](splines-2.md) | `tests/golden/splines-2/` meshes | partial |
| O18 | Primitives III | [primitives-3.md](primitives-3.md) | `tests/golden/<object>/` meshes | done |
| O58 | Modeling tools III (020/021 follow-ups) | [modeling-tools-3.md](modeling-tools-3.md) | `tests/golden/modeling-tools/` meshes | partial |
| O20 | Generators III | [generators-3.md](generators-3.md) | `tests/golden/<generator>/` meshes | partial |
| O19 | Deformers III | [deformers-3.md](deformers-3.md) | `tests/golden/<deformer>/` meshes | partial |
| O45 | Deformers IV | [deformers-4.md](deformers-4.md) | `tests/golden/<deformer>/` meshes | partial |
| O46 | Spline tools and editable splines | [spline-tools.md](spline-tools.md) | `tests/golden/spline-tools/` meshes | partial |
| O56 | Caps and bevel tab | [caps-bevel.md](caps-bevel.md) | `tests/golden/caps-bevel/` meshes | done |
| O48 | Text spline | [text.md](text.md) | `tests/golden/text/` meshes | partial |
| O49 | Fields IV: modifier layers, vertex maps | [fields-4.md](fields-4.md) | `tests/golden/fields/` samples + meshes | done |
| O42 | Shaders on a UV grid | [shaders.md](shaders.md) | `tests/golden/shaders/` samples | partial (17 of 19 cases; noise-perlin needs a Perlin type this build does not offer, layer-multiply needs a GUI-built layer stack — see the form) |
| O27 | Animation II (tracks, tags) | [animation-2.md](animation-2.md) | `tests/golden/animation-2/` samples | done |
| O38 | F-curves: tangents and behaviours | [fcurves.md](fcurves.md) | `tests/golden/keyframes/` samples | done (16 observable cases: 12 baseline plus the 4 follow-ups — linear-to-spline-2, broken-2, before-offset, before-oscillate observed 2026-11-21; zero angle, zero length and ease-in stay not observed with written reasons) |
| O53 | Animation III: more track types | [animation-3.md](animation-3.md) | `tests/golden/animation-3/` samples + meshes | todo (1 follow-up case; see the form's Follow-up section) — before: partial |
| O31 | Clone-toolset generators II | [clone-generators-2.md](clone-generators-2.md) | meshes + samples | todo (1 follow-up case; see the form's Follow-up section) — before: partial (7 of 10 cases; Voronoi Fracture and Tracer could not be baked) |
| O50 | Clone toolset III | [clone-toolset-3.md](clone-toolset-3.md) | cloner/, clone-spline/, tracer/, effector-f/, random-f/ | todo (3 follow-up cases; see the form's Follow-up section) — before: partial (44 of 49 rows observed; the 5 that need a human or a GUI step stay not observed: selection-tag, weight-tag, linear-offset-15, on-spline, on-spline-step — see the form) |
| O32 | Volume builder and mesher | [volumes.md](volumes.md) | `tests/golden/volumes/` meshes | observed (10 of 12 cases; subtract/intersect not scriptable) |
| O33 | Volumes II: filters, sources, VDB | [volumes-2.md](volumes-2.md) | `tests/golden/volumes/` meshes + .vdb | observed (11 of 13 cases; points-source and vdb-export not scriptable) |
| O34 | Rigging I: joints and skinning | [rigging.md](rigging.md) | `tests/golden/rigging/` meshes | done (baseline plus the 5 follow-up cases after brief 101, observed 2026-11-21: weights-readback, j2-bend-90-points, j2-bend-90-matrix, j2-bend-90-single, j2-bend-90-blended) |
| O35 | Rigging II: IK | [ik.md](ik.md) | `tests/golden/ik/` samples + meshes | done (9 baseline cases plus the 2 follow-ups after brief 104, observed 2026-11-21: rest-orientation, four-joints-2) |
| O36 | Constraints | [constraints.md](constraints.md) | `tests/golden/constraints/` samples + meshes | done (7 of 9 cases; see form) |
| O37 | Pose Morph | [pose-morph.md](pose-morph.md) | `tests/golden/pose-morph/` meshes | blocked (strength track not scriptable; see form) |
| O54 | Tags II | [tags-2.md](tags-2.md) | `tests/golden/tags-2/` | done |
| O28 | UV coordinates and projection | [uv-projection.md](uv-projection.md) | `tests/golden/uv/` meshes | todo (1 follow-up case; see the form's Follow-up section) — before: done |
| O29 | Scene, camera, light, material, render defaults | [scene-defaults.md](scene-defaults.md) | form only | partial |
| O30 | Render references | [render-reference.md](render-reference.md) | `tests/golden/render/` PNG + meshes | partial (12 of 14 cases: 8 of 10 baseline plus all 4 follow-ups after briefs 136/137 — light-units, light-units-200, area-soft-2, checker-plane. Spot and infinite still render a fully black frame from a script; the two black-render folders are no longer in the tree — see the form) |
| O43 | Material renders | [material-renders.md](material-renders.md) | `tests/golden/render/` PNG + meshes | done |
| O57 | Camera projection | [camera.md](camera.md) | `tests/golden/camera/` PNG + meshes | partial (8 of 14 cases: the brief-138 follow-ups default-2, aspect-square-2, film-offset-y and ortho-zoom-2 are observed, focal-100-2 renders one white colour at 100 mm; sensor and dof still not scriptable, the original default / focal-100 / aspect-square are superseded by the -2 redos — see the form) |
| O44 | Exchange references (FBX, glTF, Alembic, STL, PLY) | [exchange.md](exchange.md) | `tests/golden/exchange/` | done (morph not observed) |
| O39 | Dynamics: rigid bodies | [dynamics.md](dynamics.md) | `tests/golden/dynamics/` samples + meshes | todo (4 follow-up cases; see the form's Follow-up section) — before: partial (8 of 9 cases; gravity-half not observed because the scene gravity is not reachable from a script and the body tag has no gravity parameter - see the form) |
| O40 | Particles and forces | [particles.md](particles.md) | `tests/golden/particles/` samples + meshes | blocked (0 of 10 cases: the emitter exposes no particle list to a script, so no position, velocity or age can be read - see the form) |
| O41 | Soft bodies and connectors | [soft-bodies.md](soft-bodies.md) | `tests/golden/soft-bodies/` samples + meshes | todo (4 follow-up cases; see the form's Follow-up section) — before: done (6 of 6 cases; the ball-socket track is identical to the hinge one - see the form) |
| O10 | Selection and tool behaviour in the GUI | (form comes with batch 10) | form only | gui-only |
| G1 | GUI session: layout and managers | [GUI-SESSIONS.md](GUI-SESSIONS.md) §G1 | answers in GUI-SESSIONS.md | gui-only |
| G2 | GUI session: shortcuts and navigation | [GUI-SESSIONS.md](GUI-SESSIONS.md) §G2 | answers in GUI-SESSIONS.md | gui-only |
| G3 | GUI session: interactive modeling and selection | [GUI-SESSIONS.md](GUI-SESSIONS.md) §G3 | answers in GUI-SESSIONS.md | gui-only |
| G4 | GUI session: attribute widgets and field lists | [GUI-SESSIONS.md](GUI-SESSIONS.md) §G4 | answers in GUI-SESSIONS.md | gui-only |
| G5 | GUI session: timeline, dope sheet, F-curves | [GUI-SESSIONS.md](GUI-SESSIONS.md) §G5 | answers in GUI-SESSIONS.md | gui-only |
| G6 | GUI session: rigging tools | [GUI-SESSIONS.md](GUI-SESSIONS.md) §G6 | answers in GUI-SESSIONS.md | gui-only |
| G7 | GUI session: materials and UV | [GUI-SESSIONS.md](GUI-SESSIONS.md) §G7 | answers in GUI-SESSIONS.md | gui-only |
| G8 | GUI session: viewport and picture viewer | [GUI-SESSIONS.md](GUI-SESSIONS.md) §G8 | answers in GUI-SESSIONS.md | gui-only |

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
10. **Crash safety.** Before each case, write its name to
    `/Users/bigman/forge-observer/CURRENT_CASE`. After the case's files are written, delete
    that file. Commit and push after every **5 cases** within a job, not only at its end,
    so a crash loses at most a few cases.
    At the start of a run, if `CURRENT_CASE` exists, the app crashed during that case in the
    previous run: record "crashed the app" for that case in its form's Notes, skip it for
    good, delete `CURRENT_CASE`, and continue the job from the next case. Cases already in
    `tests/golden/` from the previous run are kept; don't redo them.
11. **One command per probe; no dialogs.** Run a single modeling or selection command per
    `execute_python_script` call, then `ping` before the next one. Never run a command
    whose menu name ends in `…` (it opens a dialog and blocks or crashes the app). Never
    start an interactive tool that waits for mouse input (Loop/Path Cut, Knife, Loop and
    Ring Selection, sculpt brushes). Write those cases off in Notes as "needs the GUI"
    (O10 territory) and move on.
12. When no `todo` row is left: print a summary (jobs done, cases per job, everything marked
   `TODO`, `partial` or `blocked`) and stop.
