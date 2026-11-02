# 147 · Familiar default layout: command bar, tool palette, manager arrangement

Status: done (parity-pending G1) · Area: L · Depends on: 011 · Observations: G1 · Touches: ui

## Goal
The default window arrangement a user of the reference app expects: command bar on top, tool palette on the left, viewport centre, object manager top right, attributes bottom right, timeline at the bottom.

## Inputs
- Area design: [../../roadmap/areas/L-ui.md](../../roadmap/areas/L-ui.md)
- GUI session G1 ([../../parity/GUI-SESSIONS.md](../../parity/GUI-SESSIONS.md)), own design until it is answered: mark those rules **parity-pending**
- GUI-SESSIONS.md G1
- docs/specs/UI.md
- crates/ui (egui_dock)

## Design decisions (planner)
- Layout defined in code (serialisable `Layout`). Panel order from G1 when answered; until then the arrangement above, marked parity-pending.
- Buttons call registry commands only.
- Never copy the reference layout files or icons.

## Done when
- [x] egui_kittest check of the default layout (panels present and in place; geometry assertions instead of an image baseline).
- [x] Every button maps to a command id.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Out of scope
Icons (148).

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: `crates/ui/tests/layout.rs` (7 tests): default layout panels present and in place (command bar above palette/viewport, palette left of viewport, managers right with Objects above Attributes, timeline strip below, status last); `Layout::standard()` equals `Dock::default()`; JSON round trip and `Editor::set_layout`; bad version / duplicated panel / bad JSON rejected; every command-bar and palette button (enumerated by `all_buttons()`) maps to a registry command, has unique text, and `object.create` buttons name registered types; buttons run their commands (create, undo, redo, `select.mode`, failing `mesh.extrude` reports in the status bar); Reset layout restores the standard layout. No parity numbers (no golden data).
- Rules: all own design, **parity-pending G1** (areas, group order, labels). Spec: docs/specs/UI.md §11 (and §2 updated).
  Layout: menu bar; Top area: command bar; Left: tool palette; Centre: viewport; right column: Object manager over Attribute manager; Bottom: Timeline (+ Coordinates tab); status bar.
  Command bar: History `edit.undo`, `edit.redo`, `object.delete`; Objects `object.create` (forge.cube, sphere, cylinder, cone, plane, torus), `mesh.make_editable`; View `view.shading` (smooth, wire), `view.frame_all`; Animation `timeline.play`, `timeline.stop`, `key.record`.
  Palette: Mode `select.mode` (model, points, edges, polys); Selection `select.all/none/invert/grow/shrink`; Modeling `mesh.extrude`, `mesh.extrude_inner`, `mesh.bevel`, `mesh.subdivide`, `mesh.triangulate`, `mesh.optimize`, `mesh.reverse_normals`.
- Deviations: (1) not on `egui_dock`: it does not build against our egui 0.37 (UI.md top), so `Layout` is serde data over our own `Dock` model; new `Area::Top`, `Panel::CommandBar`, `Panel::ToolPalette`. (2) No image snapshot baseline: geometry assertions on the rendered widgets instead (the kittest harness here has no GPU renderer). (3) Button glyphs are short ASCII tags ("<-", "#", "^^"): the bundled egui fonts have no glyph for most Unicode symbols and egui panics on a missing glyph. Texts are `glyph label`, distinct from menu items so existing label-based tests stay unambiguous. (4) Palette modeling buttons run the registry command on the stored selection with default settings; the interactive tools (with previews, cuts, bridge, pen) remain in the Mesh menu because they need pointer input. Snapping, workplane and axis buttons are omitted: no registry commands exist.
- Existing tests changed: `crates/ui/tests/models.rs` (`default_dock_layout_is_ours` now expects Left [ToolPalette] and Top [CommandBar]; `moving_and_closing_panels` expects Left [ToolPalette, Attributes] and 7 panels in total). Nothing else.
- Files: crates/ui/src/{dock.rs,layout.rs,lib.rs,app.rs}, crates/ui/Cargo.toml (serde), crates/ui/tests/{layout.rs,models.rs}, docs/specs/UI.md, this brief, docs/roadmap/BACKLOG.md, Cargo.lock.
- Open questions: G1 answers (panel order, command bar group order, Standard layout areas) would settle the parity-pending order; the owner may want a snapshot baseline once a software renderer is available in tests.
- Gate: see commit message / final lines below.
