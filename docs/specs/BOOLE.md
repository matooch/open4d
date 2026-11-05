# Boolean generator (`forge.boole`)

Sources: parity observation [docs/parity/boole.md](../parity/boole.md) (reference app
R20.059; golden data `tests/golden/boole/`); W. Thibault, B. Naylor, "Set operations
on polyhedra using binary space partitioning trees", SIGGRAPH 1987 (BSP splitting of
polygons against a solid); A. Jacobson, L. Kavan, O. Sorkine-Hornung, "Robust
inside-outside segmentation using generalized winding numbers", SIGGRAPH 2013 (point
classification); A. van Oosterom, J. Strackee, "The solid angle of a plane triangle",
IEEE Trans. Biomed. Eng. 30(2) (1983); S. Hertel, K. Mehlhorn, "Fast triangulation of
simple polygons", FCT 1983 (convex decomposition); D. Eberly, "Triangulation by Ear
Clipping", Geometric Tools (2002).

The reference's vertex numbering and its split of cut faces into quads are specific to
its internal algorithm (observed: surviving vertices renumbered, new vertices
interleaved, faces starting at varying corners), so parity is **geometric**: same
vertex positions, same surface per oriented plane, same volume and closedness
(docs/specs/INFRA.md §4.2), not the strict order of `compare_meshes`.

## 1. Node

Category `Generator`, label "Boole". Parameters (ids from the observation). Defaults are the
**R25** ones ([docs/parity/r25-defaults.md](../parity/r25-defaults.md)); `mode` stays
`a_minus_b` (O4 readback; the union in r25-drift.md was the case's own `mode`), only
`hide_new_edges` moved. R20 cases get the R20 value from `tests/defaults/r20.toml` (INFRA.md §3.1).

| id | label | kind | default | effect |
|---|---|---|---|---|
| `mode` | Boolean Type | Enum `union, a_minus_b, intersect, a_without_b` | `a_minus_b` | §3 |
| `high_quality` | High Quality | Bool | `true` | off → `Unsupported` (§5) |
| `hide_new_edges` | Hide new edges | Bool | `true` (R20: `false`) | §4 |
| `single_object` | Create single object | Bool | `false` | stored (our result is always one mesh) |
| `phong_breaks` | Create Phong breaks at intersections | Bool | `false` | stored (no effect yet) |
| `select_intersections` | Select intersections | Bool | `false` | stored (no effect yet) |
| `optimize_points` | Optimize Points | Float, distance, ≥ 0.0001 (R25 clamp) | `0.01` | stored (greyed out with the defaults) |

Inputs: the generator's **first two hierarchy children** that evaluate to a mesh, A then
B, each placed by its local transform (as the subdivision input, SUBDIVISION.md §1).
Both are *consumed* (`forge_generators::is_consumed`): the viewport draws only the
result. Fewer than two inputs: one input → that input unchanged; none → no mesh.

## 2. Algorithm (`forge_geometry::boolean`)

1. **Convex pieces.** Every input polygon that is not convex is ear-clipped into
   triangles (GEOMETRY.md §6); each piece remembers its source polygon.
2. **Split.** A BSP tree is built from B's pieces (Thibault & Naylor: each node takes
   the first polygon's plane; the others go front, back, or are split). A's pieces are
   pushed down B's tree and split at every node plane they cross; a piece coplanar with
   the node plane is passed to *both* sides, so it is cut by every plane of B's
   surface that meets it. The resulting fragments contain no part of B's surface in
   their interior, so each is wholly inside, outside, or on B's surface. B's pieces are
   split by A's tree the same way.
3. **Classify** each fragment by its vertex centroid `c` against the other solid:
   *coplanar-same* / *coplanar-opposite* when `c` lies (within ε) inside a coplanar
   polygon of the other solid with the same / opposite normal; otherwise *inside* when
   the generalized winding number of the other solid at `c` exceeds ½ (sum of the
   signed solid angles of its fan triangles, van Oosterom–Strackee, over 4π), else
   *outside*.
4. **Keep** (B's fragments that are kept as *inside* are reversed):

   | mode | A's fragments | B's fragments |
   |---|---|---|
   | `union` | outside, coplanar-same | outside |
   | `a_minus_b` | outside, coplanar-opposite | inside (reversed) |
   | `intersect` | inside, coplanar-same | inside |
   | `a_without_b` | outside, coplanar-opposite | none — A's surface outside B, **left open** |

   B's coplanar fragments are never kept (A's copy decides). This matches the observed
   behaviour when B only touches A's face (A subtract B leaves the whole of A).
5. **Merge.** Points are welded (ε = 1e-9 × the inputs' bounding-box diagonal, at
   least 1e-12). The kept fragments of each source polygon are merged back into
   regions: their directed edges are split at every fragment vertex lying on them,
   opposite edge pairs cancel, and the remaining edges are traced into loops (at a
   vertex with several outgoing edges, the one turning most to the right, so
   pinched regions separate). Loops with positive area about the source normal are
   outer boundaries; negative loops are holes, assigned to the smallest outer loop
   containing them.
6. **Clean.** Every output point lying strictly inside an edge of another region is
   inserted into that edge (no T-junctions, so closed results stay watertight). Then a
   point that was **not an input point** and is collinear (turn below ε) in every loop
   it appears in is removed — the split points the BSP introduced along straight
   edges.
7. **Polygons** per region (§4), output order: regions of A's source polygons in input
   order, then B's; points ordered as A's surviving input points, B's surviving input
   points, then new points in first use.

## 3. Modes

Observed with A = default cube, B = default cube at (50, 50, 50):
`union` 20 points / 18 quads; `a_minus_b` 14 / 12 quads (new corner (−50, −50, −50));
`intersect` 8 / 6 (box (−50, −50, −50)..(100, 100, 100)); `a_without_b` A's 13 points,
9 faces, B's 3 faces missing. Every point and the surface of every face are reproduced by §2 (face counts agree
too); numbering and start corners differ.

## 4. Hide new edges

Observed: on → each cut face of A is one n-gon (three 6-gons in `hide-edges`); off →
the same region split into quads (two per L-shaped face in `mode-a_minus_b`).

- **on**: a region without holes is one polygon; a region with holes is split into
  convex pieces (holes bridged to the outer loop, ear clipping, then Hertel–Mehlhorn
  removal of every diagonal whose removal leaves both sides convex).
- **off**: a region without holes that is convex, or whose loop equals its source
  polygon, is kept as one polygon; any other region is split into convex pieces as
  above. On the observed cases this gives the reference's counts (two quads per L,
  12 faces in `mode-a_minus_b`, 18 in `mode-union`), but which diagonal is cut and the
  corner order are ours (parity-pending for other shapes; the surface is the same).

## 5. Unsupported

- `high_quality` off: the reference triangulates the result and perturbs coordinates
  by up to about 0.005 (`quality-off`); not derived → `BuildError::Unsupported`.
- The reference's point/face numbering and quad split (geometric parity only).
- Open or self-intersecting inputs give whatever the winding-number classification
  gives (no observation yet).

## 5a. R25 cases (`boole/r25-*`, O20) — brief 043

Sources: observation O20 ([docs/parity/generators-3.md](../parity/generators-3.md)) and the
golden cases `boole/r25-union`, `-a_minus_b`, `-intersect`, `-a_without_b`; Sutherland and
Hodgman, "Reentrant polygon clipping", CACM 17(1) (1974).

The case form is `[params.input]` / `[params.input2]` cube tables plus `mode`,
`hide_new_edges` (R25 default on), `high_quality`, `optimize_points` (0.01).

**Layout (derived, all four cases).** The R25 export holds two objects: A's polygons with
their own points (`Cube`), then B's polygons with their own copies of the points (`Cube_1`).
Points shared by both are duplicated: `r25-a_minus_b` is 13 + 7 = 20 points (6 + 3 faces),
`r25-union` 13 + 13 = 26 (6 + 6), `r25-intersect` 7 + 7 = 14 (3 + 3), `r25-a_without_b` 13
(6 + 0: B has no faces). `forge_generators::boole_objects` returns this layout (`boole`
stays welded for the viewport). Inside an object the points are: the surviving input points
in the input's order, then the new points. B's polygons of `a_minus_b` are reversed.
Polygon sizes (hide on) and the surface per plane match, checked by `boole_parity.rs`
(point count, polygon sizes, `compare_geometric`).

**Start corners (derived by hand on `r25-intersect` A `x+`, `z+`, B `z-` and `r25-a_minus_b` B `z-`; not implemented, because the numbering below blocks a strict match anyway).** A kept-inside polygon
is the Sutherland–Hodgman clip of its source polygon (reversed first for the B polygons of
`a_minus_b`) against the other solid's planes in that solid's polygon order, e.g.
`r25-intersect` A `x+` face `[2,3,5,4]` → `[P2, P1, c5, P3]`, B `z-` face → `[P1, P2, c0, P5]`,
and the face order is the source order. For the outside parts (`union`, `a_minus_b`,
`a_without_b` A part, `union` B part) an untouched polygon keeps its source start and a cut
polygon starts at the last new point of its cut run (`r25-union`: A `x+` face
`[P3, 4, 2, 3, P1, P2]`, B `z-` face starts at `P5`; all 6 cut faces of the union).

**Not derived (parity-pending, so the four cases stay in `KNOWN_MISMATCH`):**
1. The numbering of the new points. A part (all modes): `P1 (100,100,-50)`, `P2 (100,-50,-50)`,
   `P3 (100,-50,100)`, `P4 (-50,100,100)`, `P5 (-50,100,-50)`, `P6 (-50,-50,100)` as 7…12
   (union, a_without_b) or 1…6 after the one surviving corner (intersect); B part: `P1, P6, P2, P5,
   P3, P4`. A walk of A's faces `x+`, `y+`, `z+` reproduces the first order and nothing
   simple reproduces the second (three hypotheses tried: first use in output order, Sutherland–
   Hodgman creation order in source and in axis order, sorting by coordinates).
2. The face order of the union family: A `z-, x+, y-, z+, y+, x-` (source order is
   `z-, x+, z+, x-, y+, y-`), B (union) `z-, y-, x+, z+, x-, y+`. Plane-major and face-major
   emission orders over B's planes (every plane permutation) do not give either.
Follow-up cases proposed in the brief report: B moved along one axis only, B smaller than A
(strictly inside) and B shifted by a different amount per axis, which separate numbering from
geometry.

## 6. Parity tests

`crates/generators/tests/boole_parity.rs`: for each `tests/golden/boole/` case, A =
our default cube, B = our default cube translated by the case's `b_pos`, options from
`[params]` (or the cube tables of `[params.input]`, `[params.input2]` for the R25 cases,
§5a); compared with `compare_geometric` (the R20 cases set
`[tolerance] compare = "geometric"`; the R25 cases are strict, in the `boole_objects` layout, §5a). `boole/quality-off` is pinned as unsupported.
Unit tests (`crates/geometry/tests/boolean.rs`): all modes on overlapping, disjoint,
touching and nested boxes (volumes, closedness, point sets) and a box with a hole.
