# 209 · Shader-node materials in the real-time raster engine

Status: done · Area: R · Depends on: 198, 207 · Observations: – (own design; values through the shading IR) · Touches: render-rt

## Goal
The raster engine (`forge.render.rt`, RENDER.md R21) still returns `Unsupported` for materials with
shader nodes (5 O30 cases pinned in `crates/render-rt/tests/rt_render.rs`). The path tracer already
compiles them through the samplers' WGSL codegen with bitmaps at `@group(1)` (R20.1b,
`render_gpu::shader::assemble`). Do the same in the raster engine.

## Design decisions (planner)
- Spec first: R21.3 gains how the raster passes get shader-aware material code and how `ShadeCtx`
  is filled per fragment (UVW, normal, view, bump), matching R20.1b and the CPU.
- Reuse the shared `ModuleBuilder` path (extend `render_gpu::shader::materials` to take samplers,
  or share one helper with `assemble`), so both engines compile materials one way.
- Bitmaps bind in a group the raster pipelines don't use yet.

## Done when
- [x] The five pinned O30 cases render and keep the CPU silhouette and light-level bounds of the other 18 (R21 tests); the pin list is empty.
- [x] A naga test translates every raster module (with shader nodes) to SPIR-V, MSL and HLSL (§5a).
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: the raster engine renders shader-node materials; the pin list is empty. Measured against the CPU (IoU, mean light-level ratio): checker-plane 0.9990 / 1.004, mat-checker-uv 0.9972 / 0.999, mat-checker-cubic 0.9972 / 1.000, mat-fresnel 0.9972 / 1.013, mat-bump-noise 0.9972 / 1.214 (bounds IoU >= 0.98, ratio 0.6-1.7, unchanged). All 23 O30 cases pass; the other 18 are unchanged.
- Rules derived (RENDER.md R21.2, R21.3, R21.7): `render_gpu::shader::materials(&[(graph, sampler)], bitmap_group)` is the one material compile path (returns the module text and the bitmap resources); the path tracer's `assemble` is that plus its constants, RNG and tracer (module text unchanged), the raster module is that with the bitmaps moved to `@group(2)` plus `rt.wgsl`. The G-buffer's `g1.w` now holds the triangle index + 1 (the object id is read back from the packed triangles for the `ObjectId` AOV); the lighting pass fills `ShadeCtx` from that triangle as the tracer's `surface_at` does (UVW re-solved from the position, outward-side normal, `vertex_weight = 0` as the CPU `None`, bump from the mean of the bump channel with two displaced contexts). Bump now shades; the normal, displacement, alpha, fog, diffusion and environment channels are still ignored.
- Deviations from the design decisions: the codegen hard-codes `@group(1)` for bitmaps, so `materials` rewrites that prefix in the declarations to the requested group instead of changing the shader crate (its text is pinned by its own tests).
- Files changed: `docs/specs/RENDER.md`, `crates/render-gpu/src/shader.rs`, `crates/render-rt/src/{shader.rs,engine.rs,rt.wgsl}`, `crates/render-rt/tests/{rt_render.rs,shader.rs,naga_translate.rs}`, `crates/render-rt/Cargo.toml`, BACKLOG, STATUS, PROGRESS, this brief.
- Open questions: none.
