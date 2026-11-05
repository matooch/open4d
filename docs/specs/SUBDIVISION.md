# Subdivision surface generator (`forge.subdivision`)

Sources: parity observation [docs/parity/subdivision.md](../parity/subdivision.md)
(reference app R20.059; golden data `tests/golden/subdivision/`); E. Catmull, J. Clark,
"Recursively generated B-spline surfaces on arbitrary topological meshes", CAD 10(6)
(1978) for the smoothing rules. Vertex and face orders were read from the exports.

## 1. Node

Category `Generator`, label "Subdivision". Parameters (ids from the observation form).
Defaults are the **R25** ones ([docs/parity/r25-defaults.md](../parity/r25-defaults.md)); R20
cases get the R20 values from `tests/defaults/r20.toml` (INFRA.md §3.1).

| id | label | kind | default | range |
|---|---|---|---|---|
| `level_view` | Viewport level | Int | 2 | 0–6 (**used for evaluation and export**, observed) |
| `level_render` | Render level | Int | 2 (R20: 3) | 0–6 (stored; no renderer yet) |
| `scheme` | Scheme | Enum `catmull_clark, catmull_clark_ngons, osd_catmull_clark, osd_catmull_clark_adaptive, osd_loop, osd_bilinear` | `catmull_clark_ngons` | – |
| `subdivide_uvs` | Subdivide UVs | Enum `standard, boundary, edge` | `edge` (R20: `standard`) | – (no UVs yet, so no effect on the mesh) |

Input: the generator's **first hierarchy child** that evaluates to a mesh, placed by the
child's local transform. The input child is *consumed*: the viewport draws only the
generator's result (§4). No input → no mesh.

## 2. Catmull–Clark step (`forge_geometry::subdivide::catmull_clark`)

Positions (Catmull–Clark 1978), for a mesh of n-gons:
- face point `F` = average of the face's vertices;
- interior edge point = average of its two endpoints and the two adjacent face points;
  boundary edge (one face) → edge midpoint;
- interior vertex of valence `n` → `(Q + 2R + (n − 3)·v) / n` (`Q` = average of adjacent
  face points, `R` = average of adjacent edge midpoints); boundary vertex with exactly
  two boundary edges → `(6v + a + b) / 8` (`a`, `b` the boundary neighbours) — this
  includes a corner used by a single face (`plane-1`: corner (−200, 0, −200) →
  (−197.5, 0, −197.5)); the cylinder rim confirms the rule on curved boundaries.

Sharp (crease) edges (H. Hoppe et al., "Piecewise smooth surface reconstruction",
SIGGRAPH 1994): an optional set of input edges treated like boundary edges — edge
point = midpoint; a vertex with exactly two sharp/boundary edges uses the boundary
rule; three or more → stays put; one → the interior rule. Each sharp edge's two halves
stay sharp at the next level. `subdivide_levels_with_creases(input, L, smooth, creases)`.
Observed need (`ngon-1`): the reference cylinder's caps are separate surfaces, so its
rim follows the boundary rule — rim vertex (50, −100, 0) → (49.81, −100, 0) =
`(6·50 + 2·50·cos 10°) / 8` — while the export welds the rim; the parity test
reconstructs this by marking the rim edges sharp.

Order (observed, `cube-1`, `tris-1`):
- new vertices: all original vertices first (same order); then, **face by face**, the
  face point followed by the edge points of that face's edges `(c_i, c_{i+1})` in corner
  order that were not created yet;
- new faces, face by face, for corners `k = 1, 2, …, n−1, 0` of a face with corners
  `c_k`: `[c_k, E(c_k, c_{k+1}), F, E(c_{k−1}, c_k)]` — every n-gon becomes n quads.
- **Level L ≥ 2** is *not* the level-1 order applied again (observed in `cube-2`, `cube-3`).
  Positions are those of L repeated Catmull–Clark steps; the order is defined on the
  **input** mesh. With `m = 2^(L−1)`, each input face `f` (corners `c_k`) is split into
  one patch per corner, in patch order `k = 1, …, n−1, 0`; patch `k` is an `m × m` grid
  `g(i, j)` with `g(0,0) = c_k`, `g(m,0) = E(c_k, c_{k+1})`, `g(m,m) = F`,
  `g(0,m) = E(c_{k−1}, c_k)` (i along `c_k → E(c_k,c_{k+1})`, j along
  `c_k → E(c_{k−1},c_k)`); its row `j = m` is patch `k−1`'s column `i = m`.
  Vertices: (1) the level-1 vertices in level-1 order; (2) for every input edge in
  first-appearance order `(a = c_i, b = c_{i+1})`, its new points half by half: the half
  next to `b` walking from `b` towards the midpoint, then the half next to `a` walking
  from `a` towards the midpoint (`2m − 2` points; `cube-3`); (3) per face, per patch, the points
  `g(i, j)` for `j = 1..m−1` (outer), `i = 1..m` (inner). Faces: per face, per patch,
  `[g(i,j), g(i+1,j), g(i+1,j+1), g(i,j+1)]` for `j = 0..m−1` (outer), `i = 0..m−1`.
  Level 0 returns the input; level 1 is the single step above.

## 3. Schemes

| scheme | result |
|---|---|
| `catmull_clark_ngons` (default), `catmull_clark` | §2 (identical on all observed cases) |
| `osd_catmull_clark` | §2 positions in the **OpenSubdiv order** (§3.1) |
| `osd_bilinear` | linear positions (face point = average, edge point = midpoint, vertices unchanged) in the OpenSubdiv order (§3.1) |
| `osd_catmull_clark_adaptive` | the **input, unsubdivided** (observed: the export ignores the adaptive level) |
| `osd_loop` | Loop subdivision in the OpenSubdiv order (§3.3); with sharp input edges `Unsupported` |

### 3.1 OpenSubdiv order (observed, `scheme-osd_bilinear`, `scheme-osd_catmull_clark`)

Applied level by level; the input edge list is the first-appearance order with the
first-seen direction `(a, b)`.
- Vertices: the previous level's vertices (same order), then one face point per face
  (face order), then one edge point per edge (edge order).
- Faces: per face, one child per corner `k = 0, …, n−1`; child `k` keeps the parent's
  orientation with its corner at position `k` (mod 4): `child[k] = c_k`,
  `child[k+1] = E(c_k, c_{k+1})`, `child[k+2] = F`, `child[k+3] = E(c_{k−1}, c_k)`.
- Edges of the new level: per face, the inner edges `(F, E(c_k, c_{k+1}))` for
  `k = 0..n−1`; then per previous edge `(a, b)` its halves `(a, m)` and `(m, b)`.
- Confirmed on an all-quad cube at level 2; the n-gon child rule (mod 4) and the inner
  edges' direction (which orders level ≥ 3) are unverified (**parity-pending**).

### 3.2 R25 types (`subdivision/r25-type-*`, O20)

The R25 Type list is `catmull_clark`, `catmull_clark_ngons` (default), `osd_catmull_clark`,
`osd_loop`, `osd_bilinear` (no adaptive); `r12_compatibility` (off) and `level_view` /
`level_render` 2 are stored. On a default cube at level 2 the first, second, third and fifth
types equal §2 / §3 exactly (`r25-type-catmull_clark`, `-catmull_clark_ngons`,
`-osd_catmull_clark`, `-osd_bilinear`: 98 points, 96 quads). `osd_loop` (98 points, 192 triangles) is §3.3 (`r25-type-osd_loop`, `scheme-osd_loop`).

### 3.3 Loop (`osd_loop`; `forge_geometry::subdivide::subdivide_loop`)

Sources: C. Loop, "Smooth subdivision surfaces based on triangles", M.Sc. thesis, Univ. of
Utah (1987); parity observations `subdivision/scheme-osd_loop` (R20) and
`subdivision/r25-type-osd_loop` (R25); the two meshes are the same.

- **Triangulation.** Every polygon `[v0, v1, …]` is fan-triangulated `(v0, v_j, v_{j+1})`
  (a cube quad `[0,1,3,2]` gives `(0,1,3)`, `(0,3,2)`). Output faces keep polygon-major order.
- **Positions** (Loop): interior edge point `3/8 (a + b) + 1/8 (c + d)` (`c`, `d` the
  vertices opposite the edge); interior vertex of valence `n`: `(1 − nβ) v + β Σ neighbours`,
  `β = (5/8 − (3/8 + cos(2π/n)/4)²) / n`; boundary edge → midpoint, boundary vertex →
  `(6v + a + b) / 8`. The cube at level 2 matches to the export's precision, including the
  unequal Z extent (181.445 against 179.091; the fan diagonals make the cube valences differ).
- **Numbering** (level by level, as §3.1): the previous vertices, then one point per edge
  (edge-list order). Each triangle `(t0, t1, t2)` with edge points `m0 = E(t0,t1)`,
  `m1 = E(t1,t2)`, `m2 = E(t2,t0)` gives four children in this order:
  `(t0, m0, m2)`, `(m0, t1, m1)`, `(m2, m1, t2)`, `(m1, m2, m0)`.
- **Edge list of level 0.** Edges are numbered by first appearance walking the triangles in
  the **internal order**: the first fan triangle of every polygon (polygon order), then the
  second fan triangles, and so on, each triangle's edges `(t0,t1)`, `(t1,t2)`, `(t2,t0)`
  with the first-seen direction. This explains why a cube quad's closing edge, the one that
  only the second triangle contains, gets a late number (`r25-type-osd_loop`: edge points 8–25,
  `(2,0)` = 22, the `y-` face last).
- **Edge list of the next level.** Per face in the internal order its three inner edges
  `(m0, m2)`, `(m0, m1)`, `(m1, m2)`, then per previous edge `(a, b)` its halves
  `(a, m)`, `(m, b)`. The next internal order is each face's four children, in internal order.
  Proven by the level-2 points 26–97 of `r25-type-osd_loop` (inner edges 26–61 in the order
  face 0, 2, 4, …, 1, 3, 5, …; halves 62–97).
- **Parity-pending:** polygons with more than four corners (fan index, then polygon, is the
  assumed internal order), non-manifold edges (midpoint), sharp edges (`Unsupported`) and the
  inner edges' direction (only matters from level 3).

## 4. Evaluation and display

`forge_generators::evaluate(scene, id) -> Result<Option<PolyMesh>, BuildError>`: the
local-space mesh of any generator node — primitives via `build`, the subdivision
generator from its input (§1); `Ok(None)` for nodes without geometry. The viewport and
picking use `evaluate` and skip nodes consumed by a parent subdivision generator.

## 5. Parity tests

`crates/generators/tests/subdivision_parity.rs`: for each `tests/golden/subdivision/`
case, the input is our own primitive when the case's `input` is `cube` or `plane`
(both have exact parity), else the case's `input.obj`; subdivide with the case's
`level` (R25 cases: `level_view`) and `scheme`, compare with the harness (strict topology,
1e-4). The input is `input = "cube"` (older cases) or an `[params.input]` cube table (R25
cases: `size`, segments, `fillet_subdivision` = our `fillet_segments`).
