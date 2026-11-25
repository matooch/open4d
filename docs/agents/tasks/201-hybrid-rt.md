# 201 · Hybrid ray-traced shadows, reflections, AO and GI

Status: in-progress (hermes) · Area: R · Depends on: 200 · Observations: – (own design; material values from O29/O42/O43 through the shading IR) · Touches: render-rt

## Goal
Replace the screen-space approximations with ray-traced soft shadows (area lights), reflections, ambient occlusion and one-bounce diffuse GI, with a spatio-temporal denoiser. Each effect toggles; fallback to Tier 1 when RT is off.

## Inputs
- Area design: [../../roadmap/areas/R-realtime.md](../../roadmap/areas/R-realtime.md)
- Decision D19 in [../../roadmap/DECISIONS.md](../../roadmap/DECISIONS.md)
- docs/specs/MATERIAL.md, SHADERS.md, RENDER.md

## Done when
- [ ] Each effect is closer to the CPU path tracer than its Tier-1 approximation on the O43 scenes (PSNR improves).
- [ ] Temporal stability: a static camera converges without flicker (frame-to-frame difference test).
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md` if parity results change, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)

AO and shadow checkpoint, using controlled scenes with O43 `mat-red-matte`
material and Forge CPU-traced references. First-frame linear beauty improves
from 17.223 to 35.230 dB PSNR for AO and from 35.917 to 48.877 dB for area
shadows on Metal and on the explicit compute fallback. Tests cover exact
Tier 1 fallback, SSAO replacement, zero AO radius, shadow lighting AOVs,
unmodified non-shadow-casting lights, decreasing successive-frame delta and
camera-change history invalidation for both effects. Reflections and GI
remain to be implemented and checked; R24 is not done.
