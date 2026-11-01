# 059 · Field lists: layers, blend modes, groups, Formula and Shader fields

Status: ready · Area: D · Depends on: 057, 052 · Observations: O13, O23 · Touches: fields

## Goal
Field lists with ordered layers, blend modes and strengths, plus Group, Formula and Shader fields, match O13/O23 (overlapping blends).

## Inputs
- Area design: [../../roadmap/areas/D-deformers-fields.md](../../roadmap/areas/D-deformers-fields.md)
- O13: [../../parity/fields-2.md](../../parity/fields-2.md) and its golden cases
- O23: [../../parity/fields-3.md](../../parity/fields-3.md) and its golden cases
- docs/parity/fields-2.md (a layer inserted later goes on top), fields-3.md (blend-* cases on overlapping spheres)

## Design decisions (planner)
- `FieldList { layers: Vec<Layer { field, blend, strength, enabled }> }`, evaluated bottom to top. Implement the 9 blend modes per the observed data. The Shader field uses `forge-shaders` (125); pin it until then.

## Done when
- [ ] FIELDS.md §lists.
- [ ] O23 blend cases match.
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
