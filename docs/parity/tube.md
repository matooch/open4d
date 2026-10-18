# Tube primitive — observation

Status: observed · App version: Cinema 4D R20.059 (Build RB272827) · OS: Windows 11 Pro 10.0.26200 · Observer: observer session · Date: 2026-10-18
Exporter settings: [export-settings.md](export-settings.md) (no deviation: No Material, Flip Z Axis off, set by hand each export).
Folder: `tests/golden/tube/<case>/`.

Reading note: the screen capture is low-resolution in this batch; every value was checked
against the exported mesh (`golden info` and vertex radii). Values marked "(reading
uncertain)" could not be read with certainty.

## Where it lives
Create > Object > Tube (new scene each case: File > New). Attributes tabs: Basic,
Coord., Object, Slice, Phong. The fillet options sit on the Object tab (no Caps tab).

## Parameters (as shown in the app)

| App label | Our key | Type | Default | Min | Max | Unit shown | Notes |
|---|---|---|---|---|---|---|---|
| Object > Inner Radius | `inner_radius` | float | 50 | 0 (-10 typed; probe export has inner vertices on the axis) | not tested | cm | |
| Object > Outer Radius | `outer_radius` | float | 200 | not tested | not tested | cm | |
| Object > Rotation Segments | `rotation_segments` | int | 36 | 3 (-5 typed; probe export has 3 sides) | not tested | – | |
| Object > Cap Segments | `cap_segments` | int | 1 | 1 (-5 typed; probe has one ring row per cap) | not tested | – | rings across the flat top/bottom ring |
| Object > Height | `height` | float | 100 | not tested | not tested | cm | |
| Object > Height Segments | `height_segments` | int | 1 | 1 (-5 typed) | not tested | – | |
| Object > Orientation | `orientation` | enum | +Y | – | – | – | same 6 options as the cylinder |
| Object > Fillet | `fillet` | bool | off | – | – | – | greys out the two fields below when off |
| Object > (Fillet) Segments | `fillet_segments` | int | 5 (reading uncertain) | not tested | not tested | – | |
| Object > (Fillet) Radius | `fillet_radius` | float | 20 (reading uncertain; mesh: first fillet vertex at radius 70 = inner 50 + 20) | not tested | not tested | cm | |
| Slice > Slice / From / To / Regular Grid / Width | `slice`, `slice_from`, `slice_to`, `regular_grid`, `regular_grid_width` | bool / float / float / bool / float | off / 0 / 360 / off / 10 | not tested | not tested | – / ° / ° / – / cm | same layout as the cylinder's Slice tab |

Enum options (label → our key): `+X` → `pos_x`, `-X` → `neg_x`, `+Y` → `pos_y`
(default), `-Y` → `neg_y`, `+Z` → `pos_z`, `-Z` → `neg_z`.

## Cases

| Case | Parameters changed from default | `golden info` output (vertices / faces / size) | Notes |
|---|---|---|---|
| defaults | – | 144 / 144 (4-gons 144) / size (400, 100, 400), centre (0, 0, 0); normals 38, uvs 144 | 4 rings × 36 (inner/outer × bottom/top); first vertex (50, -50, 0) |
| radii | inner_radius = 30, outer_radius = 120, height = 300 | 144 / 144 (4-gons 144) / size (240, 300, 240), centre (0, 0, 0); normals 38, uvs 144 | |
| segments | rotation_segments = 7, cap_segments = 2, height_segments = 3 | 70 / 70 (4-gons 70) / size (380.1937713623047, 100, 389.9711608886719), centre (9.903114318847656, 0, 0); normals 18, uvs 74 | 10 rings × 7: radius 50 and 200 at Y -50, -16.67, 16.67, 50, plus the cap mid-ring at radius 125 on top and bottom |
| fillet | fillet on (defaults) | 1296 / 1296 (4-gons 1296) / size (400, 100, 400), centre (0, 0, 0); normals 1293, uvs 1369 | 36 rings × 36; 18 Y levels; all four ring edges rounded |
| orientation-pos_x | orientation = +X | 144 / 144 (4-gons 144) / size (100, 400, 400), centre (0, 0, 0); normals 38, uvs 144 | first vertex (-50, -50, 0) |
| slice | slice on, from 0° to 270° | 148 / 148 (3-gons 4, 4-gons 144) / size (400, 100, 400), centre (0, 0, 0); normals 56, uvs 168 | 37 × 4 vertices; each cut end closed by 2 triangles |

## Behaviour notes
- Which parameters are greyed out or hidden by others: Fillet off greys out fillet
  Segments and Radius; Slice off greys out From, To, Regular Grid and Width.
- Behaviour at the minimum segment counts: Rotation Segments clamps to 3, Cap Segments
  and Height Segments to 1, Inner Radius to 0. Probe (inner 0, rotation 3, cap 1,
  height 1): 8 vertices, 12 quads; the 3 inner vertices collapse onto the axis and are
  written once per height level (0, ±50, 0); faces touching the axis are written as degenerate quads with a repeated vertex (e.g. `f 1 2 5 1`, `f 3 4 4 6`), not as triangles.
- Anything surprising: the closed tube is all quads with no seam duplicates (vertices =
  faces). With Slice each flat cut end (a quad between the inner and outer wall) is
  written as 2 triangles. No face has more than 4 corners.
