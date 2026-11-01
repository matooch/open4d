# D · Deformers and fields

## Scope (v1)

- [x] **Deformer framework:** box, modes, placement, stack order, sibling rule, applying
  after generators. Bend, Twist, Taper, Bulge, Shear and Spherify are done (briefs 01–02).
- [ ] **Bend fix:** `within_box` gating on all three axes, plus the suppression rule
  (3 known mismatches).
- [ ] **Deformers II:** Squash & Stretch, Wrap, FFD, Formula, Melt.
- [ ] **Deformers III:** Explosion, Shatter, Wind, Smoothing, Displacer, Polygon Reduction,
  Bevel deformer, Spline Wrap.
- [ ] **Deformers IV:** Spline, Spline Rail, Mesh (cage), Surface, Camera, Collision,
  Jiggle, Delta Mush, Correction.
- [ ] **Fields:**
  - object types Linear, Spherical, Box, Cylinder, Cone, Torus, Capsule, Radial, Random,
    Formula, Shader, Group;
  - the Remapping tab (strength, invert, inner offset, min/max, clamps, contour modes:
    quadratic, step, quantize, curve);
  - colour and direction channels.
- [ ] **Field lists:**
  - layers with blend modes (Normal, Min, Subtract, Multiply, Overlay, Max, Add, Screen,
    Clip), strength and order;
  - folders and groups;
  - modifier layers (Freeze, Decay, Delay, Remap, Curve, Colorize, Step).
- [ ] **Falloff everywhere:** every deformer and effector takes a field list. An empty
  list means full effect (O24).
- [ ] **Vertex maps from fields;** restriction by vertex map.
- [ ] **Noise library:** the noise types listed in O13 (31 types), seeded, 3D/4D (time),
  shared with shaders (J) and effectors (E).

## Observation jobs

| Job | Form | Status |
|---|---|---|
| bend, twist, taper, bulge, shear, spherify, deformer-common | R20 forms | done |
| O9, O11 §3 | deformers II | done |
| O19 | [deformers-3.md](../../parity/deformers-3.md) | todo |
| O45 | [deformers-4.md](../../parity/deformers-4.md) | todo |
| O7, O11 §1, O13, O23 | fields I–III | done (57 sample files) |
| O24 | [deformer-falloff.md](../../parity/deformer-falloff.md) | done |
| O49 | [fields-4.md](../../parity/fields-4.md): modifier layers, vertex maps | todo |
| O42 | [shaders.md](../../parity/shaders.md): noise sampled on a UV grid, which pins the noise library | todo |

## Implementation design

- **Deformers** stay pure functions on points in `forge-modifiers`, one file each (the
  briefs 01–02 pattern). Deformers that need extra inputs get an `inputs` slot:
  - links: spline (Spline Wrap, Spline, Spline Rail), cage mesh (Mesh), target surface
    (Surface), camera (Camera);
  - colliders (Collision);
  - time (Jiggle, Wind, Formula): through the evaluation context, which carries `time`
    and `fps`.
- **Evaluation context:** `evaluate(scene, id)` gets an `EvalCtx { time, fps, seed }`.
  This changes many signatures, so brief 050 does it once, before the time-dependent
  deformers.
- **Formula parsing:** an expression parser shared by Formula deformer, Formula field,
  Formula effector and Formula spline (`crates/expr`, `forge-expr`):
  - functions sin, cos, …; constants PI;
  - variables x, y, z, t, u, v, id, count;
  - the reference's syntax: case-insensitive function names, `^` for power.
- **Fields:** `crates/fields` (`forge-fields`).
  - `trait Field { fn sample(&self, p: DVec3, ctx: &EvalCtx) -> FieldSample }` with value,
    colour and direction.
  - A `FieldList` holds layers and blending.
  - Sampling is batched (`sample_many`) for performance; rayon is used later (N/180).
  - Parity: level B against `samples.json`, using brief 08's `compare_samples`.
- **Noise:** `crates/noise` (`forge-noise`).
  - Perlin and FBM are implemented first, from public algorithm descriptions.
  - Matching the reference's exact noise values may not be possible. Compare
    statistically (mean, variance, value range, spatial correlation length) and mark exact
    parity pending. **Never** copy constants or tables from the reference app; derive
    everything from the literature.

## Briefs

| # | Title | Depends on | Observations |
|---|---|---|---|
| 050 | Bend 3-axis gating fix + `EvalCtx` (time, fps, seed) through evaluation | – | bend.md |
| 051 | Squash & Stretch, Wrap | 050 | O9, O11 |
| 052 | Expression library `forge-expr`; FFD, Formula, Melt | 050, 063 | O9 |
| 053 | Explosion, Shatter, Wind | 050 | O19 |
| 054 | Smoothing deformer, Displacer, Polygon Reduction, Bevel deformer | 050, 125 (shaders, for the Displacer) | O19 |
| 055 | Spline Wrap, Spline, Spline Rail | 050, 028 | O19, O45 |
| 056 | Mesh (cage), Surface, Camera, Collision, Jiggle, Delta Mush, Correction | 050 | O45 |
| 057 | Fields framework + Linear, Spherical, Box + Remapping | 050 | O7, O11 |
| 058 | Field types II: Cylinder, Cone, Torus, Capsule, Radial, Random | 057, 063 | O13, O23 |
| 059 | Field lists: layers, blend modes, groups, Formula and Shader fields | 057, 052 | O13, O23 |
| 060 | Field modifier layers: Freeze, Decay, Delay, Remap, Curve, Colorize, Step | 059 | O49 |
| 061 | Deformer falloff through field lists (all deformers) | 057 | O24 |
| 062 | Vertex maps from fields; restriction | 027, 059 | O49 |
| 063 | Noise library `forge-noise` | – | O13, O42 |
| 064 | Field list widget (Attribute panel) and field display in the viewport | 059, 150 | G4 |

## Done when
Every deformer and field in scope has parity tests. Every deformer accepts a field list.
S4's falloff and S5's volume fields work.
