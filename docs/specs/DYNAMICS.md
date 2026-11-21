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

**Restitution combine rule, follow-up O39** (`bounce-combine-0`, `-50`, `drop-cube`: the cube's Bounce stays 5 %,
the plane's is 0, 50, 100 %): the reference's rebound peak after the landing is 25.72472 / 27.28798 /
29.337369 cm (rest 25, so it rises 0.72 / 2.29 / 4.34 cm) and its lowest point 23.816 / 24.178 / 24.539 cm (the
sink shrinks by exactly 0.3614 cm per 50 % of plane bounce). Inverting the ideal rebound `h = (r v)² / (2 g)` with
`v` = the impact speed gives an effective coefficient of about 0.051 / 0.091 / 0.126, roughly `0.05 (1 + 1.5 p)`
(**fitted, not a rule**). Three hypotheses on the engine's four combine rules, measured with the same scene: the
product (ours), the minimum (no rebound for any `p`: peak 24.99), the average (rebounds to 50 and 124 cm for
`p` 50 and 100 %, 0.5 and 1.0 times the impact height scale) and the maximum (114 and 389 cm): none gives a rebound of a
few centimetres, so the reference's contact is not a plain combine of two coefficients and the product stays (it is
the only rule with no visible rebound at 5 %). Pinned: the rebound peaks (`KNOWN_MISMATCH`, test
`bounce_combine_sweep`); asserted: contact frame ±1, rest height 0.05 cm, and the reference's own monotone trend.

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

## 3. Connectors and springs (brief 117)

Sources: O41 `hinge`, `ball-socket`, `spring` (`tests/golden/soft-bodies/`, docs/parity/soft-bodies.md)
and its connector settings table.

### 3.1 Connector (`forge.connector`)

An object (Create menu) with its own transform. Observed: the connector's Type defaults to Hinge
(10 options, of which Hinge, Cardan, Ball and Socket, Ragdoll, Slider, Twist Slider, Planar, Box,
Wheel Suspension, Fixed were read); Object A and Object B are link slots; Ignore Collisions defaults
on. Ours implements four types, as rapier joints:

| id | joint | notes |
|---|---|---|
| `hinge` (default) | revolute about the connector's local **Z** axis | `angular_limit`, `limit_from`, `limit_to` (radians, bank direction: positive is clockwise about Z, as the Bank angle reads; limits are relative to the start pose) |
| `ball_socket` | spherical | |
| `slider` | prismatic along the connector's local **X** axis | |
| `fixed` | fixed (start relative pose kept) | |

Other types (Cardan, Ragdoll, Twist Slider, Planar, Box, Wheel Suspension) are not derived. The
reference's attachment, axis-reference, region of influence, shape conservation, damping, cone,
steering and suspension parameters are not simulated and are not descriptors.

* Anchor: the connector's world position plus `offset` (in the connector's frame). **This is derived
  from the observation**, not assumed: in `hinge` the right cube (start (26, 300, 0), the connector
  at the default position (0, 0, 0)) stays 301 cm from the origin during all 90 frames (the radius
  of every sampled position is 301.0 ± 0.5) and its Bank angle advances as the angle of that
  pendulum; the left cube's centre (−26, 300, 0), where "Center of Mass" attachment suggests the
  pivot, would give a radius of 52 cm. The axis being Z follows from the motion staying in the x-y
  plane with only Bank changing; it is the connector's Z rather than the A object's X ("Reference
  Axis A: X of Object" in the table) because the cubes are unrotated and the observed swing is about
  Z. **Parity-pending** (one scene): which of the connector's axes the reference uses, and what its
  attachment and reference axis options change.
* Ends: an empty slot, or an object that is not a body (no tag), is a fixed point of the world at
  that object's pose (the origin for an empty slot). A connector with both slots empty is skipped.
  A joint between a body and itself is an error.
* `ignore_collisions` (default on) turns contacts between the two linked bodies off.
* `ball_socket` in the reference gives a hinge track bit for bit (observation note: the type value is
  stored but does not reach its solver). Ours is a real spherical joint; in `ball-socket` (a planar
  swing: nothing pushes the cube out of the plane) it gives the same track as the hinge to 1 cm, so
  the case matches, but it is not independent evidence for the type.

### 3.2 Spring (`forge.spring`)

A separate object, as observed ("`spring` is its own object next to the Connector"). Type `linear`
(the only one observed), Object A, Object B, rest length (default 100 cm, observed), stiffness
(default 1.0, observed), damping (default 20 %, observed). A force pair on the two bodies' centres
of mass, applied as an impulse each solver step:

* `F = −(k · (L − L0) + c · v_rel)` along the line between the anchors, equal and opposite on the two
  bodies (a static end does not move; a world end is a fixed point);
* `k = stiffness · 100 s⁻² · m_eff`, `c = 2 · damping · √(k · m_eff)` (damping is the fraction of
  critical damping), `m_eff` the dynamic body's mass (the reduced mass for two dynamic bodies).

The 100 s⁻² per unit of stiffness is **fitted** (the reference does not say what its stiffness 1.0
means): the observed settling height is 9.96 cm below the rest length, and `g / ω² = 9.96 cm` gives
`ω² = 100 s⁻²` for stiffness 1.0 at the fitted gravity; the observed period (first peak at frame 10,
trough at 19) agrees, and the observed amplitude ratio of 0.5 per half period matches a damping
ratio of 0.2 (the log decrement of 0.2 is 0.64 per half period; observed 0.69). Mass-independent
`ω²` is our design (the spring is stiffness per unit mass). **Parity-pending**: one stiffness and one
damping value were observed.

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
| `clones-fall` | clone 4 (centre): free fall frames 0..22 within 1 cm, contact frame ±1, rest y (15.0) 0.05 cm, upright within 1°; clone 0 (corner): free fall frames 0..22 within 1 cm, contact frame ±1; 9 bodies, the cloner itself none; write-back into the clone data | as above | clone 4 lateral drift (`KNOWN_MISMATCH`: the reference ends 1.87 cm from the start in x and z, ours (−2.27, 1.58)); clone 0 end track (`KNOWN_MISMATCH`: the cube's centre is exactly over the plane's corner, the reference tips off and falls to y −2227.8, ours balances on the corner at 15.08) |
| `bounce-combine-0`, `-50` (and `drop-cube` as the 100 % point) | contact frame ±1; rest y 0.05 cm; the reference's peaks 25.72 / 27.29 / 29.34 in order | as above | the rebound peaks (`KNOWN_MISMATCH`: ours stays within 0.05 cm of the rest height, §1.5) |
| `clones-fall-2` | clone 4: free fall 1 cm, contact frame ±1, rest y 0.05 cm, lateral drift from the start below 8 cm in the reference (7.9) and in ours (1.7), ours within 9 cm; clone 0: free fall 1 cm, contact frame ±1 | as above | clone 0 end track (`KNOWN_MISMATCH`: its centre starts 13 and 7 cm inside the plane's corner, so "exactly over the corner" was not the cause; the reference is kicked outwards at the landing, about 4.7 cm per frame in x and z, and falls to y −1986.9; ours stays on the plane); clone 4 drift direction (reference (−7.9, +3.05), ours (+0.7, −1.6)) |
| `collision-noise-0` | clone 4: contact frame ±1, rest y 0.05 cm, drift below 2.5 cm in ours and within 4.5 cm of the reference | as above | the drift itself: the reference still drifts with Collision Noise 0 (0.94, 0.50 cm against 1.87, 1.87 with the default 0.5 %), so the noise is not the whole cause and our (noise-free) drift is (−2.27, 1.58) cm, the other way (`KNOWN_MISMATCH`) |
| `gravity-half` | not observed (no golden data) | | `UNSUPPORTED` |

O41 connector cases (`tests/golden/soft-bodies/`, test `crates/dynamics/tests/joints_parity.rs`):

| case | asserted | tolerance | pinned |
|---|---|---|---|
| `hinge` | x, y of the right cube in every frame 0..90; Bank mod 360; the distance to the pivot | 7 cm (the worst frame differs by 6.6 cm of a 600 cm swing at about 150 cm per frame, a timing error of 0.04 frame: integration scheme); 3°; 301 ± 1.5 cm; z stays 0 within 0.5 cm | none |
| `ball-socket` | the same as `hinge`, and ours hinge = ours ball-and-socket within 1 cm | as `hinge` | none |
| `spring` | y of the dynamic cube in every frame 0..90; settle height; x, z and rotations stay 0; first peak at frame 10 ±1 | 3 cm (the worst frame is 2.6 cm of a 60 cm amplitude: the reference's first frame accelerates 25 % less than ours); 0.5 cm; 0.1 cm; 0.5° | none |
| `soft-sphere-*` | brief 118 (§8) | | |

Rotation tracks are compared modulo 360° (the reference reads B 360 at the impact frame, a
decomposition flip).

## 8. Soft bodies (brief 118)

Sources: O41 `soft-sphere-drop`, `-stiff`, `-pressure` (docs/parity/soft-bodies.md, its soft body
settings table) at level C. The observation has no per-frame samples for these cases: only the
exported meshes at frames 15, 30 and 60 (`mesh.obj`, `mesh-f30.obj`, `mesh-f60.obj`; the plane's 121
points come last) and the sphere origin at frame 60 (`y_end` 34.085, stiff 35.16).

### 8.1 Which objects, which parameters

An object whose enabled rigid body tag is `dynamic = on` and has `soft_body = polygons` ("Made of
Polygons/Lines") is a soft body: it is **not** a rapier body, its mesh points are the particles
(`build::soft_of_scene`, world space, document order). Tag parameters used (observed defaults in
brackets; fractions, 1.0 is 100 %): `keep_shape` (on), `structural` (1.0), `structural_damping`
(0.2), `shear` (0.5), `shear_damping` (0.2), `flexion` (0.5), `flexion_damping` (0.2), `friction`
(0.9), `bounce` (0.05), `linear_damping` (0.05). Not simulated: Pressure (§8.5), the elastic limits,
rest length, volume, stiffness and the other soft body block fields of the reference, Made of Clones.

### 8.2 Solver (`soft.rs`)

Position based dynamics, own code. Unit-mass particles; each solver step (the settings' step
length, §2): gravity and linear damping on the velocity, a predicted position, `ITERATIONS` = 8
rounds of distance constraints and collisions, then the velocity from the position change.
Constraints, all built from the polygons:

* **structural**: every polygon edge;
* **shear**: both diagonals of every quad with four distinct corners;
* **flexion**: for every pair of polygons sharing an edge, the two far corners (the vertex two
  places along the polygon from the edge's start), kept at their start distance.

A constraint of stiffness `s` removes the share `s / (1 + s)` of its error per iteration (0 gives 0,
1 gives one half, 10 gives 0.91): own design, so that Structural 1000 % (observed) is visibly
stiffer than 100 % without being a different solver. **Keep Soft Body Shape** pulls every particle
`KEEP_SHAPE = 0.15` of the way to its shape-matching goal per step (rest shape, rotation from the
iterated polar extraction of Müller et al. 2016). Damping: the velocity along each constraint and
relative to the body's mean velocity decays at `damping · 150 s⁻¹` (`DAMPING_RATE`). The constants
`KEEP_SHAPE`, `DAMPING_RATE` and the stiffness law are **fitted by hand** so that the observed
sphere lands without bouncing and keeps its shape within 10 %; they are not read from the reference
(**parity-pending**). Free fall uses semi-implicit Euler: over `n` steps it falls `(1 + 1/n)` times
the exact distance (2 cm of the 125.6 cm in the first 15 frames).

### 8.3 Collisions and the pinned mismatch

Soft particles collide with every rapier collider (static and dynamic rigid bodies, in their
current pose) one way: a ray from the old to the predicted position finds a surface it crossed (a
zero-thickness plane cannot be tunnelled), a point query finds a solid it sank into; the particle
is put `MARGIN` = 0.5 cm outside. Contact: the normal velocity becomes the restitution
(`bounce`), the tangential velocity is multiplied by `1 − friction`. Soft bodies do not push rigid
bodies.

**Known mismatch (pinned)**: the reference's soft sphere sinks through the plane: the lowest point
is at y −15.4 (frame 30) and −21.2 cm (frame 60) for the default, −15.0 / −15.9 cm for Structural
1000 %, and the whole body keeps falling slowly (centre 35.1 → 33.7 cm). Ours rests on the plane, 0.5
cm above it, with a 7 % sag. We do not reproduce a body that passes through a collider.

### 8.4 Write-back

`DynamicsCache::soft_at(frame)` gives the particles per frame (world space, cm; stepped from the
start frame like everything else: jumping straight to a frame gives the same shape as stepping
there, unlike the reference, which leaves a soft body rigid when a frame is jumped to). The app
(`Evaluated::get`) writes them, in the object's own space, into the hidden `soft_points` list of
the object's tag; `forge_generators::evaluate` replaces the object's points with that list when it
has one entry per point (after the deformers). The object's transform does not move.

### 8.5 Pressure: stored, not simulated

Observed: with Pressure 100 the frames are identical to the default's to the last digit (the value
"is stored but changes nothing visible"). The follow-up `soft-sphere-pressure-2` reads Pressure 100 back
from the tag before simulating (`pressure_readback = 100`; its body table still prints 0) and the three
meshes are again identical to `soft-sphere-drop` (frame 60: y 34.085033), which settles the earlier doubt: the
setting is read back and has no effect in this build. Ours stores `pressure` and does not simulate it, so
that case matches; the area design's pressure constraint is not implemented (nothing to fit).

### 8.6 Parity (O41 soft cases, `crates/dynamics/tests/soft_parity.rs`)

| case | asserted | tolerance | pinned |
|---|---|---|---|
| `soft-sphere-drop` | frame 15: centre height 74.44 and the undeformed 100 cm extents; frames 15, 30, 60: every extent of ours and of the reference within 10 % of 100 cm (reference 99.3 to 104.4 cm); the fall is strictly downward | 5 cm (semi-implicit Euler over-falls 2 cm); 0.5 cm; 10 % | the resting height: the reference sinks to y −21.2 cm, ours rests at +0.5 cm (`KNOWN_MISMATCH`) |
| `soft-sphere-stiff` | stiffer deforms less: in the reference at frames 30 and 60 (0.37 vs 0.65 cm and 0.69 vs 4.37 cm off 100 cm), in ours over the landing (peak squash 18 vs 22 cm of height); extents within 10 % at frame 60 | as above | the sinking: −15.9 cm (reference) vs +0.5 cm |
| `soft-sphere-pressure`, `-2` | frames identical to the default with Pressure 100 (ours: not simulated), and the reference's three meshes are identical to the default case's | exact | none (§8.5: the read-back of the redo settles the doubt) |
| `soft-sphere-samples` | frames 0..15: our centre height within 6 cm of the per-frame samples (the reference falls at about 981 cm/s², our fitted 1000 and semi-implicit Euler add up to 5.8 cm at frame 15) and the lowest point 50 cm under the centre; the reference's lateral drift is 1 to 3.5 cm, ours below 0.5 cm | 6 cm; 0.5 cm | from frame 20 on: the reference's lowest point is 14 to 27 cm under the plane (−26.5 at frame 21, −14.7 at rest, −15.9 at frame 60) and its centre rests at 35.3 ± 0.6 cm; ours rests on the plane, centre 6 to 14 cm higher (`KNOWN_MISMATCH`; no collision offset of the soft particles is derivable: the sink is 15 cm, not a margin we model). Note: the samples' frame 15 centre (77.73) is 3.3 cm higher than the mean of the exported `mesh.obj` points of `soft-sphere-drop` (74.44), so the two exports are not on one time base |

Bounce ("level C: rest shape, bounce") cannot be compared: the reference shows no rebound at the
three recorded frames and its contact is not a contact; ours lands without a bounce at the 5 % tag
value (frame 30 on the plane).
