# Primitives II — cylinder, cone, disc, torus, tube, capsule, pyramid (`crates/generators`)

Sources: docs/parity/{cylinder,cone,disc,torus,tube,capsule,pyramid}.md and their golden
cases in `tests/golden/<feature>/`. Conventions as in [PRIMITIVES.md](PRIMITIVES.md) §0
(`build`, `BuildError::Unsupported` for observed but underived options). Angles in
degrees in params, radians below only where stated.

## 1. Orientation (all seven, and the plane)

Enum `orientation`: `pos_x, neg_x, pos_y (default), neg_y, pos_z, neg_z`. The object is
built in its `pos_y` frame and mapped (rotations; the plane's six cases and
`cylinder/orientation-*`, `cone/…`, `pyramid/…` agree):

| key | (x, y, z) → |
|---|---|
| `pos_y` | (x, y, z) |
| `neg_y` | (−x, −y, z) |
| `pos_x` | (y, −x, z) |
| `neg_x` | (−y, x, z) |
| `pos_z` | (x, −z, y) |
| `neg_z` | (x, z, −y) |

`neg_x`, `neg_y`, `pos_z` are inferred from the plane (**parity-pending** for 3D objects).

## 2. Lathe (cylinder, cone, disc, torus, tube, capsule)

A **profile** of points `(r, y)` in the XY half-plane, swept around Y; optionally a
**south** centre `(0, y_s)` before the profile and a **north** centre `(0, y_n)` after it,
and optionally **closed** (the last point connects back to the first).
- **Columns:** `n = rotation segments`; without slice `n` columns at `φ_k = 360°·k/n`,
  with slice `n + 1` columns at `φ_k = from + (to − from)·k/n`. A profile point
  becomes `(r cos φ, y, r sin φ)` (the first column at +X, the next towards +Z).
- **Vertices:** south, north (those present), then the columns in order, each its
  profile points in order. A point **on the axis** (`r = 0`) is stored only in column 0;
  later columns reuse it (`tube` probe with inner radius 0, cone apex).
- **Faces**, per column pair `k → k+1` (wrapping without slice): the south triangle
  `[S, c(k,0), c(k+1,0)]`; the quads `[c(k,j), c(k,j+1), c(k+1,j+1), c(k+1,j)]` along the
  profile (closing back to j = 0 when closed); the north triangle
  `[c(k,m), N, c(k+1,m)]`. Quads touching the axis keep their repeated vertex
  (degenerate quads, as exported).
- **Slice cut faces** (solid shapes only; not the disc), after all other faces: the cut
  polygon `Q` = south, the profile, north (those present) at column 0 (start) and at
  column n (end). For `|Q| = 4` (`cylinder/slice`, `tube/slice`): start
  `[Q0, Q3, Q1], [Q1, Q3, Q2]`, end `[Q0, Q1, Q3], [Q1, Q2, Q3]`. Larger cut polygons are
  triangulated by the reference with a method not derived here; ours ear-clips the
  start as reversed `Q` and the end as `Q` (forge_geometry::triangulate_poly), and the
  parity test compares those cases' cut faces **geometrically** (§10).
- `regular_grid = true` → `Unsupported` (not observed).

## 3. Cylinder (`forge.cylinder`)

Params: `radius` 50, `height` 200, `height_segments` 1 (1–1000), `rotation_segments`
36 (3–1000), `orientation`, `caps` true, `cap_segments` 1 (1–1000), `fillet` false,
`fillet_segments` 5 (1–1000), `fillet_radius` 20, `slice` false, `slice_from` 0,
`slice_to` 360, `regular_grid` false, `regular_grid_width` 10.
The cone profile (§4) with `R_b = R_t = radius`, both fillets on when `fillet`, each
with radius = height = `clamp(fillet_radius, 0, min(radius, height/2))` (observed:
clamped to the radius).

## 4. Cone (`forge.cone`)

Params: `top_radius` 0, `bottom_radius` 100, `height` 200, `height_segments` 8,
`rotation_segments` 36, `orientation`, `caps` true, `cap_segments` **3**,
`fillet_segments` 5, `fillet_top` / `fillet_bottom` false, `fillet_top_radius`,
`fillet_top_height`, `fillet_bottom_radius`, `fillet_bottom_height` 50, slice params.
With `H = height/2`, `cs` cap segments, `fs` fillet segments, `hs` height segments,
`s = (R_t − R_b)/height` (side slope dr/dy), bottom fillet `(f_b, g_b)` = (radius
clamped to `R_b`, height clamped to `H`), top `(f_t, g_t)` likewise with `R_t`
(observed: the top fillet radius reads 30 with top radius 30), zero when off:
- **Bottom:** with caps, rings `((R_b − f_b)·i/cs, −H)` for i = 1..cs; without caps the
  rim `(R_b, −H)`. Bottom fillet: `θ = t·90°/fs`, t = 1..fs:
  `(R_b − f_b + f_b sin θ + s·g_b(1 − cos θ), −H + g_b(1 − cos θ))` (a quarter ellipse
  sheared along the side; `cone/fillet-bottom`).
- **Side:** `(R_b + s(y + H), y)` at `y = (−H + g_b) + (2H − g_b − g_t)·j/hs`, j = 1..hs.
- **Top fillet:** t = fs−1..0, `(R_t − f_t + f_t sin θ − s·g_t(1 − cos θ), H − g_t(1 − cos θ))`.
  Top cap rings `((R_t − f_t)·i/cs, H)` for i = cs−1..1.
- **Centres:** south `(0, −H)` with caps; north `(0, H)` with caps and `R_t > 0`.
  Profile points on the axis at a present centre's height are that centre (dropped).
  With `R_t = 0` the apex is an axis point (§2: stored once, degenerate quads).

## 5. Disc (`forge.disc`)

Params: `inner_radius` 0, `outer_radius` 100, `disc_segments` 4, `rotation_segments`
36, `orientation`, `slice`, `slice_from`, `slice_to` (no regular grid). Profile at y = 0
from outside in: `r_i = R_o − (R_o − R_i)·i/ds`, i = 0..ds; with `R_i = 0` the last point
is the **north** centre instead (triangle fan). No cut faces.

## 6. Torus (`forge.torus`)

Params: `ring_radius` 200, `ring_segments` 36 (the rotation segments), `pipe_radius`
50, `pipe_segments` 18, `orientation`, slice params. Closed profile
`(R + r cos θ, r sin θ)`, `θ = 360°·j/ps`, j = 0..ps−1 (outer equator first, then up).

## 7. Tube (`forge.tube`)

Params: `inner_radius` 50, `outer_radius` 200, `rotation_segments` 36, `cap_segments`
1, `height` 100, `height_segments` 1, `orientation`, `fillet` false, `fillet_segments`
**8** (from `tube/fillet`: 36 profile points = 4 × 9), `fillet_radius` 20, slice params.
Closed profile, sides in order bottom (inner → outer, `cap_segments` steps), outer
(bottom → top, `height_segments`), top (outer → inner), inner (top → bottom), each
shortened by the fillet `f = clamp(fillet_radius, 0, min((R_o − R_i)/2, height/2))`
(the clamp is **parity-pending**). Per side: its points `i = 0..steps−1` from the side's
start, then the end corner's arc `t = 0..fs−1` (`θ = t·90°/fs` from the side direction
towards the next side's, centre inset by f); without fillet the corner alone. The
profile thus starts at the inner-bottom corner (the fillet cube's ring rule,
PRIMITIVES.md §2.1).

## 8. Capsule (`forge.capsule`)

Params: `radius` 50, `height` 200 (total), `height_segments` 8, `cap_segments` 8,
`rotation_segments` 36, `orientation`, slice params. With `c = height/2 − radius`:
south `(0, −height/2)`, north `(0, height/2)`; profile: `(r sin θ, −c − r cos θ)` for
`θ = k·90°/cs`, k = 1..cs; `(r, −c + 2c·j/hs)` for j = 1..hs−1; `(r sin θ, c + r cos θ)`
for k = cs..1.

## 9. Pyramid (`forge.pyramid`)

Params: `size_x`, `size_y`, `size_z` 200, `segments` 1, `orientation`. Base corners
`(−x, −y, −z), (x, −y, −z), (x, −y, z), (−x, −y, z)` (half sizes), apex `(0, y, 0)`; faces
`[0,1,2,3]` (base quad) and the triangles `(4,2,1), (4,3,2), (4,0,3), (4,1,0)` as
`(A, B, C)`. Subdivided flat at `n = segments` like the platonic sphere types
(PRIMITIVES.md §3.1) without projection:
- Vertices: corners and apex; `n − 1` points per edge, edges in first-appearance order
  over the faces (each face's cycle), each walked from its lower-index vertex; then per
  face its interior points: the quad `b(i,k) = c0 + (c1−c0)·i/n + (c3−c0)·k/n`,
  i = 1..n−1 outer, k inner; each triangle as PRIMITIVES.md §3.1.
- Faces: the quad grid `[b(i,k), b(i+1,k), b(i+1,k+1), b(i,k+1)]`, i outer, k inner;
  then each triangle's rows (PRIMITIVES.md §3.1).

## 10. Parity tests

`crates/generators/tests/parity.rs` covers these features too. Cases in its
`CUT_GEOMETRIC` list (`cone/slice`, `torus/slice`, `capsule/slice`) must match every
vertex and every face exactly except a trailing run of cut triangles, and the whole
mesh must pass `compare_geometric`.
