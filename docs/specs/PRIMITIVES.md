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

`fillet = true` and `separate_surfaces = true` → `Unsupported` (not yet derived).

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

## 3. Sphere (`forge.sphere`)

| id | label | kind | default | range |
|---|---|---|---|---|
| `radius` | Radius | Float, Distance | 100 | ≥ 0 |
| `segments` | Segments | Int | 24 | 3–1000 |
| `type` | Type | Enum `standard, tetrahedron, hexahedron, octahedron, icosahedron, hemisphere` | `standard` | – |
| `render_perfect` | Render perfect | Bool | true | – (no effect on the mesh) |

`tetrahedron`, `hexahedron`, `octahedron`, `icosahedron` → `Unsupported` (their
subdivision and vertex orders are not derived yet).

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

## 4. Parity tests

`crates/generators/tests/parity.rs`: every case under `tests/golden/{plane,cube,sphere}`
is built from its `[params]` (same keys as our param ids; enums as strings) and must
either match the export with the harness (`compare_meshes`, strict topology,
tolerance 1e-4) or return `Unsupported`. The set of unsupported cases is listed in the
test; a case changing state fails the test until the list is updated.
