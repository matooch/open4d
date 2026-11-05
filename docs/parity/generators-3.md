# Generators III — observation (O20)

Status: observed (59 of 59 cases) · App version: Cinema 4D R25 (version number 25010) · OS: macOS (Darwin 25.6.0) · Observer: observer session (MCP) · Date: 2026-11-04
Method: scripted ([agents/OBSERVER.md](../agents/OBSERVER.md)). Level A. Inputs are made
through their menu commands and placed as the generator's children in the order given.
- Export the generator to `tests/golden/<feature>/<case>/mesh.obj`.
- `case.toml` lists every generator parameter under our keys, with each child under
  `[params.input]`, `[params.input2]` and so on, in child order, as the existing
  generator cases do.
- `observation = "docs/parity/generators-3.md"`.

Unblocks: PLAN phase 6, plus deeper coverage of the generators we already have.

The case names use the existing feature folders. The form's `platonic-f/…`,
`oil-tank-f/…`, `polygon-f/…`, `figure-f/…` and `landscape-f/…` group prefixes name
follow-up families; their cases live under `platonic/`, `oil-tank/`, `polygon-prim/`,
`figure/` and `landscape/` with explicit names (see the case table).

## Parameters (fill once per new generator)

Defaults read from the R25 Attribute Manager (defaults table probe). Min/Max from
typing ±999999999 and reading back (int and bool rows do not clamp through the API and
show "–").

### Metaball (`metaball`)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Hull Value | `hull_value` | 100 % | 0 % | 1000 % |
| Editor Subdivision | `editor_subdiv` | 40 | 0.001 | unbounded |
| Render Subdivision | `render_subdiv` | 5 | 0.001 | unbounded |
| Exponential Falloff | `exponential_falloff` | off | – | – |
| Accurate Normals | `accurate_normals` | off | – | – |

### Atom Array (`atom-array`)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Cylinder Radius | `cylinder_radius` | 2 | 0 | unbounded |
| Sphere Radius | `sphere_radius` | 5 | 0 | unbounded |
| Subdivisions | `subdivisions` | 8 | int, 0 typed | (max untested) |
| Single Elements | `single_elements` | off | – | – |

### Spline Mask, measured through an Extrude as in [splines-2.md](splines-2.md) (`spline-mask`)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Mode | `mode` | Union | – | Union, A subtract B, B subtract A, And, Or, Intersection |
| Axis | `axis` | XY (Along Z) | – | XY (Along Z), ZY (Along X), XZ (Along Y), Viewport (Render View) |
| Create Cap | `create_cap` | off | – | – |

## Cases

In `case.toml`, `type` in a `[params.input…]` table names the child object (`"sphere"`). A child's own Type setting goes under `<object>_type` (`sphere_type = "standard"`): TOML rejects a repeated key.

Every case below was exported and has `case.toml` + `mesh.obj` under
`tests/golden/<feature>/<case>/`. The first five are from the previous run's commit
(metaball, atom-array), the rest from this run. Folder names as used:

| Case (folder) | Generator | Children | Changed from default |
|---|---|---|---|
| metaball/two-spheres | Metaball | two default spheres at x −60 and x 60 | – |
| metaball/hull-50 | Metaball | the same | hull value 50 % |
| metaball/editor-sub | Metaball | the same | editor subdivision 20 |
| atom-array/cube | Atom Array | a default cube | – |
| atom-array/radii | Atom Array | a default cube | cylinder radius 10, sphere radius 20, subdivision 6 |
| spline-mask/union | Spline Mask inside an Extrude (offset 20, caps off) | two default Circles, the second at x 150 | – (Mode Union is the default) |
| spline-mask/subtract | the same | the same | Mode A subtract B |
| boole/r25-union | Boole | two default cubes, the second at (50, 50, 50) | Boolean Type A union B; **R25 defaults** (Hide new edges defaults to on) |
| boole/r25-a_minus_b | Boole | the same | Boolean Type A subtract B (the default) |
| boole/r25-intersect | Boole | the same | Boolean Type A intersect B |
| boole/r25-a_without_b | Boole | the same | Boolean Type A without B |
| subdivision/r25-type-catmull_clark | Subdivision Surface | a default cube | Type Catmull-Clark, R25 default levels (editor 2 / render 2) |
| subdivision/r25-type-catmull_clark_ngons | Subdivision Surface | a default cube | Type Catmull-Clark (N-Gons) (the default) |
| subdivision/r25-type-osd_catmull_clark | Subdivision Surface | a default cube | Type OpenSubdiv Catmull-Clark |
| subdivision/r25-type-osd_loop | Subdivision Surface | a default cube | Type OpenSubdiv Loop |
| subdivision/r25-type-osd_bilinear | Subdivision Surface | a default cube | Type OpenSubdiv Bilinear |
| symmetry/r25-plane-xy | Symmetry | a default cube at x 100 | Mirror Plane XY |
| symmetry/r25-plane-zy | Symmetry | a default cube at x 100 | Mirror Plane ZY (the default) |
| symmetry/r25-plane-xz | Symmetry | a default cube at x 100 | Mirror Plane XZ |
| sweep/rail | Sweep | default Circle (profile), default Helix (path), a second Helix with end radius 400 (rail) | – (2-Rail, Use Rail Direction and Use Rail Scale default to on in R25) |
| sweep/banking | Sweep | default Circle, default Helix | Banking off (the R25 default is on) |
| sweep/scale-end | Sweep | default Circle, default Helix | end scale 20 %, end rotation 180° |
| loft/three-circles | Loft | three default Circles at y 0, 200 and 400 (Plane XZ), the middle one with radius 100 | – |
| loft/linear | Loft | the same | Linear Interpolation on |
| lathe/partial | Lathe | default Rectangle at x 300 (Plane XY) | angle 180° |
| lathe/movement | Lathe | the same | movement 200 |
| platonic/hexa-seg3 | Platonic | – | Hexahedron, Segments 3 (brief 035) |
| platonic/tetra-seg3 | Platonic | – | Tetrahedron, Segments 3 |
| platonic/octa-seg3 | Platonic | – | Octahedron, Segments 3 |
| platonic/icosa-r1000 | Platonic | – | Icosahedron, Radius 1000 |
| platonic/bucky-r1000 | Platonic | – | Buckyball, Radius 1000 |
| platonic/bucky-r10 | Platonic | – | Buckyball, Radius 10 |
| platonic/bucky-seg2 | Platonic | – | Buckyball, Segments 2 |
| platonic/dodeca-seg2 | Platonic | – | Dodecahedron, Segments 2 |
| oil-tank/cap | Oil Tank | – | Cap Height 50 |
| oil-tank/cap-r50 | Oil Tank | – | Cap Height 50 and Radius 50 |
| oil-tank/orient-z | Oil Tank | – | Orientation −Z |
| oil-tank/orient-z-grid | Oil Tank | – | Orientation −Z, Slice on and Regular Grid on |
| polygon-prim/tri-seg3 | Polygon | – | Triangle on, Segments 3 |
| polygon-prim/tri-seg3-z | Polygon | – | Triangle on, Segments 3, Orientation +Z |
| polygon-prim/tri-seg3-size | Polygon | – | Triangle on, Segments 3, Width 200, Height 50 |
| figure/h90 | Figure | – | Height 90 (bounding box in Notes; the mesh is not used for parity) |
| landscape/sea-level-50 | Landscape | – | Sea Level 50 % (brief 036) |
| landscape/sea-level-50-plateau-50 | Landscape | – | Sea Level 50 % and Plateau Level 50 % |
| landscape/spherical | Landscape | – | Spherical on |
| landscape/orient-z | Landscape | – | Orientation +Z |
| landscape/scale-2 | Landscape | – | Scale 2 |
| landscape/scale-05 | Landscape | – | Scale 0.5 |
| landscape/size | Landscape | – | Size X and Z 1200, Size Y 200 |
| landscape/furrows-rough-0 | Landscape | – | Rough Furrows 0 % |
| landscape/furrows-rough-100 | Landscape | – | Rough Furrows 100 % |
| landscape/furrows-fine-0 | Landscape | – | Fine Furrows 0 % |
| landscape/furrows-fine-100 | Landscape | – | Fine Furrows 100 % |
| landscape/no-borders | Landscape | – | Borders at Sea Level off |
| landscape/segments-4 | Landscape | – | Width and Depth Segments 4 |
| landscape/seed-1 | Landscape | – | Seed 1 |
| landscape/seed-2 | Landscape | – | Seed 2 |
| landscape/seed-3 | Landscape | – | Seed 3 |
| landscape/segments-1000 | Landscape | – | Width and Depth Segments 1000 (high-resolution slice) |

## Follow-up cases (planner, after brief 043)

Brief 043 matched every case above except the strict point and face numbering of the four
`boole/r25-*` cases (BOOLE.md §5a, parity-pending), and Loop on non-quad input. These cases
separate the hypotheses it could not; run them like the cases above, R25 defaults. The `sweep/` and `lathe/` rows come from brief 042 (helix route, banking frame, 2-Rail profile, Lathe scaling); the `atom-array/` and `spline-mask/` rows from brief 041 (oblique edges, modes, axis, caps).

| Case (folder) | Generator | Children | Changed from default |
|---|---|---|---|
| boole/r25-union-shift-x | Boole | two default cubes, the second at (50, 0, 0) | Boolean Type A union B |
| boole/r25-a_minus_b-shift-x | Boole | the same | Boolean Type A subtract B |
| boole/r25-union-inside | Boole | a default cube and a second cube of size 100 at (0, 0, 0) | Boolean Type A union B |
| boole/r25-a_minus_b-inside | Boole | the same | Boolean Type A subtract B |
| boole/r25-union-uneven | Boole | two default cubes, the second at (30, 50, 70) | Boolean Type A union B |
| boole/r25-a_minus_b-uneven | Boole | the same | Boolean Type A subtract B |
| subdivision/r25-loop-pyramid | Subdivision Surface | a default Pyramid | Type OpenSubdiv Loop |
| sweep/r25-helix-sub4 | Sweep | a Circle profile (radius 20) and a default Helix with Subdivision 4 | – (also export the Helix alone as a spline with its Bezier handles) |
| sweep/r25-helix-sub8 | Sweep | the same, Helix Subdivision 8 | – |
| sweep/r25-helix-radius | Sweep | the same, Helix End Radius 400 | – |
| sweep/r25-banking-planar | Sweep | a Circle profile (radius 20) and a default Flower spline | Banking on (default) |
| sweep/r25-scale-end-only | Sweep | a Circle profile and a straight 2-point Linear spline of length 400 | End Scale 50 % |
| sweep/r25-banking-off-rotated | Sweep | a Circle profile and a default Helix rotated 90° about X | Banking off |
| sweep/r25-rail-rectangle | Sweep | a Rectangle profile (100 × 50), a default Helix and a second Helix with End Radius 400 | – (2-Rail on by default) |
| lathe/r25-scaling-50 | Lathe | a default Arc spline | Scaling 50 % |
| atom-array/pyramid | Atom Array | a default Pyramid | Subdivisions 5 |
| atom-array/rotated-cube | Atom Array | a default cube rotated (30°, 45°, 0°) | – |
| atom-array/single-elements | Atom Array | a default cube | Single Elements on |
| spline-mask/and | Spline Mask | the two circles of `spline-mask/union` | Mode And |
| spline-mask/or | Spline Mask | the same | Mode Or |
| spline-mask/intersection | Spline Mask | the same | Mode Intersection |
| spline-mask/b-minus-a | Spline Mask | the same | Mode B subtract A |
| spline-mask/axis-zy | Spline Mask | the same circles rotated into the ZY plane | Axis ZY |
| spline-mask/cap | Spline Mask | the two circles of `spline-mask/union` | Create Cap on |
| spline-mask/rect-circle | Spline Mask | a default Rectangle and a default Circle at x 100 | Mode Union |

## Notes

**R25 differences from the R20 observation docs found while running these cases:**

- **Boole** (`boole/r25-*`): the mode labels are the same four as R20 (A union B, A
  subtract B, A intersect B, A without B; default A subtract B), but **Hide new edges
  defaults to on** in R25 (R20: off), so the n-gon shapes differ from the R20 cases:
  r25-union 26 / 12 (4-gons 6, 6-gons 6), r25-a_minus_b 20 / 9 (4-gons 6, 6-gons 3),
  r25-intersect 14 / 6, r25-a_without_b 13 / 6 (4-gons 3, 6-gons 3). High Quality on,
  Optimize Points 0.01, as R20.
- **Subdivision Surface** (`subdivision/r25-type-*`): the R25 Type dropdown has only
  **5 options** — Catmull-Clark, Catmull-Clark (N-Gons) (default), OpenSubdiv
  Catmull-Clark, OpenSubdiv Loop, OpenSubdiv Bilinear. R20's "OpenSubdiv Catmull-Clark
  (Adaptive)" is not offered. A new **R12 Compatibility** toggle (default off) shows on
  every type. Defaults: Editor level 2, **Renderer level 2** (R20: 3), Subdivide UVs
  Edge (R20: Standard). Counts with one default cube: bilinear 98 / 96 (quads; bilinear keeps the flat cube ±100); loop 98 / 192 (triangles).
- **Symmetry** (`symmetry/r25-plane-*`): same three Mirror Plane options (XY, ZY default,
  XZ) and defaults as R20. Over the cube at x 100: XY and XZ merge no points (16 / 12,
  size (200, 200, 200) shifted on one axis), ZY merges the plane points (12 / 12, size
  x 400, like the R20 weld rule).
- **Sweep** (`sweep/rail`): 2-Rail, Use Rail Direction and Use Rail Scale all default
  to on in R25, so a third child spline (second default Helix with End Radius 400) acts
  as rail with no extra setting. 32436 / 32402 faces (quads + two 36-gon caps). Banking
  and scale-end also export 32436 / 32402 with the R25 defaults.
- **Figure** got a new read-only-looking row "Legacy Axis Mode" (default off per its
  readback 0) and the Platonic/Oil Tank/Landscape/Polygon/Figure objects all show a
  "Switch to Updated …" button in R25; buttons were not pressed.
- **Landscape**: the R25 defaults match the O18 defaults case (sea level 0, plateau
  100 %, seed 0). Sea Level 50 % flattens everything below the middle: max height drops
  from 73.14 (defaults) to 58.47; adding Plateau Level 50 % flattens to max 0.29.
  Borders at Sea Level off lets the border exceed 100 (max 100 exactly at scale 1).
- Metaball Hull Value clamps to 0–1000 % (raw 0–10); Editor/Render Subdivision min 0.001
  and unbounded above; Atom Array radii min 0 and unbounded.

**`golden info` per case** (vertices / faces / size, size = bbox max − min):

| Case | Verts / faces | Size |
|---|---|---|
| metaball/two-spheres | 262 / 360 (3-gons 200, 4-gons 160) | (350.136, 260.858, 260.858) |
| metaball/hull-50 | 486 / 616 (3-gons 264, 4-gons 352) | (453.070, 385.400, 385.400) |
| metaball/editor-sub | 982 / 1160 (3-gons 360, 4-gons 800) | (344.411, 257.846, 257.846) |
| atom-array/cube | 400 / 352 (3-gons 128, 4-gons 224) | (210, 210, 210) |
| atom-array/radii | 256 / 216 (3-gons 96, 4-gons 120) | (234.641, 240, 230) |
| spline-mask/union | 864 / 432 (4-gons 432) | (550, 399.833, 20) |
| spline-mask/subtract | 684 / 342 (4-gons 342) | (274.853, 399.981, 20) |
| boole/r25-union | 26 / 12 | (250, 250, 250) |
| boole/r25-a_minus_b | 20 / 9 | (200, 200, 200) |
| boole/r25-intersect | 14 / 6 | (150, 150, 150) |
| boole/r25-a_without_b | 13 / 6 | (200, 200, 200) |
| subdivision/r25-type-catmull_clark | 98 / 96 | (175.694, 175.694, 175.694) |
| subdivision/r25-type-catmull_clark_ngons | 98 / 96 | same |
| subdivision/r25-type-osd_catmull_clark | 98 / 96 | xy 175.694, z 175.694458 |
| subdivision/r25-type-osd_loop | 98 / 192 (3-gons 192) | (179.091, 179.091, 181.445) |
| subdivision/r25-type-osd_bilinear | 98 / 96 | (200, 200, 200) |
| symmetry/r25-plane-xy | 16 / 12 | (200, 200, 200) |
| symmetry/r25-plane-zy | 12 / 12 | (400, 200, 200) |
| symmetry/r25-plane-xz | 16 / 12 | (200, 200, 200) |
| sweep/rail | 32436 / 32402 | (750.329, 700.709, 199.980) |
| sweep/banking | 32436 / 32402 | (800.020, 799.980, 598.387) |
| sweep/scale-end | 32436 / 32402 | (760.301, 719.614, 439.107) |
| loft/three-circles | 144 / 130 (4-gons 128, 16-gons 2) | (400, 400, 400) |
| loft/linear | 144 / 130 | same topology, different positions |
| lathe/partial | 132 / 130 | (1000, 400, 500) |
| lathe/movement | 132 / 130 | (1000, 600, 1000) |
| platonic/hexa-seg3 | 56 / 54 (4-gons 54) | (115.470, 115.470, 115.470) |
| platonic/tetra-seg3 | 20 / 36 (3-gons 36) | (163.299, 133.333, 141.421) |
| platonic/octa-seg3 | 38 / 72 (3-gons 72) | (200, 200, 200) |
| platonic/icosa-r1000 | 12 / 20 (3-gons 20) | (1701.304, 1701.304, 1701.304) |
| platonic/bucky-r1000 | 60 / 64 | size ≈ (1926.748, 1894.823, 1840.549) |
| platonic/bucky-r10 | 60 / 64 | size ≈ (19.268, 19.248, 18.405) |
| platonic/bucky-seg2 | 234 / 256 (3-gons 48, 4-gons 208) | (192.675, 189.482, 184.055) |
| platonic/dodeca-seg2 | 74 / 96 (3-gons 48, 4-gons 48) | (186.834, 186.834, 186.834) |
| oil-tank/cap | 178 / 192 (3-gons 32, 4-gons 160) | (200, 200, 200) |
| oil-tank/cap-r50 | 178 / 192 | (100, 200, 100) |
| oil-tank/orient-z | 178 / 192 | (200, 200, 200) |
| oil-tank/orient-z-grid | 553 / 904 (3-gons 744, 4-gons 160) | z-min face at 0 |
| polygon-prim/tri-seg3 | 10 / 9 (3-gons 9) | (100, 0, 100) |
| polygon-prim/tri-seg3-z | 10 / 9 | (100, 100, 0) |
| polygon-prim/tri-seg3-size | 10 / 9 | (200, 0, 50) |
| figure/h90 | 2858 / 3206 | bbox (−45.749, −0.047, −10.266)–(45.749, 89.933, 5.204): the figure's own origin keeps its feet near y 0 with Height 90; height scales to 90 as expected |
| landscape/sea-level-50 | 10201 / 10000 | (600, 58.473, 600) |
| landscape/sea-level-50-plateau-50 | 10201 / 10000 | (600, 0.292, 600) |
| landscape/spherical | 4902 / 5000 (3-gons 200, 4-gons 4800) | (778.647, 760.363, 763.972) |
| landscape/orient-z | 10201 / 10000 | (600, 600, 73.141) |
| landscape/scale-2 | 10201 / 10000 | (600, 79.619, 600) |
| landscape/scale-05 | 10201 / 10000 | (600, 53.327, 600) |
| landscape/size | 10201 / 10000 | (1200, 146.281, 1200) |
| landscape/furrows-rough-0 | 10201 / 10000 | (600, 66.763, 600) |
| landscape/furrows-rough-100 | 10201 / 10000 | (600, 70.765, 600) |
| landscape/furrows-fine-0 | 10201 / 10000 | (600, 71.852, 600) |
| landscape/furrows-fine-100 | 10201 / 10000 | (600, 75.231, 600) |
| landscape/no-borders | 10201 / 10000 | (600, 100, 600) |
| landscape/segments-4 | 25 / 16 | (600, 52.320, 600) |
| landscape/seed-1 | 10201 / 10000 | (600, 96.328, 600) |
| landscape/seed-2 | 10201 / 10000 | (600, 70.287, 600) |
| landscape/seed-3 | 10201 / 10000 | (600, 90.116, 600) |
| landscape/segments-1000 | 1002001 / 1000000 | (600.000028, 72.612511, 600.000028); mesh.obj is 184 MB |

Method notes: every case started from a new empty document; generator objects were made
through the app's own Create menu commands (`CallCommand` with the object's command id),
children placed in the order the table gives. Every generator parameter the scene holds
was read back after the case and written to `case.toml` (percent and degrees shown as
the Attribute Manager shows them). The Spline Mask cases add `[params]` rows
`extrude_offset = 20.0` and `caps = false` for their wrapping Extrude, as in
splines-2's cases. Subdivision Surface cases also record the R25-only
`r12_compatibility` toggle.