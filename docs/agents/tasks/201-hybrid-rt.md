# 201 · Hybrid ray-traced shadows, reflections, AO and GI

Status: done · Area: R · Depends on: 200 · Observations: – (own design; material values from O29/O42/O43 through the shading IR) · Touches: render-rt

## Goal
Replace the screen-space approximations with ray-traced soft shadows (area lights), reflections, ambient occlusion and one-bounce diffuse GI, with a spatio-temporal denoiser. Each effect toggles; fallback to Tier 1 when RT is off.

## Inputs
- Area design: [../../roadmap/areas/R-realtime.md](../../roadmap/areas/R-realtime.md)
- Decision D19 in [../../roadmap/DECISIONS.md](../../roadmap/DECISIONS.md)
- docs/specs/MATERIAL.md, SHADERS.md, RENDER.md

## Done when
- [x] Each effect is closer to the CPU path tracer than its Tier-1 approximation on the O43 scenes (PSNR improves).
- [x] Temporal stability: a static camera converges without flicker (frame-to-frame difference test).
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md` if parity results change, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)

`render-rt/tests/hybrid.rs` loads unchanged O43 `mat-red-matte`, `mat-glossy`
and `mat-luminance` material inputs. The original material-only scenes do not
isolate the four effects, so the fixtures add controlled geometry and lighting
and compare to higher-sample Forge CPU renders rather than the O43 PNGs.

| Effect | Tier 1 PSNR | Tier 2 PSNR |
| --- | ---: | ---: |
| Area shadows | 35.917 dB | 48.877 dB |
| Reflections | 26.092 dB | 37.885 dB |
| AO | 17.223 dB | 35.230 dB |
| Diffuse GI | 22.981 dB | 33.915 dB |

First-frame linear beauty scores match to the precision shown on native Metal
and explicit compute. Tests cover exact Tier 1 fallback, effective SSR and SSAO
replacement, zero AO radius, non-shadow-casting lights, separate lighting AOVs
with all effects active, and no extra diffuse sky from GI misses. Successive
frames stabilize for all four effects, and camera changes reset history.
Five GPU-required hybrid and shader-translation runs passed, as did the explicit
compute run. Final gate: all passed (2,676 Rust tests, zero failures).
Vulkan hardware RT is deferred to brief 212.
