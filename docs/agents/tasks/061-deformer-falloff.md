# 061 · Deformer falloff through field lists (all deformers)

Status: ready · Area: D · Depends on: 057 · Observations: O24 · Touches: modifiers, fields

## Goal
Every deformer takes a field list. An empty list means full effect. The O24 cases match.

## Inputs
- Area design: [../../roadmap/areas/D-deformers-fields.md](../../roadmap/areas/D-deformers-fields.md)
- O24: [../../parity/deformer-falloff.md](../../parity/deformer-falloff.md) and its golden cases
- docs/parity/deformer-falloff.md (the field is in the deformer's list; the empty list gives full effect)

## Design decisions (planner)
- The deformer result is blended per point: p' = p + w·(deform(p) − p), with w = the field value at the point's world position.

## Done when
- [ ] MODIFIERS.md §falloff; all 9 O24 cases match.
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
