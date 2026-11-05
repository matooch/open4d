# 034 · Multi-object component editing; numeric component coordinates

Status: done (parity-pending G3) · Area: B · Depends on: 006 · Observations: G3 · Touches: ui

## Goal
Edit the components of several selected polygon objects at once, and type exact positions for component selections in the Coordinates panel.

## Inputs
- Area design: [../../roadmap/areas/B-modeling.md](../../roadmap/areas/B-modeling.md)
- GUI session G3 ([../../parity/GUI-SESSIONS.md](../../parity/GUI-SESSIONS.md)), own design until it is answered: mark those rules **parity-pending**
- crates/ui/src/{component_edit,coordinates}.rs

## Design decisions (planner)
- The component target becomes a list of polygon objects. Picking hits any of them, and the pivot is the centroid over all of them.
- The Coordinates panel in component modes shows the pivot position, size and rotation. Entering values transforms the selection, in one undo step.

## Done when
- [x] EDITABLE.md §3 updated (new §6); UI tests.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- Result: 15 headless tests (`crates/app/tests/multi_object.rs`) and 12 UI tests including an egui_kittest box select and gizmo drag across two objects (`crates/ui/tests/multi_object.rs`). No parity data exists (G3), so every rule is parity-pending; no `UNSUPPORTED` / `KNOWN_MISMATCH` entries.
- Rules (EDITABLE.md §6, all own design, parity-pending):
  - Model: the component targets are the selected polygon objects while the active node is one; each keeps its own three stored sets and topology, nothing crosses objects (`select::targets`, `resolve_targets`, `write_selections`).
  - `select.*` (not `select.mode`) take `nodes?`; one "Select" step for all objects; an error in one writes nothing.
  - Click picks the nearest element over all targets by depth along the ray (Replace clears the other targets); a rectangle combines per target; both are one step.
  - Pivot = mean world position of all affected points; gizmo deltas become one world matrix `M`, applied per object as `W_i^-1 M W_i`; Scale follows the active object's axes; one merged undo step. `mesh.transform` takes `nodes?` and `space?` (default `object` for one object, `world` for several).
  - Soft selection and symmetry do not cross objects (decision): weights, radius (object-space units), surface distance and the mirror plane are per object.
  - `ToolCommand` tools (extrude, extrude_inner, bevel, slide, subdivide, triangulate, untriangulate, reverse_normals, optimize, weld, melt, dissolve, split, disconnect, connect_points, mirror, center_axis, array) take `nodes?`; default is every selected polygon object; one step; an object with an empty selection for which the tool fails is skipped (listed in `skipped`). `polys/edges/points` need one object.
  - `mesh.loop_cut`, `knife`, `bridge`, `stitch`, `close_hole`, `pen` stay single-object, reject `nodes` ("works on one object") and the tool panel says "<Tool> works on the active object only".
  - Coordinates panel in component modes: editable Pivot (position), Rotation (always shows 0; typed value rotates) and Size rows, one step per entry.
- Deviations from the design decisions: the pivot is the mean of all affected points rather than a per-object centroid mix (same thing for equal weights per point); the Coordinates "Local" space equals World when several objects are targets. `CommandContext::select_components` / `select_extra` gained a `nodes` parameter (the headless mock in `crates/command/tests/registry.rs` was adapted). `SceneView.component` stays `(NodeId, EditMode)` (primary target); other selected polygon objects are targets too.
- Files changed: crates/app/src/{select,lib}.rs, crates/command/src/{context}.rs and builtin/{object,tools,components,cutting}.rs, crates/ui/src/{component_edit,components,snapping,tools,viewport_panel,app,commands}.rs, tests (two new files, registry mock), docs/specs/{EDITABLE,MODELING,COMMANDS,UI}.md, docs/manual/reference, BACKLOG.md.
- Open questions: does the reference app let soft selection fall off across objects (by world distance)? How does it pick between overlapping objects, and what does its Coordinates panel show for rotation/size of a component selection? (G3 items.)
- Gate: `GATE: all passed`.
