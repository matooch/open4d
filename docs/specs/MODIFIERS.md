# Deformers / modifiers
- [ ] Bend, Bulge, Shear, Taper, Twist, Squash & Stretch, Spherify, Wrap, Wind, Melt, Explosion, Explosion FX, Shatter, Formula, Displacer, Smoothing, Jiggle, Delta Mush, FFD, Mesh, Morph, Spline, Spline Rail, Surface, Camera, Collision, Correction, Reversion-type correctives, Polygon Reduction
- Common: Mode (limited/unlimited/within box), Size, Strength, Fit to Parent, Axis, Falloff/Fields, Order of evaluation (parent-child deformer stack), Deformer on generators vs editable mesh
- Each: parameter list, defaults, math, golden mesh comparison.

Bend, Twist, Taper, Bulge, Shear, Spherify, Squash & Stretch and Wrap are specified below
(§0–7, §9); falloff is §10; Explosion, Shatter and Wind are §11–12 (brief 053, source
docs/parity/deformers-3.md O19); Mesh (cage), Surface, Camera, Collision, Jiggle, Delta Mush
and Correction are §14–15 (brief 056, source docs/parity/deformers-4.md O45). Sources:
docs/parity/{bend,twist,deformer-common,taper,bulge,shear,spherify,deformers-2,followups-1}.md and their golden
cases. Everything else is still a checklist item.

Status: draft · Source: docs/parity/{bend,twist,deformer-common,taper,bulge,shear,spherify}.md

## 0. Common deformer behaviour

### Where a deformer applies

- A deformer applies to its **parent's mesh**, after the parent has built its geometry
  (`deformer-common/generator`: the Cube is subdivided first, then bent — the result has
  2560 quads *and* the bbox of `bend/strength`, so the cage is not bent before
  subdivision).
- A deformer under a parent **without geometry of its own** (a Null) applies to that
  parent's mesh children (`deformer-common/sibling`: Null > Cube, Bend is byte-identical
  to `bend/strength`). Whether a sibling *above* the object also applies is
  **parity-pending** (only "Cube, Bend" was observed).
- **Disabled** deformers do nothing (`deformer-common/disabled`, byte-identical to
  `input.obj`). The `enabled` param (bool, default `true`) is ours: the reference shows
  it as the object's green tick, not on the Object tab.
- **Stack order:** the enabled deformer children apply in object-tree order, top first
  (`deformer-common/stack-bend-twist` ≠ `stack-twist-bend`).

### Box and frame

- The box is the deformer's `size` (vec3, default 250³), centred on the deformer origin,
  so its y range is `-size_y/2 … +size_y/2` in the deformer's local space. Size clamps
  at 0 (`bend.md` Object tab).
- Points are taken from the deformed object's local space, transformed by the **inverse
  of the deformer's transform relative to that object**, deformed, and transformed back
  (`bend/moved`, `twist/moved`: position (0, 100, 0) moves the box to world y
  −25…225; `bend/rotated`: bank 90° turns the bend with the deformer).
- The deformation is parameterised by `t = (y + size_y/2) / size_y`, anchored at the
  **bottom** of the box. Modes (`limited` default, `within_box`, `unlimited`) decide
  what happens outside the box **in y**:
  - `limited`: below the box the point is unchanged; inside it follows the deformation;
    above it it is carried rigidly along the tangent at the top of the box
    (`bend/strength`: the cube part above y = 125 continues straight, max
    (359.15, 259.15, 100)). A deformer that only scales or offsets (§3–§5) has no
    tangent to travel along: it holds the value at `t = 1` above the box, which is why
    Taper keeps its top scale there (`taper/strength`).
  - `within_box`: only points inside the box deform, everything else is unchanged —
    on **all three axes**: `|x| ≤ size_x/2`, `|y| ≤ size_y/2`, `|z| ≤ size_z/2`,
    boundaries inclusive, all tested in the deformer's local space
    (`bend/mode-box`, `twist/mode-box`: the part above the box is not carried along;
    `bend/mode-box-narrow-x`: the `x = ±100` columns stay unmoved while `|x| ≤ 50`
    bends; `bend/mode-box-narrow-z`: all 90 vertices at `|z| = 100` exactly unmoved,
    versus the 40 in `bend/mode-box` that are outside in y as well). Second rule, the
    **suppression**: with `ex`, `ez` the x and z extents (max − min) of the object's
    points in the deformer's local space, the whole deformation is skipped when
    `size_x < ex` **and** `size_z < ez`, i.e. when the box spans the object in neither
    axis (`bend/mode-box-narrow`: size (100, 250, 100) against the 200 × 400 × 200
    cube → the undeformed cube, although points lie inside the box). "Spans" is
    `size ≥ extent`: the interactive sweep in `docs/parity/bend.md` gives (100, 250, 200)
    deforms (`size_z = ez`) and (120, 250, 190) does not. The threshold is relative to
    the object (a 100 × 400 × 100 cube with box (120, 250, 150) bends), which is why it
    uses the extent. Both rules apply to `within_box` only: in `limited` the box's x/z
    size is ignored (`deformer-common/on-plane`: points with |x|, |z| > 125 deform
    normally). **Implemented for Bend only** (the one deformer observed); applying it to
    the other deformers is **parity-pending** until a golden case narrows their box.
  - `unlimited`: the same rate continues below and above the box
    (`bend/mode-unlimited`: min y −299.40 = the arc at t = −0.3; `twist/mode-unlimited`:
    the corner (−100, −200, −100) → (−134.50, −200, −43.70) = a −27° turn).
- Rotation about the deformer's Y axis used by all deformers (left-handed export
  coordinates, sense fixed by `twist/angle`: (−100, 0, −100) → (0, 0, −141.42)):
  `R(θ)(x, z) = (x·cos θ − z·sin θ, x·sin θ + z·cos θ)`, y unchanged.

### Parameters

| Key | Label | Type | Default | Range / options |
|---|---|---|---|---|
| `size` | Size | vec3 | (250, 250, 250) | ≥ 0 |
| `mode` | Mode | enum | `limited` | `limited`, `within_box`, `unlimited` |
| `enabled` | Enabled (green tick) | bool | `true` | – |

Percent params (`strength`, `curvature`) carry `Unit::Percent` and store **the number
the app shows** — 50 = 50 %, the convention of `end_scale` (GENERATORS2 §10) — and are
divided by 100 in the maths.

Fit to Parent (a button in the app) is **not implemented** (no observation uses it). The
Falloff/Fields list is §10.

## 1. Bend (`forge.bend`)

### Parameters

| Key | Label | Type | Default | Unit |
|---|---|---|---|---|
| `strength` | Strength | float | 0 | angle (radians stored, degrees in case files) |
| `angle` | Angle | float | 0 | angle |
| `keep_y_axis_length` | Keep Y-Axis Length | bool | `false` | – |

### Math

With `h = size_y`, `sx = size_x`, `α = strength`, `t` from §0 and the neutral-line
radius

```
R = h/α + (keep_y_axis_length ? 0 : size_x/2)
```

(`α = 0` → no deformation, confirmed by `bend/strength-zero`: strength 0° with angle 45°
exports the undeformed cube). The bend happens in the (x, y) plane, z carried through;
the neutral line (x = 0) follows the circle with centre `C = (R, −h/2)` and radius `R`:

```
φ = α·t
A(φ) = (R − R·cos φ, −h/2 + R·sin φ)        (a point of the neutral line)
N(φ) = (cos φ, −sin φ)                      (unit normal, points along +x at φ = 0)
T(φ) = (sin φ, cos φ)                       (unit tangent, points along +y at φ = 0)

inside the box : q = A(φ) + x·N(φ)
above the box  : q = A(α) + x·N(α) + (y − h/2)·T(α)      (limited only)
outside (limited / within_box): q = (x, y)
```

`angle` (β) turns the bend direction: the point is taken into the bend frame with
`R(−β)` first and the result brought back with `R(β)` (§0 rotation), so
`p' = R(β)·bend(R(−β)·p)`.

**Why `R` contains `size_x/2`:** with Keep Y-Axis Length *on* the neutral line keeps the
box height as its arc length, which forces `R = h/α` (`bend/keep-length`: bbox max
(234.155, 134.155) = `R + 75`, `159.155 + 75`). With it *off* (the default) the observed
result is `R = h/α + size_x/2`, i.e. the arc of radius `h/α` sits on the box's **+X
face** instead of on the centre line, so the shape stretches:
- `bend/strength` (h = 250, α = 90°): `R = 159.1549 + 125 = 284.1549` → bbox max
  (359.155, 259.155) = `(R + 75, R − 25)`.
- `bend/strength-180` (α = 180°): `R = 79.577 + 125 = 204.577` → bbox centre x
  204.577 and max x `2R + 100 = 509.155`.
- `bend/size` (h = 200, size (100, 200, 100)): `R = 127.324 + 50 = 177.324` → bbox max
  (277.324, 177.324).
- `deformer-common/on-plane` (plane at y = 0, t = 0.5, φ = 45°): (−200, 0, −200) →
  (−58.19, 217.35, −200), computed with `R = 284.1549`.

`bend/size-nonuniform` (size (300, 250, 100), h = 250) settles the term: `R = 159.155 +
150 = 309.155` → bbox max (384.155, 284.155), which matches `size_x/2` and rules out
`size_z/2` (`R` = 209.155 → max (284.155, 184.155)) and `(size_x + size_z)/4`
(`R` = 259.155 → max (334.155, 234.155)). It does not depend on `angle`: `bend/angle`
matches with the same `R`.

### Cases (all cite their golden folder)

| Case | Rule it shows |
|---|---|
| `defaults` | α = 0 → the undeformed cube (= `input.obj`) |
| `strength` | 90° over the box; below unchanged, above carried straight; bbox max (359.155, 259.155, 100) |
| `strength-180` | 180°: the top ends pointing down, max x 509.155 = `2R + 100` |
| `angle` | β = 45°: `p' = R(β)·bend(R(−β)·p)`; x and z extents equal (353.96), max y 300.58 |
| `mode-unlimited` | t extrapolates below the box (min y −299.40, min x −95.27) |
| `mode-box` | only in-box points deform (max (255.35, 254.43): the vertex at y = 100, φ = 81°) |
| `size` | the box (100, 200, 100) drives `h` and the `size_x/2` term |
| `size-nonuniform` | box (300, 250, 100): the half-size term is `size_x/2`, not `size_z/2` or `(size_x+size_z)/4` (max (384.155, 284.155)) |
| `keep-length` | `R = h/α`, no `size_x/2` (bbox max (234.155, 134.155)) |
| `moved` | the inverse deformer transform: box at world y −25…225 (max (255.35, 354.43)) |
| `rotated` | bank 90°: the bend frame follows the deformer's transform |
| `mode-box-narrow` | box (100, 250, 100): `within_box` suppressed entirely (undeformed cube) — the box spans the object's x/z extent in neither axis |
| `mode-box-narrow-x` | box (100, 250, 250): `within_box` gates per point on x (outer columns unmoved, inner bent to x 184.26) |
| `mode-box-narrow-z` | box (250, 250, 100): `within_box` gates per point on z (same bbox as `mode-box`, z = ±100 columns unmoved) |
| `strength-zero` | α = 0 with angle 45° → the undeformed cube: a zero strength means no deformation regardless of the angle |

`within_box` details (suppression and per-point x/z gating) are §0. In `bend.rs` the
gate and the suppression are evaluated on the point in the deformer's local space
**before** the `angle` rotation (the box is not turned by `angle`; `angle` with
`within_box` was not observed: **parity-pending**).

## 2. Twist (`forge.twist`)

### Parameters

| Key | Label | Type | Default | Unit |
|---|---|---|---|---|
| `angle` | Angle | float | 0 | angle (radians stored) |

There is no strength: `angle` is the only amount.

### Math

`φ = α·t` with `t` from §0 (the mode rules decide which `t` a point gets), then

```
p' = (R(φ)(x, z), y)
```

i.e. a rotation about the deformer's Y axis, linear in y over the box: 0° at
`y = −size_y/2` and `α` at `y = +size_y/2`. The y coordinate never changes, so the
result only widens where the section turns 45°.

### Cases

| Case | Rule it shows |
|---|---|
| `defaults` | α = 0 → the undeformed cube (= `input.obj`) |
| `angle` | 90°: at y = 0 the corner (−100, 0, −100) → (0, 0, −141.42) (fixes the rotation sense) |
| `angle-big` | 360°: at y = 0 the same corner → (100, 0, 100); above the box the full turn |
| `mode-unlimited` | the rate continues below the box (corner → (−134.50, −200, −43.70) = −27°) |
| `mode-box` | points above the box keep their rest position |
| `size` | box y −100…100: 90° over 200 units, full 90° above it |
| `moved` | the inverse deformer transform (box at world y −25…225) |

## 3. Taper (`forge.taper`)

### Parameters

| Key | Label | Type | Default | Unit / range |
|---|---|---|---|---|
| `strength` | Strength | float | 0 | percent, unbounded (negative widens: `taper/negative`) |
| `curvature` | Curvature | float | **100** | percent, ≥ 0 |
| `fillet` | Fillet | bool | `false` | – |

Plus `size` and `mode` from §0. **Taper and Shear share one curve** (§5 uses this `f`).

### Math

With `s = strength/100`, `c = curvature/100` and `t` from §0:

```
f(t) = fillet ? 3t² − 2t³ : c·t² + (1 − c)·t
scale = 1 − s·f(t)        applied to x and z; y is unchanged
```

`curvature` blends the linear profile (`c = 0`: `f = t`) and the quadratic one
(`c = 1`: `f = t²`); `fillet` replaces the blend with `smoothstep(t)`, so it starts and
ends flat. Above the box in `limited` the scale at `t = 1` is held (`1 − s`), the box is
anchored at its bottom (scale 1 at `y = −size_y/2`).

**Why the blend is `c·t² + (1 − c)·t`:** `taper/strength` (c = 1) gives the rows
1, 1, 0.995, 0.955, 0.875, 0.755, 0.595, 0.5, 0.5 = `1 − 0.5·t²` for
t = 0.1, 0.3, 0.5, 0.7, 0.9; `taper/curvature` (c = 0.5) gives 0.9725, 0.8125, 0.5725 at
the same rows, i.e. `f` = 0.055, 0.375, 0.855 = `0.5·t² + 0.5·t`; `taper/fillet` gives
0.986, 0.75, 0.514 = `1 − 0.5·(3t² − 2t³)` (`f` = 0.028, 0.5, 0.972). `taper/size`
moves the same curve with the box (y −100…100: rows 1, 1, 1, 0.96875, 0.875, 0.71875,
0.5, 0.5, 0.5).

### Cases

| Case | Rule it shows |
|---|---|
| `defaults` | strength 0 → the undeformed cube (= `input.obj`) |
| `strength` | `1 − 0.5·t²`, held above the box; bbox unchanged |
| `negative` | `s < 0` widens toward the top (rows … 1.5, bbox 300 × 400 × 300) |
| `curvature` | c = 0.5 blends linear and quadratic (`f` = 0.055 at t = 0.1) |
| `mode-unlimited` | t extrapolates: y −200 → 0.955 (t = −0.3), y 200 → 0.155 (t = 1.3) |
| `mode-box` | the rows above the box are back to 1 |
| `size` | box y −100…100 drives `t` (rows 1, 1, 1, 0.96875, …) |
| `fillet` | `f = 3t² − 2t³` instead of the curvature blend |

## 4. Bulge (`forge.bulge`)

### Parameters

Identical to Taper (§3): `strength` (0), `curvature` (**100**), `fillet` (off), plus
`size` and `mode`. There is no `size` golden case: `u` only reads `size_y`, which §0
fixes for every deformer.

### Math

With `s = strength/100`, `c = curvature/100` and `u = y / (size_y/2)` — **centred on the
box**, unlike Taper's `t` — and `smoothstep(x) = 3x² − 2x³`:

```
g(u) = fillet ? smoothstep(1 − |u|) : c·(1 − u²) + (1 − c)·(1 − |u|)
scale = 1 + s·g(u)        applied to x and z; y is unchanged
```

`g(±1) = 0`: the bulge vanishes at the ends of the box, which is why `limited` and
`within_box` give byte-identical results here (`bulge/mode-box` = `bulge/strength`), and
why the blend is symmetric around the middle (`u = 0`, the maximum).

**Why:** `bulge/strength` (c = 1) gives rows −100, −100, −118, −142, −150, −142, −118,
−100, −100 = `1 + 0.5·(1 − u²)` for u = −0.8, −0.4, 0 (`g` = 0.36, 0.84, 1);
`bulge/curvature` (c = 0.5) gives −114, −136 → `g` = 0.28, 0.72 = the blend at the same
u; `bulge/fillet` gives −105.2, −132.4 → `g` = 0.104, 0.648 = `smoothstep(1 − |u|)` at
x = 0.2, 0.6; `bulge/mode-unlimited` extrapolates u (y = −200 → u = −1.6 → `g` = −1.56 →
0.22, row −22).

### Cases

| Case | Rule it shows |
|---|---|
| `defaults` | strength 0 → the undeformed cube (= `input.obj`) |
| `strength` | `1 + 0.5·(1 − u²)`, centred, 1 at the box ends |
| `negative` | `s < 0` pinches (rows … −50 …) |
| `curvature` | c = 0.5: `0.5·(1 − u²) + 0.5·(1 − |u|)` |
| `mode-unlimited` | u extrapolates past the box (y ±200 → scale 0.22) |
| `mode-box` | byte-identical to `strength` (`g(±1) = 0` already) |
| `fillet` | `smoothstep(1 − |u|)` instead of the blend |

## 5. Shear (`forge.shear`)

### Parameters

Like Taper (§3) plus `angle`:

| Key | Label | Type | Default | Unit |
|---|---|---|---|---|
| `strength` | Strength | float | 0 | percent, unbounded |
| `angle` | Angle | float | 0 | angle (radians stored) |
| `curvature` | Curvature | float | **100** | percent, ≥ 0 |
| `fillet` | Fillet | bool | `false` | – |

### Math

The offset uses **Taper's `f`** (§3) and points along `angle` turned about Y (§0's
rotation, so +X at angle 0):

```
m = (strength/100) · (size_y/2) · f(t)
p' = p + m·(cos angle, 0, sin angle)       (y unchanged)
```

Turning the offset direction is the same as turning the frame (`R(β)(R(−β)p + m·x̂) =
p + m·R(β)x̂`), so `angle` is exactly the operation of §1's Angle.

**Why:** `shear/strength` gives the rows −100, −100, −99.375, −94.375, −84.375,
−69.375, −49.375, −37.5, −37.5, i.e. offsets 0.625, 5.625, 15.625, 30.625, 50.625,
62.5 = `62.5·f(t)` with `62.5 = 50 % × 125` (`size_y/2`) and `f = t²`;
`shear/curvature` and `shear/fillet` match Taper's `f` (3.4375 = `62.5·0.055`,
1.75 = `62.5·0.028`); `shear/angle` (45°) shifts X *and* Z by up to
`62.5·cos 45° = 44.19` (bbox 244.19, centre 22.097); `shear/mode-unlimited` gives
y −200 → 5.625 (`t = −0.3`) and y 200 → 105.625 (`t = 1.3`).

### Cases

| Case | Rule it shows |
|---|---|
| `defaults` | strength 0 → the undeformed cube (= `input.obj`) |
| `strength` | offset `62.5·t²` along +X, held above the box |
| `angle` | the offset turned 45° toward +Z (both extents 244.19) |
| `curvature` | Taper's blended `f` (offset 3.4375 at t = 0.1) |
| `mode-unlimited` | the parabola extrapolates (y ±200 → +5.625 / +105.625) |
| `mode-box` | the rows above the box are not moved |
| `fillet` | Taper's `smoothstep(t)` (offset 1.75 at t = 0.1) |

## 6. Spherify (`forge.spherify`)

### Parameters

| Key | Label | Type | Default | Unit / range |
|---|---|---|---|---|
| `radius` | Radius | float | 200 | distance, ≥ 0 |
| `strength` | Strength | float | **50** | percent, clamped 0…100 |

**No `size` and no `mode`** (the observation lists neither): every point is treated,
regardless of where it is. `enabled` from §0 still applies.

### Math

In the deformer's local space (centre = the deformer's origin, so §0's relative
transform puts the sphere where the deformer is):

```
p' = p + (strength/100) · (radius · p/|p| − p)
```

a linear blend between the point and its radial projection onto the sphere — no
distance falloff (`spherify/moved` projects the whole cube). The origin (`|p| = 0`) is
left where it is; that guard was never exercised by a golden case.

**Why:** `spherify/defaults` (strength 50) sends the corner (−100, −200, −100)
(|p| = 244.949) to (−90.82, −181.65, −90.82) = halfway to its projection
(−81.65, −163.30, −81.65); `spherify/strength-full` puts every point at exactly
200 from the origin (bbox 400³); `spherify/radius` at exactly 100; `spherify/strength`
is byte-identical to `defaults`; `spherify/moved` centres the sphere on (50, 100, 0).

### Cases

| Case | Rule it shows |
|---|---|
| `defaults` | the default 50 %: already deformed, halfway to the sphere |
| `strength` | strength 50 typed explicitly → byte-identical to `defaults` |
| `strength-full` | 100 %: every point at distance `radius` |
| `radius` | radius 100 at 100 % → every point at distance 100 |
| `moved` | the sphere follows the deformer's transform (centre (50, 100, 0)) |

## 7. Squash & Stretch (`forge.squash_stretch`) and Wrap (`forge.wrap`)

Source: O9 [deformers-2.md](../parity/deformers-2.md) (parameter tables, `squash-stretch/{defaults,
factor,factor-big,bulge}`, `wrap/{defaults,cylindrical,angles}`) and O11
[followups-1.md](../parity/followups-1.md) §3 (`squash-stretch/{expand-0,expand-200}`,
`wrap/{spherical,width-90}`); golden `tests/golden/{squash-stretch,wrap}` (11 cases). Both
carry the `fields` list of §10 and the `enabled` switch of §0. Neither has `size` or
`mode`: Squash & Stretch has its own `top` / `bottom`, Wrap its `width` / `height` /
`radius`. Case files use the observation's keys (`center_percent`, `longitude_start_deg`);
the param ids drop the `_percent` / `_deg` suffix (percent params store the number the
app shows, angles store radians).

### 7.1 Squash & Stretch

| Key | Label | Type | Default | Range |
|---|---|---|---|---|
| `top` | Top | float | 100 | ≥ 0 |
| `bottom` | Bottom | float | −100 | ≤ 0 |
| `center` | Center | percent | 0 | −100…100 |
| `aspect` | Aspect | percent | 100 | – |
| `factor` | Factor | percent | 100 | ≥ 0 |
| `expand` | Expand | percent | 100 | ≥ 0 |
| `smooth_start`, `smooth_end` | Smooth Start / End | percent | 0 | 0…100 |
| `curvature` | Curvature | percent | 100 | 0…100 |
| `curvature_type` | Type | enum | `square` | `square`, `cubic`, `quartic`, `custom`, `spline` |
| `power` | Power | float | 2 | ≥ 0 |
| `fields`, `enabled` | | | | §10, §0 |

The curve of the `spline` type is not recorded (O9) and has no param yet.

**Math.** In the deformer's local space, with `f = factor/100` (clamped to ≥ 1e-6),
`e = expand/100`, `c = curvature/100`. Effective ends `Ht = top + ε`, `Hb = bottom − ε`
with **ε = 0.005** and the pivot `y0 = center/100 · (Ht if center ≥ 0 else −Hb)` (0 for
the default). The half-lengths of the two sides are `Lt = Ht − y0`, `Lb = y0 − Hb`.

```
u  = (y − y0)/Lt   for y ≥ y0,   (y − y0)/Lb   for y < y0           (−1…1 inside)
g(u) = c·(1 − |u|ⁿ) + (1 − c)·(1 − |u|)      n = 2 square, 3 cubic, 4 quartic
inside  (Hb ≤ y ≤ Ht): y' = y0 + (y − y0)·f ,  (x, z)' = (x, z)·(1 + e·(1/f − 1)·g(u))
above   (y > Ht):      y' = y + Lt·(f − 1)      (x, z unchanged)
below   (y < Hb):      y' = y − Lb·(f − 1)      (x, z unchanged)
```

The span between the ends is squashed uniformly (`y·f`), the volume is kept at
`e = 100 %` by widening the middle by `1/f`, and the parts beyond the ends travel rigidly
with the ends. `Expand` scales the widening, not the exponent (`expand-200` is ±300, not
±400). `curvature` and `n` follow Bulge's blend (§4), whose shape the same Square type has.

Why: `squash-stretch/factor` (f = ½, y rows −200…200 step 50): y' = −149.9975, −99.9975,
−50, −25, 0, … = `y·f` inside, `y + 50.0025` outside (`Lt·(f − 1) = 100.005·½`); the x
of the −50 row is 175.0025 = `100·(1 + 0.750025)`, `g = 1 − 0.5²` with `u = 50/100.005`;
the −100 row has 100.01 (`u = 0.99995`, `g = 1.0e-4`). The ε explains both small offsets.
`factor-big` (f = 1.5): y ±250.0025 = 200 + 50.0025, centre x scale 2/3. `defaults`
(f = 1): the cube. `expand-0`: x stays ±100; `expand-200`: ±300. `bulge` is the
same as `factor` (the observer used Expand = 100 % as the nearest setting).

**Parity-pending (implemented, no data):**
- `ε` as an additive 0.005 versus a factor 1.00005 (both give 100.005 for `top` = 100; a
  case with `top` = 50 would tell).
- `center` (the pivot formula, and that the squash pivots about it), and `aspect` (not
  applied: it only has its default 100 %).
- `smooth_start`, `smooth_end` (not applied: only 0 %), `curvature` < 100 % and the
  `cubic` / `quartic` shapes (the exponent family is Bulge's by analogy).
- **Unsupported (the points are left unchanged):** `custom` (uses `power`) and `spline`
  types, which were not derived.

### 7.2 Wrap

| Key | Label | Type | Default | Range |
|---|---|---|---|---|
| `width` | Width | float | 400 | ≥ 0 |
| `height` | Height | float | 400 | ≥ 0 |
| `radius` | Radius | float | 200 | ≥ 0 |
| `wrap` | Wrap | enum | `cylindrical` | `spherical`, `cylindrical` |
| `longitude_start`, `longitude_end` | Longitude Start / End | angle | 180°, 360° | – |
| `latitude_start`, `latitude_end` | Latitude Start / End | angle | −45°, 45° | −90°…90° |
| `movement` | Movement | float | 0 | – |
| `scale_z` | Scale Z | percent | 100 | 0…10000 |
| `tension` | Tension | percent | 100 | 0…100 |
| `fields`, `enabled` | | | | §10, §0 |

**Math.** The plane `(x, y)` of the deformer's space is rolled onto the cylinder or sphere
of `radius`, whose axis/centre sits at `z = radius`. With `d = radius − z·s` (`s` =
`scale_z/100`; the distance from the axis, so the front face `z = −100` is 300 away),
`lon = (lon_start + lon_end)/2 + (x + movement)/width·(lon_end − lon_start)`, and
`φ = lon − 270°`:

```
cylindrical:  p' = (d·sin φ,  y,  radius − d·cos φ)                      (height unused)
spherical:    θ = lat_mid + y/height·(lat_end − lat_start)
              p' = (d·cosθ·sin φ,  d·sinθ,  radius − d·cosθ·cos φ)
result = p + (tension/100)·(p' − p)
```

So the horizontal midpoint of the span (`lon` = 270° by default) faces −z, and `x` runs
along the longitude with `width` ↔ the whole span. A zero `width` (or, spherical, zero
`height`) leaves the points alone.

Why: `wrap/defaults`: `x = −100, z = −100` → `φ = −45°`, `d = 300` →
(−212.132, −12.132); `x = 0` → (0, −100); `x = −50` → (−114.805, −77.164) (= `d·sin 22.5°`,
`200 − d·cos 22.5°`). `wrap/cylindrical` is the same file, `wrap/angles` (Height 300) too:
height is unused cylindrically. `wrap/width-90` (End 270°): mid 225°, `x = −100` →
`lon` 202.5°, `φ = −67.5°` → (−277.164, 85.195). `wrap/spherical`: `y = 200` →
`θ = 45°` → (−150, 212.132, 50) = (`d·cos45·sin(−45°)`, `d·sin45`, `200 − d·cos45·cos45`);
the `y = 0` row equals the cylindrical one.

**Parity-pending (implemented, no data):** `movement` (taken as a shift of `x` in length
units before the mapping), `scale_z` (scales `z` in `d`), `tension` (linear blend with the
unwrapped point); latitude start/end other than ±45° (their mid and span), longitude
mids other than 225° and 270° (the rule `φ = lon − 270°` was fitted on those two),
`width` / `radius` other than the defaults (O9 notes they "did change the mesh").

### Cases

| Case | Rule it shows |
|---|---|
| `squash-stretch/defaults` | f = 1 → the undeformed cube |
| `squash-stretch/factor`, `bulge` | f = ½: `y·f` inside, `±50.0025` outside, widening `1 + 0.75` at −50 |
| `squash-stretch/factor-big` | f = 1.5: tall, y ±250.0025 |
| `squash-stretch/expand-0`, `expand-200` | the widening scales with Expand (±100, ±300) |
| `wrap/defaults`, `cylindrical`, `angles` | cylindrical roll of 180°, height unused |
| `wrap/width-90` | the span is `lon_end − lon_start`, centred at its mid |
| `wrap/spherical` | latitude from `y/height` over `lat` span |

## 8. Evaluation context

`forge_core::EvalCtx { time: f64, fps: f64, seed: u64 }` (seconds, frames per second, RNG
seed) is passed to `forge_generators::evaluate(scene, id, &ctx)` and from there to
`forge_modifiers::apply_deformer(scene, deformer, target, points, &ctx)`. The default is
frame 0 at 30 fps with seed 0 (`EvalCtx::default()`), `EvalCtx::at_frame(frame, fps)`
builds the context of a frame. Interactive callers pass the document time; no deformer
reads the context yet (time-dependent deformers and effectors will), so results do not
change. Source: own design (docs/roadmap/areas/D-deformers-fields.md).

## 9. Parity

Test: `crates/modifiers/tests/deformers_parity.rs` — features `deformer-falloff` (9) and `bend` (15 cases),
`twist` (7), `deformer-common` (6), `taper` (8), `bulge` (7), `shear` (7),
`spherify` (5), `squash-stretch` (6, §7.1) and `wrap` (5, §7.2) — 75 in all — are rebuilt from their `case.toml` (`parent`, `hierarchy`,
`rot` degrees HPB, vec3 `size` are the keys the batch added; `level` maps to
`level_view`) and compared with `export_weld` on both sides (GENERATORS2 §0).

- `UNSUPPORTED`: none.
- `KNOWN_MISMATCH`: none (the three `bend/mode-box-narrow*` cases matched once §0's
  per-axis gate and suppression landed).
- Feature `deformer-falloff` (9 cases, §10) is built as: the cube, a deformer child with
  the case's parameters (for the 051 features `squash-stretch` and `wrap` the same key translation applies: `*_percent`, `*_deg` suffixes dropped, option labels lower-cased, type `forge.squash_stretch`); (`strength_deg` is `strength` in degrees, `keep_length` is
  `keep_y_axis_length`, `alignment` is the fixed Y axis), the `[params.field]` field node
  linked into `fields` (its `layer_strength_percent` is the node's `layer_strength`).
- Evaluation order and placement rules are §0; the modifier maths are §1–§6.
- Parity-pending (implemented, seen in one configuration only): sibling deformers below
  the object (§0), `fillet` together with
  `curvature < 100` (§3, §4, §5 — the one fillet case has curvature 100, so "fillet
  replaces the blend" and "fillet blends into it" cannot be told apart), the curvature
  blend extrapolated *below* the box in `unlimited` (§3–§5 — every unlimited case has
  curvature 100, so only the quadratic term was seen outside the box), and Spherify at
  the origin / `radius` 0 (§6).

## 10. Falloff through field lists (brief 061)

Source: O24 [deformer-falloff.md](../parity/deformer-falloff.md), golden `tests/golden/deformer-falloff`
(9 cases, level A); the field maths is [FIELDS.md](FIELDS.md).

### Parameter

Every deformer (Bend, Twist, Taper, Bulge, Shear, Spherify, Squash & Stretch, Wrap) has a `fields` param:
`ParamKind::LinkList` accepting `Category::Field` nodes, **top to bottom**, default empty
(label "Fields", group "Falloff"). The O24 deformer's list parameter is labelled "Field"
on its Fields tab. The list row's *layer strength* (O24 `layer_strength_percent`) is the
field node's `layer_strength` param (percent, default 100 %, see FIELDS.md F1): one field
node is one row, which is all the data covers; real list rows (a field in several lists)
come with brief 059.

### Rule

`FieldList::from_nodes` over the live field links of the list (a link to a deleted or
non-field node is skipped). **An empty list means full effect** (`bend-empty-list`, no
extra switch exists on the deformer, the list is simply empty).

For a non-empty list, each point is deformed as usual and then blended with its undeformed
position:

```
w  = FieldList::sample(world(p)).value       p = the point before this deformer, in the object's space
p' = p + w · (deform(p) − p)
```

`world(p)` is the point taken with the **world matrix of the deformed object** (the
parent of the deformer); the deformer's own position does not move the field. `w` is not
clamped (a field value outside 0..1 extrapolates; fields clamp at their own Min/Max
unless switched off). The weight is read at the point's position *before* the deformer,
not after. A field that cannot be built (an option without derived rules, FIELDS.md) leaves
the points unchanged. The rule applies to every deformer, after the mode rules of §0 (the
blend sees the deformer's final per-point result, including `limited` carry-along and the
`within_box` gates), and the stack order of §0 is unchanged (each deformer blends on its
own list).

### Cases

| Case | Deformer | Field (list) | Rule it shows |
|---|---|---|---|
| `bend-empty-list` | Bend 90° | none | empty list = the full bend |
| `bend-spherical` | Bend 90° | Spherical 200 at (0, 200, 0) | weight by position, inner offset 50 % |
| `bend-spherical-inner` | the same | inner offset 0 % | the field's remapping drives the weight |
| `bend-linear` | the same | Linear Y+ length 200 | |
| `bend-box` | the same | Box (400, 200, 400) | |
| `bend-field-strength-50` | the same | Spherical, layer strength 50 % | the layer strength scales the weight |
| `twist-linear`, `taper-spherical`, `bulge-linear` | | | other deformers share the rule |

### Parity-pending

- Weight sampled at the point before the deformation (matched on all 8 field cases, whose
  weights vary strongly across the displaced points): proven by the data, no alternative
  open for the object-at-origin setup. With the deformed object **moved or rotated** away
  from the origin, "world position" vs. "object-local position" is untested (all cases have the
  cube at the origin): a Bend with the cube at (0, 100, 0) and the same Spherical field.
- Rows beyond one layer (blend modes, FIELDS.md): brief 059.
- A negative or >1 weight (Strength 200 % on the field) extrapolating vs. clamping: a Bend
  with Strength 200 % and Clamp Max off.
- Spherify's `p/|p|` with falloff at the origin (no data).

## 11. Explosion (`forge.explosion`) and Shatter (`forge.shatter`)

Source: O19 [deformers-3.md](../parity/deformers-3.md); golden `tests/golden/{explosion,shatter}`
(`defaults`, `strength-50` each). `explosion-fx/frame-10` belongs to O45 / another brief and
is not covered here. Neither deformer has a Falloff group in the reference, so neither has
a `fields` param; both carry `enabled` (§0).

| Key | Label | Type | Default | Range | On |
|---|---|---|---|---|---|
| `strength` | Strength | percent | 0 | 0…100 | both |
| `speed` | Speed | float | 100 | any | Explosion |
| `angle_speed` | Angle Speed | angle | 100° | any | both |
| `end_size` | End Size | percent | 0 | 0…1000 | both |
| `randomness` | Randomness | percent | 100 | 0…100 | both |

Case files use `strength_percent`, `angle_speed_deg`, `end_size`, `randomness_percent`.

### Topology (derived)
Both deformers **split the mesh into its polygons**: the result has the same polygon list
(same order, same corner order) but every polygon owns its own points, polygon `i` using
points `n·i … n·i + n − 1` (`explosion/strength-50`, `shatter/strength-50`: 640 points,
160 faces, face `i` = `4i+1…4i+4`). The points of an island are moved rigidly. This needs the
whole mesh, so the two deformers use `apply_deformer_mesh` (the points-only
`apply_deformer` ignores them). With `strength = 0` the mesh is returned **unchanged and
not split** (`explosion/defaults`, `shatter/defaults`: the export of the cube is identical
to the undeformed cube).

### Rigid motion per polygon (derived, proven by both strength-50 cases)
Each polygon `P` with centre `c` (mean of its points) goes to
`c' + k · Rot · (p − c)` for every point `p`, with a rotation `Rot`, a uniform scale `k` and
a new centre `c'`. A Kabsch fit of all 160 polygons of each case gives residual 0 (≤ 1e-6)
with `k = 0.5` exactly. With `s = strength/100`:

- `k = 1 + (end_size/100 − 1)·s` (`strength-50`, `end_size 0`: `1 − s = 0.5`).
  **Parity-pending:** linear in `s` and the unit of `end_size`: one strength only.
- `Rot = Ry(ey) · Rx(ex) · Rz(ez)` with the three angles independent and uniform in
  `±h`, `h = 0.75 · angle_speed · s` (37.5° at 100°, 50 %). Proven by the Euler fit of the
  320 rotations: of the six axis orders only `Y X Z` gives a symmetric spread, max
  |angle| 36.6–37.5° on each of the three axes (both deformers), std ≈ 21.5 ≈ 37.5/√3.
  **Parity-pending:** the factor 0.75 and the linearity in `s` and `angle_speed`.

### Centre `c'`
- **Explosion:** `c' = c + d`, `|d|` uniform in `[0, 15 · speed · s]` (max measured 749.06
  of 750 = 15·100·0.5; the 160 lengths fill 8 histogram bins evenly), direction a unit
  vector with a **radial bias**: `normalize(normalize(c) + 1.5·R·v)` with `v` a uniform unit
  vector and `R = randomness/100`. Measured mean of `dir·normalize(c)` is 0.405 (a pure random
  direction would give 0; the blend factor 1.5 gives 0.44, 1.65 gives 0.40).
  **Parity-pending:** all of it (one strength, one speed, one randomness); the 15, the bias
  law and the role of `R`.
- **Shatter:** `y' = y · (1 − min(1, 2s))` (all 160 centres land exactly on y = 0 at
  `s = 0.5`; the law between 0 and 0.5 is a guess), and `x' = x + dx`, `z' = z + dz` with
  `dx, dz = 100 · s · R · (u1 + u2 − 1)` (a triangular spread: measured std 20.8 and 19.1,
  max 46.7 of 50, lengths peaked around 25). **Parity-pending:** all of it.

`R < 1`: every uniform draw `u` is replaced by `(1 − R)/2 + R·u`, so `R = 0` gives no
rotation and no random offset. **Parity-pending** (never observed below 100 %).

### Random stream (not matchable; compared statistically)
The reference draws from one seeded stream polygon by polygon: Shatter polygon `7m + 6` has
the same rotation as Explosion polygon `6m + 5` for every block `m` (22 pairs, equal to
1e-4°), i.e. Shatter consumes 6 values and Explosion 7 per polygon from the same seed.
The generator itself is not known, so values cannot be reproduced. Ours is SplitMix64
seeded with `ctx.seed` (deterministic, each polygon draws in order). The strength-50 cases
therefore **cannot match point by point**; `KNOWN_MISMATCH` pins the exact comparison and the
parity test applies the same *statistical* checks to the golden and to our mesh (§9):
topology, `k`, rigidity, Euler range / spread, Shatter `y = 0` and offset bounds, Explosion
`|d|` bound and radial bias.
Follow-up case suggestions: strength 25 % and 100 %, End Size 100 %, Randomness 0 %, Speed
and Angle Speed 0 and 200 on a 1-polygon plane (a single polygon pins each law
in one read), and the same on a 2-polygon plane to see the stream order.

## 12. Wind (`forge.wind`)

Source: O19; golden `tests/golden/wind/frame-10` (cube 4/8/4, defaults, frame 10 at 30 fps).

| Key | Label | Type | Default | Range |
|---|---|---|---|---|
| `amplitude` | Amplitude | float | 50 | ≥ 0 |
| `size` | Size | float | 100 | ≥ 0 |
| `frequency` | Frequency | float | 1 | ≥ 0 |
| `turbulence` | Turbulence | percent | 50 | 0…100 |
| `fx`, `fy` | fx, fy | float | 1, 1 | ≥ 0 |
| `flag` | Flag | bool | on | – |
| `fields`, `enabled` | | | | §10, §0 |

The cube keeps its topology. Time comes from `EvalCtx.time` (frame 10 at 30 fps = 1/3 s).

### Derived (all 162 points of `wind/frame-10`)
- Only **z** moves; x and y are exact, the face list is unchanged.
- The offset `dz` is a function of `(x, y)` only (equal for every z).
- `dz = 0` exactly at `x = 0`; `dz` has the sign of `x`; `|dz|` is at most 72.44, below
  `1.5 · amplitude = 75`. Along y it is smooth, peaking near y = −50 for x = 100 (61.4)
  and falling to 27–42 at y = ±200.

### Model (ours, parity-pending)
In the deformer's local space with `q = (x·fx/size, y·fy/size, t·frequency)`, `t` seconds,
`turb = turbulence/100`:
`n = 1 + 0.5 · (perlin(q) + turb · perlin(2q + offset)) / (1 + turb)` (a gust in
`0.5…1.5`, never reversing: the golden `dz` has the sign of `x` at all 162 points), and
`dz = amplitude · n · g(x)` with `g(x) = sign(x) · sqrt(min(1, |x|/size))` when `flag` is on
(pinned along x = 0), else 1. `size = 0` or `amplitude = 0` leaves the points alone.
The reference noise function is not known, so the exact values cannot be matched; the
test checks the invariants above on both meshes plus `max|dz| ≤ 1.5·amplitude`.
**Parity-pending:** noise type and range, the weight `g` (measured mean `|dz|/amplitude` is
about 0.81 at |x| = 50 and 0.94 at |x| = 100, between a square-root and a linear weight),
`fx`/`fy` as sampling factors, `flag` off, the role of `frequency` and `turbulence`.
The case file has no `frame` / `fps` keys although its description and the form (O19) say
frame 10 at 30 fps; the test uses 10 / 30 when they are absent.
Follow-up case suggestions: frame 0 and frame 20 (does it move over time), `flag` off,
`size` 50 / 200, a plane with 1 row of points along x (to read `g` and the noise).

## 13. Parity of §11–12

Test: `crates/modifiers/tests/destructive_parity.rs`, features `explosion` (2), `shatter`
(2), `wind` (1). Exact (`export_weld`, `compare_meshes`): `explosion/defaults`,
`shatter/defaults`. `KNOWN_MISMATCH` (statistical checks pass instead): `explosion/strength-50`,
`shatter/strength-50` (random stream), `wind/frame-10` (noise function).

## 14. Cage, Surface, Camera, Collision, Jiggle, Delta Mush, Correction deformers (brief 056)

Source: O45 [deformers-4.md](../parity/deformers-4.md); golden `tests/golden/{mesh-deformer,
surface,camera-deformer,collision,jiggle,delta-mush,correction}` (8 cases, observation level
A for `correction/defaults` and `delta-mush/defaults`, otherwise see below).

**What the data shows.** The observer's scripted export did not run the helper-driven
deformers (O45 Findings): `mesh-deformer/cage`, `surface/defaults`, `camera-deformer/grid`,
`collision/sphere`, `jiggle/frame-10`, `jiggle/frame-20` all export the **undeformed** (or
rigidly moved) cube, so the reference maths of those six cannot be read from them. The
algorithms below are therefore **ours** (planner design decisions of brief 056 plus
public literature), everything about them is **parity-pending**, and the cases are pinned
in `KNOWN_MISMATCH` with invariants (§15), except `camera-deformer/grid`, which matches
only because the recorded camera pose (rotated 45°) keeps the cube outside our grid (it does not
exercise the grid; §15). The two substantive exact cases are the ones whose
reference result is fully determined without a solver: `correction/defaults` (Bend, then
the stored offset of point 0) and `delta-mush/defaults` (initialised on the deformed pose,
so the output is the input).

Common: all seven carry `fields` (§10) and `enabled` (§0). They run in the points-and-mesh
path `apply_deformer_resolved` (`apply_deformer_mesh` is the same without linked meshes).
The generator layer hands the deformer a **resolver** `NodeId -> Option<PolyMesh>` that
evaluates a linked helper object (with its own deformers, cycle-guarded); the deformer takes
the mesh to world space with the helper's world matrix. Every deformer works in world space:
`P = world(target) · p`, result `world(target)⁻¹ · P'`. A linked object that is missing, has
no mesh, or a stored `bind` of the wrong length, leaves the points alone. Falloff:
`p' = p + w·(deform(p) − p)` as in §10. Percent params store the number the app shows.
Strength: `p' = p + s·(deform(p) − p)`, `s = strength/100`.

*Data params* (hidden, non-animatable `Vec3List`, written by the Initialize / Update actions of
the reference, here by the caller): `bind` (Mesh, Surface, Delta Mush), `offsets` (Camera),
`reference` and `offsets` (Correction). An empty list means "not initialised"; the bind
state is then the current state, i.e. the identity deformation.

### 14.1 Mesh (`forge.mesh_deformer`), cage with mean value coordinates

| Key | Label | Type | Default |
|---|---|---|---|
| `cages` | Cages | link list (objects) | empty |
| `strength` | Strength | percent | 100 |
| `bind` | (Initialize data) | Vec3List | empty |

Reference table (O45): Auto Init, Memory, Accuracy, External are **not implemented**
(Accuracy has no analogue: the weights are exact). `bind` = the cage's world points at
Initialize. For each point `p` (world) and each cage, the weights `w_i(p)` of the **bind**
cage (triangulated with `triangulate_poly`) are the mean value coordinates of Ju, Schaefer,
Warren 2005 (per triangle `θ_m = 2 asin(l_m/2)` of the unit-sphere edge lengths,
`c_m`, `s_m`, `w_m = (θ_m − c_{m+1}θ_{m+2} − c_{m+2}θ_{m+1}) / (d_m sin θ_{m+1} s_{m+2})`),
**normalised to sum 1** (which also fixes an inside-out cage). A point on a cage vertex takes that
vertex, a point in a triangle's plane takes the 2-D barycentric weights. Then
`p' = Σ w_i · c_i` with `c_i` the **current** cage points. With several cages they apply in order.
Proof properties (unit tests): the identity when the cage is unmoved; a cage translation
translates every point; a uniform cage scale scales it (linear precision of MVC).
Case `mesh-deformer/cage`: cage 300×500×300 (1/1/1), cage point 0 moved by (−100, 0, 0) after
Initialize. The reference export is the undeformed cube, ours moves the points near cage point 0.

### 14.2 Surface (`forge.surface_deformer`), projection

| Key | Label | Type | Default |
|---|---|---|---|
| `surface` | Surface | link (object) | none |
| `strength` | Strength | percent | 100 |
| `proportional` | Proportional | bool | off |
| `bind` | (Initialize data) | Vec3List | empty |

Type `Projection` only; `Mapping (U,V)` / `(V,U)` are **not implemented** (no UVs in the
mesh; `type` is not a param). `bind` = the surface's world points at Initialize. Each point is
bound to the nearest point `q` of the bind surface's triangles: triangle `t`, barycentric
`b`, and the offset `p − q` in the triangle frame `(e1, n×e1, n)` with `e1 = normalize(B − A)`,
`n` the triangle normal. The result is `Σ b_i · C_i + frame_now · offset`, with `C_i` the current
surface points. `proportional` scales the offset by `sqrt(area_now / area_bind)` of the triangle.
Case `surface/defaults`: plane 400×400 (10×10) at y 250 with a Bend 90° child, Initialize on the flat
plane. The golden plane is exported bent (matched by our Plane + Bend); the cube is exported
undeformed.

### 14.3 Camera (`forge.camera_deformer`), screen-space grid

| Key | Label | Type | Default |
|---|---|---|---|
| `camera` | Camera | link (object) | none |
| `strength` | Strength | percent | 100 |
| `grid_x`, `grid_y` | Grid X, Y | int | 32 (1…256) |
| `offsets` | (grid point offsets) | Vec3List | empty (all zero) |

The camera is any linked object: its world matrix is the camera frame, looking along +z
(there is no camera object type yet, so no field of view is needed). A point in camera space
`c` with `c_z > 0` has screen coordinates `u = c_x/c_z`, `v = c_y/c_z`. The grid covers
`u, v ∈ [−0.75, 0.75]` with `(grid_x+1)·(grid_y+1)` points, row-major (`index = j·(grid_x+1)+i`,
`i` along u from −0.75). The grid point offsets are screen units **at distance 1000**; a point at depth `c_z`
moves in camera space by `bilinear(offsets)(u, v) · c_z/1000` along x and y (z unchanged). Outside the grid
nothing moves. **Fitted from `camera-deformer/grid`**: `grid_points = 1089 = 33²`, and the
before position of the edited point `[−46.875, −46.875, 1000]` is a multiple of `1500/32 = 46.875`:
a square grid of half-extent `0.75 · 1000` at distance 1000 (index 15 on both axes — the form calls it
point (1, 1) / index 34, which does not fit 33 points per row: **open question**, the test uses the
grid point nearest to the recorded position). `safe_frame`, `draw_grid` are display only. The
reference export is the undeformed cube; ours equals it, **vacuously**: with the recorded camera
(at (0, 0, −800), rotated H 45° / P −19.45°) the cube lies outside the grid. The test therefore also
runs the same grid point with the camera unrotated and asserts a move in (1, 50] cm.

### 14.4 Collision (`forge.collision`), push out of collider volumes

| Key | Label | Type | Default |
|---|---|---|---|
| `colliders` | Objects | link list (objects) | empty |
| `offset` | Offset | distance | 0 |
| `strength` | Strength | percent | 100 |

Reference Solver options (Inside, Inside (Stretch), Outside, Outside (Volume), Intersect), Falloff,
Stretch/Relax and the cache group are **not implemented**; the single behaviour is the planner's
"push points out of collider volumes". A collider is a closed mesh (orientation fixed by the sign
of its signed volume). A world point is *inside* when a ray along the fixed direction
`(1, 0.0123, 0.0071)` crosses an odd number of triangles. Let `q` be the closest surface point, `n`
the outward normal (the triangle normal; `(p − q)/|p − q|` when `p` is outside and `|p − q| > 1e-9`).
If `p` is inside, or outside closer than `offset`: `p' = q + n·offset`. Several colliders apply in order.
Case `collision/sphere`: a sphere r 100 at (150, 0, 0) cuts the cube's +x side (x > 50 inside it);
the reference export keeps the cube undeformed (O45: no solver cache). Invariant on ours: no point
is strictly inside the sphere, untouched points are unchanged.

### 14.5 Jiggle (`forge.jiggle`), spring simulation with a deterministic per-frame cache

| Key | Label | Type | Default |
|---|---|---|---|
| `strength` | Strength | percent | 100 |
| `stiffness` | Stiffness | percent | 40 |
| `structural` | Structural | percent | 100 |
| `drag` | Drag | percent | 10 |
| `motion_scale` | Motion Scale | percent | 100 |
| `springs` | Springs | int | 4 |
| `iterations` | Iterations | int | 4 (1…1000) |
| `gravity` | Gravity | float | 0 |
| `local` | Local | bool | off |
| `cache` | Cache | int | 20 (1…1000) |

`On Stopped`, the vertex-map links, Restrict, Forces and the cache buttons are **not implemented**.
Level C: the reference exports of `jiggle/frame-10` and `frame-20` are the rigidly moved cube (the
solver cache was not built), so the model is ours. **The simulation is a pure function of
(scene, frame)**, so it is reproducible without hidden state; the "cache" is the window of frames it
replays: with `f = round(time·fps)` and `s = max(0, f − cache)` the state at frame `s` is at rest
(`x = T`, `v = 0`) and frames `s+1 … f` are stepped, the target world matrix of an earlier frame
`k` being read from `forge_anim::scene_at(scene, k/fps)` (the same document, tracks evaluated at `k`).
Per frame: target `T_i = P0_i + m·(W_k r_i − P0_i)`, `m = motion_scale/100`, `r_i` the input point (target space),
`W_k` the target's world matrix, `P0_i = W_s r_i`. `iterations` substeps of `h = 1/(fps·iterations)`:
`a = ω²(T − x) + g·(0, −1, 0)`, `ω² = 2000·stiffness/100`, `v += a·h`, `v *= (1 − drag/100)^(1/iterations)`,
`x += v·h`; after each substep `springs` Jacobi passes on the displacement `d = x − T`:
`d_i += 0.5·(structural/100)·(mean_{j ∈ N(i)} d_j − d_i)`, `N(i)` the edge neighbours.
Output `T + strength/100·(x − T)` taken back by `W_f⁻¹`. `local` simulates in the target's space
with a constant target `r` (the object's motion is ignored; only gravity moves the points).
Invariants (unit tests and `jiggle/*`): frame 0 and a static object give the input; with a constant
velocity the points lag behind the rigid motion; the result is identical on every call; strength 0 is rigid.

### 14.6 Delta Mush (`forge.delta_mush`)

| Key | Label | Type | Default |
|---|---|---|---|
| `strength` | Strength | percent | 100 |
| `iterations` | Iterations | int | 10 (≥ 0) |
| `pin_border_points` | Pin Border Points | bool | off |
| `bind` | (Initialize data) | Vec3List | empty |

`Smooth View`, `Tangent Mode` (only Edge is implemented), `Smooth Type`, `Mode`/`Tags` are **not
implemented**. `S(x)`: `iterations` Jacobi steps `x_i ← ½x_i + ½·mean_{j∈N(i)} x_j` (border points
fixed when `pin_border_points`; a border point is on an edge that has one polygon). Per point the
frame `F(x) = (t, b, n)`: `n` the normalised sum of the Newell normals of its polygons on the smoothed mesh, `t` the unit
projection of the edge to the lowest-index neighbour on the plane ⟂ `n`, `b = n × t`. At Initialize
(the bind pose) `δ_i = F_i(S(bind))ᵀ (bind_i − S(bind)_i)` is stored in the frame; now
`p'_i = S(p)_i + F_i(S(p)) δ_i`. At the bind pose `p' = p` exactly (the frame cancels), which is
`delta-mush/defaults`: the cube was edited (point 0 moved by (80, 80, 80)) *then* Delta Mush was
added and initialised, so the golden is the edited cube: matched. A pose change smooths out
differences and keeps the detail.

### 14.7 Correction (`forge.correction`)

| Key | Label | Type | Default |
|---|---|---|---|
| `mapping` | Mapping | enum `uv`, `nearest`, `normals` | `nearest` |
| `strength` | Strength | percent | 100 |
| `reference` | (state at Update) | Vec3List | empty |
| `offsets` | (corrected point offsets) | Vec3List | empty |

`Lock`, `Scale` are **not implemented**. At Update the deformed points of the deformers above are
stored as `reference` (target space) and the corrections edited through the point list are
`offsets` (one per reference point, zero by default). Evaluation (after the deformers above it, tree
order, §0): `p'_i = p_i + s·offsets[k(i)]`, with `k(i)` the reference point matched by the mapping:
`nearest` and `normals` the nearest reference point to `p_i` (lowest index on a tie),
`uv` the same index (`k(i) = i`, the topology is unchanged). `normals` as a distinct rule is
parity-pending. Case `correction/defaults`: under Bend 90° the corrected point 0 (at (−100, −200, −100),
below the Bend box, so unmoved by it) edited by (+50, 0, 0) gives point 0 at (−50, −200, −100) and every other
point as the plain Bend: all 162 points matched.

## 15. Parity of §14

Test: `crates/modifiers/tests/helper_parity.rs`, features `mesh-deformer`, `surface`,
`camera-deformer`, `collision`, `jiggle` (2), `delta-mush`, `correction` (8 cases; 3 exact, 5 pinned). Cases are compared
the way the reference exports a scene: helper objects first (they leak into the OBJ, O45 Findings), then the
cube (`export_weld`).

- Exact: `correction/defaults`, `delta-mush/defaults`, and (vacuously, see §14.3) `camera-deformer/grid`.
- `KNOWN_MISMATCH` (the reference scripted export did not deform, so ours — which does — differs; the exact
  comparison must still fail and the invariants below hold on ours and on the golden): `mesh-deformer/cage`,
  `surface/defaults`, `collision/sphere`, `jiggle/frame-10`, `jiggle/frame-20`.
  - helper part: the leaked cage, the bent plane and the collider sphere equal the golden (ours, exactly);
  - the golden cube equals the undeformed cube (resp. the rigidly moved cube for Jiggle);
  - ours: topology preserved, points finite, plus the per-deformer invariant of §14.
- `UNSUPPORTED`: none (unobserved options are not parameters).
- Follow-up case suggestions, to be recorded interactively with the solver caches built: Mesh with the cage
  point moved by (0, 100, 0) and by uniform scale, and with Accuracy 1; Surface with the plane tilted 30° and with
  Proportional on; Camera grid point (16, 16) moved and two points moved, camera rotated; Collision with Solver
  Outside and Inside on a cube collider and Distance 0 / 50, Strength 50 %; Jiggle on a plane keyed over 0–5 frames with
  Stiffness 100 %, Drag 0 and 100 %, Gravity −980, one export per frame 1…8 (the lag curve); Delta Mush with Iterations 0 and 1
  and a bent pose (Bend 90 above, Initialize before the bend); Correction with Mapping UV and Normals, Strength 50 %.
