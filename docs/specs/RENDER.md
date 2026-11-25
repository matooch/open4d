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
`Gaussian`, `Cubic` = the cubic B-spline) and `filter_width` (pixels, the full support width,
default 1; the reference's render settings record a Cubic filter of width 2), `aovs` (the requested extra passes),
`default_light` (default true, R11), `clamp` (maximum value of one sample's contribution after
the first bounce, 0 = off), `region` (optional sub-rectangle; pixels outside stay transparent
black), `threads` (None = all). `validate()` returns `RenderError::Settings` for zero sizes,
zero samples, a region outside the image or a non-positive filter width.

## R3 Outputs and the AOV model

`RenderOutput { width, height, beauty: Image, aovs: BTreeMap<Aov, Image>, stats }`. Every
image is `forge_image::Image` (linear RGBA f32). Beauty is **premultiplied**: alpha is the coverage of
the pixel by geometry and the colour includes the background (so a black background gives
the usual premultiplied image); the AOV images are straight with alpha = coverage.

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
  the triangle (step `1e-3 * scale`), `n' = normalize(n - strength * scale * (dh/dt1 t1 + dh/dt2 t2))`
  with `scale` the scene bounds diagonal; the exact scale is **parity-pending** (O43 `mat-bump-noise` records
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
image width in x, of the image height in y; +x right, +y up). Pixel `(px, py)` (row 0 at the top, pixel centres at +0.5) maps to
the camera-space direction `((px - cx) / f_px, -(py - cy) / f_px, 1)`.
`Camera::from_hpb(position, [h, p, b], projection)`: `R = Rh * Rp * Rb`, each factor the transpose
of the usual right-handed rotation (what the observed left-handed reference does): `Rh` turns +Z
toward -X, so a heading of -15 deg looks to the right (O30: the objects sit left of the image
centre), `Rp` turns +Z toward +Y (positive pitch looks up) and `Rb` turns +X toward -Y. Only
the heading sign is observed (O30); pitch and bank follow from the same convention and are
**parity-pending**.

**Finding (brief 047): the reference's camera heading sign is opposite the usual right-handed
one.** The node transform of every object is `Ry(-h) Rx(-p) Rz(-b)` (TRANSFORM.md), which is
exactly `hpb_matrix`, so a camera *object* needs no conversion: its world matrix is the
camera-to-world matrix (SCENE-OBJECTS.md S2, test
`heading_sign_is_the_references_not_the_right_handed_one`). Do not "fix" the sign when adding
look-at helpers or exporters: a heading of -15 degrees must look to the right. The case files' `camera_rotation_deg = [-15, 0, 0]` is therefore
`[heading, pitch, bank]`: the form's prose calls it "P -15 deg", which the pictures contradict
(with pitch -15 and heading 0 the cube would be centred horizontally; see R14).

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
The CLI builds the snapshot from the scene's evaluated meshes with a default material and a
camera that frames the scene bounds; since brief 047 the scene's camera, light, environment, sky and
floor objects are applied on top (SCENE-OBJECTS.md S5: the first camera replaces the framing
camera, light objects replace the default light, the fallbacks are unchanged when the scene has
none) and the scene's stored render settings (S6) are the defaults of `--width`, `--height`,
`--samples`, `--max-depth` and `--seed`, which override them.

## R14 Parity with O30 / O43 (level C)

Source: `tests/golden/render` (19 folders, 320 x 240, `render.png` + `mesh.obj`),
`docs/parity/render-reference.md`, `docs/parity/material-renders.md`. Test:
`crates/render-cpu/tests/render_parity.rs`. Only geometry is compared. The test adapter builds the
snapshot from the case file: the OBJ groups whose name the case lists as an object (the mat-*
OBJs also hold a plane that the case file does not list and the pictures do not show), the
lights, a Standard material built with `Material::from_values` (checker, fresnel and noise
shaders from `forge-shaders`), the camera `[heading, pitch, bank]` at 36 mm equivalent focal
length. A sphere with `render_perfect` is rebuilt as a 96-segment sphere with the UV layout of
UV.md U2 (the OBJ is its 16-segment editor tessellation, 1.1 px inside the true silhouette); texture
tag projections other than UVW use `generate_uvw`.

Findings:

- **Camera.** With the documented camera (36 mm over a 36 mm gate, centred, heading -15 deg) every
  non-black silhouette lands within 1 px. A pitch of -15 deg with heading 0 (the form's wording)
  cannot be fitted: it needs a principal point at (71, 210) and a 5 % anisotropy. The cube of
  `cube-default-light` is symmetric about x = 70, as heading -15 deg predicts.
- **Silhouette definition.** The reference applies a Cubic filter of width 2, so bright edges are
  surrounded by a halo of up to two pixels at a few percent. The reference silhouette is therefore
  "any channel above 0.05 linear" (the largest value that keeps the dark edges, the smallest that
  cuts the halo); ours is `alpha >= 0.5` (half coverage) rendered with the same filter, 16 samples.
- **Result**: 15 of 19 cases match with 0 pixels beyond 1 px: cube-default-light, cube-plane,
  sphere-default-light, omni, omni-shadow, material-matte, material-red, mat-default,
  mat-red-matte, mat-glossy, mat-rough, mat-transparent, mat-luminance, mat-fresnel,
  mat-bump-noise.
- **KNOWN_MISMATCH** (builds, differs; asserted): `area-soft` (the case file says
  `light_type = "Infinite"` with rotation 0 while the form says Area light: the +Z-travelling
  light leaves the plane dark, 1500 px; observation needs checking), `mat-checker-uv` and
  `mat-checker-cubic` (the reference's black squares are exactly 0, so its luminance silhouette
  is not the object's, 557 / 130 px; and the pattern, compared through the albedo AOV, has the
  same grid size but a different phase or orientation: 2778 / 1059 px. Hypotheses tried: shader
  colour order inverted (worse), projection defaults. Open: the checker's UV origin and
  orientation on the reference sphere).
- **UNSUPPORTED**: `mat-metal` (silhouette not observable: only 725 px of the light's reflection
  are above black).
- **Not observed**: `spot` and `infinite` (black renders, the folders were never committed): the
  test asserts the folders stay absent; "observation needs redoing".
- **Brightness is not asserted.** The PSNR after exposure normalisation is 13 to 29 dB (printed by
  the test): the reference's shading (whole spheres lit frontally although the Area light stands at
  (300, 400, -300), brightest pixel 188/255 under the default light) does not follow the light
  objects the forms describe, so the lighting model is **parity-pending** until the default light
  and the light units are observed.

### R14a Render parity inventory (brief 145)

Sources: [O30](../parity/render-reference.md), [O43](../parity/material-renders.md)
and [O57](../parity/camera.md). The CPU render tests share one measurement path
for exposure-normalised PSNR and the existing silhouette rules. A case's explicit
`[tolerance].psnr` is its minimum PSNR in dB; a missing setting is diagnostic only
(none of these captures records a PSNR floor). Do not use the mesh manifest's default
PSNR as if it were an observed limit.

All 23 `render` and 8 `camera` archives are rendered and measured. The silhouette
gates remain zero disagreements beyond one pixel, or the one-sided coverage and
bounding-box check for O57 calibration frames. Compared at these gates: 17 render
cases and 7 camera cases pass. The existing `area-soft`, `area-soft-2`,
`mat-checker-uv`, `mat-checker-cubic`, `checker-plane` and `ortho-zoom-2` pins
remain; `mat-metal` reports tone only because the reflection does not bound the
object. Unavailable captures stay separately listed and are not counted as passes.

## R15 Camera parity with O57 and the thin lens (brief 138)

Source: `tests/golden/camera` (4 observed cases of 320 x 240, `render.png` + `mesh.obj`),
`docs/parity/camera.md` (O57), `docs/parity/scene-defaults.md` section 2 (O29). Test:
`crates/render-cpu/tests/camera_parity.rs`. Each case's camera table fills a `forge.camera`
object (SCENE-OBJECTS.md S2); the bridge (`apply_scene_objects`) turns it into the snapshot
camera; the scene is the OBJ of the case (a horizontal plane at y = 0 and a 200 unit cube
standing on it at z -200..0; the plane is edge-on from the camera heights used, so in the
perspective cases the visible shape is the cube). Compared like R14: silhouette within 1 px
(reference: any channel above 0.05; ours: coverage >= 0.5, Cubic filter of width 2, 16 samples).

Result: **4 of 4 exact** (0 pixels beyond 1 px): `focal-18`, `film-offset`, `ortho-front`,
`ortho-top`; plus the four **calibration-scene** follow-ups `default-2`, `aspect-square-2`,
`film-offset-y`, `ortho-zoom-2` (rotation (0, 0, 0), a checker plane P 90 and a white cube) compared
one-sided (`ortho-zoom-2` is `KNOWN_MISMATCH`, below): the checker's black cells are zero in the reference picture, indistinguishable from the
background, so its silhouette is not the geometry. The test requires every lit reference pixel to lie
within 1 px of a pixel we touch (coverage above 0.05, the reference's own threshold) and the lit bounding
boxes to agree within 1 px (`default-2` (53, 13)-(267, 227), `aspect-square-2` (40, 40)-(200, 200)); our
coverage of the black cells is not checked. The checker itself is not reproduced (the reference's period-16 px
ramp 0, 0.63, 1, 0.83 at 240 px is blurred; ours is binary). `focal-100-2` is not observed (one white
colour). Rules and the case that proves each:

- Lens from the horizontal FOV: `focal = sensor / (2 tan(fov/2))`, pixel focal `w * focal /
  sensor` (`focal-18`: 90 deg, `film-offset`: 53.13 deg).
- The camera rotation `[45, -19.45, 0]` of the case files is real: the camera looks left and down
  and the scene appears at the upper right (the heading sign of R10; `focal-18`, `film-offset`).
  The empty frames of the unobserved cases (`default`, `focal-100`, `aspect-square`) are this
  rotation, not a wrong plane placement as the form's notes guessed: at 36 mm and 100 mm the scene
  lies outside the frame (every cube corner projects beyond x = 345 px at 36 mm, and the lens
  narrows further at 100 mm). Those cases are correct black pictures, not an observation problem; their
  folders stay absent because black frames carry no information.
- **Film Offset X +20 % moves the picture left** by 20 % of the image width (64 px at 320 px):
  the lens's principal point sits at `w/2 - offset * w`. The parameters shift the frame, the
  lens moves the principal point the opposite way (`film-offset`: the cube's lower-left corner
  lands at x = 281 px, which is only true for a principal point at 96 px). **Film Offset Y +20 %
  moves the picture up by 20 % of the image height** (`film-offset-y`: 48 px at 240 px, the lit
  rows span 0..179 instead of 13..227): the Y parameter is not negated and scales with the height
  (at 4:3 a width-based reading would also give 48 px: only the height form is natural for the
  film gate; parity-pending for other aspect ratios). The earlier guess (down, by width) was wrong.
- Axis projections (`Front`, `Top`) are orthographic, ignore the camera's rotation and the
  orthographic width is **1000 units** at zoom 1 (`ortho-front`: IoU 1.0000 at zero tolerance
  with 1000, 0.9624 at 1016, 0.9385 at 1024; `ortho-top`: within 1 px at 1000. Its silhouette is
  one pixel smaller on two sides than a full plane at zero tolerance because the checker's
  black squares sit on the plane's edge, so it cannot fix the scale better than 1.6 %). The
  width at other zoom values is **not** `1000 / zoom`: `ortho-zoom-2` (Zoom 2) shows the plane
  at 35.0..285.6 px (250.6 px for 400 units) and the white cube 125.3 px wide for 200 units, i.e. a width of
  about 510.8 units where `1000 / zoom` gives 500 (plane 31..288 px, 7 px too wide): pinned
  `KNOWN_MISMATCH` (three readings tried: `1000 / zoom`, `1024 / zoom` = 512, which fits this case within
  0.6 px but breaks `ortho-front` (IoU 0.9385 at zero tolerance), and a power law `zoom^-0.97`; a third zoom
  value is needed). `aspect-square-2` (240 x 240) matches with the horizontal FOV kept, so the aspect ratio does
  not change the horizontal field.

**Thin lens (depth of field), own design.** `Camera::aperture = Some(Aperture { radius,
focus_distance })` and `RenderSettings::depth_of_field` (default false; stored as
`RenderConfig::depth_of_field`, `forge render --dof`, command `render.settings`). When on, the
camera ray of a sample starts at a point of the lens disc (concentric map of one `f2()` draw of the
pixel stream; pinhole renders draw nothing, so their pixels are unchanged) and aims at the point
where the pinhole ray crosses the plane `focus_distance` ahead. The scene object gives
`radius = focal_mm / (2 f-stop) * 0.1` (focal lengths are millimetres, scene units centimetres)
and the focus distance parameter. Orthographic cameras ignore the lens. Test `camera_lens.rs`:
the lens rays of a pixel meet on the focus plane, and the blur matches the circle of confusion:
a sphere of radius r at distance z with focus F and lens radius a has per-axis standard deviation
`sqrt(r^2 + c^2) / 2 * f_px / z` with `c = a |F - z| / F` (within 6 %, four distances).
The reference's Physical renderer could not be switched on by script (`dof` case: "parameter
set failed"), so DOF has **no parity data**: bokeh shape (blades, bias, anisotropy), the
f-stop/exposure coupling and the Standard renderer's DOF map are not modelled.

**Not done: motion blur.** It needs the snapshot at two times (shutter open and close) and the
shutter angle/offset semantics, none of which is observed; `shutter_speed`, `shutter_angle` and
`shutter_offset` are stored on the camera object (SCENE-OBJECTS.md S2) and ignored. Lens
distortion, vignetting and chromatic aberration are likewise stored and not rendered.

## R16 Environment images (brief 133)

`Lighting::Image` is a lat-long image: pixel `(x, y)` of a `w x h` image covers
`u = x / w`, `v = y / h` with azimuth `phi = u * 2 pi + rotation` about +Y (`d = (sin t sin phi,
cos t, sin t cos phi)`) and polar angle `t = v * pi` from +Y (nearest pixel, radiance
`pixel * intensity`). The scene object feeds it from an HDR/EXR file (SCENE-OBJECTS.md S4).

**Importance sampling.** The tracer builds one `EnvDist` per render: a piecewise constant density
over the pixels proportional to `luminance * sin(t)` (Rec. 709 weights), sampled through the
cumulative row weights and then the cumulative pixel weights of the row, with the point inside the
pixel taken from the remainders of the two uniform numbers. Its solid angle density is
`p_pixel * w * h / (2 pi^2 sin t)`. Next-event estimation draws from it and the MIS weight of a
BSDF-sampled ray that escapes uses the same density (`env_pdf`), so the estimator stays unbiased;
constant and sky lighting still sample the sphere uniformly. An image with no light in it
(all black) falls back to uniform sampling.
Tests (`crates/render-cpu/tests/environment.rs`): the density integrates to 1 over the sphere
and each sample reports the density of its own direction; a white sphere in a constant image
shows 1 (furnace); **a sphere under a dim sky image with a small sun 330 times brighter renders at
64 spp within 32.8 dB of a 4096 spp render** (the goal was 30 dB; with the uniform sampling it was
15.8 dB, measured by switching the distribution off once).

Not done: importance sampling by the BSDF at the same time as the image (the MIS pairing is
there, but the BSDF lobe is not importance sampled against the environment), an HDRI mip chain for
filtered background display, sun/sky models other than the gradient of `forge.sky`.

## R17 Picture Viewer and render queue (brief 140)

Sources: [area K](../roadmap/areas/K-rendering.md), [G8 §3](../parity/GUI-SESSIONS.md)
and the existing R4 render control. GUI behaviour is native and **parity-pending**
until G8 is recorded.

- Open **Picture Viewer** from the Window menu. Queue the current frame, capturing
  the scene and render settings at submission. One background worker at a time evaluates
  the captured frame, builds the render snapshot (including scene cameras, lights and
  supported materials), and runs the CPU engine. Progress callbacks wake the UI.
  Jobs run FIFO; removing a waiting job or cancelling the active job adds no history.
  Render failures are shown without stopping later jobs.
- Completed frames appear newest-first in bounded session history. Selecting a row shows
  its beauty; **Set A** and **Set B** pin the selected row. **Show A/B** switches between
  pinned renders without rerendering and keeps the same channel where available. Data
  passes use display mappings; **Exposure** adjusts colour passes only. **Save PNG**
  saves the displayed RGB view, not a replacement for the CLI's raw EXR/multipass export.
- Capture beauty by default; an opt-in **All passes** checkbox captures every AOV.
  Limit the queue and history to eight entries each, with a 128 MiB linear image
  budget and a maximum edge of 4096 pixels for an individual render. The viewer is
  for stills; CLI sequence rendering is unchanged.
- Tests: real queued CPU renders appear in egui history; select the two renders,
  pin/switch A and B, and exercise channel selection without remaking the render.

## R18 Interactive render preview (brief 141)

Sources: [area K](../roadmap/areas/K-rendering.md), R4 tile/cancel control and the
brief's snapshot-invalidation requirement. This is a native preview, not reference parity.

- **Interactive Render** is a viewport-area dock tab. While shown, it renders the
  current evaluated frame, using the same materials, camera and lights as R17.
  Cap the longest edge at 512 px, samples at 16 and tiles at 16 px to keep the
  preview responsive. It does not add to Picture Viewer history.
- The document identity, revision and timeline frame identify the submitted scene snapshot.
  A changed key clears accumulation and cancels the old generation; coalesce edits
  into a single desired snapshot and start it when the previous worker exits.
  Only the current generation may display tiles. Hidden tabs cancel their render;
  Stop/Start pauses and resumes without changing the scene or history.
- Render and evaluate off the UI thread. Blit finished tiles onto a bounded preview
  buffer and wake the UI, then show the engine's final output on success. The
  restart-latency test uses an injected engine to measure cancellation and start
  of the new render within 100 ms of a parameter edit, independent of CPU shading cost. On shared CI runners wall-clock time includes scheduling noise, so the test makes five edits
  (each must cancel, reject the late tile and restart) and checks the best restart against
  the 100 ms budget; the stub render never finishes by itself, so only the cancel path can
  restart it.

## R19 S8 look-dev acceptance (brief 146)

Sources: [roadmap README S8](../roadmap/README.md), [area K](../roadmap/areas/K-rendering.md).
This is a scenario test through the CLI, not a new render rule.

- Build the scene only with `forge new` / `forge cmd ... --save`: a cube, one Area light
  (`type: area`, rectangle, size), and a physical `forge.camera` with a short focus
  distance and a small f-stop so the thin lens is on. The first camera in tree order is
  the render camera and the first light replaces the default light (R11, S5).
- Give the cube a Standard material whose **color** slot is a `forge.shader.noise` node
  and whose **bump** slot is a `forge.shader.fresnel` node, both hosted by
  `shader.set`. The node-graph material (`forge.material.node`) has no noise or fresnel
  node definitions, so the scenario uses the shader node tree; this is recorded as a
  deviation, not a new rule.
- Render 96×72 at 8 samples and depth 3 with `--dof --aov all`; every pass and the beauty
  are written to `{pass}.png` (R12). A second render with the same seed is byte-identical.
  Changing only the focus distance changes the beauty (the aperture is live).
- Checks: beauty is framed (lit but not full) and not uniform; every AOV file exists and
  decodes; depth/normal/albedo are non-black where geometry is; the object and material
  id passes hold background 0 plus at least one non-zero id.
- Cross-OS: `tests/acceptance/s8-reference.png` is the checked-in macOS beauty.
  The same test runs in the CI matrix on macOS, Linux and Windows, and each render is
  compared with the reference. Each colour channel's mean linear value must stay within
  **1 dB** (`20 log10(mean/ref)`), and PSNR must be at least 30 dB. Set
  `FORGE_S8_WRITE_REFERENCE=1` to regenerate the reference when the scene changes
  intentionally.

## R20 GPU compute path tracer (brief 142)

`forge-render-gpu` is a second `RenderEngine` (`id` = `forge.render.gpu`) that runs the R9
integrator on a wgpu compute pass. It is **not** a new renderer: it reproduces the CPU tracer's
per-sample decisions so the two images agree. It is skipped when the machine has no adapter and
required (`FORGE_REQUIRE_GPU=1`) in the gate.

### R20.1 Scene upload

- Geometry is flattened **world space**: every instance's triangles are transformed on the CPU
  (positions, corner normals, corner UVWs), so the shader has one triangle list. The winding
  normal is `e1 x e2` after the instance's inverse-transpose, flipped when the mesh is closed and
  has negative signed volume (R6, `Blas::flip`). Instance/material ids ride along per triangle
  (`ObjectId`, `MaterialId` AOVs use the CPU + 1 convention).
- The BVH is the same binned-SAH tree as R6 over the world triangle boxes: node `(bounds,
  first, count)`, leaf `count > 0`, otherwise `first` = left child and `right = first + 1`.
  The shader walks it with an explicit stack, nearest child first, and the same slab test.
- Lights are prepared by `forge_render_cpu::lights::prepare` (one implementation of R8) and
  packed: kind tag, position, axis, colour, tangent/bitangent, falloff (kind + radius), shadow
  (cast, density, colour), radius, diffuse/specular flags, `visible`, spot half angles, area shape
  (kind, width/height or radius, two-sided).
- Materials are `forge_material::wgsl::compile` modules (R1 of REALTIME.md): one `forge_surface`
  per material (renamed per slot, sharing the first prologue). Shader-node (`Texture`) channels
  are **not** ported in this brief, so a graph with one returns
  `Unsupported("shader-node material channels on the GPU (REALTIME.md §2, brief 207)")`.

### R20.2 Sampling and integrator parity

- The pixel seed is `splitmix(seed ^ splitmix((x << 32) | y))` and the per-pixel stream is
  PCG32, both implemented in WGSL with 64-bit emulation over two `u32` words (shift-add
  multiply). `Rng::f()` is `((hi << 21) ^ lo) / 2^53`, evaluated as `hi / 2^32 + lo / 2^53` and
  rounded to `f32`, i.e. the CPU value to ~1e-7. The AOV/beauty sample loop, the R2 pixel
  sequence, `filter_offset` (all four filters, including the cubic's extra uniforms), the DOF
  lens draw and `trace_path` (area-light hits, emission, NEE + MIS, dielectric branch,
  Russian roulette at depth 3, `clamp`) keep the CPU's order of RNG calls.
- Arithmetic is `f32`; the acceptance rule is the image comparison of R20.3, not bit identity.
- Tiles: one dispatch per render tile, `(tile.width, tile.height, 1)` invocations of 1 pixel
  each; `Started`, `Tile`, `Progress`, `Finished` and `Control::cancel` behave as R4 (cancel is
  checked between tiles, a cancelled render returns `RenderError::Cancelled`).

### R20.3 Parity rule

Test `crates/render-gpu/tests/cpu_vs_gpu.rs`: for every O30 case whose materials have no
shader-node channel, render `CpuEngine` and `GpuEngine` with the R14 settings (320 x 240,
16 spp, cubic filter width 2) and require **PSNR >= 35 dB** between them (beauty, linear RGB,
`compare_image`). The five shader-node cases (`mat-checker-uv`, `mat-checker-cubic`,
`mat-fresnel`, `mat-bump-noise`, `checker-plane`) are listed in `UNSUPPORTED` with the reason
above and asserted to return `Unsupported`.

### R20.4 Unsupported on the GPU (with the reason)

- materials with `Texture` channels (REALTIME.md §2 is part 2, brief 207);
- an image environment (importance sampling of R16 needs the image upload; the CPU path keeps
  it);
- everything `prepare` already rejects (falloff `Steps`, object-shaped area lights);
- `threads`, `tile` and the region are CPU-side scheduling only; a `region` renders its own
  rectangle into the full-size image exactly as R2.

## R13 Not done / follow-ups

The material hosted in the document (brief 194), object-shaped area lights, falloff `Steps`, Sobol sampling,
adaptive sampling, denoising, volumetrics, motion blur and the camera's lens effects (R15).
