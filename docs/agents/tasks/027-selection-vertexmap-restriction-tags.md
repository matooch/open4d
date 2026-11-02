# 027 · Selection tags, vertex map tags, restriction tags

Status: waiting: O54 · Area: B · Depends on: 023 · Observations: O54 · Touches: core, generators, ui, modifiers

## Goal
Stored selections and vertex weights exist as tags, and deformers can be restricted by them.

## Inputs
- Area design: [../../roadmap/areas/B-modeling.md](../../roadmap/areas/B-modeling.md)
- O54: [../../parity/tags-2.md](../../parity/tags-2.md) and its golden cases
- docs/parity/tags-2.md (O54 point/polygon/edge selection and vertex map cases)

## Design decisions (planner)
- Tag node types `forge.selection_points|edges|polys` (IndexList param) and `forge.vertex_map` (`Value::FloatList` per point; add the Value variant). Commands `select.store` and `select.restore`.
- Restriction tag: names a selection or vertex map. Deformers multiply their effect by the weights (0/1 for selections). This hooks into `apply_deformer`.
- Topology changes (modeling tools) remap or clear the tags. Document the rule: indices are kept when the topology is unchanged, and tags are cleared otherwise.

## Done when
- [ ] EDITABLE.md §6.
- [ ] O54 selection and vertex map cases match.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Out of scope
Vertex maps from fields (062).

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
