# 033 · N-gon handling: reference triangulation order and edge visibility

Status: done · Area: B · Depends on: – · Observations: O6, O18 · Touches: geometry, generators, viewport

## Goal
N-gons are triangulated the way the reference exports them where the goldens show it (dodecahedron 12 triangles + 12 quads, the cut triangles of slices), and hidden N-gon edges are not drawn.

## Inputs
- Area design: [../../roadmap/areas/B-modeling.md](../../roadmap/areas/B-modeling.md)
- O6: [../../parity/modeling-tools.md](../../parity/modeling-tools.md) and its golden cases
- O18: [../../parity/primitives-3.md](../../parity/primitives-3.md) and its golden cases
- tests/golden/platonic/type-dodeca (O18 note: each pentagon is a triangle plus a quad)
- the slice cases pinned as geometric-only (cone/torus/capsule slice)
- docs/specs/GEOMETRY.md §6

## Design decisions (planner)
- Derive the exporter's N-gon split rule from the dodecahedron and the slice cut faces. If one rule explains them, switch those slice cases from geometric to exact.
- Viewport: draw only the original N-gon boundary edges, not the internal triangulation edges.

## Done when
- [x] GEOMETRY.md §6 updated.
- [x] The dodecahedron matches exactly; the slice cases are upgraded or keep a documented reason.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- Result: `crates/geometry/tests/triangulate_reference.rs` (6 tests) added. Parity: `cone/slice` is now exact (9/9 cone cases, `CUT_GEOMETRIC` shrank to `torus/slice`, `capsule/slice`); `loft/defaults` is now exact (both caps; `CAP_GEOMETRIC` shrank to `loft/three`); `loft/three` start cap exact, end cap differs in 2 of 54 cap triangles (one tie quad). `torus/slice` and `capsule/slice` stay geometric (reason below). `platonic/type-dodeca` is reproduced face by face by `split_pentagon`, but there is no Platonic generator yet (brief 035), so it is tested against the golden file rather than a generator parity case. Whole-workspace parity unchanged elsewhere.
- Rules derived (GEOMETRY.md §6.1):
  - R1 the triangle set is the constrained Delaunay triangulation (ear clip, then Lawson flips): proved by `loft/defaults` (all 26 interior edges of the 29-gon cap are locally Delaunay with margins 1e-5..3e-4 and our set is equal), `loft/three` start cap, `cone/slice` (12-gon with collinear corners, unique, equal; both cuts).
  - R2 the listing order: triangles by lowest walk corner, written from it counter-clockwise, ties by the angle of the next corner around it from "down" (world -Y projected into the plane): proved by `torus/slice` start and end cut (group of corner 7 is `(7,11,15), (7,8,9), (7,9,11)` because corner 11 is exactly below corner 7) and by all other listed cuts/caps, checked from the exported sets alone (`emission_order_reproduces_every_exported_listing`).
  - R3 dodecahedron: pentagon ring `(t, n, a, b, p)` becomes triangle `[n, p, t]` (faces 0..11) and quad `[p, n, a, b]` (faces 12..23); the ear tip of face i is vertex i (`type-dodeca`, all 12 faces).
  - Start cut of a lathe: walk `Q` ascending, triangles then wound the other way (`torus/slice`, `cone/slice`); start cap of a loft wound as the ring, end cap the reversed ring (`loft/defaults`).
- Parity-pending / not derivable: (a) cocircular polygons (`torus/slice`, `capsule/slice`, the symmetric quad of `loft/three` end ring): the export contains edges that are not locally Delaunay even on its own float32 coordinates (6 of 15 interior edges of the torus cut have a positive in-circle determinant of 1e-7 relative), so the reference decided on noise below the file's precision; our ties keep the ear-clip diagonal (matches 3 of the 4 tie quads seen). (b) "Down" for polygons whose normal is parallel to Y (-Z chosen; loft caps are insensitive). (c) Sweep caps: `rechecks/sweep-open-triangles` lists its 34 cap triangles in an order that is neither sorted nor R2 (first corners 3, 19, 33, 6, 10, 35, 10, ...), so Sweep caps may use another path; our Sweep caps use the Loft path. (d) Extrude triangle caps over 4 points and the tool Triangulate (fan) are unchanged: no golden shows a larger outline.
- Deviations from the design decisions: the slice cases were not all upgraded (see (a)); the viewport needed no change: it already builds its line buffer from `PolyMesh::edges()` (polygon boundary edges), so only a geometry-level test (a pentagon has 5 edges, 3 triangles) and the spec note were added. `triangulate_poly` (ear clipping) is kept for Boole, tools and shading; the new `triangulate_delaunay` is used by lathe cuts and loft/sweep triangle caps only.
- Files changed: crates/geometry/src/triangulate.rs, crates/geometry/tests/triangulate_reference.rs (new), crates/generators/src/{lathe,spline_gens}.rs, crates/generators/tests/{parity,generators_parity,editable}.rs, crates/io/tests/export_parity.rs, docs/specs/{GEOMETRY,PRIMITIVES2,GENERATORS2}.md, docs/parity/STATUS.md, docs/roadmap/BACKLOG.md, this brief.
- Open data that would settle the pending rules: (1) an export of a cut/cap on a polygon that is not cocircular and not mirror-symmetric (e.g. a Torus slice with a profile segment count that makes the circle uneven, or a Loft of an elliptic spline) to confirm R1 beyond the lofts; (2) the same Loft cap with the section rotated (Start Angle) to see whether tie diagonals follow the corner numbering; (3) a Sweep with an open path and a non-circular closed profile, Caps Type Triangles, to derive the Sweep cap order; (4) a slice on a cutting plane normal to Y (Orientation +X/+Z primitive) to fix "down" for normals parallel to Y; (5) the Platonic Dodecahedron at Segments > 1 (does each pentagon keep the triangle + quad split).
- The last lines of the gate output: `advisories ok, bans ok, licenses ok, sources ok` / `cleanroom: ok` / `ok: 471 cases` / `GATE: all passed`.
