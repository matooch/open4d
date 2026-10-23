# Deformers / modifiers
- [ ] Bend, Bulge, Shear, Taper, Twist, Squash & Stretch, Spherify, Wrap, Wind, Melt, Explosion, Explosion FX, Shatter, Formula, Displacer, Smoothing, Jiggle, Delta Mush, FFD, Mesh, Morph, Spline, Spline Rail, Surface, Camera, Collision, Correction, Reversion-type correctives, Polygon Reduction
- Common: Mode (limited/unlimited/within box), Size, Strength, Fit to Parent, Axis, Falloff/Fields, Order of evaluation (parent-child deformer stack), Deformer on generators vs editable mesh
- Each: parameter list, defaults, math, golden mesh comparison.

Bend and Twist are specified below (§0–2, §9). Sources: docs/parity/{bend,twist,
deformer-common}.md and their golden cases. Everything else is still a checklist item
(task 02 covers Taper, Bulge, Shear, Spherify).

Status: draft · Source: docs/parity/{bend,twist,deformer-common}.md

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
    (359.15, 259.15, 100)).
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

Every case has `size_x = size_z`, so whether the term is `size_x/2`, `size_z/2` or
`(size_x + size_z)/4` cannot be told apart from the data: **parity-pending**, `size_x/2`
is used. It does not depend on `angle`: `bend/angle` matches with the same `R`.

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

## 9. Parity

Test: `crates/modifiers/tests/deformers_parity.rs` — features `bend` (10 cases),
`twist` (7) and `deformer-common` (6) are rebuilt from their `case.toml` (`parent`,
`hierarchy`, `rot` degrees HPB, vec3 `size` are the new keys; `level` maps to
`level_view`) and compared with `export_weld` on both sides (GENERATORS2 §0).

- `UNSUPPORTED`: none.
- `KNOWN_MISMATCH`: none.
- Evaluation order and placement rules are §0; the modifier maths are §1–§2.
- Parity-pending (implemented, seen in one configuration only): the `size_x/2` term of
  `R` (§1), sibling deformers below the object (§0), `within_box` gating only on y (§0),
  zero strength with a non-zero `angle` (§1).
