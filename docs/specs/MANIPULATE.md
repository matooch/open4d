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

## 6. Snapping 2.0 (brief 032; own design, **parity-pending** GUI session G3)

Sources: Ericson (2005) §5.1 (closest point on a segment and a triangle), §5.3 (ray/plane,
ray/triangle); docs/specs/VIEWPORT.md §3 (the grid snap and `snap_scalar` stay as they are).
The reference app's snapping dialog has not been observed; the target list, the priority order,
the quantize step names and the 2D/3D meaning below are our own and are tied to the roadmap line
"Snapping 2.0: points, edges, polygons, axes, grid, workplane; quantize" (areas/B-modeling.md).

### 6.1 Settings (`forge_modeling::snap::SnapSettings`; editor state, not saved, not undoable)

| key | kind | default | range / values |
|---|---|---|---|
| `enabled` | bool | false | snapping to targets on/off |
| `mode` | enum | `3d` | `3d`, `2d` |
| `radius` | float (px) | 10 | 1 .. 100 |
| `visible_only` | bool | true | ignore point/edge targets hidden behind a polygon (screen snapping only) |
| `targets.vertex` | bool | true | |
| `targets.edge` | bool | true | nearest point on an edge |
| `targets.edge_midpoint` | bool | true | |
| `targets.polygon_center` | bool | true | mean of the polygon's points |
| `targets.polygon` | bool | false | point on the polygon surface under the cursor |
| `targets.grid_point` | bool | false | nearest grid point of the workplane |
| `targets.grid_line` | bool | false | nearest grid line of the workplane |
| `targets.workplane` | bool | false | the cursor ray's hit on the workplane |
| `targets.object_axis` | bool | false | the origin ("axis") of every object in view |
| `grid_spacing` | float | 0 | `0` = automatic (the visible grid, VIEWPORT.md §4; 100 headless), else units; 0 .. 1e9 |
| `workplane.origin` | vec3 | `[0,0,0]` | |
| `workplane.normal` | vec3 | `[0,1,0]` | non-zero; stored normalised |
| `quantize.enabled` | bool | false | |
| `quantize.move` | float | 10 | units, > 0 (axis quantize: translation along the dragged axis or plane axes) |
| `quantize.angle` | float | 15 | degrees, > 0 (rotation) |
| `quantize.percent` | float | 10 | percent, > 0 (scale factor steps of `percent / 100`) |

`SnapSettings::apply_json(obj)` sets the present keys (`targets`, `workplane`, `quantize` are
objects of partial keys), validates, and changes nothing on an error; `to_json()` returns every
key. Unknown keys are errors.

### 6.2 Targets and priority

A **candidate** is `{kind, position, distance, depth, source, index}`. Kinds and their rank
(lower wins): `Vertex`, `EdgeMidpoint`, `PolygonCenter`, `ObjectAxis` rank 0 (points);
`Edge` rank 1; `Polygon` rank 2; `GridPoint` rank 3; `GridLine` rank 4; `Workplane` rank 5.
The winner minimises `(rank, distance, depth, kind order, source, index)` (kind order is the list
order above; `total_cmp` for floats). So any point inside the radius beats an edge or a polygon
under the cursor ("point > edge > face"), and nearer wins among equal ranks. Only candidates
with `distance <= radius` take part. Only enabled targets generate candidates.

`SnapSource { mesh (world space), origin: Option<DVec3>, skip_points: sorted Vec<u32> }`: the
evaluated meshes in view. A point listed in `skip_points` generates no vertex candidate and any
edge or polygon that uses it generates none either (the points being moved cannot snap to
themselves; moved objects are not sources at all). Sources keep scene order.

### 6.3 Screen snapping (`forge_viewport::snap::snap_cursor`)

`snap_cursor(sources, cam, x, y, w, h, settings) -> Option<SnapHit>` with the cursor in pixels.
`distance` is the screen distance in pixels to the projection of the candidate, `depth` the
projected NDC depth.
- Vertex, midpoint, centre, axis: the point itself. Points behind the camera give none.
- Edge: the point of the edge closest to the cursor ray (`closest_on_edge`), measured after
  projection.
- Polygon: nearest ray hit over all triangles of the polygons without skipped points
  (`distance` 0).
- Grid point and grid line: the cursor ray is intersected with the workplane (no hit: none);
  in workplane coordinates `(u, v)` (basis in §6.5) the grid point is `(round(u/g)·g, round(v/g)·g)`;
  the grid line rounds the coordinate with the smaller offset (the other keeps the hit's value);
  both are measured after projection. `g` = `grid_spacing`, or the visible grid's spacing.
- Workplane: the hit itself (`distance` 0).
- `visible_only`: a vertex, midpoint, centre or edge candidate with a polygon of any source
  between it and the camera is dropped (same test as `pick_point`).
- `mode = 2d`: the hit's `position` is projected onto the workplane (`p − n·((p − o)·n)`); `raw`
  keeps the 3D position. `mode = 3d`: `position == raw`.

`SnapHit { kind, position, raw, source: Option<usize>, index: Option<u32>, distance }`; `index` is the
point id for `Vertex`, the edge id for `Edge` and `EdgeMidpoint`, the polygon id for `Polygon` and
`PolygonCenter`.

### 6.4 World-space query (`forge_modeling::snap::snap_point`, command `snap.query`)

`snap_point(sources, p, radius, settings) -> Option<SnapHit>`: the same candidates and the same
order with `distance` = 3D distance to `p` (world units, `radius` in world units): points as
they are; edge = closest point on the segment; polygon = closest point on its triangles; grid point,
grid line and workplane relative to the projection of `p` on the workplane (distance = 3D distance
from `p`). No occlusion. `mode = 2d` projects the result onto the workplane. The headless
equivalent of what the editor does with the cursor; agents use it to find where a point would land.

### 6.5 Workplane and grid

Basis of workplane `(o, n)`: `u = normalize(Y × n)` (`X` when `|n·Y| > 0.999`), `v = u × n`.
Coordinates `(u, v) = ((p−o)·u, (p−o)·v)`. For the default plane `u = X`, `v = Z`.
`grid_point(p, o, n, g)` and `grid_line(p, o, n, g)` give the snapped world positions of the
projection of `p`. `g <= 0` is an error for these helpers (callers resolve the automatic value).

### 6.6 Quantize

`quantize_steps(settings, ctrl) -> Option<Steps { move, angle (rad), scale }>`: active when
`quantize.enabled` xor the Ctrl/Cmd key is held. The steps replace the old hard-coded
10 units / 15° / 0.1 (the defaults equal them). They feed `Drag::update(..., snap)` unchanged:
translate `move` (per axis; plane drags per plane axis, centre drags per world component), rotate
`angle`, scale `percent / 100`.

### 6.7 Where snapping applies (`forge_ui::snapping`)

- **Gizmo translate drags** (objects, and in axis mode §7.4): with `enabled`, each pointer move
  calls `snap_cursor` over the sources collected when the drag began (every drawn mesh except the
  dragged object and its descendants, plus its object axis if enabled). A hit moves the gizmo origin to
  `hit.position` constrained by the handle: axis handle: the component of `hit − origin` along the
  axis; plane handle: `hit − origin` minus its normal component; centre handle: the whole
  difference (`Drag::translate_to(target) -> DragDelta`). No hit: quantize if active, else the
  free drag. A target hit wins over quantize. Rotate and scale drags only quantize.
- **Component transforms**: the same, with the sources of all drawn objects; the moved object's
  selected points go to `skip_points` (its other points are targets, so points can be snapped onto each other).
- **Polygon Pen** (MODELING.md §6.7): with `enabled` the corner under the cursor is the snap hit: a
  `Vertex` hit of the target object is the `Point` corner (exactly the old behaviour); any other
  hit becomes a `New` corner at the hit position in object space; no hit falls back to the old
  rule (point within the pick radius, else the plane through the camera target).
- **Knife Line / Plane**: with `enabled` the dragged end points move to the pixel of the hit's
  projected position (the cut is computed from that pixel as before).
- **Loop/Path Cut**: with `enabled` and `edge_midpoint` the offset snaps to `0.5` when the cursor
  is within `radius` pixels of the projected edge midpoint.
- A small marker with the kind label is drawn at the current hit while a drag or tool hover is
  active.

### 6.8 Commands (headless; COMMANDS.md §6)

- `snap.set` `{enabled?, mode?, radius?, visible_only?, grid_spacing?, targets?, workplane?, quantize?}`
  merges settings and returns the full settings; no arguments reads them. Not undoable.
- `snap.toggle` `{enabled?}`: flips `enabled` (or sets it); returns `{enabled}`. Key `Shift+S` (`X` is the Pan hold key, UI.md §6).
- `snap.reset`: defaults. Not undoable.
- `snap.query` `{point, radius?, nodes?}`: sources = the drawn meshes (world space) of `nodes`
  (default: the whole scene; each source's origin is the node's world position); `radius` in
  world units, default 10. Returns `{hit: null}` or `{hit: {kind, position, raw, node, index,
  distance}}`. Uses the current settings with `enabled` ignored. Not undoable.

## 7. Axis (pivot) editing (brief 032; own design, **parity-pending** G3)

An object's **axis** is its transform (position, rotation, scale; TRANSFORM.md). Axis editing
changes that transform while the content stays where it is in the world.

### 7.1 Math (`forge_generators::axis`)

With `O` = the old local matrix, `N` = the new local matrix and `K = N⁻¹·O`:
- a **polygon object** (`forge.polygon`) gets every point `p ← K·p` (stored mesh; selections
  and indices are untouched), so `W·N·K·p = W·O·p` for the parent world matrix `W`;
- every **direct child** with transform params gets its local matrix `C ← K·C`, decomposed back
  into `pos`, `rot` (HPB) and `scale` (`decompose`: position = translation column, scale =
  column lengths, negated on x when the determinant is negative, rotation from the normalised
  columns). Rotation and scale params are rewritten only when they change by more than 1e-12.
  This is exact unless the edit mixes a rotation with a non-uniform scale (shear cannot be
  stored in a TRS child; the child is then approximated and the world position of its
  origin is still exact);
- **other objects** (generators, nulls, cameras, lights): their own geometry or placement
  follows the axis (it is parametric and cannot be counter-transformed); only the children stay.
  Use Make Editable first to keep a generator's points.
World-space points and child matrices agree with the old ones to 1e-9 (tests use 1e-9;
translation-only edits agree to ~1e-13).
Stored params are written; animation keys are not (autokey is ignored by axis edits).
Every command below is one transaction, so one undo step, however many nodes and children it touches.

### 7.2 Commands

| command | arguments | what it does |
|---|---|---|
| `object.set_axis` | `node?`, `pos?`, `rot?` (degrees HPB), `scale?`, `space?` (`local` default, `world`) | sets the given components of the axis like `transform.set` and counter-transforms the content. `space = world`: `pos` is a world point, `rot` a world HPB rotation (`R' = R_P⁻¹·R(rot)`); `scale` is always the object's own scale. |
| `object.center_axis` | `nodes?`, `to?` (`bounding_box` default, `points`, `selection`, `children`) | moves the axis to the centre without moving the content. `bounding_box`, `points` (average) and `selection` (average of the points of the stored point/edge/polygon selection) need a polygon object; `children` (average of the direct children's positions) works for any object. Position moves by `O·c`; rotation and scale are kept. |
| `object.align_axis` | `nodes?`, `to` (`world`, `parent`, `normal`) | rotation only. `world`: the world rotation becomes identity (`rot = hpb(R_P⁻¹)`); `parent`: `rot = 0`; `normal`: the average (area weighted) normal `n` of the selected polygons (stored polygon selection; else the polygons that use a selected point or edge) becomes the local Z axis: basis `Z = n`, `X` = the first selected polygon's first edge direction made orthogonal to `n` (`Y × n` when degenerate), `Y = Z × X`; new rotation `R·B`. Polygon objects only. |

`nodes` default to the selection; a missing node, a tag, no polygon selection for `normal`, or
a non-polygon object for the polygon-only options is `Failed` and changes nothing (the whole
command is one transaction, rolled back as a whole). Results: `{nodes: [...]}`; `center_axis` also
`center` per node (object space) as `{centers: {id: [x,y,z]}}`.
`mesh.center_axis` (MODELING.md §4.11) stays as it is: the same operation for a polygon object, as a mesh tool.

### 7.3 Pure helpers

`counter_matrix(old, new) -> DMat4` (`K`), `decompose(m) -> Transform`, `normal_basis(n, hint) -> DMat3`,
`AxisEdit::capture(scene, node)` (start transform, mesh, children local matrices) and
`AxisEdit::apply(tx, &new)`; a drag recomputes from the captured start each frame, so repeated
updates never accumulate error (as `apply_drag`).

### 7.4 Axis mode (editor)

`edit.axis_mode` `{enabled?}` toggles the editor's axis mode (`TransformSettings.axis_mode`,
key `L`, Edit menu "Axis mode"); returns `{enabled}`; not undoable. While on, the gizmo sits at the
active object's origin (not on component pivots), and gizmo drags (translate, rotate, scale, with
snapping and quantize of §6) edit the axis with `AxisEdit` instead of the object: one drag is one
undo step (`Move axis`, `Rotate axis`, `Scale axis`). Component drags are off; the sources of
snapping include the object's own points and axis (they do not move in the world while the axis
is dragged). The viewport label shows "Axis mode".
