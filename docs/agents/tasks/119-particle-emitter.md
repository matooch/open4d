# 119 · Particle emitter and particle data

Status: done (parity-pending: O40, own design D20) · Area: I · Depends on: 050 · Observations: O40 (not capturable by script; own design per D20) · Touches: dynamics

## Goal
Emitter object with birth rate, lifetime, speed, variation, seeded.

## Inputs
- Area design: [../../roadmap/areas/I-dynamics.md](../../roadmap/areas/I-dynamics.md)
- O40: [../../parity/particles.md](../../parity/particles.md) and its golden cases
- docs/parity/particles.md (O40)

## Design decisions (planner)
- **Own design (D20), parity-pending; statistical tests.** The reference app's particle random sequence is private, so tests check statistics (count per frame from the birth rate, speed and lifetime distributions within tolerance of the spec's parameters, deterministic for a fixed seed, the cloud's response to each force), not individual particles. If the observer's capture (O40 follow-up) yields data, compare those statistics against it.
- Struct-of-arrays particle data; emission count per frame must match exactly; distributions compared statistically.

## Done when
- [x] O40 emission count exact; distribution moments within tolerance.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: new crate `forge-particles` (`crates/particles`: `emitter.rs`, `particles.rs`, `cache.rs`,
  `force.rs` (brief 120), `source.rs`), re-exported as
  `forge_dynamics::particles`. Emitter node `forge.particle_emitter` (O40's parameter table plus
  shape, size, emitter type, angles, substeps), struct-of-arrays `Particles`, `ParticleCache`
  (frame cache, scrubbing), `particles_at`. Registered in `forge-app`; icons `emitter` (and `force`).
  Tests: `crates/particles/tests/emitter.rs` (17 tests). No golden data exists (O40 captured no
  particle), so parity is statistical: 17/17.
- Rules (docs/specs/DYNAMICS.md §9), all own design (D20), parity-pending (O40): birthrate per
  frame with exact fractional accumulation `floor(R (k+1) + 1e-9) - floor(R k + 1e-9)`; start
  inclusive, stop exclusive; editor rate for viewport/data consumers, renderer rate by
  `ParticleMode::Render`; per-particle SplitMix64 stream from `hash_cell(seed, [n])` (a particle
  does not depend on the rate or other particles); variation `base (1 + v u)`, u uniform [-1, 1);
  angular cone (uniform disc) and pyramid (uniform square) directions; alive while
  `frame - birth < lifetime * fps - 1e-9`.
- Tests and tolerances: counts exact against an integer oracle for rates 10, 2.5, 0.1, 0.7, 33.3, 1;
  alive counts (150 steady state, 145 then 5 then 0 after the stop); seed identity / difference;
  birth independence from the rate; N = 3000 bursts with fixed seed: speed and lifetime mean within
  4 standard errors (std / sqrt N, std = base v / sqrt 3) and std within 4 x std sqrt(0.2 / N);
  cone / pyramid: mean of vx/vz and of its square against E[u^2] = 1/4 or 1/3 with the 4th-moment
  standard errors (derivations in the test comments); birth area second moment (a^2/3 rectangle,
  a^2/4 ellipse) with its standard error; transform places births and directions; scale scales
  the area not the speed; animated emitter trail (0.2 cm for scene_at's millisecond rounding);
  scrub/cache consistency; file round trip.
- Deviations: the module lives in its own crate (`forge-particles`), not inside `forge-dynamics`,
  because the generators (cloner, Tracer) and the volume builder must read particles and
  `forge-dynamics` depends on the generators. Visibility and Relative Speed are stored only.
  No viewport drawing of particles (nothing in the brief asks for it).
- Files changed: crates/particles/**, crates/app/{Cargo.toml,src/lib.rs},
  crates/ui/{icons/emitter.svg,icons/force.svg,src/icon_table.rs}, docs/specs/DYNAMICS.md §9-§11,
  docs/manual/{particles.md,SUMMARY.md}, BACKLOG, STATUS.
- Open questions: O40 follow-up data (an Alembic or Matrix/Tracer capture) would let the
  birth-position, speed and lifetime moments and the default Stop Emission (150) be compared.
- The last lines of the gate output: see the final report.
