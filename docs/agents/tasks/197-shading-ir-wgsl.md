# 197 · Shading IR to WGSL codegen and shader cache (part 1 of 2: IR core)

Status: done · Area: R · Depends on: 123 · Observations: – (own design; material values from O29/O42/O43 through the shading IR) · Touches: material, shaders

## Goal
Compile the shading IR (crates/material, brief 123) to WGSL: one specialised shader per material, cached by IR hash, recompiled on edit. This is the bridge from node materials to every GPU tier. **Part 1** covers the IR itself (its node types and every material channel) and the infrastructure; the shader-node families of 125/126 (noise, gradients, tiles, layers, …) are part 2, brief [207](207-shader-nodes-wgsl.md). Split on the implementer's recommendation (2026-11-24): the whole port was too large for one run.

## Inputs
- Area design: [../../roadmap/areas/R-realtime.md](../../roadmap/areas/R-realtime.md)
- Decision D19 in [../../roadmap/DECISIONS.md](../../roadmap/DECISIONS.md)
- docs/specs/MATERIAL.md, SHADERS.md, RENDER.md

## Design decisions (planner)
- Write the codegen design in a new section of RENDER.md (or a new `docs/specs/REALTIME.md`, §1) first: the WGSL entry points, how a material becomes one function (`fn material_N(ctx: ShadeCtx) -> Surface`), the binding layout for textures and parameters, the fallback rule for unsupported nodes, and the cache key (stable hash of the IR + codegen version).
- Unsupported shader nodes (everything from 125/126 until 207 lands) compile to a documented constant fallback (the node's default colour) and are listed by the compiler, so materials always compile.
- Validate every generated module with `naga` (already in the tree through wgpu) in plain unit tests, so CI checks codegen without a GPU.

## Done when
- [x] The four IR node types and every material channel have a WGSL equivalent; shader nodes return the documented fallback and are reported.
- [x] Every generated module passes `naga` validation in a unit test (no GPU needed).
- [x] A shader cache keyed by IR hash: the same IR gives the same module without recompiling; an edit recompiles only that material.
- [x] A test harness evaluates the IR channels on a grid of sample points through the CPU interpreter and through the WGSL on the GPU (skipped without an adapter; runs on the M5 Pro's Metal) and compares within 1e-4 (f32). Part 2 extends this harness to the shader nodes.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md` if parity results change, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: part 1 is complete. `forge_material::wgsl` generates one `forge_surface` per material from the IR: all four node types (Color, Texture fallback, Scale, Mix with all four modes) and every material channel (diffuse, specular, emission, transmission, environment, fog, alpha, bump, normal, displacement, diffusion) are packed into 24 `vec4` Surface rows. `ShaderCache` keys modules by a stable FNV-1a hash of `CODEGEN_VERSION` and the canonical IR; the same graph returns the same `Arc`, an edit recompiles only that material.
- Rules/design in REALTIME.md §1: entry point, Surface layout, enum indices, the `(0.8,0.8,0.8)` fallback for `Texture`, naga validation, cache key. Four plain tests: all nodes/channels validate with naga, an empty graph validates, cache reuse + one-edit recompile + compile counter, and a 50-node material compiles well under 100 ms. The GPU test dispatches a compute pass over 64 contexts and compares all 24 rows with the CPU `ShadingGraph::eval` (fallback sampler) within 1e-4; it ran on the Mac's Metal adapter in 2.7 s and skips without an adapter (or panics under `FORGE_REQUIRE_GPU=1`).
- Deviations: the spec asks for `fn material_N(ctx: ShadeCtx) -> Surface`; the emitted function is `forge_surface` (one module per material, so no index is needed). Shader families are part 2 (brief 207) and use the fallback here, as the split allows.
- Files changed: `crates/material/src/wgsl.rs` (new), `crates/material/src/lib.rs`, `crates/material/Cargo.toml` (naga/wgpu/pollster dev-deps), `crates/material/tests/wgsl.rs` (new), `crates/material/tests/wgsl_gpu.rs` (new), `docs/specs/REALTIME.md` (new), `docs/parity/STATUS.md`, `docs/roadmap/BACKLOG.md`.
- Open questions: none for part 1; brief 207 decides the per-family numerical bounds (especially noise).
- The last lines of the gate output: `GATE: 2550 tests passed, 0 failed` and `GATE: all passed`.
