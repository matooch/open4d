# Particles

The **Emitter** (`forge.particle_emitter`) sends particles out along its own +Z axis from a
rectangle or ellipse in its XY plane. **Forces** act on them: Gravity, Wind, Attractor,
Deflector, Friction, Turbulence and Rotation (`forge.force_*`). The rules, with the formulas and
the tests that check them, are in [DYNAMICS.md](../specs/DYNAMICS.md) sections 9 to 11. The
design is our own: the reference tool's particle sequence is private, so nothing here is claimed
to match it particle by particle, and the tests compare statistics.

## The emitter

`Birthrate Editor` and `Birthrate Renderer` are particles **per frame** and may be fractional:
a rate of 2.5 gives 2, 3, 2, 3 particles in successive frames, and after `m` frames exactly
`floor(2.5 m)` have been born. The editor rate is used in the viewport and by everything that
reads particles as data (Tracer, Cloner, Volume Builder); the renderer rate by the renderer.
Emission runs from `Start Emission` up to, but not including, `Stop Emission`.

Every particle has a lifetime (seconds), a speed (cm/s), a spin rate and an end scale. Each
`Variation` is a percentage: the value is drawn uniformly between `base * (1 - v)` and
`base * (1 + v)`. `Seed` changes the draws; the same seed always gives the same particles, and a
particle's draws do not depend on the rate or on what happened in other frames. The cloud is
simulated frame by frame from the start frame and cached, so scrubbing back shows exactly the
frames that were simulated going forward.

`Emitter Type` Angular spreads the directions over an elliptical cone, Pyramid over a
rectangular pyramid, with the full `Horizontal Angle` and `Vertical Angle`; angles of 0 send
every particle straight along +Z. Moving, rotating or scaling the emitter (animated or not)
moves the birth points and directions; scaling changes the birth area, not the speed.

## Forces

Each force is an object. By default it acts on **every emitter in the scene**; list emitters in
its `Emitters` field to restrict it. Add field objects to its `Fields` list to limit it to a
region: the force is multiplied by the list's value at the particle. Rates and strengths:

| Force | Does |
|---|---|
| Gravity | constant acceleration along the object's -Y axis (981 cm/s^2) |
| Wind | pulls the velocity toward `Speed` along the object's +Z axis with `Air Resistance` |
| Attractor | accelerates toward the object (negative strength repels), constant inside `Radius`, inverse square beyond |
| Friction | slows particles exponentially |
| Turbulence | smooth noise acceleration, zero on average; `Seed`, `Scale`, `Speed` |
| Rotation | swirls particles around the object's Y axis |
| Deflector | a plane (normal +Y) particles bounce off with `Bounce` and `Friction`, or die with `Kill` |

Particles are integrated in `Substeps` semi-implicit Euler steps per frame (default 4).

## Particles as points

An emitter can be used wherever a list of points is wanted:

- **Tracer**: put the emitter in the Tracer's links; Trace Paths draws one path per particle.
- **Cloner, Object mode**: use the emitter as the Object; every live particle becomes a clone,
  turned along its velocity when Align Clone is on.
- **Volume Builder**: make the emitter a child; each live particle becomes a sphere of radius
  `0.735 * Particle Voxel Range * Voxel Size`.
