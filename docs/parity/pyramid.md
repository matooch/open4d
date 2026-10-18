# Pyramid primitive — observation

Status: observed · App version: Cinema 4D R20.059 (Build RB272827) · OS: Windows 11 Pro 10.0.26200 · Observer: observer session · Date: 2026-10-18
Exporter settings: [export-settings.md](export-settings.md) (no deviation: No Material, Flip Z Axis off, set by hand each export).
Folder: `tests/golden/pyramid/<case>/`.

Defaults were first read on the sharper display during the subdivision batch
(2026-10-17) and re-read now; every case was checked against the exported mesh.

## Where it lives
Create > Object > Pyramid (new scene each case: File > New). Attributes tabs: Basic,
Coord., Object, Phong (no Caps or Slice tab).

## Parameters (as shown in the app)

| App label | Our key | Type | Default | Min | Max | Unit shown | Notes |
|---|---|---|---|---|---|---|---|
| Object > Size (three fields: X, Y, Z) | `size_x` / `size_y` / `size_z` | float | 200 / 200 / 200 | not tested | not tested | cm | Y is the height (base at -size_y/2, apex at +size_y/2) |
| Object > Segments | `segments` | int | 1 | not tested | not tested | – | one value for all directions |
| Object > Orientation | `orientation` | enum | +Y | – | – | – | same 6 options as the cylinder |

Enum options (label → our key): `+X` → `pos_x`, `-X` → `neg_x`, `+Y` → `pos_y`
(default), `-Y` → `neg_y`, `+Z` → `pos_z`, `-Z` → `neg_z`.

## Cases

| Case | Parameters changed from default | `golden info` output (vertices / faces / size) | Notes |
|---|---|---|---|
| defaults | – | 5 / 5 (3-gons 4, 4-gons 1) / size (200, 200, 200), centre (0, 0, 0); normals 6, uvs 8 | base quad `f 1 2 3 4` at y = -100 (vertices (-100,-100,-100), (100,-100,-100), (100,-100,100), (-100,-100,100)), apex 5 = (0, 100, 0); identical to `tests/golden/subdivision/tris-1/input.obj` |
| size | size = (100, 200, 300) | 5 / 5 (3-gons 4, 4-gons 1) / size (100, 200, 300), centre (0, 0, 0); normals 6, uvs 8 | |
| segments | segments = 3 | 29 / 45 (3-gons 36, 4-gons 9) / size (200, 200, 200), centre (0, 0, 0); normals 15, uvs 40 | base is a 4 × 4 vertex grid (9 quads); sides have rows at y = -33.33 (8 vertices) and 33.33 (4 vertices) plus the apex; all side faces are triangles |
| orientation-pos_x | orientation = +X | 5 / 5 (3-gons 4, 4-gons 1) / size (200, 200, 200), centre (0, 0, 0); normals 6, uvs 8 | base at x = -100, apex (100, 0, 0) |

## Behaviour notes
- Which parameters are greyed out or hidden by others: none (only three parameters).
- Behaviour at the minimum segment counts: Segments 1 is the default; lower values not
  tested.
- Anything surprising: with Segments > 1 the four sides are subdivided into triangles
  only (36 for Segments 3), while the base becomes a quad grid. The apex is a single
  vertex. No face has more than 4 corners.
