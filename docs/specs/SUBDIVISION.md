# Subdivision surface generator (`forge.subdivision`)

Sources: parity observation [docs/parity/subdivision.md](../parity/subdivision.md)
(reference app R20.059; golden data `tests/golden/subdivision/`); E. Catmull, J. Clark,
"Recursively generated B-spline surfaces on arbitrary topological meshes", CAD 10(6)
(1978) for the smoothing rules. Vertex and face orders were read from the exports.

## 1. Node

Category `Generator`, label "Subdivision". Parameters (ids from the observation form):

| id | label | kind | default | range |
|---|---|---|---|---|
| `level_view` | Viewport level | Int | 2 | 0–6 (**used for evaluation and export**, observed) |
| `level_render` | Render level | Int | 3 | 0–6 (stored; no renderer yet) |
| `scheme` | Scheme | Enum `catmull_clark, catmull_clark_ngons, osd_catmull_clark, osd_catmull_clark_adaptive, osd_loop, osd_bilinear` | `catmull_clark_ngons` | – |
| `subdivide_uvs` | Subdivide UVs | Enum `standard, boundary, edge` | `standard` | – (no UVs yet) |

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
| `osd_catmull_clark`, `osd_bilinear` | `Unsupported`: same counts and bbox as §2 but a different (OpenSubdiv) vertex and face numbering, shared by both (first face `[0, 74, 26, 81]` in `scheme-*`); not derived yet. `linear()` (face point = average, edge point = midpoint, vertices unchanged) is ready for bilinear once the order is known |
| `osd_catmull_clark_adaptive` | the **input, unsubdivided** (observed: the export ignores the adaptive level) |
| `osd_loop` | `Unsupported` (triangulates first; the split is not derived) |

## 4. Evaluation and display

`forge_generators::evaluate(scene, id) -> Result<Option<PolyMesh>, BuildError>`: the
local-space mesh of any generator node — primitives via `build`, the subdivision
generator from its input (§1); `Ok(None)` for nodes without geometry. The viewport and
picking use `evaluate` and skip nodes consumed by a parent subdivision generator.

## 5. Parity tests

`crates/generators/tests/subdivision_parity.rs`: for each `tests/golden/subdivision/`
case, the input is our own primitive when the case's `input` is `cube` or `plane`
(both have exact parity), else the case's `input.obj`; subdivide with the case's
`level` and `scheme`, compare with the harness (strict topology, 1e-4).
