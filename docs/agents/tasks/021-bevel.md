# 021 · Bevel tool (all modes)

Status: done · Area: B · Depends on: 020 · Observations: O6, O25, O15 · Touches: modeling, ui

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
- [x] MODELING.md §3.
- [x] Every bevel case matches, or is pinned with a reason.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Out of scope
The User/Profile bevel shapes (v1.1).

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: tests added: `crates/modeling/tests/tools_parity.rs` extended (Bevel cases and `rechecks/extrude-bevel-subdiv`), `crates/modeling/tests/bevel.rs` (9: defaults, closed meshes stay closed for edge/four edges/solid/polygon, removed volume and arc radius of a fine chamfer, edge order independent of selection order, proportional on a non-cube box, polygon lift, every unsupported option, bad selections/valence-4/corner/open mesh, a non-right-angle edge), `crates/app/tests/tools.rs` (+4: `mesh.bevel` by point pair and edge id as one undo step, stored selection, errors change nothing, enum/component schema), `crates/ui/tests/tools.rs` (+2: Bevel preview/drag/apply on a stored edge selection with enum settings, panel with combo rows), one Rhai script. **Parity 9/9 exact** (8 `bevel-*` cases of `modeling-tools` + `rechecks/extrude-bevel-subdiv`), raw and after `export_weld`; `UNSUPPORTED` and `KNOWN_MISMATCH` empty. Extrude/Inset still 11/11.
- Rules derived (spec docs/specs/MODELING.md §3), with the proving case:
  - Polygon mode = Extrude Inner mitre + lift by `extrusion` (default 10) along the group normal, caps in place, sides `[a', a, b, b']` (`bevel-poly`; `bevel-poly-extrusion0` for Extrusion 0).
  - Edge chamfer profile = circle arc as a rational quadratic (control point = the edge, weight cos(phi/2)) at uniform t: the sub-2 points (97.445208, -89.780834) are on the radius-20 circle at 29.28 degrees, not 30 (`bevel-edge-sub`); 45 degrees at sub 1 (`bevel-edges-top`).
  - Edge direction a->b = the lower half-edge index (4*poly + corner for quads); strips run from the side that holds b->a to the side that holds a->b; strips are appended row 0 of every edge first, then rows 1..N edge by edge, rows >= 1 start one corner later (`bevel-edges-top` face 9 = edge 7->1, `bevel-edge-sub` faces 7, 8, `bevel-mitering-uniform` faces 6..17).
  - New points are created in two passes (end points, then intermediates; per first-met vertex in edge order, a before b) and appended, then the old points of all beveled vertices are removed by ascending swap-remove (last point moves into the slot). This is the only rule that explains the golden files having new points in slots 1, 3 (5, 7) (`bevel-edge`, `bevel-edge-sub`, `bevel-edges-top`, `bevel-mitering-uniform`).
  - End points: one selected edge: line of each level cut by the end polygon's plane; two selected edges: closest point of the two strips' level lines (cube corner points), shared by both strips; faces: the polygon between the two edges gets `C_0`, the two others `C_{N+1}`; the cap gets the whole profile in walk order (`bevel-edge-sub` polygons 1, 3).
  - Solid: keeps the old points, same four new points; the two edge polygons are cut, the part with the first corner stays in place and the other part is appended in polygon order; caps get `[p1, v, p2]` (`bevel-solid`).
  - Proportional offset = fraction of the beveled edge's length (`bevel-proportional`, 0.3 x 200 = 60).
  - Extrude: the Bevel value also has no effect at Subdivision 2 (`rechecks/extrude-bevel-subdiv` now in the parity test).
  - **Parity-pending**: angles other than 90 degrees (all data are cube edges), proportional relative to the edge itself rather than to the adjacent polygon edges, Mitering Default = Uniform, Limit, Phong Break no effect, rows >= 1 for Subdivision >= 3, interior group points under the polygon lift, creation order at vertices where the fan orientation differs from the cube's.
  - `Unsupported`: point mode, Shape User/Profile (out of scope), Offset Mode Radial, Tension or Depth other than 1, Mitering Radial/Patch, Ending, Partial Rounding, Corner/Rounding N-gons, Solid with Subdivision / two edges at a vertex / in polygon mode, polygon mode with Subdivision or Proportional, open or non-manifold edges, vertices that are not valence 3, three selected edges at a vertex.
- Deviations from the design decisions:
  - Point mode is `Unsupported`: the brief lists three modes but no golden case has a point selection.
  - The tool function type is now `fn(&PolyMesh, &Selection, &Settings)` (was polygons only) so Bevel can read edges; `tool_fn(id)` returns it, `Tool::preview` and `tool_target` carry the whole stored selection. The command takes `edges` (ids or point pairs) and `points` besides `polys`; the tool UI got combo rows for enum settings. `EmptySelection` now reads "nothing is selected".
  - Tension, Depth and the other settings without data return `Unsupported` instead of being guessed (the design said "using tension").
  - The `extrusion` setting is shown in every mode (the reference hides it outside polygon mode).
- Files changed: crates/modeling/src/{bevel.rs (new),desc.rs,extrude.rs,lib.rs,settings.rs}, crates/modeling/Cargo.toml, crates/modeling/tests/{bevel.rs (new),tools.rs,tools_parity.rs}, crates/command/src/builtin/tools.rs, crates/ui/src/{app,tools}.rs, crates/ui/tests/tools.rs, crates/app/tests/tools.rs, crates/script/tests/script.rs, docs/specs/{MODELING,COMMANDS}.md, docs/manual/reference/{commands,mcp}.md (regenerated), docs/parity/STATUS.md, docs/roadmap/BACKLOG.md, this brief.
- Open questions or data that would settle them:
  - A bevel of a non-90-degree edge (a wedge or a box with a 45-degree face) with Subdivision 2, to confirm weight cos(phi/2) and the level lines for other angles; the same with Proportional on a non-square face (distance from the beveled edge or from the adjacent edge), and a skewed corner for the closest-point rule.
  - Mitering Default vs Uniform at an angle other than 90 degrees, and Subdivision 2 with Default mitering at the cube corners; Radial and Patch mitering.
  - Subdivision 3 or more on 4 edges (row order and rotation), and Solid with Subdivision 1.
  - Polygon mode with Subdivision 1, and a group of two or more polygons (interior points); polygon mode in Solid.
  - Point mode (one cube corner, Offset 20) and a full corner of three beveled edges.
  - Tension 0.5 / 2 and Depth 0.5 on bevel-edge-sub; Limit off with an offset larger than the face; Ending Extend/Inset; Partial Rounding Full/Convex; Corner and Rounding N-gons on; Radial offset mode.
  - A beveled vertex of valence 4 (a cube with 2 segments, edge in the middle).
- The last lines of the gate output: `GATE: 599 tests passed, 0 failed` / `cleanroom: ok` / `ok: 471 cases` / `GATE: all passed`
