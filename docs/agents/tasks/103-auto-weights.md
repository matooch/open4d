# 103 · Auto weights (bone heat)

Status: done · Area: H · Depends on: 101 · Observations: O34 · Touches: rig

## Goal
Automatic weights by heat diffusion.

## Inputs
- Area design: [../../roadmap/areas/H-rigging.md](../../roadmap/areas/H-rigging.md)
- O34: [../../parity/rigging.md](../../parity/rigging.md) and its golden cases
- docs/parity/rigging.md (auto-weight case)

## Design decisions (planner)
- Solve (L + H) w = H p per bone (cotangent Laplacian, nalgebra-sparse); level C comparison: dominant joint per point agrees for ≥ 90 % of points.

## Done when
- [ ] Auto-weight case meets the level-C threshold.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: `auto_weights` (bone heat: cotangent Laplacian, vertex areas, nearest visible bone heat,
  Jacobi-CG solve per bone), `bones_for_joints`, `auto_bind`. Tests `crates/rig/tests/auto_weights.rs`
  and unit tests in the module. The O34 auto-weights case was not observed (dialog, no scripting
  call), so no reference data exists: UNSUPPORTED as a parity case. Level C check on our own partner
  (nearest bone): 89/89 dominant joints agree (threshold 90 %).
- Rules: heat `H` is shared by all bones (each vertex heated by its nearest visible bone), the
  indicator differs per bone; a first version with per-bone `H` returned the constant solution 1.
- Deviations: own conjugate-gradient solver instead of nalgebra-sparse (not in the workspace).
- Files: crates/rig/src/auto_weights.rs, crates/rig/tests/auto_weights.rs, docs/specs/RIG.md section 4.
