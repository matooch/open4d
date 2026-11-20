# UV — corner UVs and primitive UV layouts (brief 128); projections (brief 127)

Sources: O28 [docs/parity/uv-projection.md](../parity/uv-projection.md) and `tests/golden/uv/`
(10 primitive cases, Part A; 7 projection cases, Part B); the OBJ `vt` / `f v/vt/vn` records of
those files. Nothing is read from the reference app.

## U1 Corner UVs

`forge_geometry::CornerUvs { corners: Vec<Vec<[f64; 2]>> }` is parallel to `PolyMesh::polys`
(one `(u, v)` per polygon corner). **Deviation from the area design:** the design puts a UV
attribute inside `PolyMesh`; that struct is built by literal in 200+ places across crates owned
by other tracks, so the UVs live beside the mesh (`CornerUvs`) and travel with it explicitly.
`CornerUvs::indexed(tol)` gives the OBJ view (deduplicated table in first-use order plus a table
index per corner). OBJ reading and writing of `vt` is **not** done in this brief (the harness
parses `vt` for the test; `forge-io` is untouched): see U5.

## U2 Primitive layouts (`forge_generators::uv::primitive_uvs`)

`primitive_uvs(type_id, values, &mesh) -> Option<CornerUvs>`; `build_with_uvs` returns both.
`None` = no layout derived (U5). All layouts are for Orientation +Y, no slice, no fillet.
Every case below is a golden of `tests/golden/uv`; faces are matched per corner within 1e-5 and the
deduplicated table size (positions / texture coordinates in the table) must equal O28's count.

- **Cell pattern.** A quad of a swept or gridded surface lists its corners as
  `(u_i, v_j), (u_i, v_j+1), (u_i+1, v_j+1), (u_i+1, v_j)`. Used by plane, cube, cylinder / cone /
  tube walls, torus, sphere and capsule.
- **Plane** (`plane`): `u = (x - x_min) / width`, `v = (z - z_min) / height` per vertex (+Y).
- **Cube** (`cube`, `cube-segments`): per face, `e_u = p3 - p0`, `e_v = p1 - p0` of the first
  polygon of the face (faces group by their `(e_u, e_v)` pair); `u`, `v` = projections on `e_u`,
  `e_v` normalised by the face's extent. Every face spans 0..1, so the six faces share one 0..1
  layout (4 / 32 texture coordinates).
- **Disc** (`disc`): planar, `u = 0.5 + x / (2 R)`, `v = 0.5 + z / (2 R)`, `R` = outer radius.
- **Revolved prims** (sphere, cylinder, cone, tube, capsule, torus): `N` rotation segments, sector
  `i = floor(atan2(z, x) / (2 pi / N))` of the face centroid; a quad's corners get
  `u = [i, i, i+1, i+1] / N` (so the seam's last sector ends at 1.0, never 0). `v` per vertex:
  - Cylinder, cone, tube outer wall: `v = (y - y_min) / (y_max - y_min)` (`cylinder`, `cone`, `tube`);
    the cone's degenerate top ring still gets `v = 1`.
  - Tube inner wall (all corners at the inner radius): `v = (y_max - y) / (y_max - y_min)` (flipped).
  - Sphere (Standard): `v = atan2(rho, -y) / pi` (equal to ring / rings); pole triangles take the
    pole's `u` from the lower of their other two `u` (`sphere`: pole `0.75` in sector 12).
  - Capsule: `v` = cumulative chord length of the meridian polyline from the south pole over its
    total (cap arcs are `cap_segments` chords, the middle `height_segments` equal steps; e.g. 78.03 /
    256.07 = 0.3047 at the cap's end); pole triangle `u` = sector centre `(i + 0.5) / N`
    (`capsule`).
  - Torus: cell based, `u = [i, i, i+1, i+1] / ring_segments`, `v = [j, j+1, j+1, j] / pipe_segments`
    with `j` from the pipe angle of the face centroid (`atan2(y, rho - R)`), so the last cell ends
    at 1.0 (`torus`).
- **Caps** of cylinder, cone, tube (planar, all corners at `y_min` or `y_max`): `v = 0.5 + z / (2 R)`,
  `u = 0.5 + x / (2 R)` on the top cap and `u = 0.5 - x / (2 R)` on the bottom cap (mirrored), `R` =
  the largest radius of the mesh (cone bottom radius, tube outer radius).

## U3 Counts of the golden table

Each golden's texture-coordinate count equals ours after `indexed(1e-6)`: cube 4, cube-segments **29 vs the reference's 32 (every corner value matches; the reference table repeats some values that different faces share, so the table sizes differ; pinned, not a layout difference)**,
sphere 151, cylinder 97, cone 97, torus 561, plane 121, disc 18, capsule 219 used (the reference lists 220: one table entry no face uses), tube 109.

## U5 Unsupported / parity-pending

- No layout (returns `None`): other orientations, slices, fillets, `separate_surfaces`, sphere types
  other than Standard, hemispheres, regular-grid torus / capsule, platonic solids, pyramid, oil tank,
  figure, landscape, polygon prim (not observed).
- Parity-pending: cone with a top radius > 0 (cap radius), tube cap annulus for a non-default inner
  radius, cube with separate surfaces.
- No OBJ `vt` reader or writer in `forge-io` (follow-up); the O28 files are parsed in the tests.

## U4 Projections and Generate UVW (brief 127)

Sources: O28 Part B (`proj-*`, 7 cases: a default Sphere made editable, 114 points / 128 polygons,
projection set on the texture tag, then Generate UVW Coordinates). `forge_material::projection`:
`generate_uvw(mesh, &TagSpace, Projection, object_to_world) -> Result<CornerUvs, ProjectionError>`.

- **Tag space.** `TagSpace { position, rotation (heading, pitch, bank, radians), size }` with the tag's
  params `position`, `rotation`, `size` (default 100: O28 `size`). Points go object -> tag space by
  `R^-1 (p - position)` (identity at the defaults; only the defaults and the Size were observed).
  Coordinates scale by `1 / size`: `proj-cubic-scaled` (Size 50) is exactly twice `proj-cubic`.
- **Flat** (`proj-flat`): `u = 0.5 + x / (2 sx)`, `v = 0.5 + y / (2 sy)`.
- **Cylindrical** (`proj-cylindrical`): `u = atan2(z, x) / 2 pi` (mod 1), `v = 0.5 + y / (2 sy)`.
- **Spherical** (`proj-spherical`): `u` as cylindrical, `v = atan2(rho, -y) / pi`. Seam rule as the primitive sphere (U2): per polygon, if the corner `u` span exceeds 0.5 the corners
  below 0.5 get `+1`; a corner on the axis takes the `u` of the previous corner of the polygon (bottom pole fans take the
  larger of the other two, top fans the smaller; unlike the primitive sphere, which takes the lower one). Cylindrical and
  spherical coincide with the sphere's own layout where the sphere's rings are equally spaced.
- **Cubic** (`proj-cubic`, `proj-cubic-scaled`): per polygon, the axis of the largest Newell-normal
  component (ties: x, then y, then z) and its sign pick the face; `(u, v) = 0.5 + (a p_i, b p_j) /
  (2 size)` with: normal -x `(-z, y)`, +x `(z, y)`, -y `(x, -z)`, +y `(x, z)`, -z `(x, y)`,
  +z `(-x, y)`; zero error on all 128 polygons of the sphere.
- **Offset, length, tiles** are not applied by Generate UVW: `proj-cylindrical-offset` (Offset U 25 %)
  equals `proj-cylindrical` value for value. They stay on the tag for render-time mapping.
  Length / tiles / repetitions: **parity-pending** (assumed the same).
- **Frontal** (`proj-frontal`): a pinhole projection through the document's scene camera
  (`Camera { position, heading, pitch, bank, aspect }`): `forward = (-sin H cos P, sin P, cos H cos P)`,
  `right = normalize(up x forward)`, `u = 0.5 + x_c / z_c`, `v = 0.5 + (16 / 9) y_c / z_c`. A free
  fit of all 114 vertices gives that form (residual 3e-6) with `ku = 0.99905`, heading 44.985 deg,
  bank 0.005 deg, so the recorded rotation (45, -19.45) and focal ratio do not reproduce the
  reference exactly. **Known mismatch**: with the recorded camera the largest corner error is
  3.6e-4 (tolerance 1e-5); pinned. The cause (rounded recorded angles or an unrecorded film setting)
  needs a re-observation with the camera's focal length, film size and exact rotation.
- **Unsupported** (`ProjectionError::Unsupported`): Spatial, UVW Mapping (identity, no data),
  Shrink Wrapping, Camera Mapping, a Frontal projection without a camera, (a non-default tag
  `rotation` or `position` is implemented but was not observed: **parity-pending**).
