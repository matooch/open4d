# 198 · Real-time raster PBR engine (render-rt)

Status: done · Area: R · Depends on: 197, 133, 137 · Observations: – (own design; material values from O29/O42/O43 through the shading IR) · Touches: render-rt (new), viewport, render-api

## Goal
A new crate render-rt implementing RenderEngine (brief 135): a small frame graph, deferred or clustered-forward PBR with the IR BSDF, directional/spot/point/area lights with shadow maps, and image-based lighting from the scene environment (133).

## Inputs
- Area design: [../../roadmap/areas/R-realtime.md](../../roadmap/areas/R-realtime.md)
- Decision D19 in [../../roadmap/DECISIONS.md](../../roadmap/DECISIONS.md)
- docs/specs/MATERIAL.md, SHADERS.md, RENDER.md

## Done when
- [ ] Renders the O30/O43 scenes headless (`forge render --engine rt`), deterministic per adapter.
- [ ] Lights and materials come from the scene (047, 194), no special cases.
- [ ] Runs on Linux (Vulkan), macOS (Metal) and Windows (Vulkan/DX12) in CI where an adapter exists; skipped otherwise.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md` if parity results change, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)

Status: done

### What was built

`crates/render-rt` (`forge-render-rt`) implements `RenderEngine` as `forge.render.rt`
(RENDER.md R21, own design): a small validated frame graph; a deferred two-target G-buffer;
one shadow-map tile per shadow-casting light (six cube faces for omni) in a 4096x2048 depth
atlas; deferred lighting with the shared IR surface functions (the same `forge_surface_i` code as
the compute tracer), the R20.2 BRDF, light falloff/spot cones, PCF, the R8 shadow colour rule and
constant/sky/lat-long-image environment lighting; beauty plus all ten R3 AOVs (the light split and
albedo run as extra full-screen passes only when asked for). Geometry, lights and materials all
come from the `Snapshot` through `render-gpu`'s packing, so there are no special cases.
`forge render --engine cpu|gpu|rt` selects the engine (default unchanged).

### Result

- `crates/render-rt/tests/rt_render.rs`: directional, omni-falloff, glossy two-light and
  two-material scenes against the CPU tracer with its indirect light off: 48.0, 38.3, 45.2 and
  37.5 dB (budget 30 dB). All 18 renderable O30 cases keep the CPU silhouette (IoU 0.9966-1.0000,
  budget 0.98) and mean luminance ratio 0.74-1.07 (bound 0.6-1.7). Light intensity scales exactly,
  a spot shadow map darkens the occluded plane, constant and image environments light a sphere,
  the ten AOVs satisfy R3, two renders are byte-identical, the region/events/cancel contract holds.
- `crates/render-rt/tests/shader.rs`: the WGSL (with and without materials) parses and validates
  with naga; texture-node materials are pinned `Unsupported`.
- Frame-graph unit tests (order, one writer, duplicate names, unknown reads, host inputs).
- `apps/forge/tests/render.rs`: `forge render --engine rt` renders through the CLI (skipped
  without an adapter unless `FORGE_REQUIRE_GPU=1`); an unknown engine is a usage error.
- `sh scripts/gate.sh`: see the gate result in the commit message.

### Pinned (parity-pending, all with reasons in R21)

- Shader-node material channels: `Unsupported` (brief 207), as on the compute tracer —
  `mat-checker-uv`, `mat-checker-cubic`, `mat-fresnel`, `mat-bump-noise`, `checker-plane`.
- Transmission is not refracted (R2 is opaque): `mat-transparent` is compared by silhouette only.
- Depth of field: `Unsupported` in the raster tier.
- No path-traced GI, AO or screen-space effects: Tier 1 direct light plus IBL only (brief 199 adds
  the screen-space effects, brief 201 the ray-traced GI).
- Region: one tile covering the region (the raster has no per-tile scheduling).

### Notes for the planner

- Metal limits colour attachments to 32 bytes/sample, so the G-buffer is two `Rgba32Float`
  targets and the material is re-evaluated in the lighting pass (UVW is not stored; only
  unsupported texture channels read it).
- `render-gpu` gained `Packed::light_count` and `shader::materials()`; both are used by the new
  tier and keep the compute tracer's behaviour unchanged.
- Vulkan/DX12 were not run on this machine: the wgpu code paths are shared, and the GPU tests
  skip without an adapter. Recorded as parity-pending per the queue's instructions.

### Follow-up: Windows CI (DirectX 12, software adapter)

- Readback no longer panics: the map callback stores its `Result`, the engine returns
  `RenderError::Output` (R20, R21). Shared helpers `map_read` / `await_mapped` in `forge-render-gpu`.
- Compute tracer submits bands of rows under a pixels x samples budget, so no submission trips a
  driver watchdog (R20.4); the image is byte-identical for any budget (test in `cpu_vs_gpu.rs`).
- Shadows on DX12: cause not found without a DX12 device. Shadow passes already submit once per
  tile, so the last-matrix-wins hypothesis does not apply; what was checked and what is still open
  is recorded in R21.4 ("Per-pass state"). New test `two_shadowed_lights_each_cast_their_own_shadow`.
