# UV coordinates and projection — observation (O28)

Status: observed (17 of 17 cases) · App version: Cinema 4D 2025.1.0 (25.010) · OS: macOS 26.7.2 (25G332, arm64) · Observer: Hermes · Date: 2026-11-20
Method: scripted. The usual OBJ settings already export UVs (Original), so every golden
mesh carries `vt` lines.

Folder `tests/golden/uv/<case>/mesh.obj`, with `feature = "uv"` and
`observation = "docs/parity/uv-projection.md"`. `[params]` holds the object, its
parameters, and the projection settings.

Unblocks: P4 (UV tools, texture tags) and the UVs of the primitives we already build.

## Part A: the primitives' own UVs

These cases are the primitive with its default settings. They check that our UV layout
matches the reference. Counts are positions / texture coordinates / polygons in `mesh.obj`.

| Case | Object | Result |
|---|---|---|
| cube | Cube, defaults | 8 / 4 / 6 — the six faces share one 0–1 quad, so the four corners carry all the texture coordinates |
| cube-segments | Cube, segments 2/3/4 | 54 / 32 / 52 |
| sphere | Sphere, defaults | 114 / 151 / 128 |
| cylinder | Cylinder, defaults | 82 / 97 / 96 |
| cone | Cone, defaults | 81 / 97 / 80 |
| torus | Torus, defaults | 512 / 561 / 512 |
| plane | Plane, defaults | 121 / 121 / 100 |
| disc | Disc, defaults | 17 / 18 / 16 |
| capsule | Capsule, defaults | 178 / 220 / 192 |
| tube | Tube, defaults | 160 / 109 / 160 — fewer texture coordinates than positions, so the caps reuse the wall's coordinates |

## Part B: projection

Use a sphere made editable. Make a material through its menu command and apply it, which
adds a texture tag. Set the tag's projection, then run **Generate UVW Coordinates** (the
command that bakes a tag's projection into the UVW tag). Export.

Every case below starts from a default Sphere (114 points / 128 polygons), made editable,
with one default material ("Mat") in a texture tag. After Generate UVW Coordinates the tag
reports its projection as "UVW Mapping"; `case.toml` keeps both the projection that was set
(`projection_set`) and what the tag reads afterwards (`projection_after_generate`).

| Case | Projection | Result |
|---|---|---|
| proj-flat | Flat | 114 / 65 / 128; first texture coordinates 0.691342 0.038060, 0.853553 0.146447, 0.826641 0.146447 |
| proj-cubic | Cubic | 114 / 49 / 128; first 0.691342 0.500000, 0.853553 0.500000, 0.826641 0.364701 |
| proj-spherical | Spherical | 114 / 151 / 128; first 0.000000 0.125000, 0.000000 0.250000, 0.062500 0.250000 |
| proj-cylindrical | Cylindrical | 114 / 151 / 128; first 0.000000 0.038060, 0.000000 0.146447, 0.062500 0.146447 |
| proj-frontal | Frontal (record the camera) | 114 / 114 / 128; first 0.529613 0.310567, 0.556747 0.330300, 0.572426 0.349509. Camera: the document's scene camera "Camera" at (600, 300, -600), rotation (45, -19.45, 0) |
| proj-cubic-scaled | Cubic, tag scale (50, 50, 50) | 114 / 54 / 128; first 0.882683 0.500000, 1.207107 0.500000, 1.153281 0.229402 — the half-size tag doubles the coordinates and pushes them past 1 |
| proj-cylindrical-offset | Cylindrical, offset U 25 % | 114 / 151 / 128; every texture coordinate is identical to `proj-cylindrical` (the two files match value for value), so Offset U 25 % does not reach the generated coordinates in this build |

## Follow-up (planner, after brief 127)

| Case | Redo as | What was missing |
|---|---|---|
| proj-frontal | proj-frontal-2 | Record the camera exactly: position, rotation (H, P, B) read back, focal length, sensor/film size and render resolution, in `case.toml` |

## Notes

- The default OBJ export already writes the UV layer ("Original"): no extra setting was needed
  for any of the 17 cases, and every `mesh.obj` carries `vt` lines.
- The cube's own layout is a single shared 0–1 quad: 8 positions but only 4 texture
  coordinates, and all six faces read the same four corners. With segments 2/3/4 the counts go
  to 54 / 32 / 52, i.e. 32 coordinates for 54 positions, so the layout is still per-face-planar
  rather than per-vertex.
- Tube is the one primitive whose texture-coordinate count is lower than its position count
  (109 against 160) because the caps reuse the wall coordinates.
- Generate UVW Coordinates is reachable from a script and works on the active texture tag: the
  tag's projection reads "UVW Mapping" afterwards and the coordinates replace the sphere's own.
  Before the command the tag carries the projection that was set (`Flat`, `Cubic`, `Spherical`,
  `Cylindrical`, `Frontal`).
- The projection cycles offered in this build are Spherical, Cylindrical, Flat, Cubic, Frontal,
  Spatial, UVW Mapping, Shrink Wrapping, Camera Mapping (Spherical is the default). The first
  five are the ones the form asks for.
- Frontal is baked around the document's scene camera, not around a created camera object: the
  default document camera "Camera" sits at (600, 300, -600) with rotation (45, -19.45, 0), and
  that name, position and rotation are recorded in the case.
- Cubic reads the tag's Size as a divisor: Size (50, 50, 50) doubles every coordinate relative
  to Size (100, 100, 100), which is why values above 1 appear.
- Offset U 25 % made no difference: `proj-cylindrical-offset` and `proj-cylindrical` export
  identical texture coordinates (49 of them per file are byte-identical, and the whole `vt`
  block matches). The parameter stores 25 % on the tag but the generated coordinates ignore it.
- The material itself is irrelevant to the result beyond creating the tag: a default material
  ("Mat") with no shader gives the same coordinates for all seven projections except where the
  projection or the tag size differs.
