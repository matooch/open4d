# 207 · Shader nodes to WGSL and the GPU comparison over all nodes (part 2 of 197)

Status: done · Area: R · Depends on: 197, 125, 126 · Observations: – (own design; shader values from O42 through the CPU shaders) · Touches: shaders, material

## Goal
Port every shader-node family of briefs 125/126 to WGSL so node materials look the same on the GPU as in the CPU path tracer: noise (all 17 kinds), gradients, tiles, layers, colorizer, fusion, filter, normalizer, vertex map, AO, bitmaps. Replaces part 1's constant fallbacks.

## Inputs
- Brief [197](197-shading-ir-wgsl.md) (codegen, cache, harness) and the spec section it wrote
- docs/specs/SHADERS.md (the CPU rules, the source of truth), O42 golden cases in tests/golden/shaders*
- Area design: [../../roadmap/areas/R-realtime.md](../../roadmap/areas/R-realtime.md)

## Design decisions (planner)
- One WGSL function per shader family, shared by every material that uses it (emitted once per module).
- Noise must be bit-compatible with the CPU tables/hash (same permutation and seed rules) so the comparison holds; where f32 cannot reach 1e-4 (e.g. high-octave noise), document the measured error and use the tightest bound that holds, stated per family.
- Bitmaps sample through a texture binding with the same filtering rule as the CPU (spec it).

## Done when
- [x] Every shader node from 125/126 has a WGSL equivalent; any remaining fallback is documented with the reason.
- [x] The 197 harness covers every family on a grid (CPU vs GPU, skipped without an adapter) within 1e-4, or the documented per-family bound.
- [x] Recompiling a 50-node material takes under 100 ms (measured in a test, not a hard CI limit).
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md` if parity results change, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: every `Shader` variant of 125/126 now compiles to WGSL; nothing falls back. `forge_material::wgsl` gained the `TextureCodegen` hook (`compile_with`, `ShaderCache::module_with`, `ModuleBuilder` with once-per-module library functions and bound resources) and `ShadeCtx` carries `vertex_weight` / `ao` (codegen version 2). `forge_shaders::wgsl` (`compile`, `ShaderSetCodegen`) implements it; the cache key folds in a fingerprint of each shader tree, bitmap pixels included.
- Noise: the CPU noise has no permutation table, it hashes cells with SplitMix64 on `u64`; the port keeps the identical hash and seed rules on `vec2<u32>` (16-bit-limb multiply), all 17 kinds in 4D. Bitmaps: `Rgba32Float` `texture_2d` at `@group(1) @binding(k)` read with `textureLoad`, filtering in the shader (exactly the CPU rule, no sampler).
- Measured GPU vs CPU max error (lavapipe, 192 contexts): noise 3.6e-6 (incl. 8 octaves), gradient 3.7e-6, colour/checkerboard/fresnel/falloff 1.7e-6, tiles 2.9e-6, layer/fusion 5.9e-7, colorizer/filter/normalizer 9.0e-7, vertex map/AO 6e-8, bitmap 5.7e-7. Every family keeps the default bound 1e-4 (tight bounds would only fit one driver). Per-case numbers: REALTIME.md §2.9. 50-node compile about 2 ms.
- Deviations: the shader-aware compile lives in `forge-shaders` (it depends on `forge-material`, so material cannot call it); the harness for it is `crates/shaders/tests/wgsl_gpu.rs` (case lists in `tests/common`), not the material `wgsl_gpu.rs`, which only gained the two new `ShadeCtx` fields. An unknown shader name is white (as `ShaderSet`), not the 0.8 fallback.
- Open questions: none.
