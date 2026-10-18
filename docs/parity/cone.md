# Cone primitive — observation

Status: observed · App version: Cinema 4D R20.059 (Build RB272827) · OS: Windows 11 Pro 10.0.26200 · Observer: observer session · Date: 2026-10-18
Exporter settings: [export-settings.md](export-settings.md) (no deviation: No Material, Flip Z Axis off, set by hand each export).
Folder: `tests/golden/cone/<case>/`.

Reading note: the screen capture is low-resolution in this batch, so field text is small.
Every case was checked against the exported mesh (`golden info`); values marked
"(reading uncertain)" could not be read with certainty.

## Where it lives
Create > Object > Cone (new scene each case: File > New). Attributes tabs: Basic,
Coord., Object, Caps, Slice, Phong.

## Parameters (as shown in the app)

| App label | Our key | Type | Default | Min | Max | Unit shown | Notes |
|---|---|---|---|---|---|---|---|
| Object > Top Radius | `top_radius` | float | 0 | 0 (-10 typed → 0; exported apex is one point) | not tested | cm | |
| Object > Bottom Radius | `bottom_radius` | float | 100 | not tested | not tested | cm | |
| Object > Height | `height` | float | 200 | not tested | not tested | cm | |
| Object > Height Segments | `height_segments` | int | 8 | 1 (-5 typed; exported side is one row) | not tested | – | |
| Object > Rotation Segments | `rotation_segments` | int | 36 | 3 (-5 typed; exported cone has 3 sides) | not tested | – | |
| Object > Orientation | `orientation` | enum | +Y | – | – | – | same 6 options as the cylinder |
| Caps > Caps | `caps` | bool | on | – | – | – | off greys out the rest of the tab |
| Caps > Cap Segments | `cap_segments` | int | **3** | not tested | not tested | – | the field was first read as 1, but the default mesh has cap rings at 1/3 and 2/3 of the radius, and typing 3 changed nothing (identical file) |
| Caps > Fillet Segments | `fillet_segments` | int | 5 (reading uncertain) | not tested | not tested | – | greyed until Top or Bottom fillet is on |
| Caps > Top | `fillet_top` | bool | off | – | – | – | enables its Radius and Height |
| Caps > Top > Radius | `fillet_top_radius` | float | 50 | not tested | not tested | cm | read as 30 cm after Top was ticked with Top Radius 30 (reading uncertain; possibly clamped to the top radius) |
| Caps > Top > Height | `fillet_top_height` | float | 50 | not tested | not tested | cm | |
| Caps > Bottom | `fillet_bottom` | bool | off | – | – | – | enables its Radius and Height |
| Caps > Bottom > Radius | `fillet_bottom_radius` | float | 50 | not tested | not tested | cm | |
| Caps > Bottom > Height | `fillet_bottom_height` | float | 50 | not tested | not tested | cm | |
| Slice > Slice / From / To / Regular Grid / Width | `slice`, `slice_from`, `slice_to`, `regular_grid`, `regular_grid_width` | bool / float / float / bool / float | off / 0 / 360 / off / 10 | not tested | not tested | – / ° / ° / – / cm | same layout as the cylinder's Slice tab |

Enum options (label → our key): `+X` → `pos_x`, `-X` → `neg_x`, `+Y` → `pos_y`
(default), `-Y` → `neg_y`, `+Z` → `pos_z`, `-Z` → `neg_z`.

## Cases

| Case | Parameters changed from default | `golden info` output (vertices / faces / size) | Notes |
|---|---|---|---|
| defaults | – | 362 / 396 (3-gons 36, 4-gons 360) / size (200, 200, 200), centre (0, 0, 0); normals 204, uvs 433 | 9 Y levels (-100 … 75 in steps of 25, plus the apex at 100). The 36 triangles are the bottom-cap centre fan; the apex row is 36 **degenerate quads** that repeat the apex vertex (e.g. `f 4 5 5 8`) |
| frustum | top_radius = 30 | 470 / 504 (3-gons 72, 4-gons 432) / size (200, 200, 200), centre (0, 0, 0); normals 248, uvs 433 | top cap also gets 3 rings |
| segments | rotation_segments = 7, height_segments = 3 | 37 / 42 (3-gons 7, 4-gons 35) / size (190.09688568115234, 200, 194.98558044433594), centre (4.951557159423828, 0, 0); normals 23, uvs 54 | |
| caps-off | caps = off, top_radius = 30 | 324 / 288 (4-gons 288) / size (200, 200, 200), centre (0, 0, 0); normals 245, uvs 333 | side only: 9 rings × 36 |
| cap-segments | cap_segments = 5, top_radius = 30 | 614 / 648 (3-gons 72, 4-gons 576) / size (200, 200, 200), centre (0, 0, 0); normals 248, uvs 509 | **5 used instead of 3**, because 3 is the default (the export with 3 was byte-identical to frustum). Bottom rings at radius 20, 40, 60, 80, 100. |
| fillet-bottom | bottom fillet on (defaults) | 542 / 576 (3-gons 36, 4-gons 540) / size (160.55650329589844, 200, 160.55650329589844), centre (0, 0, 0); normals 420, uvs 628 | 14 Y levels; the fillet pulls the widest point in to radius ≈ 80.28 |
| fillet-top | top_radius = 30, top fillet on (defaults) | 542 / 576 (3-gons 72, 4-gons 504) / size (200, 200, 200), centre (0, 0, 0); normals 411, uvs 626 | 14 Y levels |
| orientation-pos_x | orientation = +X | 362 / 396 (3-gons 36, 4-gons 360) / size (200, 200, 200), centre (0, 0, 0); normals 204, uvs 433 | first vertex (-100, 0, 0): the base is at -X, the apex at +X |
| slice | slice on, from 0° to 270° | 372 / 416 (3-gons 56, 4-gons 360) / size (200, 200, 200), centre (0, 0, 0); normals 294, uvs 436 | |

## Behaviour notes
- Which parameters are greyed out or hidden by others: Caps off greys out Cap Segments,
  Fillet Segments and both fillets; Fillet Segments and each fillet's Radius/Height are
  greyed until Top or Bottom is ticked; Slice off greys out its fields.
- Behaviour at the minimum segment counts: Rotation Segments clamps to 3, Height
  Segments to 1, Top Radius to 0. The probe (rotation 3, height 1, top 0) exported 11
  vertices / 12 faces (3 triangles + 9 quads): a 3-ring bottom cap and one row of
  degenerate apex quads.
- Anything surprising: unlike the cylinder (cap Segments default 1), the cone's Cap
  Segments default is 3. With Top Radius 0 there is a single apex vertex and the top row
  is written as quads with the apex repeated, not triangles. No face has more than 4
  corners.
