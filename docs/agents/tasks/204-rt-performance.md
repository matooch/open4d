# 204 · Real-time performance targets

Status: done (benchmark delivery; both Metal FPS targets missed; Vulkan pending) · Area: R · Depends on: 199, 201 · Observations: – (own design; material values from O29/O42/O43 through the shading IR) · Touches: render-rt, bench

## Goal
Benchmarks for the S10 scene (1 M polygons, 10 000 clones): Tier 1 at ≥ 60 fps and Tier 2 at ≥ 30 fps at 1080p on both reference machines: an NVIDIA RTX 3090 (Vulkan, Windows/Linux) and an Apple M5 Pro (Metal); frame-time breakdown per pass.

## Inputs
- Area design: [../../roadmap/areas/R-realtime.md](../../roadmap/areas/R-realtime.md)
- Decision D19 in [../../roadmap/DECISIONS.md](../../roadmap/DECISIONS.md)
- docs/specs/MATERIAL.md, SHADERS.md, RENDER.md

## Done when
- [ ] Benchmarks exist and report per-pass timings; the owner runs them on the reference machine (manual step) and records the numbers in the brief.
- [ ] No pass regresses by more than 10 % between runs on the same machine (bench check).
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md` if parity results change, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)

Implemented S10 as an opt-in release benchmark for 1920×1080, with one million
quads and 10,000 shared cube instances (2,120,000 rendered triangles). Each
median uses five frames after two warmups; complete frames are timed in a
separate run without profiling waits. Tier 2 enabled all four effects and used
actual Metal hardware ray queries. No golden data or reference app used.

Apple M5 Pro, Metal, release medians in ms (pass medians need not sum to the
independently measured frame median):

| Pass | Tier 1 | Tier 2 |
|---|---:|---:|
| Complete frame | 1163.566 | 4810.549 |
| Setup / packing | 1140.161 | 1116.222 |
| Shadow atlas | 7.937 | 8.016 |
| G-buffer | 5.328 | 5.734 |
| Hybrid primary | — | 8.250 |
| RT AO | — | 511.233 |
| RT shadows | — | 736.368 |
| RT reflections | — | 1199.621 |
| RT GI | — | 1203.294 |
| Hybrid upload | — | 21.270 |
| Lighting | 2.871 | 9.629 |
| Output | 27.044 | 22.728 |
| **FPS (target)** | **0.859 (60, missed)** | **0.208 (30, missed)** |

The largest Tier 1 cost is setup/packing. The largest additional Tier 2 costs
are GI and reflections. The benchmark does not change the rendering algorithms
or treat these measurements as meeting the targets.

Same-machine repeats on the same build improved whole-frame medians to
1132.779 ms and 4746.261 ms, but the check correctly exited nonzero for Tier 1
G-buffer (5.328 → 7.018 ms) and Tier 2 hybrid primary (8.250 → 10.583 ms).
These are observed run-to-run differences, not attributed to a code change.
The 10% rule remains unchanged and per-pass repeatability was not demonstrated.
The exact 10% boundary, incompatible identities, missing passes, and S10
fixture and settings are covered by automated tests.

Vulkan RT run pending on the owner's RTX 3090 (brief 212). No substitute result
is reported. Full gate passed: 2735 tests, zero failures.
