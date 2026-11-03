# Primitives III — platonic, oil tank, polygon, figure (`crates/generators`)

Sources: observation O18 ([primitives-3.md](../parity/primitives-3.md)) and its golden cases
`tests/golden/{platonic, oil-tank, polygon-prim, figure}`; the vertex-order and flat
subdivision rules of [PRIMITIVES.md](PRIMITIVES.md) §3.1 and [PRIMITIVES2.md](PRIMITIVES2.md)
§1–2, §8–9; the triangle/quad split of the pentagon in
[GEOMETRY.md](GEOMETRY.md) §6.1 R3. Conventions as in PRIMITIVES.md §0 (`build`,
`BuildError::Unsupported` for observed options that are not derived). Case files hold
degrees; params store radians. Landscape (same observation) is §6, brief 036.

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

## 6. Landscape (`forge.landscape`) — brief 036

Source: O18 `landscape` (4 golden cases, 10201 / 441 vertices), the parameter table of
[primitives-3.md](../parity/primitives-3.md), and the noise library of [NOISE.md](NOISE.md)
(Perlin, `octave_seed`). Created headless with `object.create` and the registry defaults.

| id | label | kind | default | range |
|---|---|---|---|---|
| `size_x`, `size_y`, `size_z` | Size | distance | 600 / 100 / 600 | 0 … |
| `width_segments` | Width Segments | int | 100 | 4 … 1000 |
| `depth_segments` | Depth Segments | int | 100 | 4 … 1000 |
| `rough_furrows` | Rough Furrows | percent | 50 % | 0 … 100 % |
| `fine_furrows` | Fine Furrows | percent | 50 % | 0 … 100 % |
| `scale` | Scale | float | 1 | 0 … 10 |
| `sea_level` | Sea Level | percent | 0 % | 0 … 100 % |
| `plateau_level` | Plateau Level | percent | 100 % | 0 … 100 % |
| `orientation` | Orientation | enum | `pos_y` | |
| `multifractal` | Multifractal | bool | on | |
| `seed` | Seed | int | 0 | |
| `borders_at_sea_level` | Borders At Sea Level | bool | on | |
| `spherical` | Spherical | bool | off | |

Percent params are stored as fractions (`0.5`); the case files hold `50.0` and the parity test
divides by 100. The observed defaults are the table above (every default is in the case files).

### L1. Topology (exact; all 4 cases)
Grid of `n = width_segments + 1` columns by `m = depth_segments + 1` rows, row by row along
Z with X running fastest: vertex `k * n + i` (column `i`, row `k`) is at
`x = -size_x/2 + size_x * i / width_segments`, `z = -size_z/2 + size_z * k / depth_segments`
(`landscape/defaults`: vertex 0 = `(-300, 0, -300)`, vertex 1 = `(-294, 0, -300)`, vertex 101 =
`(-300, 0, -294)`; `landscape/segments`: step 30). Quads `[a, a+n, a+n+1, a+1]` with
`a = k * n + i`, row-major over `(k, i)` (face 0 = `[0, 101, 102, 1]` 0-based, last face =
`[10098, 10199, 10200, 10099]`). The positions in the export carry accumulated float noise
(`-276.000001`), within the 1e-4 tolerance of ours.

### L2. Facts measured on the heights (O18 `landscape/*`)
- **F1. Continuous in position.** `landscape/segments` (20 x 20) equals `landscape/defaults`
  sampled every 5th point times one constant `1.02489` (all 361 interior ratios between
  1.024863 and 1.024890). So the height field is a function of the position, not of the
  index, and only a global factor depends on the grid: ours is a normalisation by the
  extremes of the sampled grid (L3).
- **F2. Border mask.** All border vertices are exactly 0 (400 zeros; 401 in `flat`). Near the
  border the height grows like the square of the distance to the nearest border:
  `h(2,2)/h(1,1) = 4.26`, `h(3,3)/h(1,1) = 10.3` on the diagonal, and `h(1,1) = 0.0111` is about
  `h(2,1)`, which a product of per-axis masks would make 4 times larger. The mask is therefore a
  function of the Chebyshev distance `t = min(u, 1-u, v, 1-v)` (u, v in `[0,1]`), about `t^2`
  near 0. Ours: `sin^2(pi t)`. Row ratios `h(i+1)/h(i)` at the border (4.06, 2.28, 1.80 for
  i = 1, 2, 3) are matched by it to about 3 %. **Parity-pending:** the exact curve far from the
  border.
- **F3. Field statistics.** The height divided by the mask (|t| >= 0.1, 6561 points) has, in
  units of Size Y: defaults mean 0.59, std 0.20, p10 0.34, p90 0.85; seed 12345 mean 0.55,
  std 0.24; `flat` (Multifractal off) mean 0.45, std 0.19. The lag-10 autocorrelation (10 cells
  = 0.1 of the size) is 0.51 / 0.72 / 0.45, lag-1 0.99, so the smoothest feature is about
  a third of the size and there is fine detail on top. Size Y 100 is reached (seed 12345 peaks at
  99.6, defaults at 73.1, `flat` 67.9).

### L3. Our height model (own design, level C: not the reference noise)
Exact heights were not derived (hypotheses tried: H1 product-of-axes border mask, rejected by
F2; H2 Chebyshev-distance `sin^2` mask, kept; H3 grid-extreme normalisation as the origin of the
F1 factor, kept; the reference lattice noise itself cannot be identified from a mesh of
heights at 6 cm to 30 cm spacing, so no third noise hypothesis is testable).
With `L = 0.3 * max(size_x, size_z) * scale` (`scale` 0 behaves as 0.001), the noise point
of a vertex is `q = (x / L, 0.37 + z / L, 0)` (so Perlin's zero lattice is never sampled), and
octave `i` (0 … 7) is `n_i = perlin3(octave_seed(seed, i), 2^i q + 17.3 i)`, with weights
`a_0 = 1`, `a_i = r^i` for `i = 1 … 3`, `a_i = r^3 f^(i-3)` for `i = 4 … 7`, `r = rough_furrows`,
`f = fine_furrows`. Rough furrows therefore set the mid octaves, fine furrows the highest four.
- *Not multifractal:* `s = 0.5 + 0.5 * (sum a_i n_i) / (sum a_i)`, then rescaled to `[0, 1]` by
  the minimum and maximum over the grid.
- *Multifractal* (Musgrave heterogeneous terrain, published in "Texturing and Modeling"):
  `s = 1.2 + n_0`; for `i >= 1`: `s += a_i * (n_i + 1.2) * s`, `s = max(s, 0)` at the end,
  then divided by the grid maximum. Detail is stronger where the lower octaves are high.
Then `w = clamp(s, sea_level, plateau_level)`, `w = (w - sea_level) / (1 - sea_level)` (0 when
`sea_level >= 1`), the border mask when `borders_at_sea_level` (`w *= sin^2(pi t)`), and
`y = size_y * w`. Sea 0 % and plateau 100 % leave `s` unchanged (the observed defaults; other
values are **parity-pending**: never observed). The seed is the integer reduced to 32 bits.
- *Spherical* (never observed, own design, **parity-pending**): the grid is wrapped around a
  sphere of radius `R = min(size_x, size_z) / 2`: longitude `2 pi u`, latitude `pi (v - 0.5)`,
  radius `R + y`. The height field and the seam are as above.
- *Orientation* is the shared map of PRIMITIVES2.md §1 (only `plus_y` observed, **parity-pending**).
- Segments below 4 are rejected by the range; sizes of 0 give a degenerate flat mesh.

### L4. Tests
- `landscape_matches_the_reference_topology` (parity file): for all 4 cases the face lists are
  identical and the x/z positions are within 1e-4 of the export.
- `landscape_heights_are_statistically_close`: heights divided by Size Y, our mesh against
  the export of the same case, with fixed tolerances: mean within 0.07, std within 0.07,
  maximum in `[0.45, 1.0]`, the 10-bin histogram intersection at least 0.60, border vertices all
  0, and the correlation of the two height maps at least 0.25. The correlation is explained
  by the shared border mask only (the two noises are independent), so it is a check of the
  mask and of the scale of the envelope, not of the noise; the histogram and moments are the
  statistical check of the heights. The comparison is statistical because the reference noise
  function cannot be derived from a mesh of heights (F1 to F3 are all that the data shows).
  Measured (ours / export, heights over Size Y): defaults mean 0.155 / 0.173, std 0.162 / 0.182,
  max 0.69 / 0.73, histogram overlap 0.94, correlation 0.92; flat 0.132 / 0.141, 0.147 / 0.164,
  0.67 / 0.68, 0.94, 0.81; seed 0.146 / 0.170, 0.145 / 0.207, 0.67 / 1.00, 0.90, 0.79;
  segments 0.147 / 0.164, 0.165 / 0.185, 0.69 / 0.70, 0.94, 0.93. The offset 1.2 of the
  multifractal was chosen so that the moments sit inside these tolerances (0.5 gave means of
  0.095); it is a free constant of our model, not an observed value. `seed` peaks at 0.996 in
  the export and 0.67 in ours: peak height is not modelled.
  `landscape/segments` additionally checks F1 for ours: the 20 x 20 grid equals the 100 x 100
  grid every 5th point up to one constant factor.
- Unit tests: determinism per seed, different seeds differ, sea level raises the zero set,
  plateau caps the height, `borders_at_sea_level` off leaves non-zero borders, spherical keeps
  the vertex count, orientation, closed topology.

