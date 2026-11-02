# MoGraph spec checklist (verify each against manual + observation; add anything missing in the installed version's menus)
## Generators
- [ ] Cloner (brief 065: clone data, Linear, Radial, Grid Array, Honeycomb and instance modes done, see the clone-toolset sections below; Object, order modes, effectors pending): modes Linear, Radial, Grid Array, Honeycomb, Object (vertex/edge/polygon/surface/volume/axis), Blend; Iterate/Random/Blend/Sort clone order; fix clone, fix texture, instance modes (Instance/Render Instance/Multi-Instance), effectors list, per-clone transforms, scale/rotation/color/weight, MoData output
- [ ] Matrix, Fracture (modes + connectors), Voronoi Fracture (sources, distribution, shape, details), MoSpline (Spline/Turtle/Simple modes), MoText, MoInstance, Tracer (trace links/vertices, interval, spline types)
## Effectors
- [ ] Plain (brief 068: framework + Plain done, see section E below), Random (brief 069, sections E11-E17), Delay, Formula, Inheritance, Python, Shader, Sound, Spline, Step, Target, Time, Volume, Push Apart, Reeffector, Group
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

Object mode and its parameters (Object, Distribution, Selection, Seed, Count) are brief 066:
`mode = object` is selectable but evaluating it is `BuildError::Unsupported("cloner object mode")`.

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
   matrix (**parity-pending**: only the default was observed).
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

`t_i = i + linear_offset` (i = 0 .. count - 1; **parity-pending** for a non-zero offset: all samples
have offset 0). `u_i = t_i / (count - 1)` (0 when count is 1).

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
starts at 30°: (25, 0, 43.301). Offset Variation (seeded, **parity-pending**): each angle gets
`+ variation · step · (2 r - 1) / 2` with `r` the next number of the seeded generator (C9).

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
  data point: **parity-pending** (sphere only; cylinder fill and non-uniform sizes unobserved,
  normalisation per axis by the half extent is our reading).
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

Offset Variation, Honeycomb variations use `forge_clone_engine::SplitMix64` seeded with the
parameter seed: deterministic and platform independent. The reference's sequence is not
reproducible from the observations (no sample with a non-zero variation exists), so any such case
is compared statistically (range and moments), never exactly (area file, "Randomness").

## C10. Parity

Test: `crates/generators/tests/cloner_parity.rs` rebuilds every cloner case without effectors from
its `case.toml` (the app's labels, degrees, percent as typed; `count` etc. are mapped to the
mode-prefixed ids) under a `forge.cube` of size 20 and compares

- the clone data of the evaluated node with `samples.json` (`compare_samples`, expected tolerance 1e-4,
  visibility and count exact), and
- the expanded mesh with `mesh.obj` (`compare_meshes`, strict; after `export_weld` on both sides).

Matched: **19 of 19** cases run, each at both levels (samples and expanded mesh): linear 5
(`linear-defaults`, `-count-step`, `-cumulative`, `-mode-endpoint`, `-scale-step`), radial 5
(`radial-defaults`, `-plane-start-end`, `-align-off`, `-offset`, `-xy`), grid 6 (`grid-defaults`,
`-3x2x4`, `-endpoint`, `-sphere-form`, `-cylinder-form`, `-fill-50`), honeycomb 3
(`honeycomb-defaults`, `-small`, `-offset`). `KNOWN_MISMATCH` and `UNSUPPORTED` are empty.

Not run here (pinned by count in the test, 47 of the 66 cloner cases): 35 cases with
an effector (`[params.effector]` or `effector_count > 0`: briefs 068+), 8 Object-mode cases
(`object-*`: brief 066), `order-iterate`, `order-random`, `order-sort` (several children and the
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
- **Falloff.** An effector has the link list `fields` (category `Field`, **top to bottom**, the order
  of FIELDS.md F1). `FieldList::from_nodes` builds the list and the effector samples it at the **world**
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
| `scale_value` | Scale | float | 0 | the value under Uniform Scale (`scale_1018` in the case files) |
| `color_mode` | Color Mode | enum | `fields_color` | `off`, `effector_color`, `custom_color`, `fields_color` |
| `color` | Color | colour | white | the Custom Color swatch |
| `use_alpha_strength` | Use Alpha/Strength | bool | off | colour block (the app switches it on with Custom Color) |
| `blending_mode` | Blending Mode | enum | `default` | `default`, `add`, `subtract`, `multiply`, `divide`; all but `default` are `Unsupported` |
| `weight_transform`, `u_transform`, `v_transform` | Weight, U, V Transform | float, percent | 0 | |
| `modify_clone` | Modify Clone | float, percent | 0 | stored only (nothing observed) |
| `visibility` | Visibility | bool | off | |

`use_alpha_strength_1071` (the Weight tab's own Use Alpha/Strength) is not stored: no case sets it.

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

`use_alpha_strength_1071` is not stored (as E2).

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
