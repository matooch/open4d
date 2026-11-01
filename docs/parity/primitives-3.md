# Primitives III — observation (O18)

Status: observed · App version: Cinema 4D R25 (version number 25010) · OS: macOS (Darwin 25.6.0) · Observer: observer session (MCP) · Date: 2026-10-29
Method: scripted, exactly as the primitive forms (for example [cube.md](cube.md)). Level A.
Unblocks: the solid primitives we don't have yet (PLAN phase 5).

Every case:
- The object is made through its menu command, at the origin.
- Only the row's changes are set.
- The object is exported as `tests/golden/<feature>/<case>/mesh.obj`, with the feature
  names given below.

`case.toml` lists **every** parameter under our keys, with
`observation = "docs/parity/primitives-3.md"`.

How the values were read: one probe (outside the repo) made each object with its menu
command in an empty document and read every visible Attribute Manager parameter: label,
default and, for numbers, what the app keeps after 999999999 and -999999999 are assigned
through the scripting interface (it clamps to the field's limits, like typing). Angles are
in degrees, percentages in %. "none" = ±999999999 accepted. Nothing was typed into the GUI,
so a limit enforced only by the GUI would not show. Enum values are in `case.toml` as
lower-case words (`plus_y` = +Y).

## Parameters

### Platonic (`platonic`)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Radius | `radius` | 100 cm | 0 | none |
| Segments | `segments` | 1 | 1 | 1000 |
| Type | `type` | Icosa | – | – |

Type options, in order: Tetra, Hexa, Octa, Dodeca, Icosa, Bucky.

### Oil Tank (`oil-tank`)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Radius | `radius` | 100 cm | 0 | none |
| Height | `height` | 200 cm | 0 | none |
| Height Segments | `height_segments` | 4 | 1 | 1000 |
| Cap Height | `cap_height` | 25 cm | 0 | none |
| Cap Segments | `cap_segments` | 4 | 1 | 1000 |
| Rotation Segments | `rotation_segments` | 16 | 3 | 1000 |
| Orientation | `orientation` | +Y | – | – |
| Slice | `slice` | off | – | – |
| From | `slice_from` | 0° | none | none |
| To | `slice_to` | 180° | none | none |
| Regular Grid | `regular_grid` | off | – | – |
| Width | `grid_width` | 10 cm | 0.01 | none |

Orientation options, in order: +X, -X, +Y, -Y, +Z, -Z (the same list for every object below
that has one).

### Polygon (`polygon-prim`)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Width | `width` | 100 cm | 0 | none |
| Height | `height` | 100 cm | 0 | none |
| Segments | `segments` | 1 | 1 | 1000 |
| Triangle | `triangle` | off | – | – |
| Orientation | `orientation` | +Y | – | – |

### Figure (`figure`)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Height | `height` | 180 cm | 0 | none |
| Segments | `segments` | 14 | 4 | 1000 |

### Landscape (`landscape`)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Size (X / Y / Z) | `size_x` / `size_y` / `size_z` | 600 / 100 / 600 cm | 0 | none |
| Width Segments | `width_segments` | 100 | 4 | 1000 |
| Depth Segments | `depth_segments` | 100 | 4 | 1000 |
| Rough Furrows | `rough_furrows` | 50 % | 0 | 100 % |
| Fine Furrows | `fine_furrows` | 50 % | 0 | 100 % |
| Scale | `scale` | 1 | 0 | 10 |
| Sea Level | `sea_level` | 0 % | 0 | 100 % |
| Plateau Level | `plateau_level` | 100 % | 0 | 100 % |
| Orientation | `orientation` | +Y | – | – |
| Multifractal | `multifractal` | on | – | – |
| Seed | `seed` | 0 | none | none |
| Borders At Sea Level | `borders_at_sea_level` | on | – | – |
| Spherical | `spherical` | off | – | – |

## Cases

| Case | Object | Changed from default | `golden info` (vertices / faces / size) |
|---|---|---|---|
| platonic/defaults | Platonic | – (Icosa) | 12 / 20 (3-gons 20) / (170.130, 170.130, 170.130), centre 0 |
| platonic/type-tetra | Platonic | Type Tetra | 4 / 4 (3-gons 4) / (163.299, 133.333, 141.421), centre (0, 33.333, 23.570) |
| platonic/type-hexa | Platonic | Type Hexa | 8 / 6 (4-gons 6) / (115.470, 115.470, 115.470), centre 0 |
| platonic/type-octa | Platonic | Type Octa | 6 / 8 (3-gons 8) / (200, 200, 200), centre 0 |
| platonic/type-dodeca | Platonic | Type Dodeca | 20 / 24 (3-gons 12, 4-gons 12) / (186.834, 186.834, 186.834), centre 0 |
| platonic/type-icosa | Platonic | Type Icosa (same as defaults) | 12 / 20 (3-gons 20) / (170.130, 170.130, 170.130), centre 0 |
| platonic/type-bucky | Platonic | Type Bucky | 60 / 64 (3-gons 12, 4-gons 52) / (192.675, 189.482, 184.055), centre (-0.076, 1.621, -0.170) |
| platonic/segments | Platonic | Segments 3 (Icosa) | 92 / 180 (3-gons 180) / (170.130, 170.130, 170.130), centre 0 |
| oil-tank/defaults | Oil Tank | – | 178 / 192 (3-gons 32, 4-gons 160) / (200, 200, 200), centre 0 |
| oil-tank/segments | Oil Tank | Height Segments 3, Cap Segments 6, Rotation Segments 12 | 170 / 180 (3-gons 24, 4-gons 156) / (200, 200, 200), centre 0 |
| oil-tank/slice | Oil Tank | Slice on, To 270° | 189 / 214 (3-gons 54, 4-gons 160) / (199.518, 200, 199.518), centre (0.241, 0, -0.241) |
| polygon-prim/defaults | Polygon | – | 4 / 1 (4-gons 1) / (100, 0, 100), centre 0 |
| polygon-prim/triangle | Polygon | Triangle on | 3 / 1 (3-gons 1) / (100, 0, 100), centre 0 |
| polygon-prim/segments | Polygon | Segments 4 | 25 / 16 (4-gons 16) / (100, 0, 100), centre 0 |
| figure/defaults | Figure | – | 2858 / 3206 (3-gons 812, 4-gons 2394) / (182.995, 179.960, 30.941), centre (0, 89.887, -5.062) |
| landscape/defaults | Landscape | – (seed shown: 0) | 10201 / 10000 (4-gons 10000) / (600, 73.141, 600), centre (0, 36.570, 0) |
| landscape/seed | Landscape | Seed 12345 | 10201 / 10000 (4-gons 10000) / (600, 99.605, 600), centre (0, 49.802, 0) |
| landscape/segments | Landscape | Width and Depth Segments 20 | 441 / 400 (4-gons 400) / (600, 70.293, 600), centre (0, 35.147, 0) |
| landscape/flat | Landscape | Multifractal off, Rough and Fine Furrows 50 % | 10201 / 10000 (4-gons 10000) / (600, 67.892, 600), centre (0, 33.946, 0) |

19 cases. No case failed. Readbacks equal the values set in every case (no clamping hit).

## Notes
- **Landscape is deterministic from its settings.** The default object (seed 0) was built and
  exported twice in separate empty documents; the two OBJ files were byte-identical (the
  second copy is not kept). Seed 12345 gives a different mesh (peak height 99.605 vs 73.141),
  so the seed alone selects the mesh, given the other settings below. Settings that
  influence it, all recorded in each `case.toml`: Size, Width/Depth Segments, Rough and Fine
  Furrows, Scale, Sea Level, Plateau Level, Multifractal, Seed, Borders At Sea Level,
  Spherical, Orientation. Spherical was never turned on (not in the form's case list).
- **The "Fractal Noise off" case is Multifractal off.** The R25 Landscape page has no
  other fractal-noise switch; the nearest is Multifractal. The furrows already default to
  50 %, so `landscape/flat` differs from the defaults only in Multifractal.
- Landscape Sea Level and Plateau Level are shown as percentages; the OBJ has all heights
  between 0 and the Size Y of 100 cm (peak 73.1 cm at the defaults).
- The exported Dodecahedron has 12 triangles and 12 quads for its 12 pentagonal faces (24 faces,
  20 vertices). The counts suggest each pentagon is written as a triangle plus a quad; the
  face lists were not inspected to confirm. The mesh is kept as exported.
- Platonic Tetra is not centred on the origin: its bounding box runs from y -33.33 to
  100 and z -47.14 to 94.28 (the vertex on +Y is at the radius). The other types are
  centred. Bucky is irregular (centre slightly off the origin), as exported.
- The Polygon primitive lies flat in the XZ plane at Orientation +Y (size 100 × 0 × 100).
- Figure: Segments minimum is 4; the figure's lowest point is slightly below zero
  (y -0.094) and its top is 179.867, not exactly the Height of 180.
- Oil Tank with Slice on, From 0° and To 270° adds vertices on the cut faces (189 vs 178)
  and 22 more triangles than the unsliced tank.
- Not covered by the form's cases, so not observed: Platonic Radius, Oil Tank Orientation
  other than +Y, Regular Grid on, Polygon Width/Height other than 100, Landscape Spherical,
  Sea Level and Plateau Level other than their defaults.
