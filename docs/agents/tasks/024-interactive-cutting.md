# 024 · Interactive cutting: Loop/Path Cut, Knife (line, plane, loop), Slide

Status: done · Area: B · Depends on: 020, 023 · Observations: G3 · Touches: modeling, ui, viewport

## Goal
Cutting tools in the viewport: loop cuts with offset and count, the knife in three modes, and edge/point slide.

## Inputs
- Area design: [../../roadmap/areas/B-modeling.md](../../roadmap/areas/B-modeling.md)
- GUI session G3 ([../../parity/GUI-SESSIONS.md](../../parity/GUI-SESSIONS.md)), own design until it is answered: mark those rules **parity-pending**
- crates/viewport/src/picking.rs (edge picking)

## Design decisions (planner)
- Pure functions `loop_cut(mesh, start_edge, offsets: &[f64])`, `knife_line(mesh, plane_or_segment)` and `slide(mesh, sel, amount)`. The interactive layer handles the hover preview line and click to commit, and the HUD shows count and offset.
- Behaviour follows the public manual plus G3 answers when they arrive. Until then, own design, parity-pending.
- Commands `mesh.loop_cut {edge, offsets}` and `mesh.knife {mode, ...}` so the tools are scriptable.

## Done when
- [x] MODELING.md §5.
- [x] Unit tests on cubes and cylinders (exact counts and positions); UI tests for hover and commit.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- Result: parity 0/0. There is no golden case for these tools: O25 could not run `loopcut-mid`,
  `loopcut-quarter` and `knife-line` (scripted calls did nothing) and no Slide case exists, so
  nothing is `UNSUPPORTED` or `KNOWN_MISMATCH`. Tests added: `crates/modeling/tests/cut.rs` (19:
  loop cut on cube, prism, open grid, triangle end; offsets/orientation; knife plane, corners,
  miss, concave, restriction, line in parallel view and from the eye; slide chain, loop, sides,
  points, errors), `crates/app/tests/cut_commands.rs` (6), `crates/ui/tests/cutting.rs` (9: hover
  preview leaves the document alone, click = one undo step, count/fixed offset, knife loop, panel
  plane, knife drag, slide drag, menu), one picking test.
- Commands: `mesh.loop_cut {node?, edge (id | [a,b]), offsets? | count?}`; `mesh.knife {node?, mode:
  line|plane|loop, polys?, plane: point+normal, line: from+to+(eye|direction)+visible_only?, loop:
  edge+offset?}`; `mesh.slide {node?, edges?, points?, settings:{amount [-1,1], toward}}`.
  Loop cut / knife select the new edges and clear polys/points; slide keeps the selection. One
  undo step each.
- Interaction (own design, parity-pending): hover finds the edge (`pick_edge_param`, new in
  forge-viewport) and the position along it, preview drawn in place of the mesh, HUD line, click
  commits; Knife Line / Plane by dragging a screen line (plane through the eye and the line,
  Line also bounds the cut to the segment); Knife Loop = single loop cut; Slide uses the
  standard tool drag (amount, one viewport height = 2.0). Panel: count, fixed offset, offset,
  visible only, plane point/normal; Reset Values, Apply, Done as for the other tools.
- Rules (all parity-pending, proof in MODELING.md §5 and the tests named there): ring of quads
  with parallel orientation (`loop_cut_around_a_cube`, `offsets_follow_the_start_edge_in_every_quad`);
  non-quad neighbours get the points (`a_neighbour_that_is_not_a_quad_gets_the_point`,
  `cylinder_barrel_and_rim`); chords paired along the cut line (`knife_concave_polygon_gets_two_chords`);
  on-plane corners (`knife_plane_through_corners_adds_no_points`); extent and visibility
  (`knife_line_in_a_parallel_view`, `knife_line_from_the_eye_matches_the_plane`); slide left/right
  rails (`slide_a_chain_of_edges`, `slide_needs_a_side`).
- Deviations: Path Cut is not a separate mode (use Knife line); a knife chord that ends outside the
  segment is dropped whole instead of trimmed; Slide and the cutting tools are listed apart from
  `tool_descs()`/`tools()` (`cutting_descs`, `interactive_cut_descs`, `cutting_tools`) so the older
  tests' lists stay valid.
- Files: crates/modeling/src/{cut,slide,desc,lib}.rs, crates/command/src/builtin/{cutting,tools,mod}.rs,
  crates/viewport/src/picking.rs, crates/ui/src/{cutting,tools,app,lib}.rs, tests above, docs/specs/{MODELING,COMMANDS}.md,
  docs/manual/reference/*, docs/parity/STATUS.md, docs/roadmap/BACKLOG.md.
- Open data that would settle the pending rules (G3 items 1-2 plus a few observations): (1) Loop/Path
  Cut on the 1/1/1 cube at half and quarter height: resulting point order (new points' indices and
  positions), polygon order, and which edge/direction the quarter offset is measured from;
  (2) what the HUD shows and how count and offset are changed (scroll, keys, drag); (3) Knife Line on
  the top polygon with a known segment: whether cuts that end inside a polygon leave a dangling cut,
  the visible-only default and its effect on back faces; (4) Knife Plane and Loop steps; (5) Slide on a
  loop of the 2/2/2 cube and on a single edge: value range, which side is positive, clamping, and
  point slide's target; (6) whether Path Cut has a distinct mode.
- Gate (last lines): `GATE: 674 tests passed, 0 failed`; `cleanroom: ok`; `ok: 471 cases`; `GATE: all passed`.
