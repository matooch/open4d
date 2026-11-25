# 142 · GPU path tracer (wgpu compute)

Status: done · Area: K · Depends on: 136 · Observations: O30 · Touches: render-gpu

## Goal
A wgpu compute path tracer matching the CPU tracer.

## Inputs
- Area design: [../../roadmap/areas/K-rendering.md](../../roadmap/areas/K-rendering.md)
- O30: [../../parity/render-reference.md](../../parity/render-reference.md) and its golden cases

## Design decisions (planner)
- Same IR compiled to WGSL; BVH flattened to buffers.

## Done when
- [ ] CPU vs GPU PSNR ≥ 35 dB on O30 scenes (skip without an adapter).
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)

- **Result:** `forge-render-gpu` is a second `RenderEngine` (`forge.render.gpu`) that runs the R9
  integrator as a wgpu compute pass. Tests in `crates/render-gpu/tests/cpu_vs_gpu.rs`:
  - `beauty_matches_the_cpu_on_the_o30_cases`: 18/18 cases at 320x240, 16 spp, cubic filter
    width 2 — CPU against GPU PSNR **71.5–158.2 dB** (goal 35). Cases that agree to the last f32
    digit (e.g. `cube-default-light` 153.5 dB) show that the sampling is the CPU's sequence, not
    just a converged image.
  - `aovs_match_the_cpu`: all ten AOVs on `cube-plane` (Depth 95 dB, Normal 183, Albedo 164,
    Position 108, ids exact, Shadow 159, Diffuse/Specular/Emission > 100).
  - `random_sequence_matches_the_cpu`: the WGSL 64-bit emulation, splitmix and PCG32 reproduce
    `forge_render_cpu::sampling` bit for bit (`f()` within 1e-6 of the f64 value).
  - `events_and_cancel`: one `Tile` per tile (payload rebuilds the image), `Started`/`Progress`/
    `Finished` in order, a cancelled control returns `RenderError::Cancelled`.
  - `image_environment_is_unsupported`, `unsupported_pins_are_exact`.
  - `UNSUPPORTED` (asserted exactly, all five return `Unsupported`): `mat-checker-uv`,
    `mat-checker-cubic`, `mat-fresnel`, `mat-bump-noise`, `checker-plane` — their materials have a
    `Texture` (shader-node) channel, which the IR codegen compiles to the fallback colour until
    REALTIME.md §2 lands (brief 207). `KNOWN_MISMATCH`: none.
- **Rules derived (and the cases that prove them):**
  - World-space triangles plus the R6 binned-SAH tree give the CPU's hit: every O30 case matches.
  - The pixel seed `splitmix(seed ^ splitmix((x << 32) | y))` and the PCG32 stream must be the
    CPU's, including the R2 sequence and the extra cubic-filter draws: with a different sequence
    the 16 spp images drop to ~3.6 dB (`area-soft`), with it they match (`random_sequence`,
    `beauty`).
  - NEE weights: `w = 1` for delta lights (`ls.pdf == 0`), power heuristic otherwise
    (`cube-default-light` renders black without it).
  - The AOV split follows the *first* lobe: `target_of(lobe)` maps diffuse→Diffuse,
    specular→Specular, none→Emission; using the lobe number directly put every bounce into
    Emission (`aovs_match_the_cpu`).
  - Shadow rays are only traced when the light casts shadows (`cube-default-light`).
- **Deviations from the design decisions:** none of substance. The material dispatcher renames the
  generated `forge_surface`/`ir*` per material slot so several graphs share one module; a material
  with a shader node is rejected instead of silently using the fallback colour, so the five cases
  above are `UNSUPPORTED` rather than quietly wrong. The BVH builder and light preparation are the
  CPU's own (`forge_render_cpu::bvh`, `lights::prepare`) — one implementation, flattened to
  buffers — which needed two accessors (`Bvh::nodes/prims`, `Blas::flip`).
- **Parity-pending:** nothing observed was left unread; the GPU-vs-CPU rule itself is our own
  design (R20.3), not an observation.
- **Files changed:** `crates/render-gpu/{Cargo.toml,src/{lib,engine,pack,shader}.rs,src/{rng,trace}.wgsl,tests/cpu_vs_gpu.rs}`,
  `crates/render-cpu/src/{bvh,scene,engine,lib}.rs` (accessors only), `docs/specs/RENDER.md` §R20,
  `docs/parity/STATUS.md`, `docs/roadmap/BACKLOG.md`.
- **Open questions:** (1) shader-node materials need REALTIME.md §2 (brief 207) before the GPU can
  render `mat-checker-*`, `mat-fresnel`, `mat-bump-noise`, `checker-plane`; (2) image
  environments need the lat-long image uploaded plus R16's importance sampling in WGSL — until
  then the GPU returns `Unsupported` and the CPU keeps that path; (3) `threads`/`tile` remain
  CPU scheduling knobs, and Vulkan/Metal portability of the shader is untested outside this Mac
  (the gate's CI machine has no adapter, so its GPU tests skip).
- **Gate:** `sh scripts/gate.sh` with `RUST_TEST_THREADS=1` (the known
  `a_shared_node_runs_once_per_evaluation` race): fmt ok, clippy `-D warnings` ok, 2592 tests
  passed 0 failed (the GPU tests ran on Metal), python ok, `cargo deny` ok, cleanroom ok, golden
  1117 cases ok, `GATE: all passed`.
- **Follow-up (planner), shader-node materials on the GPU:** RENDER.md R20.1b. The tracer now
  compiles every material into one shared `ModuleBuilder` (`forge_material::wgsl::compile_into`,
  new) through the sampler's new `TextureSampler::codegen()` hook (`ShaderSet` returns
  `ShaderSetCodegen`, `White` returns `WhiteShaders`); bitmaps are uploaded as `Rgba32Float`
  textures at `@group(1) @binding(k)`. The per-hit `ShadeCtx` of `surface_at` already matched the
  CPU (uvw interpolated from the baked corner UVWs, so cubic projection needs nothing extra;
  position, shading normal, `view = wo`, bump re-interpolates `uvw` at the displaced point); the
  one difference found was `vertex_weight`, which was 1 and is now 0 (the CPU `None`, read as 0 by
  Vertex Map). All five cases are unpinned (`UNSUPPORTED` is empty) and pass the 35 dB bound:
  `mat-checker-uv` 75.50 dB, `mat-checker-cubic` 75.43 dB, `mat-fresnel` 102.54 dB,
  `mat-bump-noise` 83.26 dB, `checker-plane` 119.89 dB (lavapipe, 320x240, 16 spp). New test
  `shader_materials_share_one_module_and_match_the_cpu` (two bitmaps, Vertex Map, AO, bump from a
  bitmap, one module; 134.7-136.1 dB) and a sampler without codegen is `Unsupported`.
  The naga 30 SPIR-V writer (Vulkan, lavapipe) panics on the former `acc_add(&out.acc, ...)`
  (pointer to a sub-field as an argument), so `acc_add` now takes the whole `TraceOut`; the Metal
  path never hit it. Image environments stay `Unsupported` (R20.4).
