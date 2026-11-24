# Generators III — Atom Array and Spline Mask (`crates/generators`)

Sources: observation O20 ([generators-3.md](../parity/generators-3.md)) and its cases
`tests/golden/atom-array/{cube,radii}` and `tests/golden/spline-mask/{union,subtract}`
(the Spline Mask cases are an Extrude, Offset 20, caps off, of the mask, as in
[splines-2.md](../parity/splines-2.md)). Spline booleans: SPLINES.md §8.3. Brief 041.
Both are plugin-registered generator nodes; every parameter is a data-driven descriptor.

## 1. Atom Array (`forge.atom_array`, `atom_array.rs`)

Takes the first mesh child (an input, consumed and not drawn, placed by its local transform,
as GENERATORS2.md §0). Builds a sphere at every point and a cylinder along every edge.

| id | label | kind | default | range |
|---|---|---|---|---|
| `cylinder_radius` | Cylinder Radius | float, distance | 2 | >= 0 |
| `sphere_radius` | Sphere Radius | float, distance | 5 | >= 0 |
| `subdivisions` | Subdivisions | int | 8 | 3 to 1000 (**parity-pending**: the form typed 0; below 3 is not a sphere, so the range stops at 3) |
| `single_elements` | Single Elements | bool | off | |

Rules (case in brackets):
1. **Order:** all spheres first (one per input point, in point order), then all cylinders
   (one per edge); points and faces are listed in that order, nothing is shared or welded
   (`atom-array/cube`: 8 spheres x 26 points = 208, 12 cylinders x 16 = 192, 400 points;
   8 x 32 + 12 x 8 = 352 faces).
2. **Sphere:** the Sphere primitive (PRIMITIVES.md §3, Standard type) with `radius =
   sphere_radius` and `segments = subdivisions`, moved to the point (`cube`: points 1..26 are
   the sphere at (-100, -100, -100), south pole first; `radii`: Subdivisions 6 gives 14
   points and 18 faces per sphere, 112 + 144 = 256 points).
3. **Edges:** the unique edges of the input polygons in order of first appearance, each
   directed as it first appears (`cube`: the cube's first face lists the edges P0-P1, P1-P3,
   P3-P2, P2-P0, the next faces add P3-P5, P5-P4, P4-P2, P5-P7, P7-P6, P6-P4, P7-P1, P0-P6; the
   cylinders start at the first point of each pair).
4. **Cylinder** from `a` to `b`, `n = subdivisions` sides, no caps, radius
   `cylinder_radius`: point `2k` is `a + o_k`, point `2k + 1` is `b + o_k`, with
   `o_k = r (u cos t_k + w sin t_k)`, `t_k = 2 pi k / n`, `d = unit(b - a)`,
   `u = unit(ref x d)`, `w = u x d`, `ref = +X`, or `+Y` if `d` is parallel to X. Faces
   `[2k, 2k+1, 2(k+1)+1, 2(k+1)]` (indices mod 2n) (`cube`: all 12 edges, axis-parallel in
   the six directions +-X, +-Y, +-Z; the hand of the ring is clockwise about `d`).
   **Oblique edges (settled by `atom-array/rotated-cube`, a cube rotated (30°, 45°, 0°), all 12
   edges oblique, matches exactly):** the reference axis `ref` is the world axis `d` is least
   aligned with, the first of equal ones in the order X, Y, Z (so the six axis directions keep
   `ref = +Y` for ±X and `+X` for ±Y and ±Z; an edge with `d = (0.354, 0.707, -0.612)` takes
   `+X` and one with `d = (0.866, 0, 0.5)` takes `+Y`). Ties use a tolerance of 1e-9.
5. **Single Elements** does not change the exported mesh (`atom-array/single-elements` equals
   `atom-array/cube` point for point), so it is stored and ignored. Subdivisions 5 on a Pyramid
   (`atom-array/pyramid`, 5 points, 8 edges) follows rules 1 to 4 unchanged.
6. Odd `subdivisions`: the sphere follows the Sphere primitive's rule (`ceil(n/2)` bands) and
   the cylinder has `n` sides. **Parity-pending** (cases use 8 and 6).

`Unsupported` (`BuildError::Unsupported`): a spline child.

## 2. Spline Mask (`forge.spline_mask`, `spline_mask.rs`)

A spline generator (it counts as a spline: Extrude, Lathe, Loft and Sweep take it as input).
Its children are splines; the first two, A and B, are consumed. Both must be closed and lie
at z = 0 in the mask's space (else `Failed`).

Rules (cases `spline-mask/union`, `spline-mask/subtract`; both match exactly):
1. **The mask works on the sampled outlines, not on the Bezier curves.** Each input is
   replaced by the polygon of its own samples (Intermediate Points as set on the input: 36
   points for the default Circle) and the polygons are combined with the loop rules of
   SPLINES.md §8.3 (`spline_tools::boolean` on Linear operands: crossings of polygon edges,
   even-odd in/out test, pieces chained end to start, same loop start and direction).
   Node counts prove it: `union` is 23 points of A outside B + 23 of B outside A + the 2
   crossings = 48, `subtract` 23 + 13 B points inside A + 2 = 38 (SPLINES.md §8.3 would give a
   Bezier result of 6 nodes).
2. **The result is a closed spline whose pieces are straight chords** between those nodes
   (handles on the end points, `splines::straight`) with Intermediate Points **Uniform,
   8**: `nodes * 9` points, equally spaced in arc length over the whole loop (union
   `48 * 9 = 432` per ring, 864 points; subtract `38 * 9 = 342`, 684). The reference
   parametrises each chord as `3t^2 - 2t^3` (the straight-piece rule of GENERATORS2.md §6.2);
   with linear thirds handles the samples differ by up to 1e-2, with these handles they match.
3. Extrude Direction Auto follows the signed area of the result (SPLINES.md §8.8).

| id | label | kind | default |
|---|---|---|---|
| `mode` | Mode | enum `union`, `a_minus_b`, `b_minus_a`, `and`, `or`, `intersection` | `union` |
| `axis` | Axis | enum `xy`, `zy`, `xz`, `viewport` | `xy` |
| `create_cap` | Create Cap | bool | off |

Mode to operation (SPLINES.md §8.3 names), all proven by `tests/golden/spline-mask/*`:
`union` is `Union` (`spline-mask/union`), `a_minus_b` is `Subtract` of A by B
(`spline-mask/subtract`), `and` is `And` (`spline-mask/and`, 28 nodes), `or` is `Or`
(`spline-mask/or`: two loops of 38 nodes), `intersection` is `Intersect`
(`spline-mask/intersection`: three loops, 104 nodes); each matches exactly, so the
SPLINES.md §8.3 loop sets and orders hold on the sampled outlines. `b_minus_a` is the **first
loop of `Or`** (B outside A, then A inside B reversed; `spline-mask/b-minus-a`): it starts at
the same crossing as `Or` but runs clockwise, which is also why its Extrude Direction Auto goes
to -z. It is not `Subtract` with the operands swapped.
With fewer than two spline children the mask has no curve.

Further rules from the follow-up cases:
4. **Result sampling = the first child's Intermediate Points** (type, number, angle, length),
   not always Uniform 8: a Rectangle (Adaptive) with a Circle gives the 19 nodes of the union
   with no points in between (`spline-mask/rect-circle`: 38 vertices, 19 faces); two Circles give
   Uniform 8 as before. (Hypothesis "any child Adaptive" is not separated by the cases.)
5. **A polygon that only touches the other outline is not a crossing.** The default Circle
   (Uniform, 36 samples) has a sample on the top edge of the 400 x 400 Rectangle
   (`spline-mask/rect-circle`); that tangent point does not split the loops, so the corner
   (200, 200) stays in the union and the loop starts at the first real crossing (200, 173.34).
   `spline_tools::intersections` skips a hit on a vertex whose two neighbours lie on the same
   side of the other edge.
6. **Axis ZY** (`spline-mask/axis-zy`, the circles rotated into the ZY plane): the mask works in
   the frame (u, v) = (z, y), i.e. the XY rules with x and z swapped, inputs at x = 0; the result
   is mapped back, so the Extrude (Auto) runs along -x (the plane normal u x v) for a
   counter-clockwise result: x in [-20, 0]. Matches exactly. Axis XZ and the render view stay
   `Unsupported` (no case).
7. **Create Cap on** (`spline-mask/cap`): the mask is a polygon object, the sampled outline ring
   (432 points for the union) triangulated with the constrained Delaunay rule of
   GEOMETRY.md §6.1 (`triangulate_delaunay`, the ring as wound), 430 triangles, all at z = 0; the
   surrounding Extrude contributes nothing. Points and 429 of 430 triangles match;
   **parity-pending:** triangle 13 (`[6, 7, 210]` in the export, `[6, 7, 209]` here) is a
   tie-break among the near-cocircular samples of a Bezier arc.

`Unsupported`: `axis` XZ and viewport (no case). A mask input with several loops is
`Unsupported` too.

## 3. Parity test (`crates/generators/tests/generators3_parity.rs`)

Discovers `tests/golden/atom-array` and `tests/golden/spline-mask`, builds the scene from the
case file (`[params]`, `[params.input]`, `[params.input2]`; the mask's `extrude_offset` and
`caps` belong to the wrapping Extrude), and compares with `compare_meshes` after `export_weld`.
Results are in docs/parity/STATUS.md.

## 3. Metaball (`forge.metaball`, `metaball.rs`)

Sources: observation O20 ([generators-3.md](../parity/generators-3.md) §Metaball) and its
cases `tests/golden/metaball/{two-spheres,hull-50,editor-sub}`. A generator whose sphere
and spline children are inputs (consumed, not drawn).

| id | label | kind | default | range |
|---|---|---|---|---|
| `hull_value` | Hull Value | float, percent | 100 % (1.0) | 0 .. 1000 % (0 .. 10) |
| `editor_subdiv` | Editor Subdivision | float | 40 | 0.001 .. unbounded |
| `render_subdiv` | Render Subdivision | float | 5 | 0.001 .. unbounded |
| `exponential_falloff` | Exponential Falloff | bool | off | – |
| `accurate_normals` | Accurate Normals | bool | off | – |

### 3.1 Field

1. Each **sphere child** is a point source at its placed origin with the sphere's
   `radius` and strength 1. Its contribution at distance `d` is `f = (radius / d)^2`; the
   gradient is `-2 radius^2 (p - c) / d^4`. The field is the **sum** of the sources, and
   the hull value is the iso level (`hull 100 %` = 1, `hull 50 %` = 0.5).
   Proof: on every vertex of `two-spheres` the summed field is 0.99 ± 0.04 (mean 0.99),
   and on `hull-50` it is 0.50 ± 0.01 (mean 0.496); the bbox `±175.07 / ±130.43` follows
   from the sum, not from a union of radius-100 spheres. **Parity-pending**: only radius
   100, strength 1 and hull 50/100 are observed.
2. Each **spline child** contributes the same falloff with `d` the distance to its sampled
   polyline and `radius` the spline's own `radius` parameter when it has one (else 100).
   **Parity-pending**: O20 has no spline metaball case; the radius rule is a literal
   reading of the design decision, not an observation.
3. `exponential_falloff` and `accurate_normals` are stored and change nothing in the
   observed default cases (**parity-pending**: their effect is unobserved).
4. A hull value of 0 has no finite surface, and a metaball without sphere/spline children
   has no geometry (`Ok(None)`).

### 3.2 Mesh

- The lattice is aligned at multiples of `editor_subdiv` (the golden vertex coordinates
  include exact multiples of 40 and 20) and spans the union of each source's reach
  `radius * sqrt(n / hull)`, padded by one cell. Values are sampled densely; the field is
  zero outside.
- The surface is extracted with `forge_geometry::isosurface` (the cell mesher shared with
  the volume mesher, VOLUME.md §7) with inside above the hull value. Every vertex is then
  projected onto the true iso surface with two Newton steps on the analytic field, because
  a cell mean sits inside the surface on curved parts.
- `render_subdiv` is stored; evaluation uses `editor_subdiv`, which is the spacing the
  golden meshes were exported at (**parity-pending**: the render subdivision is not
  exercised by O20).
- Results: `two-spheres` and `hull-50` pass the geometric comparison (Hausdorff 7.9 / 5.9
  below voxel/2, area -1.3 % / -0.9 %, volume -1.7 % / -1.3 %, Euler 2). `editor-sub`
  matches geometrically (Hausdorff 2.1, area -0.5 %) but its reference mesh is **not
  closed** (Euler 12, boundary edges), so its Euler characteristic cannot equal a closed
  iso-surface; pinned as `KNOWN_MISMATCH` in `metaball_parity.rs`.
