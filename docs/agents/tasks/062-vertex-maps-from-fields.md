# 062 · Vertex maps from fields; restriction

Status: done · Area: D · Depends on: 027, 059 · Observations: O49 · Touches: fields, modifiers

## Goal
A vertex map tag can be filled from a field list, and restriction uses it (the O49 vmap cases).

## Inputs
- Area design: [../../roadmap/areas/D-deformers-fields.md](../../roadmap/areas/D-deformers-fields.md)
- O49: [../../parity/fields-4.md](../../parity/fields-4.md) and its golden cases
- docs/parity/fields-4.md (vmap-from-field, restrict-bend)

## Design decisions (planner)
- The vertex map tag gets a fields list. Weights are evaluated per frame (or frozen). Restriction multiplies the deformer weights.

## Done when
- [x] FIELDS.md §vertex maps; the cases match.
- [x] The gate passes and BACKLOG.md is updated.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- 2/2 O49 cases match: `vmap-from-field` weights and base mesh, and `restrict-bend`
  deformed mesh. The app test reads the manifests and the recorded outputs.
- Maps sample field rows in world space at input points. Restriction applies the
  live or stored weight to the displacement; the test moves a live field and
  verifies that a frozen map is unaffected. O49 `restrict-bend` uses both Bend
  values as stored radians; the displacement slope and full mesh distinguish that
  from displayed degrees. FIELDS.md documents the rule.
- Core owns the descriptor and name lookup. The fields crate resolves live weights;
  both modifier entry points use them. No golden data or tolerances changed.
- Gate: `2452 tests passed, 0 failed`; `GATE: all passed`, including licenses.
  A prior run saw the unrelated graph test `a_shared_node_runs_once_per_evaluation`
  report 2 instead of 1; it passed on rerun without changes to that test.
