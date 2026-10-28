# Generators II — mesh-input generators (`crates/generators`)

Sources: docs/parity/{symmetry,array,connect,instance}.md and their golden cases.
Spline primitives and the spline-input generators (extrude, lathe, loft, sweep) follow
in §6–10 (docs/parity/splines.md: splines do not export).

## 0a. Defaults and ranges (R25)

Defaults and ranges are the **R25** ones ([docs/parity/r25-defaults.md](../parity/r25-defaults.md),
O4); where R20 differed the old value follows as "(R20: x)", and R20 cases are built with the
R20 values from `tests/defaults/r20.toml` (INFRA.md §3.1). Ranges changed only where O4 shows
a finite clamp that differs from the previous one: `array` `copies` 1–1000, `frequency` and
`array_frequency` ≥ 0; spline `intermediate_number` 0–5000, `intermediate_angle` 0–90°,
`intermediate_max_length` ≥ 0.01; helix `radial_bias`, `height_bias` 0–100 %, `height` ≥ 0;
extrude `subdivision` 1–4000; lathe `subdivision` 1–4000, `scaling` ≥ 0; loft
`mesh_subdivision_u` / `_v` 2–4000; sweep `end_scale` ≥ 0.
**R25 defaults not supported yet** (the descriptor keeps the R20 value, since the build does
not implement the R25 option and would return `Unsupported` or ignore it): circle and helix
`intermediate` Uniform (R25) vs Adaptive (§6: only Adaptive 5° builds); loft `cap_type`
N-gon (R25) vs Triangles (loft caps are always ear-clipped triangles, §9). O4 lists no `!` for
lathe and sweep `cap_type` (it records N-gon for R25 and, for lathe, N-gons for R20), but the
R20 meshes show Triangles (§8, `lathe/angle`): left as Triangles, **to be re-observed**. The
R25-only parameters (extrude Direction, bevel tabs, sweep rotation from/to, "use new
version" switches of cylinder and torus, tube and cone fillet defaults) are out of scope.

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

Params: `radius` 250, `copies` 7 (1–1000; the result has `copies + 1` copies), `amplitude`
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

## 5. Spline inputs

Extrude and lathe take their first spline child, sweep its first two (profile, path),
loft all of them, in child order; each is placed by its local transform and consumed.
Spline primitives have no mesh of their own (they are not drawn as meshes yet).

## 6. Spline primitives (`forge.circle_spline`, `forge.rectangle_spline`, `forge.helix_spline`)

Splines do not export (docs/parity/splines.md); everything below is derived from the
spline-input generators' cases. Common params: `plane` enum `xy (default), zy, xz`;
`reverse` false; `intermediate` enum `none, natural, uniform, adaptive (default),
subdivided` (R25 default of the circle and helix: Uniform, **not supported yet**, §0a);
`intermediate_number` 8; `intermediate_angle` 5° (radians stored);
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
  `height` 200, `subdivision` 100 per turn): the points `(r cos a, r sin a, −height·s)`
  at `s = i/(subdivision·turns)` (rises along −Z: `sweep/helix` centre z −100), joined
  by lines, **skipping the second and the second-to-last point** (observed: the default
  helix path has 199 points at z = 0, −2, −3, …, −198, −200). **Parity-pending** for
  other settings.
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

Params: `movement` (0, 0, **100**; R20: 20; O4 calls the R25 field Offset, one number plus a
Direction list, out of scope); `subdivision` 1; caps `cap_start`, `cap_end` enum `none,
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

Params: `angle` 360°, `subdivision` 32 (R20: 24), `movement` 0, `scaling` 100 %, caps as Extrude
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

## 9. Loft (`forge.loft`)

Params: `mesh_subdivision_u` 16 (R20: 30), `mesh_subdivision_v` 4 (R20: 10), `organic_form`, `loop`,
`linear_interpolation` false, `subdivision_per_segment` true, caps as Extrude with
`cap_type` default triangles; other option values, open sections and a single section
→ `Unsupported`.
- **Sections:** every spline child (child order), placed by its transform, sampled at
  `U − 1` points uniform in spline parameter, `s_i = segments·i/(U − 1)` (the closed
  seam not repeated; `loft/defaults`: none of the 29 points lands on ±200).
  **Known mismatch:** with U = 7 the reference takes s = 0, 2/3, 1, 2, 8/3, 3 instead of
  0, 2/3, 4/3, 2, 8/3, 10/3 (`loft/mesh-subdivision`; not understood).
- **Between sections:** per sample index, a cubic Hermite curve through the sections'
  samples, V rows per section pair (`t = j/V`). Tangent at an end section: the chord to
  its neighbour; at an inner section m: the unit of `P(m+1) − P(m−1)` times the chord of
  the segment being built (fitted to `loft/three` to 4 digits; two sections give
  straight, evenly spaced rows).
- **Faces:** per row pair, `[r(j,i+1), r(j+1,i+1), r(j+1,i), r(j,i)]` (wound opposite to
  Extrude, and against the caps: the exported loft is not consistently oriented), i
  wrapping. Caps from the first ring (wound as the ring) and the last ring
  (reversed), triangulated by the reference with an underived method; ours ear-clip,
  and parity compares the caps geometrically (§11).

## 10. Sweep (`forge.sweep`)

Params: `end_scale` 100 %, `end_rotation` 0°, `banking` true, `constant_cross_section`
true, caps as Extrude with `cap_type` default triangles; other values → `Unsupported`.
Inputs: the first spline child is the profile, the second the path (both placed by
their transforms; the profile is read in its XY coordinates).
- **Rings:** one per path point (closed paths don't repeat the first). Tangent: the
  bisector of the unit incoming and outgoing chords (ends: the one chord). Normal: at
  the first point away from the bend, then parallel-transported point to point;
  binormal = tangent × normal. Profile (x, y) → `normal·x·mitre + binormal·y`.
- **Constant cross section:** `mitre = 1/cos(half the turn)` where the path turns by 5°
  or more, else 1 (`sweep/defaults`: rings 15–16, turning 4.6°, are not stretched; the
  5° threshold is **parity-pending**, a chord-ratio rule would fit too).
- **Scale and twist:** with f = length along the path / total length (closed paths
  include the closing chord), the profile is scaled by `1 + (end_scale − 1)·f` and
  rotated by `−end_rotation·f` before the mitre (`sweep/end-scale`, `end-rotation`).
- **Faces:** `[g(k,i), g(k,i+1), g(k+1,i+1), g(k+1,i)]` per ring pair and profile span;
  open paths with closed profiles get triangulated caps.
- **Known mismatch:** along a non-planar path (`sweep/helix`) the reference's frame starts
  0.4° tilted and twists less than parallel transport (48° against 56° over the default
  helix); not derived.

## 11. Parity tests

`crates/generators/tests/generators_parity.rs`: each case rebuilds its inputs from
`[params.input]`, `[params.input2]`, … (`type` = a primitive id suffix, `pos` the
position, other keys that primitive's params) in a scene, evaluates the generator, and
compares `export_weld` of ours and of the export strictly (topology, 1e-4); its
`KNOWN_MISMATCH` list pins the two cases above that we build but don't match, and
`CAP_GEOMETRIC` the loft cases whose caps are compared geometrically (all points and
every face before the caps exactly).

### 11.1 R25 drift cases and the R25 extrude cap order

`crates/generators/tests/r25_drift.rs` rebuilds `tests/golden/r25-drift/<feature>-<case>`
(INFRA.md §3.1: R25 defaults, no overlay) with the feature's builder and compares like the
feature's own test (strict, strict after `export_weld` for generators, geometric for boole,
exact-up-to-cut-triangles for the torus slice). 6 of 7 match.
- **Known mismatch `r25-drift/extrude-rectangle-caps-quadrangles` ("R25 vertex order")**:
  the 8 points, the 4 side faces and the face set are those of `extrude/rectangle-caps-quadrangles`
  (geometric comparison passes; with Offset 100 the depth is 100), but the two 4-point caps
  start at another corner: R25 start cap `[3, 2, 1, 0]` (R20 `[0, 3, 2, 1]`), end cap
  `[4, 5, 6, 7]` (R20 `[7, 4, 5, 6]`). Hypotheses: (1) R25 writes the start cap as the
  reversed outline from the last point and the end cap as the outline from the first, with
  no start-corner rule; (2) both R20 lists rotated left by one position (`[0,3,2,1]` →
  `[3,2,1,0]`, `[7,4,5,6]` → `[4,5,6,7]`); (3) the R20 fan-vertex corner rule (above) kept
  but moved by one corner. (1) and (2) give the same lists for a 4-point cap and fit both
  caps; (3) cannot be told from them on this outline. All rest on this one case
  (**parity-pending**), and larger outlines (the circle) are not observed in R25. The order
  depends on the app version and our generators have no version switch, so adopting it would
  change the R20 order that the R20 cases pin. Left as is, **to be decided with a
  version-aware cap order or after observing a larger outline in R25**.
- `r25-drift/torus-slice` matches with the same comparison as `torus/slice`; the vertex
  order O1 mentions was not seen (our points equal the golden ones in order).
- `r25-drift/boole-*` (both): geometric match with R25 defaults (the six extra duplicate
  points are welded on both sides; face split depends on `hide_new_edges`, which the
  geometric comparison ignores).
- `r25-drift/sphere-type-icosahedron` led to the icosahedron frequency fix, PRIMITIVES.md §3.1.
