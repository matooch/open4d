# 05 · Point, edge and polygon modes with selection

Status: done · Branch: `batch-08` · Planner: main session · Implementer: Claude Sonnet 5.5 (local, macOS)

## Goal
With a polygon object selected you can switch between Model, Points, Edges and Polygons
mode, click and drag-rectangle to select components in the viewport, see the selection
highlighted, and the selection is stored on the object (undoable, saved).

## Inputs
- Brief 03 (`forge.polygon`, `Value::Mesh`).
- `crates/viewport/src/picking.rs` (`pick_point`, `pick_edge`, `pick_poly`,
  `rect_select`, `Element`), `crates/geometry/src/selection.rs` (`IndexSet`,
  `Selection`, conversions, grow/shrink), `docs/specs/GEOMETRY.md` §1 (edge ids), §4,
  `docs/specs/VIEWPORT.md` (picking), `crates/viewport/src/render.rs` (`MeshBuffers::new`
  already takes a `&Selection`: check what it draws).
- `crates/ui/src/viewport_panel.rs`, `crates/ui/src/manipulate.rs` (object picking),
  `crates/ui/src/state.rs`.

## Design decisions (planner)
1. **Storage**: three hidden, non-animatable params on `forge.polygon`:
   `selection.points`, `selection.edges`, `selection.polys`, each a new
   `Value::IndexList(Vec<u32>)` (sorted, unique; `ParamKind::IndexList`; validate sorts
   and dedupes). Edge ids are `PolyMesh::edges()` ids (GEOMETRY.md §1). Selection
   changes go through `doc.transact("Select", …)` so they undo and save. Setting a
   param on every click is fine; one drag-rectangle = one transaction.
2. **Mode** is editor state: `EditorState.mode: EditMode { Model, Points, Edges, Polys }`
   (default `Model`). Component modes act on the **active** object only, and only if
   it is a `forge.polygon`; otherwise clicks fall back to object picking.
3. **Interaction** (level C, parity-pending until brief 10 observes the reference):
   click = replace selection with the picked element (empty click clears); Shift+click =
   add; Ctrl/Cmd+click = remove; drag on empty space = rectangle select with the same
   modifiers; "visible only" on by default (`rect_select`'s `visible_only`). Select
   All / Deselect All / Invert in the Edit menu (Cmd/Ctrl+A, Cmd/Ctrl+Shift+A, none) act
   on the current mode's set. Grow/Shrink selection in a "Select" menu.
4. **Display**: in a component mode, draw the object's points (Points mode) / edges
   (Edges) / polygon outlines (Polys) and highlight the selected ones in the theme's
   selection colour (add the colour to `crates/ui/src/theme.rs` / the viewport palette).
   Other objects draw as usual.
5. Mode buttons in the viewport header for now (four toggle buttons labelled with text);
   icons and the left tool palette come in brief 10.

## Hints (planner)
- Keep the picking/selection logic in pure functions in `crates/ui/src/` (e.g.
  `components.rs`: `fn pick_component(mesh, cam, px, mode) -> Option<u32>`,
  `fn apply_click(sel, picked, modifiers) -> IndexSet`) so tests don't need a GPU.
- The picking functions work in world space; transform the mesh by the object's world
  matrix (or the ray by its inverse) the way object picking does in `manipulate.rs`.
- Deformed objects: pick against the **evaluated** (deformed) mesh, store indices of the
  base mesh (deformers don't change topology).

## Done when
- [ ] Spec `docs/specs/EDITABLE.md` §2 (modes, storage, interaction, display; own design,
      parity-pending markers), CORE.md §1 updated for `IndexList`.
- [ ] Tests: core param validation for `IndexList`; `crates/ui/tests/components.rs`
      (pick each element type on a cube with a known camera; click/shift/ctrl rules;
      rectangle select; undo restores the previous selection; selection survives
      save/load; selecting on a non-polygon object picks objects instead).
- [ ] `sh scripts/gate.sh` prints `GATE: all passed`.
- [ ] `docs/parity/STATUS.md` row "Component modes + selection".
- [ ] Committed on `batch-08` with the trailer below; report written.

Commit trailer: `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`

## Out of scope
Moving components (06), loop/ring selection and soft selection (batch 11), selection
tags as separate tag nodes, keyboard shortcuts for the modes (10).

## Report (implementer fills in)
- Result: no golden data exists for modes or selection, so nothing is "N/M exact"; everything is
  own design and **parity-pending** (EDITABLE.md section 2). 17 UI tests in
  `crates/ui/tests/components.rs` (pick point, edge and polygon on a made-editable cube from the
  editor's default camera; click, Shift, Ctrl rules on a pure set and on the stored set; rectangle
  select with all three modifiers; one undo step per rectangle; undo and redo; save/load through
  `core::file` and through `Editor::save_to`/`open_path`; a version 2 file without the params;
  object picking when there is no polygon target, in Model mode and for a non-polygon active
  object; picking on a deformed mesh stores base indices; select all/none/invert/grow/shrink for
  all three element types and in Model mode; header buttons, Ctrl/Cmd+A, Ctrl/Cmd+Shift+A and the
  Edit and Select menus through egui_kittest; `display_selection`, `component_shading`).
  Plus 2 core param tests, 2 core file tests and 1 GPU test (sized points).
- Rules derived: none from observation. Own design: Ctrl/Cmd beats Shift when both are held;
  point radius 8 px, edge radius 6 px; rectangle select uses `Mode::All` and `visible_only`
  (edges and polygons need all their points inside and visible); the three sets are independent
  (switching mode converts nothing); grow/shrink of edges = edges sharing a point.
- Version bump: `file::VERSION` 2 -> 3. Old files load without it (missing params fill in), but
  an older build reading a version 2 file that contains `IndexList` would fail with a RON parse
  error instead of `UnsupportedVersion`; the bump gives the clean error. One existing core test
  used version 3 as its "unsupported version" example and now uses 4.
- Deviations / choices the brief did not spell out:
  1. Renderer change (not listed in the brief): points were 1 px, unusable for editing. Added
     `Settings.point_size` (0 = old 1 px behaviour, so existing renders are unchanged), a point-quad
     pipeline (`vs_point`), `MeshBuffers::with_device_no_points`, and a `point` uniform. VIEWPORT.md
     section 6 updated.
  2. In a component mode a shading without wire is shown as its wire twin (Smooth as Smooth + wire,
     Flat as Flat + wire) so edges and outlines are visible; the stored `shading` is unchanged.
  3. The component target is drawn without the object tint, and `Palette.selection` is set to the
     new `Theme.component_selection` (#38C6E8) for the frame, so a selected object (orange) and
     selected components (cyan) differ.
  4. Model mode: Select All / Deselect All / Invert act on the object selection (all hierarchy nodes,
     no tags); Grow/Shrink do nothing there. In a component mode without a polygon target they return
     the error "select a polygon object" (status bar).
  5. `Command::SetMode` and `Command::Select(SelectCmd)` were added so menus, shortcuts and tests share
     one path. `forge-modifiers` is a new dev-dependency of forge-ui (deformed-pick test only).
  6. The object gizmo of the active object is still drawn and usable in component modes (moving
     components is brief 06). A primary drag that starts on a gizmo handle moves the object; any
     other primary drag is a rectangle select.
  7. Open resets the mode to Model; New does too (fresh state).
  8. Ctrl/Cmd+A and Ctrl/Cmd+Shift+A did not clash with any existing shortcut (nothing else used A).
     Like the others they are ignored while a text field has focus.
- Files changed: Cargo.lock; crates/core/{src/{param,file}.rs,tests/{params,file}.rs};
  crates/generators/src/{lib,polygon}.rs; crates/viewport/{src/render.rs,src/render.wgsl,tests/render.rs};
  crates/ui/{Cargo.toml,src/{app,app/files,attributes,commands,components,lib,state,theme,viewport_panel}.rs,
  tests/components.rs}; docs/specs/{EDITABLE,CORE,UI,VIEWPORT}.md; docs/parity/STATUS.md; this brief.
- Not verified without a display (only the pure logic, the egui widgets through kittest and the
  renderer through offscreen lavapipe renders were tested): the live rectangle drag (the rectangle
  outline, `drag_started`/`drag_stopped` and whether egui reports a pointer position on the release
  frame; the code falls back to the last stored corner), the feel of the 8 px / 6 px pick radii, the
  look of 7 px points and the cyan colour, and Ctrl+click on macOS (where egui may report it as a
  secondary click; `modifiers.command` is Cmd there).
- Open questions for the planner (data that would settle them): the reference's click/Shift/Ctrl
  rules, whether switching modes converts the selection, rectangle select defaults (visible only,
  all-or-any points for edges and polygons), grow/shrink for edges, and the highlight colours.
- Gate output: `GATE: 343 tests passed, 0 failed` / `cleanroom: ok` / `ok: 202 cases` / `GATE: all passed`
