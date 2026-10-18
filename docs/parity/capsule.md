# Capsule primitive — observation

Status: observed · App version: Cinema 4D R20.059 (Build RB272827) · OS: Windows 11 Pro 10.0.26200 · Observer: observer session · Date: 2026-10-18
Exporter settings: [export-settings.md](export-settings.md) (no deviation: No Material, Flip Z Axis off, set by hand each export).
Folder: `tests/golden/capsule/<case>/`.

Reading note: the screen capture is low-resolution in this batch; the segment defaults
were not readable on screen and are taken from the exported default mesh.

## Where it lives
Create > Object > Capsule (new scene each case: File > New). Attributes tabs: Basic,
Coord., Object, Slice, Phong.

## Parameters (as shown in the app)

| App label | Our key | Type | Default | Min | Max | Unit shown | Notes |
|---|---|---|---|---|---|---|---|
| Object > Radius | `radius` | float | 50 | not tested | not tested | cm | |
| Object > Height | `height` | float | 200 | not tested | not tested | cm | total height including both hemispheres |
| Object > Height Segments | `height_segments` | int | 8 (from the mesh: cylinder part split at Y -50 … 50 in steps of 12.5) | 1 (-5 typed; probe has one straight row) | not tested | – | |
| Object > Cap Segments | `cap_segments` | int | 8 (from the mesh: 8 latitude steps per hemisphere) | 1 (-5 typed; probe hemisphere is one triangle fan) | not tested | – | |
| Object > Rotation Segments | `rotation_segments` | int | 36 | 3 (-5 typed; probe has 3 sides) | not tested | – | |
| Object > Orientation | `orientation` | enum | +Y | – | – | – | same 6 options as the cylinder |
| Slice > Slice / From / To / Regular Grid / Width | `slice`, `slice_from`, `slice_to`, `regular_grid`, `regular_grid_width` | bool / float / float / bool / float | off / 0 / 360 / off / 10 | not tested | not tested | – / ° / ° / – / cm | same layout as the cylinder's Slice tab |

Enum options (label → our key): `+X` → `pos_x`, `-X` → `neg_x`, `+Y` → `pos_y`
(default), `-Y` → `neg_y`, `+Z` → `pos_z`, `-Z` → `neg_z`.

## Cases

| Case | Parameters changed from default | `golden info` output (vertices / faces / size) | Notes |
|---|---|---|---|
| defaults | – | 830 / 864 (3-gons 72, 4-gons 792) / size (100, 200, 100), centre (0, 0, 0); normals 615, uvs 924 | 2 poles + 23 rings × 36; the 72 triangles are the pole fans; first vertices are the poles (0, -100, 0), (0, 100, 0) |
| size | radius = 30, height = 300 | 830 / 864 (3-gons 72, 4-gons 792) / size (60, 300, 60), centre (0, 0, 0); normals 615, uvs 924 | |
| segments | height_segments = 3, cap_segments = 3, rotation_segments = 7 | 58 / 63 (3-gons 14, 4-gons 49) / size (95.04844284057617, 200, 97.49279022216795), centre (2.475778579711914, 0, 0); normals 52, uvs 79 | ring Y levels: ±93.301, ±75, ±50, ±16.667 |
| orientation-pos_x | orientation = +X | 830 / 864 (3-gons 72, 4-gons 792) / size (200, 100, 100), centre (0, 0, 0); normals 615, uvs 924 | |
| slice | slice on, from 0° to 270° (added: the capsule has the same Slice tab as the others) | 853 / 910 (3-gons 118, 4-gons 792) / size (100, 200, 100), centre (0, 0, 0); normals 653, uvs 924 | |

## Behaviour notes
- Which parameters are greyed out or hidden by others: Slice off greys out From, To,
  Regular Grid and Width; Width is greyed while Regular Grid is off.
- Behaviour at the minimum segment counts: Height, Cap and Rotation Segments clamp to
  1, 1 and 3 (probe: 8 vertices, 6 triangles + 3 quads; Y levels -100, -50, 50, 100).
- Anything surprising: each hemisphere has its latitude rings evenly spaced in angle
  (default: Y = -50 - 50·sin(k·11.25°)), and the cylinder part is split evenly in Y.
  Poles are single vertices with triangle fans. No face has more than 4 corners.
