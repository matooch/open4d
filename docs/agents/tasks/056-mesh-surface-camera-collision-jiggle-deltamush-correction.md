# 056 · Mesh (cage), Surface, Camera, Collision, Jiggle, Delta Mush, Correction deformers

Status: done · Area: D · Depends on: 050 · Observations: O45 · Touches: modifiers

## Goal
The remaining deformers match O45.

## Inputs
- Area design: [../../roadmap/areas/D-deformers-fields.md](../../roadmap/areas/D-deformers-fields.md)
- O45: [../../parity/deformers-4.md](../../parity/deformers-4.md) and its golden cases
- docs/parity/deformers-4.md

## Design decisions (planner)
- Mesh cage: mean-value coordinates, bound at initialise time.
- Surface: bind to the nearest triangle; follow it.
- Camera: a grid in screen space.
- Collision: push points out of collider volumes.
- Jiggle: a spring simulation over time; needs a per-frame cache.
- Delta Mush: smoothing plus stored deltas.
- Correction: stores point offsets after the deformers above it.

## Done when
- [x] MODIFIERS.md sections.
- [x] Cases match, or are pinned (Jiggle level C).
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- Result: `crates/modifiers/tests/helper_parity.rs` added (8 cases). 3/8 exact: `correction/defaults` (Bend 90 + stored offset of corrected point 0), `delta-mush/defaults` (identity at the bind pose), `camera-deformer/grid` (vacuous: the recorded camera pose keeps the cube outside the grid; a second test aims the camera). 5 in `KNOWN_MISMATCH` (`mesh-deformer/cage`, `surface/defaults`, `collision/sphere`, `jiggle/frame-10`, `jiggle/frame-20`): the reference scripted export did not run those solvers (O45 Findings), so the golden cube is the undeformed (jiggle: rigidly moved) cube. For these the leaked helper objects (cage, bent plane, sphere) equal ours exactly, the golden cube equals the undeformed cube, and ours satisfies per-deformer invariants.
- Rules derived: Correction (`correction/defaults`): offset of the nearest stored point is added after the deformers above; Delta Mush (`delta-mush/defaults`): `p' = S(p) + F(S(p)) F(S(bind))^T (bind - S(bind))`, identity at the bind pose. Everything else is our own model (mean value coordinates for Mesh, nearest-triangle projection for Surface, a 0.75-half-extent square screen grid for Camera fitted from the recorded grid point spacing 46.875 = 1500/32, push-out for Collision, a deterministic spring replay for Jiggle) and parity-pending.
- Deviations: deformers that need a linked helper object get its mesh through a resolver (`apply_deformer_resolved`; `apply_deformer_mesh` is the same without one) which `forge_generators::evaluate` supplies with a cycle guard. Jiggle is a pure function of (scene, frame) replaying at most `cache` frames through `forge_anim::scene_at`, instead of hidden mutable cache state. There is no camera object type, so the Camera deformer links any object and uses its world matrix. Unobserved options (Accuracy, External, Mapping (U,V), Collision solvers/falloff, On Stopped, Tangent Mode other than Edge, Lock, Scale) are not params.
- Files changed: crates/modifiers/{Cargo.toml, src/{lib,helper,cage,surface,camera,collision,jiggle,delta_mush,correction}.rs, tests/helper_parity.rs}, crates/generators/src/lib.rs, crates/ui/{src/icon_table.rs, icons/{mesh_deform,surface_deform,camera_deform,collision,jiggle,delta_mush,correction}.svg}, docs/specs/MODIFIERS.md, docs/parity/STATUS.md, docs/roadmap/BACKLOG.md, Cargo.lock, this brief.
- Open questions / follow-up cases (MODIFIERS.md §15): re-record the six solver cases interactively (the scripted path skipped them); Camera grid point index (the form says (1, 1) / index 34, the recorded before-position is index 15 of 33) and the camera aimed at the cube; Jiggle per-frame lag on a plane for frames 1..8 with Stiffness 100 %, Drag 0 and 100 %, Gravity; Collision Solver Outside/Inside on a cube collider, Strength 50 %; Surface tilted plane and Proportional; Mesh cage point moved (0, 100, 0) and a uniform scale; Delta Mush Iterations 0/1 with a bent pose initialised before the bend; Correction Mapping UV and Normals.
- The last lines of the gate output: see below.
