# 121 · Particles in the clone toolset and volumes

Status: done (parity-pending: own design D20) · Area: I · Depends on: 119, 076, 081 · Observations: – · Touches: dynamics, clone-engine, volume

## Goal
Particles as Tracer input, cloner object-mode source and volume builder point source.

## Inputs
- Area design: [../../roadmap/areas/I-dynamics.md](../../roadmap/areas/I-dynamics.md)

## Design decisions (planner)
- Expose particles as a point source trait used by all three consumers.

## Done when
- [x] Integration test for each consumer.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: the shared point-source interface `forge_particles::PointSource` (`points_at(frame)` ->
  `ParticlePoint { id, position, velocity, age, scale }`, world space, ordered by id) with
  `EmitterPoints` (one cache per consumer call, editor rate). Three consumers:
  Tracer (`crates/generators/src/tracer.rs`), cloner Object mode
  (`crates/generators/src/cloner/object.rs`), Volume Builder (`crates/volume/src/node.rs`,
  `Source::Particles`). Tests: `crates/generators/tests/particles_consumers.rs` (4: Trace Paths,
  Connect All, cloner clones/alignment/space, cloner geometry) and
  `crates/volume/tests/particles_source.rs` (3: sphere radius from the particle range, range 4,
  empty). Existing tracer, cloner and volume suites unchanged and passing.
- Rules (DYNAMICS.md §11, MOGRAPH.md C2 and T1, VOLUME.md §6.2), own design (D20), parity-pending:
  Tracer = one chain per particle id over the sampled frames in which it is alive (appended after
  the other links' chains), Connect All = live particles by id; cloner = one clone per live
  particle, in the cloner's space, +Z along the velocity when Align Clone is on, uniform particle
  scale; volume = sphere of radius `0.735 * particle_voxel_range * voxel_size` (the spline factor
  reused), `points_to_sdf` with `PointUnion::Min`.
- Tests' tolerances: Tracer and cloner numbers are exact (1e-9: emitter at the origin, size 0,
  speed 150, one particle per frame, so particle n stands at z = 5 (f - n)). Volume: the mesh
  extents within 10 cm (one voxel at voxel size 10) of the sphere radius 14.7 / 29.4 and of
  the particle positions 0, 99, 201, 300 (the animated emitter's time is rounded to a
  millisecond by `scene_at`), and a surface point at each sphere's equator within 5 cm.
- Deviations: the Tracer's frame-0 and the cloner/volume frame come from the evaluation context
  (`round(time * fps)`), the editor birthrate always applies (no render flag in `EvalCtx`).
- Files changed: crates/particles/src/source.rs (from 119), crates/generators/{Cargo.toml,
  src/tracer.rs,src/cloner/object.rs}, crates/volume/{Cargo.toml,src/node.rs},
  crates/dynamics/{Cargo.toml,src/lib.rs} (re-export), the two test files, docs/specs
  (DYNAMICS.md §11, MOGRAPH.md, VOLUME.md), BACKLOG, STATUS, PROGRESS.
- Open questions: how the reference tool orients clones on a particle source and sizes a particle
  in a volume (an O40 or Matrix/Tracer capture would settle both).
- The last lines of the gate output: see the final report.
