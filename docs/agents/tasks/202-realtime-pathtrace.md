# 202 · Real-time path tracing with denoising

Status: waiting: 200, 201 · Area: R · Depends on: 200, 201 · Observations: – (own design; material values from O29/O42/O43 through the shading IR) · Touches: render-rt

## Goal
Progressive path tracing of the full scene with temporal accumulation and a denoiser (Open Image Denoise, Apache-2.0, or an own filter). Also the GPU final-frame renderer (supersedes the goal of brief 142).

## Inputs
- Area design: [../../roadmap/areas/R-realtime.md](../../roadmap/areas/R-realtime.md)
- Decision D19 in [../../roadmap/DECISIONS.md](../../roadmap/DECISIONS.md)
- docs/specs/MATERIAL.md, SHADERS.md, RENDER.md

## Done when
- [ ] After N samples the image is within the PSNR budget of the CPU tracer on the O30/O43 scenes (R7 sets the budgets).
- [ ] Camera or scene edits restart accumulation; the denoised preview appears within one frame.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md` if parity results change, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
