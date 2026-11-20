# 113 · Rig mirroring (joints and weights, left/right naming)

Status: done · Area: H · Depends on: 101 · Observations: – · Touches: rig

## Goal
Mirror a half rig with its weights.

## Inputs
- Area design: [../../roadmap/areas/H-rigging.md](../../roadmap/areas/H-rigging.md)

## Design decisions (planner)
- Name rules L_/R_, .L/.R, _left/_right; nearest-point weight transfer across the plane.

## Done when
- [ ] Unit test: mirrored rig deforms symmetrically.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: `mirror_name`, `joint_mirror_map`, `mirror_weights`, `mirror_tag_weights`, `add_joints_to_tag`
  (joint mirroring `mirror_chain` came with brief 100). Tests `crates/rig/tests/mirroring.rs` (4) and
  name tests in `rig_units.rs`: a mirrored rig deforms symmetrically to 1e-9. No parity data (own design).
- Rules: name rules and nearest-point weight transfer as in RIG.md section 5.
- Deviations: nearest-point search is brute force (O(n^2)); fine for rig-sized meshes, a grid would be
  the follow-up for 100k-point meshes.
- Files: crates/rig/src/mirror.rs, crates/rig/tests/mirroring.rs, docs/specs/RIG.md section 5.
