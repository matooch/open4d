# 050 · Bend 3-axis gating fix + EvalCtx through evaluation

Status: done · Area: D · Depends on: – · Observations: – · Touches: modifiers, generators, core

## Goal
Two things land together: (1) the three pinned bend mismatches pass (`within_box` gates on x, y and z, plus the suppression rule); (2) evaluation receives time, fps and seed, so later deformers and effectors can use them.

## Inputs
- Area design: [../../roadmap/areas/D-deformers-fields.md](../../roadmap/areas/D-deformers-fields.md)
- MEMORY.md (bend follow-ups: rules proven by bend/mode-box-narrow{,-x,-z})
- docs/parity/bend.md, MODIFIERS.md §0
- crates/modifiers/tests/deformers_parity.rs (KNOWN_MISMATCH)

## Design decisions (planner)
- `within_box`: a point deforms only if |x| ≤ sx/2, |y| ≤ sy/2 and |z| ≤ sz/2. The whole deformation is suppressed when the box spans the parent's x/z extent in neither axis (object-relative threshold; see MEMORY.md for the exact proof cases).
- `EvalCtx { time: f64, fps: f64, seed: u64 }` is threaded through `forge_generators::evaluate` and `apply_deformer`. `forge_anim::scene_at` callers pass the frame time. Default ctx = frame 0, 30 fps.

## Done when
- [x] MODIFIERS.md §0 updated; the 3 cases leave KNOWN_MISMATCH.
- [x] All callers compile with the ctx; no behaviour change elsewhere (the gate proves it).
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: tests added (4 bend unit tests, 2 `EvalCtx` unit tests); parity 55/55 in `deformers_parity` (bend 15/15); `UNSUPPORTED` and `KNOWN_MISMATCH` both empty.
- Rules derived (first hypothesis each, no budget used):
  - `within_box` gates per point on all three axes, inclusive, in the deformer's local space before `angle`: `bend/mode-box-narrow-x` (x = ±100 columns unmoved), `bend/mode-box-narrow-z` (|z| = 100 unmoved).
  - Suppression: with `ex`, `ez` the object's x/z extents in the deformer's space, `within_box` does nothing when `size_x < ex` and `size_z < ez` (`bend/mode-box-narrow`). "Spans" is `size >= extent`, from the interactive sweep in `docs/parity/bend.md`. Only in `within_box`; `limited` ignores x/z size (`deformer-common/on-plane`).
  - Parity-pending: Bend only (not applied to Twist, Taper, Bulge, Shear, Spherify); `angle` with `within_box`; whether the extent is taken before or after the deformer rotation for rotated deformers; floating-point boundary (size == extent exactly).
- API: `forge_core::EvalCtx { time, fps, seed }` (+ `Default` = frame 0, 30 fps, seed 0; `at_frame`, `at_time`). `forge_generators::evaluate(scene, id, &ctx)` and `forge_modifiers::apply_deformer(.., &ctx)` take it; no deformer reads it yet. `app::eval_mesh` and the viewport (`SceneView.ctx`, new `scene_meshes_at`) pass the frame time.
- Deviations: `scene_meshes`, `make_editable`, `current_state_to_object`, `pick_mesh`, `mesh_size` and OBJ export keep their signatures and use `EvalCtx::default()` (they have no time available; no deformer reads it yet). The internal presence probes of the generators also use the default ctx.
- Files changed: crates/core (eval.rs, lib.rs), crates/modifiers (bend.rs, lib.rs, tests/deformers_parity.rs), crates/generators (lib.rs, boole, modeling, subdivision, polygon, tests), crates/app/src/lib.rs, crates/ui (viewport_panel, app, components, coordinates, tests), crates/io/src/scene.rs and tests, crates/anim/tests/rules.rs, docs/specs/MODIFIERS.md (§0, §1, new §8, §9), docs/parity/STATUS.md, BACKLOG.md.
- Open questions: a golden case for Twist/Taper/Bulge/Shear with a narrow `within_box` would settle whether the same gate and suppression apply; a rotated deformer with `within_box` would settle the extent frame.
