# 103 · Auto weights (bone heat)

Status: waiting: O34 · Area: H · Depends on: 101 · Observations: O34 · Touches: rig

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
- Result: tests added; parity N/M (`UNSUPPORTED` / `KNOWN_MISMATCH` with reasons).
- Rules derived, with the case that proves each; parity-pending rules.
- Deviations from the design decisions, and why.
- Files changed.
- Open questions or data that would settle them (the reviewer turns these into follow-up forms).
- The last lines of the gate output.
