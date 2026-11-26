# 211 · Metal hardware ray-tracing backend (part 2 of 200)

Status: done · Area: R · Depends on: 200 · Observations: – (own design) · Touches: render-rt

## Goal
Implement `RtBackend` (R23, brief 200) with **Metal ray tracing** (acceleration structures and
intersection queries): through wgpu's experimental ray-query support on Metal if it covers
the R23 trait, otherwise per D19 with a narrow `objc2-metal` backend behind the same trait.

## Done when
- [x] On the Mac (M5 Pro), Metal RT hits equal the compute backend's on the R23 test scene
      (same ids; t and barycentrics within 1e-4); skipped without Metal RT.
- [x] Feature detection picks Metal RT where available and falls back to compute otherwise.
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
- Result: five GPU-required runs on the Apple M5 Pro selected Metal and agreed with
  compute and the CPU reference on all 276 ray results per run. Hit/miss and IDs
  matched exactly; `t`, `u` and `v` stayed within `1e-4`. Verified one BLAS for
  10,000 clones, BLAS reuse when moving a clone, and sparse and empty snapshots.
- Implementation (RENDER.md R23.1): wgpu's Metal ray queries cover the trait,
  so no native dependency was added. The factory requests compute when Metal
  ray queries are unavailable, the experimental device request fails, or
  hardware is explicitly disabled. The acceptance test also executes that
  explicit compute path. Naga translated the hardware shader to SPIR-V, MSL and HLSL.
- Parity: no reference observations or golden cases apply. The fixture avoids
  equal-distance ties, as R23 specifies. Vulkan hardware remains brief 212.
- Files: `crates/render-rt/src/ray.rs`, `crates/render-rt/src/ray/`,
  `crates/render-rt/tests/ray_scene.rs`, `crates/render-rt/tests/naga_translate.rs`,
  `docs/specs/RENDER.md`, and the backlog and task brief.
- Gate: `GATE: 2664 tests passed, 0 failed`; `GATE: all passed`.
