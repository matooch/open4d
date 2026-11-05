# 028 · Editable spline object and point/tangent editing

Status: done · Area: B · Depends on: 003 · Observations: O3, O46 · Touches: core, geometry, generators, ui

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
- [x] SPLINES.md (new spec).
- [x] O46 editable and interp cases match.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: tests added (`crates/generators/tests/spline_tools_parity.rs` 6 tests, `crates/app/tests/spline_commands.rs` 6 tests, unit tests in `editable_spline.rs`); parity 6/6 exact: `editable-circle`, `editable-rectangle`, `interp-linear`, `interp-akima`, `interp-bspline`, `interp-bezier`. No `UNSUPPORTED` / `KNOWN_MISMATCH`. The other 52 cases of `tests/golden/spline-tools` (`interp-f-*`, `prim-f-*`: briefs 037/038 follow-ups; `chamfer`, `outline*`, booleans, `reverse`, `subdivide`: brief 029) are listed by name in the test, which fails if a new unclaimed case appears.
- Rules derived (SPLINES.md): Make Editable keeps the primitive's Bezier segments (`editable-circle`: 36 samples per ring with Bezier + Uniform 8; `editable-rectangle`: Linear, no handles, Adaptive); Intermediate Points None outputs the control points whatever the type, B-Spline included (all four `interp-*` cases, identical 8-vertex meshes); tangent read-back = 0.75 x geometric Bezier offset (Notes: 83 vs 110.667 for the circle; helix to a relative 1e-3). Parity-pending: Cubic/Akima/B-Spline curves under every other Intermediate Points mode (Cubic = Catmull-Rom is a guess), mirrored unused end handles (one form-only case), Cogwheel handles.
- Deviations from the design decisions: (1) `Value::Vec3List` added, but the scene file version is not bumped (a bump breaks ~10 pinned-version tests and v9 fixtures; documented in CORE.md section 6.8, planner follow-up). Segments use two `IndexList` params (`segment_ends`, `closed_segments`) instead of a new list kind. (2) Viewport drawing of points/handles and dragging in Points mode is NOT done: the viewport draws no splines at all yet, so it needs a spline-drawing brief first; editing is available through the `spline.*` commands (CLI/RPC/MCP/scripts). (3) Handles are stored as geometric Bezier offsets, not in the app's read-back scale. (4) The "shared module from 038" is reused (`spline_sampling`), nothing new there.
- Files changed: crates/core/src/{param,file}.rs, crates/generators/src/{editable_spline (new),lib,splines,polygon}.rs, crates/command/src/builtin/{spline (new),mod,param}.rs, crates/app/src/query.rs, crates/ui/src/{attributes,icon_table,widgets/readonly}.rs, crates/ui/icons/spline*.svg (3 new), the two new test files, docs/specs/{SPLINES (new),CORE,COMMANDS,EDITABLE}.md, docs/manual/reference/{commands,mcp}.md (regenerated), docs/parity/STATUS.md, docs/roadmap/BACKLOG.md.
- Open questions / follow-up observation suggestions: (a) a case with Cubic, Akima and B-Spline under Intermediate Points Natural or Uniform (e.g. `interp-akima-uniform`, an editable circle with a moved point so the curves differ), to pin those curve definitions; (b) the tangent read-back of a hand-moved point (does the app rescale 0.75 on write?); (c) a non-default editable Bezier spline (moved point plus changed handle) extruded, to prove the handle semantics under edits; (d) an editable Ring/Cogwheel with its segments, closed flags and point order (our Ring is 4+4 points); (e) viewport brief for drawing splines, points and handles.
