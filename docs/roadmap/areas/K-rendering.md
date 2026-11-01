# K · Rendering and viewport

## Scope (v1)

- [ ] **Render API:**
  - scene snapshot (flattened instances, lights, camera, materials as IR);
  - settings;
  - outputs (beauty plus AOVs: depth, normal, albedo, position, object ID, material ID,
    shadow, diffuse, specular, emission);
  - progressive tiles, cancel, progress events.
- [ ] **CPU path tracer:**
  - BVH (SAH) over triangles, instancing (two-level BVH);
  - PBR BSDFs from the IR;
  - lights: point/omni, spot, infinite, area (rect, disc, sphere, object), sky/HDRI;
  - MIS, Russian roulette;
  - deterministic sampling (Sobol), adaptive sampling, AA filter;
  - depth of field, motion blur.
- [ ] **Render settings:** resolution, frame range, output path and format (PNG/EXR,
  8/16/32-bit), samples, max depth, multipass, alpha.
- [ ] **Picture viewer:** history, A/B compare, channels, exposure, save as. **Render
  queue.**
- [ ] **IPR:** interactive render in a viewport region or window.
- [ ] **GPU path tracer** (wgpu compute) on the same IR, with an interactive preview.
- [ ] **Viewport:**
  - PBR forward shading with lights and shadows (shadow maps);
  - AA (MSAA/FXAA);
  - display modes: Gouraud, quick, constant, hidden line, lines, wireframe, isoparms,
    box, skeleton;
  - 4-view layout (perspective, top, front, right);
  - safe frames; grid settings;
  - GPU instancing for clones; LOD for huge meshes.
- [ ] **Camera object:** focal length, sensor, FOV, film offset, ortho projections, DOF
  settings, render camera selection.
- [ ] **v2:** denoiser, toon/sketch, volumetric rendering of fog grids, hair rendering.

## Observation jobs

| Job | Form | Status |
|---|---|---|
| O30 | [render-reference.md](../../parity/render-reference.md): 10 reference renders (default light, omni/spot/infinite/area, shadows, two materials) | todo |
| O43 | material renders (see J) | todo |
| O29 | camera, light and render-settings defaults | todo |
| O57 | [camera.md](../../parity/camera.md): renders through cameras with different focal lengths, sensor sizes, film offset, ortho views; projection checked by rendering a calibration grid | todo |
| G8 | GUI-SESSIONS §G8: viewport display modes, 4-view layout, navigation feel, picture viewer | gui-only |

## Implementation design

- **Crates:** `crates/render-api` (`forge-render-api`), `crates/render-cpu`,
  `crates/render-gpu`. Each is behind `trait RenderEngine` (ARCHITECTURE.md).
- **Parity: level C, by PSNR.**
  - The reference's Standard renderer isn't a path tracer, so exact pixels are
    impossible.
  - Targets: PSNR ≥ 25 dB on the diffuse-only scenes after exposure normalisation,
    plus matching projection geometry. Silhouettes must match within 1 px, which proves
    the camera model.
  - The harness gets `compare_image(expected, actual) -> {psnr, silhouette_iou}`.
- **Determinism:** fixed seeds per pixel and sample, so CI renders are reproducible on all
  OS (no `f32` fast-math differences; use `f64` where needed, or tolerate within the
  PSNR budget).
- **Viewport:** extend `forge-viewport::render` (wgpu): add a light uniform buffer, a
  shadow-map pass, material uniforms from the IR's preview approximation, and an instance
  buffer for clones.

## Briefs

| # | Title | Depends on | Observations |
|---|---|---|---|
| 135 | Render API: snapshot, settings, outputs, AOV model, progress; harness `compare_image` | 123 | – |
| 136 | CPU path tracer core (BVH, instancing, BSDFs from the IR, MIS) | 135 | O30 |
| 137 | Lights (omni, spot, infinite, area, sky) and shadows | 136 | O30, O29 |
| 138 | Physical camera: projection, sensor, ortho, DOF, motion blur | 136, 047 | O57, O29 |
| 139 | Render settings, image sequences, formats, multipass/AOVs | 136, 130 | O29 |
| 140 | Picture viewer and render queue | 139 | G8 |
| 141 | IPR | 136 | – |
| 142 | GPU path tracer (wgpu compute) | 136 | O30 |
| 143 | Viewport upgrades: lights, shadows, AA, display modes, 4-view layout | – | G8 |
| 144 | Viewport performance: GPU instancing, LOD, large meshes | 143 | – |
| 145 | Render parity tests against O30/O43/O57 | 137, 138 | O30, O43, O57 |
| 146 | S8 acceptance test and rendering docs | 139, 145 | – |

## Done when
S8 passes. Reference renders meet the PSNR and silhouette thresholds. Renders are
identical across the three OS within 1 dB.
