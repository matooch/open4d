# 028 · Editable spline object and point/tangent editing

Status: waiting: O46 · Area: B · Depends on: 003 · Observations: O3, O46 · Touches: core, geometry, generators, ui

## Goal
Splines become editable objects with points, tangents, segments and interpolation types, and Make Editable on spline primitives produces them.

## Inputs
- Area design: [../../roadmap/areas/B-modeling.md](../../roadmap/areas/B-modeling.md)
- O3: [../../parity/make-editable.md](../../parity/make-editable.md) and its golden cases
- O46: [../../parity/spline-tools.md](../../parity/spline-tools.md) and its golden cases
- docs/parity/spline-tools.md (editable circle/rectangle facts, interpolation cases)
- docs/parity/make-editable.md spline case (4 points)
- crates/geometry/src/spline.rs

## Design decisions (planner)
- Node type `forge.spline` with params: points (`Value::Vec3List`, add the variant), left/right tangents, segment counts, closed flags, interpolation (linear, cubic, Akima, B-spline, Bezier), and intermediate points (shared module from 038).
- Viewport: draw spline points and tangent handles in Points mode. Drag points and handles with the existing component transform.
- Parity: extruded meshes of the editable splines (O46) match, level A.

## Done when
- [ ] SPLINES.md (new spec).
- [ ] O46 editable and interp cases match.
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
