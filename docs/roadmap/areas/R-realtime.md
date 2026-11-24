# Area R · Real-time GPU renderer (game-engine style, node materials, ray tracing)

Owner's request (2026-11-24): a GPU renderer that works like a game engine, with real-time
rendering and lighting, on **Vulkan and Metal**, while materials and shaders are still built in a node-based system
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
- **Hardware ray tracing on two APIs (owner requirement: Vulkan *and* Metal):**
  - **Vulkan** (Linux, Windows): `VK_KHR_acceleration_structure` + `VK_KHR_ray_query`
    (NVIDIA RTX, AMD RDNA2+, Intel Arc).
  - **Metal** (macOS): Metal's ray tracing API (`MTLAccelerationStructure`, intersection
    queries in shaders). It runs on **every Apple-silicon Mac**, hardware-accelerated on M3 and
    later; on M1/M2 the same API runs on the GPU's compute units, so Tier 2–3 still work on
    every Apple-silicon Mac, just slower.
  - wgpu exposes ray queries as an experimental feature; per backend, where that is missing or
    too limited, a narrow native backend for the RT passes only is the fallback (decision D19):
    `ash` for Vulkan, `objc2-metal` for Metal. Both sit behind one internal `RtBackend` trait,
    so the passes are written once.
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
2. **Tier 2, hybrid ray tracing (RT GPUs: Vulkan RT, every Apple-silicon Mac via Metal).** Ray-traced shadows (soft, from area lights),
   reflections and ambient occlusion, plus one-bounce diffuse GI with a denoiser, replacing the
   screen-space approximations. Toggles per effect; silent fallback to Tier 1.
3. **Tier 3, real-time path tracing (Vulkan RT or Metal RT; compute fallback elsewhere).** Progressive path tracing of
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
- Optional `crates/render-rt-native` (only if D19's fallback is needed): the `RtBackend` trait
  with an `ash` (Vulkan) and an `objc2-metal` (Metal) implementation of acceleration-structure
  builds and ray dispatch. Everything else stays on wgpu.

## Briefs

| # | Brief | Depends on |
|---|---|---|
| 197 | R1 · Shading IR → WGSL codegen, shader cache, codegen-vs-interpreter tests | 123, 125, 126 |
| 198 | R2 · Real-time raster PBR engine (`render-rt`): frame graph, deferred/clustered lights, shadow maps, IBL | 197, 133, 137 |
| 199 | R3 · Post: SSAO, SSR, bloom, TAA, tone mapping; viewport "Render" mode | 198 |
| 200 | R4 · GPU scene representation: acceleration structures for meshes and instances (clones), updated per frame, on Vulkan RT and Metal RT behind one `RtBackend`; compute-BVH fallback | 142, 198 |
| 201 | R5 · Hybrid ray-traced shadows, reflections, AO and one-bounce GI with denoising (hardware RT, fallback compute) | 200 |
| 202 | R6 · Real-time path tracing with temporal accumulation and denoiser; GPU final-frame output | 200, 201 |
| 203 | R7 · Consistency tests: each tier vs the CPU path tracer on the O30/O43 scenes and material set (PSNR budgets per tier) | 198, 202 |
| 204 | R8 · Performance: S10 scene at ≥ 60 fps (Tier 1) / ≥ 30 fps (Tier 2) on a reference GPU; benchmarks in CI where a GPU exists | 199, 201 |

## Done when

- A node material built in the editor (brief 124) renders identically in structure in all three
  tiers and within the PSNR budget of the CPU tracer (R7).
- Tier 1 runs on every supported OS. Tiers 2–3 run with hardware RT on Vulkan (RTX-class GPUs)
  and on Metal (every Apple-silicon Mac; hardware-accelerated on M3+), and fall back to the
  compute tracer without errors elsewhere. The same scene gives the same image (within the R7
  budget) on Vulkan and Metal.
- Rendering through `RenderEngine` works headless (CLI `forge render --engine rt`) for tests.

## Risks

- wgpu's ray-query support is experimental and may differ per backend: D19 limits the exposure
  (native Vulkan/Metal only for the RT passes, behind one trait).
- Two RT APIs double the RT test surface: R7 runs the consistency suite on both, and the
  owner's Mac is the Metal RT reference machine.
- Denoiser quality on low sample counts; budget a dedicated tuning brief if R6 falls short.
- CI has no RT GPU: Metal RT is tested on the owner's Mac and Vulkan RT on an RTX-class machine
  (manual gate steps); CI covers the compute fallback. macOS CI runners are Apple silicon, so
  Metal RT tests can also run there if the runner exposes the API.
