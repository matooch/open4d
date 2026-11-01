# Fields: framework, Linear / Spherical / Box, Remapping (brief 057)

Wording: "field", "field list", "remapping". Sources: parity observations
O7 [fields.md](../parity/fields.md), O11 §1 [followups-1.md](../parity/followups-1.md),
O13 [fields-2.md](../parity/fields-2.md), O23 [fields-3.md](../parity/fields-3.md),
O24 [deformer-falloff.md](../parity/deformer-falloff.md); design
[D-deformers-fields.md](../roadmap/areas/D-deformers-fields.md); evaluation context
[MODIFIERS.md §8](MODIFIERS.md). Golden data: `tests/golden/fields/` (`samples.json`, kind
`field`, level B, 729 points on the grid `{-300, -225, ..., 300}^3`, x fastest, tolerance 1e-5).
Out of scope here: Cylinder, Cone, Torus, Capsule, Radial, Random (058); blend modes other
than Normal, groups, Formula and Shader fields (059); modifier layers (060); wiring fields into
deformer falloff (061) and effectors (072); the colour and direction channels (not observed).

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
  ids_top_to_bottom)` builds a list with every layer at strength 100 %; `with_strength` sets one.
  `FieldError::Unsupported` names an observed option without derived rules.
- Types are registered by the `FieldToolset` plugin (installed by `forge_app::default_registry`),
  so `object.create`, `param.set`, undo, save/load and the CLI, RPC, MCP and script front ends work
  from the parameter descriptors.

## F2. Node types

Type ids `forge.field.linear`, `forge.field.spherical`, `forge.field.box`; category `Field`
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

Every `linear-*`, `spherical-*` and `box-*` case of `tests/golden/fields` (27 cases: 26 field cases and `layer-opacity-50`, a Spherical field in a list at layer strength 50 %) builds a
scene through the registry (type, `position`, `rotation_deg`, `scale`, parameters from
`case.toml`, percent divided by 100), samples the 729 points through a one-layer `FieldList`
and compares with `compare_samples` at the file's tolerance. Cases of other types (blend-*,
cylinder-*, ...) are 058 and later and are not run; the test asserts exactly which prefixes are
skipped.

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
