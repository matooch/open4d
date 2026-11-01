# 021 · Bevel tool (all modes)

Status: ready · Area: B · Depends on: 020 · Observations: O6, O25, O15 · Touches: modeling, ui

## Goal
Bevel in polygon, edge and point modes reproduces the reference cases: chamfer and solid, the fixed/radial/proportional offset modes, subdivision, depth, mitering, endings, the hidden Extrusion field (O11), and N-gon options.

## Inputs
- Area design: [../../roadmap/areas/B-modeling.md](../../roadmap/areas/B-modeling.md)
- O6: [../../parity/modeling-tools.md](../../parity/modeling-tools.md) and its golden cases
- O25: [../../parity/modeling-tools-2.md](../../parity/modeling-tools-2.md) and its golden cases
- O15: [../../parity/rechecks-2.md](../../parity/rechecks-2.md) and its golden cases
- tests/golden/modeling-tools/ (O6, O11 §4, O25 cases) and the settings tables in docs/parity/modeling-tools.md and modeling-tools-2.md
- docs/parity/followups-1.md §4 (Extrusion field confirmed)

## Design decisions (planner)
- `forge-modeling::bevel(mesh, sel, mode, &BevelSettings)`. Edge bevel builds the offset curves per edge with miter patches at corners (Default, Uniform, Radial and Patch mitering), then rounds with `subdivision` segments using `tension` and `shape` Round. The User and Profile shapes are out of scope: return `Unsupported`.
- Polygon mode = an inset plus an extrusion lift (the default Extrusion 10 explains bevel-poly's +10, followups-1 §4).
- Proportional offset is stored as a fraction (O25 note: 0.3 → 60 on a 200 edge).

## Done when
- [ ] MODELING.md §3.
- [ ] Every bevel case matches, or is pinned with a reason.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Out of scope
The User/Profile bevel shapes (v1.1).

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
