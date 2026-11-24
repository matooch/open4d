# MATERIAL — Standard material, texture tag, shading IR (brief 123)

Sources: O29 parameter table, [docs/parity/scene-defaults.md](../parity/scene-defaults.md) §4
(channel defaults, ranges, options; every default below is read from it); O43
[docs/parity/material-renders.md](../parity/material-renders.md) (which channels exist and what
the Width field of a reflection layer does; render parity is not part of this brief, there is no
renderer yet); O28 [docs/parity/uv-projection.md](../parity/uv-projection.md) (texture tag
projection list); area design [J-materials.md](../roadmap/areas/J-materials.md). Published
shading models: Lambert diffuse, Oren and Nayar 1994 (rough diffuse), Blinn 1977 and Phong 1975
(specular lobes), Walter et al. 2007 (GGX), Schlick 1994 (Fresnel), Snell's law. Nothing is
read from the reference app.

Crate `forge-material` (`crates/material`): `MaterialPlugin` registers two node types, and the
crate holds the shading IR. It depends on `forge-core` only.

## M1 Node types

- `forge.material.standard` (`Category::Material`, not in the Create menu): the Standard
  material. Parameters are `<channel>.<field>` ids (ids allow `[a-z0-9_.]`). Percent fields are
  stored as fractions (`1.0` = 100 %, `Unit::Percent`); distances in scene units; colours are
  linear RGB `Value::Color` in 0..1 as listed by O29. Hidden legacy rows of O29 (the old
  Reflection / Specular channels) are **not** registered; the O29 rows "Layer Setup" and
  "Project Scale" are not scalar and are not registered either.
- `forge.tag.texture` (`Category::Tag`): the texture tag (M4).
- Materials and shaders are hosted in the document since brief 194 (CORE.md section 2.1, M6): a
  material is an entry of the material list, its shaders are its children, the texture tag's
  `material` link and every texture slot are links. A material can still be built from a parameter
  map in code with `Material::new()` / `Material::from_values`.

## M2 Standard material parameters (from O29 section 4)

Defaults are exactly the O29 column; the test `defaults_match_o29` pins a sample of each group.

| Group | Parameters (id: default) |
|---|---|
| Basic | `icon_color` [1, .9, .4]; `enable.<channel>` for color (on), diffusion, luminance, transparency, reflectance (on), environment, fog, bump, normal, alpha, glow, displacement (all off) |
| Color | `color.color` [.8,.8,.8]; `color.brightness` 100 %; `color.texture` ""; `color.mix_mode` normal; `color.mix_strength` 100 %; `color.model` lambertian (lambertian, oren_nayar); `color.diffuse_falloff` 0 % (-1000..1000); `color.diffuse_level` 100 % (0..1000); `color.roughness` 50 % (0..200) |
| Diffusion | `diffusion.brightness` 100 %; `diffusion.affect_luminance` off; `diffusion.affect_specular` on; `diffusion.affect_reflection` off; `diffusion.texture`, `.mix_mode`, `.mix_strength` |
| Luminance | `luminance.color` [1,1,1]; `luminance.brightness` 100 % (0..1e11); texture + mix |
| Transparency | `transparency.color` [1,1,1]; `.brightness` 100 %; `.refraction_preset` glass (17 presets); `.refraction` 1.517 (.25..4); `.total_internal_reflection` on; `.exit_reflections` on; `.fresnel_reflectivity` 100 %; `.additive` off; `.absorption_color` [1,1,1]; `.absorption_distance` 100; `.blurriness` 0; `.min_samples` 5; `.max_samples` 128; `.accuracy` 50 %; texture + mix |
| Reflectance | `reflectance.global_reflection` 100 % (0..1000); `reflectance.global_specular` 100 %; `reflectance.separate_pass` off; default specular layer `reflectance.spec.type` (default `blinn_legacy`; options beckmann, ggx, phong, ward, anisotropic, lambertian, oren_nayar, irawan, reflection_legacy, blinn_legacy, phong_legacy), `.attenuation` additive (additive, metal), `.width` 50 %, `.falloff` 0 % (-100..100), `.inner_width` 0 %, `.strength` 20 % (0..10000), `.colored` off, `.bump_strength` 100 %, `.bump_mode` default, `.separate_pass` off, `.color` [1,1,1], `.color_brightness` 100 %, `.color_texture` "" + mix, `.mask_amount` 100 %, `.mask_color` [1,1,1], `.mask_texture` "" + mix |
| Environment | `environment.color` [1,1,1]; `.brightness` 100 %; texture; `.mix_mode` **multiply**; `.mix_strength`; `.tiles_x`/`.tiles_y` 1; `.exclusive` on |
| Fog | `fog.color`, `fog.brightness` 100 %, `fog.distance` 1000 |
| Bump | `bump.strength` 20 % (-1e6..1e6 as a fraction of percent), `bump.mip_falloff` on, `bump.texture` |
| Normal | `normal.strength` 100 %, `normal.method` tangent (tangent, object, world), `normal.flip_x/y/z`, `normal.swap_yz`, `normal.texture` |
| Alpha | `alpha.color` [0,0,0]; `alpha.delta` [.5,.5,.5]; `alpha.invert` off; `alpha.soft` on; `alpha.image_alpha` on; `alpha.premultiplied` off; `alpha.texture` |
| Glow | `glow.color`, `.brightness`, `.inner_strength` 100 %, `.outer_strength` 500 %, `.radius` 10, `.random` 0, `.frequency` 1, `.use_material_color` on |
| Displacement | `displacement.strength` 100 %, `.height` 10, `.type` intensity_centered (6 options), `.texture`, sub-polygon displacement group (`.subpoly`, `.subdivision_level` 4 (1..12), `.round_geometry`, `.round_contour` on, `.map_rounded` on, `.map_resulting` off, `.keep_edges` on, `.best_distribution` on) |
| Illumination | `illumination.*`: polygon light off, portal off, GI generate on 100 %, GI receive on 100 %, caustics generate/receive on 100 %, caustics radius 10, samples 100 (strength / saturation rows as in O29) |
| Viewport | `viewport.display` combined; `viewport.show.<channel>` on for each of the ten; `viewport.texture_size` default; `viewport.animate_preview` off |

## M3 Shading IR (`ShadingGraph`)

`compile(&Material) -> ShadingGraph` is pure and deterministic. The graph is a small DAG in an
arena (`Vec<IrNode>`, children before parents) plus output slots:

```
IrNode = Color(rgb) | Texture(name) | Scale(input, factor) | Mix { base, top, mode, strength }
Outputs = { diffuse: Option<Diffuse>, specular: Option<Specular>, emission: Option<NodeIx>,
            transmission: Option<Transmission>, environment: Option<NodeIx>, fog: Option<Fog>,
            alpha: Option<Alpha>, bump: Option<Bump>, normal: Option<NormalMap>,
            displacement: Option<Displacement>, diffusion: Option<Diffusion> }
```

Rules (published model; render parity is parity-pending until the renderer exists):

1. **Channel colour.** A channel with colour `c`, brightness `b` and a texture slot compiles to
   `Scale(Color(c), b)` when the texture is empty, else `Mix { base: Scale(Color(c), b), top:
   Scale(Texture(name), b), mode, strength }`.
2. **Mix.** `result = base + (op(base, top) - base) * strength` with `op` = `top` (normal),
   `base + top` (add), `base - top` (subtract), `base * top` (multiply), per component. Strength
   is the fraction 0..1. Subtract and the final colour are not clamped here (the renderer clamps).
3. **Enable boxes.** A disabled channel gives `None` and contributes nothing. Color and
   Reflectance are on by default; the default material therefore compiles to a diffuse lobe of
   `(0.8, 0.8, 0.8)` and one default specular lobe.
4. **Diffuse** carries `level` (`color.diffuse_level`), `falloff`, the model and `roughness`
   (used only by Oren-Nayar). **Specular** (the default layer) carries the type, width,
   falloff, inner width, strength (`strength * global_specular` for the Blinn / Phong legacy
   types and `global_reflection` for the physical types), the layer colour (rule 1 with the
   layer colour fields, mask applied as `Scale(mask_amount)`), attenuation and `colored`.
5. **Transmission** carries colour (rule 1), `ior` (`refraction`), absorption colour and distance,
   `fresnel_reflectivity`, the two total-internal-reflection flags and `additive`.
6. **Emission** is the luminance colour (rule 1). **Environment** and **Fog** as listed.
7. **Alpha**, **Bump**, **Normal**, **Displacement** carry their texture node, strength and
   flags. A bump or normal channel without a texture compiles to `None` (nothing to perturb).
8. **Interpreter.** `ShadingGraph::eval(ix, &dyn TextureSampler, &ShadeCtx) -> [f64; 3]` evaluates
   a node; `TextureSampler::sample(name, ctx)` is the shader hook (`forge-shaders` implements
   it). An unknown texture samples as white `(1, 1, 1)`.

## M4 Texture tag (`forge.tag.texture`)

Parameters: `material` (link, accepts Material, none); `selection` (string, restricts to a
polygon selection tag by name, empty = whole object); `projection` enum `spherical` (default:
O28 notes "Spherical is the default"), `cylindrical`, `flat`, `cubic`, `frontal`, `spatial`,
`uvw`, `shrink_wrap`, `camera`; `side` enum `both`, `front`, `back`; `mix_textures` off;
`offset_u`, `offset_v` 0 %; `length_u`, `length_v` 100 %; `tiles_u`, `tiles_v` 1;
`repeat_u`, `repeat_v` 1; `seamless` off; `position` (0,0,0), `rotation` (0,0,0) radians (heading, pitch, bank),
`size` (100,100,100) (the tag's own transform with its Size, O28: the projection space, UV.md U4). Projection maths is
in UV.md.

## M6 Hosting in the document (brief 194)

Level C (own design, no observation): the reference's Material manager is not observed (GUI
sessions G6/G7 pending), so every rule here is **parity-pending**.

- **Where.** `Scene::materials()` lists the document's materials in manager order; a shader tree
  hangs below its material (CORE.md section 2.1). `forge.material.standard` is created by
  `Tx::create_material`.
- **Texture slots are links.** The eleven slot params (`color.texture`, `diffusion.texture`,
  `luminance.texture`, `transparency.texture`, `environment.texture`, `bump.texture`,
  `normal.texture`, `alpha.texture`, `displacement.texture`, `reflectance.spec.color_texture`,
  `reflectance.spec.mask_texture`) are `Link` params accepting `Category::Shader`; the link names the
  root of that slot's shader tree, a child of the material (or deeper, below another shader). Before
  brief 194 they were strings (a shader name); no scene file ever held a material node (CORE.md
  section 6.11), so nothing is migrated. In code the string form is still read: `Material::from_values`
  accepts `Value::String(name)` and the IR then samples `ShaderSet` by that name.
- **Reading a hosted material.** `Material::from_node(node)` turns a link `Some(id)` into the
  texture name `#<id>` (`forge_material::shader_name(id)`); `forge_shaders::ShaderSet::from_material
  (scene, material, images)` builds every linked shader tree (SHADERS.md S11) under the same names,
  so `compile(&Material::from_node(..))` and the set plug together.
- **Texture tag.** Its `material` link (accepts Material) resolves to a list entry. Deleting the
  material clears the link in the same undo step (CORE.md section 2.1).
- **Commands** (COMMANDS.md section 6): `material.create`, `material.delete`, `material.assign`
  (adds a texture tag per object), `shader.set` (a shader of a type on a channel or as the input of a
  shader; replaces what the slot held), `uv.generate` (UV.md U6).
- **UI.** The Materials panel (`Panel::Materials`, opened from the Window menu) lists
  `Scene::materials()` with the icon colour swatch and the name; New runs `material.create`, a row's
  Assign button runs `material.assign` for the selected objects, Delete runs `material.delete`.
  Edit parameters in the Attributes panel through `param.set` / `shader.set`.

## M7 Material manager previews (brief 131)

Sources: [area J](../roadmap/areas/J-materials.md), [G7 §§1–2](../parity/GUI-SESSIONS.md)
and [RENDER §R1–R2](RENDER.md). No G7 answers are recorded; layout, lighting and drag
interaction are native and **parity-pending**.

- The Materials panel shows a wrapping card grid in document order. Its thumbnails are
  deterministic 64×64, four-sample CPU path traces of a smooth UV sphere in a fixed
  studio scene, using the compiled material and hosted shader tree. A single preview
  renders at a time off the UI thread. The icon colour stands in until a thumbnail is
  available; edits to the material or its linked shaders invalidate the thumbnail,
  and late results for deleted or changed materials are not installed.
- Drag a card's preview/name onto an Object Manager object row or a pickable viewport
  object, not onto empty space. Treat a hit on a clone as its cloner. A drop runs one
  `material.assign` command for that object only, with one undo step and no selection
  change. Assign-to-selection and Delete remain available.
- Tests exercise a real pointer drag and undo in `egui_kittest`, plus the CPU thumbnail's
  size, repeatability and response to a material colour change.

## M5 Unsupported / parity-pending

- No node materials (brief 124), no
  Layer Setup (a list of reflectance layers: only the default layer is modelled).
- O43 render parity: not tested (level C needs a renderer); the IR is checked structurally.
- All IR shading rules are **parity-pending**: they follow the published models.
