# Dynamics spec (area I)

Briefs: 115 rigid bodies, 116 dynamics on clones, 117 connectors and springs, 118 soft bodies.
Crate: `forge-dynamics` (`crates/dynamics`), on rapier3d (DECISIONS D8; `rapier3d-f64`, features
`dim3 f64 enhanced-determinism std`; the licences are Apache-2.0 and pass `cargo deny`).

Sources (clean room): public rapier documentation for the engine; the parity observation files
[dynamics.md](../parity/dynamics.md) (O39) and [soft-bodies.md](../parity/soft-bodies.md) (O41) with
their golden cases `tests/golden/dynamics/` and `tests/golden/soft-bodies/`; the area design
[I-dynamics.md](../roadmap/areas/I-dynamics.md). Nothing else.

Parity is **level C**: a different physics engine does not reproduce another engine's samples to
1e-4, so each case asserts the quantities that follow from the physics (contact frame, rest height,
settle position, slide distance) at a tolerance written below with its reason. Everything else is
pinned as `KNOWN_MISMATCH` with the measured difference. No reference value is invented: a number
that is fitted to an observation says so.

## 1. Rigid body tag (`forge.rigid_body`)

One tag type is both the body and the collider, as observed (O39: "the collider command puts the
same tag type on the plane"); the **Dynamic** switch decides. The tag also carries the soft body
switch (§8). Parameters (descriptors in `nodes.rs`; percent values are stored as fractions):

| id | kind | default | note |
|---|---|---|---|
| `enabled` | bool | true | O39 Enabled |
| `dynamic` | enum `off` `on` `ghost` `kinematic` | `on` | O39 Off/On/Ghost; `kinematic` is ours (§1.3) |
| `custom_initial_velocity` | bool | false | O39 |
| `initial_linear_velocity` | vec3, cm/s | 0 | O39 |
| `initial_angular_velocity` | vec3, rad/s (shown as deg/s) | 0 | O39 |
| `shape` | enum `automatic` `box` `sphere` `convex_hull` `static_mesh` | `automatic` | O39 Shape (12 options, labels not read: only these five are derived) |
| `size_increment` | float, cm | 0 | O39; added to the half extents of box and sphere shapes |
| `bounce` | float 0..1 | 0.05 | O39 Bounce 5 %; the descriptor stops at 100 % (§1.5) |
| `friction` | float ≥ 0 | 0.9 | O39 Friction 90 % |
| `mass_mode` | enum `world_density` `custom_density` `custom_mass` | `world_density` | O39 Use Mass |
| `density` | float | 1 | g/cm³ (O39 Density 1) |
| `mass` | float | 10 | kg (O39 Mass 10) |
| `linear_damping` | float | 0.05 | O39 5 %, used as rapier's per-second factor |
| `angular_damping` | float | 0.05 | O39 5 % |
| `soft_body`, `keep_shape`, `structural`, `structural_damping`, `shear`, `shear_damping`, `flexion`, `flexion_damping`, `pressure` | | | §8 |

### 1.1 Which objects are bodies

Every object with an enabled tag of this type and a mesh (the caller supplies the evaluated mesh;
`bodies_of_scene`), in document order (depth first). The order fixes the rapier handles and so the
result. `dynamic = off`: a fixed collider. `on`: a dynamic body. A disabled tag, `ghost`, and an
object without geometry take no part. Parameters of the observed tag that are **not** simulated and
so are not descriptors (listed so a later brief can add them): Trigger and its velocity threshold,
Dynamic Transition, Inherit Tag / Individual Elements, Self Collisions, Use Deformed Object, Use
Collision Margin and Margin, Collision Noise, Rotational Mass, Center of Mass, Follow Position and
Rotation, Force Mode, Drag, Lift, Two-Sided, Use Accurate Solver. Moving Mesh shapes (the other
options of Shape) are not derived.

### 1.2 Pose and units

The scene is in cm, rapier in metres; the factor `0.01` is applied in `sim.rs` and nowhere else.
The body origin is the object's origin; the collider is built from the mesh points scaled by the
world scale. The world matrix at the start frame (`scene.settings().start`) gives translation and
rotation; the scale only sizes the shape and is kept in the output matrix.

### 1.3 Shapes

* `automatic`: a fixed body (`dynamic = off`) gets the triangulated mesh (triangle mesh); a dynamic
  body gets the convex hull of its points (a box if the hull degenerates). On a `forge.sphere`
  primitive, `automatic` is an exact ball (radius: mean half extent): rolling on a polyhedron slows
  a sphere visibly (measured, `slope-sphere`: a 16-segment hull reaches x 513 cm at frame 90, the
  ball 609 cm, the reference 669 cm).
* `box`: the bounding box plus `size_increment`. `sphere`: the mean half extent plus the increment.
  `convex_hull`: as automatic for a dynamic body. `static_mesh`: the triangle mesh.
* `kinematic` bodies follow the pose passed to `step_frame_with`; they push dynamic bodies and are
  not moved by them. (Ours: the observation has no kinematic case.)

### 1.4 Mass, damping, initial velocity

`world_density` is density 1 g/cm³ times the collider volume; `custom_density` uses `density`;
`custom_mass` sets the mass in kg. Linear and angular damping are rapier's `linear_damping` /
`angular_damping`. The initial velocities apply to dynamic bodies only and only when
`custom_initial_velocity` is on.

### 1.5 Contact material

Rapier combines the two colliders' coefficients. **Restitution** uses the product (body bounce ×
the plane's 100 %). **Friction** uses the maximum of the two. Derivation (three hypotheses
budgeted, `drop-friction-slide`): with the product (0.9) the cube tips over while sliding (slide
60.2 cm against the observed 45.03); with the maximum (1.0) it slides flat and stops after 41.5 cm
(−7.8 %), and the observed 45.03 cm equals `v²/(2 g μ)` for μ = 1 at the fitted g. **Parity-pending**
(one case). Bounce above 100 % is not supported: the descriptor range ends at 1.

## 2. Scene settings (`forge.dynamics_settings`)

A settings object (Create menu; any place in the tree). The first one in document order is used;
without one the defaults apply. The reference's scene-level settings could not be read by script
(O39 Settings table), so the defaults are **fitted** to the observed free fall of `drop-cube`
(frames 0 to 22, no contact), not read:

| id | default | note |
|---|---|---|
| `enabled` | true | false: no simulation (bodies keep the start pose) |
| `gravity` | 1000 cm/s², downward | fitted, §2.1 |
| `time_scale` | 1 | the solver step is `time_scale / (fps · steps)` |
| `steps_per_frame` | 2 | fitted, §2.1 |

### 2.1 Fit

Grid search over gravity 900..1290 cm/s² (step 10), steps per frame 1/2/4/8, linear damping 0 or
5 %: the smallest maximum height error over frames 0 to 22 against `drop-cube` is 0.74 cm
(gravity 1000, 2 steps, damping 5 %); 981 cm/s² gives 3.4 to 6.6 cm. The free fall is not
reproduced to the 1e-3 of the file: no choice of the three parameters does it, so the free fall is
asserted at 1 cm (§7). `gravity-half` was not observed, so halving gravity is only tested for
consistency with free fall (contact frame ∝ 1/√g), not for parity.

## 3. Connectors and springs

(Brief 117; filled in with that brief.)

## 4. Simulation

`Simulation::new(settings, fps, specs)` builds a rapier `PhysicsWorld` (bodies and colliders in the
order of `specs`; `IntegrationParameters::dt = step_dt`, `length_unit` 1 m). `step_frame` runs
`steps_per_frame` solver steps. Determinism: `enhanced-determinism`, no threads, fixed order, no
random numbers (the tag's Collision Noise is not simulated). Two builds of one scene give bit-equal
frames (tested).

## 5. Cache and scrubbing (`DynamicsCache`)

The cache holds one world matrix per body per frame, from the start frame. The live world sits at
the last cached frame: asking for a later frame steps forward frame by frame (the observed rule that
a simulation only runs when stepped in order), asking for an earlier one reads the cache. The cache
is rebuilt, from the start frame, when the scene changes, so the same scene always gives the same
frames however the timeline was scrubbed. (Rapier worlds are not `Clone`, so there are no mid-run
restarts; the design's "nearest cached frame" is the last cached frame.) `at(frame)` for a frame
before the start returns the start pose.

### 5.1 In the app

`forge-app` keeps one `DynamicsCache` per scene revision (`crates/app/src/dynamics.rs`, used by
`Evaluated::get`): the scene animated to a frame gets the simulated position and rotation of every
**dynamic** body written into its local transform (`apply::overrides`; the parent's world matrix is
divided out, scale is untouched). Static and kinematic bodies and objects with an animated pose
keep their evaluated transform. Kinematic bodies are not driven by the timeline in the app yet
(`Simulation::step_frame_with` takes the targets; wiring the animated pose is brief 122's bake
work). Editing the scene bumps the revision and so restarts the simulation.

## 6. Dynamics on clones

A cloner that carries an enabled rigid body tag does not become one body: **each visible clone is
a body** (`clones::clone_bodies`), keyed `BodyKey::Clone { cloner, index }` with the clone's index
in the cloner's clone data, in cloner order then clone order. Hidden clones take no part and keep
their generated matrix. The tag's kind, shape, material and velocities apply to every clone.

* Start pose: cloner world matrix × clone matrix (clone data from `cloner_instances`, effectors
  included). Shape: the clone's prototype mesh (child transform baked unless Reset Coordinates),
  scaled by the clone matrix's scale.
* Write back: the app (`Evaluated::get`) writes the simulated poses, in cloner space, into the
  cloner's hidden parameters `dynamics_pos` and `dynamics_rot` (lists of vec3, one entry per clone;
  rotation H, P, B in radians). `CloneData::apply_poses` reads them when both have one entry per
  clone: it replaces each clone's translation and rotation and keeps its scale, so the expanded
  geometry, the instancing display and effectors that read clone data all see the fallen clones.
  Clones that are not simulated (hidden, or a static tag) keep the generated pose in the lists.
  The lists are never saved by a simulation: the evaluation writes them into the evaluated copy
  of the scene only.
* The cloner object itself keeps its transform (observed: global y 300.000 at frames 0 and 90).
* The cache is keyed by clone index; a change of the clone count (any edit) rebuilds it (§5).
* Ours, not observed: effectors still apply to the generated start pose; per-clone forces and
  clones that appear later (Blend/Time effectors) are not simulated.

## 7. Parity: rigid bodies (brief 115)

Test: `crates/dynamics/tests/rigid_parity.rs`. Scenes are rebuilt from the case descriptions (the
case files hold the tag tables, not positions); `compare_trajectory`
(`forge_parity_harness`, `docs/specs/INFRA.md` level C) checks derived quantities with
per-case `Thresholds`:

* contact frame: first frame whose per-frame step is larger than the step before (a fall
  accelerates, a contact stops it), allowed difference **±1 frame** (the area design; the fall is
  fitted to 0.74 cm);
* rest position: **0.05 cm** (reason: contact softness; both engines rest 0.002 to 0.015 cm inside
  the plane: observed 25.002, ours 24.99);
* rebound ratio: ±10 % (area design);
* slide distance: ±10 % (area design).

| case | asserted | tolerance | pinned |
|---|---|---|---|
| `drop-cube` | free fall frames 0..22 within 1 cm; contact frame; rest y; at rest (|Δy| < 0.05 between frames 35 and 90); rotation at frame 90 within 1° of the start | as above | rebound ratio (`KNOWN_MISMATCH`: observed rise 4.34 cm for bounce 5 %; the engine's 5 % restitution gives no visible rebound; the observation does not say how it combines the coefficients) |
| `drop-bounce` | contact frame | ±1 | the rest (`KNOWN_MISMATCH`: bounce 1e+18 gains energy in the reference; our bounce stops at 100 %) |
| `drop-friction-slide` | slide distance; rest y | ±10 %; 0.05 cm | none |
| `drop-friction-high` | – | – | whole case (`KNOWN_MISMATCH`: friction 1e+18 slides 66.1 cm, further than 90 %; ours tumbles at 56.6 cm, −14 %; the observation says only that it is not adjusted) |
| `stack` | every cube stays within 0.25 cm vertically and 0.3 cm sideways of its start and within 1° of upright | reference drift ≤ 0.045 / 0.12 cm; ours is larger because our contacts are softer (0.17 cm sink) | – |
| `slope-sphere` | x at frame 30; final x and final y | ±10 % | positions per frame (the rolling friction model differs) |
| `tumble` | contact frame; rest y; rests flat (P and B each within 1° of a multiple of 90°) | ±1; 0.05 cm | rest x, z and heading H (`KNOWN_MISMATCH`: which face it lands on and how far it travels depend on the engine's contact order) |
| `clones-fall` | clone 4 (centre): free fall frames 0..22 within 1 cm, contact frame ±1, rest y (15.0) 0.05 cm, upright within 1°; clone 0 (corner): free fall frames 0..22 within 1 cm, contact frame ±1; 9 bodies, the cloner itself none; write-back into the clone data | as above | clone 4 lateral drift (`KNOWN_MISMATCH`: the reference ends 1.87 cm from the start in x and z, ours 0); clone 0 end track (`KNOWN_MISMATCH`: the cube's centre is exactly over the plane's corner, the reference tips off and falls to y −2227.8, ours balances on the corner at 15.08) |
| `gravity-half` | not observed (no golden data) | | `UNSUPPORTED` |

Rotation tracks are compared modulo 360° (the reference reads B 360 at the impact frame, a
decomposition flip).
