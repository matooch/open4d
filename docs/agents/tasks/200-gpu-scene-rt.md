# 200 · GPU scene: acceleration structures and compute fallback

Status: ready · Area: R · Depends on: 142, 198 · Observations: – (own design; material values from O29/O42/O43 through the shading IR) · Touches: render-rt, render-gpu

## Goal
Build and update bottom/top-level acceleration structures for meshes and clone instances each frame. Use hardware RT on **both** Vulkan (ray query) and Metal (Metal ray tracing) through wgpu's experimental features, or per D19 through a narrow native backend per API behind one `RtBackend` trait; else the compute BVH from brief 142.

## Inputs
- Area design: [../../roadmap/areas/R-realtime.md](../../roadmap/areas/R-realtime.md)
- Decision D19 in [../../roadmap/DECISIONS.md](../../roadmap/DECISIONS.md)
- docs/specs/MATERIAL.md, SHADERS.md, RENDER.md

## Done when
- [ ] Vulkan RT, Metal RT and the compute path return identical hit results on a test scene (ray-by-ray comparison; each RT path runs where its API exists).
- [ ] Instancing: 10 000 clones of one mesh use one BLAS and a TLAS update per frame.
- [ ] Feature detection: no RT support selects the compute path without errors.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md` if parity results change, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
