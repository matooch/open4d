# 198 · Real-time raster PBR engine (render-rt)

Status: waiting: 197, 133, 137 · Area: R · Depends on: 197, 133, 137 · Observations: – (own design; material values from O29/O42/O43 through the shading IR) · Touches: render-rt (new), viewport, render-api

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
