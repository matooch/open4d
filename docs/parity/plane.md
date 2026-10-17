# Plane primitive — observation

Status: observed · App version: Cinema 4D R20.059 (Build RB272827) · OS: Windows 11 Pro 10.0.26200 · Observer: observer session · Date: 2026-10-17
Exporter settings: [export-settings.md](export-settings.md). Folder: `tests/golden/plane/<case>/`.

Created with Create > Object > Plane in a new scene (File > New). Parameters are in the
Attributes panel, "Object" tab (tabs: Basic, Coord., Object, Phong), group "Object
Properties", in this order: Width, Height, Width Segments, Height Segments, Orientation.

## Parameters (as shown in the app)

| App label | Our key | Type | Default | Min | Max | Unit shown | Notes |
|---|---|---|---|---|---|---|---|
| Width | `width` | float | 400 | 0 (-100 typed → 0 cm) | none seen (999999999 accepted; the Coordinates manager showed 999999999 cm) | cm | |
| Height | `height` | float | 400 | 0 (-100 typed → 0 cm) | none seen (999999999 accepted; field shows 99999999… truncated, the Coordinates manager Z size showed a value starting 1000000013.9, cut off) | cm | |
| Width Segments | `segments_w` | int | 20 | 1 (-5 typed → 1) | 1000 (999999 typed → 1000) | none | |
| Height Segments | `segments_h` | int | 20 | 1 (-5 typed → 1) | 1000 (999999 typed → 1000) | none | |
| Orientation | `orientation` | enum | +Y | – | – | – | dropdown, 6 options, listed below |

Orientation options (label → our key), in the order the dropdown lists them:
`+X` → `pos_x`, `-X` → `neg_x`, `+Y` → `pos_y` (default), `-Y` → `neg_y`,
`+Z` → `pos_z`, `-Z` → `neg_z`.

## Cases

| Case | Set | `golden info` (vertices / faces / size) | Notes |
|---|---|---|---|
| defaults | – | 441 / 400 (4-gons 400) / size (400, 0, 400), centre (0, 0, 0) | normals 1 (`vn 0 1 0`), uvs 441 |
| sizes | width = 300, height = 100 | 441 / 400 (4-gons 400) / size (300, 0, 100), centre (0, 0, 0) | Width runs along X, Height along Z (at +Y) |
| segments | segments = (3, 5) | 24 / 15 (4-gons 15) / size (400.00001525878906, 0, 400), centre (0.00000762939453125, 0, 0) | 4 distinct X values, 6 distinct Z values: Width Segments split X, Height Segments split Z. First vertices: (-200, 0, -200), (-66.66666412353516, 0, -200), (66.66667175292969, 0, -200) |
| orientation-pos_x | orientation = +X | 441 / 400 (4-gons 400) / size (0, 400, 400), centre (0, 0, 0) | `vn 1 0 0`; first vertex (0, 200, -200) |
| orientation-neg_x | orientation = -X | 441 / 400 (4-gons 400) / size (0, 400, 400), centre (0, 0, 0) | `vn -1 0 0`; first vertex (0, -200, -200) |
| orientation-pos_y | orientation = +Y | 441 / 400 (4-gons 400) / size (400, 0, 400), centre (0, 0, 0) | `vn 0 1 0`; first vertex (-200, 0, -200). Same as defaults. |
| orientation-neg_y | orientation = -Y | 441 / 400 (4-gons 400) / size (400, 0, 400), centre (0, 0, 0) | `vn 0 -1 0`; first vertex (200, 0, -200) |
| orientation-pos_z | orientation = +Z | 441 / 400 (4-gons 400) / size (400, 400, 0), centre (0, 0, 0) | `vn 0 0 1`; first vertex (-200, 200, 0) |
| orientation-neg_z | orientation = -Z | 441 / 400 (4-gons 400) / size (400, 400, 0), centre (0, 0, 0) | `vn 0 0 -1`; first vertex (-200, -200, 0) |

## Behaviour notes
- The plane is a single-sided grid of (Width Segments + 1) × (Height Segments + 1)
  vertices and Width Segments × Height Segments quads, centred on the origin, with one
  shared normal pointing along the chosen Orientation.
- Every orientation file has the same face index list (first face
  `f 1/1/1 22/2/1 23/3/1 2/4/1`); only the vertex positions and the normal change.
- Minimum segments is 1 and maximum 1000 on both axes; sizes clamp at 0 at the low end.
