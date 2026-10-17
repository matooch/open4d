# Manipulation — rotate gizmo, object axes, coordinate panel

Sources: docs/specs/TRANSFORM.md (from docs/parity/transform.md: HPB convention,
composition, and the coordinates-manager modes observed there — "Object (Rel)",
"Object (Abs)", "World"; middle column "Scale" / "Size" / "Size+"; a default cube
shows Size 200 and, after scale (2, 3, 4), Size 400 / 600 / 800);
docs/specs/VIEWPORT.md §5 (translate/scale gizmo); docs/specs/SMOOTHING.md §4
(left-handed display). Ericson (2005) §5.3 for ray/plane. Gizmo look, sizes and key
bindings are our own (**parity-pending**).

## 1. Rotation ↔ HPB (`forge_core::transform`)

- `hpb_from_rotation(m: DMat3) -> DVec3`: the `(h, p, b)` with
  `Ry(−h)·Rx(−p)·Rz(−b) = m` for a pure rotation `m`; computed as
  `(a, b, c) = m.to_euler(EulerRot::YXZ)` (glam intrinsic Y·X·Z) and returning
  `(−a, −b, −c)`. At the pitch singularity (|p| = 90°) any valid split is returned.
- `Transform::rotation() -> DMat3` (the rotation part of `matrix()`).
- `Scene::parent_world_matrix(id) -> Option<DMat4>`: identity for roots, the
  parent's `world_matrix` otherwise; `None` for missing nodes and tags.
- `rotation_part(m: &DMat4) -> DMat3`: the upper 3×3 with each column normalized
  (exact for parents without non-uniform scale; documented approximation otherwise).

## 2. Gizmo frames and rotation (`forge_viewport::gizmo`)

- `Gizmo` gains `axes: [DVec3; 3]` (orthonormal; default world X, Y, Z) and
  `Axis::index()`. `Handle::Axis(a)` uses `axes[a]`; `Plane` handles use their two
  axes and the third as normal. All §5 translate/scale behaviour of VIEWPORT.md
  holds in this frame.
- `GizmoMode::Rotate`: one ring per axis — circle of radius `L` around the origin in
  the plane ⟂ `axes[a]`, sampled at 64 points. `hit_test` (rotate mode): the ring with
  the smallest screen distance ≤ 6 px to its projected polyline (ties X, Y, Z);
  `Center` is not offered. Rings seen edge-on stay pickable (they project to a line).
- Rotate drag: `begin` intersects the start ray with the ring's plane (through the
  origin, normal `axes[a]`); fails if parallel or the hit is within 1e-9 of the
  origin. `update`: same intersection for the current ray, angle from start vector
  `s` to current vector `c` = `atan2((s × c)·n, s·c)` with `n = axes[a]` (right-handed
  about `n` in world coordinates), snapped with the increment if given (radians);
  result `DragDelta::Rotate { axis: n, angle }`. A parallel ray returns the last delta.
- Scale drags always report factors **along the gizmo axes**.
- `Space::{Object, World}`: the editor builds the gizmo frame from the active node's
  world rotation (`Object`, columns of `rotation_part(world)`) or world axes (`World`).
  Scale mode always uses `Object`.

## 3. Applying a drag (`forge_ui::manipulate`)

`apply_drag(doc, id, start: &Transform, delta: DragDelta, gesture) -> Result<(), EditError>`
writes the node's transform params from the transform it had when the drag began
(`start`), so repeated updates never accumulate error. With `P` = parent world matrix
and `R_P = rotation_part(P)`:
- `Translate(w)` (world vector): `pos = start.pos + P⁻¹·w` (as a direction).
- `Rotate { axis, angle }` (world axis): `R' = R_P⁻¹ · R(axis, angle) · R_P · start.rotation()`,
  `rot = hpb_from_rotation(R')`.
- `Scale(f)` (factors along the object's own axes): `scale = start.scale · f`.
One continuous drag is one undo step (`Gesture::Drag`).

## 4. Coordinate panel (`forge_ui::coordinates`; new dock panel `Coordinates`)

Our layout: three rows (Position, Rotation, Scale-or-Size) × three fields, a space
switch `Local | World` (Local ≙ the observed "Object (Rel)") and a third-row switch
`Scale | Size`. Rotation fields are shown as H / P / B in degrees.
- `coord_view(scene, id, space, third) -> Option<CoordView { pos, rot, third: DVec3 }>`:
  `Local`: the node's params; `World`: translation and `hpb_from_rotation(rotation_part)`
  of `world_matrix`, and world scale = column lengths. `Size`: (local mesh bounding-box
  size of a generator node) × scale, or `None` in that row when the node has no mesh.
- `set_coord(doc, id, space, row, value: DVec3, gesture) -> Result` (`Gesture` as in UI.md §4:
  a field drag is one undo step): `Local` writes params
  directly. `World` position → `pos = P⁻¹ · value`; world rotation →
  `rot = hpb_from_rotation(R_P⁻¹ · R(value))`; world scale → `scale = value / parent
  scale` (per axis, parent scale = column lengths of `P`). `Size` → `scale = value /
  bbox size` per axis (axes with zero bbox size keep their scale).
- "Object (Abs)" and "Size+" are not implemented (behaviour not observed in detail).
- Default dock: the Coordinates panel joins the Bottom area after Timeline.

## 5. Editor viewport interaction (`forge_ui`)

- Picking: `pick_node(scene, cam, x, y, w, h) -> Option<NodeId>`: nearest polygon hit
  over all generator meshes in world space (VIEWPORT.md §2 `pick_poly`); ties → lower
  node id. Left click without Alt in empty space clears the selection; on an object
  selects it (Ctrl toggles, as in the tree).
- Gizmo: shown at the active node's world position when a node is selected; mode
  keys (ours) **W** translate, **E** rotate, **R** scale; **Q** toggles `Object`/`World`
  space (also in the View menu). Left-dragging a handle drives §3; snapping with Ctrl
  held (translate 10 units, rotate 15°, scale 0.1).
- The gizmo is drawn last, without depth test (VIEWPORT.md §6); rotate rings use the
  axis colours.
