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
- **Raw reflection secondary lighting:** inline Compute and Metal matched
  the batched CPU-shaded reference at two history seeds for finite, area and
  infinite lights, per-light lobe flags and tinted occlusion. The near
  emitter matched its no-shadow control; the far emitter behind the same
  blocker received the expected RGB tint. SPIR-V, MSL and HLSL translation
  passed. This is still a standalone probe; no live reflection or S10 change.
- **Raw reflection dispatch probe:** inline Compute and Metal matched the
  batched CPU-shaded reference for emission, dark hits, and misses under
  None, Constant, Sky, and rotated linear lat-long image lighting. The probe
  also checked two history seeds and exact clears for uncovered pixels and
  invalid receiver keys. Five required-GPU lib, hybrid and translation runs
  passed; full gate: 2750 tests passed, `GATE: all passed`. Secondary direct
  light and live integration are pending; no new S10 or R26 result.

- [Raw reflection kernel checkpoint](../../specs/RENDER.md#r248-gpu-reflection-signal-brief-214):
  the standalone shader reconstructs the receiver and sums inline R23 hit
  emission or scene lighting on misses, including unfiltered lat-long
  image lighting. The complete Compute and hardware variants translated to
  SPIR-V, MSL and HLSL in five consecutive runs. Full gate: 2749 tests
  passed, `GATE: all passed`. Numeric dispatch acceptance and secondary
  direct light remain pending; no live Tier 2 change or new S10 measurement.

- [Reflection sampler checkpoint](../../specs/RENDER.md#r248-gpu-reflection-signal-brief-214):
  the standalone GPU sampler preserves the eight opaque GGX attempts, both
  hemisphere checks, and their RNG stream. Metal directions and RGB weights
  matched the CPU reference at minimum roughness and for a broad transmitting
  lobe; rejected attempts were zero and the final PCG word matched exactly.
  The probe translated to SPIR-V, MSL and HLSL. Five required-Metal runs
  passed; full gate: 2748 tests passed, `GATE: all passed`. Tier 2 still
  uses the batched reflection estimator; no new S10 measurement or R26 change.

- **GPU direct-shadow runtime:** Tier 2 now uses inline R23 visibility with
  cached R25 receiver attributes and the current raster bitmap views. It
  retains eight samples, the CPU spatial filter and per-jitter history, and
  falls back to batched visibility if inline dispatch is unsupported. Cropped
  warm and cold renders matched after geometry, material and bitmap edits.
  Replacing a bitmap in the same slot reused the shadow pipeline and attributes.
- Unchanged full-size S10, M5 Pro, actual Metal, all Tier 2 effects: the shadow
  pass fell from **838.808 ms** to **48.862 ms**; total frame median fell from
  **3829.127 ms / 0.261 fps** to **2734.881 ms / 0.366 fps**. Reflections
  (**1309.754 ms**) and GI (**1330.511 ms**) now dominate; Tier 2 still misses 30 fps.
- Strict R26 held **18/23**, **19/23**, and **23/23** against CPU on both Metal
  and Compute, with the same nine pins and Metal-vs-Compute minima of exact,
  165.12 dB, and 75.66 dB. Five required-GPU lib, hybrid and translation runs
  passed. Runtime checkpoint gate: 2746 tests passed, zero failed;
  `GATE: all passed`. Vulkan hardware was not exercised.

- [Raw direct-shadow dispatch probe](../../specs/RENDER.md#r247-gpu-direct-shadow-visibility-brief-214):
  bound the existing R23 traversal scene, R25 receiver attributes and shared
  packed lights. Compute and Metal matched the unfiltered batched reference
  across all eight components for cropped finite, area, infinite and mixed
  lights at two history indices. The probe also checked exact clears,
  finite-emitter clipping, tinted occlusion and light lobe flags. The
  required-GPU lib, hybrid and shader translation suites passed five times;
  full gate: 2746 tests passed, `GATE: all passed`. Tier 2 still uses the
  batched estimator, so there is no new S10 measurement.
- [Raw direct-shadow kernel checkpoint](../../specs/RENDER.md#r247-gpu-direct-shadow-visibility-brief-214):
  assemble a standalone inline R23 shader that reconstructs and shades each
  covered receiver, retains eight samples per prepared shadow light, and
  writes the two unfiltered signal rows. R20/R25 and the new kernel share
  the same material evaluation and BSDF code. Compute and hardware variants
  translated to SPIR-V, MSL and HLSL in five consecutive runs. The kernel is
  not dispatched by Tier 2 yet: GPU/CPU raw-signal comparison and new S10
  measurements remain pending. Full gate: 2745 tests passed, zero failed;
  `GATE: all passed`.
- [Raw receiver checkpoint](../../specs/RENDER.md#r247-gpu-direct-shadow-visibility-brief-214):
  shared R25 geometry rows reconstruct the R6 receiver from the raster key,
  world position and jittered pixel centre without a flattened attribute table.
  The Metal probe matched CPU `PrimaryGeometry` on synthetic cropped inputs,
  including clear and invalid keys, empty instances, shared mirrored/nonuniform
  instances and both projections. The receiver translated to SPIR-V, MSL and
  HLSL; the targeted suites passed five times. Strict R26 passed on Metal and
  Compute with unchanged pins. The receiver is not yet used for Tier 2 shadow
  dispatch; no new S10 measurement. Gate: 2744 tests passed, `GATE: all passed`.
- [Direct-shadow sampling checkpoint](../../specs/RENDER.md#r247-gpu-direct-shadow-visibility-brief-214):
  extracted the existing R20/R25 packed light sampler without changing its
  formulas. The standalone sampler and full transport translated through
  SPIR-V, MSL and HLSL in five consecutive test runs. Strict R26 passed
  with actual Metal and Compute and the same nine pins. No Tier 2 runtime
  change or new S10 throughput claim; receiver shading and inline dispatch
  remain open. Gate: 2742 tests passed, zero failed; `GATE: all passed`.
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
- [Compiled program checkpoint](../../specs/PERFORMANCE.md#63-compiled-raster-programs-brief-214):
  retain the raster pipelines and layouts while assembled WGSL and bitmap binding
  count match. Bitmap pixels remain live per frame. Five required-GPU runs passed
  the raster cache, hybrid, and shader translation tests; cached and fresh output
  compared exactly after camera, light, graph, and bitmap edits.

  | Tier | Frame median (ms) | fps | Setup (ms) | Camera-only setup (ms) |
  | --- | ---: | ---: | ---: | ---: |
  | 1 | 9.966 | 100.339 | 2.013 | 2.230 |
  | 2 | 4380.789 | 0.228 | 3.116 | 3.106 |

  Same full-size M5 Pro / Metal protocol; Tier 2 used actual Metal with all
  four effects. Tier 1 and camera-only setup meet their targets in this run.
  Per-mesh invalidation and the Tier 2 throughput target remain open. Gate:
  2738 tests passed, zero failed; `GATE: all passed`.
- [AO budget checkpoint](../../specs/RENDER.md#r244-reduced-rate-ambient-occlusion-brief-214):
  one finite-radius ray per rotating 2×2 representative, with independent
  rays where material, normal or depth cannot support reuse. The existing
  spatial filter and per-jitter history remain. Five required-GPU lib and
  hybrid runs passed, including recorded ray count, odd/cropped coverage,
  reconstruction, camera reset, fallback, and convergence.
- Full-size S10 Tier 2, all four effects and actual Metal on M5 Pro:
  AO **407.894 ms**, down from 622.286 ms (34.5%); uninstrumented frame
  **4204.666 ms / 0.238 fps**, down from 4380.789 ms. Its 30 fps target is
  still missed. This checkpoint changes no Tier 1 estimator.
- R26 O30/O43 matrix at unchanged settings: Tier 1 **18/23**, Tier 2
  **19/23**, Tier 3 **23/23** against CPU on both Compute and Metal;
  the same nine pins remain. Worst Metal-vs-Compute scores were exact,
  165.12 dB and 75.66 dB respectively. No Vulkan hardware run.
- AO checkpoint gate: 2739 tests passed, zero failed; `GATE: all passed`.
- [Visibility readback checkpoint](../../specs/RENDER.md#r245-visibility-only-readback-brief-214):
  AO and both types of shadow query return 4 bytes per ray instead of a
  32-byte hit record. Bounce rays still return full hits. Five required-Metal
  test runs passed, including visibility vs. closest-hit checks on Compute and
  Metal, and shader translation to SPIR-V, MSL and HLSL. The strict R26 matrix
  passed on both backends with the same nine pins.
- Full-size S10 Tier 2 on M5 Pro with actual Metal: AO **394.190 ms** and
  shadows **838.808 ms** (previously 407.894 and 864.245 ms). The uninstrumented
  median was **4238.415 ms / 0.236 fps**, slightly slower than 4204.666 ms;
  this run does not show a frame-rate gain. The change saves readback bytes
  but does not reduce traversal work or eliminate CPU/GPU synchronization.
- Visibility checkpoint gate: 2739 tests passed, zero failed; `GATE: all passed`.
- [GPU AO kernel checkpoint](../../specs/RENDER.md#r246-gpu-ambient-visibility-dispatch-brief-214):
  the raw visibility shader reads primary textures and runs inline traversal and
  quad reconstruction without a ray list. Five translation runs passed for
   Compute and hardware variants (SPIR-V, MSL and HLSL). At this checkpoint there
   was no host dispatch or new S10 or R26 result. Checkpoint gate:
   2740 tests passed, zero failed; `GATE: all passed`.
- **GPU AO runtime:** the engine binds its existing R23 scene and G-buffer directly
  and reads back one raw float per pixel for the unchanged CPU spatial filter
  and per-jitter history. It uses the batched R24.4 path if there is no inline
  scene binding or the output exceeds dispatch limits. Five required-GPU runs
  passed, including exact raw visibility on Compute and Metal across all four
  phases on an odd-sized fixture with cleared and incompatible neighbours.
- Full-size S10 Tier 2 on M5 Pro, actual Metal, all effects: AO **16.738 ms**
  (previously 394.190 ms); the uninstrumented median was **3829.127 ms /
  0.261 fps** (previously 4238.415 ms / 0.236 fps). CPU filtering and signal upload
  remain, along with CPU orchestration for the other effects. Tier 2 still misses
  30 fps.
- Strict R26 retained **18/23**, **19/23**, and **23/23** against CPU on both
  Compute and Metal with the same nine pins. Metal-vs-Compute minima were exact,
  165.12 dB, and 75.66 dB, respectively. No Vulkan hardware run.
- GPU AO runtime checkpoint gate: 2741 tests passed, zero failed; `GATE: all passed`.
