# Area R · Real-time GPU renderer (game-engine style, node materials, ray tracing)

Owner's request (2026-11-24): a GPU renderer that works like a game engine, with real-time
rendering and lighting, while materials and shaders are still built in a node-based system
(the way the common node-based production renderers and DCCs work), using hardware ray tracing
where the GPU has it for fast, high-quality real-time results.

This is **our own design**, not a parity area: the reference app's own real-time engine is not
a v1 target (D13). Material *values* still come from the reference (O29, O42, O43) through the
shared shading IR, so a material looks the same in the CPU path tracer (the reference renderer
for quality), the real-time renderer and the viewport.

## Is it possible?

Yes. The pieces it needs already exist or are planned:

- **One shading IR** (`crates/material`, brief 123): every material, node graph (brief 124)
  included, compiles to it. The CPU tracer interprets it today.
- **wgpu 30** in the viewport: it runs on **Vulkan** (Linux, Windows), Metal (macOS) and DX12,
  from one code base.
- **Hardware ray tracing:** Vulkan's `VK_KHR_acceleration_structure` + `VK_KHR_ray_query`
  (NVIDIA RTX, AMD RDNA2+, Intel Arc) gives acceleration-structure builds and ray queries from
  any shader stage. wgpu exposes ray queries as an experimental feature; where that is missing
  or too limited, a narrow `ash` (raw Vulkan) backend for the RT passes only is the fallback
  (decision D19). macOS has no Vulkan ray tracing (MoltenVK), so Macs use the software path.
- **Software ray tracing fallback:** a compute-shader BVH tracer (brief 142) runs everywhere,
  slower, with the same results.

## Design: three tiers, one material system

```
 node graph (124) ──► shading IR (123) ──► WGSL codegen (R1) ─┬─► Tier 1: real-time raster PBR (R2, R3)
                                                             ├─► Tier 2: hybrid RT effects (R5)
                                                             └─► Tier 3: real-time path tracing (R6)
                       CPU path tracer (136) = ground truth for all tiers (R7)
```

1. **Tier 1, real-time raster (every GPU).** Deferred or clustered-forward PBR; the
   metallic-roughness BSDF matching the IR; directional/spot/point/area lights with shadow
   maps; image-based lighting from the scene environment (brief 133); screen-space AO and
   reflections, bloom, temporal anti-aliasing, filmic tone mapping. 60 fps on a mid-range GPU
   for S10-sized scenes. This becomes the viewport's "Render" display mode.
2. **Tier 2, hybrid ray tracing (RT GPUs).** Ray-traced shadows (soft, from area lights),
   reflections and ambient occlusion, plus one-bounce diffuse GI with a denoiser, replacing the
   screen-space approximations. Toggles per effect; silent fallback to Tier 1.
3. **Tier 3, real-time path tracing (RT GPUs, compute fallback).** Progressive path tracing of
   the same scene with temporal accumulation and a denoiser (Intel Open Image Denoise, Apache-2.0,
   or an own SVGF-style filter), converging towards the CPU tracer's image. This doubles as the
   GPU final-frame renderer (supersedes brief 142's goal).

**Node materials.** The node graph never runs per pixel as a graph: the IR is compiled to WGSL
(R1) with one specialised shader per material (cached by IR hash, recompiled on edit in under
100 ms). Texture nodes (noise, gradient, image, fresnel, layer …) become WGSL functions shared
with the path tracers, so procedural shaders look the same everywhere. Nodes a tier cannot do
fall back documented (e.g. subsurface → diffuse wrap in Tier 1).

## Crates

- `crates/render-rt` (new): the real-time engine (frame graph, passes, shader cache, light and
  shadow management). Implements `RenderEngine` (brief 135) and a viewport hook.
- `crates/shaders` / `crates/material`: gain the WGSL codegen (R1).
- `crates/render-gpu`: the compute BVH tracer (142), reused by Tiers 2–3 on non-RT GPUs.
- Optional `crates/render-vk` (only if D19's fallback is needed): `ash`-based RT passes.

## Briefs

| # | Brief | Depends on |
|---|---|---|
| 197 | R1 · Shading IR → WGSL codegen, shader cache, codegen-vs-interpreter tests | 123, 125, 126 |
| 198 | R2 · Real-time raster PBR engine (`render-rt`): frame graph, deferred/clustered lights, shadow maps, IBL | 197, 133, 137 |
| 199 | R3 · Post: SSAO, SSR, bloom, TAA, tone mapping; viewport "Render" mode | 198 |
| 200 | R4 · GPU scene representation: BLAS/TLAS for meshes and instances (clones), updates per frame; compute-BVH fallback | 142, 198 |
| 201 | R5 · Hybrid ray-traced shadows, reflections, AO and one-bounce GI with denoising (hardware RT, fallback compute) | 200 |
| 202 | R6 · Real-time path tracing with temporal accumulation and denoiser; GPU final-frame output | 200, 201 |
| 203 | R7 · Consistency tests: each tier vs the CPU path tracer on the O30/O43 scenes and material set (PSNR budgets per tier) | 198, 202 |
| 204 | R8 · Performance: S10 scene at ≥ 60 fps (Tier 1) / ≥ 30 fps (Tier 2) on a reference GPU; benchmarks in CI where a GPU exists | 199, 201 |

## Done when

- A node material built in the editor (brief 124) renders identically in structure in all three
  tiers and within the PSNR budget of the CPU tracer (R7).
- Tier 1 runs on every supported OS; Tiers 2–3 use hardware RT where present and fall back
  without errors.
- Rendering through `RenderEngine` works headless (CLI `forge render --engine rt`) for tests.

## Risks

- wgpu's ray-query support is experimental: D19 limits the exposure (raw Vulkan only for RT).
- Denoiser quality on low sample counts; budget a dedicated tuning brief if R6 falls short.
- CI has no RT GPU: RT passes are tested on the owner's machine (manual gate step) and in CI
  through the compute fallback.
