# Fields: framework, Linear / Spherical / Box, Remapping (brief 057)

Wording: "field", "field list", "remapping". Sources: parity observations
O7 [fields.md](../parity/fields.md), O11 §1 [followups-1.md](../parity/followups-1.md),
O13 [fields-2.md](../parity/fields-2.md), O23 [fields-3.md](../parity/fields-3.md),
O24 [deformer-falloff.md](../parity/deformer-falloff.md); design
[D-deformers-fields.md](../roadmap/areas/D-deformers-fields.md); evaluation context
[MODIFIERS.md §8](MODIFIERS.md). Golden data: `tests/golden/fields/` (`samples.json`, kind
`field`, level B, 729 points on the grid `{-300, -225, ..., 300}^3`, x fastest, tolerance 1e-5).
Cylinder, Cone, Torus, Capsule, Radial and Random are section F6 (brief 058). Out of scope here: blend modes other
than Normal, groups, Formula and Shader fields (059); modifier layers (060); wiring fields into
effectors (072); deformer falloff is wired by 061 (MODIFIERS.md §10); the colour and direction channels (not observed).

## F1. API (`forge-fields`)

```rust
pub trait Field: Send + Sync {
    fn sample(&self, p_world: DVec3, ctx: &EvalCtx) -> FieldSample;
    fn sample_many(&self, points: &[DVec3], ctx: &EvalCtx) -> Vec<FieldSample>; // default: map sample
}
pub struct FieldSample { pub value: f64, pub color: [f64; 3], pub direction: DVec3 }
```

- `value` is the remapped field value (the observed channel). `color` is the field's `Color`
  parameter (a constant colour, white by default; brief 068 observed it through the Plain effector's
  Fields Color mode, MOGRAPH.md E6) and `direction` is zero until the direction channel is observed
  (**parity-pending**). A list mixes the colour with the layer strengths like the value; its blending
  beyond one layer is parity-pending.
- Points are **world** positions. A field object converts them to its own space with the inverse
  of its world matrix (position, rotation, scale and parents), so a scaled field is an ellipsoid
  (`spherical-scaled`) and a moved one follows (`spherical-moved`).
- `FieldList { layers }` holds layers **top to bottom** (the order the observer reads them; a
  layer added later goes on top, O13). A layer is `{ field, strength, blend }`; `strength` is
  the layer strength (fraction), `blend` is `BlendMode::Normal` (the other modes are 059).
  `FieldList::sample(p, ctx)` evaluates bottom to top starting at 0:
  `acc = acc + (value - acc) * strength` (Normal). One layer at 50 %
  therefore halves its value (`layer-opacity-50`); the two-layer rule is **parity-pending** until
  059 matches `blend-normal` and `blend-normal-50`.
- **An empty list means full effect**: `FieldList::sample` returns value 1 (O24 `bend-empty-list`).
  `is_empty()` lets a deformer skip the evaluation.
- `field_from_node(scene, id) -> Result<Box<dyn Field>, FieldError>` builds the field of a
  `Category::Field` node from its parameters and world matrix. `FieldList::from_nodes(scene,
  ids_top_to_bottom)` builds a list whose layer strengths are the nodes' `layer_strength` (100 % default); `with_strength` sets one.
  `FieldError::Unsupported` names an observed option without derived rules.
- Types are registered by the `FieldToolset` plugin (installed by `forge_app::default_registry`),
  so `object.create`, `param.set`, undo, save/load and the CLI, RPC, MCP and script front ends work
  from the parameter descriptors.

## F2. Node types

Type ids `forge.field.linear`, `forge.field.spherical`, `forge.field.box` (F6 adds six more); category `Field`
(hierarchy node: position, rotation, scale come first). Percent values are stored as fractions
(case files hold percent; angles none here).

| id | label | kind | default | range / options |
|---|---|---|---|---|
| `length` (Linear) | Length | float, distance | 100 | 0 .. 1e20 |
| `direction` (Linear) | Direction | enum | `x_pos` | `x_pos` X+, `x_neg` X-, `y_pos`, `y_neg`, `z_pos`, `z_neg` |
| `radius` (Spherical) | Size | float, distance | 100 | 0 .. 1e20 |
| `size` (Box) | Size | vec3, distance | (100, 100, 100) | 0 .. 1e20 |
| `clip_to_shape` | Clip to Shape | bool | off | |
| `layer_strength` | Layer Strength | float, percent | 100 % | the node's row in a field list (F1; deformer falloff, brief 061) |
| `color` | Color | colour | white | the constant colour channel (brief 068, MOGRAPH.md E6, E8) |
| `enable_remapping` | Enable Remapping | bool | on | |
| `strength` | Strength | float, percent | 100 % | -1e20 .. 1e20 |
| `invert` | Invert | bool | off | |
| `inner_offset` | Inner Offset | float, percent | Linear 0 %; Spherical, Box 50 % | 0 .. 100 % |
| `min` / `max` | Min / Max | float, percent | 0 % / 100 % | -1e20 .. 1e20 |
| `clamp_min` / `clamp_max` | Clamp Min / Clamp Max | bool | on | |
| `multiplier` | Multiplier | float, percent | 100 % | -1e20 .. 1e20 |
| `contour_mode` | Contour Mode | enum | `none` | `none`, `quadratic`, `step`, `quantize`, `curve` |
| `curve` | Curve | float, percent | 0 % | -100 .. 100 % |
| `steps` | Steps | int | 1 | 1 .. 2147483647 |
| `mirror_graph` | Mirror Graph | bool | Linear off; Spherical, Box on | |
| `spline_offset`, `spline_range` | Spline Offset / Range | float, percent | 0 % / 100 % | stored only |

The colour and direction channel remapping parameters of O7 (Color Mode, Length, Invert
Direction, Strength, Invert Rotations) are not parameters here: no case observes their effect. Only
the constant `color` is (above).

## F3. Shape value

Let `p` be the sample in field space and `t` the **ramp position**, 1 in the core and 0 outside
(clamped to 0..1):

- **Linear:** with `a` the signed direction axis, `t = clamp(0.5 + p_a / (2 * length))`. The ramp is
  `2 * length` wide, centred on the field (`linear-defaults`: `0.5 + x / 200`; `linear-length`:
  `0.5 + x / 800`; `linear-direction-x`, mirrored; `linear-rotated`, rotation 45 degrees gives
  `0.5 + 75 cos 45 / 200 = 0.765`). `length = 0`: `t = 1` for `p_a > 0`, 0 for `p_a < 0`, 0.5 on the plane.
- **Spherical:** `t = clamp(1 - |p| / radius)`. `radius` is the field's Size read as the radius
  (`spherical-radius`: 250 gives 0.8 at 150). `radius = 0`: `t = 1` at the centre, 0 elsewhere.
- **Box:** `t = clamp(1 - max(|p_x| / size_x, |p_y| / size_y, |p_z| / size_z))` (a per-axis
  normalised Chebyshev distance: the value reaches 0 at `|p_i| = size_i`, the plateau ends at
  `i * size_i`; so the reach per axis is the whole Size, as for the sphere radius; `box-size`, `box-inner-0`). A zero size component gives
  `|p_i| / 0 = inf` unless `p_i = 0`.

## F4. Remapping

Applied to `t` when `enable_remapping` is on; otherwise the value is `t` with inner offset 0.
With `i = inner_offset`:

1. **Inner offset:** `v = 1` if `t >= 1 - i`, else `t / (1 - i)` (`i = 1` makes every `t >= 0` a 1).
   Proof: `spherical-inner-0` (0.25 at distance 75), the default 50 % (0.5), `spherical-inner-80`
   (all values 0 or 1), `linear-inner-50` (0.25 at x -75, 1 from 0), `box-inner-0`.
2. **Contour:** `none` identity. `quadratic` is a symmetric ease: `q(v) = 2v^2` for `v < 0.5`,
   `1 - 2(1-v)^2` otherwise, blended with the identity by `curve` (`v + c (q(v) - v)` for
   `c >= 0`; for `c < 0` the inverse ease `qi(v)` with weight `-c`); at the default `curve = 0`
   it is the identity, which is what `spherical-remap-contour` shows (0.5 stays 0.5).
   `step`: `ceil(v * steps) / steps` (`spherical-interp-step`: steps 1 turns 0.5 into 1 and keeps
   0 and 1; `spherical-steps-4`: 0.5 stays 0.5). `quantize`: `floor(v * steps + 0.5) / steps`
   (`spherical-quantize-4`, byte-identical to the step case on this data). `curve` returns
   `FieldError::Unsupported("remapping curve")`, as the Remapping spline is not stored.
3. **Invert:** `v = 1 - v` (`spherical-remap-invert`: 0 at the centre, 1 outside).
4. **Min / Max:** `v = min + v * (max - min)` (`spherical-remap-minmax`: 0.2 outside, 0.8 at the centre).
5. **Strength and multiplier:** `v = v * strength * multiplier` (`spherical-strength-50`: largest 0.5).
6. **Clamps:** with `lo = min(min, max)`, `hi = max(min, max)`, `clamp_min` applies `v = max(v, lo)`
   and `clamp_max` applies `v = min(v, hi)`.
7. **Clip to Shape:** if on, a sample outside the shape (Spherical `|p| > radius`, Box some
   `|p_i| > size_i`) gets 0. Linear has no outside, so the flag changes nothing
   (`linear-clip`, `spherical-clip` equal their defaults cases).

`mirror_graph` and `spline_*` are stored and have no effect yet.

## F5. Parity (`crates/fields/tests/fields_parity.rs`)

Every `linear-*`, `spherical-*` and `box-*` case of `tests/golden/fields` (27 cases: 26 field cases and `layer-opacity-50`, a Spherical field in a list at layer strength 50 %) builds a
scene through the registry (type, `position`, `rotation_deg`, `scale`, parameters from
`case.toml`, percent divided by 100), samples the 729 points through a one-layer `FieldList`
and compares with `compare_samples` at the file's tolerance. Cases of other types (blend-*, ...) are 059 and later and are not run; the test asserts exactly how many are
skipped. Brief 058 extends the same test (36 exact cases) and adds the statistical Random test (F6).

### Parity-pending rules (no observation tells them apart) and what would settle them

| rule | alternatives | data that settles it |
|---|---|---|
| order of invert, min/max, strength | any order | Spherical with Invert on and Strength 50 %; with Min 20 % and Strength 50 % |
| Min/Max remap or clamp-only | `min + v (max - min)` vs `clamp(v, min, max)` | Spherical, Inner Offset 0 %, Min 20 %, Max 80 % (0.25 maps to 0.35 or 0.25) |
| where clamps act (before or after strength) | | Strength 200 % with clamps on and off |
| Step formula | `ceil` vs round vs floor+1 | Spherical Inner Offset 0 % with Step 4: value 0.25 and the in-between 0.3 |
| Quantize differs from Step? | | the same file with Quantize 4 and Steps 3 |
| Quadratic shape and `curve` | | Spherical Inner Offset 0 % with Quadratic and Curve 0 %, 50 %, -50 % |
| Clip to Shape | before or after remapping, edge | Spherical with Invert on and Clip on |
| Remapping off | inner offset and other rules | Spherical with Enable Remapping off |
| `mirror_graph` | effect on contour | Spherical, Quadratic, Curve 50 %, Mirror Graph on and off |
| Inner Offset 100 % and size 0 | edge cases | each at 100 % / 0 |
| Box with a non-axis point | Chebyshev vs other norms (the grid has off-axis points; matched) | covered by `box-defaults` |
| field with a parent or non-uniform scale | | covered by `spherical-scaled` for object scale only |
| colour, direction channels | | sampled colour and direction of the three fields |

## F6. Cylinder, Cone, Torus, Capsule, Radial, Random (brief 058)

Sources: O13 [fields-2.md](../parity/fields-2.md) (parameters, defaults, `cylinder-*`, `cone-defaults`,
`torus-defaults`, `capsule-defaults`, `radial-defaults`, `random-defaults`, `random-seed`,
`random-random-mode`), O23 [fields-3.md](../parity/fields-3.md) (`radial-half`, `radial-transition`,
`radial-iterations`, `random-global`). Type ids `forge.field.cylinder`, `.cone`, `.torus`, `.capsule`,
`.radial`, `.random`; category `Field`. All carry the Remapping parameters of F2, `color` and
`layer_strength`; `clip_to_shape` exists on every type except Random. Inner Offset / Mirror Graph
defaults: Cylinder, Cone, Torus, Capsule 50 % / on; Radial 0 % / on; Random 0 % / **off** (O13). Each
shape is one source file in `crates/fields/src/` and plugs into `Shape` (the ramp `t` of F3) or, for
Random, is its own `Field`.

| id | label | kind | default | range / options |
|---|---|---|---|---|
| `height` (Cylinder, Cone, Capsule) | Height | float, distance | 100, 100, 150 | 0 .. 1e20 |
| `radius` (Cylinder, Cone, Torus) | Radius | float, distance | 75, 100, 100 | 0 .. 1e20 |
| `thickness` (Torus) | Thickness | float, distance | 35 | 0 .. 1e20 |
| `top_radius`, `bottom_radius` (Capsule) | Top / Bottom Radius | float, distance | 75 | 0 .. 1e20 |
| `direction` (Cylinder, Cone, Capsule) | Direction | enum | `y_pos` | six signed axes as Linear |
| `direction` (Torus) | Direction | enum | `y_pos` | `x_pos`, `y_pos`, `z_pos` |
| `start_angle`, `end_angle`, `offset` (Radial) | Start Angle, End Angle, Offset | float, angle (radians) | 0, 360 degrees, 0 | -1e20 .. 1e20 |
| `start_transition`, `end_transition` (Radial) | Start / End Transition | float, angle | 0 | 0 .. 360 degrees |
| `iterations` (Radial) | Iterations | int | 1 | 1 .. i64 max |
| `axis` (Radial) | Axis | enum | `y` | `x`, `y`, `z` |
| `random_mode` (Random) | Random Mode | enum | `noise` | `random`, `sorted`, `noise` |
| `seed` (Random) | Seed | int | 123456 | 0 .. i64 max |
| `noise_type` (Random) | Noise Type | enum | `perlin` | the 31 names of O13 |
| `space` (Random) | Space | enum | `field` | `global`, `field`, `none` |
| `scale` (Random) | Scale | float, percent | 100 % | 0.01 % .. 1e20 % |
| `relative_scale` (Random) | Relative Scale | vec3, percent | 100 % each | 0.01 % .. 1e20 % |
| `animation_speed` (Random) | Animation Speed | float, percent | 0 % | -1e20 .. 1e20 % |
| `randomized_sample_positions`, `sample_position_seed`, `color_shift`, `color_alpha` (Random) | as O13 | bool, int 123456, percent 10 %, enum `none`/`strength` | | stored only |

The viewplane preview and display box parameters of O13 are editor preview settings and are not
parameters here.

### Ramp `t` of the shapes
Let `a` be the signed coordinate along the direction axis (negated for the `neg` options), `r` the
distance from that axis, and `ratio(x, y) = |x| / y` (0 for 0/0, inf for x/0). Each shape gives a
normalised distance `d`; `t = clamp(1 - d)` then the Remapping of F4 (Clip to Shape: 0 where `d > 1`).

- **Cylinder:** `d = max(r / radius, |a| / height)` (`cylinder-defaults`, `cylinder-size`: the
  same Chebyshev-style combination as the Box; height is the half extent, so the field reaches
  `height` along the axis, as the sphere radius does).
- **Cone:** the base is at `a = -height / 2` and the apex at `a = +height / 2`. The cone radius at `a` is
  `R(a) = radius * clamp((height / 2 - a) / height, 0, 1)` (full radius at and below the base,
  0 at and above the apex) and `d = max(r / R(a), |a| / height)` (`cone-defaults`: value 0.5 at
  `(75, -75, 0)`, 0 at `(75, 0, 0)` and at `(75, -75, 75)`, 0.5 at `(0, 75, 0)`). **Parity-pending:**
  one case; the axial extent and the slope between base and apex are fitted to its zero/non-zero
  pattern, not derived (see the table below).
- **Torus:** the ring of radius `radius` lies in the plane perpendicular to the axis;
  `d = sqrt((r - radius)^2 + a^2) / thickness`, `thickness` being the tube radius
  (`torus-defaults`: 4/7 at `(75, 0, 0)` is `(1 - 25/35) / 0.5`; 1 at `(75, 0, 75)`).
- **Capsule:** `height` is the length of the straight segment from `a = -height / 2` to `+height / 2`
  (the end caps lie beyond it). With `dy = max(|a| - height / 2, 0)`: `d = sqrt(r^2 + dy^2) / R`
  with `R` the radius interpolated along the segment from `bottom_radius` (at `-height / 2`) to
  `top_radius` (at `+height / 2`), clamped at the ends (`capsule-defaults`: 1 at `(0, +-75, 0)`,
  0 at `(0, +-150, 0)` and at `(75, 0, 0)`). **Parity-pending:** unequal radii and the Y- direction (only
  the equal-radius default is observed).
- **Radial** (angular, around the axis): the angle `theta` is the right-handed rotation about the axis,
  `atan2(-z, x)` for Y, `atan2(z, y)` for X, `atan2(y, x)` for Z. With `span = end - start`:
  `span >= 360` degrees gives `t = 1` everywhere (`radial-defaults`); `span <= 0` gives 0. Otherwise
  `phi = (theta * iterations - offset - start) mod 360` and the range is the half-open `[0, span)`:
  `t = 0` for `phi >= span`, else `t = 1` limited by the transitions inside the range:
  `phi / start_transition` for `phi < start_transition` and `(span - phi) / end_transition` for
  `phi > span - end_transition`, the smaller one where both apply. `radial-half`: 369 ones, and the
  ray at `theta = 180` (x < 0, z = 0) is 0 while `theta = 0` (x > 0, z = 0) and the centre are 1;
  `radial-transition` (end 180, transition 45): `(180 - theta) / 45` inside the last 45 degrees
  (0.312 at `(-300, y, -75)`); `radial-iterations` and `radial-defaults`: all 1. **Parity-pending:** start
  transition, offset sign, iterations on a partial range (the data has only full-circle iterations),
  X and Z axes, overlapping transitions, Clip to Shape (outside the range).

### Random
`t` is a pseudo-random value in `[0, 1]`, then F4. The sample position `q` is the field-space position
(Space Field, the default), the world position (Global) or 0 (None, constant: **parity-pending**).
`random-global` equals `random-defaults` because the field is at the origin, unrotated.

- **Noise:** `t = unit(perlin-like noise)` through `forge-noise` (NOISE.md): `Noise { kind, seed, scale =
  100 * scale, time }`, lattice coordinate `q / (100 scale * relative_scale) + offset(seed)`, where the offset in
  `[0, 1)^3` is derived from the seed (the origin is not a lattice point: the reference value at the origin is not
  0.5). `animation_speed` moves along the fourth axis: `time = ctx.time * animation_speed`.
  The Noise Type maps to a `NoiseKind` where NOISE.md defines one: Box, Blistered Turbulence, Cell,
  Displaced Turbulence, Displaced Voronoi, Electric, FBM, Gaseous, Perlin, Ridged Multifractal,
  Turbulence, Voronoi 1/2/3, Wavy Turbulence; the 16 others (Buya, Cell Voronoi, Cranal, Dents, Hama, Luka,
  Mod. Noise, Naki, Nutous, Ober, Pezo, Poxo, Sema, Stupl, VL Noise, Zada) return
  `FieldError::Unsupported("noise type")` in Noise mode (Random and Sorted modes ignore the type).
- **Random:** `t` is a hash of the seed and the sample position in `[0, 1)`: uniform, uncorrelated.
- **Sorted:** the same values, but `sample_many` sorts them ascending in sample order (a single `sample`
  gives the unsorted value). **Parity-pending:** no case; the reference sorts by clone index.
- Randomized Sample Positions, Color Shift and Color Alpha are stored only.

### Parity (statistical, N7)
The exact noise values of the reference are not derivable (NOISE.md N7), so the four Random cases
are compared statistically in `crates/fields/tests/random_statistics.rs`: range inside `[0, 1]`,
mean within 0.05 of the reference mean, standard deviation within 25 % of the reference, lag-1
correlation (neighbours 75 apart) within 0.15 of 0 (for the reference as well), Random mode uniform
(mean 0.5 +- 0.05, std 0.289 +- 0.04). `random-global` must equal `random-defaults` exactly in both
the reference and ours. Reason: feature size, octave count and seed hashing of the reference are
unknown, and a 9x9x9 grid at spacing 75 cannot pin them.

### Parity-pending rules of F6 and the data that settles them
| rule | alternatives | data that settles it |
|---|---|---|
| Cone extent and slope | base at `-height/2` vs `-height`; linear taper vs other | Cone with Height 300 and Radius 150, a finer grid of samples (spacing 25), and one with Direction Y- |
| Cylinder / Capsule / Torus Clip to Shape | `d > 1` vs other | each with Clip to Shape on and Invert on |
| Capsule with unequal radii, Y- | radius interpolation vs round-cone distance | Top Radius 100 and Bottom Radius 50 |
| Torus metric | Euclidean tube distance vs other | Torus with Thickness 100, samples spaced 25 near the ring |
| Radial start transition, offset sign, partial range with iterations 3, X and Z axes | | Radial with Start Angle 45, Start Transition 45, End Angle 270, Offset 30, Iterations 2, axes X and Z |
| Random Noise: feature size, octaves, seed hashing | | a Random field sampled on a 1-unit line (`x` from 0 to 400, step 1) for Perlin and seeds 0, 1 |
| Random Space None, Sorted mode, Animation Speed | | Space None; Sorted on a cloner; Animation Speed 100 % at times 0 and 1 s |
| Noise types without a literature definition (16) | | one line sample for each type |

## F-UI. Disabled field rows (brief 079)

A field node whose object state `object.enabled` is false (TRANSFORM.md §2, UI.md §16.1) is
left out by `FieldList::from_nodes`: the layer does not exist, so the remaining layers (or the
empty list, value 1) decide. The enable box in a field list writes `object.enabled` of the
field node. Own design, parity-pending G4.
