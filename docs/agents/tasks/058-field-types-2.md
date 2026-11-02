# 058 · Field types II: Cylinder, Cone, Torus, Capsule, Radial, Random

Status: ready · Area: D · Depends on: 057, 063 · Observations: O13, O23 · Touches: fields, noise

## Goal
The O13/O23 field types match. Random is compared statistically unless the noise matches.

## Inputs
- Area design: [../../roadmap/areas/D-deformers-fields.md](../../roadmap/areas/D-deformers-fields.md)
- O13: [../../parity/fields-2.md](../../parity/fields-2.md) and its golden cases
- O23: [../../parity/fields-3.md](../../parity/fields-3.md) and its golden cases
- docs/parity/fields-2.md, fields-3.md

## Design decisions (planner)
- One file per shape. Radial: angles, transitions, iterations, axis. Random: modes random, sorted and noise, through `forge-noise`.

## Done when
- [ ] FIELDS.md extended; cases match, or are pinned.
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
