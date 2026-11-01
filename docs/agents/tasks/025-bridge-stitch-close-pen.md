# 025 · Bridge, Stitch and Sew, Close Polygon Hole, Polygon Pen

Status: done (parity-pending: no golden case covers these tools; every rule is own design) · Area: B · Depends on: 020 · Observations: G3 · Touches: modeling, ui

## Goal
Topology-building tools: bridge two border loops or polygons, stitch, fill holes, and draw polygons point by point.

## Inputs
- Area design: [../../roadmap/areas/B-modeling.md](../../roadmap/areas/B-modeling.md)
- GUI session G3 ([../../parity/GUI-SESSIONS.md](../../parity/GUI-SESSIONS.md)), own design until it is answered: mark those rules **parity-pending**
- docs/parity/modeling-tools-2.md Notes (these returned false through the script; GUI only)

## Design decisions (planner)
- Pure functions: `bridge(mesh, loop_a, loop_b, segments)` (matches loop orientation by the shortest total edge length); `close_hole(mesh, border_edge)` (one N-gon); `stitch(mesh, pairs)`.
- Polygon Pen: click to add points or polygons and snap to existing points (uses snapping from 032 if present).
- Own design, parity-pending.

## Done when
- [x] MODELING.md §6.
- [x] Unit tests: bridging two open cube ends gives a closed tube; close_hole restores the face count.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- Result: parity 0/0. O25 could not run `bridge-two`, `stitch` and `close-hole` (scripted calls
  returned false; GUI only), so `tests/golden/modeling-tools/` has no case for these tools and nothing
  is `UNSUPPORTED` or `KNOWN_MISMATCH`. Tests added: `crates/modeling/tests/build.rs` (23: border loops
  and pinched borders, bridge of two open cube ends and of two polygons with exact volume, segments and
  ring positions, rotation by shortest total length and the tie rule, errors; close hole restores the
  deleted cube face and starts at the given edge; stitch of a disconnected polygon, middle/first,
  chains, collapsing polygon, errors; pen new points, snap, one corner, errors),
  `crates/app/tests/build_commands.rs` (9: each command headless, one undo step, stored selection,
  argument errors change nothing, schemas), `crates/ui/tests/building.rs` (8: hover preview leaves the
  document alone, click = one undo step, border-edge-only hover, two-pick Bridge and Stitch, Apply with
  selected edges, pen corners, snapping, closing on the first corner, Reset Values, menu).
- Commands: `mesh.bridge {node?, edges:[e,e] | polys:[p,q], segments?}`, `mesh.close_hole {node?,
  edge}`, `mesh.stitch {node?, points:[[a,b]..] | edges:[[e,e]..], position?: middle|first}`,
  `mesh.pen {node?, corners:[{point:i} | {pos:[x,y,z]}]}`. An edge is an id or a point pair. One undo
  step each; results `{node, points, polys, new_polys, new_points}` (`merged` for stitch). Bridge, hole
  and pen select the new polygons (pen with one corner selects the new point, stitch the merged points);
  the edge selection is cleared.
- Rules (all parity-pending; proof in MODELING.md §6 and the tests named there): border loop follows
  the direction of its polygon (`a_border_loop_runs_in_the_direction_of_its_polygon`); loops are
  paired in reverse so new quads are wound like the mesh, the rotation with the smallest total edge
  length wins, ties go to the smallest (`the_rotation_with_the_shortest_total_edge_length_is_chosen`,
  `a_tie_chooses_the_smallest_rotation`, `bridging_two_polygons_joins_a_stack_of_cubes_into_one_block`
  checks volume 20); intermediate rings and quad order (`segments_add_rings_between_the_loops`); hole
  polygon = reversed loop starting at the edge's start (`the_new_polygon_starts_at_the_start_of_the_given_edge`);
  stitch merges into the lowest index, Middle averages (`stitch_middle_and_first_choose_the_position`,
  `chains_of_pairs_merge_into_the_lowest_index`); edge pairs join `u-y`, `v-x` (`stitch_edges_pairs_the_facing_corners`);
  pen appends new points in corner order and keeps the given winding (`pen_*`).
- Deviations from the design decisions: Bridge fixes the pairing direction by winding and searches
  only the rotation (a reversed pairing would give an inverted tube), so "matches loop orientation by
  shortest length" is the rotation only. `stitch` takes point pairs; `stitch_edges` and the `edges`
  argument convert border-edge pairs. Bridge also accepts two polygons (removed, outlines bridged).
  Snapping from 032 does not exist yet: the pen snaps to existing points within the point pick radius
  itself. Bridging loops of different sizes, non-border edges and pinched borders are `Invalid`.
- Files: crates/modeling/src/{build,desc,lib}.rs, crates/command/src/builtin/{building,cutting,mod}.rs,
  crates/ui/src/{building,cutting,tools,app,lib}.rs, tests crates/modeling/tests/build.rs,
  crates/app/tests/build_commands.rs, crates/ui/tests/building.rs, docs/specs/{MODELING,COMMANDS}.md,
  docs/manual/reference/*, docs/parity/STATUS.md, docs/roadmap/BACKLOG.md.
- Open data that would settle the pending rules (G3): (1) Bridge on the 1/1/1 cube with top and bottom
  polygons deleted (O25 `bridge-two`) and on two equal-size polygons: result point/polygon order, which
  rotation the reference picks when edge lengths tie, whether it keeps the polygon order of the loops,
  segments' ring order, and whether unequal loop sizes are allowed; (2) Close Polygon Hole (`close-hole`):
  new polygon's corner order and start, and whether several holes are closed at once; (3) Stitch and
  Sew (`stitch`, 2/1/1 with polygon 6 disconnected): merge position (middle vs first/second), point
  order after removal, the sew tolerance, how edges are paired; (4) Polygon Pen: click to polygon
  workflow, snapping radius, how points are added to edges and polygons are extended.
