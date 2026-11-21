# 125 · Shaders I: Noise, Gradient, Color, Fresnel, Falloff, Layer, Colorizer, Bitmap

Status: done · Area: J · Depends on: 063, 123 · Observations: O42 · Touches: shaders (new)

## Goal
Procedural shaders sampled at UVW, matching O42 samples.

## Inputs
- Area design: [../../roadmap/areas/J-materials.md](../../roadmap/areas/J-materials.md)
- O42: [../../parity/shaders.md](../../parity/shaders.md) and its golden cases
- docs/parity/shaders.md (O42)

## Design decisions (planner)
- New crate `forge-shaders`; `fn sample(&ShadeCtx) -> Color`. Noise types via forge-noise (statistical if the basis differs).
- Parity: UV grid samples (field-kind format, points (u, v, 0)).

## Done when
- [x] O42 cases pass at level B, or statistical with reason.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: crates/shaders (new), 5 parity tests + 11 unit tests. O42: 6 of 17 cases exact in all channel files (gradient-u, -v, -circular, checkerboard, tiles, fresnel); 10 KNOWN_MISMATCH (8 noise cases, gradient-turbulence, colorizer over noise: the reference noise basis is not public; checked statistically); 1 UNSUPPORTED (brick, 126).
- Rules derived: smoothstep ramps, V = 1 - v, circular = sqrt2 * r; checker cells half a period; tiles grout/bevel profile and min() combination; colour params are sRGB decoded (see SHADERS.md S2-S4).
- Deviations: shaders cannot be scene nodes yet (see 123 report); Layer list uses one shared blend mode.
- Files: crates/shaders, docs/specs/SHADERS.md, crates/app (plugin), icon shader.
