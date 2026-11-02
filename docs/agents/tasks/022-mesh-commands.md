# 022 · Mesh commands: Subdivide, Triangulate, Untriangulate, Reverse, Optimize, Weld, Melt, Dissolve, Split, Disconnect, Connect, Mirror, Center Axis

Status: done · Area: B · Depends on: 020 · Observations: O6, O11, O25 · Touches: modeling, ui

## Goal
The common mesh commands exist as registry commands and match the observed cases.

## Inputs
- Area design: [../../roadmap/areas/B-modeling.md](../../roadmap/areas/B-modeling.md)
- O6: [../../parity/modeling-tools.md](../../parity/modeling-tools.md) and its golden cases
- O11: [../../parity/followups-1.md](../../parity/followups-1.md) and its golden cases
- O25: [../../parity/modeling-tools-2.md](../../parity/modeling-tools-2.md) and its golden cases
- tests/golden/modeling-tools/ (O6, O11 §4, O25 cases) and the settings tables in docs/parity/modeling-tools.md and modeling-tools-2.md

## Design decisions (planner)
- One pure function per command in `forge-modeling`, one command each (`mesh.subdivide`, `mesh.triangulate`, …). Commands without readable settings in O25 use the documented values from the cases plus our own defaults, marked parity-pending.
- Subdivide with Hyper on: O25 shows the export is not rounded (bbox ±100), so check whether the smoothing applies only on display. Derive this from subdivide-smooth.
- Split creates a second object (`Cube.1`) holding a copy of the selected polygons (O25 split-top).
- Center Axis moves the object position to the bounding-box centre and compensates the points (O25 center-axis).
- Mirror with an explicit plane point and normal (O25 mirror-x). Duplicate off flips the winding.
- Dissolve and Close Polygon Hole had no scripted result in O25, so implement them as our own design, marked parity-pending (G3).

## Done when
- [x] MODELING.md §4.
- [x] Every matching case passes, and the rest are pinned with reasons.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Out of scope
Interactive tools (024, 025).

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: tests added: `crates/modeling/tests/tools_parity.rs` (+`mesh_commands_match_the_reference`, 15 golden cases), `crates/modeling/tests/commands.rs` (18: counts and closedness of Subdivide, Hyper volume, fallbacks, Triangulate n-gon / concave / subset, Untriangulate round trip / N-gons / hinge / concave pair, Reverse, Optimize switches and tolerance, Weld swap-remove and collapse, Melt interior point / hole / groups / lone polygon / closed surface, Dissolve edges / points / polygons, Split, Disconnect, Connect, Mirror, Center Axis, vector settings), `crates/app/tests/mesh_commands.rs` (7: one undo step per command, Melt selection, Split object and naming, Center Axis with a child, errors change nothing, Mirror duplicate, schemas), `crates/ui/tests/tools.rs` (+2: no drag for setting-less commands, vector rows, Mesh menu and panel). **Parity 15/15 exact** (raw file and after `export_weld`; `COMMAND_UNSUPPORTED` and `COMMAND_KNOWN_MISMATCH` empty): subdivide-1, subdivide-smooth, subdivide-2, triangulate, untriangulate, reverse-normals, optimize, optimize-real, weld-two-points, melt-two, split-top, disconnect-top, connect-points, mirror-x, center-axis. Per command: Subdivide 3/3, Triangulate 1/1, Untriangulate 1/1, Reverse Normals 1/1, Optimize 2/2, Weld 1/1, Melt 1/1, Split 1/1, Disconnect 1/1, Connect 1/1, Mirror 1/1, Center Axis 1/1; Dissolve has no case (own design). Extrude/Inset/Bevel still 20/20.
- Rules derived (spec docs/specs/MODELING.md §4), with the proving case:
  - Subdivide uses its own lattice order, different from the generator's at level 1 and level 2: points = originals, edge points (`edges()` order, along the first-seen direction), then each polygon's interior points `R` outer, `C` inner; polygons = the `2^L x 2^L` cells row by row, the cell next to corner `c2` replaces the polygon, the others are appended (`subdivide-1` face 0 `20 9 3 10`, faces 6..8; `subdivide-2` face 0 `52 13 3 14`). Hyper on = Catmull-Clark (`subdivide-smooth`: corners at 5/9, edge points 75, face points unchanged), so the export **is** rounded; the bbox stays +-100 only because the face points do not move. The design question "display only?" is answered: smoothing is in the mesh.
  - Triangulate = fan from corner 0, the first triangle keeps the index, the second of every quad is appended after all polygons (`triangulate`).
  - Untriangulate = each triangle takes the lowest-index coplanar neighbour that gives a strictly convex quad; the result is the boundary loop started at the first boundary edge of the lower triangle (`untriangulate`: tri 0 pairs with 24, not 26).
  - Reverse Normals keeps the first corner `[p0, p_n-1 .. p1]` (`reverse-normals`, also `mirror-x`).
  - Removal of points **and polygons** is swap-remove in ascending order (`weld-two-points`: point 7 moves to slot 1; `melt-two`: polygon 9 moves to slot 7, so the bottom polygons come out as 9, 8).
  - Weld = average at the lowest index (`weld-two-points`); Melt/Dissolve/Untriangulate share the boundary-loop merge (`melt-two` `[1,11,9,7,5,3]`).
  - Split copies the polygons with the points in ascending original index and leaves the source whole (`split-top`); Disconnect copies the shared points ascending, polygons replaced in place (`disconnect-top`); Connect Points cuts `p[0..=i]+p[j..]` in place and appends `p[i..=j]` (`connect-points`); Mirror reflects in place and reverses keeping the first corner (`mirror-x`); Center Axis moves the bbox centre into the position (`center-axis`: centre (50, 0, 0)); Optimize merges points within the tolerance into the lowest index and drops the unused (`optimize-real`).
  - **Parity-pending**: every default the O6/O25 forms could not read (all but the values the cases set), Hyper default off (taken from `subdivide-1`'s description since the case has no settings), Dissolve (the O25 case did not run, own design), Connect on edges (Unsupported), Optimize survivor order after removing non-final points and its defaults, Melt of interior points and of a lone polygon, Disconnect's "only points shared with outside polygons are copied", Mirror with Duplicate on, Split naming `<name>.N`, the Center Axis children rule and `point_average`, Subdivide of non-quads and level >= 3 (lattice rule extrapolated from levels 1, 2), Triangulate of n-gons, Untriangulate N-gons and its angle default.
- Deviations from the design decisions:
  - The tool framework is reused as is: all 13 commands are `ToolDesc` entries and appear in the Mesh menu; they have an empty `drag_param` (no drag) and `ToolDrag::begin` returns `None` for such tools. `Settings` gained vec3 values (Mirror's plane), the Attribute tool panel got a vec3 row, `ToolError` got `Invalid`.
  - "Empty selection = every polygon" for Subdivide, Triangulate, Untriangulate, Reverse Normals, Mirror; Optimize and Center Axis take the whole mesh (spec §4).
  - Split and Center Axis need more than a mesh replacement: `mesh.split` adds a polygon object (`forge_generators::spawn_split`, named `<name>.N`, selected) and `mesh.center_axis` writes `transform.pos` (and shifts direct children back) in the same undo step.
  - Dissolve and Close Polygon Hole: Dissolve is implemented (edges, points, polygons); Close Polygon Hole belongs to brief 025 and is not in this brief's list.
  - Spec sections: the brief names MODELING.md §4, so Parity and Checks moved to §7 and §8 (briefs 024 and 025 reserve §5 and §6). Two existing assertions changed by the new tool count: `tools.rs` `tool_descs().len()` 3 -> 16 and the id list in `crates/ui/tests/tools.rs`.
  - Also in this branch, as a separate commit: `golden check | head -1` no longer panics on a closed pipe.
- Files changed: crates/modeling/src/{desc,lib,settings}.rs and new {merge,points,surface,util}.rs, crates/modeling/tests/{commands (new),tools_parity,tools}.rs, crates/command/{Cargo.toml,src/builtin/tools.rs}, crates/generators/src/{lib,polygon}.rs, crates/ui/src/{app,tools}.rs, crates/ui/tests/tools.rs, crates/app/tests/mesh_commands.rs (new), Cargo.lock, docs/specs/{MODELING,COMMANDS}.md, docs/manual/reference/{commands,mcp}.md (regenerated), docs/parity/STATUS.md, docs/roadmap/BACKLOG.md, this brief; crates/parity-harness/src/bin/golden.rs (separate commit).
- Open questions or data that would settle them (for follow-up observation forms):
  - Read the defaults of every command's dialog in the GUI: Subdivide (Hyper, Angle), Triangulate, Untriangulate (Angle, N-gons), Optimize (Tolerance and the three switches), Weld (tolerance, target), Mirror (Duplicate, Weld, Tolerance), Center Axis to (targets).
  - Subdivide 3 levels, a mixed mesh (cube with one triangulated face) and a triangle, and Subdivide of a polygon subset; each with Hyper off.
  - Triangulate of a pentagon / hexagon and a concave quad; Untriangulate with N-gons on and a tolerance case (a slightly bent pair).
  - Melt of a 3 x 3 patch with an interior point (does the point go, and in what order), Melt of one polygon, Melt of a ring; Dissolve of one edge, of a point, and of a polygon (the `dissolve-edge` GUI run).
  - Disconnect of a 2 x 1 x 1 cube's top with Preserve Groups on and off, and of all polygons (copies of unshared points).
  - Optimize on two cubes with Connect's Weld off (merges and the order of survivors when the removed points are not the last ones).
  - Mirror with Duplicate on, with Weld, on a mesh that straddles the plane; Mirror of a polygon subset.
  - Center Axis to with a child object; Split with two disconnected polygons selected and naming of a second split.
  - Connect Points with edges selected and with three points in one polygon.
- The last lines of the gate output: `GATE: 627 tests passed, 0 failed` / `cleanroom: ok` / `ok: 471 cases` / `GATE: all passed`
