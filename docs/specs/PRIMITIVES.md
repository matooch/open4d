# Primitives I — plane, cube, sphere (`crates/generators`, package `forge-generators`)

Sources: parity observations [docs/parity/plane.md](../parity/plane.md),
[docs/parity/cube.md](../parity/cube.md), [docs/parity/sphere.md](../parity/sphere.md)
(reference app R20.059; golden data `tests/golden/{plane,cube,sphere}/`), exported with
[export-settings.md](../parity/export-settings.md) (no axis flip, scale 1). Vertex and
face orders below were read from those exports and are part of parity (selections
and vertex maps index into them). Coordinates follow docs/specs/TRANSFORM.md
(the reference app's own numbers, +Y up).

Labels are our own wording; parameter ids are the observation forms' keys.

## 0. Plugin and evaluation

`forge_generators::Primitives` is a `forge_core::Plugin` registering
`forge.plane`, `forge.cube`, `forge.sphere` (category `Generator`; transform params
are prepended by the registry). `build(type_id, &params) -> Result<PolyMesh,
BuildError>` evaluates a node's params (missing params → defaults) in local space;
`BuildError::UnknownType` for other ids, `BuildError::Unsupported(&str)` for observed
options not implemented yet. Generators never return approximate geometry.

## 1. Plane (`forge.plane`)

| id | label | kind | default | range |
|---|---|---|---|---|
| `width` | Width | Float, Distance | 400 | ≥ 0 |
| `height` | Height | Float, Distance | 400 | ≥ 0 |
| `segments_w` | Width segments | Int | 20 | 1–1000 |
| `segments_h` | Height segments | Int | 20 | 1–1000 |
| `orientation` | Orientation | Enum `pos_x, neg_x, pos_y, neg_y, pos_z, neg_z` | `pos_y` | – |

- Grid `(u, v)`: `u_i = −w/2 + i·w/sw` (i = 0..sw), `v_j = −h/2 + j·h/sh` (j = 0..sh);
  vertex index `j·(sw+1) + i`.
- Quads, j outer, i inner, with `a = j·(sw+1) + i`: `[a, a+sw+1, a+sw+2, a+1]`.
- Position by orientation (from the first exported vertices of each case):
  `pos_y (u, 0, v)`, `neg_y (−u, 0, v)`, `pos_x (0, −u, v)`, `neg_x (0, u, v)`,
  `pos_z (u, −v, 0)`, `neg_z (u, v, 0)`. Face lists are identical for all orientations.

## 2. Cube (`forge.cube`)

| id | label | kind | default | range |
|---|---|---|---|---|
| `size_x`, `size_y`, `size_z` | Size X/Y/Z | Float, Distance | 200 | ≥ 0 |
| `segments_x`, `segments_y`, `segments_z` | Segments X/Y/Z | Int | 1 | 1–1000 |
| `separate_surfaces` | Separate surfaces | Bool | false | – |
| `fillet` | Fillet | Bool | false | – |
| `fillet_radius` | Fillet radius | Float, Distance | 40 | ≥ 0 (app clamps to half the smallest size) |
| `fillet_segments` | Fillet segments | Int | 5 | 1–1000 |

`separate_surfaces = true` (without fillet) → `Unsupported` (not yet derived). With
`fillet = true` it is ignored (observed: disabled in the app while Fillet is on), and
the cube follows §2.1.


With half-sizes `(hx, hy, hz)` and segments `(sx, sy, sz)`:
- **Ring:** the `n = 2(sx + sz)` perimeter points in XZ, walked from (−hx, −hz):
  along +X at z = −hz (r = i), along +Z at x = +hx (r = sx + j), along −X at z = +hz
  (r = sx + sz + (sx − i)), along −Z at x = −hx (r = 2sx + sz + (sz − j)), with
  `x_i = −hx + i·2hx/sx`, `z_j = −hz + j·2hz/sz`. Ring point r has a column of
  `sy + 1` vertices, bottom to top (`y_k = −hy + k·2hy/sy`): index `r·(sy+1) + k`.
- **Top interior** points (0 < i < sx, 0 < j < sz) follow at `n·(sy+1)`, **i outer,
  j inner** (`cube/segments-wide`: (−33, −33), (−33, 33), (33, −33), (33, 33)); then the
  **bottom interior** points the same way but with `j` counted from z = +hz
  (`z = +hz − j·2hz/sz`).
- **Side faces**, side by side (the 4 ring stretches in walk order), each with k outer
  and ring segment inner: `[c(r)+k, c(r)+k+1, c(r+1)+k+1, c(r+1)+k]`, `c(r) = r·(sy+1)`,
  r + 1 wrapping to 0.
- **Top faces**, j outer, i inner: `[p(i,j), p(i,j+1), p(i+1,j+1), p(i+1,j)]` where
  `p` is the top vertex at grid point (i, j) (ring column top for boundary points,
  top interior otherwise). **Bottom faces** the same over the bottom grid
  (j from z = +hz).

### 2.1 Fillet (observed, `cube/fillet`, `cube/fillet-segments`)

Fillet radius `f = clamp(fillet_radius, 0, min(hx, hy, hz))`, `s = fillet_segments`,
inner half-sizes `(ix, iy, iz) = (hx − f, hy − f, hz − f)`, quarter step `q = 90°/s`.
- **Sides** of the inner box in XZ, walked from (−ix, −iz): −Z side along +X (`sx`
  steps, normal −Z), +X side along +Z (`sz`, +X), +Z side along −X (`sx`, +Z), −X side
  along −Z (`sz`, −X). The `2(sx + sz)` straight points (start of each side included,
  end excluded) are the **cap boundary**, in that order.
- **Ring** (`2(sx + sz) + 4s` entries): per side, its straight points (direction = the
  side normal), then the arc of the side's end corner for `t = 0..s−1`, direction
  `n_side·cos(tq) + n_next·sin(tq)`. Each entry also names the cap boundary point it
  sits on: its own for straight points, the corner's (the next side's first) for arcs.
- **Rows** bottom to top, `(y, ρ)`: `(−iy − f cos(jq), f sin(jq))` for j = 1..s−1;
  then `(−iy + m·2iy/sy, f)` for m = 0..sy; then `(iy + f cos(jq), f sin(jq))` for
  j = s−1..1. A row's ring point is `inner + ρ·direction` at height y.
- **Vertices:** bottom cap boundary (y = −hy); the rows, each a full ring; top cap
  boundary (y = +hy); then the top and the bottom cap interior points `(x_i, ±hy, z_k)`,
  `x_i = −ix + i·2ix/sx`, `z_k = −iz + k·2iz/sz`, i = 1..sx−1 outer, k = 1..sz−1 inner
  (both caps in the same orientation: **parity-pending**, a single interior point was
  observed).
- **Faces:** per ring segment `r → r+1` (wrapping), with cap points a, b under r, r+1:
  the bottom face `[a, R₀(r), R₀(r+1), b]` (a triangle `[a, R₀(r), R₀(r+1)]` when
  `a = b`), the quads `[R_j(r), R_{j+1}(r), R_{j+1}(r+1), R_j(r+1)]` up the rows, and the
  top face `[R_L(r), a', b', R_L(r+1)]` (triangle `[R_L(r), a', R_L(r+1)]`). Then the top
  cap `[t(i,k), t(i,k+1), t(i+1,k+1), t(i+1,k)]` and the bottom cap
  `[b(i,k), b(i+1,k), b(i+1,k+1), b(i,k+1)]`, each **i outer, k inner** over the cap grid
  (boundary grid points via the cap boundary walk).


## 3. Sphere (`forge.sphere`)

| id | label | kind | default | range |
|---|---|---|---|---|
| `radius` | Radius | Float, Distance | 100 | ≥ 0 |
| `segments` | Segments | Int | 24 | 3–1000 |
| `type` | Type | Enum `standard, tetrahedron, hexahedron, octahedron, icosahedron, hemisphere` | `standard` | – |
| `render_perfect` | Render perfect | Bool | true | – (no effect on the mesh) |

The platonic types follow §3.1.

- Bands `B = ⌈segments / 2⌉`, meridians `L = 2B` (9 segments → 10 meridians, observed).
- Ring r (r = 1..B−1, bottom to top): polar angle `θ = r·π/B` from the south pole,
  `y = −R cos θ`, ring radius `R sin θ`; vertex k (k = 0..L−1) at azimuth `φ = 2πk/L`
  from +X towards +Z: `(R sin θ cos φ, y, R sin θ sin φ)`. The seam is at k = 0 (+X).
- **Standard:** vertex 0 south pole (0, −R, 0), vertex 1 north pole, then rings:
  `v(r, k) = 2 + (r−1)·L + k`. Faces: quads for r = 1..B−2, k inner:
  `[v(r,k), v(r+1,k), v(r+1,k+1), v(r,k+1)]`; then for each k the south triangle
  `[S, v(1,k), v(1,k+1)]` followed by the north triangle `[v(B−1,k+1), v(B−1,k), N]`.
- **Hemisphere** (requires even B; odd B → `Unsupported`, not observed): vertex 0 the
  north pole, then rings r = B/2..B−1 (equator first): `v(r, k) = 1 + (r − B/2)·L + k`.
  Faces: quads as above for r = B/2..B−2, then the north triangles
  `[v(B−1,k+1), v(B−1,k), N]`. Open at the equator.

### 3.1 Platonic types (observed, `sphere/type-*hedron`, `type-*hedron-segments`)

A platonic solid of circumradius R, each face cut flat into `n` steps per edge, every
new point projected onto the sphere (`normalize(p)·R`).
- **Frequency** `n = max(1, ⌊segments / d⌋)` with d = 2 (tetrahedron), 3 (hexahedron),
  4 (octahedron), 6 (icosahedron). Observed at segments 24 (n = 12, 8, 6, 4) and
  8 (n = 4, 2, 2, 1); other values follow the same rule (**parity-pending**).
- **Triangle types** (base vertices and faces `(A, B, C)`, 0-based, unit radius):
  - tetrahedron: `(−s·sin60°, −1/3, −s·cos60°)`, `(s·sin60°, −1/3, −s·cos60°)`,
    `(0, −1/3, s)`, `(0, 1, 0)` with `s = √8/3`; faces (0,1,2), (0,3,1), (1,3,2), (2,3,0).
  - octahedron: −X, −Z, +X, +Z, +Y, −Y; faces (4,1,0), (4,2,1), (4,3,2), (4,0,3),
    (5,0,1), (5,1,2), (5,2,3), (5,3,0).
  - icosahedron: `(0, a, −b), (0, −a, −b), (b, 0, −a), (b, 0, a), (0, a, b), (0, −a, b),
    (−b, 0, a), (−b, 0, −a), (a, b, 0), (−a, b, 0), (−a, −b, 0), (a, −b, 0)` with
    `a = 1/√(1+φ²)`, `b = φa`; faces (9,8,0), (8,2,0), (8,3,2), (8,4,3), (8,9,4),
    (4,9,6), (6,9,7), (9,0,7), (11,10,1), (2,11,1), (2,3,11), (3,5,11), (5,10,11),
    (5,6,10), (6,7,10), (7,1,10), (1,7,0), (2,1,0), (4,5,3), (4,6,5).
  - Vertices: the base vertices; then `n − 1` points per edge, edges in first-appearance
    order over the faces (A→B, B→C, C→A), each walked from its lower-index vertex;
    then per face its interior points `P(r, i)`, r = 2..n−1 outer, i = 1..r−1 inner,
    where `P(r, i) = A + (r−i)/n·(B−A) + i/n·(C−A)` (so `P(r,0)` lies on A→B,
    `P(r,r)` on A→C, `P(n,i)` on B→C).
  - Faces: per face, rows r = 0..n−1, i = 0..r: `[P(r,i), P(r+1,i), P(r+1,i+1)]`,
    each (for i < r) followed by `[P(r+1,i+1), P(r,i+1), P(r,i)]`.
- **Hexahedron:** the cube grid `x, y, z ∈ {−1 + 2i/n}` projected. Vertices: a side ring
  of `4n` columns walked from (−1, −1) in XZ (along +X at z = −1, +Z at x = +1, −X at
  z = +1, −Z at x = −1), each column `n + 1` points bottom to top (`c·(n+1) + j`); then
  the top interior points (y = +1), then the bottom interior points (y = −1), each
  **i outer, k inner** over x index i and z index k, with z counted from −1 on top and
  from +1 at the bottom. Faces: the 4 sides in ring order, j outer, column inner,
  `[g(c,j), g(c,j+1), g(c+1,j+1), g(c+1,j)]`; then the top and the bottom caps,
  k outer, i inner, `[t(i,k), t(i,k+1), t(i+1,k+1), t(i+1,k)]`.

## 4. Parity tests

`crates/generators/tests/parity.rs`: every case under `tests/golden/{plane,cube,sphere}`
is built from its `[params]` (same keys as our param ids; enums as strings) and must
either match the export with the harness (`compare_meshes`, strict topology,
tolerance 1e-4) or return `Unsupported`. The set of unsupported cases is listed in the
test; a case changing state fails the test until the list is updated.
