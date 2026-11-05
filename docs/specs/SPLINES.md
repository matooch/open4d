# Editable splines (`forge.spline`; `crates/generators/src/editable_spline.rs`)

Sources: own design (brief docs/agents/tasks/028-editable-spline.md) and the observation
[spline-tools.md](../parity/spline-tools.md) (O46), cases in `tests/golden/spline-tools`
(each is an Extrude, Offset 20, caps off, of the spline; measured through the same
harness as [splines-2.md](../parity/splines-2.md)). Rules say which case proves them.
Values: CORE.md §1; spline primitives and Intermediate Points: GENERATORS2.md §6.
The spline tools and booleans (brief 029) are section 8.

## 1. The editable spline object

`forge.spline`, label "Spline", `Category::Generator` (so it takes the transform params,
tags, and can be the input of Extrude, Lathe, Loft and Sweep like a spline primitive;
`is_spline` is true for it). Not in the Create menu: it comes from Make Editable (§3) or
`spline.create` (§5). It has no mesh: `evaluate` gives `None`, like every spline.

### 1.1 Params (data-driven descriptors, `editable_spline::node_type`)

| id | kind | default | meaning |
|---|---|---|---|
| `points` | `Vec3List` | empty | the points, in segment order, in the object's local space |
| `left` | `Vec3List` | empty | per point, the left (incoming) Bezier handle as an offset from the point |
| `right` | `Vec3List` | empty | per point, the right (outgoing) handle offset |
| `segment_ends` | `IndexList` | empty | cumulative point count at the end of each segment (strictly increasing, last = number of points); empty = one segment of all points |
| `closed_segments` | `IndexList` | empty | indices of the segments that are closed; empty = all open |
| `type` | enum `linear`, `cubic`, `akima`, `bspline`, `bezier` | `bezier` | interpolation type (labels Linear, Cubic, Akima, B-Spline, Bezier) |
| `intermediate`, `intermediate_number`, `intermediate_angle`, `intermediate_max_length` | as every spline | Uniform, 8, 5 deg, 5 | Intermediate Points (GENERATORS2.md §6.2), the shared descriptors of `spline_sampling::add_params` |

`left` and `right` always have as many entries as `points` (all zero for a spline without
handles). A spline is *valid* if the three lists agree in length, `segment_ends` is
consistent with them, `closed_segments` names existing segments, and every segment has at
least two points. An invalid spline is `BuildError::Failed("...")` (not a panic); the
commands of §5 never produce one.

Handles are the **geometric Bezier offsets**: a segment from point `i` to point `j` is
`[P_i, P_i + right_i, P_j + left_j, P_j]`. The reference app's public tangent read returns
**3/4 of these offsets** (`editable-circle`: radius 200 reads 83, while the circle's handle
is `83/150 · 200 = 110.667`, GENERATORS2.md §6.1; the default Helix reads 6.278974 where
its piece is 25.13 long and its handle a third of its 3-D chord, 8.3986, split into tangential
8.371965 and vertical 0.667976). The constant is
`TANGENT_READBACK = 0.75` in code, used only to compare against the observation form's
Notes (`editable_spline::tangent_readback`); the stored value is never scaled.

### 1.2 Interpolation types (`curve_of`)

A segment list of control points becomes a `Curve` (cubic Bezier pieces, GENERATORS2.md
§6.2) so the shared sampler produces the points:
- **Intermediate Points None** (any type): the output is exactly the control points joined
  by straight pieces (`interp-linear`, `interp-akima`, `interp-bspline`, `interp-bezier`:
  all four give the same 4 points `(±200, 0)`, `(0, ±200)` of the circle, 8 vertices, and
  the type has no effect). So with None the curve is a polyline of the points, even for
  B-Spline, whose curve does not pass through its control points.
- **Linear**: straight pieces (`Curve.linear`): no interior samples in any mode
  (`editable-rectangle`: Linear + Adaptive gives only the 4 corners).
- **Bezier**: the pieces above (`editable-circle`: Bezier + Uniform 8 reproduces the
  circle's 36 samples per ring, 72 vertices).
- **Cubic**: Catmull-Rom (`forge_geometry::Spline` kind `CatmullRom`), **Akima**,
  **B-Spline** (uniform cubic B-spline): the piece between two points is converted to the
  equivalent Bezier piece from the curve's value and derivative at both ends
  (`B0 = C(0)`, `B1 = B0 + C'(0)/3`, `B2 = B3 - C'(1)/3`, `B3 = C(1)`). Open ends use the
  reflected phantom points of `forge_geometry::Spline`. The handles are ignored.
  **Parity-pending** for every mode other than None: no golden case samples these curves.

Each segment of the spline is its own loop (`Curve.extra` carries the second and later
ones, as the reference reports a Ring or Cogwheel hole as extra segments).

## 2. Value and file support

`Value::Vec3List(Vec<[f64; 3]>)`, `ParamKind::Vec3List` (CORE.md §1): every component
finite; stored in the scene file as RON text, `Vec3List([(x, y, z), ...])` (additive under file version 9, CORE.md §6.8). The Attributes
panel shows it read-only ("N points"); it is changed through the commands of §5.

## 3. Make Editable and Current State to Object

`mesh.make_editable` (EDITABLE.md §1.2) on a spline primitive (circle, rectangle, helix and
every `spline_prims` type) replaces it with a `forge.spline`:
- Same place, name, `transform.*` params, tracks and tags; links to the old node are
  redirected; children move to the new object; the old node is deleted (the same steps as
  for meshes).
- `points`/`left`/`right`: each segment `[p0, c0, c1, p1]` of the built curve contributes
  `p0` with `right = c0 - p0`; the following point gets `left = c1 - p1`. A closed loop
  has as many points as segments; an open one has one more. The unused handle at an open
  end mirrors the used one (`left` of the first point is `-right`, `right` of the last is
  `-left`; the Notes' default Helix, points 0 and 100: **parity-pending**, one case,
  geometry does not use them). Splines made of straight pieces have zero handles. A loop whose segments do not chain
  (`segment[i][3] != segment[i+1][0]`) is `BuildError::Unsupported`.
- The primitive's extra loops (Ring inner circle, Cogwheel hole) become further segments.
- `type`: `linear` when the curve is made of straight pieces (rectangle, n-side without
  rounding, ...), else `bezier` (`editable-circle`, `editable-rectangle`: Type Bezier for
  the circle and Linear for the rectangle, Notes). `bezier` handles are exactly the
  primitive's, so the extrude of the editable spline equals the primitive's.
- `intermediate*`: copied from the primitive (the circle keeps Uniform 8 and the rectangle
  Adaptive: Notes "Intermediate Points raw 2 / 3").
- A reversed primitive (`reverse` on) or a plane other than XY: the plane is applied to
  the points; `reverse` is `Unsupported` (as `build_curve`).
- A `forge.spline` is already editable: its own id is returned.
- Current State to Object on a spline primitive inserts the same object after it and
  leaves the source.

Proofs: `editable-circle` (36 points per ring, Bezier + Uniform 8), `editable-rectangle`
(Linear + Adaptive, 4 points); the points and tangents of the Notes are unit tested
(`(200, 0, 0), (0, 200, 0), (-200, 0, 0), (0, -200, 0)`, all eight handle vectors of the
table read back as 83 long; the rectangle's corners `(200, 200), (-200, 200), (-200, -200),
(200, -200)` with zero handles; a Ring gives two segments of 4 points; the default Helix
gives 101 open Bezier points, positions of points 0, 1 and 100 exact to 1e-5).

## 4. Parity test (`crates/generators/tests/spline_tools_parity.rs`)

Cases of `tests/golden/spline-tools` owned by this brief: `editable-circle`,
`editable-rectangle`, `interp-linear`, `interp-akima`, `interp-bspline`, `interp-bezier`.
The scene is an Extrude (Offset 20 on Z, caps none) over the case's `[params.input]`
spline (circle if the table has `radius`, rectangle if it has `width`); the spline is made
editable with the `mesh.make_editable` code path, then `[params.settings]` is applied
(`intermediate_points` to `intermediate`, `type` to `type`), and the extruded mesh is
compared with `compare_meshes` after `export_weld` on both sides (strict faces, positions
to the case's tolerance). The tool cases (`chamfer`, `outline*`, `union`, `subtract`, `intersect`, `and`, `or`,
`reverse`, `subdivide`) are checked by the same test through section 8.9; only the
`interp-f-*` and `prim-f-*` follow-ups (briefs 037/038) are listed by prefix as owned
elsewhere, and the test asserts that no case is left unclaimed, so a new observation shows
up.

## 5. Commands (COMMANDS.md §6; one undo step each)

| id | arguments | result |
|---|---|---|
| `spline.create` | `points: [[x,y,z]]` (at least 2), `closed?` (false), `type?` (bezier), `name?`, `parent?` | `{node}` |
| `spline.edit_points` | `node`, `edits: [{index, position?, left?, right?}]` (at least one; each at least one field), `gesture?` | `{changed}` |
| `spline.insert_point` | `node`, `index` (0 ..= points), `position`, `left?`, `right?` | `{index}` |
| `spline.delete_points` | `node`, `indices` | `{deleted}` |
| `spline.set_closed` | `node`, `segment?` (0), `closed` | null |
| `spline.set_type` | `nodes?` (selection), `type` | `{changed}` |

- `node` must be a `forge.spline` (a primitive is `Failed` with "make it editable first").
  Indices out of range are `InvalidArgs`. `edit_points` with `gesture` merges the steps of
  one drag into one undo step (`transact_merged`).
- `insert_point` puts the point at `index` of the list (inside the segment that contains
  `index - 1` or, at a boundary, the earlier segment) and defaults to zero handles;
  deleting leaves a segment of fewer than two points `Failed` (delete the whole object).
- `create` writes zero handles (a Linear/cubic/Akima/B-spline needs none; for `bezier`
  the user edits them).
- No command takes a file path; the fuzz harness needs no exclusion.

## 6. Viewport

Spline objects (primitives and `forge.spline`) draw as polylines from the same sampling as the
generators; in Points mode an editable spline shows its points and Bezier handles, and they
are dragged through `spline.edit_points` with one undo step per drag (brief 190,
VIEWPORT.md section 11). `forge_generators::splines::sampled_loops(scene, id)` gives the
sampled loops (object space) the viewport draws. The handle-link rule while dragging is
**parity-pending** (GUI session G3 item 9).

## 7. Parity-pending and unsupported

- Interpolation types other than Bezier/Linear under any Intermediate Points mode but None
  (no case samples them); Cubic's definition (Catmull-Rom) is a guess.
- Tangent handles of other primitives (Cogwheel `prim-f-cog-tooth`, Notes) were not
  compared: the form gives them, no test consumes them (follow-up below).
- The `0.75` tangent read-back factor is shown by two cases (circle, helix). The helix
  read-back (`interp-f/helix-handles`, form only) agrees to the form's six digits since brief
  191: the handle is a third of the 3-D chord along the central difference (GENERATORS2.md
  §6.1). That test (`editable_helix_points_and_tangents_match_the_notes`) is a unit check
  (2e-6), not a mesh parity case.
- Make Editable of a reversed spline is `Unsupported` (reversal itself is the tool of section 8.4).

## 8. Spline tools and booleans (brief 029; `crates/generators/src/spline_tools.rs`)

Sources: O46 ([spline-tools.md](../parity/spline-tools.md)), cases `chamfer`, `outline`,
`outline-neg`, `reverse`, `subdivide`, `union`, `subtract`, `intersect`, `and`, `or` of
`tests/golden/spline-tools`; each is measured as an Extrude (Offset 20, caps off) of the
tool's result, through the same harness as section 4. The harness applies the Extrude
Direction Auto guess of `spline_sampling.rs` (negative Z when the first loop of the result
runs clockwise); the real rule belongs to brief 044. All tools are pure functions on
`SplineData` (section 1) and registered commands (section 8.7).

### 8.1 Chamfer (`chamfer`)

Params: `radius` (distance along both edges, default 10), `flat` (off), `indices` (default
all points). Each selected corner point `P` of a segment (an open segment's two ends are
not corners) is replaced by two points: `A` on the incoming edge and `B` on the outgoing
edge, both at distance `min(radius, half the edge)` from `P`, in the order `A`, `B`.
Corners are found on the chords between the points (the handles of the neighbours are not
followed: **parity-pending** for curved neighbours).
- `flat` on: `A` and `B` get zero handles (a straight cut).
- `flat` off: an arc. `A.right = dir_in * h`, `B.left = -dir_out * h` (the other two
  handles zero, so the pieces to the neighbours are straight), with
  `h = CIRCLE_KAPPA * rho * turn / 90 deg`, `rho = d / tan(turn / 2)` (`d` the distance,
  `turn` the angle between the two edges' directions), i.e. the quarter-arc handle of
  GENERATORS2.md section 6.1. The type becomes `bezier`.
Proof: `chamfer` (rectangle, radius 40, Flat off): 4 corners to 8 points
`(200,160) (160,200) (-160,200) ...`, then the shared Adaptive sampler gives the 72 points
per ring of the export (the same curve as the rounded Rectangle of splines-2). Flat on and
turn angles other than 90 degrees: **parity-pending** (no case; the arc handle is the
primitive's rule applied to any corner).

### 8.2 Create Outline (`outline`) - known mismatch

Param `distance` (default 10). Every segment must be closed (an open one is `Failed`:
**parity-pending**). The result has the original segments and, for each, an offset
segment; the offset segment is the offset curve **reversed as a list** (last point
first); order: `distance > 0`: offset segments, then originals; `distance < 0`: originals,
then offset segments (`outline`: first loop `(0,-220.0026)` clockwise, then the circle;
`outline-neg`: the circle, then `(0,-179.9974)` clockwise).
Offset curve: every point moves by `distance` to the right of the direction of travel
(outward for a counter-clockwise loop) along the normal; at a corner (tangents differing)
the move is the miter `distance / cos(half angle)` along the bisector; each handle of a
segment is scaled by `(L + distance * phi) / L` (`L` the segment's length measured on 50
chords, `phi` the signed turn from start to end tangent), straight pieces keep their
zero/third handles. Type `bezier`, Intermediate Points unchanged.
**Known mismatch** (`outline`, `outline-neg`): the structure (2 loops, 36 samples each,
order, direction, face lists) matches, positions are within 3.7e-3 (the control points are
0.0026 short of the export's). The export is the input
circle scaled by exactly `1.100013035` about the centre (`0.899986965` for `-20`): its
control points sit at radius `220.002607`/`179.997393` instead of `220`/`180` and the
handles scale by the same factor (the sampled radii are `1.100013 * r_input`, constant to
1e-6). Three rules tried and rejected: miter offset of the 36-point or Adaptive polyline
(gives 220.07 / 220.02), Tiller-Hanson offset of the control polygon (handles 118.95 rather
than 121.73), curvature based scale factors (`mean k`, arc length ratio: R0 = 200.06 /
200.09 where the export needs R0 = 199.974). Follow-up case suggestion: Create Outline
of a Rectangle (Linear, miter corners), of a circle with radius 100 and distance 10 (does
the excess `0.0026` scale with the distance or the radius?) and of an open Arc.

### 8.3 Spline booleans

`spline.boolean` with `op` = `union`, `and`, `or`, `subtract`, `intersect` on two closed
splines `a` (the circle at the origin of the cases) and `b` (at x 150), each with one
segment. Both are brought into `a`'s local space (`z` must be 0 there, else `Failed`).
Rules, all from the five cases (they all show the same two circles):
1. **Intersections are found on the sampled outlines.** Each input is sampled with its own
   Intermediate Points (`spline_sampling::sample_params`, which also returns the global
   parameter of every sample); outline edges are intersected pairwise. `union`: the two
   points are `(75, +-184.9824)`, the chord crossings of the 36-point Uniform outlines (not
   the points of the Bezier curves, which are at `y = +-185.52`).
2. **The curves are split, not the outline.** An intersection on the outline edge from
   sample `s0` to `s1` at fraction `f` lies at global parameter `s0 + f (s1 - s0)` of its
   curve (`union`: 0.7593439 for the upper intersection; the export's piece from that
   point to the circle's top is exactly the de Casteljau piece at t = 0.75934 with its
   start point moved onto the crossing: fitted handle length 25.0447, direction 157.956
   degrees, end handle 26.633 against 25.04507 / 157.9555 / 26.63304 computed). So a
   piece keeps the **handle offsets** of the exact split and the new point's position is
   the crossing.
3. Pieces between consecutive split points are in or out of the other curve (even-odd test
   of the middle of the piece against the other's outline). Whole loops without any
   crossing are one piece each.
4. Regions, with `a_in/a_out` the pieces of a, `b_in/b_out` those of b, `+` a piece
   forward and `-` reversed (swapping its handles):
   | op | loops |
   |---|---|
   | `union` | `a_out+ b_out+` |
   | `and` | `a_in+ b_in+` |
   | `subtract` (a minus b) | `a_out+ b_in-` |
   | `or` | `b_out- a_in+`, then `a_out+ b_in-` (B minus A, A minus B; the lens is left out) |
   | `intersect` | `b_out- a_in+`, then `a_in+ b_in+`, then `a_out+ b_in-` (all three regions) |
   Pieces are chained end to start through their split points. A loop that contains a
   reversed piece starts with its first reversed piece, other loops with their first piece
   in a's order (`subtract` starts at the lower crossing with the reversed b piece,
   `union` at the upper one with a's piece; `and` at the lower one).
5. Joined pieces share a point whose left handle comes from the piece before and whose
   right handle from the piece after. A loop is a closed segment of the result; the
   result is one spline of all the loops, type `bezier` (`linear` with zero handles if
   both inputs are Linear), Intermediate Points **Adaptive** with the default 5 degrees
   (the export's samples of whole Bezier pieces are the Adaptive samples of the circle,
   not the inputs' Uniform ones).
Counts: `union` 88 points per ring, `subtract` 70, `and` 52, `or` 70 + 70, `intersect`
70 + 52 + 70 (192). Parity-pending: more than two crossings, tangent touches, inputs with
several segments (`Failed`), the rule for the loop order of `or`/`intersect` (it is the order
B minus A, both, A minus B in all cases seen) and the loop orientations (B minus A runs
clockwise in the export).

### 8.4 Reverse (`reverse`)

Each segment's points are listed in reverse order and the left and right handles swap;
segment order and closed flags are unchanged (`reverse`: the rectangle `(200,-200),
(-200,-200), (-200,200), (200,200)`; the extrude goes to -Z by Direction Auto).

### 8.5 Subdivide (`subdivide`)

Param `subdivisions` N (default 1, at least 1): every span between two neighbouring points
(including the closing span of a closed segment) is split into N equal parts by N - 1 new
points at parameters `k/N` (`subdivide`, N = 2: one midpoint per edge, 4 to 8 points,
`(200,200) (0,200) (-200,200) ...`). The reference's meaning of the count (N parts, or
`2^(N-1)` parts) is **parity-pending**: both give one midpoint for 2. The new points keep
the curve: Bezier spans are split by de Casteljau, Linear and B-Spline spans interpolate
the points, Cubic and Akima spans evaluate their curve (new points get zero handles).

### 8.6 Cross Section and GUI-only parts

Cross Section is not scriptable in the reference (observation Notes, `cross-section`
skipped) and no case exists: **not implemented**; follow-up needs an observation through
the GUI. Interactive point selection for the tools is the `indices` argument.

### 8.7 Commands (COMMANDS.md section 6; one undo step each)

| id | arguments | result |
|---|---|---|
| `spline.chamfer` | `node`, `radius?` (10), `flat?` (false), `indices?` | `{points}` |
| `spline.outline` | `node`, `distance?` (10) | `{segments}` |
| `spline.boolean` | `op`, `a`, `b`, `keep_inputs?` (false) | `{node}` |
| `spline.reverse` | `node` | `{points}` |
| `spline.subdivide` | `node`, `subdivisions?` (1) | `{points}` |

`node`, `a` and `b` must be `forge.spline` objects (else `Failed`: make it editable first).
The first four rewrite the spline's data params in place (and `type` for a non-flat
chamfer); `boolean` creates the result next to `a` (same parent and transform), deletes the
inputs unless `keep_inputs`, selects the result.

### 8.8 Extrude winding of several loops

The tools' results are the first splines with several loops that run either way, which
settled the Extrude's face winding (GENERATORS2.md section 6.1): outward of the material
per loop, see there. The Direction Auto guess of the test is the sign of the **total**
signed area of all loops (a total of zero follows the first loop): `outline` and `or` go to
-Z (`or`: the two crescents cancel, so the first loop, clockwise, decides), `intersect` goes
to +Z (the crescents cancel again and the counter-clockwise lens decides). Brief 044 owns
the real rule.

### 8.9 Parity test (`crates/generators/tests/spline_tools_parity.rs`)

Section 4's harness extended: for each tool case the editable inputs are built with
`mesh.make_editable`, the tool function is applied, the result extruded (Direction Auto
guess) and compared with `compare_meshes` after `export_weld`. Counts per tool are in the
STATUS row.
