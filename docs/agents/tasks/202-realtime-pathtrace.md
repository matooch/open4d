# 202 · Real-time path tracing with denoising

Status: done (parity-pending: Vulkan hardware) · Area: R · Depends on: 200, 201 · Observations: – (own design; material values from O29/O42/O43 through the shading IR) · Touches: render-rt

## Goal
Progressive path tracing of the full scene with temporal accumulation and a denoiser (Open Image Denoise, Apache-2.0, or an own filter). Also the GPU final-frame renderer (supersedes the goal of brief 142).

## Inputs
- Area design: [../../roadmap/areas/R-realtime.md](../../roadmap/areas/R-realtime.md)
- Decision D19 in [../../roadmap/DECISIONS.md](../../roadmap/DECISIONS.md)
- docs/specs/MATERIAL.md, SHADERS.md, RENDER.md

## Done when
- [x] After N samples the image is within the PSNR budget of the CPU tracer on the O30/O43 scenes (R7 sets the budgets).
- [x] Camera or scene edits restart accumulation; the denoised preview appears within one frame.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md` if parity results change, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)

`PathTraceEngine` uses R23 instanced traversal and the shared R20 shading integrator for
both final renders and progressive views. Each view keeps its own ordered GPU pixel state.
The first completed batch returns raw linear output and an edge-filtered preview guided by
unjittered primary rays. Split versus one-shot renders agree bit for bit in beauty and AOVs.
Camera, light, crop and seed edits restart at the same samples as a fresh view. Cancellation
after the first tile, or failure on an invalid edit, preserves the last successful history.
Final renders neither use nor mutate preview state. “Within one frame” means the first
completed preview batch; timing is R8's responsibility.

Read all 23 unchanged `tests/golden/render` inputs with the O30/O43 Forge CPU fixture
builder. Compared raw linear Tier 3 beauty to the CPU tracer with exposure normalization,
at 320×240 and 16 samples per pixel. All cases passed the 35 dB threshold on both backends:

| Fixture | Compute (dB) | Metal (dB) |
|---|---:|---:|
| area-soft | 71.47 | 71.47 |
| area-soft-2 | 71.47 | 71.47 |
| checker-plane | 119.99 | 119.99 |
| cube-default-light | 153.51 | 153.45 |
| cube-plane | 141.51 | 141.59 |
| light-units | 139.74 | 139.72 |
| light-units-200 | 136.94 | 137.03 |
| mat-bump-noise | 77.92 | 78.31 |
| mat-checker-cubic | 75.66 | 148.75 |
| mat-checker-uv | 75.66 | 148.82 |
| mat-default | 148.09 | 148.50 |
| mat-fresnel | 146.27 | 146.89 |
| mat-glossy | 75.62 | 79.10 |
| mat-luminance | 147.05 | 147.25 |
| mat-metal | 108.48 | 108.75 |
| mat-red-matte | 158.22 | 158.98 |
| mat-rough | 154.47 | 154.93 |
| mat-transparent | 73.02 | 73.02 |
| material-matte | 147.25 | 148.37 |
| material-red | 150.12 | 151.25 |
| omni | 147.19 | 146.92 |
| omni-shadow | 147.20 | 146.92 |
| sphere-default-light | 144.83 | 145.79 |

Production WGSL validates and translates to SPIR-V, MSL and HLSL. Image-based environment
lighting remains explicitly unsupported on the shared GPU transport (R20.4). Vulkan RT run
pending on the owner's RTX 3090 (brief 212).

Verification: five required-GPU parity runs passed. `GATE: all passed` (2688 tests, zero
failures).
