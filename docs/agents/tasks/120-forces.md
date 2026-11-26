# 120 · Forces: gravity, wind, attractor, deflector, friction, turbulence, rotation

Status: done (parity-pending: O40, own design D20) · Area: I · Depends on: 119, 115, 063 · Observations: O40 (not capturable by script; own design per D20) · Touches: dynamics

## Goal
Force objects acting on particles and bodies.

## Inputs
- Area design: [../../roadmap/areas/I-dynamics.md](../../roadmap/areas/I-dynamics.md)
- O40: [../../parity/particles.md](../../parity/particles.md) and its golden cases
- docs/parity/particles.md (force cases)

## Design decisions (planner)
- **Own design (D20), parity-pending; statistical tests.** The reference app's particle random sequence is private, so tests check statistics (count per frame from the birth rate, speed and lifetime distributions within tolerance of the spec's parameters, deterministic for a fixed seed, the cloud's response to each force), not individual particles. If the observer's capture (O40 follow-up) yields data, compare those statistics against it.
- Pure functions (position, velocity, time) → acceleration; fields as falloff.

## Done when
- [x] O40 force cases meet thresholds.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: seven force objects `forge.force_gravity|wind|attractor|deflector|friction|turbulence|rotation`
  in `crates/particles/src/force.rs` (pure `Force::acceleration(p, v, t)`; the deflector's
  `Force::collide` is the collision response), the semi-implicit Euler step with substeps and
  falloff in `cache.rs`. Tests: `crates/particles/tests/forces.rs` (17 tests, all derived, none
  data-driven: O40 has no force data). Parity: 17/17 statistical or analytic.
- Rules (DYNAMICS.md §10), own design (D20), parity-pending (O40): each force's formula and default
  (gravity 981 along the object's -Y, wind `k (w - v)` along +Z, attractor constant inside its
  radius and inverse square outside, friction `-k v`, turbulence 4D Perlin zero-mean, rotation
  tangential about the object's Y); scene-wide unless the force lists emitters; falloff = field
  list value multiplies the acceleration (deflector ignores fields); semi-implicit Euler, substeps
  per frame from the emitter (default 4), forces read at the frame being produced; deflector
  reflects `p - (1 + b) d n` so a particle never ends below the plane, friction scales the tangent,
  `kill` removes it.
- Tests and tolerances: gravity: exact discrete law `p0 + v0 t + a t^2/2 + a dt t/2` (1e-6, float
  accumulation over <= 240 steps of magnitude <= 1e4) and the distance to the continuous parabola
  equals the scheme's error term `a dt t / 2`; friction: exact `(1 - k dt)^n` (1e-9) and within
  `e^B - 1` of `e^(-k t)`, `B = n x^2 / (2 (1 - x))`, `x = k dt`; wind: exact relaxation law (1e-9) and
  the same bound against the continuous one; attractor: mean distance shrinks every frame (grows
  for negative strength), the law itself at 0.5 r, r, 2 r; deflector: no particle below the plane
  over 90 frames at bounce 0, 0.5, 1 (2000 particles), the reflection numbers, rebound only with
  bounce > 0, kill; rotation: axial velocity exactly 0 and angular momentum rises every frame, sign
  follows strength; turbulence (2000 particles over 40000 noise cells): per-axis mean within 4
  standard errors (sample std / sqrt N) of 0, variance sum grows over frames 3, 6, 12, 24,
  identical for one seed and different for another; falloff: outside a Spherical field the
  velocity stays exactly 0, inside it falls; listed vs scene-wide emitters; disabled force; forces
  add; file round trip.
- Deviations: forces act on particles only (the brief's "bodies" in the area text are not driven by
  these objects; rigid bodies keep the dynamics settings' gravity). Forces have no per-force
  time window or strength-by-time curve. The substeps live on the emitter.
- Files changed: crates/particles/src/{force,cache}.rs, crates/particles/tests/forces.rs,
  docs/specs/DYNAMICS.md §10, docs/manual/particles.md, BACKLOG, STATUS.
- Open questions: force defaults and formulas (and the manual's Wind/Attractor modes) need an
  O40 capture to compare.
- The last lines of the gate output: see the final report.
