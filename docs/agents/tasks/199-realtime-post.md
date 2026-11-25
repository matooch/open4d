# 199 · Post effects and viewport Render mode

Status: done · Area: R · Depends on: 198 · Observations: – (own design; material values from O29/O42/O43 through the shading IR) · Touches: render-rt, viewport, ui

## Goal
Screen-space AO and reflections, bloom, temporal anti-aliasing and filmic tone mapping; a viewport display mode "Render" that uses render-rt.

## Inputs
- Area design: [../../roadmap/areas/R-realtime.md](../../roadmap/areas/R-realtime.md)
- Decision D19 in [../../roadmap/DECISIONS.md](../../roadmap/DECISIONS.md)
- docs/specs/MATERIAL.md, SHADERS.md, RENDER.md

## Done when
- [ ] Each effect has a toggle in the render settings and a test that it changes the image as intended (e.g. AO darkens a crease).
- [ ] Switching the viewport to Render mode and back works with undo-free UI state (egui_kittest).
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md` if parity results change, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)

Status: done

### What was built

The raster tier's post chain (RENDER.md R22, own design) and the viewport's Render display mode:

- `forge_render_api::PostSettings` on `RenderSettings` (SSAO, SSR, bloom, TAA, filmic tone
  mapping; every effect its own toggle, all off by default, validated). The CPU and compute
  tracers ignore the field, so R21's images and tests are unchanged.
- SSAO inside the lighting pass (it must scale the ambient term only): twelve fixed
  hemisphere samples read back through the G-buffer, `ao = 1 - strength * occluded/12`.
- SSR as a full-screen pass over the lit beauty: a geometric march of `ssr_steps` steps against
  the G-buffer, the first screen hit's radiance replacing the environment reflection weighted by
  `1 - smoothstep(0.1, 0.6, roughness)`.
- TAA as `taa_samples` Halton(2,3) subpixel-jittered subframes averaged in linear radiance; the
  frame uniform gained `JITTER_X/Y`, and the jitter is applied in both projections. No history
  across calls, so a render stays a pure function of its snapshot (R21.6).
- Bloom (bright pass, 9-tap separable Gaussian, σ = 8 px) and the ACES filmic fit (Narkowicz
  2016) with exposure, both on the readback; `post::tonemap` is the same function the test uses.
- `forge render --engine rt --post ssao,ssr,bloom,taa,tonemap` (CLI reference regenerated).
- `crates/ui`: `EditorState::display: DisplayMode::{Shaded, Render}` and `EditorState::post`, set
  from the View menu ("Display: Render"/"Display: Shaded" plus the five post checkboxes); UI
  state, so no undo entry and no scene change. In Render mode a pane shows a render-rt frame of
  its own camera (`render_mode::RenderView`, cached by scene revision/camera/size/post, capped at
  640 x 400), with the gizmo/HUD overlays left out.

### Result

- `crates/render-rt/tests/post.rs` (GPU, one device behind the lock, skips without an adapter):
  SSAO darkens the crease (202 pixels below 0.9 of the unoccluded level) while 4432 of 4757
  covered pixels keep >0.98 and the silhouette is identical; `ssao_strength = 0` is byte-identical
  to no AO. SSR turns the glossy floor's reflection of a bright red cube red (96 pixels, max red
  delta 71.7 against 0.007). Bloom lifts the background three pixels past a bright cube (0
  without bloom) and leaves a dim object byte-identical. TAA turns 0 fractional-coverage pixels
  into 3+ graded ones and keeps the covered count. Tone mapping maps every pixel by
  `post::tonemap(linear)` within 1e-4 and keeps the alpha; values above 1 land below 1.
  `PostSettings` validation rejects every out-of-range value; two renders are byte-identical with
  the default block and with every effect on.
- `crates/ui/tests/render_mode.rs`: the View menu switches to Render and back with the undo depth
  and scene revision unchanged; the post checkboxes are UI state; the frame of a pane camera puts
  an object up and to the right of the target in the top-right quadrant (no flip or mirror).
  Run 5x, 5/5 green.
- `apps/forge/tests/render.rs`: `--post` changes the image and an unknown effect is a usage error.
- `sh scripts/gate.sh`: see the commit message.

### Pinned (with the reason in R22)

- Bloom, TAA and tone mapping run on the readback (CPU), not as GPU passes: a few ms at preview
  size; a GPU implementation is a performance follow-up (brief 204). The post settings on the CPU
  and compute tracers are ignored (documented).
- SSR has no off-screen rays (falls back to the environment) and the `Specular` AOV does not
  include the SSR delta (the split pass is pre-SSR).
- SSAO is one bounce, twelve fixed samples, no denoise; ray-traced AO is brief 201.
- TAA has no cross-frame history or motion reprojection (that is Tier 3, brief 202); a render
  with TAA reports its subframes' events, and the returned image is their mean.
- The Render display mode renders synchronously; a background worker with latest-snapshot
  cancellation is the performance follow-up (brief 204). Without a device the pane says
  "Render unavailable".

### Notes for the planner

- `--post` is a new CLI flag, so `docs/manual/reference/cli.md` was regenerated.
- The viewport camera converts with `view().inverse() * diag(1, 1, -1)`: the viewport's view
  space looks along -Z and the raster camera space along +Z (R21.2). The test pins the resulting
  orientation (top-right quadrant), not just that an image appears.
