# 063 · Noise library `forge-noise`

Status: ready (planner decision: implement now from O13 data and the public noise literature; shader-noise parity stays pending until O42) · Area: D · Depends on: – · Observations: O13, O42 · Touches: noise (new)

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
- [ ] NOISE.md spec; statistical parity tests for every O42 noise case once it is observed (until then, unit tests only).
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: tests added; parity N/M (`UNSUPPORTED` / `KNOWN_MISMATCH` with reasons).
- Rules derived, with the case that proves each; parity-pending rules.
- Deviations from the design decisions, and why.
- Files changed.
- Open questions or data that would settle them (the reviewer turns these into follow-up forms).
- The last lines of the gate output.
