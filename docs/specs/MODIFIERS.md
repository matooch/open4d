# Deformers / modifiers
- [ ] Bend, Bulge, Shear, Taper, Twist, Squash & Stretch, Spherify, Wrap, Wind, Melt, Explosion, Explosion FX, Shatter, Formula, Displacer, Smoothing, Jiggle, Delta Mush, FFD, Mesh, Morph, Spline, Spline Rail, Surface, Camera, Collision, Correction, Reversion-type correctives, Polygon Reduction
- Common: Mode (limited/unlimited/within box), Size, Strength, Fit to Parent, Axis, Falloff/Fields, Order of evaluation (parent-child deformer stack), Deformer on generators vs editable mesh
- Each: parameter list, defaults, math, golden mesh comparison.

Bend, Twist, Taper, Bulge, Shear and Spherify are specified below (§0–6, §9). Sources:
docs/parity/{bend,twist,deformer-common,taper,bulge,shear,spherify}.md and their golden
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
  - `within_box`: only points inside the box deform, everything else is unchanged
    (`bend/mode-box`, `twist/mode-box`: the part above the box is not carried along).
  - `unlimited`: the same rate continues below and above the box
    (`bend/mode-unlimited`: min y −299.40 = the arc at t = −0.3; `twist/mode-unlimited`:
    the corner (−100, −200, −100) → (−134.50, −200, −43.70) = a −27° turn).
- Whether `within_box` also gates on x and z is **parity-pending**: the observed objects
  are narrower than the box in x/z except `deformer-common/on-plane`, where points with
  |x|, |z| > 125 deform normally in `limited`.
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

Fit to Parent (a button in the app) and the Falloff/Fields tab are **not implemented**
(task: out of scope; no observation uses them).

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

(`α = 0` → no deformation). The bend happens in the (x, y) plane, z carried through;
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

`α = 0` with `angle ≠ 0` was never observed; a zero strength is treated as "no
deformation" (**parity-pending**).

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

## 9. Parity

Test: `crates/modifiers/tests/deformers_parity.rs` — features `bend` (11 cases),
`twist` (7), `deformer-common` (6), `taper` (8), `bulge` (7), `shear` (7) and
`spherify` (5) — 51 in all — are rebuilt from their `case.toml` (`parent`, `hierarchy`,
`rot` degrees HPB, vec3 `size` are the keys the batch added; `level` maps to
`level_view`) and compared with `export_weld` on both sides (GENERATORS2 §0).

- `UNSUPPORTED`: none.
- `KNOWN_MISMATCH`: none.
- Evaluation order and placement rules are §0; the modifier maths are §1–§6.
- Parity-pending (implemented, seen in one configuration only): sibling deformers below
  the object (§0), `within_box` gating only on y (§0),
  zero strength with a non-zero `angle` (§1), `fillet` together with
  `curvature < 100` (§3, §4, §5 — the one fillet case has curvature 100, so "fillet
  replaces the blend" and "fillet blends into it" cannot be told apart), the curvature
  blend extrapolated *below* the box in `unlimited` (§3–§5 — every unlimited case has
  curvature 100, so only the quadratic term was seen outside the box), and Spherify at
  the origin / `radius` 0 (§6).
