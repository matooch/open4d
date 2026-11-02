# 062 · Vertex maps from fields; restriction

Status: waiting: O49 · Area: D · Depends on: 027, 059 · Observations: O49 · Touches: fields, modifiers

## Goal
A vertex map tag can be filled from a field list, and restriction uses it (the O49 vmap cases).

## Inputs
- Area design: [../../roadmap/areas/D-deformers-fields.md](../../roadmap/areas/D-deformers-fields.md)
- O49: [../../parity/fields-4.md](../../parity/fields-4.md) and its golden cases
- docs/parity/fields-4.md (vmap-from-field, restrict-bend)

## Design decisions (planner)
- The vertex map tag gets a fields list. Weights are evaluated per frame (or frozen). Restriction multiplies the deformer weights.

## Done when
- [ ] FIELDS.md §vertex maps; the cases match.
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
