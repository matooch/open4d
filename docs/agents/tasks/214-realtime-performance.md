# 214 · Make the real-time tiers real-time (S10 targets)

Status: in-progress (hermes) · Area: R · Depends on: 204 · Observations: – (own design) · Touches: render-rt, render-gpu

## Goal
Brief 204 measured the S10 scene (1 M polygons, 10 000 clones, 1080p) on the M5 Pro (Metal):
**Tier 1 0.86 fps (target 60), Tier 2 0.21 fps (target 30).** The benchmark exists
(`cargo bench -p forge-render-rt --bench s10`, PERFORMANCE.md); this brief meets the targets.

## Where the time goes (204's Report, ms per frame)
- Tier 1: **setup/packing 1140** of 1164 — the CPU re-packs triangles, instances and BVHs every
  frame; the GPU passes are ~45 ms together.
- Tier 2 adds RT AO 511, RT shadows 736, RT reflections 1200, RT GI 1203.

## Steps (spec first, measure after each step on the M5 Pro, record in the Report)
1. **Persistent GPU scene.** Pack meshes and BLASes once; keep them on the GPU across frames;
   per frame update only what changed (instance transforms → TLAS refit/rebuild; changed meshes
   re-packed), keyed by scene revision and per-object dirty state (CORE.md dirty/cache). Target:
   setup under 5 ms for a camera-only change, Tier 1 ≥ 60 fps.
2. **Tier 2 budgets.** Per effect: half- or quarter-resolution tracing with depth-aware upsample,
   1 ray per pixel per effect with temporal accumulation (reprojection + history rejection, the
   R25 preview filter where it fits), shorter rays for AO; spec each in R24 with its quality
   effect, and keep the R26 budgets (re-measure R26 after each change).
3. Re-run S10; repeat until both targets are met, or record what is left and why.

## Done when
- [ ] S10 on the M5 Pro: Tier 1 ≥ 60 fps, Tier 2 ≥ 30 fps (or a documented, owner-accepted gap).
- [ ] R26 cross-tier budgets still met (pins unchanged or reduced); all GPU tests pass on lavapipe and Metal.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Step 1 contract: [persistent scene data](../../specs/PERFORMANCE.md#61-persistent-scene-data-brief-214).
- Baseline gate: `GATE: 2735 tests passed, 0 failed`; `GATE: all passed`. No new S10
  measurements at the spec checkpoint.
- Step 1 checkpoint: cache the flat raster upload, shadow bounds and CPU object-ID lookup
  per engine/device. Reuse on exact mesh and instance input matches; revalidate references
  and camera each frame. This is not per-mesh instancing or BLAS reuse on mesh edits.
- Full-size S10, M5 Pro / Metal, unchanged 204 fixture and warm protocol, all four Tier 2
  effects enabled with actual `BackendKind::Metal`. Separate camera-only series via
  `FORGE_S10_CAMERA_SETUP=1` (alternating only camera X translation; same fixture).

  | Tier | Frame median (ms) | fps | Setup (ms) | Camera-only setup (ms) |
  | --- | ---: | ---: | ---: | ---: |
  | 1 | 40.845 | 24.483 | 6.972 | 6.960 |
  | 2 | 4586.251 | 0.218 | 7.443 | 7.274 |

  Neither fps target nor the camera-only setup target is met. Tier 1 output alone took
  29.338 ms in the profiled series. Tier 2 effects are still unbudgeted; do not interpret
  these runs as passing the per-pass 10% regression check.
- Cache acceptance passed five times with GPU required, including exact warm/cold
  beauty and AOV equality and invalid-input rejection. Checkpoint gate: 2736 tests
  passed, zero failed; `GATE: all passed`.
- [Output checkpoint](../../specs/PERFORMANCE.md#62-demand-driven-raster-output-brief-214):
  copy only requested attachments, retain hybrid primary readbacks, and construct
  cropped Tile payloads only for callbacks. Five required-GPU runs passed exact
  cropped beauty, AOV and event comparisons, with bloom and tone mapping enabled.
  Same full-size S10 protocol on M5 Pro / Metal:

  | Tier | Frame median (ms) | fps | Setup (ms) | Camera-only setup (ms) | Output (ms) |
  | --- | ---: | ---: | ---: | ---: | ---: |
  | 1 | 13.963 | 71.620 | 5.871 | 5.943 | 3.359 |
  | 2 | 4318.308 | 0.232 | 6.889 | 7.094 | 3.599 |

  Tier 1 met 60 fps in this run (previously 24.483 fps). Camera-only setup still
  exceeds 5 ms. Tier 2 used actual Metal with all four effects and missed 30 fps.
  This checkpoint does not change any effect estimator or R26 budget. Gate:
  2738 tests passed, zero failed; `GATE: all passed`.
