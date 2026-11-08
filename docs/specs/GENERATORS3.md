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
   **Parity-pending:** the frame for edges not parallel to an axis, and the parallel test
   (`|d.x| > 1 - 1e-9` here), are extrapolated from the six axis directions.
5. Odd `subdivisions`: the sphere follows the Sphere primitive's rule (`ceil(n/2)` bands) and
   the cylinder has `n` sides. **Parity-pending** (cases use 8 and 6).

`Unsupported` (`BuildError::Unsupported`): `single_elements` on (no case), a spline child.

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

Mode to operation (SPLINES.md §8.3 names): `union` is `Union` (`spline-mask/union`), `a_minus_b` is `Subtract` of A by
B (`spline-mask/subtract`), `and` / `or` / `intersection` are `And` / `Or` / `Intersect`
(**parity-pending**: SPLINES.md §8.3 matched them on the Bezier curves; the mask is observed in two modes only),
`b_minus_a` is `Subtract` with the operands swapped (**parity-pending**, no case).
With fewer than two spline children the mask has no curve.

`Unsupported`: `axis` other than `xy` (no case: the other planes, and the render view),
`create_cap` on (no case). A mask input with several loops is `Unsupported` too.

## 3. Parity test (`crates/generators/tests/generators3_parity.rs`)

Discovers `tests/golden/atom-array` and `tests/golden/spline-mask`, builds the scene from the
case file (`[params]`, `[params.input]`, `[params.input2]`; the mask's `extrude_offset` and
`caps` belong to the wrapping Extrude), and compares with `compare_meshes` after `export_weld`.
Results are in docs/parity/STATUS.md.
