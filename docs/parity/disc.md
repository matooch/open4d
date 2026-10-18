# Disc primitive — observation

Status: observed · App version: Cinema 4D R20.059 (Build RB272827) · OS: Windows 11 Pro 10.0.26200 · Observer: observer session · Date: 2026-10-18
Exporter settings: [export-settings.md](export-settings.md) (no deviation: No Material, Flip Z Axis off, set by hand each export).
Folder: `tests/golden/disc/<case>/`.

Reading note: the screen capture is low-resolution in this batch; every value was checked
against the exported mesh (`golden info` and vertex radii).

## Where it lives
Create > Object > Disc (new scene each case: File > New). Attributes tabs: Basic,
Coord., Object, Slice, Phong (no Caps tab).

## Parameters (as shown in the app)

| App label | Our key | Type | Default | Min | Max | Unit shown | Notes |
|---|---|---|---|---|---|---|---|
| Object > Inner Radius | `inner_radius` | float | 0 | 0 (-10 typed → 0) | not tested | cm | |
| Object > Outer Radius | `outer_radius` | float | 100 | not tested | not tested | cm | |
| Object > Disc Segments | `disc_segments` | int | 4 | 1 (-5 typed; export of that state is a single fan) | not tested | – | the default mesh has rings at radius 25, 50, 75, 100 |
| Object > Rotation Segments | `rotation_segments` | int | 36 | 3 (-5 typed; export of that state has 3 triangles) | not tested | – | |
| Object > Orientation | `orientation` | enum | +Y | – | – | – | same 6 options as the cylinder |
| Slice > Slice | `slice` | bool | off | – | – | – | greys out From / To when off |
| Slice > From | `slice_from` | float | 0 | not tested | not tested | ° | |
| Slice > To | `slice_to` | float | 360 | not tested | not tested | ° | |

Enum options (label → our key): `+X` → `pos_x`, `-X` → `neg_x`, `+Y` → `pos_y`
(default), `-Y` → `neg_y`, `+Z` → `pos_z`, `-Z` → `neg_z`. The disc's Slice tab has no
Regular Grid option (unlike the cylinder's).

## Cases

| Case | Parameters changed from default | `golden info` output (vertices / faces / size) | Notes |
|---|---|---|---|
| defaults | – | 145 / 144 (3-gons 36, 4-gons 108) / size (200, 0, 200), centre (0, 0, 0); normals 2, uvs 145 | centre vertex + 4 rings × 36; normals `vn 0 0 0` (unused) and `vn 0 1 0` |
| inner-radius | inner_radius = 30 | 180 / 144 (4-gons 144) / size (200, 0, 200), centre (0, 0, 0); normals 1, uvs 180 | 5 rings × 36 at radius 30, 47.5, 65, 82.5, 100 (evenly spaced between inner and outer); no centre vertex, no triangles |
| segments | rotation_segments = 7, disc_segments = 3 | 22 / 21 (3-gons 7, 4-gons 14) / size (190.09688568115234, 0, 194.98558044433594), centre (4.951557159423828, 0, 0); normals 2, uvs 22 | |
| segments-inner | inner_radius = 30, rotation_segments = 7, disc_segments = 3 | 28 / 21 (4-gons 21) / size (190.09688568115234, 0, 194.98558044433594), centre (4.951557159423828, 0, 0); normals 1, uvs 28 | |
| orientation-pos_x | orientation = +X | 145 / 144 (3-gons 36, 4-gons 108) / size (0, 200, 200), centre (0, 0, 0); normals 2, uvs 145 | normal `vn 1 0 0` |
| slice | slice on, from 0° to 270° | 149 / 144 (3-gons 36, 4-gons 108) / size (200, 0, 200), centre (0, 0, 0); normals 2, uvs 149 | the 36 rotation segments are spread over the 270° (37 spokes: 1 + 37 × 4 = 149) |

## Behaviour notes
- Which parameters are greyed out or hidden by others: Slice off greys out From and To.
- Behaviour at the minimum segment counts: Rotation Segments clamps to 3 and Disc
  Segments to 1 (probe: 4 vertices, 3 triangles); Inner Radius clamps to 0.
- Anything surprising: with Inner Radius 0 the centre is a triangle fan around one centre
  vertex; with Inner Radius > 0 the disc is quads only, and the Disc Segments rings are
  spaced evenly between the inner and outer radius. Slice keeps the Rotation Segments
  count and spreads it over the sliced angle. Files with a centre vertex contain an
  unused `vn 0 0 0`.
