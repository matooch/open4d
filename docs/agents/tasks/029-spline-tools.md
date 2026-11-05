# 029 · Spline tools and spline booleans

Status: done · Area: B · Depends on: 028 · Observations: O46 · Touches: geometry, modeling

## Goal
Chamfer, Create Outline, Cross Section, Reverse, Subdivide and spline booleans (union, subtract, intersect, and, or) match the O46 cases.

## Inputs
- Area design: [../../roadmap/areas/B-modeling.md](../../roadmap/areas/B-modeling.md)
- O46: [../../parity/spline-tools.md](../../parity/spline-tools.md) and its golden cases
- docs/parity/spline-tools.md

## Design decisions (planner)
- Booleans: flatten to the spline plane, then 2D polygon clipping with the `i_overlay` crate (MIT), then back to linear splines with the reference point order where derivable.
- Outline: an offset curve with miter joins.

## Done when
- [x] SPLINES.md §tools.
- [x] O46 cases match, or are pinned.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: tests added (`crates/generators/tests/spline_tools_parity.rs` extended with `tool_of`, 8 unit tests in `spline_tools.rs`, 5 tests in `crates/app/tests/spline_commands.rs`). Parity: chamfer 1/1, reverse 1/1, subdivide 1/1, booleans 5/5 (union, subtract, intersect, and, or), outline 0/2 (`KNOWN_MISMATCH` `outline`, `outline-neg`: same loops, order, direction and face lists, positions within 3.7e-3 against the 1e-4 tolerance, asserted with a 5e-3 bound). Together with brief 028 the folder is 14/16 exact, 2 pinned, 42 owned by briefs 037/038. No `UNSUPPORTED`.
- Rules derived (SPLINES.md section 8), with the proving case: Chamfer (`chamfer`: corner -> two points at the distance, quarter-arc handle `83/150 r`, zero-handle straight pieces, then the Adaptive sampler = the rounded rectangle); Reverse (`reverse`: list reversal); Subdivide (`subdivide`: one midpoint per span for 2); booleans (`union`, `subtract`, `and`, `or`, `intersect`): crossings of the sampled Uniform outlines (75, +-184.9824, not the Bezier curves), mapped to curve parameters by the chord fraction (0.7593439; the export's cut piece fits the exact de Casteljau piece to 6e-9 in handle length 25.0447 / direction 157.956 / end handle 26.633 once the new point is moved onto the crossing and the handle OFFSETS are kept), pieces in/out of the other outline, loop sets `a_out+ b_out+` (union), `a_in+ b_in+` (and), `a_out+ b_in-` (subtract), `b_out- a_in+` then `a_out+ b_in-` (or), all three (intersect), reversed piece first, result Bezier + Adaptive 5 degrees. Extrude face winding by loop orientation and nesting (`outline`, `outline-neg`, `or`, `intersect`, `circle-ring`), changing `spline_gens::extrude` (existing splines-2 and generator parity unchanged). Extrude Direction Auto guess of the test: sign of the total signed area (tie: first loop).
- Parity-pending: Subdivide meaning of the count (N parts or 2^(N-1) parts, both give one midpoint for 2), Flat chamfer, non-90-degree corners and curved neighbours, outline of open or non-circle splines, booleans with more than two crossings or tangent touches or several segments, loop order of `or`/`intersect`, the real Extrude Direction rule (brief 044).
- Outline hypotheses (3+) that failed: miter offset of the 36-point or Adaptive polyline (220.07 / 220.02), Tiller-Hanson offset of the control polygon (handle 118.95 vs 121.73), curvature based factors (mean curvature radius 200.06 / 200.09, needs 199.974). The export is the input scaled by exactly `1.100013035` about the centre (control radius 220.002607), `0.899986965` for -20. Follow-up cases: Create Outline of a Rectangle (Linear, miter) and of a circle with radius 100 / distance 10 (does the 0.0026 excess scale with the radius or the distance?), of an open Arc; a Subdivide with 3 subdivisions; Chamfer with Flat on, on a Star (non-90-degree corners); a boolean of a circle and a rectangle (four crossings), of nested circles and disjoint ones; Cross Section needs a GUI observation (not scriptable, no case, not implemented).
- Deviations from the design decisions: no `i_overlay` crate (no new dependency): the reference clips the splines' own Bezier pieces, found on the sampled outlines, so a polygon clipper could not reproduce the export; the clipping is own code (deterministic, pieces + crossings). Outline is an offset with miter corners as planned but with scaled handles, not a polyline offset. The commands are `spline.chamfer`, `spline.outline`, `spline.boolean` (one command, `op` argument), `spline.reverse`, `spline.subdivide`; Cross Section is not implemented.
- Files changed: crates/generators/src/{spline_tools (new),spline_sampling,spline_gens,editable_spline,lib}.rs, crates/generators/tests/spline_tools_parity.rs, crates/command/src/builtin/spline.rs, crates/app/tests/spline_commands.rs, crates/ui/src/icon_table.rs, crates/ui/icons/spline_{chamfer,outline,reverse,subdivide}.svg (new), docs/specs/{SPLINES,GENERATORS2,COMMANDS}.md, docs/manual/reference/{commands,mcp}.md (regenerated), docs/parity/STATUS.md, docs/roadmap/BACKLOG.md, this brief.
- Gate: `GATE: 1785 tests passed, 0 failed` ... `GATE: all passed`.
