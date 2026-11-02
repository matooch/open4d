# Primitives III — platonic, oil tank, polygon, figure (`crates/generators`)

Sources: observation O18 ([primitives-3.md](../parity/primitives-3.md)) and its golden cases
`tests/golden/{platonic, oil-tank, polygon-prim, figure}`; the vertex-order and flat
subdivision rules of [PRIMITIVES.md](PRIMITIVES.md) §3.1 and [PRIMITIVES2.md](PRIMITIVES2.md)
§1–2, §8–9; the triangle/quad split of the pentagon in
[GEOMETRY.md](GEOMETRY.md) §6.1 R3. Conventions as in PRIMITIVES.md §0 (`build`,
`BuildError::Unsupported` for observed options that are not derived). Case files hold
degrees; params store radians. Landscape (same observation) is brief 036.

All four are plugin-registered generator nodes (`Primitives::register`), created headless
with `object.create` and the registry defaults below (observed R25 defaults; every default
is in the golden `case.toml`). They are not on the command bar: its design (UI.md §11) lists
only the six commonest primitives.

Node type ids: `forge.platonic`, `forge.oil_tank`, `forge.polygon_prim` (`forge.polygon`
is the editable polygon object of EDITABLE.md), `forge.figure`. Case keys map to ours
unchanged except: the orientation words `plus_x … minus_z` (case files) are `pos_x … neg_z`
(PRIMITIVES2.md §1), and the Oil Tank's `grid_width` is `regular_grid_width` (shared slice
params of the round primitives). The parity tests translate those two spellings.

## 1. Platonic (`forge.platonic`)

| id | label | kind | default | range |
|---|---|---|---|---|
| `radius` | Radius | distance | 100 | 0 … |
| `segments` | Segments | int | 1 | 1 … 1000 |
| `type` | Type | enum `tetra, hexa, octa, dodeca, icosa, bucky` | `icosa` | |

(O18 table.) The radius is the circumradius: every type's vertices lie at distance `radius`
from the origin, except that **Tetra is not centred** and **Bucky is irregular** (below).

**P1. Base solids** at `segments = 1` (cases `platonic/type-*`, `platonic/defaults`):
- *tetra, octa, icosa* are the point and face lists of PRIMITIVES.md §3.1 (the sphere
  types, `triangle_solid`) scaled by `radius`. **The icosa corners `(0, ±1, ±φ)` are divided
  by the rounded length `1.90211`, not by `√(1+φ²) = 1.9021130…`**: the export has
  `52.573195, 85.065216` (the exact value 52.573111 misses vertex 0 of
  `platonic/defaults` by 1.35e-4, over the 1e-4 tolerance), i.e. a circumradius of 100.00016 (`platonic/defaults`, `type-icosa`, `segments`; the other types are
  exact). **Parity-pending:** that the same rounded length applies at other radii (assumed
  linear). The Tetra keeps its vertex on +Y, so its
  bounding box runs from `y = −radius/3` to `radius` (observed size
  `(163.299, 133.333, 141.421)`).
- *hexa* has the 8 corners `(±1, ±1, ±1)·radius/√3`; points `0..8` =
  `(−−−), (−+−), (+−−), (++−), (+−+), (+++), (−−+), (−++)` (signs of x, y, z), faces
  `[0,1,3,2], [2,3,5,4], [4,5,7,6], [6,7,1,0], [1,7,5,3], [6,0,2,4]`.
- *dodeca* see P3. *bucky* see P4.

**P2. Segments are flat.** `segments = n > 1` cuts every face of the base solid into the
flat grid of PRIMITIVES.md §3.1 / PRIMITIVES2.md §9 (edge points in first-appearance order,
interior points per face, triangle rows / quad grid) and does **not** project onto the
sphere: `platonic/segments` (icosa, n = 3) has edge points at distance 86.85 from the
origin, face points likewise, and its vertex and face lists equal `flat_subdivision` with
the identity map (92 points, 180 triangles). Hexa, tetra and octa use the same routine
(**parity-pending**: observed for the icosa only). Dodeca and bucky with `segments > 1`
return `BuildError::Unsupported("platonic dodecahedron/bucky segments")` (not observed).

**P3. Dodecahedron** (`platonic/type-dodeca`, 20 points, 12 triangles then 12 quads).
Points `0..12` are the vertices `(0, ±1/φ, ±φ)`, `(±φ, 0, ±1/φ)`, `(±1/φ, ±φ, 0)` in the
order of the case (`φ = (1+√5)/2`, all scaled so the circumradius is `radius`), points
`12..20` the cube corners `(±1, ±1, ±1)`. The 12 pentagons are exported as
`split_pentagon(ring, 0)` (GEOMETRY.md §6.1 R3): triangles `0..12` first, quads `12..24`
after, the pentagon of face `i` having its ear at point `i`. The rings (point lists,
ear tip first, in face winding) are the generator-side assignment of R3 — a table in
`platonic.rs` (`DODECA_RINGS`), because each pentagon has three possible ear tips and the
reference's pick has no geometric rule we found (the tip is neither the lowest nor the highest
index of the three; centres are the 12 icosahedron axes). The table is topology only; the
positions are the regular dodecahedron.

**P4. Bucky** (`platonic/type-bucky`, 60 points, 64 faces = 12 triangles + 52 quads) is a
truncated icosahedron. The export is **irregular**: its points lie at distances 96.67 to 99.79
from the origin (98.12 to 98.31 from their own centroid `(−0.082, 1.622, −0.142)`), the
bounding box is `(192.675, 189.482, 184.055)` centred on `(−0.076, 1.621, −0.170)`, and
the coordinates differ from a regular solid by 0.05 to 0.2.
We build the regular truncated icosahedron of circumradius `radius` with a pentagon facing +Z:
the 12 pentagons become a triangle and a quad as in P3, the 20 hexagons two quads each
(12 + 12 + 40 = 64 faces, 52 quads). Listing order is our own: points in layers of
descending Z, counter-clockwise inside each layer. **Known mismatch**: the observed positions
are not derivable (noise of 0.05–0.2 cm, tolerance 1e-4) and neither is its face order.
Settling data: Bucky at Radius 1000 and 10 (is the irregularity proportional to the radius?),
and Bucky with Segments 2.

**Unsupported:** none except the `segments > 1` of P2.

## 2. Oil Tank (`forge.oil_tank`)

| id | label | kind | default | range |
|---|---|---|---|---|
| `radius` | Radius | distance | 100 | 0 … |
| `height` | Height | distance (total) | 200 | 0 … |
| `height_segments` | Height Segments | int | 4 | 1 … 1000 |
| `cap_height` | Cap Height | distance | 25 | 0 … |
| `cap_segments` | Cap Segments | int | 4 | 1 … 1000 |
| `rotation_segments` | Rotation Segments | int | 16 | 3 … 1000 |
| `orientation` | Orientation | enum | `pos_y` | PRIMITIVES2.md §1 |
| `slice` | Slice | bool | off | |
| `slice_from` / `slice_to` | From / To | angle | 0° / 180° | |
| `regular_grid` | Regular Grid | bool | off | |
| `regular_grid_width` | Width | distance | 10 | 0.01 … |

A capsule-shaped lathe (PRIMITIVES2.md §8: south and north centre points, profile from
south to north, `cap_segments` points per cap including the cap/cylinder junction, then
`height_segments − 1` points on the straight side) whose caps are flat **elliptical arcs**.
With `c = max(0, height/2 − cap_height)`, `h` the clamped cap height, `Θ = 22.5°` and
`θ_k = Θ·k/cap_segments`, bottom cap point `k = 1..cs`:
`(radius·sin θ_k / sin Θ, −c − h + h·(1 − cos θ_k)/(1 − cos Θ))`; the straight side
`(radius, −c + 2c·j/hs)`, `j = 1..hs−1`; the top cap is the mirror image, `k = cs..1`;
south `(0, −c − h)`, north `(0, c + h)`.
Proof: `oil-tank/defaults` (all 178 points, 192 faces, equal to the lathe order of
PRIMITIVES2.md §2, triangles at both poles); `oil-tank/segments` (cap 6, height 3, rotation
12: same formula with other counts, so `Θ` does not depend on the cap segments).
`Θ = 22.5°` was fitted on the default cap, whose four ring radii `25.6131, 50.9796, 75.8550,
100` and heights `1.5815, 6.3106, 14.1419, 25` equal the formula to the 4-decimal export.
**Parity-pending:** whether `Θ` depends on `cap_height / radius` (every case has `25/100`);
other orientations are the PRIMITIVES2.md §1 maps; a zero `cap_height` gives flat ends
(`y` offset 0); a `cap_height` above `height/2` is clamped to `height/2`.

**Slice** (`oil-tank/slice`, To 270°): the lathe slice of PRIMITIVES2.md §2 (rotation
segments + 1 columns from `from` to `to`), the two cut faces triangulated by the reference
Delaunay rule of GEOMETRY.md §6.1 (13-gon, 11 triangles each: 189 points, 214 faces).
**Unsupported:** `regular_grid` on (`BuildError::Unsupported("slice regular grid")`, as the
other round primitives; no case).

## 3. Polygon (`forge.polygon_prim`)

| id | label | kind | default | range |
|---|---|---|---|---|
| `width` | Width | distance | 100 | 0 … |
| `height` | Height | distance | 100 | 0 … |
| `segments` | Segments | int | 1 | 1 … 1000 |
| `triangle` | Triangle | bool | off | |
| `orientation` | Orientation | enum | `pos_y` | |

A single flat face in the XZ plane (`width` along X, `height` along Z). With half sizes
`x = width/2, z = height/2`:
- quad (`polygon-prim/defaults`): corners `(−x,0,−z), (−x,0,z), (x,0,z), (x,0,−z)`, one face
  `[0,1,2,3]`;
- triangle (`polygon-prim/triangle`): `(−x,0,−z), (0,0,z), (x,0,−z)`, face `[0,1,2]`;
- segments `n` (`polygon-prim/segments`, n = 4: 25 points, 16 quads): the flat grid of
  PRIMITIVES2.md §9 on the quad: `n − 1` points per edge in edge order `(0,1) (1,2) (2,3)
  (0,3)`, then the interior `i` outer along `c0→c1` (Z), `k` inner along `c0→c3` (X), faces
  `[b(i,k), b(i+1,k), b(i+1,k+1), b(i,k+1)]`. The triangle with segments uses the triangle
  rows of PRIMITIVES.md §3.1 (**parity-pending**: observed with one segment only).
The `orientation` maps are PRIMITIVES2.md §1 (**parity-pending**: only `plus_y` observed).
Width or height 0 gives a degenerate face with all points kept.

## 4. Figure (`forge.figure`) — own design, level C

| id | label | kind | default | range |
|---|---|---|---|---|
| `height` | Height | distance | 180 | 0 … |
| `segments` | Segments | int | 14 | 4 … 1000 |

**Level C.** The reference mannequin is a sculpted mesh; CLAUDE.md rule 2 forbids copying
assets, so **no vertex, face count, topology or proportion table of the golden mesh is
used** (the vertex and face counts 2858 / 3206 of O18 are not targets). Ours is a
procedural T-pose mannequin built from public figure-drawing proportions (the eight-head
canon: head = `H/8`, shoulders at `0.82 H`, crotch at `0.5 H`, knees at `0.28 H`, arm span
≈ height). It is assembled by merging closed parts:

| part | construction |
|---|---|
| head | ellipsoid lathe, centre `0.9375 H`, radii `(0.055, 0.0625, 0.06) H` |
| neck | tapered capsule, `0.84 H … 0.89 H` |
| torso | tapered capsule from the pelvis `0.50 H` to the chest `0.78 H`, depth squashed to 70 % |
| each arm (2) | upper arm and forearm tapered capsules along ±X at `0.80 H`, hand box |
| each leg (2) | thigh and shin tapered capsules from `0.50 H` to `0.05 H`, foot box on the ground |

Every round part is a lathe with `segments` columns and `max(2, segments / 4)` cap rings, so
`segments` changes the polygon count and `height` scales every dimension linearly. The
feet stand on `y = 0`, the head top is at `y = height`.

**Test (the only parity claim):** the bounding box of the default figure equals the
observed `(182.995, 179.960, 30.941)` within **±5 %** per axis (the observed numbers are the
only golden data the test reads); changing `height` or `segments` changes the mesh; the
mesh is closed per part and valid (`PolyMesh::validate`). The reference's vertical position
(lowest point −0.094, top 179.867) is not matched exactly; ours is `0 … H`.

## 5. Parity tests

`crates/generators/tests/primitives3_parity.rs` covers the cases of `platonic`, `oil-tank`,
`polygon-prim` with the strict mesh comparison (`UNSUPPORTED` and `KNOWN_MISMATCH` lists asserted
exactly) and `figure` with the level-C bounding-box check. `crates/generators/tests/
primitives3.rs` holds the unit tests (defaults, parameters, orientation, closed meshes).
