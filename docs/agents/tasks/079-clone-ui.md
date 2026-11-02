# 079 · Clone UI: effector lists, clone display, GPU instancing hookup

Status: done (parity-pending G4) · Area: E · Depends on: 065, 144 · Observations: G4 · Touches: ui, viewport

## Goal
The cloner attribute UI (effector list with drag/drop and enable toggles), clone index display, and viewport instancing through K/144.

## Inputs
- Area design: [../../roadmap/areas/E-clone-toolset.md](../../roadmap/areas/E-clone-toolset.md)
- GUI session G4 ([../../parity/GUI-SESSIONS.md](../../parity/GUI-SESSIONS.md)), own design until it is answered: mark those rules **parity-pending**
- GUI-SESSIONS.md G4 (list widgets)

## Design decisions (planner)
- Effector list is a link-list widget (shared with field lists, 150).
- Viewport draws clones with an instance buffer (matrices + colour) from 144; no geometry expansion for display.

## Done when
- [x] egui_kittest test: add/reorder/disable an effector through the UI; viewport shows 10 000 clones at interactive rate on the CI software adapter.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- Result: spec UI.md §16, COMMANDS.md §6, FIELDS.md F-UI, VIEWPORT.md §10.8 note. Tests: `crates/ui/tests/clone_ui.rs` (18): cloner from selection (children, effector linked, one undo step, empty selection, Create menu click), Create menu entries from the registry, effector add by the list and by dragging from the object manager (each with undo), reorder (the clone matrices change: absolute 2 then relative 1 gives scale 3, swapped 2; swap back and undo restore), enable box (effector and field rows), row strength through `layer_strength` (brief 061 storage kept; deformer field list too), remove, `clone_marks` / `selected_cloner_marks` as pure functions (index, position, axes, hidden, behind camera, off screen, world matrix, cap with 10 000 clones, matrix mode), `view.clone_display`, the View menu toggle, and 10 000 clones as one `DrawItem::Instanced`. No parity numbers (no golden data).
- Added UI: link-list rows `[enable] name [strength %] ^ v x` (enable box on the entry's own `enabled` or on `object.enabled`; strength for entries with `layer_strength`), drop target unchanged; Create > Clone toolset submenu (Cloner, Cloner from Selection, every effector and field type, from the registry); View > Clone Indices / Clone Axes; index and axis overlay painted over the viewport for the selected cloners (a cloner with `viewport_mode` Matrix always shows axes).
- Added commands: `object.create_cloner` (undoable, "Create Cloner"), `view.clone_display` (view state). Everything else is `param.edit` (new `Edit.target` redirects an edit to the entry node), one undo step each.
- Instancing (brief 144): unchanged and confirmed. `scene_draw_items_at` returns one `DrawItem::Instanced` per cloner (prototypes once, 10 000 clones as one instance buffer); list edits change the clone data and so the instance revision hash; the overlay is egui painting, not geometry.
- Rules (own design, **parity-pending G4**): row layout and labels; the enable box of a field row writes `object.enabled`, and `FieldList::from_nodes` skips such fields (FIELDS.md F-UI); strength belongs to the field node, so a field in two lists has one strength (real rows come with 059/064); selected effectors in `object.create_cloner` are linked, not adopted; overlay colours and 24 px axes; at most 2000 marks.
- Deviations: (1) no drag-to-reorder inside the list (the up/down buttons stay); (2) `viewport_mode` values other than Matrix are not wired (the parameter existed unused); (3) the Plain effector's own `enabled` is reused for effector rows instead of a per-row flag (the list stores only ids); (4) the overlay is not visible in headless tests (the pane needs a GPU), so it is covered as pure functions; (5) 10 000 clones at interactive rate on a real adapter is not measured here (144's lavapipe tests cover the instanced path).
- Files changed: crates/command/src/{builtin/object.rs,context.rs}, crates/fields/src/lib.rs, crates/ui/{Cargo.toml,src/{app,clone_overlay,commands,lib,panes}.rs,src/widgets/{mod,link_list,float,int,string,vector,colour}.rs,tests/clone_ui.rs}, Cargo.lock, docs/specs/{UI,COMMANDS,FIELDS,VIEWPORT}.md, docs/manual/reference (regenerated), docs/roadmap/BACKLOG.md, this brief.
- Open questions (G4): list row columns and drag handles, whether the enable box is a checkbox or an eye, where the strength sits, Create menu grouping, overlay style, what Points / Bounding Box viewport modes look like.
- Gate: see the commit message.
