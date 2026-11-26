# 212 · Vulkan hardware ray-tracing backend (part 3 of 200)

Status: done (parity-pending: RTX 3090 run) · Area: R · Depends on: 200 · Observations: – (own design) · Touches: render-rt

## Goal
Implement `RtBackend` (R23, brief 200) with **Vulkan ray query** (`VK_KHR_acceleration_structure`
+ `VK_KHR_ray_query`): through wgpu's experimental ray queries if they cover the R23 trait,
otherwise per D19 with a narrow `ash` backend behind the same trait.

## Done when
- [x] Uses the cross-platform wgpu API; the hit-comparison test (Vulkan RT vs compute) runs where a Vulkan RT
      device exists and skips elsewhere. **Run on the owner's RTX 3090** (record the result in the
      Report; until then this brief is `done (parity-pending: RTX 3090 run)`).
- [x] Feature detection picks Vulkan RT where available and falls back to compute otherwise.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Inputs
- Brief 200 (RtBackend trait, compute backend, test scene) and RENDER.md R23; decision D19
- docs/roadmap/areas/R-realtime.md (reference machines: M5 Pro = Metal, RTX 3090 = Vulkan)

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report

- R23.2 uses the same wgpu `HardwareBackend` on Metal and Vulkan. The pinned
  wgpu 30.0.1 Vulkan backend supplies ray queries and BLAS/TLAS builds, so
  D19's native `ash` fallback is not needed. Batch queries, hybrid rendering,
  Tier 3 and the S10 runner distinguish Vulkan hardware from Compute and Metal.
- On the M5 Pro, five required-GPU runs passed `ray_scene`, `path_trace`, and
  `naga_translate`. The comparison exercised 10,000 clones and transform-only
  updates with unchanged hit tolerances. The shared hardware WGSL translated to
  SPIR-V, MSL and HLSL. Vulkan-only selection on this Mac skipped as expected;
  requiring Vulkan on the Metal device failed as intended. No reference
  observations or R23 golden cases apply. No Vulkan runtime result is claimed.
- **Vulkan RT run pending on the owner's RTX 3090.** Run the following on the
  Vulkan host (POSIX shell). The first command cannot pass on compute fallback.
  For R26, copy the 69 Metal captures from the same revision as described in
  [203](203-rt-consistency.md); for R8, compare only with a same-machine baseline
  as described in [204](204-rt-performance.md).

  ```sh
  export CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
  export WGPU_BACKEND=vulkan FORGE_REQUIRE_GPU=1
  FORGE_REQUIRE_VULKAN_RT=1 cargo test -p forge-render-rt --test ray_scene -- --nocapture
  FORGE_R26_HARDWARE=Vulkan FORGE_R26_METAL_IMAGES="$PWD/r26-metal" \
    cargo test -p forge-render-rt --test path_trace o30_and_o43_cross_tier_consistency -- --nocapture
  cargo bench -p forge-render-rt --bench s10 -- rtx-3090 tier1 s10-vulkan-tier1.json
  cargo bench -p forge-render-rt --bench s10 -- rtx-3090 tier2 s10-vulkan-tier2.json
  ```

- Gate on macOS: `GATE: 2735 tests passed, 0 failed`; `GATE: all passed`.
