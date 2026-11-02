# 096 · Rotation: Euler filter, quaternion interpolation option

Status: waiting: O27, O53 · Area: G · Depends on: 089 · Observations: O27, O53 · Touches: anim

## Goal
Match the reference on rotation crossing ±180° and add the quaternion interpolation option.

## Inputs
- Area design: [../../roadmap/areas/G-animation.md](../../roadmap/areas/G-animation.md)
- O27: [../../parity/animation-2.md](../../parity/animation-2.md) and its golden cases
- O53: [../../parity/animation-3.md](../../parity/animation-3.md) and its golden cases
- docs/parity/animation-3.md (euler-cross, quaternion)

## Design decisions (planner)
- Euler keys interpolate as stored values (O53 euler-cross tells whether it goes the long way). Euler filter command makes adjacent keys continuous.
- Quaternion option: slerp between key orientations, decomposed back to HPB for display.

## Done when
- [ ] O53 euler-cross and quaternion cases pass.
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
