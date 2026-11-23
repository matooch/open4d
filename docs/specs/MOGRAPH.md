# MoGraph spec checklist (verify each against manual + observation; add anything missing in the installed version's menus)
## Generators
- [ ] Cloner (brief 065: clone data, Linear, Radial, Grid Array, Honeycomb and instance modes done, see the clone-toolset sections below; Object, order modes, effectors pending): modes Linear, Radial, Grid Array, Honeycomb, Object (vertex/edge/polygon/surface/volume/axis), Blend; Iterate/Random/Blend/Sort clone order; fix clone, fix texture, instance modes (Instance/Render Instance/Multi-Instance), effectors list, per-clone transforms, scale/rotation/color/weight, MoData output
- [ ] Matrix, Fracture (modes + connectors), Voronoi Fracture (sources, distribution, shape, details), MoSpline (Spline/Turtle/Simple modes), MoText, MoInstance, Tracer (trace links/vertices, interval, spline types)
## Effectors
- [ ] Plain (brief 068: framework + Plain done, see section E below), Random (brief 069, sections E11-E17), Delay (brief 070), Formula (070), Inheritance, Python, Shader, Sound, Spline, Step (070), Target (070), Time (070), Volume, Push Apart, Reeffector, Group
- [ ] Common: strength, selection, transform (P/S/R), color, weight, UV, visibility, falloff (shapes/directions/field list), min/max, mode (Add/Subtract/Multiply/etc.)
## Fields
- [ ] Solid, Spherical, Box, Cylinder, Capsule, Cone, Torus, Formula, Linear, Radial, Random, Shader, Sound, Step, Delay, Object, Group, Python
- [ ] Field layers: blend modes, remapping (curve, clamping, inverse), mask/folder layers, direction/color output
## Related
- [ ] MoGraph selection tag, Multi Shader, Color Shader, MoGraph Weight/Cache tags, Dynamics Body/Collider tags interplay, Cloner + deformer ordering rules
Each item: spec file docs/parity/<item>.md, golden export, parity test.

---

# Clone toolset: clone data, cloner, Linear / Radial / Grid Array / Honeycomb (brief 065)

Wording: "clone toolset", "cloner", "clone data", "effector" (DECISIONS; the cleanroom gate
rejects the reference app's brand name in code and UI). Sources: parity observations
O8 [cloner.md](../parity/cloner.md), O11 §2 [followups-1.md](../parity/followups-1.md),
O21 [cloner-3.md](../parity/cloner-3.md), design [E-clone-toolset.md](../roadmap/areas/E-clone-toolset.md),
evaluation context [MODIFIERS.md §8](MODIFIERS.md). Golden data: `tests/golden/cloner/`
(`samples.json`, kind `clones`, level B, tolerance 1e-4; `mesh.obj`, level A).
Out of scope here: Object mode (brief 066), several children and the Clones order modes
Random / Blend / Sort, nesting (067), effectors (068+), fields on effectors, GPU instancing
and clone display (079).

## C1. Clone data (`forge_clone_engine::CloneData`)

A struct of arrays, one entry per clone, all vectors the same length. It is `Clone` and
holds plain vectors, so effectors (068+) take it by `&mut` and copy it cheaply.

| field | type | meaning |
|---|---|---|
| `matrices` | `DMat4` | clone matrix in cloner space: columns are the X, Y, Z axes (scale included) and the position (`pos`, `x`, `y`, `z` of the samples) |
| `colors` | `[f64; 3]` | RGB, 0..1 |
| `weights` | `f64` | weight (fraction; the sample `weight`) |
| `visible` | `bool` | the clone-on flag; hidden clones keep their slot (`grid-sphere-form`: 125 entries, 33 visible) |
| `uvw` | `DVec3` | position of the clone in the distribution, 0..1 per axis (own design, **parity-pending**: not observed) |
| `index` | `u32` | clone index = position in the list |
| `time` | `f64` | per-clone time offset in seconds, 0 for the cloner itself (effectors write it later) |
| `child` | `u32` | which prototype (child of the cloner) the clone shows; Iterate: `index % children` (brief 067 adds the other orders) |

Consumers must check `visible`. Geometry expansion (C3) skips invisible clones.

## C2. The cloner node (`forge.cloner`)

Registered by the plugin `forge_clone_engine::CloneToolset` (id `forge.clone-toolset`) in
category `Generator`, label "Cloner". `forge_app::default_registry` installs it, so
`object.create {type: "forge.cloner"}` makes one from the UI, CLI, RPC, MCP and Rhai alike. Every
parameter below is a `ParamDesc`; the attribute panel, undo, animation tracks and the
file format derive from them. Angles are radians (`Unit::Angle`), percent values are
fractions (`Unit::Percent`, 1.0 = 100 %), lengths are cm. Case files hold the labels,
degrees and percent as typed in the app; the parity test converts.

A new cloner starts in **Grid Array** (O8: "The new cloner's mode is Grid Array, not
Linear"). The groups are the app's tabs: Object, Transform.

Common (O8 table "Common to all modes"):

| id | label | kind | default | options / range |
|---|---|---|---|---|
| `mode` | Mode | enum | `grid` | `object`, `linear`, `radial`, `grid`, `honeycomb` |
| `clones_mode` | Clones | enum | `iterate` | `iterate`, `random`, `blend`, `sort` |
| `reset_coordinates` | Reset Coordinates | bool | true | |
| `fix_texture` | Fix Texture | enum | `off` | `off`, `straight`, `alternate_x`, `alternate_y` |
| `instance_mode` | Instance Mode | enum | `instance` | `instance`, `render_instance`, `multi_instance` |
| `viewport_mode` | Viewport Mode | enum | `object` | `off`, `points`, `matrix`, `bounding_box`, `object` |
| `display` | Display | enum | `none` | `none`, `weight`, `uv`, `color`, `index` |
| `clone_position` | P | vec3 | (0, 0, 0) | |
| `clone_rotation` | R | vec3 (h, p, b) | (0, 0, 0) | angle |
| `clone_scale` | S | vec3 | (1, 1, 1) | |
| `clone_color` | Color | color | (1, 1, 1) | |
| `uv_orientation` | W(UV)-Orientation | enum | `z_plus` | `x_plus`, `x_minus`, `y_plus`, `y_minus`, `z_plus`, `z_minus` |
| `clone_weight` | Weight | float, percent | 0 | 0 .. 1e20 |
| `animation_mode` | Animation Mode | enum | `play` | `play`, `loop`, `fixed`, `fixed_loop` |

Linear (`mode = linear`):

| id | label | kind | default | range |
|---|---|---|---|---|
| `linear_count` | Count | int | 3 | 0 .. 1 000 000 |
| `linear_offset` | Offset | float | 0 | >= 0 |
| `linear_mode` | Mode | enum | `per_step` | `end_point`, `per_step` |
| `amount` | Amount | float, percent | 1 | |
| `transform_position` | P (per step) | vec3 | (0, 50, 0) | distance |
| `transform_scale` | S (per step) | vec3, percent | (1, 1, 1) | |
| `transform_rotation` | R (per step) | vec3 (h, p, b) | (0, 0, 0) | angle |
| `step_mode` | Step Mode | enum | `single` | `single` ("Single Value"), `cumulative` |
| `step_size` | Step Size | float, percent | 1 | >= 0 |
| `step_rotation` | Step Rotation | vec3 (h, p, b) | (0, 0, 0) | angle |

Radial (`mode = radial`):

| id | label | kind | default | range |
|---|---|---|---|---|
| `radial_count` | Count | int | 5 | 0 .. 1 000 000 |
| `radius` | Radius | float | 50 | |
| `plane` | Plane | enum | `xz` | `xy`, `zy`, `xz` |
| `align` | Align | bool | true | |
| `start_angle` | Start Angle | float | 0 | angle |
| `end_angle` | End Angle | float | 360° | angle |
| `radial_offset` | Offset | float | 0 | angle |
| `radial_offset_variation` | Offset Variation | float, percent | 0 | >= 0 |
| `radial_offset_seed` | Offset Seed | int | 1234567 | 0 .. 2147483647 |

Grid Array (`mode = grid`):

| id | label | kind | default | range |
|---|---|---|---|---|
| `grid_count` | Count | vec3 (whole numbers) | (3, 1, 3) | 1 .. 1 000 000 per axis |
| `grid_mode` | Mode | enum | `per_step` | `per_step` ("Per-Step"), `endpoint` ("Endpoint") |
| `grid_size` | Size | vec3 | (200, 200, 200) | |
| `grid_form` | Form | enum | `cubic` | `cubic`, `sphere`, `cylinder`, `object` |
| `fill` | Fill | float, percent | 1 | 0 .. 1 |

Honeycomb (`mode = honeycomb`):

| id | label | kind | default | range |
|---|---|---|---|---|
| `orientation` | Orientation | enum | `z` | `z` ("Z (XY)"), `x` ("X (ZY)"), `y` ("Y (XZ)") |
| `offset_direction` | Offset Direction | enum | `height` | `width`, `height` |
| `honeycomb_offset` | Offset | float, percent | 0.5 | 0 .. 1 |
| `honeycomb_offset_variation` | Offset Variation | float, percent | 0 | 0 .. 1 |
| `perpendicular_variation` | Perpendicular Variation | float, percent | 0 | 0 .. 1 |
| `honeycomb_seed` | Seed | int | 123456 | 0 .. 2147483647 |
| `count_width` | Count Width | int | 10 | 1 .. 1 000 000 |
| `count_height` | Count Height | int | 10 | 1 .. 1 000 000 |
| `honeycomb_mode` | Mode | enum | `per_step` | `per_step`, `end_point` |
| `size_width` | Size Width | float | 300 | >= 0 |
| `size_height` | Size Height | float | 346.4101616 | >= 0 |
| `honeycomb_form` | Form | enum | `square` | `circle`, `square`, `spline` |

The app swaps the two counts and the two sizes when Offset Direction changes (cloner-3.md
notes). That is a UI behaviour of the reference; our parameters are independent and the
direction only changes which count/size is the line axis (C7).

**Object mode (brief 066, [O21](../parity/cloner-3.md)).** `object` links the source,
`distribution` defaults to `surface`, `object_count` to 20, `object_seed` to 1234567,
`align_clone` to true, `up_vector` to `none`, and `selection` to the empty tag name.
Use source positions in cloner-local space. Axis uses the linked object's origin.
Vertex uses point order and the uniform mean of incident unit normals. Edge uses
unique edges in first polygon encounter order, their midpoints, the mean of the
incident unit normals, and Y along the edge toward its first endpoint.
Polygon Center uses face order and the unit face normal; a quad uses the mean of
its corners, and a triangle uses `f64::from(0.333_f32) * (a+b+c)` (O21 sphere caps).
The normal is the clone's Z axis. With alignment off, use identity axes.
The named selection on the source filters matching point, edge or polygon
indices before sampling. A name without a compatible tag produces no clones.
**Known mismatches:** `object-vertex` and `object-polycenter` keep exact position and
Z; the unconstrained roll differs at the sphere poles/rings and on the lower triangular
caps. The fallback projects +Y for vertices and the first edge for triangle Y;
the quad uses the last-to-first edge for X. Named Up Vector overrides that roll.

Surface samples triangles in proportion to area, with uniform barycentric
coordinates; Volume rejection-samples points inside the closed mesh's bounding box.
SplitMix64 supplies seeded draws. O21 random cases are compared statistically
(count, per-axis bounds within 45% of the recorded extent, means within 3.5 pooled
standard errors, and variance within a factor of four), not by sample order.
Volume on open meshes or with a polygon selection is unsupported.

Limit: a cloner makes at most `MAX_CLONES = 1 000 000` clones; more is `Unsupported`.

## C3. Evaluation and geometry expansion

1. The children of the cloner that have geometry (`evaluate_geometry` is not `None`) are the
   **prototypes**, in child order. They are evaluated once (with their own deformers, MODIFIERS.md
   §0). Children without geometry (deformers, effectors later) are ignored. Prototypes are *consumed*:
   `is_consumed` is true for them, so export and the viewport draw the clones, not the prototypes
   (every golden `mesh.obj` holds only the clones).
2. `forge_clone_engine::generate(&Settings, &EvalCtx) -> CloneData` makes the clone data of the
   mode (C4–C7), then the common Transform step (C3.1).
3. Each **visible** clone, in list order, appends its prototype's points transformed by the clone
   matrix and its polygons with the point indices offset. The golden OBJ lists one block per clone
   (`Cube_0`, `Cube_1`, ...) in clone order, each with the prototype's own vertex and face order
   (`linear-defaults`: clone 1 is the cube block shifted by (0, 50, 0)). Invisible clones add
   nothing (`grid-sphere-form`: 33 cubes, 264 vertices).
4. `Reset Coordinates` on (default): the prototype's own local transform is ignored, so clones sit
   exactly at the clone matrices. Off: the prototype's local matrix is applied first, then the clone
   matrix (`reset-coords-off-2`, matches).
5. The cloner's own deformer children deform the expanded mesh as for any generator.
6. `instance_mode` (Instance, Render Instance, Multi-Instance) and `viewport_mode`, `display`,
   `fix_texture`, `animation_mode`, `uv_orientation` are stored and shown by the viewport/renderer
   hooks (brief 079); they never change the exported geometry (O8: the export of an Instance-mode
   cloner holds the cloned cubes).
7. The result is in cloner space; the caller applies the cloner's world matrix.
8. Make Editable on a cloner produces a polygon object with this expanded mesh and deletes the
   prototypes (the generic rule of EDITABLE.md §1 for consumed inputs).
9. With more than one prototype and `clones_mode = iterate`, clone `i` shows prototype
   `i % n` (`CloneData::child`). The other Clones modes with more than one prototype are
   `Unsupported` until brief 067 (the order cases are not claimed). With one prototype all four
   Clones modes give the same result.

### C3.1 Transform tab (all modes)

After the mode has produced the clone matrix `M_i`, the clone is `T(P) · M_i · Rot(R) · S(S)` with the
Transform tab's `clone_position` P, `clone_rotation` R (HPB, TRANSFORM.md §3) and `clone_scale` S.
Colour is `clone_color`, weight is `clone_weight` (fraction), `time` 0, `index` = list position.

- P is added to the position: `linear-count-step` has P = (0, 50, 0), per step (0, 50, 0): clone 0 sits
  at y = 50, clone 4 at y = 250.
- R, S, colour and weight are neutral at their defaults in every sample. Their formulas are
  **parity-pending** (own design). P is added in cloner space, not rotated by the clone's rotation
  (**parity-pending**: `linear-count-step` rotates about Y and P is along Y, so it cannot tell).

## C4. Linear

`t_i = i + linear_offset` (i = 0 .. count - 1). `u_i = t_i / (count - 1)` (0 when count is 1).

**Offset** (`cloner/linear-offset`, offset 1, count 5, P = (0, 50, 0): the samples hold **6** clones at
y = 0, 50, ..., 250 and the first one is hidden (`visible = false`; the mesh holds the 5 visible cubes
at y = 50 .. 250)). An integral offset `o >= 1` therefore prepends `o` hidden clones to the list: the
list has `count + o` entries, entry `j` has `t_j = j` (so the visible ones are `t = o .. o + count - 1`,
the same positions as `t_i = i + o`) and `u` stays `t / (count - 1)`. A non-integral offset keeps the
shifted formula without extra clones (**parity-pending**: 1.5 was not observable, the app's Offset is a
checkbox in this build).

Per Step (`linear_mode = per_step`, default), with `a = amount`:

- position `t_i · a · P_step` — `linear-defaults`: (0, 0, 0), (0, 50, 0), (0, 100, 0).
- scale per axis `1 + t_i · a · (S_step - 1)` — **linear, not a power**: `linear-scale-step` (S 90 %)
  has axis lengths 1, 0.9, 0.8, 0.7, 0.6.
- rotation (HPB) `t_i · a · R_step + s_i · step_size · Step_rot` with `s_i = t_i` for Step Mode
  Single Value and `s_i = t_i (t_i + 1) / 2` for Cumulative. `linear-count-step`: Step Rotation
  H = 10° gives H = 0, 10°, 20°, 30°, 40° (`x = (cos H, 0, sin H)`, i.e. `Ry(-H)`, TRANSFORM.md §1).
  `linear-cumulative` (H = 15°, Cumulative) gives 0, 15°, 45°, 90°, 150° (x of clone 4 =
  (-0.866, 0, 0.5)). **Cumulative changes only the rotation**: the positions of that case stay
  linear (y = 0, 50, ..., 200).
- `matrix = T(pos) · Rot(rot) · S(scale)`.

End Point (`linear_mode = end_point`): the per-step values are the values of the **last** clone,
`pos = u_i · a · P`, `scale = 1 + u_i · a · (S - 1)`, rotation `u_i · a · R` plus the Step Rotation
rule above. `linear-mode-endpoint` (count 5, P y = 200): y = 0, 50, 100, 150, 200.

Parity-pending: `amount` (neutral at 100 %: the formula above is our reading), `step_size` (same),
the roles of R (per step) against Step Rotation (they add; only Step Rotation H was set), the
scale/rotation of End Point (positions only observed), `linear_offset`.

## C5. Radial

Angle of clone `i`: `a_i = start + offset + i · (end - start) / count`. The divisor is the
**count, also for partial arcs**: `radial-plane-start-end` (count 5, 0°..180°) has a_i = 0, 36°, 72°,
108°, 144° (y = 150 cos a_i = 121.35 for i = 1). With the defaults the arc is the full circle and the
last clone is one step short of the first: 0, 72°, ... 288°. `radial-offset` (offset 30°, count 8)
starts at 30°: (25, 0, 43.301). Offset Variation scales the **Offset**, not the step
(`radial-var-1`, `-var-2`: Offset 0, Variation 50 %, seeds 1 and 2 give exactly the unvaried circle).
Each angle gets `+ variation · offset · r / 2` with `r` in [-1, 1] the next number of the seeded
generator (C9), so with Offset 0 nothing moves. The control the observer reports without numbers (Offset
30°, Variation 50 %, two seeds: clone 0 stays on the offset position, the later clones fan out around
it) is read as: clone 0 takes no jitter (**parity-pending**, no numbers).

Position and axes by plane (`s = sin a`, `c = cos a`, radius `R`), proven by `radial-defaults`
(XZ), `radial-plane-start-end` (ZY) and `radial-xy` (XY):

| plane | position | X axis | Y axis | Z axis |
|---|---|---|---|---|
| XZ (default) | (R s, 0, R c) | (c, 0, -s) | (0, 1, 0) | (s, 0, c) |
| ZY | (0, R c, R s) | (1, 0, 0) | (0, c, s) | (0, -s, c) |
| XY | (R s, R c, 0) | (c, -s, 0) | (s, c, 0) | (0, 0, 1) |

(The Z / Y / Y axis points away from the centre.) With Align off the axes stay the identity:
`radial-align-off` has clone 1 at (47.553, 0, 15.451) with the unrotated frame.

## C6. Grid Array

Counts `(nx, ny, nz)`, size `(sx, sy, sz)`, centred on the origin. The list runs **x fastest, then
y, then z**: index `ix + nx · (iy + ny · iz)` (`grid-3x2x4`: clones 0..2 along x at y = -100, clones
3..5 at y = +100, then z steps).

- Per-Step: `pos_k = (i_k - (n_k - 1) / 2) · size_k` — `grid-3x2x4` (size 300, 200, 400): x = -300, 0,
  300; y = -100, 100; z = -600, -200, 200, 600. `grid-defaults`: ±200 on x and z, y = 0.
- Endpoint: the size is the whole extent, `pos_k = (i_k - (n_k - 1) / 2) · size_k / (n_k - 1)`, and 0
  for an axis with one clone: `grid-endpoint` (size 300, count 3 x 1 x 3): ±150 with step 150, y = 0.
- Matrices are translations only; colour, weight as C3.1.

Forms. With `u_k = (i_k - (n_k - 1) / 2) / ((n_k - 1) / 2)` in [-1, 1] (0 for `n_k = 1`):

- Cubic: every clone visible.
- Sphere: `r = |u|`. Visible iff `r <= 1`. `grid-sphere-form` (5 x 5 x 5): 125 entries, **33 visible** (the
  points with `i^2 + j^2 + k^2 <= 4` in step units: 1 + 6 + 12 + 8 + 6).
- Cylinder: axis Y, `r = sqrt(ux^2 + uz^2)`, visible iff `r <= 1`; `grid-cylinder-form`: **65 visible**
  (13 per y level; the visible x/z counts per level are 5, 15, 25, 15, 5).
- Fill `f` (all non-cubic forms): a clone is also hidden when `r < 1 - f`. `grid-fill-50` (sphere, 50 %):
  **32 visible**, the centre clone (r = 0) hidden, the six `r = 0.5` clones kept (the bound is
  inclusive). Fill 100 % hides nothing more. Linear in `r` is the only form checked against one
  data point: **parity-pending** (non-uniform sizes unobserved, normalisation per axis by the half
  extent is our reading).
  Cylinder fill (`grid-cyl-fill`, 5 x 5 x 5, Fill 50 %: **62 visible** of 125, 13 on each end layer
  `uy = ±1` and 12 on the three layers between): the hollow is the axis column only, `r = sqrt(ux^2 +
  uz^2) < 1 - f` hides a clone **unless it is on an end layer** (`|uy| = 1`): the centre column is
  hidden at y = 0, ±200 and visible at y = ±400. **Parity-pending** (one case, so whether the end
  layers stay filled for other fills or only the layer ring is unproven).
- Object form: `Unsupported` (brief 066).

Epsilon of the visibility tests: 1e-9 on `r`.

## C7. Honeycomb

The lattice is staggered. Terms: **lines** run along one axis, **along** is the other. For
`offset_direction = height` (default) the lines are columns: the line axis is the width axis (x for
orientation Z), `L = count_width` lines with spacing `size_width`, and along each line the height axis
with `N = count_height` half-cells of spacing `size_height / 2`. For `width` the roles swap: lines are rows
along height (`L = count_height`, spacing `size_height`), along width with `N = count_width` and
`size_width`. (cloner-3.md: the app's swap of counts and sizes when the direction changes is a UI
behaviour; the final values of `honeycomb-offset` are width 3 x height 2, 346.41 x 300, and that is
the case the rule below reproduces.)

- Line `c` (0-based) has coordinate `(c - (L - 1) / 2) · line_spacing`.
- Line `c` holds `K = ceil(N / 2)` clones if `c` is even, `floor(N / 2)` if odd. So a 10 x 10 honeycomb
  has 50 clones and a 3 x 2 one has 3 (**not** 6: `honeycomb-small`).
- Clone `m` (0-based) of line `c` is at `along = (m + off · (c mod 2)) · s - (N - 1) / 4 · s` with
  `s` the full along spacing (`size_height` for height, `size_width` for width) and `off` =
  `honeycomb_offset`. Centring is by the first term; the formula equals "half-cell lattice
  `j = 2m + (c mod 2)`, odd lines shifted by `off - 1/2` cells".
- List order: line by line, ascending along. Every clone has the identity frame.

Proofs: `honeycomb-defaults` (50 clones): x = -1350 .. 1350 step 300, even columns y = -779.42 + 346.41 k,
odd columns -606.22 + 346.41 k. `honeycomb-small` (3 x 2): (-300, -86.60), (0, 86.60), (300, -86.60).
`honeycomb-offset` (width, offset 25 %, 3 x 2): row 0 (y = -150): x = -173.21, 173.21; row 1 (y = 150):
x = -86.60 (`(0 + 0.25) · 346.41 - 173.21`).

End Point mode (`honeycomb_mode = end_point`, **parity-pending**): the sizes are the extents,
`line_spacing = size / (L - 1)` (0 for L = 1) and `s = 2 · size / (N - 1)` (0 for N = 1).
Orientation maps (width, height) to world axes: Z (XY) -> (x, y), X (ZY) -> (z, y), Y (XZ) -> (x, z);
the plane coordinate is 0. Only Z was observed (**parity-pending** for X and Y). Offset Variation and
Perpendicular Variation (seeded, **parity-pending**): along `+= var · s · (2 r - 1) / 2`, line
coordinate `+= pvar · line_spacing · (2 r - 1) / 2`. `Circle` and `Spline` forms are `Unsupported`
(nothing observed).

## C8. Instance modes

`instance_mode` is stored as an enum. All three modes expand to the same geometry (C3.6); the
viewport and renderer read the mode in brief 079 (Instance = per-clone object, Render Instance and
Multi-Instance = GPU/renderer instancing of one prototype).

## C9. Randomness

Radial Offset Variation and the Honeycomb variations use `forge_clone_engine::SplitMix64` seeded with
the parameter seed: deterministic and platform independent. The reference's sequence is not
reproducible from the observations (hypothesis 1: SplitMix64 `next_f64` of seed 1, with
`(2 r - 1)` or `r` as the draws, fits none of the eight line shifts of `honey-var`), so any case with a moving variation is
compared statistically (range and moments), never exactly (area file, "Randomness").

**Honeycomb variations** (`honey-var`, 10 x 10 Z (XY), Offset Direction Height, both variations 50 %,
seed 1, 50 clones): the variations move **whole lines**. All clones of a line (a column of 5 here) share one
shift; the first and the last line (columns 0 and 9) do not move; the eight interior lines shift
perpendicular by -106.2 .. +124.8 (x; line step 300, so within `± perpendicular_variation · line step`
= ±150) and along by -72.0 .. -11.2 (y, never positive; the along step is 346.41, so within
`-offset_variation · along step · u`, `u` in [0, 1)). Perpendicular shift of interior line k, in
order: 26.368, -60.396, 22.001, 99.473, -106.182, -89.244, 124.811, 98.085; along: -38.359, -71.953, -16.779,
-21.785, -65.264, -66.542, -11.237, -49.960. Our model draws two numbers per interior line (perpendicular
`r` in [-1, 1), then along `u` in [0, 1)); the exact sequence differs, so `honey-var` is a
`KNOWN_MISMATCH` (seed-sequence mismatch, structure derived). The earlier per-clone jitter was wrong
(**parity-pending**: the sign of the along shift and the draw order are inferred from one case).

## C10. Parity

Test: `crates/generators/tests/cloner_parity.rs` rebuilds every cloner case without effectors from
its `case.toml` (the app's labels, degrees, percent as typed; `count` etc. are mapped to the
mode-prefixed ids) under a `forge.cube` of size 20 and compares

- the clone data of the evaluated node with `samples.json` (`compare_samples`, expected tolerance 1e-4,
  visibility and count exact), and
- the expanded mesh with `mesh.obj` (`compare_meshes`, strict; after `export_weld` on both sides).

Matched: **31 of 34** cases run, each at both levels (samples and expanded mesh): linear 8
(`linear-defaults`, `-count-step`, `-cumulative`, `-mode-endpoint`, `-scale-step`, `-amount`,
`-endpoint-sr`, `-offset`), radial 7 (`radial-defaults`, `-plane-start-end`, `-align-off`, `-offset`,
`-xy`, `-var-1`, `-var-2`), grid 8 (`grid-defaults`, `-3x2x4`, `-endpoint`, `-sphere-form`,
`-cylinder-form`, `-fill-50`, `-cyl-fill`, `-sphere-uneven`), honeycomb 8 (`honeycomb-defaults`, `-small`,
`-offset`, `honey-endpoint`, `-odd-even`, `-orient-x`, `-orient-y`) and `transform-rs`. The case files may spell the mode
as the label (`Grid Array`, `Linear`); the test maps it to the id.

`KNOWN_MISMATCH` (2):
- `honey-var`: structure derived (C9: one shift per interior line, ends fixed) but the reference's random
  sequence is not reproducible, so the shifts differ.
- `linear-rstep` (R = (0, 20, 0), Step Rotation (0, 10, 0), count 5, P = (0, 50, 0)): the clone axes turn 20 degrees
  about X per clone (y axis of clone 1: (0, 0.866, -0.5)), but the positions follow
  `p_i = p_(i-1) + 50 · (0, cos 10i°, -sin 10i°)` (clone 1: (0, 49.240, -8.682), clone 2: (0, 96.225, -25.783),
  clone 4: (0, 177.829, -82.923)), i.e. a chain `(Rx(10°) · T(P))^i` for the position and an extra `10°` per clone for the
  orientation. Hypotheses tried: (1) orientation `R·t + Step·t` (30° per clone, fails); (2) the chain with the step
  rotation only (orientation 10°, fails); (3) the chain angle taken from R (20°, fails the positions). The description
  says the step rotation is "H" 10° but the file stores it in the second (P) slot: bad data. The redo
  `linear-rstep-2` (step rotation in H, read back `[10, 0, 0]`) **matches** with our R + Step Rotation rule (C4), so the
  rule needs no change; the old folder stays pinned.

`UNSUPPORTED` (1): `reset-coords-off` (superseded: its child position (0, 50, 0) and rotation are only in the
description). Its redo `reset-coords-off-2` records the child transform in `[params.child]` (position (0, 50, 0),
`rotation_deg` (30, 0, 0) = H, scale 1) and **matches**: with Reset Coordinates off the clone matrices stay the identity
(three clones) and the child's own transform is baked into the prototype (x/z turned by H 30° about Y, y 40..60);
the earlier "turn about X or Z" worry was a misreading of the missing transform. `order-iterate-2` (the three children's
settings recorded) matches like `order-iterate`. Matched cases: 37 (34 + `linear-rstep-2`, `order-iterate-2`,
`reset-coords-off-2`).

Not run here (pinned by count in the test, 54 effector cases and 13 others of the 101 cloner cases): the cases with
an effector (`[params.effector]` or `effector_count > 0`: briefs 068+), 8 Object-mode cases
(`object-*`: covered in `crates/generators/tests/cloner_object.rs`), `order-iterate`, `order-random`, `order-sort`, `blend-order` (several children and the
Clones modes: brief 067) and `nested`. `nested` is covered by a separate test
(`a_cloner_inside_a_cloner_matches_the_nested_mesh`): a Radial cloner (count 4) as the only
prototype of a Linear cloner (count 3) expands with no extra code and matches the mesh; the case
file does not record the inner cloner's parameters, so the claim stands on that rebuild.

Other tests: `crates/clone-engine/tests/modes.rs` (edge counts, seeds, forms, limits, honeycomb
counts), `crates/generators/tests/cloner.rs` (prototypes, consumption, reset coordinates,
instance modes, deformers, two prototypes, Make Editable), `crates/app/tests/cloner.rs`
(`object.create`, `param.set` in display units, undo/redo, save/load, OBJ export),
`crates/script/tests/script.rs` (Rhai).

## C11. Open data that would settle the parity-pending rules

One observation per bullet, each a small cloner case with a 20 cm cube child:

- Transform tab R, S, colour, weight (C3.1): set R = (0, 0, 45), S = (2, 1, 1), colour, weight 50 %
  with Linear count 2; and P with a rotated clone (Step Rotation H 90°, P = (50, 0, 0)).
- Linear: Offset 1 and 1.5 (`linear_offset`), Amount 50 %, Step Size 50 % with Step Rotation set,
  R (per step) set alone and together with Step Rotation, End Point with S and R set, and Cumulative
  with P/S set.
- Radial: Offset Variation 50 % with two seeds (compare statistically), Radius negative.
- Grid: Fill 50 % on Cylinder, Sphere with unequal sizes (e.g. 300, 200, 100) and unequal counts,
  Endpoint with a Sphere form, Fill 0 %.
- Honeycomb: Orientation X (ZY) and Y (XZ), Mode End Point, Offset Variation and Perpendicular
  Variation (two seeds), odd and even counts in both Offset Directions, Form Circle and Spline.
- Reset Coordinates off with a child placed away from the origin.
- UVW of the clones (`uvw`), e.g. through the Display = UV overlay or a shader reading it.


---

# Clone toolset: effector framework and the Plain effector (brief 068)

Wording as above ("effector", "clone data", "clone toolset"). Sources: parity observations
O8 [cloner.md](../parity/cloner.md), O14 [effectors-2.md](../parity/effectors-2.md),
O22 [effectors-3.md](../parity/effectors-3.md), design [E-clone-toolset.md](../roadmap/areas/E-clone-toolset.md),
fields [FIELDS.md](FIELDS.md). Golden data: the 14 Plain cases of `tests/golden/cloner/`
(`samples.json` level B, tolerance 1e-4; `mesh.obj` level A). Out of scope (Random is E11-E17): Step, Formula,
Time, Delay, Target (070), the other effectors (071), field-list falloff beyond what the Plain cases
need (072: layer blend modes, groups, direction channel).

## E1. Framework (`forge_clone_engine`)

```rust
pub trait Effector {
    fn apply(&self, data: &mut CloneData, falloff: &FieldList, ctx: &EffectorCtx)
        -> Result<(), CloneError>;
}
pub struct EffectorCtx<'a> { pub eval: &'a EvalCtx, pub cloner_world: DMat4 }
```

- An effector is a node of `Category::Effector`, registered by the `CloneToolset` plugin
  (so `object.create {type: "forge.effector.plain"}` works from UI, CLI, RPC, MCP and Rhai).
  `effector_from_node(scene, id)` builds the `Effector` of a node (`None` for a missing, disabled
  or non-effector node; an effector type without an implementation is `Unsupported`).
- **Linking model.** The cloner has the parameter `effectors`, a *link list* (`Value::LinkList`,
  `ParamKind::LinkList`: an ordered list of node ids, each node at most once, category `Effector`,
  not the node itself). It is set with `param.set {node: <cloner>, param: "effectors", value: [id, ...]}`
  (CLI, RPC, MCP, Rhai `set_param`), is undoable, saved, and its targets count for the cloner's
  cache revision. Effectors are applied **in list order** (`apply_effectors`): the second effector
  sees the clone data the first one left. A link to a deleted node is skipped.
- **Falloff.** An effector has the row list `fields` (`Value::LinkRows`, category `Field`, **top to bottom**, the order
  of FIELDS.md F1; per-row blend mode, strength and enable box, F7). `FieldList::from_param` builds the list and the effector samples it at the **world**
  position of each clone (`cloner_world · clone position`, taken when the effector starts). An empty
  list is value 1 (full effect, FIELDS.md F1). A clone's effect weight is `w = strength · value`.
- The result of all effectors is the clone data of the cloner; `clone_data(scene, cloner, ctx)` and the
  expanded geometry (C3) both run the effectors, so hidden clones (visibility) vanish from the mesh.
- Per-effector strength on the cloner (the extra field "Plain 100 %" a cloner gets for each effector in
  the app, O8 notes) is not modelled: every sample has 100 %. **parity-pending**; it would be a second
  list parameter next to `effectors`.

## E2. Plain effector parameters (`forge.effector.plain`)

Angles in radians, percent as fractions (`Unit::Percent`), lengths in cm; case files hold the labels,
degrees and percent as typed.

| id | label | kind | default | notes |
|---|---|---|---|---|
| `enabled` | Enabled | bool | on | off: the effector does nothing |
| `strength` | Strength | float, percent | 100 % | -1e20 .. 1e20 |
| `selection` | Selection | string | empty | stored only (clone selection tags are brief 078) |
| `deformation` | Deformation | enum | `off` | `off`, `object`, `point`, `polygon`; anything but `off` is `Unsupported` |
| `fields` | Fields | link list | empty | falloff, top to bottom |
| `maximum`, `minimum` | Maximum, Minimum | float, percent | 100 %, 0 % | used by Remap (E3) |
| `transform_mode` | Transform Mode | enum | `relative` | `relative`, `absolute`, `remap` |
| `transform_space` | Transform Space | enum | `node` | `node`, `effector`, `object` |
| `position` | Position | bool | **on** | Plain: P is on by default (O14 table) |
| `p` | P | vec3, distance | (0, 100, 0) | |
| `rotation` | Rotation | bool | off | |
| `r` | R | vec3 (h, p, b), angle | (0, 0, 0) | |
| `scale` | Scale | bool | off | |
| `uniform_scale` | Uniform Scale | bool | off | |
| `absolute_scale` | Absolute Scale | bool | off | |
| `s` | S | vec3 | (0, 0, 0) | used when Uniform Scale is off |
| `scale_value` | Scale | float | 0 | the value under Uniform Scale |
| `color_mode` | Color Mode | enum | `fields_color` | `off`, `effector_color`, `custom_color`, `fields_color` |
| `color` | Color | colour | white | the Custom Color swatch |
| `use_alpha_strength` | Use Alpha/Strength | bool | off | colour block (the app switches it on with Custom Color) |
| `blending_mode` | Blending Mode | enum | `default` | `default`, `add`, `subtract`, `multiply`, `divide`; all but `default` are `Unsupported` |
| `weight_transform`, `u_transform`, `v_transform` | Weight, U, V Transform | float, percent | 0 | |
| `modify_clone` | Modify Clone | float, percent | 0 | stored only (nothing observed) |
| `visibility` | Visibility | bool | off | |

`use_alpha_strength_weight` (the Weight tab's own Use Alpha/Strength) is not stored: no case sets it.

## E3. Transform modes and strength

`w = strength · value` (E1). Relative, Absolute and Remap all use `w`, except Remap, whose `w` is
remapped between Minimum and Maximum: `w' = minimum + w · (maximum - minimum)`.

- **Strength is linear** — `plain-strength-50`: P (0, 100, 0) at 50 % moves every clone up by 50.
- **Relative** adds the values: position `+ P w`, scale `+ S w` (a clone of scale 1 with Scale 2 gets 3:
  `plain-scale`, axis length 3.0), rotation `Rot(R w)` on top of the clone's, weight `+ weight_transform w`
  (`plain-weight`: 0.5), colour as E6.
- **Absolute**, **Remap**: with Position P (0, 100, 0) and default Minimum/Maximum both give exactly the
  Relative result (`plain-absolute`, `plain-remap`: y 100 ... 300). So the position delta is added in all
  three modes. The other channels in Absolute mode move towards the value, `old + (value - old) w` (as for
  Absolute Scale, E5); Remap and Minimum/Maximum values other than 0/100 % are **parity-pending**, as is
  every Absolute channel except position (O22 notes list "Remap with changed Minimum/Maximum" as not
  checked). Minimum and Maximum do nothing in Relative and Absolute (**parity-pending**).

## E4. Spaces, position and rotation

`Transform Space` picks the axes `F` (a rotation) in which the position delta and rotation are expressed:

- `node`: the clone's own axes (its matrix columns, normalised);
- `object`: the cloner's axes, i.e. the identity in cloner space;
- `effector`: the effector node's axes relative to the cloner, `normalised(cloner_world⁻¹ · effector_world)`.

Position: `pos += F (P w)`. Rotation: `M3 = F · Rot(R w) · Fᵀ · M3` (the delta turns about the space's axes;
`Rot` is the HPB rotation of TRANSFORM.md §3, `Ry(-h) Rx(-p) Rz(-b)`; for the node space this is `M3 · Rot` when
the clone is not scaled). Scale does not enter `F`, so a scaled clone moves by `P w`, not `P w · scale`:
`two-effectors-swapped` (scale 3 first, then P (0, 100, 0)) gives y 100 ... 300, equal to `two-effectors`.

Proofs: `plain-rotation` (R.H 90 degrees: first clone x axis (0, 0, 1), z axis (-1, 0, 0), positions kept);
`plain-effector-space` (effector rotated H 90 degrees, P along its Y axis: no change; a P in another direction
would show the rotation: **parity-pending**, the node-space and object-space formulas of a rotated clone are
too, no sample has a rotated clone with P set). Only the position and rotation use the space; scale
always acts along the clone's own axes (**parity-pending**).

## E5. Scale

With Scale on: the value is `scale_value` on all three axes if Uniform Scale is on, else `S`. For the clone's
axis length `l`: Relative `l + value w`, Absolute (Absolute Scale on, or Transform Mode Absolute)
`l + (value - l) w`. The column is scaled by `new / l` (zero-length axes are left alone). Proof: `plain-scale`
(Relative, uniform 2: 3.0). Absolute Scale is **parity-pending** (own reading: the value is the final scale).

## E6. Colour

`Color Mode`: `off` leaves the colour; `custom_color` blends towards `color`; `fields_color` blends towards the
colour channel of the falloff field list (the field's Color, FIELDS.md F2) and does nothing when the list is
empty; `effector_color` is `Unsupported` (nothing observed). The blend is `c + (target - c) · k` with
`k = clamp(value · (strength if use_alpha_strength else 1), 0, 1)`.

Proofs: `plain-color` (Custom Color red: every clone (1, 0, 0), `use_alpha_strength` on, strength 100 %);
`plain-field-linear` (Linear field, Y+, length 100, clones at y = 0, 50, 100 have field value 0.5, 0.75, 1;
the field colour (0.192, 0.434, 0.2336) gives (0.596, 0.717, 0.6168), (0.394, 0.5755, 0.4252), (0.192, 0.434,
0.2336) = `1 + (c - 1) · value`); `plain-field-spherical` (value 1 clones take the field colour, value 0 stay
white). Whether the effector strength scales the colour only through `use_alpha_strength` is **parity-pending**
(every colour sample has strength 100 %).

## E7. Weight, U, V, visibility

`weight += weight_transform · w` (`plain-weight`: 0.5), `uvw.x += u_transform · w`, `uvw.y += v_transform · w`
(own design, **parity-pending**: no UV sample). Visibility on hides a clone whose `w < 0.5` (own design,
**parity-pending**: every sample has Visibility off). `modify_clone` is stored only.

## E8. Field falloff in the Plain cases

`plain-field-spherical`: a Spherical field (size 100, inner offset 50 %) at (0, 100, 0): clones at y = 50, 100, 150
have value 1 (moved up by 100: y 150, 200, 250), the clones at 0 and 200 value 0 (stay at 0 and 200).
`plain-field-linear`: positions y 50, 125, 200, 250, 300 = `y + 100 · value`. The empty list is value 1
(`linear-plain`: every clone moves by the full 100). The Linear field default direction in the app is X+, so a clone
column at x = 0 has value 0.5 (`linear-plain-field`: y + 50).

### Field lists of any shape (brief 072)
Every effector type reads the same `fields` row list through `FieldList::from_param` (no second copy of the layer logic):
Plain, Random, Step, Formula, Time, Target and Delay all take `falloff = list.sample(world position of the clone)`, so
everything F7 and F8 define applies unchanged to an effector: blend modes and strengths of the rows, Group fields,
modifier layers (Remap, Invert, Clamp, Colorize, identity Curve), field transforms (a moved or scaled field acts in
world space) and switched-off or disabled rows (left out). A modifier layer is evaluated over the rows below it
(F8), so Invert over a Spherical gives `1 - v`. A field type or modifier without rules (`Unsupported` in the fields crate,
e.g. Step layer, Shader field) makes the effector report `CloneError::Unsupported`. Rules here are **parity-pending**
beyond the single-layer cases of E8: no observation holds more than one layer on an effector.
Test: `crates/app/tests/cloner.rs::effector_falloff_takes_a_whole_field_list`.

The field's colour channel is its `Color` parameter (a constant colour, white by default; F2 adds
the `color` parameter). Colour of a multi-layer list and the colour-mode variants of the fields are 072.

## E9. Parity

Test: `crates/generators/tests/cloner_parity.rs::plain_effector_cases_match_the_reference` rebuilds the 14 cloner
cases whose effectors are all Plain (`linear-plain`, `linear-plain-field`, `plain-absolute`, `plain-color`,
`plain-effector-space`, `plain-field-linear`, `plain-field-spherical`, `plain-remap`, `plain-rotation`,
`plain-scale`, `plain-strength-50`, `plain-weight`, `two-effectors`, `two-effectors-swapped`), with the
effectors linked through the `effectors` list and the fields through `fields`, and compares the clone data
(level B) and the expanded mesh (level A) like C10.

Matched: **14 of 14** Plain cases, each at both levels (clone data and expanded mesh): 13 through the case
files (`linear-plain`, `plain-absolute`, `plain-color`, `plain-effector-space`, `plain-field-linear`,
`plain-field-spherical`, `plain-remap`, `plain-rotation`, `plain-scale`, `plain-strength-50`, `plain-weight`,
`two-effectors`, `two-effectors-swapped`) and `linear-plain-field` by its own test
(`linear_plain_field_matches_with_the_default_linear_field`): its `case.toml` has **no field table**, only the
description (a Linear field with defaults, only layer), so the test builds the default Linear field at the
origin. The position and the expanded mesh are checked on their own (y + 50 for x = 0); the field's Color was
not recorded either, so it is read back from the samples with the E6 rule (`1 + (c - 1) · 0.5` per channel),
which makes the colour part a consistency check, not an independent match. `PLAIN_KNOWN_MISMATCH` is empty.
The earlier test (C10) still runs the same 19 + 35 + 12 split and is unchanged. Other tests:
`crates/clone-engine/tests/effectors.rs` (spaces with a rotated clone, scale, Remap, visibility, weight/UV,
`Unsupported`), `crates/generators/tests/cloner_parity.rs` (list order and the link checks),
`crates/app/tests/cloner.rs` (`object.create` + `param.set` link, undo, save/load), `crates/script/tests/script.rs`.

## E10. Open data that would settle the parity-pending rules

One small cloner case each (Linear, count 5, step (0, 50, 0), 20 cm cube):

- Node / Object / Effector space with a rotated clone: Step Rotation H 90 degrees and P (50, 0, 0); the effector
  rotated H 90 degrees with P (50, 0, 0); Rotation R.P 45 degrees in each space.
- Absolute mode with Rotation (R.H 45 degrees), Scale (S 2 and Absolute Scale on and off) and Weight Transform 50 %.
- Remap with Minimum 20 %, Maximum 80 %, P (0, 100, 0) (clone 0 to 4 with values 0 and 1 and a field).
- Colour: Custom Color red at Strength 50 % with Use Alpha/Strength on and off; Fields Color with a field value of
  0.5 at Strength 50 %; Effector Color; Blending Mode Add / Multiply.
- U Transform and V Transform 50 % with the Display = UV overlay; Visibility on with a Linear field (values 0.25 and
  0.75), and with strength 50 %.
- The Linear field of `linear-plain-field`: its case file holds no `[params.effector.field]` table; re-observe it with the field's Color recorded (the colour of the app's new field is not white).
- Two effectors where the order changes the result: Plain Absolute Scale 2 then Plain Relative Scale 1.
- The per-effector strength on the cloner (the "Plain" field of the cloner) at 50 %.

## E11. Random effector (`forge.effector.random`)

Sources: O22 [effectors-3.md](../parity/effectors-3.md) (parameter table, cases `random-gaussian`, `random-noise`,
`random-indexed`, `random-scale`), O8 [cloner.md](../parity/cloner.md) (`grid-random`, `grid-random-seed`: Seed 12345 and P
(50, 50, 50) are the defaults), O11 [followups-1.md](../parity/followups-1.md) (`grid-random-seed-7`, `grid-random-rot`),
noise from [NOISE.md](NOISE.md). The Random effector is the Plain effector (E2-E8) with a different source of the per-clone
value: everything that is not listed here (Strength, Transform Mode, Transform Space, P / R / S, Color Mode, Weight, U, V,
Visibility, Selection, Deformation, the `fields` falloff, Enabled) is **shared code and the same parameters** as Plain.

### Parameters

Only the differences from the Plain table (E2) and the new parameters:

| id | label | kind | default | notes |
|---|---|---|---|---|
| `maximum`, `minimum` | Maximum, Minimum | float, percent | 100 %, **-100 %** | the range of the random value (E12), in every Transform Mode |
| `position` | Position | bool | on | |
| `p` | P | vec3 | **(50, 50, 50)** | |
| `color_mode` | Color Mode | enum | **`off`** | |
| `uniform_scale`, `scale_value`, `s` | | | on, 1, (1, 1, 1) | the O22 probe set Uniform Scale on and Scale 1; the hidden defaults were not read (**parity-pending**) |
| `random_mode` | Random Mode | enum | `random` | `random`, `gaussian`, `noise`, `turbulence`, `sorted`; `sorted` is `Unsupported` (listed in O22, nothing observed) |
| `synchronized` | Synchronized | bool | off | one value per clone for all channels (E12) |
| `indexed` | Indexed | bool | off | a switch next to Random Mode, not a list entry (`random-indexed`) |
| `seed` | Seed | int | 12345 | 0 .. 1e9 |
| `space` | Space | enum | `global` | `global`, `uv` (E13) |
| `animation_speed` | Animation Speed | float, percent | 100 % | noise modes only (E13) |
| `noise_scale` | Scale | float, percent | 100 % | noise modes only (E13); the case files call it `scale_percent` |

`use_alpha_strength_weight` is not stored (as E2).

### E12. The random value

Each clone gets a **value** `v = minimum + r (maximum - minimum)` with `r` in `[0, 1]`, per channel, from the mode below.
With the defaults `v` is in `[-1, 1]`. The effect weight of a channel is `w_c = strength · falloff · v_c` (the Plain `w`
with `v_c` in place of 1; falloff as E1; **Remap does not remap again**: Minimum and Maximum are the range of `v`
in every mode). Then, per channel, exactly the Plain rules:

- Position: `pos += F (P ∘ w_pos)` (`∘` is the per-axis product: every axis has its own draw); `grid-random`: the offset of
  an axis is `50 v` cm.
- Rotation: `Rot(R ∘ w_rot)` in the space `F`. Scale: axis `k` gets `l + S_k w_scale,k`; Uniform Scale uses **one** draw for the
  three axes (`random-scale`: x, y, z lengths equal, `1 + v` in `[0, 2]`). Weight, U, V and Visibility use a seventh draw.
- Colour (E6) does not use the random value; it is the falloff's (Random Color Mode default is Off).

Draws per clone, in this order: position (3), rotation (3), scale (3, or 1 if Uniform Scale is on), weight (1); each is drawn
whether its channel is on or not, so switching a channel never changes the others (own design). Synchronized uses the first
draw of the clone for all channels.

`random` (uniform) and `gaussian` use a counter-based generator: `u(seed, key, k)` is the SplitMix64 finaliser of
`seed · 0x9E3779B97F4A7C15 + key · 0xD1B54A32D192ED03 + k · 0x94D049BB133111EB`, mapped to `[0, 1)` with 53 bits. The `key` of a
clone is its **list position** `i` (Random) or its **clone index** `data.index[i]` (Indexed): with Indexed a clone keeps
its value when the list is permuted or other clones are hidden or added, which the plain Random does not promise
(own design; the observed Indexed data does not reveal the rule, E15).

- `random`: `r = u`.
- `gaussian`: `r = clamp(0.5 + g / 12, 0, 1)` with `g` the standard normal from Box-Muller of two draws `u(.., k)` and
  `u(.., k + 32)`. The standard deviation of `v` is `1/6 (max - min)/2`: O22 `random-gaussian` has 15 values, `var(v)` 0.024,
  `|v| <= 0.27`, mean 0.025; a standard deviation of 1/6 gives `var` 0.0278.
- `noise`, `turbulence`: E13. `sorted`: `Unsupported`.

### E13. Noise and Turbulence, Space, animation

`noise` samples forge-noise Perlin (`NoiseKind::Perlin`, 1 octave), `turbulence` samples `NoiseKind::Turbulence`
(1 octave, NOISE.md N4), both 4D: `n = noise.sample4(q, t)` with `q = point / L`, feature size `L = 300 cm · noise_scale`, the
point in **world** space (the world position of the clone, like a field) for Space Global and `uvw · 4 L` (clone UVW times four feature sizes) for Space UV
(**parity-pending**: no UV sample). `t = time · animation_speed` (seconds, speed as a fraction), so frame 0 is static.
`r = (n + 1) / 2`. The three channel groups use three noises of seeds `octave_seed(seed, 1 + k)`, `k` = 0 position,
1 rotation, 2 scale and weight; component `k` of a channel samples its noise at
`point + k (71.3, 37.7, 19.9)` cm (own design, decorrelates x, y, z; weight is component 3 of the scale noise). Synchronized uses the position noise at the point
for everything. Continuity in position is the rule the cases show: neighbouring clones have close values.

Choice of `L = 300`: O22 `random-noise` has clones 50 cm apart along Y and a root-mean-square step between neighbours of
0.119 per channel; our Perlin gives 0.115 at `L = 300` (0.168 at 200, 0.087 at 400), measured over 4800 pairs
(scratch experiment, `crates/clone-engine/tests/random.rs::the_feature_size_reproduces_the_observed_smoothness` repeats it).

### E14. Pending

Parity-pending: all exact random values (E15), Turbulence mode (no sample), Synchronized, Space UV, Animation Speed, Sorted, the
hidden Scale defaults, Absolute/Remap with random channels, the Scale checkbox with several draws, the channel decorrelation of the noise
(at the first clone the three observed channels are within 0.01, at the second within 0.26: no model of ours reproduces both).

### E15. The reference's random sequence (three hypotheses)

The cases give 59 values in `[-1, 1]` for Random mode (the 27 position offsets of `grid-random` / `grid-random-seed` with
Seed 12345, the 27 of `grid-random-seed-7` with Seed 7, the 5 uniform scales of `random-scale`), the position offsets
divided by P (50); values have 1e-12 precision. Hypotheses, each checked against every observed value (a value matches if
some output equals it within 1e-9, as `r`, `2r - 1` or `1 - 2r`):

1. **One sequential stream** of a standard generator started at the Seed (any order of use and any number of skipped
   outputs: the first 20 000 outputs are searched): MT19937 (32 bit and 53 bit), minstd (16807 and 48271), the LCGs
   of glibc, ANSI C, MSVC, Numerical Recipes and Java, xorshift32, xorshift64, SplitMix64, PCG32, Python's `random`.
   No value matches.
2. **A generator re-seeded per clone** with `seed + i`, `seed xor i`, `seed (i + 1)`, `i`, `seed 65537 + i`, `seed + i + 1`
   (first 8 outputs, same generators). No value matches. The same for `random-indexed` with the seeds `i`, `i + 1`,
   `seed + i`, `seed xor i`, `seed (i + 1)`, `seed`. No match.
3. **A counter-based hash** of `(seed, clone, channel)`: murmur3 `fmix32` / `fmix64`, Wang, PCG and Jenkins hashes,
   with the combinations `seed + 3i + c`, `seed (3i + c + 1)`, `(seed << 8) xor (3i + c)`, `seed xor (3i + c)`,
   `(3i + c) + seed · 0x9E3779B1`, and a chained hash. No match.

The budget is spent: Random, Gaussian and Indexed are compared **statistically** (E16), and the reference's sequence is
not reproduced. (Our own generator above is deterministic and platform independent, C9.)

### E16. Parity

Test: `crates/generators/tests/cloner_parity.rs::random_effector_cases_match_the_reference` rebuilds the 8 cloner cases with a
Random effector (`random-gaussian`, `random-noise`, `random-indexed`, `random-scale`, `grid-random`, `grid-random-seed`,
`grid-random-seed-7`, `grid-random-rot`) from their `case.toml`, with the effector linked through `effectors`, and compares:

- **exact (clone data, 1e-4)**: the clone count, every `visible` flag, colour and weight, and every channel the effector does not
  randomise (the position of `random-scale` and `grid-random-rot`, scale and rotation of the position cases) 8 of 8 cases;
  the expanded mesh keeps the reference's vertex and face counts (level A counts) 8 of 8;
- **statistical (randomised channel)**: the observed values `v` (offset / P, angle / R, scale - 1) must lie in `[-1, 1]` (all
  cases), and
  - Random mode (`grid-random`, `grid-random-seed`, `grid-random-seed-7`, `random-scale`, `grid-random-rot`): the pooled mean of
    the observed values and variance against the model `U[-1, 1]` (mean 0, variance 1/3): `|mean| <= 3 sqrt(1/3 / n)` and
    `|var - 1/3| <= 3 · 0.298 / sqrt(n)` (the standard deviation of the sample variance of a uniform, `sqrt((1/5 - 1/9) / n)`,
    is `0.298 / sqrt(n)`); the position and scale values pooled: n = 59, observed mean 0.047, variance 0.337 (inside the band); the rotation values of `grid-random-rot`
    (angle / R, n = 27, mean 0.195, variance 0.268) are checked against the same band separately;
  - Gaussian (`random-gaussian`, n = 15): `|mean| <= 3 · (1/6) / sqrt(n)`, `|var - 1/36| <= 3 · (1/36) sqrt(2 / n)`;
  - Noise (`random-noise`, n = 15): every value in `[-1, 1]`, and the root-mean-square step between consecutive clones within a
    factor of 2 of ours on the same five clone positions (observed 0.119, ours 0.157; 0.115 averaged over 40 seeds);
  - Indexed (`random-indexed`, n = 15): range only. **Not matched**: the observed mean is -0.555 and the variance 0.117, more than 3.7
    standard errors from a uniform's mean (0): the reference's Indexed values are not uniform in `[-1, 1]`. 15 values cannot fit another
    distribution, so the mean and variance are listed in the test as `KNOWN_MISMATCH` (asserted to stay outside the uniform band)
    and Indexed is checked for determinism per index in `crates/clone-engine/tests/random.rs`.
  The tolerances are 3 standard errors of the model distribution; they were chosen from the model, not from the observed
  values, and are never used for an exact channel.

Other tests: `crates/clone-engine/tests/random.rs` (large-sample moments of each mode, seed sensitivity, Indexed stable under
permutation, Synchronized, channels independent, noise continuity, Minimum / Maximum range, Unsupported Sorted).

### E17. Open data that would settle the pending rules

- Random Mode Random with Count 50 and Seed 12345, Position only (distribution, and one more seed to look for a hash structure);
  the same with Seed 0 and Seed 1 (are small seeds special?) and with Strength 50 %.
- Indexed with Count 50 (the distribution; the observed mean is -0.555), and Indexed with Count 5 and 8 (does clone 0 keep its value?).
- Gaussian with Count 50 (the standard deviation and clipping), Noise with Count 20 along Y at 10 cm steps, Scale 50 % and 200 %
  (feature size), Animation Speed 0 % and frame 30 (time axis), Space UV, Turbulence, Synchronized, Sorted.
- Random with Rotation (R.H only, other axes 0): are the three position axes independent draws from the same stream as the rotation?

---

# Clone toolset: Step, Formula, Time, Delay and Target effectors (brief 070)

Wording as above. Sources: parity observation O14 [effectors-2.md](../parity/effectors-2.md) (parameter tables
and the cases `step-defaults`, `step-position`, `formula-defaults`, `formula-frame-10`, `target-defaults`,
`time-frame-30` in `tests/golden/cloner/`), the Plain rules E2-E8, the formula language of [EXPR.md](EXPR.md)
(degrees mode, E5). Level B (`samples.json`, tolerance 1e-4) and level A (`mesh.obj`). The Delay's first design was own; the follow-up cases (below) settled Blend and Spring.

Step, Formula and Time are the Plain effector (E2-E8) with a different source of the effect weight `w`; they share
the Plain parameter table (E2) and differ in the defaults below. Target and Delay have their own parameters.

## E18. Defaults of the shared parameter block

| id | Plain | Step | Formula | Time |
|---|---|---|---|---|
| `position` / `p` | on / (0, 100, 0) | off / (0, 0, 0) | on / (50, 0, 0) | off / (0, 0, 0) |
| `rotation` / `r` | off / 0 | off / 0 | off / 0 | **on** / (90, 0, 0) degrees |
| `scale` | off | **on** | on | off |
| `uniform_scale` / `scale_value` | off / 0 | on / 1 | on / 1 | off / 0 |
| `color_mode` | `fields_color` | `off` | `off` | `off` |
| `minimum` | 0 | 0 | 0 | 0 |

(O14 tables; Time's P default is not recorded, 0 is own design.) The other parameters (strength, selection, deformation,
fields, spaces, mode, colour, weight, U, V, visibility, maximum) are as E2. All four use the Plain application
(E3-E8) with the weight below; `w = amount(value · source)` where `amount(x) = strength · x`, remapped between
Minimum and Maximum in Remap mode (E3) and `value` is the falloff value at the clone (E1).

## E19. Step (`forge.effector.step`)

Extra parameters: `curve` (enum, `linear`; also `ease_in`, `ease_out`, `ease_in_out` and `custom`: the app's editable spline, two
Bezier knots held in `curve_start`, `curve_start_handle`, `curve_end`, `curve_end_handle`, x and y of each), `step_gap` (int,
0, 0 .. 2147483647).

- Source `s = curve(i / (n - 1))`, `i` the position in the clone list, `n` the clone count; `s = 0` if `n = 1`.
  `ease_in(t) = t²`, `ease_out(t) = 1 - (1 - t)²`, `ease_in_out(t) = t²(3 - 2t)`.
- Proofs: `step-defaults` (Scale 1 relative: clone scale 1, 1.25, 1.5, 1.75, 2); `step-position` (P (0, 100, 0):
  y = 0, 75, 150, 225, 300 = `50 i + 100 s`; the app returned 74.9999 and 225.0001). **Known mismatch (level A of
  `step-position`)**: the reference weights are `0.2499990830`, `0.5000008583`, `0.7500009170` instead of 0.25, 0.5, 0.75
  (errors -9.17e-7, +8.58e-7, +9.17e-7: its default spline is not an exact straight line, or its solver stops early). The
  clone data (level B) still match at 1e-4 (largest error 9.2e-5), but the expanded cube corners add the position and the
  scale error to 1.01e-4, over the 1e-4 mesh tolerance. Three hypotheses (exact line; float32 weights, which does not fit the
  digits; the offsets as multiples of 2^-20, 0.96, 0.90, 0.96, which has no rule from three points) gave no rule, so the
  curve stays the straight line and the case is pinned in the test (`STFT_MESH_MISMATCH`). A second case with Count 9 or a
  changed spline would give more points.
- **Step Gap** `g`: the clones come in runs of `g + 1`, and every clone takes the weight of its run's first clone, measured
  against the last clone: `s = curve((i div (g + 1)) (g + 1) / (n - 1))`. Proofs: `step-gap-1` (Count 5, Scale 1: scales 1, 1,
  1.5, 1.5, 2) and `step-gap-2` (1, 1, 1, 1.75, 1.75).
- **Custom curve:** the knots are `(position, handle)` pairs in the curve's own units (x along the list, y the weight) and the
  Bezier control point of a knot is `position + 4/3 * handle`; the weight is `y` at the parameter where `x` equals
  `i / (n - 1)` (solved numerically). Proof: `step-spline` (knots (0, 0) handle (1/3, 0) and (1, 1) handle (-1/3, -2/3), i.e.
  control points x 0, 4/9, 5/9, 1 and y 0, 0, 1/9, 1: weights 0, 0.02274, 0.16667, 0.52274, 1; largest error 7.9e-5, the
  reference solver's usual 1e-4-scale noise as in `step-position`). The `4/3` was fitted from the three interior weights
  (ninths come out exactly); a second curve would confirm it (**parity-pending**). The default handles are 0.25 (a straight line).
- **Known mismatch (level A of `step-count-9`):** like `step-position` the clone data match (Count 9, 9.2e-5) and the expanded mesh
  does not, for the same reason (pinned in `STFT_MESH_MISMATCH`).

## E20. Formula (`forge.effector.formula`)

Extra parameters: `formula` (string, `sin(((id / count) + t) * f * 360.0)`), `t_project_time` (float, 1),
`f_frequency` (float, 1). The formula is parsed with `Expr::parse_degrees` (EXPR.md E5: the default text needs
degrees) with the variables, in this order: `px py pz` (clone position, cloner space), `rx ry rz` (clone rotation
h, p, b in radians), `sx sy sz` (axis lengths), `u v w` (the clone's uvw), `id` (list position, 0-based), `count`
(clone count), `falloff` (the falloff value), `t`, `f`.

- `source = formula(...)`, a non-finite result counts as 0. A formula that does not parse is `Unsupported`.
- `t = project time in seconds × t_project_time`, `f = f_frequency`.
- Proofs: `formula-defaults` (frame 0: x = 50 sin(72 i degrees) = 0, 47.5528, 29.3893, -29.3893, -47.5528, scale
  `1 + sin` = 1, 1.9511, 1.5878, 0.4122, 0.0489); `formula-frame-10` (frame 10 at 30 fps, `t = 1/3`: clone 0
  `sin(120 degrees)` · 50 = 43.3013, clone 1 `sin(192 degrees)` · 50 = -10.3956 ...). The case shows the result follows the
  document time although the `t` field reads 1, so `t` is the time, not the field. Frame 0 cannot tell `t = time` from
  `t = time + 1` (period 360 degrees). Settled: `formula-project-time-2` (`t` = 2 at frame 10) and `formula-frequency-2` (`f` = 2) match at
  1e-15 with `t = time · t_project_time` and `f` as a multiplier.
- Meaning of `px ... w` and `falloff` (value before or inside the formula; the result is also multiplied by the field
  value here) is **parity-pending**: no case uses them.

## E21. Time (`forge.effector.time`)

- Source `s = project time in seconds`: the values are per second. Proof: `time-frame-30` (1 s): P (0, 100, 0) adds 100
  (y = 100 ... 300) and R.H 90 degrees turns every clone as in `plain-rotation` (x axis (0, 0, 1), z axis (-1, 0, 0)).
  Settled: `time-frame-15` and `time-frame-60` match exactly (P adds 50 and 200: linear in the time in seconds).
- Clone `time` (the per-clone time offset of C1) is not written (nothing observed).

## E22. Target (`forge.effector.target`)

Parameters: `enabled`, `strength` (100 %), `selection`, `deformation` (as E2), `fields`, `target_mode` (`object_target`
default; `look_at_camera`, `next_node`, `previous_node`, `field_direction`), `target_object` (link, empty), `use_pitch`
(on), `reverse_heading` (off), `up_vector` (`none` default; `object`, `x_plus`, `x_minus`, `y_plus`, `y_minus`,
`z_plus`, `z_minus`), `repel` (off). Positions never change.

- A clone is turned so that its **z axis points at the target** and its **x axis stays horizontal** (perpendicular to the up
  axis, which is Y for `none`): with `u` the up axis and `d` the unit direction to the target, `x = normalise(u × d)`,
  `y = d × x`, `z = d`. The clone's axis lengths are kept. The target point is the target object's world position taken
  into cloner space.
- Strength and falloff: the clone's rotation is the spherical interpolation from its own rotation to the aimed one by
  `w = strength · value` (so 0 leaves it, 1 aims fully).
- `use_pitch` off removes the component of `d` along `u` before aiming (heading only). `reverse_heading` turns the whole
  direction around, `d = -d` (pitch included; `target-reverse`: clone at y = 50 has `z = (0, 0.1644, -0.9864)`, `x = (-1, 0, 0)`). A clone at the target (zero direction) or a direction parallel to `u`
  keeps its rotation.
- `next_node` / `previous_node` aim at the next / previous clone (before this effector), the last / first clone keeps the
  direction of the neighbouring pair (**parity-pending**).
- **Up Vector** other than None (`up` = the chosen world axis): the clone looks **away** from the target, `z = -d`, its X axis
  is the up axis made perpendicular to `z`, `x = normalise(up - (up . z) z)`, and `y = z x x`. Proofs: `target-up-x` (clone 0:
  `x = (1, 0, 0)`, `y = (0, -1, 0)`, `z = (0, 0, -1)`) and `target-up-y` (`x = (0, 0.9864, 0.1644)`, `y = (1, 0, 0)`,
  `z = (0, 0.1644, -0.9864)` for the clone at y = 50). The negative axes and Z are the same rule by assumption (**parity-pending**);
  with Reverse Heading as well the combination is unobserved.
- Proof: `target-defaults` (Null at (0, 0, 300), a Null made through its menu command; the case file records the position
  only in the description): clone 0 at the origin keeps the identity frame (`d = +z`); clone at y = 200 has
  `z = (0, -200, 300) / 360.56 = (0, -0.5547, 0.8321)`, `y = (0, 0.8321, 0.5547)`, `x = (1, 0, 0)`, all five clones at 1e-4;
  `target-offaxis` (Null at (200, 100, 300)), `target-strength-50` and `target-next-node` match with the same rules.
- `Unsupported`: modes `look_at_camera` and `field_direction`, Up Vector `object` (needs an up-object link), `repel`
  (nothing observed), a missing Target Object in Object Target mode does nothing.

## E23. Delay (`forge.effector.delay`)

Parameters: `enabled`, `strength` (**50 %**, 0 .. 100 %), `selection`, `deformation`, `fields`, `mode` (`even` default,
`blend`, `spring`), `position`, `rotation`, `scale` (all on). Blend and Spring are observed (`delay-blend-f1`, `-f2`, `-f5`,
`-f10`, `delay-spring-f5`: all five match at 1e-4 level B and A); Even has no case (`delay-even-f5` was not observed) and
shares Blend's rule (**parity-pending**), the replay window is own design.

A clone follows its input (the clone data the earlier effectors left) with a delay. The delay needs the earlier frames,
so it is computed without hidden state: the effectors before the Delay and the cloner itself are evaluated again at the
frames `f - W ... f - 1` (`W = 240`, frame = `round(time · fps)`, none for frame 0 or when the frame count changes) and
the smoothing runs from the oldest of them (whose output is its input) to the current frame. The result depends only on the
scene and the frame, so scrubbing, undo and saved files stay deterministic. Only parameters of the *current* scene are
replayed (a parameter animated by a track holds its current-frame value in the replay); changes that come from time
(Time, Formula, Random noise) are followed. `s = clamp(strength · value, 0, 1)` per clone (`value`: the falloff value), `x`
the input pose, `y` the output pose (position, rotation quaternion, axis lengths; a channel whose switch is off is not
smoothed):

The smoothing keeps `p = 0.999 · s` of the previous output per frame (the factor is fitted: one frame at strength 50 % moves
0.5005 of the way, `delay-blend-f1`; the exponential recurrence with `a = 0.5005` gives 0.5005, 1.2510, 4.0330, 9.0030 frames
of Time input at frames 1, 2, 5, 10, observed 0.5005, 1.2510, 4.0330, 9.0030):

- `blend` (and `even`): exponential smoothing `y_f = y_(f-1) + (x_f - y_(f-1)) (1 - p)` (rotation: slerp), the first frame
  `y = x`. (The earlier one-frame `lerp(x_(f-1), x_f)` reading of Blend was wrong.)
- `spring`: a velocity `v_f = v_(f-1) p + (x_f - y_(f-1)) (1 - p)`, `y_f = y_(f-1) + v_f` per channel (rotation: the rotation vector
  of `x · y⁻¹`), `v = 0` at the first frame (`delay-spring-f5`: 17.082914, ours 17.0829138; without the 0.999: 17.0833).

At frame 0 the output equals the input (`delay-skip`). At `s = 0` the output equals the input at every frame. Colour,
weight, visibility and uvw are not changed. More than one Delay in one cloner's list is `Unsupported` (nested replays).

## E24. Parity

Test: `crates/generators/tests/cloner_parity.rs::step_formula_time_target_cases_match_the_reference` rebuilds the six
cases above and the 19 menu-created follow-ups (`params_of` reads `[params.cloner]`, `[params.effectors_N]` with the type told by their rows, `frame`/`fps`, a Target's Null, a Step curve) from their `case.toml` (effectors created as nodes, the Null of `target-defaults` at (0, 0, 300), the
document frame as `EvalCtx::at_frame(frame, 30)`), compares clone data (level B) and expanded mesh (level A). Result: **6 of 6 at level B** (`step-defaults`,
`step-position`, `formula-defaults`, `formula-frame-10`, `target-defaults`, `time-frame-30`) and **5 of 6 at level A**
(`step-position` is the known mismatch of E19). Unit tests:
`crates/clone-engine/tests/step_formula_time_target_delay.rs` (curves, the formula variables and error, Time scaling, Target
modes and Unsupported, the Delay modes, frame 0 identity, determinism). The Delay replay is tested in
`crates/generators/tests/delay.rs` through a Time effector.

## E25. Open data that would settle the pending rules

- Step: a changed spline (e.g. ease), Step Gap 1 and 2, Count 1; Position with Absolute Transform Mode.
- Formula: `t - Project Time` 2 at frame 10 (is `t` multiplied or added?), Frequency 2, a formula using `px` and `falloff`
  with a Spherical field, Rotation on with R.H 90 degrees, Formula `id` to check 0-based.
- Time: the same case at frame 15 and 60 (linear in time?), Rotation only at frame 15.
- Target: Reverse Heading on; Use Pitch off; Up Vector Y+, X+; Next Node and Previous Node (all clones, first and last);
  Strength 50 %; a Target Object at (0, 100, 0) (clone at the same height), at (300, 0, 0).
- Delay: Mode Even / Blend / Spring with a Time effector before it, Strength 50 %, frames 0, 1, 2, 5, 10 (record the
  clone data at each).

---

# Clone toolset: Matrix (brief 073)

Sources: observation O31 [clone-generators-2.md](../parity/clone-generators-2.md) (parameter tables,
cases `matrix/defaults`, `matrix/linear`), the cloner sections C1-C9 above. Golden data: `tests/golden/matrix/`
(`samples.json` kind `clones`, tolerance 1e-4; `mesh.obj` is the observer's separate 20 cm cube).

## M1. The Matrix object (`forge.matrix`)

A Matrix makes clone data and no geometry. It has the cloner's distribution modes and Transform tab
unchanged (C3-C7, C3.1) and none of the prototype options: it has no `clones_mode`, `reset_coordinates`,
`fix_texture`, `instance_mode` or `viewport_mode`. Its parameter ids are the cloner's, plus `generate`.

| Id | Label | Default |
|---|---|---|
| `mode` | Mode (Object, Linear, Radial, Grid, Honeycomb) | Grid (the cloner's default too; the observed label is "Grid", not "Grid Array") |
| `generate` | Generate (Matrices Only, Particles) | Matrices Only |
| `grid_count` | Count | (40, 1, 40) (cloner: (3, 1, 3)) |
| `grid_size` | Size | (10.5, 10.5, 10.5) (cloner: 200) |
| all others | as the cloner's | as the cloner's (O31 per-mode tables agree for Linear: P (0, 50, 0), step values 100 %) |

Rules:
- `matrix_data(scene, id, ctx)` (crates/generators) is `generate` of the settings read through the Matrix's
  own descriptors, then the node's effector list (E1). The result has `child = 0` for every clone.
- Evaluating the node gives no mesh (`Ok(None)`): `matrix/defaults` and `matrix/linear` export only the separate cube.
- Consumers: Object-mode cloners (brief 066) and the Tracer (M3 below) read `matrix_data`. Viewport matrix
  axes belong to brief 079.
- `matrix/defaults` (Grid, 40 x 1 x 40, size 10.5, per step): 1600 clones, first at (-204.75, 0, -204.75), x fastest, same as
  C6. `matrix/linear` (Linear, count 5, P (0, 50, 0)): clones at y = 0, 50, ... 200, as C4.

Returns `BuildError::Unsupported`: `generate` = Particles (no particle system in the clone toolset), and
Matrix Object mode. The mesh-based Cloner Object mode is evaluated separately (C2).

**Parity-pending:** the per-mode default counts of the O31 table are mislabelled in the observation (the "Object" table shows
count 5, the "Linear" one shows (40, 1, 40)); we keep the cloner's Linear default count (3) because `matrix/linear` sets
the count to 5 explicitly. Radial, Honeycomb and Matrix-with-effectors have no observed case; they share the cloner code.

## M2. Parity

Test `crates/generators/tests/matrix_parity.rs` rebuilds both cases from `case.toml` (labels mapped per mode: on the Linear
tab `p` is the step position and the Transform tab's P is `effects_p`; on the Grid tab `p` is the Transform tab's P),
compares the clone data (level B) and checks the node produces no geometry and that our 20 cm cube equals the case's
`mesh.obj` (level A on the standalone cube). Result: **2 of 2** exact. `UNSUPPORTED` / `KNOWN_MISMATCH`: none.

---

# Clone toolset: Tracer and the spline generator (brief 076)

Sources: observations O31 [clone-generators-2.md](../parity/clone-generators-2.md) (Tracer parameter table; the
`tracer/two-nulls` case was not observed) and O50 [clone-toolset-3.md](../parity/clone-toolset-3.md) (cases
`tracer/vertices`, `tracer/interval`, `clone-spline/simple`, `clone-spline/spline-mode`, `clone-spline/turtle`).
Every golden mesh is the generator's spline through an Extrude with its defaults. Both nodes are splines: they can
be the input of Extrude, Lathe, Loft and Sweep, and they draw as splines.

## T1. Tracer (`forge.tracer`)

| Id | Label | Default |
|---|---|---|
| `tracing_mode` | Tracing Mode (Trace Paths, Trace Single Object, Connect All Objects) | Trace Paths |
| `sample_step` | Sample Step | 1 |
| `trace_active`, `trace_vertices`, `use_subgroups` | Trace Active, Trace Vertices, Use Subgroups | on |
| `handle_cloners` | Nodes Only, Nodes and Edges | Nodes Only |
| `include_cloner`, `close_spline`, `reverse_sequence` | | off |
| `space` | Global, Local | Global |
| `limit` | None, From, To | None |
| `spline_type` | Linear, Cubic, Bezier, B-Spline, Akima | Linear |
| `intermediate*` | the Intermediate Points set (spec SPLINES.md) | None |
| `trace_links` | link list of the traced objects | empty |

Rules:
- **Trace Paths** (`tracer/vertices`): the Tracer samples the objects at the frames stepped through, 1..F for the document
  frame F (none at frame 0), and makes one open polyline per traced node: with Trace Vertices on every vertex of the linked
  object in its point order (8 chains for the cube, in the cube's point order), otherwise its origin; a Matrix or cloner link
  contributes one node per visible clone (its position). Chains of several links follow in link order. All chains are
  loops of one spline, in world space (the points are expressed in the Tracer's local frame so that they land at their world
  position).
- **Sample Step** `s` (`tracer/interval`, s = 5 gives frames 1, 5, 10, ... 30): the frames `f` with `f = 1` or `f mod s = 0`
  (parity-pending: one case; whether frame 1 is always sampled or the first step is relative is not separable).
- The linked object is evaluated at each sampled frame from the scene with its tracks read at exactly that frame
  (no millisecond rounding of the document time: the observed cube is at x = 10.000106, not 9.9, at frame 1).
- **Connect All Objects**: one polyline through the origins of the linked objects (the clones of a Matrix or cloner) at the
  current frame, in link order (parity-pending: not observed, `tracer/two-nulls` was empty in the reference).
- Close Spline closes every chain; Reverse Sequence reverses each chain's order.
- Extrude of several open loops: all loops wind the normal way and caps are not made (open splines have none)
  (`tracer/vertices`: faces `[a, b, b+240, a+240]` for all 232).

`BuildError::Unsupported`: Trace Single Object, spline types other than Linear, Space Local, Limit From/To, Nodes and Edges.

## S1. Clone-toolset spline generator (`forge.clone_spline`)

| Id | Label | Default |
|---|---|---|
| `mode` | Simple, Spline, Turtle | Simple |
| `count` | Count (segments of Simple) | 100 |
| `length` | Length of Simple | 200 |
| `source` | Source spline (Spline mode) | none |
| `intermediate*` | Intermediate Points | None |

- **Simple** (`clone-spline/simple`): an open straight line along +Z from the origin of `length`, in `count` equal segments
  (`count` + 1 = 101 points, 2 apart).
- **Spline** (`clone-spline/spline-mode`): an open polyline through the sampled points of the source spline (its own
  Intermediate Points setting); a closed source repeats its first point at the end, so the default circle gives 37 points
  and the Extrude 36 quads with no caps (the doubled point welds in the export).
- **Turtle** (L-system, `clone-spline/turtle-2`, `clone-spline/turtle`; sources: the full Turtle tab of `turtle-2`
  `[params.mospline]`, public Lindenmayer-system / turtle-graphics literature for the symbol meanings; the
  observations fix what is marked *derived*). Extra params: `premise` (`^(90)FFFA`), `rules` (lines `X=string`,
  default `A=!"[B]///[B]///[B]///B` and `B=&FTFTFTA`, the `"` being a plain character), `growth` (iterations, 5),
  `default_angle` (28 degrees), `default_movement` (10), `tropism` (100 %), `move_multiplier` (90 %),
  `scale_multiplier` (90 %), `randomness` (0 %), `seed`.
  1. **Grammar.** The string is rewritten `growth` times; every character with a rule is replaced by the rule's
     string at once (no rule: unchanged). Rule lines are `X=string`, one per line, one predecessor character; the first
     line for a character wins. Context rules (`X<Y>Z=`), stochastic rules and `randomness > 0` are `Unsupported`; so is
     a result longer than 2 000 000 characters. Letters other than `F` and `f` (`A`, `B`, ...) are placeholders: they
     draw nothing and only exist for the rules. Derived (`turtle-2`: 4 iterations give the drawn tree, 5 draw the same,
     so `growth` as the iteration count is parity-pending between 4 and 5).
  2. **Symbols.** `F` moves forward by the current movement and draws; `f` moves without drawing (and ends the
     chain); `+` / `-` turn left / right about the up axis, `&` / `^` pitch down / up about the left axis, `/` / `\` roll
     right / left about the heading, `|` turns 180 degrees about the up axis. An angle symbol is followed by an optional
     `(x)` with the angle in **degrees** (`^(90)`), else the default angle (28 degrees) is used. `F(x)` / `f(x)` move `x`.
     `[` pushes and `]` pops the turtle state (position, frame, movement, scale, chain). `"` multiplies the movement by
     the move multiplier (**derived**: steps 10, 9, 8.1 = 10 x 0.9^k after one and two `"`), `!` multiplies the scale
     (width, no geometry; parity-pending), `T` applies the tropism (below). Any other character is `Unsupported`
     (including `(` after a symbol that takes no value). Angle units: input degrees, internal radians.
  3. **Frame.** The turtle starts at the origin with heading +Z, up +Y (as Simple mode runs along +Z); `^(90)` makes the
     heading +Y (the trunk of `turtle-2` rises along Y). Heading `H`, up `U`, `L = U x H`. `&` by `a` is the
     right-handed rotation of the frame about `L` by `+a` (`H' = cos a H - sin a U`: the first branch of `turtle-2`
     leans 28 degrees from vertical towards +Z), `+` rotates about `U` by `+a`, `/` rotates about `H` by `+a`
     (`U' = cos a U + sin a (H x U)`: each `///` turns the next branch by 84 degrees, azimuth 90, 6, -78, -162
     degrees about Y). `\`, `-` and `^` use `-a`. The angle multiplier of the tab does not scale the angle here:
     the deeper `&` in `turtle-2` still tilt by 28 degrees (parity-pending).
  4. **Tropism `T`** (**derived**, `turtle-2`: four successive bends fit `k = 0.2000`): the heading becomes
     `normalize(H + k * (0, -1, 0))` with `k = 0.2 * tropism / 100`, and the frame is carried along by the shortest
     rotation from the old to the new heading. The 0.2 is parity-pending (only the default tropism 100 % was seen).
  5. **Chains.** The result is a list of open polylines (the generator's spline has one segment run per chain,
     Tracer-style: the first chain is the main curve and the rest `extra`). A chain starts at the turtle's position at
     its first `F` (so the tree shares branch points) and every further `F` appends the new position. `[` ends the
     current chain for the new state (the first `F` inside the bracket starts a new chain at the branch point), `]`
     restores the chain that was current at the `[`, which later `F` extend (the trunk and its last `B` are one
     chain). Chains are listed in creation order (the trunk first, each branch before its own sub-branches).
     `turtle-2`: 16 chains, 79 points, 63 segments.
  Result: `turtle-2` and `turtle` (defaults) both match the Extrude of this spline within the case tolerance
  (the two reference meshes differ from each other by float noise up to 4.5e-4). `Unsupported` otherwise: context / stochastic rules, randomness,
  unknown symbols, `grow_*` flags other than on, `generation_mode` other than Vertex.

## T2. Evaluation context

`evaluate_spline_gen` passes the evaluation context to its spline inputs (`placed_curve_at`), so a Tracer under an Extrude
follows the document frame. The viewport's spline drawing and Make Editable use frame 0 (parity-pending: the Tracer shows
nothing before frame 1).

## T3. Parity

Test `crates/generators/tests/tracer_parity.rs`: Extrude (defaults) over the rebuilt generator, compared with `mesh.obj`
(`compare_meshes`, strict). The animated cube of the Tracer cases has size 200 and a position-x track with straight-line
(chord) tangents.

| Case | Result |
|---|---|
| `clone-spline/simple` | exact |
| `clone-spline/spline-mode` | exact |
| `clone-spline/turtle` | exact (S1 Turtle, node defaults) |
| `clone-spline/turtle-2` | exact (S1 Turtle, the recorded settings set from `[params.mospline]`) |
| `tracer/vertices` | `KNOWN_MISMATCH`: counts, face lists and the 8 x 30 chains match; positions differ by up to 5.2e-4 |
| `tracer/interval` | `KNOWN_MISMATCH`: counts and faces match; positions differ by up to 9.8e-4 |

The two Tracer cases differ from our values by the reference's animation solver noise: the same frame-1 position reads
10.001106 in `interval` and 10.000106 in `vertices`, so no rule is within 1e-4 of both. The test also asserts
`topology_match` and a position error below 2e-3 for them, so a regression in the structure still fails.

---

# Clone toolset: Fracture and Voronoi Fracture (brief 074)

Sources: O31 [clone-generators-2.md](../parity/clone-generators-2.md) (parameter tables, cases `fracture/explode-segments`,
`fracture/plain`; the three `voronoi-fracture/*` cases were not observed). Golden data: `tests/golden/fracture/`.

## F1. Fracture (`forge.fracture`)

Parameters: `mode` (Straight, Explode Segments, Explode Segments & Connect; default Straight, the observed option list and
default 0), the cloner's Transform tab (`display`, `clone_position`, `clone_rotation`, `clone_scale`, `clone_color`,
`clone_weight`) and the `effectors` list. The children are the inputs and are consumed.

Rules:
- Explode Segments (`fracture/explode-segments`): every input mesh is split into its polygon islands (polygons that share a
  point are one island; islands are ordered by their first polygon). A cube whose 6 quads share no point gives 6 fragments, and
  the output is the input: the fragments keep the input's points (24) and quads (6), no point is added.
- A fragment keeps its points in ascending input index and its polygons' corner order, so the output order is fragment by
  fragment, equal to the input when the islands are in polygon order.
- Each fragment is a clone whose pivot is its bounding box centre (**parity-pending**: the observation only has identity
  clones). Its matrix is `T(P) · T(pivot) · Rot(R) · Scale(S)` (the Transform tab as on a cloner, C3.1); effectors in the
  `effectors` list apply to the fragments in list order (E1), so a Plain effector with P (0, 100, 0) raises every fragment by
  100.
- A Plain effector as a child of the Fracture does nothing (`fracture/plain`: identical to `fracture/explode-segments`): only the
  `effectors` list feeds it.
- Straight: one fragment per input object, so a single input is unchanged (parity-pending, not observed).
- The input's own transform is baked into its points.

`BuildError::Unsupported`: Explode Segments & Connect (the connector geometry was not observed).

## F2. Voronoi Fracture

**Blocked, not implemented.** The observer could not bake it (the cache stayed empty after three attempts), so the
observation has the parameter table and no meshes. The node is not registered. The design stays as in the area plan: cells from
clipping the input by the bisector planes of seed points (point count, seed), inner faces capped, parity by cell count and
total volume. Data that would settle it: any baked result (Current State to Object of a Voronoi Fracture with a point count
of 10 and a seed of 1 on a cube).

## F3. Parity

Test `crates/generators/tests/fracture.rs`: both cases rebuild the 24-point, 6-quad cube as a polygon object (the reference
output equals it, so the input is the golden mesh itself: this checks the pipeline, not an independent derivation) and compare
with `mesh.obj` strictly. Result **2 of 2** exact (`explode-segments`, `plain`). Unit test: two islands, effector list moves
them, Connect is `Unsupported`.

---

# Clone toolset: clone order, several children, nesting (brief 067)

Sources: O21 [cloner-3.md](../parity/cloner-3.md) (`order-iterate`, `order-random`, `order-sort`, `nested`) and the O8 case
`blend-order` ([cloner.md](../parity/cloner.md)); cases in `tests/golden/cloner/`.

## O1. Children order

The cases `order-*` have three children listed in the order **cone, sphere, cube** (the first clone of `order-iterate` is the cone, not the
cube the description names first): the observer's insertion put each new child first. The cone and sphere have radius 10
and default segments, the cube is 20 (these child values are only in the descriptions). `blend-order` lists cube, sphere (two
children, 5 clones). The cloner's `clones_seed` (id, label Seed, default 123456, the value the observation read back) belongs
to the Random order; the Matrix has none.

## O2. The four orders (`CloneData::assign_children(n, mode, seed)`)

- **Iterate** (`order-iterate`: clones are cone, sphere, cube, cone, sphere, cube): clone `i` shows child `i mod n`.
- **Random** (`order-random`, seed 123456: cube, sphere, cube, sphere, sphere, sphere, i.e. child indices 2 1 2 1 1 1): a child per
  clone from SplitMix64 started at the seed, `floor(u n)`. The reference's sequence is **not reproduced**; three hypotheses
  on the six observed indices: (1) SplitMix64 and Python/MT19937 draws scaled `floor(u n)` or `mod n`, (2) the glibc `rand`
  generator, `mod n` and scaled, (3) NumPy `RandomState` scaling and `randint`, each also tried under every permutation of
  the child order. No match. `order-random` is `KNOWN_MISMATCH` (positions exact, children differ). A test checks the
  draw is deterministic per seed, in range and uses all children.
- **Blend** (`blend-order`: cube, cube, sphere, sphere, sphere for 5 clones and 2 children): `t = i / (count - 1)`, child
  `floor(t (n - 1) + 1/2)`. **Parity-pending**: one case, two children; the reference's Blend may instead blend clone
  parameters, here the observed meshes are whole children.
- **Sort** (`order-sort`: all six clones show the first child): child `floor(w (n - 1) + 1/2)` clamped from the clone's weight
  `w` (0 to 1), after the effectors. **Parity-pending**: only the all-zero-weights case was observed, so this reading (and the
  value of the axis/sort options) is a guess consistent with it.
- One child (or none) always gives child 0.

## O3. Nesting

A cloner whose child is a cloner needs no code: the inner cloner is a prototype with geometry and its mesh is expanded by every
outer clone, outer clone major (`nested`: Linear count 3 over a Radial count 4). The case file does not record the inner cloner,
so the test `a_cloner_inside_a_cloner_matches_the_nested_mesh` builds it (Radial count 4, cube 20).

## O4. Parity

`cloner_parity.rs::cloner_modes_match_the_reference` now runs the four order cases (at level B and A): **3 of 4 exact**
(`order-iterate`, `order-sort`, `blend-order`), `order-random` in `KNOWN_MISMATCH` (above). `nested` stays in the separate test.
Unit test: `crates/clone-engine/tests/modes.rs::children_are_assigned_by_the_clones_mode`.
