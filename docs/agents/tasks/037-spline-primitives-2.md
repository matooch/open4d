# 037 · Spline primitives II

Status: ready · Area: C · Depends on: 038 · Observations: O17 · Touches: generators

## Goal
Arc, n-Side, Star, Flower, 4-Side, Cogwheel, Cycloid, Profile, Cissoid and Formula splines, plus the Circle ellipse/ring and Rectangle rounding options, matching the O17 extrude cases.

## Inputs
- Area design: [../../roadmap/areas/C-objects.md](../../roadmap/areas/C-objects.md)
- O17: [../../parity/splines-2.md](../../parity/splines-2.md) and its golden cases
- docs/parity/splines-2.md (tables, 22 cases, notes on Cogwheel coupling and Extrude direction)
- crates/generators/src/splines.rs, spline_gens.rs (extrude)

## Design decisions (planner)
- One function per spline in `splines.rs`, each returning a `Spline` (points, tangents, closed). Intermediate points come from 038.
- Formula spline uses `forge-expr` (052). If 052 is not done yet, return Unsupported for formula-defaults and pin it.
- Cogwheel: involute teeth from the standard gear formulas (pitch radius, module, pressure angle). The coupled limits are TODO in O17; clamp to the observed defaults only.
- O17 note: Extrude Direction Auto sends some splines to −z. The test harness builds the extrude the way the case says; any direction rule found goes to 044.

## Done when
- [ ] GENERATORS2.md §splines extended.
- [ ] All 22 O17 cases match, or are pinned.
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
