# 200 · GPU scene: RtBackend trait, instanced scene, compute backend (part 1 of 3)

Status: done · Area: R · Depends on: 142, 198 · Observations: – (own design) · Touches: render-rt, render-gpu

## Goal
The ray-tracing foundation every later tier (201–204) uses: one `RtBackend` trait and a **compute
backend** that works on every GPU. Hardware backends come in two small follow-ups: Metal (211) and
Vulkan (212). Split on 2026-11-25 because the whole brief was too big to start in one session.

## Inputs
- Area design: [../../roadmap/areas/R-realtime.md](../../roadmap/areas/R-realtime.md), decision D19
- Brief 142's compute tracer (crates/render-gpu: the flattened SAH tree, `trace.wgsl` traversal)
- docs/specs/RENDER.md R20 (GPU path tracer), R21 (raster engine)

## Steps (one commit each; each is a good session-sized unit)
1. **Spec** (RENDER.md, new R23): the `RtBackend` trait (build/update the scene, trace a batch of
   rays → hits: t, triangle id, instance id, barycentrics), the scene layout (one BLAS per unique
   mesh, one TLAS of instance transforms, rebuilt or refit per frame), feature detection order
   (Metal RT, Vulkan ray query, else compute), and the hit-comparison rule used by 211/212.
2. **Tests first** (CPU-side reference + GPU): a small instanced scene; rays traced on the CPU
   (reuse forge-render-cpu's BVH) and by the compute backend must give identical hits (same
   triangle and instance ids; t and barycentrics within 1e-4). Plus: 10 000 clones of one mesh use
   one BLAS and one TLAS update per frame; feature detection without RT picks compute.
3. **Code**: the trait, the scene build (BLAS per mesh, TLAS per frame), the compute backend (two-
   level traversal in WGSL, reusing 142's BLAS layout), feature detection.
4. naga translation test for every new module (SPIR-V, MSL, HLSL), gate, Report.

## Done when
- [x] Compute backend hits equal the CPU reference on the test scene; instancing and detection tests pass.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md` if parity results change, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)

- R23 uses the R20.1 BVH builder and node layout with successor links for stackless traversal.
  The compute backend uploads object-space triangles once per indexed mesh and reuses those
  buffers on transform-only updates. Exact-distance ties and extreme values outside `f32`
  precision are not part of the CPU agreement fixture.
- Compute ran on the M5 Pro's Metal adapter without requesting experimental ray queries.
  The GPU agreed with the CPU on hit/miss, IDs, and `t/u/v` within `1e-4`, including 10,000
  clones, a moved clone, overlapping nonuniformly scaled instances, and a second BLAS
  with interior nodes. Naga translated the compute shader to SPIR-V, MSL and HLSL.
- No reference observations or golden cases apply. Metal hardware RT belongs to 211;
  Vulkan RT run pending on the owner's RTX 3090 (`parity-pending` for 212).
- Gate: 2663 tests passed, 0 failed; `GATE: all passed`.
