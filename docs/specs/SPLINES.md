# Editable splines (`forge.spline`; `crates/generators/src/editable_spline.rs`)

Sources: own design (brief docs/agents/tasks/028-editable-spline.md) and the observation
[spline-tools.md](../parity/spline-tools.md) (O46), cases in `tests/golden/spline-tools`
(each is an Extrude, Offset 20, caps off, of the spline; measured through the same
harness as [splines-2.md](../parity/splines-2.md)). Rules say which case proves them.
Values: CORE.md §1; spline primitives and Intermediate Points: GENERATORS2.md §6.
Spline tools, booleans and Cross Section belong to brief 029 and are not specified here.

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
its piece is 25.13 long and its handle a third of it, 8.37). The constant is
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
to the case's tolerance). The other cases of the folder are listed by prefix and name as
owned elsewhere (`interp-f-*`, `prim-f-*` follow-ups of briefs 037/038; `chamfer`,
`outline*`, `union`, `subtract`, `intersect`, `and`, `or`, `reverse`, `subdivide`: brief
029) and the test asserts that no case is left unclaimed, so a new observation shows up.

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

## 6. Viewport (not part of this brief's parity)

Splines are not drawn by the viewport yet (the primitives are invisible too). Drawing the
control points, handles and Points-mode dragging is left for a follow-up; point editing is
available through `spline.edit_points` (CLI, RPC, MCP, scripts).

## 7. Parity-pending and unsupported

- Interpolation types other than Bezier/Linear under any Intermediate Points mode but None
  (no case samples them); Cubic's definition (Catmull-Rom) is a guess.
- Tangent handles of other primitives (Cogwheel `prim-f-cog-tooth`, Notes) were not
  compared: the form gives them, no test consumes them (follow-up below).
- The `0.75` tangent read-back factor is shown by two cases (circle, helix). The helix
  read-back (`interp-f/helix-handles`, form only) agrees only to a relative 1e-3: our helix
  handle is a third of the piece length, the app's is 0.07 % shorter (6.278974 against
  6.283081 at point 0), the same helix deviation as `helix-uniform` (GENERATORS2.md §6.1).
  That test (`editable_helix_points_and_tangents_match_the_notes`) is a unit check with its
  own relative tolerance, not a mesh parity case.
- Make Editable of a reversed spline is `Unsupported` (reversal itself is a brief-029 tool).
