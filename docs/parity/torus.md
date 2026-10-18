# Torus primitive — observation

Status: observed · App version: Cinema 4D R20.059 (Build RB272827) · OS: Windows 11 Pro 10.0.26200 · Observer: observer session · Date: 2026-10-18
Exporter settings: [export-settings.md](export-settings.md) (no deviation: No Material, Flip Z Axis off, set by hand each export).
Folder: `tests/golden/torus/<case>/`.

Reading note: the screen capture is low-resolution in this batch; every value was checked
against the exported mesh (`golden info`).

## Where it lives
Create > Object > Torus (new scene each case: File > New). Attributes tabs: Basic,
Coord., Object, Slice, Phong.

## Parameters (as shown in the app)

| App label | Our key | Type | Default | Min | Max | Unit shown | Notes |
|---|---|---|---|---|---|---|---|
| Object > Ring Radius | `ring_radius` | float | 200 | not tested | not tested | cm | distance from the centre to the middle of the pipe (default outer size 500 = 2 × (200 + 50)) |
| Object > Ring Segments | `ring_segments` | int | 36 | 3 (-5 typed; probe export 3 × 3 = 9 quads) | not tested | – | around the ring |
| Object > Pipe Radius | `pipe_radius` | float | 50 | not tested | not tested | cm | |
| Object > Pipe Segments | `pipe_segments` | int | 18 | 3 (-5 typed; probe as above) | not tested | – | around the pipe |
| Object > Orientation | `orientation` | enum | +Y | – | – | – | same 6 options as the cylinder |
| Slice > Slice / From / To / Regular Grid / Width | `slice`, `slice_from`, `slice_to`, `regular_grid`, `regular_grid_width` | bool / float / float / bool / float | off / 0 / 360 / off / 10 | not tested | not tested | – / ° / ° / – / cm | same layout as the cylinder's Slice tab |

Enum options (label → our key): `+X` → `pos_x`, `-X` → `neg_x`, `+Y` → `pos_y`
(default), `-Y` → `neg_y`, `+Z` → `pos_z`, `-Z` → `neg_z`.

## Cases

| Case | Parameters changed from default | `golden info` output (vertices / faces / size) | Notes |
|---|---|---|---|
| defaults | – | 648 / 648 (4-gons 648) / size (500, 98.48077392578124, 500), centre (0, 0, 0); normals 644, uvs 703 | 36 × 18; height 98.48 < 100 because 18 pipe segments put no vertex exactly on top (first vertices (250, 0, 0), (246.98, 17.10, 0), …: the pipe starts at the outer equator and goes up) |
| radii | ring_radius = 150, pipe_radius = 20 | 648 / 648 (4-gons 648) / size (340, 39.3923110961914, 340), centre (0, 0, 0); normals 643, uvs 703 | |
| segments | ring_segments = 7, pipe_segments = 5 | 35 / 35 (4-gons 35) / size (475.2422180175781, 95.10565185546876, 487.4639587402344), centre (12.378890991210938, 0, 0); normals 35, uvs 48 | |
| orientation-pos_x | orientation = +X | 648 / 648 (4-gons 648) / size (98.48077392578124, 500, 500), centre (0, 0, 0); normals 644, uvs 703 | |
| slice | slice on, from 0° to 270° | 666 / 680 (3-gons 32, 4-gons 648) / size (500, 98.48077392578124, 500), centre (0, 0, 0); normals 687, uvs 703 | the open ends get flat caps of 16 triangles each, built from the 18 rim vertices of that end only (no extra centre vertex; the first cap is a fan from vertex 1: `f 1 3 2`, `f 1 4 3`, …); 666 = 37 rings × 18; the 36 ring segments are spread over the 270° |

## Behaviour notes
- Which parameters are greyed out or hidden by others: Slice off greys out From, To,
  Regular Grid and Width; Width is greyed while Regular Grid is off.
- Behaviour at the minimum segment counts: Ring Segments and Pipe Segments both clamp to
  3 (probe: 9 vertices, 9 quads).
- Anything surprising: the full torus is all quads with no seam duplicates (vertices =
  faces). With Slice the two cut ends are closed with triangle caps that reuse the rim vertices (no centre vertex). No face has more
  than 4 corners.
