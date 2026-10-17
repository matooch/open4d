# Object transforms (`forge-core`: `transform`, registry, scene)

Sources: parity observations [docs/parity/transform.md](../parity/transform.md) and
[docs/parity/export-settings.md](../parity/export-settings.md) (reference app R20.059,
golden data in `tests/golden/transform/` and `tests/golden/calibration/`). Matrix
helpers: glam 0.34 (`glam::DMat4`).

## 1. Conventions (observed)

- **Coordinates are the reference app's own numbers.** Golden files are exported
  with the exporter's "Flip Z Axis" **off** (its default is on), so they hold the
  app's internal coordinates; the calibration cube typed at (10, 20, 30) exports
  centred at (10, 20, 30). Forge3D stores the same numbers: a parameter value equals
  the value typed in the app. +Y is up.
- **Units:** lengths are the numbers shown in the app's fields (project unit
  centimetres, scale 1); no conversion. Angles are shown in degrees and stored in
  radians (`Unit::Angle`; the UI converts, UI.md §4).
- **Rotation fields** `(h, p, b)` = the app's 1st/2nd/3rd rotation field (labels
  R.H, R.P, R.B): `h` turns about Y, `p` about X, `b` about Z. Read with the usual
  right-handed matrices, each is a rotation by the **negated** angle — the app's world
  is left-handed:
  - `rot-1-30`: vertex (100, −100, −100) → (136.6025, −100, −36.6025) = `Ry(−30°)`
  - `rot-2-30`: (−100, −100, −100) → (−100, −136.6025, −36.6025) = `Rx(−30°)`
  - `rot-3-30`: (−100, −100, −100) → (−136.6025, −36.6025, −100) = `Rz(−30°)`
- **Rotation order** "HPB" (the default; the only one observed): `b` first, then `p`,
  then `h` — `rot-combined` vertex 1 (−100, −100, −100) → (−70.005, −44.829, −151.953).
  The other offered orders (XYZ, XZY, YXZ, YZX, ZXY, ZYX) are **not supported** until
  observed.
- **Composition:** scale, then rotation, then translation (`scale`, `full`); a child's
  world transform is its parent's world transform times its own local one (`child`:
  parent at (0, 50, 0) with h = 90°, child at local (100, 0, 0) → world centre
  (0, 50, 100)).
- Export bakes world transforms into vertices; the parity test therefore compares
  world-space points.

Not covered yet (needs observation): which way the reference viewport's default
camera looks, so the viewport can show the left-handed world un-mirrored
(VIEWPORT.md keeps its right-handed display until then); object-axis vs world-axis
gizmos; "Freeze Transformation"; quaternion mode; coordinate-manager Size/Size+ modes.

## 2. Transform parameters (registry)

Every type in a hierarchy category (`Category::is_hierarchy`) gets three parameters
**prepended** to its own at registration, in group `"Coordinates"`:

| id | label | kind | default | unit |
|---|---|---|---|---|
| `transform.pos` | Position | `Vec3`, unbounded | (0, 0, 0) | Distance |
| `transform.rot` | Rotation | `Vec3`, unbounded, radians `(h, p, b)` | (0, 0, 0) | Angle |
| `transform.scale` | Scale | `Vec3`, unbounded | (1, 1, 1) | None |

- A plugin type declaring any param id starting with `transform.` is rejected with
  `RegistryError::ReservedParam`.
- Non-hierarchy types (tags, materials, …) get none.

## 3. Math (`forge_core::transform`)

`Transform { pos: DVec3, rot: DVec3, scale: DVec3 }`, `Transform::IDENTITY`.

- `matrix() = T(pos) · Ry(−h) · Rx(−p) · Rz(−b) · S(scale)` (right-handed glam
  matrices, column vectors).
- `Transform::from_params(&BTreeMap<String, Value>)`: reads the three `Vec3` values;
  a missing or non-`Vec3` value falls back to the identity component (so unknown-type
  nodes with transform params still place correctly).
- `Scene::local_transform(id) -> Option<Transform>`: `None` for missing nodes and for
  tags.
- `Scene::world_matrix(id) -> Option<DMat4>`: product of `local` matrices from the
  root down to `id`.

## 4. Parity tests

`crates/core/tests/transform_parity.rs` builds, for every case under
`tests/golden/transform/` and `tests/golden/calibration/`, a scene with a node of
the case's transform (and, if the case has `parent_*` params, a `forge.null` parent
with those), transforms the vertices of `tests/golden/transform/identity/mesh.obj`
(the default cube in local space) by `world_matrix`, and compares against the case's
`mesh.obj` with the harness (`compare_meshes`, tolerance from the case, default
1e-4 — the exports are f32).

Case key mapping: `pos_k` → `transform.pos[k−1]`, `rot_k` (degrees) →
`transform.rot[k−1]` (radians), `scale_k` → `transform.scale[k−1]`; `parent_` prefix
→ the parent node.
