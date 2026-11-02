# 063 · Noise library `forge-noise`

Status: done (parity-pending O42) · Area: D · Depends on: – · Observations: O13, O42 · Touches: noise (new)

## Goal
A seeded 3D/4D noise library with the reference's noise types. Exact where derivable, statistically close otherwise.

## Inputs
- Area design: [../../roadmap/areas/D-deformers-fields.md](../../roadmap/areas/D-deformers-fields.md)
- O13: [../../parity/fields-2.md](../../parity/fields-2.md) and its golden cases
- O42: [../../parity/shaders.md](../../parity/shaders.md) and its golden cases
- docs/parity/fields-2.md (31 noise types listed)
- docs/parity/shaders.md (O42: noise sampled on a UV grid)

## Design decisions (planner)
- Implement from public literature only: Perlin (improved), FBM, Turbulence, Ridged Multifractal, Voronoi/Worley F1/F2/F2−F1, Cell, Box, Wavy Turbulence, Electric, Gaseous, and the rest as approximations named after the types. **Never** copy tables or constants from the reference app.
- Each type has octaves, lacunarity, gain, scale, seed and time.
- Parity: compare the O42 noise samples statistically (mean, std, range, spatial autocorrelation at lag 1). Mark exact matching parity-pending. A Perlin case may match exactly if the reference uses the classic permutation; try it.

## Done when
- [x] [NOISE.md](../../specs/NOISE.md) spec; statistical parity tests for every O42 noise case once it is observed (until then, unit tests only).
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: new crate `crates/noise` (`forge-noise`), spec `docs/specs/NOISE.md`. 14 unit tests (`tests/noise.rs`: determinism, pinned values, range, continuity, moments, seed independence, scale, time axis, lattice, warp, octaves) and 2 statistical tests (`tests/o13_statistics.rs`). No O42 golden exists yet, so no parity count: exact parity is pending O42.
- Rules derived: improved Perlin, value, simplex (3D/4D), Worley (F1/F2/id), fBm, turbulence, ridged multifractal, domain warp, plus 10 composite kinds from the O13 list defined by our own formulas (NOISE.md N4). All from public literature; SplitMix64 hash. Statistical comparison with the O13 Random field samples (random-defaults, random-seed) and the shader-noise sample: reference mean about 0.5, std 0.12 to 0.15, lag-1 correlation about 0 at step 75; our Perlin at scale 50-100 matches (NOISE.md N7). Parity-pending: all exact values, seed mapping, feature size, unit mapping, remaining invented-name types (Buya, Cranal, Dents, Hama, Luka, Mod. Noise, Naki, Nutous, Ober, Pezo, Poxo, Sema, Stupl, VL Noise, Zada).
- Deviations: Billow dropped (identical to Turbulence); `Box` and `Cell` are our own definitions; Worley F2 can rarely be inexact (3^n search).
- Files changed: Cargo.lock, crates/noise/**, docs/specs/NOISE.md, this brief, docs/roadmap/BACKLOG.md.
- Open questions: O42 per-type samples (several scales and seeds) to fix exact Perlin/FBM matching and the unit mapping; feature size of the Random field.
- Gate (last lines): `GATE: 814 tests passed, 0 failed`; python `OK` (38 tests); `cleanroom: ok`; `ok: 471 cases`; `GATE: all passed`.
