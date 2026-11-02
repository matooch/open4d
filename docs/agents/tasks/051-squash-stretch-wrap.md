# 051 · Squash & Stretch and Wrap deformers

Status: ready · Area: D · Depends on: 050 · Observations: O9, O11 · Touches: modifiers

## Goal
Squash & Stretch and Wrap match their O9/O11 cases.

## Inputs
- Area design: [../../roadmap/areas/D-deformers-fields.md](../../roadmap/areas/D-deformers-fields.md)
- O9: [../../parity/deformers-2.md](../../parity/deformers-2.md) and its golden cases
- O11: [../../parity/followups-1.md](../../parity/followups-1.md) and its golden cases
- docs/parity/deformers-2.md, followups-1.md §3
- tests/golden/{squash-stretch,wrap}

## Design decisions (planner)
- Pure functions as in briefs 01–02. Squash: Top/Bottom/Center, factor, expand, curvature types (Square/Cubic/Quartic first; Custom/Spline Unsupported). Wrap: Cylindrical/Spherical, longitude/latitude ranges, movement, Scale Z, tension.

## Done when
- [ ] MODIFIERS.md §7–8.
- [ ] All cases match, or are pinned.
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
