# 05 · Point, edge and polygon modes with selection

Status: ready (after 03) · Branch: `batch-08` · Planner: main session · Implementer: Claude Sonnet 5.5 (local, macOS)

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
