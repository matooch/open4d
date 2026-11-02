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
**R25 defaults not yet applied:** loft `cap_type`
N-gon (R25) vs Triangles (loft caps are always Delaunay triangles, §9). The arc, circle and helix
`intermediate` default is **Uniform** (R25, [splines-2.md](../parity/splines-2.md); brief 038,
§6.2); the R20 cases keep Adaptive through the `[circle_spline]` and `[helix_spline]` rows of
`tests/defaults/r20.toml`. O4 lists no `!` for
lathe and sweep `cap_type` (it records N-gon for R25 and, for lathe, N-gons for R20), but the
R20 meshes show Triangles (§8, `lathe/angle`): left as Triangles, **to be re-observed**.
The
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

## 6. Spline primitives (`forge.circle_spline`, `forge.rectangle_spline`, `forge.helix_spline`, §6.3)

Splines do not export (docs/parity/splines.md); everything below is derived from the
spline-input generators' cases (R20: `extrude`, `lathe`, `loft`, `sweep`) and from the
O17 extrudes ([splines-2.md](../parity/splines-2.md), `tests/golden/splines-2`, caps off).
Common params: `plane` enum `xy (default), zy, xz`; `reverse` false; the five
Intermediate Points params of §6.2 (default **Uniform** for circle, helix and arc, Adaptive
for the rest). The helix biases and `reverse` → `Unsupported`. The circle's `ellipse` and
`ring` and the rectangle's `rounding` are in §6.1; the other ten primitives of O17 are §6.3.

### 6.1 Shapes (in the XY frame, then the plane)
- **Circle** `radius` 200: four cubic Bezier quarters counter-clockwise from `(r, 0)`
  through `(0, r)`, `(−r, 0)`, `(0, −r)`, handles of length `κ·r` along the tangents with
  **κ = 83/150 = 0.5533…** (fitted: every sample of `extrude/defaults` gives
  0.5533333 to 7 digits, not the textbook 0.5523; `splines-2/circle-uniform` agrees).
- **Circle, ellipse** (`ellipse` on, `radius` 200, `radius_y` 200; `splines-2/circle-ellipse`
  with radius_y 100, 5e-7): the same four pieces with the y extent `radius_y`; the handle
  along x is `κ·radius`, along y `κ·radius_y`. Ellipse and ring together → `Unsupported`.
- **Circle, ring** (`ring` on, `inner_radius` 100; `splines-2/circle-ring`, 5e-7): a second
  closed loop, the circle of radius `inner_radius` (same direction, starting at
  `(inner, 0)`), reported by the reference as a second segment of the same spline. A
  [`Curve`] keeps it in `extra`; the Extrude puts every loop's points into each row (outer
  first) and winds the faces of the further loops the other way (`[a, a', b', b]`, the
  outer loop `[a, b, b', a']`, `circle-ring` faces 36–71). Caps of a spline with several
  loops, and Lathe/Loft/Sweep of one → `Unsupported`.
- **Rectangle** `width`, `height` 400: lines through `(w/2, h/2), (−w/2, h/2),
  (−w/2, −h/2), (w/2, −h/2)`, closed (`extrude/rectangle`, `lathe/defaults`).
- **Rectangle, rounding** (`rounding` on, `rounding_radius` 50; `splines-2/rectangle-rounding`,
  exact): 8 pieces, a quarter arc then a straight piece at each corner, counter-clockwise from
  the arc of the top right corner (it starts at `(w/2, h/2 − r)`). The arcs are the circle
  quarter with the handle rule of §6.3 (`κ`); the radius is clamped to half the shorter side
  (**parity-pending**: clamping not observed).
- **Helix, Adaptive (R20 data)** (`start_radius`, `end_radius` 200, `start_angle` 0,
  `end_angle` 720°, `height` 200, `subdivision` 100 per turn): the points
  `(r cos a, r sin a, −height·s)` at `s = i/(subdivision·turns)` (rises along −Z:
  `sweep/helix` centre z −100), joined by lines, **skipping the second and the
  second-to-last point** (observed: the default helix path has 199 points at
  z = 0, −2, −3, …, −198, −200). **Parity-pending** for other settings.
- **Helix, every other mode (R25 data, `splines-2/helix-uniform`)**: a Bezier spline of
  `subdivision + 1` points (101 for the default: the count is the total, not per turn) at
  `s_i = i/subdivision`, on the same curve `f(s) = (r cos a, r sin a, −height·s)`,
  `a = start_angle + (end_angle − start_angle)·s`, `r` linear in `s`. Point `i` has its
  handles along `f(s_i + 1/N) − f(s_i − 1/N)` (the analytic curve is extended past the
  ends, so the end points have handles too), each of length `L/3` with `L` the length of
  the helix piece the handle belongs to (`√((rθ)² + Δz²)` for constant radius, 25.2125
  per piece by default); out handle forward, in handle backward.
  **Known mismatch (`helix-uniform`):** the 901 sampled points are within 3.5e-4 of the
  export (tolerance 1e-4). The exported handles carry a small inward radial part
  (−2.95e-4 on 8.377 tangential, −0.668 in z, the same for every piece) that this
  direction/length rule misses, and the exported points drift by ≈1e-7 of their
  parameter. Variants tried (true helix tangent, chord direction with arc/3 length (above), chord direction with chord/3
  length; equal-`t` and equal-arc sampling; the best has 3.5e-4 error). The
  adaptive helix of R25 is not observed, so the two helix forms are told apart by the
  mode only (**parity-pending**).
- **Plane:** `xy` as is; `xz` maps (x, y, z) → (x, −z, y) (the loft circles); `zy`
  (x, y, z) → (−z, y, x) (not observed).

### 6.2 Intermediate Points (`spline_sampling.rs`)

A spline is a list of cubic Bezier segments `[p0, c0, c1, p1]` (`linear` splines are
straight pieces and get **no** intermediate points in any mode), closed or open, plus a
`Sampling { mode, number, angle, max_length }` read from its params. `sample(curve)` gives
the ordered point list the generators consume: for every segment its start point, then the
segment's interior samples (parameters `t` in `(0, 1)`), and for an open spline the end
point; a closed spline does not repeat its first point. Param ranges (O4/O17): `number` 0–5000
(default 8), `angle` 0–90° (5°), `max_length` ≥ 0.01 (5). Number is used by Natural and
Uniform, Angle by Adaptive and Subdivided, Maximum Length by Subdivided.

| Mode | Interior samples of one curved segment | Evidence |
|---|---|---|
| None | none: only the spline's own points | – (**parity-pending**) |
| Uniform | `segments · (number + 1)` equal steps of arc length over the **whole** spline (a closed spline keeps all of them, an open one gets the end point too); the spline's own points are not kept unless a step lands on one. Lengths are the polylines through every segment at `t = k/50`; the sample's `t` is interpolated linearly inside its piece | `splines-2/circle-uniform` (36 points of 4 quarters; 5e-7 with 50 pieces, 3e-3 with an exact length, 2e-3 with 40, 3e-3 with 64), `helix-uniform` (900 steps, mismatch of §6.1 aside), `arc-defaults` (37 points of 4 segments), `arc-sector` (45 points for 5 pieces: arc and chord are stepped as one outline, 5e-7) |
| Natural | `number` samples at `t = k/(number + 1)` (equal steps of the segment's own parameter) | **parity-pending**: no case. Equal-`t` was the first reading of Uniform and is 1.4 off `circle-uniform`, so it is not Uniform; Natural takes it as the only other even spacing |
| Adaptive | walk `t` in steps of 1/50 and emit a sample once the tangent differs from the tangent at the previous sample by `angle` (the angle between the two tangents, not the accumulated turn: the flower petal, whose tangent turns back, needs this). If the turn still to come to the segment's end is smaller than the piece just closed, the sample goes to the middle of the previous sample and the end instead (on the 1/50 grid, rounded down), and later samples are measured from the candidate's tangent. For the circle: `t = 0.06·k` (k = 1..15) and 0.94, i.e. 17 points per quarter, 68 in all, for any radius. A piece that turns less than `angle` gets nothing | `extrude/defaults`, `lathe/circle`, `splines-2/{cycloid-defaults, cycloid-epi, flower-defaults, rectangle-rounding, nside-rounding}` (the lone sample at the middle of the cycloid pieces, grid points 18/34 of the epicycloid pieces, the S-shaped flower piece); 5° only, other angles **parity-pending** |
| Subdivided | the smallest `m` such that each of `m` equal-`t` pieces turns by at most `angle` and has a chord of at most `max_length`; samples at `t = k/m` | **parity-pending**: no case |

- **Open end of Uniform:** the last point of an open spline is at `1 − 10⁻⁴` of the whole arc
  length, not at its end: `arc-defaults` ends 0.009° short of 90° (x 0.0315), `arc-angles`
  likewise, `helix-uniform` ends at z −199.98. With `number = 0` Uniform behaves like None.
  The R20 Adaptive helix ends exactly on its end point (z −200), so the shortfall is taken as
  part of Uniform (**parity-pending** for None/Natural/Subdivided, which keep the exact end).
- Uniform steps are global (`arc-sector`: the chord of the closed arc gets the same step as
  the arcs). A straight piece of a curve with curved pieces is a cubic whose handles sit on
  its end points (`splines::straight`): the samples along it follow `3t² − 2t³`
  (`arc-sector`, 7e-7). A spline made only of straight pieces (`linear`) gets no interior
  points in any mode (**parity-pending**: Uniform on a rectangle is not observed).
- Uniform counts are `(number + 1)` per segment, so a closed circle gives `4·(number + 1)`
  points and an open spline with `n` segments `n·(number + 1) + 1`.
- `Number`, `Angle` and `Maximum Length` of a mode that does not use them are kept but have
  no effect.

### 6.3 More spline primitives (O17, brief 037)

Source: [splines-2.md](../parity/splines-2.md) and `tests/golden/splines-2` (an Extrude of
offset 20, caps off, over each spline). Code: `crates/generators/src/spline_prims.rs`; every
type is `forge.<name>_spline` and has `plane` (`xy` default, `zy`, `xz`), `reverse` (false;
`true` → `Unsupported`) and the five Intermediate Points params of §6.2. Angle params store
radians (the tables show degrees); `percent` values are stored as typed (30 = 30 %).
Ranges are the observed Min/Max of O17; counts are ints.

| Type | Params (id default) | `intermediate` default |
|---|---|---|
| `arc` | `arc_type` arc (arc, sector, segment, ring), `radius` 200, `inner_radius` 100, `start_angle` 0, `end_angle` 90 (both −360..360) | Uniform |
| `nside` | `radius` 200, `sides` 6 (≥ 2), `rounding` off, `rounding_radius` 50 | Adaptive |
| `star` | `inner_radius` 100, `outer_radius` 200, `twist` 0 % (−100..100), `points` 8 (3..10⁶) | Adaptive |
| `flower` | `inner_radius` 100, `outer_radius` 200, `petals` 8 (≥ 3) | Adaptive |
| `fourside` | `fourside_type` diamond (diamond, kite, parallelogram, trapezium), `a` 200, `b` 100, `angle` 30° (−90..90) | Adaptive |
| `cogwheel` | `legacy_mode`, `show_guides`, `guide_color` (1, 0.913725, 0.498039), `teeth_type` involute (none, involute, ratchet, flat), `teeth` 20 (5..2000), `lock_radius`, `teeth_orientation` 0, `undercut`, `root_radius` 180, `addendum_radius` 220, `pitch_radius` 200, `module` 20, `diametral_pitch` 0.05, `dedendum` 20, `pressure_angle` 20°, `radial_ratio` 1.5 (0..20), `witness_ratio` 10 %, `undercut_ratio` 10 % (0..100), `inner_type` none (none, spokes, holes, arches, waves), `inner_orientation` 0, `invert`, `center_hole` on, `hole_radius` 15, `cutout` off, `cutout_depth` 10, `cutout_width` 10, `cutout_orientation` 0 | Adaptive |
| `cycloid` | `cycloid_type` cycloid (cycloid, epicycloid, hypocycloid), `radius` 25, `r` 50, `a` 75, `start_angle` 0, `end_angle` 360 (unbounded) | Adaptive |
| `profile` | `profile_type` h_shape (h_shape, l_shape, t_shape, u_shape, z_shape), `height` 200, `b` 100, `s` 20, `t` 20 | Adaptive |
| `cissoid` | `cissoid_type` cissoid (cissoid, lemniscate, strophoid), `width` 400, `tension` 2 (1.5..100) | Adaptive |
| `formula` | `x_t` `100.0*t`, `y_t` `100.0*Sin(t*PI)`, `z_t` `0.0`, `t_min` −1, `t_max` 1, `samples` 20 (≥ 2), `cubic_interpolation` off | Adaptive |

The Cogwheel and Formula tables list what O17 recorded; the coupled limits of the Cogwheel's
Teeth rows are not modelled (the values are stored as given, no clamping).

**Handle rules** (shared by several shapes):
- *Arc handles.* A piece that turns by `θ` around a centre at radius `ρ` has handles tangent
  to the circle of length `κ·ρ·θ/(π/2)` with `κ = 83/150` (§6.1): linear in the turn, not the
  textbook `4/3·tan(θ/4)`. Proven by `circle-uniform` (θ = 90°), the 60° fillets of
  `nside-rounding` (mid point of a fillet, 5e-7) and the four pieces of `arc-angles` (67.5°).
- *Automatic tangents* (`splines::auto_tangent_curve`, for `flower` and `cycloid`): at every
  point the tangent runs along `next − previous`, and **both** handles are a third of the
  shorter of the two adjacent chords long; the two ends of an open curve have only their chord
  (handle = chord/3 along it). `cycloid-defaults`, `cycloid-epi` (mid samples of 53 pieces,
  6.6e-7) and `flower-defaults` (3 pieces per petal, 5e-7) all follow it; the straight
  Hermite form with `f'(t)·Δ/3` and Catmull-Rom (1/6) were off by 2–4e-3.

**Shapes** (XY frame, counter-clockwise from +x unless noted):
- **Arc** (`arc-defaults`, `arc-angles`, 3e-5 and 9e-5): four equal pieces from `start_angle`
  to `end_angle` (also 270°, `arc-angles`): the spline is open with 5 points and
  `intermediate` Uniform, so 37 points. `arc_type = sector` (`arc-sector`, 5e-7) closes the
  four pieces with the straight chord from the end back to the start (a fifth, `straight`
  piece, so 5 points, Uniform makes 45). **Observation note:** the option the observer read
  back as "Sector" produced this chord shape, not a pie slice through the centre; the id
  `sector` keeps the observed shape, and `segment`/`ring` → `Unsupported` (**parity-pending**
  until the option labels are re-observed). The `inner_radius` is unused so far.
- **n-Side** (`nside-defaults`, `nside-8`): `sides` points on the circle `radius`, the first
  at `(r, 0)`, joined by lines. With `rounding` (`nside-rounding`, 5e-7) every corner becomes
  an arc piece of the fillet circle (radius `rounding_radius`, centre on the bisector at
  `rf / cos(π/n)` from the corner, tangent points `rf·tan(π/n)` along the edges, handles by the
  arc rule, turn `2π/n`) followed by a straight piece: 12 spline points for 6 sides, the first
  arc at the corner on +x. The radius is clamped so the tangent points stay within half an
  edge (**parity-pending**: only radius 20 observed).
- **Star** (`star-defaults`, `star-twist`, 3e-7/5e-7): `2·points` points, a line polygon,
  alternating outer at `k·2π/points` and inner at `(k + ½ + twist/100)·2π/points`; the first is
  an outer point at angle 0.
- **Flower** (`flower-defaults`, 5e-7; **parity-pending** except for these defaults): per petal
  `k` (angle `a = k·2π/n`) three points: the inner point `ri·(cos a, sin a)`, the lobe point
  `inner + (ro − ri)·(cos(a + π/n), sin(a + π/n))` and its reflection across the petal axis (the
  line at `a + π/n`); the 3n points are joined by automatic tangents (closed). Radii are
  fitted as listed; with the default `ro − ri = ri = 100` the lobe offset cannot be told from
  `ri`, nor the outer radius from `2·ri`.
- **4-Side** (`fourside-defaults`, `fourside-types`): points right, top, left, bottom around
  the origin. Diamond: `(±a/2, 0)`, `(0, ±b/2)`; `angle` unused. Kite (`d = √(a² + b²)`):
  `(a·b/d, 0)`, `(0, b²/d)`, `(−a·b/d, 0)`, `(0, −a²/d)` (right angles at the left and right
  vertices; sides `b` above, `a` below). Parallelogram and Trapezium → `Unsupported`.
- **Profile** (`profile-defaults`): the H shape, origin at the lower left corner, points
  `(0,0) (b,0) (b,t) (w1,t) (w1,h−t) (b,h−t) (b,h) (0,h) (0,h−t) (w0,h−t) (w0,t) (0,t)` with
  `w0 = (b − s)/2`, `w1 = (b + s)/2`; the other four types → `Unsupported`.
- **Cissoid** (`cissoid-defaults`, 5e-7): an open polyline of 101 points at `t = −1 + 0.02k`,
  `y = (W/2)·(1 + T²)/T² · u²/(1 + u²)` with `u = T·t`, `x = t·y`, `W` = `width`, `T` =
  `tension` (so the ends are `(±W/2, W/2)` and the cusp sits at the origin). `T = 2` only
  observed (**parity-pending**); lemniscate and strophoid → `Unsupported`.
- **Cycloid** (`cycloid-defaults`, `cycloid-epi`, 5e-7): points at `t_i = start + i·(end −
  start)/n`, `n = round(62·|end − start|/2π)` (62 for a full turn, both types), joined by
  automatic tangents, open. Cycloid: `x = R·t − a·sin t`, `y = R − a·cos t` (`R` = `radius`; `r`
  is unused); Epicycloid: `x = (R + r)·cos t − a·cos(k·t)`, `y = (R + r)·sin t − a·sin(k·t)`,
  `k = (R + r)/r`. Hypocycloid → `Unsupported`. The piece count 62 is **parity-pending** for
  other angle spans and radii.
- **Formula** → `Unsupported` ("the expression library (brief 052) does not exist yet"):
  `formula-defaults` is a line polyline of `samples + 1` points at `t_min + (t_max − t_min)·i/samples`,
  open. Needs `forge-expr` for `x_t`, `y_t`, `z_t`.
- **Cogwheel** → `Unsupported` (`cog-defaults`): the outline is not derived. What the export
  shows (20 teeth, pitch 18°, tooth centre at 4.5° + 18°·k): two loops (outline and the centre
  hole, a circle of `hole_radius` 15 with Adaptive 68 points), 12 spline points per tooth
  (240 + 4). Per tooth, from the gap centre: a root land on the root circle (180) ±1.3554°
  around 13.5° with an own point at its middle; a fillet of radius 7.5 (= 0.375·module?) from the
  root circle at ±7.6446° around the tooth centre up to `(r = 187.5, ±5.354°)` (5.354° =
  half tooth angle 4.5° + inv(20°) = the angle where the involute leaves the base circle
  `200·cos 20° = 187.94`); the flank follows the involute of that base circle up to the tip
  (angle ±1.81° around the tooth centre at `r ≈ 220.006`); a tip land with an own point at its
  middle (r = 220). Adaptive samples per tooth: 52. Open data that would settle it: the
  editable spline of one tooth (point positions and tangents), then the radial/witness/undercut
  ratio effects.

**Extrude Direction Auto (for brief 044).** The default Cycloid is the only export that
extrudes to −z. All 22 exports fit "−z exactly when the spline, closed by its chord, has a
negative signed area in its plane" (cycloid −29431; every other flat spline positive, the
Formula zero and +z). The O17 test applies that guess; **parity-pending** (one negative case,
only the XY plane).

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
  (reversed), the constrained Delaunay triangulation of the ring in the reference's
  listing (GEOMETRY.md §6.1). `loft/defaults` is exact; `loft/three` differs in its end
  cap by one exactly cocircular (mirror-symmetric) quad whose diagonal the export takes
  the other way, so parity compares those caps geometrically (§11).

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
`CAP_GEOMETRIC` the loft case (`loft/three`) whose caps are compared geometrically (all
points and every face before the caps exactly).

`crates/generators/tests/spline_sampling.rs` runs the 22 O17 cases (`splines-2`): an Extrude
of offset 20, caps off, over the spline, with the offset negated when the signed area of the
spline is negative (§6.3, Direction Auto), compared strictly after `export_weld`. **19 of 22
exact**; `helix-uniform` is a known mismatch (§6.1) and `cog-defaults` and `formula-defaults`
are `Unsupported` (§6.3). `tests/spline_prims.rs` covers defaults, options and the refusals.

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
