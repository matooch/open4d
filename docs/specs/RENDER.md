# RENDER — render API, CPU path tracer, lights, settings and outputs (briefs 135, 136, 137, 139)

Sources: published rendering literature only (path tracing with next-event estimation and
multiple importance sampling, binned SAH BVH, GGX microfacet model, Moller-Trumbore
intersection, Russian roulette); the observations `docs/parity/render-reference.md` (O30),
`docs/parity/material-renders.md` (O43) and `docs/parity/scene-defaults.md` (O29); our own
design (docs/roadmap/areas/K-rendering.md). The reference renderer is **not** a path tracer, so
pixel parity is impossible: parity is level C (section R14).

Conventions: world space is left-handed, +Y up, the camera looks along its local +Z with +X to
the right and +Y up (the reference exports are in this system). Lengths in scene units.
Angles in radians inside the API; the case files use degrees. Colours are linear RGB.
Everything is deterministic: the same snapshot, settings and seed give the same pixels
on any thread count.

## R1 Snapshot (`forge_render_api::Snapshot`)

Engines never see the live document. A snapshot is plain data:

| Field | Meaning |
|---|---|
| `meshes: Vec<MeshData>` | triangle meshes in object space: `positions`, `triangles` (indices), optional per-corner `normals` and `uvs` (3 entries per triangle, `uvs` are UVW) |
| `instances: Vec<Instance>` | `mesh` index, `transform` (object to world, 16 values column-major), `material` index, `object_id` |
| `materials: Vec<MaterialData>` | name, `graph: ShadingGraph` (forge-material IR, MATERIAL.md M3) and a `sampler` (`Arc<dyn TextureSampler + Send + Sync>`) that answers the IR's texture nodes |
| `lights: Vec<Light>` | section R8 |
| `camera: Camera` | section R10 |
| `environment: Environment` | `Background` (colour, or the environment itself) plus `Lighting` (`None`, `Constant`, `Sky`, `Image`) |

`Snapshot::validate()` rejects out-of-range mesh, material or triangle indices, non-finite
transforms and zero-sized cameras. Flattening (clones, instances, deformers) is the job of the
adapter that builds the snapshot; the render-bridge crate holds the scene adapter and the
tests hold a thin adapter from the golden OBJ files. Materials come from
`forge_material::Material::from_values` + `compile` until the document hosts them (brief 194).

## R2 Settings (`RenderSettings`)

`width`, `height` (pixels, >= 1), `samples` per pixel (>= 1), `max_depth` (path vertices,
default 5), `seed` (u64, default 1), `tile` (pixels, default 32), `filter` (`Box`, `Triangle`,
`Gaussian`) and `filter_width` (pixels, default 1 for box), `aovs` (the requested extra passes),
`default_light` (default true, R11), `clamp` (maximum value of one sample's contribution after
the first bounce, 0 = off), `region` (optional sub-rectangle; pixels outside stay transparent
black), `threads` (None = all). `validate()` returns `RenderError::Settings` for zero sizes,
zero samples, a region outside the image or a non-positive filter width.

## R3 Outputs and the AOV model

`RenderOutput { width, height, beauty: Image, aovs: BTreeMap<Aov, Image>, stats }`. Every
image is `forge_image::Image` (linear RGBA f32, straight alpha). Beauty alpha is the coverage of
the pixel by geometry (0 over the background colour, 1 inside).

`Aov` (ordered): `Depth` (distance along the camera ray to the first hit, in R, 0 where
nothing is hit), `Normal` (world-space shading normal, RGB = XYZ, facing the viewer),
`Albedo` (diffuse colour times level of the first hit), `Position` (world-space first hit),
`ObjectId` (instance `object_id + 1` in R, 0 for background), `MaterialId` (material index + 1),
`Shadow` (fraction of the lights' unshadowed contribution that reaches the first hit, 1 = fully
lit, averaged over lights weighted by their unshadowed contribution; 1 where there is no light or
no hit), `Diffuse`, `Specular`, `Emission`. The light split is by first scattering event:
`Emission` is what the first hit emits, `Diffuse` is everything gathered through the diffuse
lobe of the first hit (direct and indirect), `Specular` through its specular or transmission
lobes, so `Beauty = Emission + Diffuse + Specular (+ background)` per sample. Depth, Normal,
Position, ObjectId, MaterialId, Albedo are first-hit values averaged with the pixel filter and
weighted by coverage (they are un-premultiplied by alpha where alpha > 0).

## R4 Engine, progress, cancel

```
trait RenderEngine: Send + Sync {
    fn id(&self) -> &str;
    fn render(&self, snapshot: &Snapshot, settings: &RenderSettings, control: &Control)
        -> Result<RenderOutput, RenderError>;
}
```
`Control` carries a cancel flag (`Control::cancel()`, clonable) and an optional event callback.
Events: `Started { width, height, tiles }`, `Tile { x, y, width, height, rgba }` (finished
pixels of a tile, row-major, for progressive display), `Progress { done, total }` (tiles),
`Finished`. A cancelled render returns `RenderError::Cancelled`. Events may arrive from worker
threads, in any order; the final image does not depend on the order. Errors: `Cancelled`,
`Settings`, `Snapshot`, `Unsupported(String)`.

## R5 Image comparison (`forge_parity_harness::compare_image`)

`compare_image(expected: &ImageBuf, actual: &ImageBuf, opts: &CompareOptions) -> ImageReport`.
`ImageBuf { width, height, rgb: Vec<f32>, alpha: Option<Vec<f32>> }` (linear, row 0 at the top).

1. Size mismatch is an error (`ImageError::Size`).
2. **Exposure normalisation**: the gain `g = sum(a*e) / sum(a*a)` over all RGB samples (0 when
   `a` is all zero) scales `actual`; `exposure_gain` reports it.
3. `psnr` = `10 log10(1 / mse)` of the scaled actual against expected over all RGB samples, peak 1
   (infinite when identical; reported as `f64::INFINITY`).
4. **Silhouette**: a pixel is in the silhouette when its alpha (if the image has an alpha plane)
   or its largest RGB component is above `opts.silhouette_threshold` (default 0). `silhouette_iou`
   is |E and A| / |E or A| (1 when both are empty). `silhouette_mismatch_px(tol)` counts the pixels
   of either mask with no pixel of the other mask within `tol` pixels (Chebyshev), so a
   silhouette that is off by at most `tol` pixels everywhere reports 0. `ImageReport` carries the
   count for `opts.tolerance_px` (default 1) and the pixel counts of both masks.

## R6 CPU path tracer: geometry (brief 136)

- Per mesh a bottom-level BVH over triangles, per snapshot a top-level BVH over instance world
  bounds (two-level, so cloned instances share one BLAS). The builder is a binned SAH builder
  (16 bins, leaves of at most 4 primitives, median split when all centroids coincide).
- A ray is transformed into object space with the inverse instance matrix (the direction is not
  renormalised, so `t` is the same in both spaces).
- Triangles: Moller-Trumbore in `f64`, double sided, `t > 1e-9` after an origin offset of
  `1e-6 * scale` along the geometric normal (scale = scene bounds diagonal).
- Orientation: a mesh whose edges are all shared by exactly two triangles is *closed*, its
  outward side is found from the sign of its signed volume (so winding direction does not matter).
  Open meshes are thin double-sided sheets.
- Shading normals: the interpolated per-corner normals when present, else the geometric normal.

## R7 CPU path tracer: surfaces from the IR (brief 136)

At a hit the integrator builds a `ShadeCtx` (uvw interpolated from corner UVs, world position,
shading normal, view direction) and evaluates the compiled `ShadingGraph`:

- **Diffuse**: Lambert, `albedo = eval(diffuse.color) * diffuse.level`. No `color` channel = 0.
  (Oren-Nayar and diffuse falloff are **parity-pending**: they render as Lambert.)
- **Specular layer** (`reflectance`): a GGX microfacet lobe with Smith G (height-correlated
  uncorrelated form), weight `strength * eval(color)` (no Fresnel: the reference layer is an
  additive highlight, MATERIAL.md M2), roughness `r = width` (a fraction), `alpha = max(r, 0.02)^2`.
  Kinds `ggx`, `beckmann`, `phong`, `blinn_legacy`, `phong_legacy`, `ward` and the rest render with
  the same GGX lobe: the width-to-lobe mapping of the legacy kinds is **parity-pending**
  (O43 `mat-glossy`, `mat-rough`, `mat-metal` show width as the roughness control only).
  `metal` attenuation tints the lobe with the layer colour and removes the diffuse part by the
  strength (`diffuse *= 1 - strength`); additive adds to the diffuse.
- **Transmission** (`transparency`): the surface is a mixture `(1 - T) * opaque + T * dielectric`
  with `T = eval(color)` (brightness included). The dielectric is an ideal smooth interface with
  `ior`, Fresnel (Schlick-exact dielectric form) chooses reflection or refraction; total internal
  reflection follows `total_internal_reflection`. A closed mesh refracts; an open mesh is a thin
  sheet and passes the ray straight on (throughput `T`). Absorption, blurriness, additive mode,
  `fresnel_reflectivity` and `exit_reflections` are **parity-pending / ignored**.
- **Emission** (`luminance`): `eval(emission)`, one-sided on the shading normal side facing the
  viewer, added to the path when hit (it lights other surfaces only through BSDF sampling).
- **Bump**: central differences of the mean of the bump texture over two tangent directions of
  the triangle (step `1e-3 * scale`), `n' = normalize(n - 20 * strength * (dh/du t1 + dh/dv t2))`
  with a unit scale `k = 1`; the exact scale is **parity-pending** (O43 `mat-bump-noise` records
  only a picture).
- **Ignored with a documented reason** (the channel is compiled but has no render rule yet):
  `normal` map, `alpha`, `displacement`, `fog`, `environment` (material), `glow`, `diffusion`.

Every unsupported channel that is switched on is listed in `RenderStats.ignored_channels`.

## R8 Lights (brief 137)

`Light { kind, color, intensity, transform, falloff, falloff_radius, shadow, shadow_density,
shadow_color, radius, diffuse, specular, visible }` with the defaults of O29 (intensity 100 %,
colour white, falloff none, radius-decay 500, shadow none, density 100 %, shadow colour black).

| Kind | Shape |
|---|---|
| `Omni` | point at the transform origin (or a sphere of `radius` for soft shadows) |
| `Spot` | omni restricted to a cone about local +Z: full cone angle `outer`, smooth step between `inner` and `outer` (angles are full cone angles; **parity-pending**) |
| `Infinite` | parallel light travelling along local +Z, with angular diameter `angle` (default 0.5 deg) |
| `Area` | `Rectangle { width, height }` or `Disc { radius }` in local XY, one-sided toward local +Z (`two_sided` option), or `Sphere { radius }`; an `Object` shape is `Unsupported` |

Units: radiance factor. A white Lambert surface facing a light of `intensity = 1` with falloff
none and no angle of incidence shows 1; for an area light `intensity` is its emitted radiance, so
a white surface under a constant environment of radiance 1 shows 1 (the white furnace test).
`Falloff`: `None` (1), `Linear` (`max(0, 1 - d/r)`), `InverseLinear` (`min(1, r/d)`),
`InverseSquare` (`min(1, (r/d)^2)`) with `r = falloff_radius`; `Steps` is `Unsupported`.
Shadows: `None` (no shadow ray), `Hard` (one shadow ray to the sampled point), `Soft`
(the sampled point is drawn over the light's area, `radius` for omni and spot, `angle` for
infinite). Area lights are always sampled over their area and shadowed unless the mode is `None`.
An occluded sample contributes `unshadowed * (1 - density + density * shadow_color)`.
`diffuse` / `specular` switches mask the light's lobes.
Environment: `Constant`, `Sky { zenith, horizon, ground }` (vertical gradient on the world +Y
direction, smoothstep) and `Image` (equirect lat-long, linear, `rotation` about Y, `intensity`),
sampled uniformly over the sphere and combined with BSDF sampling by MIS.

Light selection: every light is sampled once at every path vertex (the lights are summed, not
chosen), so there is no light-selection noise.

## R9 CPU path tracer: integrator and sampling (brief 136)

Unidirectional path tracer, `max_depth` vertices, next-event estimation at every non-delta
vertex plus BSDF sampling, power-heuristic MIS between the two for area lights and the
environment (delta lights use NEE only), Russian roulette from depth 3 on throughput
(survival probability `clamp(max(throughput), 0.05, 1)`). Delta (mirror / refraction) events do
not use MIS.

Sampling: per pixel, sample `i`: the pixel position uses the 2D R2 sequence with a per-pixel
Cranley-Patterson shift; everything else uses a PCG32 stream seeded with
`hash(seed, x, y)` and advanced deterministically, so a pixel's value does not depend on which
thread or tile computed it. (A Sobol sequence and adaptive sampling are **not** implemented in
this brief; depth of field and motion blur belong to brief 138.)

Tiles (`settings.tile`) are rendered in parallel with rayon and assembled in tile order.

## R10 Camera model

`Camera { transform (camera to world), projection }`; `Projection::Perspective { focal_mm,
sensor_mm, film_offset: [f64; 2] }` or `Projection::Orthographic { width }`. The film gate
width `sensor_mm` spans the image width: the pixel focal length is `f_px = width_px * focal_mm /
sensor_mm`; the principal point is the image centre shifted by `film_offset` (fractions of the
image width, +x right, +y up). Pixel `(px, py)` (row 0 at the top, pixel centres at +0.5) maps to
the camera-space direction `((px - cx) / f_px, -(py - cy) / f_px, 1)`.
`Camera::from_hpb(position, [h, p, b], projection)`: `R = Rh * Rp * Rb` with, in this
left-handed system, `Rh` turning +Z toward +X, `Rp` turning +Z toward +Y (positive pitch looks
up, so the observed camera `P -15 deg` looks down), `Rb` turning +X toward +Y. Only the pitch
sign is observed (O30); heading and bank signs are **parity-pending**.

## R11 Default light

With `settings.default_light` and an empty `lights` list the engine adds an Omni of intensity 1
and no falloff at the camera position (a head light). The reference does not document its
default light's direction (O29 section "default light": not observed), so brightness is
**parity-pending** and the comparison does not use it.

## R12 Render settings, image sequences and formats (brief 139)

`OutputSettings { width, height, frame_start, frame_end, frame_step, frame_rate, path,
format, depth, alpha, straight_alpha, multipass: Separate | Multipart }` (defaults from O29:
frame range "current frame", step 1, rate 30, format name "TIF" with name template
`Name0000`, 8 bits, alpha off).

`frames(range, current, scene_range)` -> list of frame numbers: `Current` (one frame),
`All` (scene range), `Manual { start, end }`; `step >= 1`; `end < start` gives an empty list
(a `RenderError::Settings`).

Path tokens in `path`: `{frame}` (the number), `{frame:N}` (zero padded to N digits), `{pass}`
(`beauty` or the AOV name), `{name}` (the scene name). A template without `{frame}` for more than
one frame gets `{frame:4}` appended to the file stem, so frames never overwrite each other.
The extension comes from `format`: `png`, `tif`, `jpg`, `exr`, `hdr`.

Formats and depths (via `forge-image`): PNG and TIFF 8/16 bit, JPEG 8 bit, HDR 32 bit, EXR 16/32
bit. `alpha` adds the alpha plane to formats that have one; `straight_alpha` selects straight
over premultiplied colour. An invalid pairing (JPEG at 16 bit) is an error, never a silent change.

AOV output: `Separate` writes one file per pass with `{pass}` in the name (an AOV file path
without `{pass}` gets `_<pass>` before the extension); `Multipart` (EXR only) writes one file with
a layer per pass: beauty as `R G B A`, depth as `Z`, vector passes as `R G B`, id passes as `Y`.
`Multipart` with another format is an error.

CLI: `forge render <scene> --out <path> [--width W] [--height H] [--samples N] [--frame N |
--start A --end B --step S] [--aov depth,normal,...] [--multipart] [--seed N]` (docs/specs/CLI.md).
The CLI builds the snapshot from the scene's evaluated meshes with a default material, the default
light and a camera that frames the scene bounds (the scene has no camera or light objects yet).

## R13 Not done / follow-ups

Light and camera scene objects (node types and their registration, icon table), the material
hosted in the document (brief 194), object-shaped area lights, falloff `Steps`, Sobol sampling,
adaptive sampling, denoising, volumetrics, depth of field and motion blur (138).
