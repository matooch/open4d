# Phase 2a — geometry foundations (`crates/geometry`, package `forge-geometry`)

Sources: published literature only (allowed by docs/CLEANROOM.md). Each section names
its references. No reference-app observation is used. Where the reference app's
exact behaviour matters (normal weighting, spline variants, selection growth, polygon
winding), this spec picks a documented standard and marks it **parity-pending**; a
later docs/parity/ observation may change the default but not the math.

Deferred (need observations or golden data): subdivision surfaces, booleans,
transforms (docs/specs/CORE.md).

Math: `glam::DVec3` (f64). All algorithms are deterministic: no hashing-order
dependence, outputs sorted or in input order.

## 1. Polygon mesh (`mesh`)

`PolyMesh { points: Vec<DVec3>, polys: Vec<Vec<u32>> }`: n-gons, vertex indices into
`points`, any polygon size ≥ 3.

- `validate() -> Result<(), MeshError>`: `IndexOutOfRange { poly, index }`,
  `TooFewVertices { poly }`, `RepeatedVertex { poly, vertex }` (a vertex twice in one
  polygon).
- Edges are unordered vertex pairs `Edge(a, b)` with `a < b`. `edges()` lists unique
  edges in first-appearance order walking polygons in order and each polygon's
  corners in order (corner `i` → `i+1`, wrapping). Edge ids are indices into it.
- Winding: counter-clockwise when viewed from the side the normal points to
  (right-hand rule). **Parity-pending**: the reference app's export winding is
  handled by the harness `[import] flip_z`.

## 2. Half-edge connectivity (`halfedge`)

Refs: M. Mäntylä, *An Introduction to Solid Modeling* (1988); L. Kettner, "Using
generic programming for designing a data structure for polyhedral surfaces",
CGTA 13 (1999).

`HalfEdgeMesh::build(&PolyMesh) -> Result<HalfEdgeMesh, TopologyError>` (validates
the mesh first: `TopologyError::Mesh(MeshError)`).
- One half-edge per polygon corner: half-edge id = running corner index (polygon
  order, then corner order). Fields: `origin`, `next`, `prev`, `face`,
  `twin: Option<HeId>`.
- `NonManifoldEdge(Edge)` if more than two polygons use an edge.
  `InconsistentOrientation(Edge)` if two polygons traverse an edge in the same
  direction.
- Queries: `dest(he)`, `face_half_edges(f)`, `is_boundary_edge(he)` (no twin),
  `is_closed()`, `boundary_loops()` (each loop ordered along its boundary
  half-edges, i.e. following the faces' winding; each loop starts at its smallest
  vertex; loops sorted lexicographically). The next boundary half-edge after `h` is
  found by rotating around `dest(h)`: start at `next(h)`, while it has a twin go to
  `next(twin)` — this keeps the walk inside one fan at non-manifold vertices, `vertex_neighbors(v)` (sorted,
  unique), `vertex_faces(v)` (sorted, unique), `face_neighbors(f)` (faces sharing an
  edge, sorted), `euler_characteristic()` = V − E + F where V counts only vertices
  used by a polygon.
- Non-manifold *vertices* (e.g. two cones touching at a tip) are allowed; vertex
  queries use stored per-vertex incidence lists, not fan rotation.

## 3. Normals (`normals`)

Refs: M. Newell's method as presented by F. Tampieri, "Newell's Method for Computing
the Plane Equation of a Polygon", *Graphics Gems III* (1992); G. Thürmer,
C. Wüthrich, "Computing Vertex Normals from Polygonal Facets", JGT 3(1) (1998);
H. Gouraud, "Continuous Shading of Curved Surfaces", IEEE TC (1971).

- `poly_normal(points, poly)`: Newell's method, normalized; zero vector for
  degenerate polygons (length < 1e-12). `poly_area` = |Newell vector| / 2.
- `vertex_normals(mesh, Weighting) -> Vec<DVec3>`: sum of weighted polygon normals
  at each vertex, normalized; zero for unused vertices.
  `Weighting::Uniform` (Gouraud), `Area`, `Angle` (Thürmer–Wüthrich: corner angle).
  Default `Angle`. **Parity-pending.**
- `corner_normals(mesh, smoothing_angle, Weighting) -> Vec<Vec<DVec3>>` (per polygon
  corner): the corner of polygon `f` at vertex `v` averages, with the given
  weighting, the normals of every polygon `g` at `v` whose normal is within
  `smoothing_angle` (radians, inclusive) of `f`'s normal; `f` itself always counts.
  `smoothing_angle ≥ π` equals `vertex_normals`; `0` gives flat (poly) normals for
  non-coplanar neighbours. **Parity-pending** (pairwise rule, not fan regions).

## 4. Selections (`selection`)

`IndexSet`: sorted set of `u32` (deterministic iteration). `new`, `from_iter`,
`insert`, `remove`, `contains`, `len`, `is_empty`, `iter` (ascending), `union`,
`intersection`, `difference`, `invert(count)` (complement within `0..count`).

`Selection { points, edges, polys }` with conversions (all pure functions of mesh +
input set, using edge ids from `PolyMesh::edges()`):

| Function | Result |
|---|---|
| `polys_to_points(mesh, polys)` | every corner vertex of the polygons |
| `polys_to_edges(mesh, polys)` | every edge of the polygons |
| `points_to_polys(mesh, points, Mode)` | `Mode::All`: polygons with all corners selected; `Mode::Any`: with at least one |
| `points_to_edges(mesh, points, Mode)` | edges with both (`All`) / either (`Any`) endpoints selected |
| `edges_to_points(mesh, edges)` | edge endpoints |
| `grow_points(mesh, points)` | adds edge-neighbours of selected points |
| `shrink_points(mesh, points)` | keeps points whose edge-neighbours are all selected |
| `grow_polys(mesh, polys)` | adds polygons sharing an **edge** with a selected one |
| `shrink_polys(mesh, polys)` | keeps polygons whose edge-neighbours are all selected |

Grow/shrink rules are **parity-pending**. Out-of-range input indices are ignored.

## 5. Splines (`spline`)

Refs: G. Farin, *Curves and Surfaces for CAGD* (5th ed., 2002) — Bézier/de Casteljau,
Hermite form; R. Bartels, J. Beatty, B. Barsky, *An Introduction to Splines for Use in
Computer Graphics and Geometric Modeling* (1987) — uniform cubic B-spline basis;
E. Catmull, R. Rom, "A Class of Local Interpolating Splines" (1974); H. Akima, "A New
Method of Interpolation and Smooth Curve Fitting Based on Local Procedures", JACM
17(4) (1970).

`Spline { kind, points: Vec<DVec3>, handles: Vec<[DVec3; 2]>, closed }`.
Constructors: `Spline::new(kind, points, closed)` for non-Bézier kinds,
`Spline::bezier(points, handles, closed)`; error `SplineError::TooFewPoints` (< 2),
`HandleCount` (Bézier handles ≠ points), `NotBezier`/`IsBezier` for the wrong
constructor.

Segments: open `n − 1`, closed `n` (last segment joins `p[n−1]` to `p[0]`).
Each segment has local parameter `t ∈ [0, 1]`. `eval(seg, t)`, `derivative(seg, t)`.

| Kind | Segment `i` (points `p`, wrapping when closed) |
|---|---|
| `Linear` | `lerp(p[i], p[i+1], t)` |
| `Bezier` | cubic Bézier `p[i]`, `p[i] + out[i]`, `p[i+1] + in[i+1]`, `p[i+1]`; handles are offsets `[in, out]` |
| `CatmullRom` | uniform Catmull–Rom (Hermite with tangents `(p[i+1] − p[i−1]) / 2`); open ends use phantom points `p[−1] = 2p[0] − p[1]`, `p[n] = 2p[n−1] − p[n−2]` |
| `BSpline` | uniform cubic B-spline over control points `p[i−1], p[i], p[i+1], p[i+2]`; open ends use reflected phantom points (`p[−1] = 2p[0] − p[1]`, `p[n] = 2p[n−1] − p[n−2]`), which makes the curve start/end exactly at `p[0]`/`p[n−1]`. Approximating otherwise |
| `Akima` | Hermite with Akima tangents per coordinate, unit parameter spacing; open ends extrapolate slopes (`m[−1] = 2m[0] − m[1]`, `m[−2] = 2m[−1] − m[0]`, mirrored at the end); equal-weight case uses the mean of the two neighbouring slopes. With 2 points it equals `Linear` |

Variant choices (uniform vs centripetal Catmull–Rom, B-spline end conditions) are
**parity-pending**.

Utilities:
- `tessellate(steps_per_segment) -> Vec<DVec3>`: `steps` samples per segment
  (`t = k/steps`, `k < steps`) plus the final end point for open splines.
- `length()`: sum over segments of a 5-point Gauss–Legendre quadrature of `|C'(t)|`
  on 16 equal subintervals.
- `sample_by_length(count) -> Vec<DVec3>`: `count ≥ 2` points equally spaced in arc
  length from start to end (closed: `count` points around the loop, not repeating
  the start), via a cumulative-length table (64 samples per segment) inverted by
  binary search and linear interpolation of the parameter.
