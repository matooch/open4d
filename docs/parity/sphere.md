# Sphere primitive — observation

Status: observed · App version: Cinema 4D R20.059 (Build RB272827) · OS: Windows 11 Pro 10.0.26200 · Observer: observer session · Date: 2026-10-17
Exporter settings: [export-settings.md](export-settings.md). Folder: `tests/golden/sphere/<case>/`.

Created with Create > Object > Sphere in a new scene (File > New). Parameters are in the
Attributes panel, "Object" tab (tabs: Basic, Coord., Object, Phong), group "Object
Properties", in this order: Radius, Segments, Type, Render Perfect.

## Parameters (as shown in the app)

| App label | Our key | Type | Default | Min | Max | Unit shown | Notes |
|---|---|---|---|---|---|---|---|
| Radius | `radius` | float | 100 | 0 (-100 typed → 0 cm) | none seen (999999999 accepted; the Coordinates manager showed size 1999999998 cm) | cm | |
| Segments | `segments` | int | 24 | 3 (-5 typed → 3) | 1000 (999999 typed → 1000) | none | one number only |
| Type | `type` | enum | Standard | – | – | – | dropdown, 6 options, listed below |
| Render Perfect | `render_perfect` | bool | on | – | – | – | checkbox; greyed out (disabled) when Type = Hemisphere. Not changed in any case. |

Type options (label → our key), in the order the dropdown lists them:
`Standard` → `standard` (default), `Tetrahedron` → `tetrahedron`,
`Hexahedron` → `hexahedron`, `Octahedron` → `octahedron`,
`Icosahedron` → `icosahedron`, `Hemisphere` → `hemisphere`.

## Cases

| Case | Set | `golden info` (vertices / faces / size) | Notes |
|---|---|---|---|
| defaults | – | 266 / 288 (3-gons 48, 4-gons 240) / size (200, 200, 200), centre (0, 0, 0) | normals 267, uvs 323. 13 distinct Y values. |
| radius | radius = 37.5 | 266 / 288 (3-gons 48, 4-gons 240) / size (75, 75, 75), centre (0, 0, 0) | normals 267, uvs 323 |
| segments-8 | segments = 8 | 26 / 32 (3-gons 16, 4-gons 16) / size (200, 200, 200), centre (0, 0, 0) | normals 35, uvs 43. 5 distinct Y values. |
| segments-odd | segments = 9 | 42 / 50 (3-gons 20, 4-gons 30) / size (190.2113037109375, 200, 180.90170288085938), centre (0, 0, 0) | normals 43, uvs 64. 6 distinct Y values; bbox (-95.10565185546875, -100, -90.45085144042969)..(95.10565185546875, 100, 90.45085144042969). Odd counts reveal pole/seam rules. |
| type-standard | type = standard | 266 / 288 (3-gons 48, 4-gons 240) / size (200, 200, 200), centre (0, 0, 0) | Same as defaults. |
| type-tetrahedron | type = tetrahedron | 290 / 576 (3-gons 576) / size (200, 200, 198.19805145263672), centre (0, 0, 0.9009742736816406) | normals 291, uvs 325. bbox (-100, -100, -98.19805145263672)..(100, 100, 100) |
| type-hexahedron | type = hexahedron | 386 / 384 (4-gons 384) / size (200, 200, 200), centre (0, 0, 0) | normals 386, uvs 81 |
| type-octahedron | type = octahedron | 146 / 288 (3-gons 288) / size (200, 200, 200), centre (0, 0, 0) | normals 147, uvs 176 |
| type-icosahedron | type = icosahedron | 162 / 320 (3-gons 320) / size (200, 200, 200), centre (0, 0, 0) | normals 163, uvs 206 |
| type-hemisphere | type = hemisphere | 145 / 144 (3-gons 24, 4-gons 120) / size (200, 100.00000000000001, 200), centre (0, 49.99999999999999, 0) | normals 146, uvs 174. bbox Y from -0.00000000000002 to 100: upper half only, open at the bottom. |
| type-tetrahedron-segments | type = tetrahedron, segments = 8 | 34 / 64 (3-gons 64) / size (200, 181.649658203125, 186.6025390625), centre (0, 9.1751708984375, 6.69873046875) | normals 35, uvs 45. bbox (-100, -81.649658203125, -86.6025390625)..(100, 100, 100) |
| type-hexahedron-segments | type = hexahedron, segments = 8 | 26 / 24 (4-gons 24) / size (200, 200, 200), centre (0, 0, 0) | normals 54, uvs 9 |
| type-octahedron-segments | type = octahedron, segments = 8 | 18 / 32 (3-gons 32) / size (200, 200, 200), centre (0, 0, 0) | normals 49, uvs 28 |
| type-icosahedron-segments | type = icosahedron, segments = 8 | 12 / 20 (3-gons 20) / size (170.13015747070313, 170.13015747070313, 170.13015747070313), centre (0, 0, 0) | normals 21, uvs 23. Points at ±85.06507873535156, not ±100. |
| type-hemisphere-segments | type = hemisphere, segments = 8 | 17 / 16 (3-gons 8, 4-gons 8) / size (200, 100, 200), centre (0, 50, 0) | normals 18, uvs 26. 3 distinct Y values. |

Does Segments mean the same thing for every type? No. The same Segments = 8 gives 26
vertices for Standard and Hexahedron, 34 for Tetrahedron, 18 for Octahedron, 12 for
Icosahedron and 17 for Hemisphere (raw counts above; not interpreted further).

## Behaviour notes
- Where the seam is (which side) and whether poles are triangles or n-gons: read from
  the exported Standard/Hemisphere files (vertices that carry more than one UV U value).
  The seam is the meridian on the **+X side** (z = 0, x > 0, from pole to pole: e.g.
  (25.88, -96.59, 0), (50, -86.6, 0) … in defaults; (70.71, -70.71, 0), (100, 0, 0),
  (70.71, 70.71, 0) in segments-8). **Poles are triangle fans**, not n-gons: every
  triangle in the Standard and Hemisphere files touches a pole (defaults 48 of 48,
  segments-8 16 of 16, segments-odd 20 of 20, hemisphere-segments 8 of 8), and the
  rest are quads.
- The first vertex is the south pole (0, -100, 0) for Standard; for Hemisphere it is the
  top pole (0, 100, 0).
- Render Perfect is disabled for Hemisphere.
- The Tetrahedron type is not centred on the origin (centre Z 0.90 at defaults, centre
  (0, 9.18, 6.70) at segments 8).
