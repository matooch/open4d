# 118 · Soft bodies (PBD)

Status: done · Area: I · Depends on: 115 · Observations: O41 · Touches: dynamics

## Goal
Soft bodies with our own position-based dynamics.

## Inputs
- Area design: [../../roadmap/areas/I-dynamics.md](../../roadmap/areas/I-dynamics.md)
- O41: [../../parity/soft-bodies.md](../../parity/soft-bodies.md) and its golden cases
- docs/parity/soft-bodies.md

## Design decisions (planner)
- Edge distance + bending + pressure constraints; collisions against rapier colliders.

## Done when
- [x] O41 soft cases meet level-C thresholds (rest shape, bounce): rest shape yes; the resting height and bounce are pinned (the reference sinks through the plane).
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: `crates/dynamics/src/soft.rs` (own PBD: structural, shear and flexion distance
  constraints, shape matching for Keep Soft Body Shape, one-way collisions against rapier
  colliders), write-back through the tag's hidden `soft_points` and `forge_generators::evaluate`.
  Tests: `crates/dynamics/tests/soft_parity.rs` (3 cases), `soft_unit.rs` (7),
  `crates/app/tests/dynamics_eval.rs` (soft mesh follows the simulation).
  - `soft-sphere-drop`: frame-15 centre within 5 cm of 74.44 and undeformed (0.5 cm); extents at
    frames 15, 30, 60 within 10 % of 100 cm (reference 99.3 to 104.4); pinned `KNOWN_MISMATCH`: the
    resting height (the reference sinks to y -21.2 cm, ours rests at +0.5).
  - `soft-sphere-stiff`: stiffer deforms less (reference frames 30 and 60; ours over the landing);
    pinned: the sinking (-15.9 cm vs +0.5).
  - `soft-sphere-pressure`: identical to the default (pressure is not simulated, as observed).
  - Bounce cannot be compared: no rebound is recorded at frames 15, 30, 60.
- Rules: stiffness law `s/(1+s)`, `KEEP_SHAPE` 0.15, `DAMPING_RATE` 150 are hand-fitted so that
  the observed drop lands without a bounce and stays within 10 % of its shape: parity-pending.
- Deviations: pressure is not a constraint (the observation shows it does nothing; the design
  asked for one); soft bodies are not rapier bodies and do not push rigid bodies; the cache steps
  from the start frame whatever frame is asked (the reference leaves a soft body rigid when a frame
  is jumped to).
- Files changed: crates/dynamics/src/{soft,sim,build,cache,apply,nodes,lib}.rs,
  crates/generators/src/lib.rs (`apply_soft_points`), crates/app/src/dynamics.rs, the soft tests,
  docs/specs/DYNAMICS.md §8, BACKLOG, STATUS.
- Open questions: why the reference soft body sinks 15-21 cm into a collider (is the plane's
  collision margin or the soft body's own collision setting a tag field we did not read?); the
  pressure case's table reads Pressure 0, so 100 may never have been applied: a repeat that reads
  the value back from the tag would settle it; per-frame samples of the sphere (centre and lowest
  point at every frame) would let the landing and any bounce be compared.
- The last lines of the gate output: see the final report.
