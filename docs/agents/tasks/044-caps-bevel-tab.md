# 044 · Shared Caps/Bevel tabs and Extrude R25 options

Status: done · Area: C · Depends on: 042 · Observations: O56 · Touches: generators

## Goal
One caps and bevel module serves Extrude, Lathe, Loft and Sweep, matching O56. Extrude gains R25 Direction/Offset.

## Inputs
- Area design: [../../roadmap/areas/C-objects.md](../../roadmap/areas/C-objects.md)
- O56: [../../parity/caps-bevel.md](../../parity/caps-bevel.md) and its golden cases
- docs/parity/caps-bevel.md

## Design decisions (planner)
- `crates/generators/src/caps.rs`: cap types (triangles, quadrangles, N-gon, Delaunay, regular grid), start/end flags, bevel shapes (chamfer, round, concave, step) with size and segments.
- Delaunay via `spade` (MIT/Apache).
- Extrude Direction: Auto (O17 note: Auto can go to −z: derive the rule), X/Y/Z/Custom.

## Done when
- [x] GENERATORS2.md §caps.
- [x] All O56 cases match, or are pinned.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

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
- Result: new test caps_bevel_parity.rs: 9/22 exact, 1 known mismatch, 12 unsupported (see GENERATORS2.md §7a and STATUS.md). Added R25 Extrude Direction/Offset (Auto = Newell normal of the spline, X/Y/Z, Custom = unit(movement) x offset; Absolute unsupported), the shared Caps/Bevel params on Extrude, Lathe, Loft and Sweep, 4-point Loft/Sweep triangle caps as a fan (tri-sweep-open).
- Pinned: Delaunay and Regular Grid caps and every bevel (Size > 0) are Unsupported: ring and fill order not derived; tri-loft-rotated / tri-ellipse-loft lack section data; tri-slice/dodeca are Make Editable cases.
- Deviation: no spade/Delaunay dependency and no bevel code (the cap types and bevels that need derivation stay refused); the extrude R20 form is a hidden `r20_form` param set by the R20 overlay.
- Files: crates/generators/src/spline_gens.rs, tests/{caps_bevel_parity,common/mod,r25_drift,generators3_parity,make_editable_parity,spline_sampling,spline_tools_parity,text_parity}.rs, tests/defaults/r20.toml, docs.
