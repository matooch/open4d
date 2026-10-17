# Smoothing tag and viewport handedness

Sources: parity observations [docs/parity/smoothing.md](../parity/smoothing.md)
(golden `tests/golden/smoothing/`) and [docs/parity/viewport.md](../parity/viewport.md)
(reference app R20.059). Math: docs/specs/GEOMETRY.md §3 (`corner_normals`),
docs/specs/VIEWPORT.md §1 (camera).

## 1. Observed behaviour

- Smooth vs faceted shading is controlled by a **tag** on the object, present on
  every new primitive, with an angle-limit switch (default on), a smoothing angle
  (default **40°**, range 0–180°, degrees) and an edge-break switch (default on).
- Exported corner normals (`f v/vt/vn`) follow the pairwise rule of
  `corner_normals(mesh, angle, …)`: a default cube is faceted at 40° and 89°
  (6 face normals) and fully smoothed at 91° (corner normals `(±1, ±1, ±1)/√3`); a
  default cylinder smooths its 36 sides (10° apart) but keeps its caps separate (90°);
  a 3-sided cone keeps each side face's own normal (120° apart).
- These cases are symmetric, so they do not distinguish the normal **weighting**;
  `Weighting::Angle` stays the parity-pending default.
- Not observed: the angle limit switched off, edge breaks, a node without the tag,
  exactly 90°.

## 2. The smoothing tag (`forge.smoothing`, core built-in, category Tag)

| id | label | kind | default |
|---|---|---|---|
| `angle_limit` | Angle limit | Bool | true |
| `angle` | Smoothing angle | Float, Angle (radians), range 0–π | 40° |
| `edge_breaks` | Edge breaks | Bool | true (no effect until editable meshes can mark edges) |

`forge_core::smoothing::smoothing_angle(scene, id) -> Option<f64>`: the first
`forge.smoothing` tag on the node decides — `angle_limit` on → `angle`, off → π (fully
smooth); no such tag → `None` (faceted). *(Faceted-without-tag and "off = fully
smooth" are our assumptions until observed.)*

Creation: the editor's Create command attaches one smoothing tag to every new
**Generator** node, in the same undo step (UI.md §5), matching the observed
defaults.

## 3. Rendering

`MeshBuffers::with_device(device, mesh, selection, smoothing: Option<f64>)`: the
"smooth" normals are `corner_normals(mesh, angle, Angle)` for `Some(angle)` and the
flat polygon normals for `None`. `Shading::Flat` still forces flat normals.

## 4. Viewport handedness (observed)

docs/parity/viewport.md: the reference **Front** view shows +X to the right and +Y up
while looking along the Z axis; the default perspective view shows +X to the right
and nearer, +Y up, +Z up-right and **farther**, seen from above. With the reference
app's own coordinates (TRANSFORM.md) that is a left-handed display: a right-handed
view of the same numbers would show the scene mirrored.

`Camera::handedness: Handedness::{Right, Left}` (default `Right`; the editor uses
`Left`). The camera's state (`target`, `yaw`, `pitch`) lives in a right-handed
**display space**; world → display is `M = diag(1, 1, −1)` for `Left`, identity for
`Right`.
- `view() = look_at(eye, target, +Y) · M`; so `project` takes and `ray` returns
  **world** coordinates.
- World-space vectors: `target_world()`, `eye_world()`, `forward_world()`,
  `right_world()`, `up_world()` (= `M ·` the display vector). Picking occlusion,
  gizmo sizing/drags, the grid centre and the renderer's headlight use these.
- `frame(min, max)` takes a world box. `look_along(view)` names **world** sides
  (`Left` + `PosZ` places the eye on world +Z).
- Editor default camera: `Left`, yaw 0.68, pitch 0.52, distance framing a 400-unit
  neighbourhood — matches the observed screen directions; the exact default pose is
  "not shown" by the reference app (**parity-pending**).

## 5. Parity tests

- `crates/geometry/tests/smoothing_parity.rs`: for every `tests/golden/smoothing/`
  case, take the exported mesh (positions, faces) and its exported corner normals;
  `corner_normals(mesh, smoothing_angle or 40°, Angle)` must match every corner within
  1e-5 per component (exports are f32).
- `crates/viewport/tests/camera.rs`: a `Left` camera looking along world +Z (Front-like)
  shows +X right and +Y up; the default editor pose shows +X right-and-nearer, +Z
  right-up-and-farther, +Y up; project/ray stay inverse; pan still follows the cursor.
