# Cylinder primitive — observation

Status: observed · App version: Cinema 4D R20.059 (Build RB272827) · OS: Windows 11 Pro 10.0.26200 · Observer: observer session · Date: 2026-10-18
Exporter settings: [export-settings.md](export-settings.md) (no deviation: No Material, Flip Z Axis off, set by hand each export).
Folder: `tests/golden/cylinder/<case>/`.

Reading note: from this batch on the screen runs at a lower capture resolution, so field
text is small. Defaults below were read at the earlier, sharper resolution (2026-10-17
smoothing batch) and re-read now; every case was checked against the exported mesh
(`golden info`), which is the reliable source. Values marked "(reading uncertain)" could
not be read with certainty.

## Where it lives
Create > Object > Cylinder (new scene each case: File > New). Attributes tabs: Basic,
Coord., Object, Caps, Slice, Phong.

## Parameters (as shown in the app)

| App label | Our key | Type | Default | Min | Max | Unit shown | Notes |
|---|---|---|---|---|---|---|---|
| Object > Radius | `radius` | float | 50 | 0 (-10 typed → 0) | none seen (999999999 accepted; export bbox ±1e9) | cm | |
| Object > Height | `height` | float | 200 | 0 (-10 typed → 0) | not tested | cm | |
| Object > Height Segments | `height_segments` | int | 1 | 1 (-5 typed → 1) | 1000 (999999 typed → 1000) | – | |
| Object > Rotation Segments | `rotation_segments` | int | 36 | 3 (-5 typed; export of that state has a 3-sided cylinder) | 1000 (999999 typed → 1000) | – | |
| Object > Orientation | `orientation` | enum | +Y | – | – | – | 6 options |
| Caps > Caps | `caps` | bool | on | – | – | – | off greys out Segments and the fillet fields |
| Caps > Segments | `cap_segments` | int | 1 | 1 (-5 typed → 1) | 1000 (999999 typed → 1000) | – | |
| Caps > Fillet | `fillet` | bool | off | – | – | – | off greys out the two fields below |
| Caps > (Fillet) Segments | `fillet_segments` | int | 5 | 1 (-5 typed → 1) | 1000 (999999 typed → 1000) | – | |
| Caps > (Fillet) Radius | `fillet_radius` | float | 20 | 0 (-5 typed → 0) | 50 with the default size (9999 typed → 50 cm, i.e. the cylinder Radius; reading uncertain) | cm | |
| Slice > Slice | `slice` | bool | off | – | – | – | off greys out From, To, Regular Grid, Width |
| Slice > From | `slice_from` | float | 0 | no clamp seen (-9999 accepted, reading uncertain) | – | ° | |
| Slice > To | `slice_to` | float | 360 | – | no clamp seen (9999 accepted, reading uncertain) | ° | |
| Slice > Regular Grid | `regular_grid` | bool | off | – | – | – | not used in any case |
| Slice > Width | `regular_grid_width` | float | 10 | not tested | not tested | cm | greyed while Regular Grid is off |

Enum options (label → our key), dropdown order: `+X` → `pos_x`, `-X` → `neg_x`,
`+Y` → `pos_y` (default), `-Y` → `neg_y`, `+Z` → `pos_z`, `-Z` → `neg_z`.

## Cases

| Case | Parameters changed from default | `golden info` output (vertices / faces / size) | Notes |
|---|---|---|---|
| defaults | – | 74 / 108 (3-gons 72, 4-gons 36) / size (100, 200, 100), centre (0, 0, 0); normals 39, uvs 109 | byte-identical (apart from line endings) to `tests/golden/smoothing/cylinder-default/mesh.obj` |
| size | radius = 75, height = 300 | 74 / 108 (3-gons 72, 4-gons 36) / size (150, 300, 150), centre (0, 0, 0) | radius 50 is the default, so 75 was used instead |
| segments | rotation_segments = 7, height_segments = 3 | 30 / 35 (3-gons 14, 4-gons 21) / size (95.04844284057617, 200, 97.49279022216795), centre (2.475778579711914, 0, 0); normals 12, uvs 47 | 4 distinct Y levels; the odd count leaves the bbox off-centre in X (a vertex at x = +50) |
| caps-off | caps = off | 72 / 36 (4-gons 36) / size (100, 200, 100), centre (0, 0, 0); normals 36, uvs 74 | no centre vertices; first vertex (50, -100, 0) |
| cap-segments | cap_segments = 3, rotation_segments = 7 | 44 / 49 (3-gons 14, 4-gons 35) / size (95.04844284057617, 200, 97.49279022216795), centre (2.475778579711914, 0, 0); normals 10, uvs 59 | each cap: a triangle fan of 7 around the centre plus 2 rings of 7 quads |
| fillet | caps on, fillet on (default fillet Segments 5, Radius 20) | 434 / 468 (3-gons 72, 4-gons 396) / size (100, 200, 100), centre (0, 0, 0); normals 435, uvs 517 | 12 distinct Y levels: 434 = 2 + 36 × 12 |
| fillet-segments | fillet on, fillet_segments = 3, fillet_radius = 20, height_segments = 2 | 326 / 360 (3-gons 72, 4-gons 288) / size (100, 200, 100), centre (0, 0, 0); normals 327, uvs 406 | 9 Y levels: 326 = 2 + 36 × 9 (confirms 3 fillet segments; the field reading itself was uncertain) |
| orientation-pos_x | orientation = +X | 74 / 108 (3-gons 72, 4-gons 36) / size (200, 100, 100), centre (0, 0, 0) | first vertex (-100, 0, 0) |
| orientation-neg_z | orientation = −Z | 74 / 108 (3-gons 72, 4-gons 36) / size (100, 100, 200), centre (0, 0, 0) | first vertex (0, 0, 100) |
| slice | slice on, from 0° to 270° | 76 / 112 (3-gons 76, 4-gons 36) / size (100, 200, 100), centre (0, 0, 0); normals 42, uvs 121 | the viewport shows a quarter removed with flat cut faces |

## Behaviour notes
- Which parameters are greyed out or hidden by others: Caps off greys out cap Segments,
  Fillet, fillet Segments and Radius; Fillet off greys out fillet Segments and Radius;
  Slice off greys out From, To, Regular Grid and Width; Width is greyed while Regular
  Grid is off.
- Behaviour at the minimum segment counts: Rotation Segments clamps to 3 (a triangular
  prism), Height Segments, cap Segments and fillet Segments clamp to 1.
- Anything surprising: caps are triangle fans around a centre vertex (the first two
  vertices of the file are the bottom and top centres (0, ∓100, 0)); with cap Segments > 1
  only the innermost ring is triangles, the rest quads. No face has more than 4 corners.
  The side is welded (caps-off: 72 vertices for 36 quads, no seam duplicates). The fillet
  radius is limited by the cylinder radius.
