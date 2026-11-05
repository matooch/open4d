# Fields: framework, Linear / Spherical / Box, Remapping (brief 057), field lists (059)

Wording: "field", "field list", "remapping". Sources: parity observations
O7 [fields.md](../parity/fields.md), O11 §1 [followups-1.md](../parity/followups-1.md),
O13 [fields-2.md](../parity/fields-2.md), O23 [fields-3.md](../parity/fields-3.md),
O24 [deformer-falloff.md](../parity/deformer-falloff.md); design
[D-deformers-fields.md](../roadmap/areas/D-deformers-fields.md); evaluation context
[MODIFIERS.md §8](MODIFIERS.md). Golden data: `tests/golden/fields/` (`samples.json`, kind
`field`, level B, 729 points on the grid `{-300, -225, ..., 300}^3`, x fastest, tolerance 1e-5).
Cylinder, Cone, Torus, Capsule, Radial and Random are section F6 (brief 058); blend modes, rows, Group,
Formula and Shader fields are section F7 (brief 059). Out of scope here: modifier layers (060); wiring fields into
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
  the layer strength (fraction), `blend` one of the nine `BlendMode`s (F7).
  `FieldList::sample(p, ctx)` evaluates bottom to top starting at 0:
  `acc = acc + (blend(acc, value) - acc) * strength`, which for Normal is
  `acc + (value - acc) * strength`. One layer at 50 % therefore halves its value
  (`layer-opacity-50`), and a top layer at 50 % over a full one shows the one below through
  (`blend-normal-50`). The blend functions and the rows' storage are F7.
- **An empty list means full effect**: `FieldList::sample` returns value 1 (O24 `bend-empty-list`).
  `is_empty()` lets a deformer skip the evaluation.
- `field_from_node(scene, id) -> Result<Box<dyn Field>, FieldError>` builds the field of a
  `Category::Field` node from its parameters and world matrix. `FieldList::from_param(scene, owner,
  param)` builds the list of a `LinkRows` parameter (a deformer's or effector's `fields`),
  `from_rows` that of a row slice, `from_nodes(scene, ids_top_to_bottom)` one of Normal rows at
  100 %; `with_strength` sets a strength after the fact (F7).
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

Every `linear-*`, `spherical-*` and `box-*` case of `tests/golden/fields` (27 cases: 26 field cases and `layer-opacity-50`, a Spherical field in a list row at strength 50 %) builds a
scene through the registry (type, `position`, `rotation_deg`, `scale`, parameters from
`case.toml`, percent divided by 100), samples the 729 points through a one-layer `FieldList`
and compares with `compare_samples` at the file's tolerance. Brief 058 extends the same test (36 exact cases) and adds the statistical Random test (F6); brief 059
adds the list cases (52 exact cases, F7, with `shader-noise` pinned as Unsupported).

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
`clip_to_shape` exists on every type except Random. Inner Offset / Mirror Graph
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

## F-UI. Disabled field rows (brief 079, changed by 059)

A field node whose object state `object.enabled` is false (TRANSFORM.md §2, UI.md §16.1) is
left out by `FieldList::from_rows`: the layer does not exist, so the remaining layers (or the
empty list, value 1) decide. Since brief 059 the enable box in a field list is the row's own
`enabled` (F7); a row that is off is left out the same way. Own design, parity-pending G4.

## F7. Field lists: rows, blend modes, Group, Formula, Shader (brief 059)

Sources: O13 [fields-2.md](../parity/fields-2.md) (`two-layers-add`, `two-layers-max`, `layer-opacity-50`;
a layer added later goes on top), O23 [fields-3.md](../parity/fields-3.md) (`blend-*`, `three-layers`,
`group-field`, `formula-defaults`, `formula-custom`, `shader-noise`), O24 [deformer-falloff.md](../parity/deformer-falloff.md)
(`bend-field-strength-50`). Golden data: `tests/golden/fields/`. Parity test:
`crates/fields/tests/fields_parity.rs` (`exact_field_types_match_the_reference`, 52 exact cases),
plus `crates/fields/tests/lists.rs`.

### Rows (storage)
A field list is a parameter of kind `LinkRows` (`Value::LinkRows(Vec<LinkRow>)`, CORE.md section 6.10) on
the node that owns it: the `fields` parameter of every deformer and effector, and the `fields` parameter
of a Group field. A row is `{ target, blend, strength, enabled }`: the field node, a blend option id,
the layer strength as a fraction (100 % = 1, any finite number) and the enable box. Rows are ordered **top
to bottom**, a node appears at most once per list, the targets must be `Category::Field` nodes, the
blend id one of the nine below (`ParamDesc::link_rows(id, label, accepts, blends)`; the default of a new
row is the first blend, Normal, at 100 %, on). Because the row belongs to the list, a field used by two
lists has two strengths and two blend modes. All edits go through `param.set` / `param.edit` of the owner's
`fields` parameter (one undo step per edit; JSON rows are `{target, blend, strength (percent), enabled}`
and a bare node id is a default row); the Attributes manager draws each row as
`[enable] name [blend] [strength %] ^ v x` (UI.md §16.1). Setting a plain `LinkList` on such a
parameter turns it into default rows.

`FieldList::from_rows` leaves out a row that is switched off, a node that no longer exists or is not a
field, and a field node whose own object state `object.enabled` is off (F-UI). The remaining rows keep
their order, blend mode and strength. The per-row enable box replaces the use of `object.enabled` as
the list's enable box (brief 061/079 used the node's state); both still remove the layer.

### Blend modes
Evaluated bottom to top from `acc = 0`. With `base` the accumulated value and `top` the layer's own
value, `acc' = base + (blend(base, top) - base) * strength`. The blend functions, all matching on the
729 samples of O23 to 1e-15 (two overlapping Spherical fields A at -75 and B at +75, Size 200, B on top):

| mode | `blend(base, top)` | proving case |
|---|---|---|
| Normal | `top` | `blend-normal`, `three-layers` (C replaces everything below), `blend-normal-50` (strength 50 %: `base + (top - base) / 2`) |
| Max | `max(base, top)` | `blend-max`, `two-layers-max` |
| Min | `min(base, top)` | `blend-min` |
| Add | `clamp(base + top, 0, 1)` | `blend-add` (29 ones), `two-layers-add` |
| Subtract | `clamp(base - top, 0, 1)` | `blend-subtract` (base is the lower layer; `top - base` fails on 102 samples) |
| Multiply | `base * top` | `blend-multiply` |
| Overlay | `2 base top` if `base < 0.5`, else `1 - 2 (1 - base)(1 - top)` | `blend-overlay` (the swapped test on `top` fails on 34 samples) |
| Screen | `base + top - base top` | `blend-screen` |
| Clip | `min(base, top)` | `blend-clip` (equal to Min on this data; `base * top` fails on 28 samples) |

The colour channel is blended with the same function per component and the direction channel always
with Normal (**parity-pending**: no case samples them). Strength with a mode other than Normal
(`base + (blend - base) * strength`), Add and Subtract on values outside 0..1, a list whose bottom layer is
not Normal (the start value is 0) and **Clip as a distinct mode** are parity-pending (see the table at
the end).

### Group field (`forge.field.group`)
A field whose value is a field list: the node's own `fields` rows are built with `FieldList::from_rows`
(nested groups are allowed; a depth over 16 or a cycle gives `FieldError::Unsupported`), sampled at the
world position and then passed through the group's own Remapping tab as the ramp position `t`
(F4). The Remapping defaults are those of `group-field`: Enable Remapping **off**, Mirror Graph off,
Inner Offset 0 %, Strength 100 %, Min 0 %, Max 100 %, both clamps on, Multiplier 100 %, Contour None. With
remapping off the group's value is the list's value: `group-field` (B over A, both Normal) equals B alone
and matches to 1e-5. The node's position, rotation and scale do not move its layers (they have their own).
A group without layers has value 0 (**parity-pending**; an empty list on a deformer is full effect).
The group's `color` is stored only (the list's colour channel is used). A group reads time if one of
its enabled layers does (`reads_time`).

### Formula field (`forge.field.formula`)
Parameters: `formula` (string, default the text of `formula-defaults`), `time_delta` (percent, 100 %; the
O23 key `t_d_time_delta_time_percent`), `frequency` (float, 1; `f_frequency`), `color` and the
Remapping tab with **both clamps off**, Inner Offset 0 %, Mirror Graph off (`formula-defaults`).
The text is compiled with `forge-expr` in **degrees mode** (EXPR.md E5) with the variables
`px py pz id count subfields d t f`:

- `px py pz`: the sample position in the field's own space (`formula-custom`: `px/100`; `x` alone is not a
  variable, EXPR.md rejects it and the observation gave zeros);
- `id`, `count`: the index of the sample in the batch and the batch size (`formula-defaults`: sample
  number `i` of 729 gives `0.5 + sin(i / 729 * 360 deg) * 0.5` at all 729 points to 1e-5, so `subfields`,
  `d` are 0 and `f` is 1). A single `sample` is `id = 0`, `count = 1`;
- `t`: `ctx.time * time_delta`; `f`: `frequency`; `subfields`, `d`: 0 (**parity-pending**, no case).

The result is limited to `0..1` as the ramp position (`formula-custom`: `px/100` gives 0 for `px <= 0`
and 1 for `px >= 100` although both clamps are off, so the limit is not the Remapping clamp), a non-finite
result is 0, then the Remapping tab of F4 is applied. A text that does not parse gives
`FieldError::Formula(message)`. A formula reads time when it names `t` and `time_delta` is not 0.

### Shader field (`forge.field.shader`)
The node type exists (parameter `shader`, a text, plus the Remapping tab) so lists, files and undo handle it,
but building it returns `FieldError::Unsupported("shader field (needs brief 125)")` until the shader
library exists. `shader-noise` is pinned as Unsupported in the parity test (the Noise shader of O23 is a
brief 125 case). Its shader parameters (`noise_*`, channel, material tag, UV offsets and lengths) are not
parameters yet.

### Parity-pending rules of F7 and the data that settles them
| rule | alternatives | data that settles it |
|---|---|---|
| Clip vs Min | `min` vs a clip against a threshold or the lower layer's support | Clip with the top layer at 50 % and with values the lower layer does not reach; a lower layer with a negative strength |
| Blend with strength other than 100 % for modes other than Normal | `base + (blend - base) s` vs `blend(base, top s)` | `blend-add` and `blend-multiply` with the top strength 50 % |
| Add / Subtract limits | clamp to 0..1 vs unclamped until the end | Add of three layers (0.7 each), top Subtract of 0.2 |
| Bottom layer blend | start value 0 vs transparent | the bottom layer set to Add or Multiply |
| Colour and direction blending | per-component blend vs Normal only | two Spherical layers with different colours, Plain effector Fields Color mode, blend Add |
| Group remapping on | list value as `t` vs remapping only the shape | Group with Enable Remapping on, Invert on, Inner Offset 50 % over B over A |
| Group transform and own Strength | whether group position moves layers | Group at (100, 0, 0) holding A and B |
| Empty group | 0 vs 1 | a Group field without layers in a deformer's list |
| Formula `d`, `subfields`, space of `px` | field space vs world; `d` distance | a Formula `px/100` on a field moved to (100, 0, 0) and rotated; `d` and `subfields` on their own as formulas |
| Formula result limit | clamp 0..1 before vs after Remapping | formula `2*px/100` with Invert on |
| Formula `t` and `f` | seconds scale, units of `t` | formula `t` sampled at 0 s and 1 s with the time delta 100 % and 50 % |
| Shader field | everything | brief 125 and a shader-noise case on a finer grid |
