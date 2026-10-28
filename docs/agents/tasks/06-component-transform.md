# 06 · Move, rotate and scale components

Status: ready (after 05) · Branch: `batch-08` · Planner: main session · Implementer: Claude Sonnet 5.5 (local, macOS)

## Goal
In Points, Edges or Polygons mode the gizmo sits on the selected components and the move,
rotate and scale tools change their points; one drag is one undo step; deformers and
generators above or below the object update live.

## Inputs
- Briefs 03 and 05.
- `docs/specs/MANIPULATE.md` (gizmo, axes, drag deltas, snapping),
  `crates/ui/src/manipulate.rs` (`apply_drag` for objects), `crates/viewport/src/gizmo.rs`
  (`DragDelta`), `crates/viewport/src/snap.rs`.

## Design decisions (planner)
1. **Pure core of the task** in `forge-generators` or a new `crates/ui/src/component_edit.rs`
   (say which in the report): `fn transform_points(mesh: &MeshData, points: &IndexSet,
   m: DMat4) -> MeshData` applies `m` (object-local) to the given points only. Edges and
   polygons map to their points (`edges_to_points`, `polys_to_points`).
2. **Pivot**: the centroid of the affected points (mean of positions, object-local). Axes
   follow the existing gizmo axis setting (world/object, MANIPULATE.md). For world axes,
   convert the world delta into object-local space with the inverse world matrix
   (non-uniform scale included).
3. **Drag**: like `apply_drag`, keep the mesh at drag start and set
   `mesh = transform_points(start_mesh, sel, delta)` on every frame inside one
   transaction per drag (whatever pattern `apply_drag` uses to make a drag one undo step:
   reuse it). Snapping and the coordinate panel's numeric entry: snapping yes (reuse
   `snap.rs`), numeric entry out of scope.
4. The Coordinates panel shows the pivot's position in component modes (read-only is
   fine for this brief).

## Hints (planner)
- Test the maths without a GPU: a default cube made editable, select the top polygon,
  move +50 on y → the four top points have y = 150, others unchanged; scale 2 about the
  centroid; rotate 90° about y — compare exact numbers.
- With a deformer child, the viewport must show the deformed result of the edited mesh:
  that already follows from `evaluate` + the cache keyed on `deep_revision`, but write a
  test that edits points under a Bend and evaluates.

## Done when
- [ ] Spec `docs/specs/EDITABLE.md` §3 (pivot, axes, drag, undo; parity-pending markers).
- [ ] Tests: unit tests for `transform_points` (move/rotate/scale, world-axes on a
      rotated and non-uniformly scaled object), a UI-level drag test (one undo step),
      deformer-after-edit test.
- [ ] `sh scripts/gate.sh` prints `GATE: all passed`.
- [ ] `docs/parity/STATUS.md` row "Component transform".
- [ ] Committed on `batch-08` with the trailer below; report written.

Commit trailer: `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`

## Out of scope
Modeling tools that change topology (batch 11), soft selection, numeric entry in the
Coordinates panel for components, axis/pivot editing.

## Report (implementer fills in)
