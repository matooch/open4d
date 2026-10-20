# Generators II — mesh-input generators (`crates/generators`)

Sources: docs/parity/{symmetry,array,connect,instance}.md and their golden cases.
Spline primitives and the spline-input generators (extrude, lathe, loft, sweep) follow
in §6–10 (docs/parity/splines.md: splines do not export).

## 0. Inputs and the exporter's weld

- A generator's **inputs** are its hierarchy children that evaluate to a mesh, in child
  order, each placed by its own local transform (as SUBDIVISION.md §1); they are
  consumed (not drawn themselves). Symmetry and Array take the first, Connect all.
- **Export weld (observed):** the reference OBJ exporter merges coincident points of the
  whole export (`symmetry/weld-off` is byte-identical to `symmetry/weld`;
  `connect/weld-off` shares the touching cubes' points). The merge is exact: points that
  differ by rounding noise survive (`lathe/angle`'s end cap, §8). Parity tests therefore
  compare `forge_parity_harness::export_weld` of **both** meshes: points within 1e-6
  merge into their first occurrence, later points are renumbered in order, faces keep
  their corners. Our generators themselves do not depend on it.

## 1. Symmetry (`forge.symmetry`)

Params: `mirror_plane` enum `xy, zy (default), xz`; `weld_points` true; `tolerance`
0.01 (cm); `symmetrical` true (no effect on the observed cases).
Result: the input's points, then the mirrored points (the plane's normal coordinate
negated: zy → x, xy → z, xz → y); with `weld_points`, an input point within `tolerance`
of the plane is not duplicated (its mirror is itself). Faces: the input's faces, then
each face mirrored with reversed winding keeping its first corner
(`[a, b, c, d] → [a', d', c', b']`). Faces lying on the plane are kept twice (observed,
`symmetry/weld`).

## 2. Array (`forge.array`)

Params: `radius` 250, `copies` 7 (≥ 0; the result has `copies + 1` copies), `amplitude`
0, `frequency` 0, `array_frequency` 10, `render_instance` false.
With `N = copies + 1`, copy k = 0..N−1 at angle `θ_k = 360°·k/N`: every input point p
becomes `Ry(θ_k)·p + Ry(θ_k)·(0, 0, radius) + (0, h_k, 0)`, where
`Ry(θ)(x, y, z) = (x cos θ + z sin θ, y, −x sin θ + z cos θ)` (copy 0 at +Z, copy 1
towards +X) and `h_k = amplitude · sin(360°·array_frequency·k/N)` at frame 0
(`array/wave`; the animated phase from `frequency` is not modelled: **parity-pending**).
Copies are concatenated in k order (points, then faces, per copy).

## 3. Connect (`forge.connect`)

Params: `weld` true, `tolerance` 0.1 (cm), `phong_mode` enum `manual, average, lowest,
highest, as_breaks (default)`, `textures` true, `center_axis` false (the link list
`objects` is not modelled; the children are the inputs).
Result: the inputs concatenated in child order. With `weld`: points within `tolerance`
merge into the first occurrence (later points renumbered in order), and a face whose
corner set equals that of a **later** face is dropped (`connect/defaults`: A's face at
x = 100 goes, B's stays). `center_axis = true` → `Unsupported` (not observed).

## 4. Instance (`forge.instance`)

Params: `reference` (link to an object), `instance_mode` enum `instance (default),
render_instance, multi_instance`. The local mesh is the reference's local mesh (same
point and face order); the instance's own transform places it. A missing or meshless
reference gives no geometry, and so does a cycle of instances (an instance of an
instance follows the chain to its first non-instance). The reference is not consumed
(it stays visible).

## 6. Spline primitives (`forge.circle_spline`, `forge.rectangle_spline`, `forge.helix_spline`)

Splines do not export (docs/parity/splines.md); everything below is derived from the
spline-input generators' cases. Common params: `plane` enum `xy (default), zy, xz`;
`reverse` false; `intermediate` enum `none, natural, uniform, adaptive (default),
subdivided`; `intermediate_number` 8; `intermediate_angle` 5° (radians stored);
`intermediate_max_length` 5. Anything but Adaptive 5°, `reverse`, the circle's
ellipse/ring, the rectangle's rounding and the helix biases → `Unsupported`.

### 6.1 Shapes (in the XY frame, then the plane)
- **Circle** `radius` 200: four cubic Bezier quarters counter-clockwise from `(r, 0)`
  through `(0, r)`, `(−r, 0)`, `(0, −r)`, handles of length `κ·r` along the tangents with
  **κ = 83/150 = 0.5533…** (fitted: every sample of `extrude/defaults` gives
  0.5533333 to 7 digits, not the textbook 0.5523).
- **Rectangle** `width`, `height` 400: lines through `(w/2, h/2), (−w/2, h/2),
  (−w/2, −h/2), (w/2, −h/2)`, closed (`extrude/rectangle`, `lathe/defaults`).
- **Helix** (`start_radius`, `end_radius` 200, `start_angle` 0, `end_angle` 720°,
  `height` 200, `subdivision` 100 per turn): an open polyline of `subdivision·turns`
  segments, `(r cos a, r sin a, −height·s)` (rises along −Z: `sweep/helix` centre
  z −100). **Parity-pending** beyond the point count (201).
- **Plane:** `xy` as is; `xz` maps (x, y, z) → (x, −z, y) (the loft circles); `zy`
  (x, y, z) → (−z, y, x) (not observed).

### 6.2 Adaptive points
A spline's points are its segment start points (and the end point when open) plus, on
curved segments, interior samples: walk t in steps of 1/50 accumulating the tangent's
turn; emit t once the turn reaches the angle (5°) and restart the sum; the last interior
sample is placed one step before the end (`1 − Δ`) instead of a sample that would leave
less than one step. For the circle this gives t = 0.06·k (k = 1..15) and 0.94, i.e. 17
points per quarter, 68 in all, for any radius (`extrude/defaults`, `lathe/circle`).
The rule is fitted to 5° only (**parity-pending** for other angles). Lines get no
interior samples.

## 7. Extrude (`forge.extrude`)

Params: `movement` (0, 0, 20); `subdivision` 1; caps `cap_start`, `cap_end` enum `none,
cap (default), fillet, fillet_cap`; `cap_type` enum `triangles, quadrangles, ngons
(default)`; `flip_normals` false. Fillets and flipped normals → `Unsupported`.
The input is the first spline child, placed by its transform. Rows j = 0..sub of the
spline points offset by `movement·j/sub`; faces per row, j outer, i inner:
`[r(j,i), r(j,i+1), r(j+1,i+1), r(j+1,i)]` (closing back to i = 0 for closed splines).
Closed splines get caps from the first and last row (they share the rows' points):
- n-gons: start `[q0, q(n−1), …, q1]`; end `[q0, …, q(n−1)]`, except that a 4-point end
  cap starts at q3 (`[q3, q0, q1, q2]`, `extrude/rectangle`): the n-gon starts at the
  vertex its triangulation fans around (**parity-pending** for other outlines).
- triangles, 4 points: start `[q3, q2, q0], [q2, q1, q0]`, end `[q0, q1, q3], [q1, q2,
  q3]`; larger outlines ear-clipped (**parity-pending**).
- quadrangles, ≤ 4 points: as n-gons; more → `Unsupported`.

## 8. Lathe (`forge.lathe`)

Params: `angle` 360°, `subdivision` 24, `movement` 0, `scaling` 100 %, caps as Extrude
with `cap_type` default **triangles** (observed: the 270° rectangle caps are triangles).
Movement/scaling other than default → `Unsupported`. The first spline child's points
(placed by its transform) are rotated about Y by `angle·k/subdivision`,
`(x cos φ − z sin φ, y, x sin φ + z cos φ)`: `subdivision` columns for a full turn,
`subdivision + 1` otherwise. Faces per column k, profile index j inner:
`[c(k,j), c(k,j+1), c(k+1,j+1), c(k+1,j)]`. Partial turns of closed profiles get caps
from the first and last columns as Extrude's (start from column 0, end from the last).
The reference writes the end cap's points separately (they differ from the last column
only by ~1e-14 and survive its exact weld); ours share the last column, and parity
compares both sides after `export_weld` (§0).

## 11. Parity tests

`crates/generators/tests/generators_parity.rs`: each case rebuilds its inputs from
`[params.input]`, `[params.input2]`, … (`type` = a primitive id suffix, `pos` the
position, other keys that primitive's params) in a scene, evaluates the generator, and
compares `export_weld` of ours and of the export strictly (topology, 1e-4); its
`UNSUPPORTED` list names the cases not derived yet.
