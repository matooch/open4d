# I · Dynamics and particles

## Scope (v1)

- [ ] **Rigid body tag:** dynamic, static (collider), kinematic.
  - Shapes: automatic, box, sphere, convex hull, moving mesh, static mesh.
  - Mass, density, bounce, friction, linear and angular damping.
  - Initial velocity; follow position/rotation.
- [ ] **Scene settings:** gravity, time scale, steps per frame, enabled; a cache to bake,
  clear and play back.
- [ ] **Dynamics on clones:** each clone is a body; per-clone forces.
- [ ] **Connectors:** hinge, ball-socket, slider, fixed, spring. **Forces:** Gravity,
  Wind, Attractor, Deflector, Friction, Turbulence, Rotation.
- [ ] **Soft bodies:** position-based dynamics, with stiffness, structural and shear
  springs, pressure, and a rest-shape strength.
- [ ] **Particle emitter:** rate, lifetime, speed, variation, emitter shapes, seed. Emitted
  objects as children (instancing), plus particle data for the clone toolset (Tracer,
  Volume points).
- [ ] **v2:** cloth, fluids, the full particle-system node system.

## Observation jobs

| Job | Form | Status |
|---|---|---|
| O39 | [dynamics.md](../../parity/dynamics.md): cube dropped on a plane (bounce, friction); stack of 3 boxes; sphere rolling down a slope; clones falling; per-frame positions and rotations as samples | todo |
| O40 | [particles.md](../../parity/particles.md): emitter defaults; seeded emission; per-frame particle positions for 30 frames under gravity/wind/attractor | todo |
| O41 | [soft-bodies.md](../../parity/soft-bodies.md): soft sphere dropped on a plane; hinge and spring connectors | todo |

## Implementation design

- **Crate:** `crates/dynamics` (`forge-dynamics`), wrapping **rapier3d** (DECISIONS D8).
  - Fixed time step = 1/(fps × steps). The simulation state is cached per frame (a
    `Vec<(NodeId, Isometry)>` per frame).
  - Scrubbing back re-simulates from the nearest cached frame.
  - Bake writes keys (G/093) or a cache tag.
- **Parity: level C.** Physics engines differ, so we compare:
  - resting positions within a tolerance;
  - first-contact frame ±1;
  - bounce height ratio ±10 %;
  - sliding distance ±10 %.

  The harness gets `compare_trajectory` with these per-case thresholds in `case.toml`'s
  `[tolerance]`.
- **Soft bodies:** our own PBD solver on the mesh's edges plus bending constraints. Level C.
- **Particles:** `forge-dynamics::particles`.
  - Emission is deterministic per seed; particle data lives in a struct-of-arrays.
  - The reference's random sequence can't be reproduced, so compare statistically:
    emission count per frame exactly, spatial distribution moments, and speed
    distribution.
  - Forces are pure functions on positions and velocities.

## Briefs

| # | Title | Depends on | Observations |
|---|---|---|---|
| 115 | rapier integration: rigid body tag, shapes, scene settings, per-frame cache, scrubbing; `compare_trajectory` | 050, 010 | O39 |
| 116 | Dynamics on clones | 115, 065 | O39 |
| 117 | Connectors and springs | 115 | O41 |
| 118 | Soft bodies (PBD) | 115 | O41 |
| 119 | Particle emitter and particle data | 050 | O40 |
| 120 | Forces: gravity, wind, attractor, deflector, friction, turbulence, rotation | 119, 115, 063 | O40 |
| 121 | Particles in the clone toolset and volumes (Tracer, builder point sources) | 119, 076, 081 | – |
| 122 | Bake to keys and cache tags; dynamics UI (scene settings panel) | 115, 093 | – |

## Done when
Falling, stacking, rolling and particle cases meet the level-C thresholds. Simulations are
deterministic run to run on all three OS.
