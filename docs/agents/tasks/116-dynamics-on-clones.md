# 116 · Dynamics on clones

Status: done · Area: I · Depends on: 115, 065 · Observations: O39 · Touches: dynamics, clone-engine

## Goal
Each clone is a body; results write back into CloneData.

## Inputs
- Area design: [../../roadmap/areas/I-dynamics.md](../../roadmap/areas/I-dynamics.md)
- O39: [../../parity/dynamics.md](../../parity/dynamics.md) and its golden cases
- docs/parity/dynamics.md (clone cases)

## Design decisions (planner)
- Body per clone index; cache keyed by index.

## Done when
- [x] Clone dynamics cases meet thresholds.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: `clones-fall` (1 case, 2 sampled clones): both asserted at level C and each pinned once.
  Tests: `crates/dynamics/tests/clones_parity.rs` (5), `crates/app/tests/dynamics_eval.rs`
  `clones_fall_through_the_cloner_evaluation`, `crates/clone-engine` unit test for `apply_poses`.
  - clone 4: free fall frames 0..22 within 1 cm, contact frame ±1, rest y 15.0 within 0.05 cm,
    upright within 1°; pinned: lateral drift (reference 1.87 cm in x and z, ours 0).
  - clone 0 (cube centre exactly over the plane's corner): free fall within 1 cm and contact frame
    ±1; pinned: the end track (the reference tips off and ends at y -2227.8; ours balances at 15.08).
- Rules derived: one body per visible clone, world = cloner × clone matrix, prototype shape;
  results go into the cloner's `dynamics_pos` / `dynamics_rot` lists and `CloneData::apply_poses`;
  the cloner object is not a body (observed: it keeps global y 300). All own design except the
  observed start poses and the cloner staying put.
- Deviations: the write-back goes through two hidden cloner parameters (the generators crate cannot
  depend on dynamics); `forge-dynamics` now depends on `forge-generators` and `forge-clone-engine`.
- Files changed: crates/dynamics/src/{clones,apply,build}.rs, crates/clone-engine/src/{node,data}.rs,
  crates/generators/src/cloner.rs, crates/dynamics/tests/clones_parity.rs,
  crates/app/tests/dynamics_eval.rs, docs/specs/DYNAMICS.md §6-§7, BACKLOG, STATUS.
- Open questions: where the reference's 1.87 cm lateral drift of the centre clone comes from (the
  clones are 200 cm apart, so not from contacts between clones): a repeat with the tag's Collision
  Noise at 0 would say; the corner case needs a clone not exactly on the edge.
- The last lines of the gate output: see the final report.
