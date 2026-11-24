# 197 · Shading IR to WGSL codegen and shader cache (part 1 of 2: IR core)

Status: ready · Area: R · Depends on: 123 · Observations: – (own design; material values from O29/O42/O43 through the shading IR) · Touches: material, shaders

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
- [ ] The four IR node types and every material channel have a WGSL equivalent; shader nodes return the documented fallback and are reported.
- [ ] Every generated module passes `naga` validation in a unit test (no GPU needed).
- [ ] A shader cache keyed by IR hash: the same IR gives the same module without recompiling; an edit recompiles only that material.
- [ ] A test harness evaluates the IR channels on a grid of sample points through the CPU interpreter and through the WGSL on the GPU (skipped without an adapter; runs on the M5 Pro's Metal) and compares within 1e-4 (f32). Part 2 extends this harness to the shader nodes.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md` if parity results change, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
