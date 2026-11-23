# 077 · Instance-over-time generator

Status: done (parity-pending: Trail mode) · Area: E · Depends on: 065, 088 · Observations: O50 · Touches: clone-engine, anim

## Goal
A generator that leaves copies of its child along its animated path (step, start/end).

## Inputs
- Area design: [../../roadmap/areas/E-clone-toolset.md](../../roadmap/areas/E-clone-toolset.md)
- O50: [../../parity/clone-toolset-3.md](../../parity/clone-toolset-3.md) and its golden cases
- docs/parity/clone-toolset-3.md

## Design decisions (planner)
- Sample the child world matrix at past frames (scene_at) every N frames; produce CloneData. Cache per frame.

## Done when
- [x] O50 `instance-time/defaults` matches its clone sample and exported scene mesh.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- **Parity:** 1/1 exact. The test reads the `instance-time/defaults` manifest, clone
  samples, and OBJ. The sole clone has an identity local matrix, and the linked
  cube still exports at x=300. Mesh comparison checks ordered topology and positions
  at the fixture tolerance. Neutral weight 1 represents the missing sample channel,
  not a measured weight.
- **Own design, parity-pending:** Trail samples its child's animated world transform
  at spaced past frames and expands the current prototype. It does not replay older
  geometry or dynamics. The scheduler reuses same-frame geometry and recomputes
  on frame change even without animation tracks.
- **Coverage:** O50 reconstruction, animated-ancestor and sub-frame Trail samples,
  cyclic link termination, scheduler time invalidation and reuse.
- **Files:** clone-toolset spec I1; generators `instance_time` and its integration
  tests; app scheduler and UI icon registration.
- **Follow-up:** capture multiple historical placements and non-default spacing to
  compare Trail conventions.
- **Gate:** `GATE: 2454 tests passed, 0 failed`; `GATE: all passed`.
