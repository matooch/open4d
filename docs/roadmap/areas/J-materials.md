# J · Materials, shaders, UVs

## Scope (v1)

- [ ] **Standard material** (the reference's classic channel material):
  - Color, Diffusion, Luminance, Transparency, Reflectance (layers: specular, GGX
    reflection), Bump, Normal, Alpha, Displacement;
  - channel defaults as observed (O29).
- [ ] **Node material:** a physically based graph material.
  - BSDF nodes: diffuse, GGX, sheen, transmission, emission.
  - Mixing nodes; texture and maths nodes.
  - A conversion of the standard material into an equivalent node graph, which is what
    the renderers actually consume.
- [ ] **Shaders:** Noise (the noise library, D/063), Gradient (2D/3D, circular, radial),
  Color, Fresnel, Falloff, Layer (blend modes), Colorizer, Bitmap, Tiles, Checkerboard,
  Brick, Fusion, Filter, Normalizer, AO (render time), Vertex Map, Field shader.
- [ ] **Texture tag:**
  - projection: UVW, spherical, cylindrical, flat, cubic, frontal, spatial, shrink-wrap,
    camera;
  - offset, length, tiles;
  - selection restriction;
  - mix with other tags;
  - Generate UVW command.
- [ ] **UVs:**
  - the primitives' own UVs (parity with the golden `vt` lines);
  - a UV editor: view, select, move/scale/rotate, relax (LSCM), project, pack, mirror,
    UV seams.
- [ ] **Images:** PNG, JPEG, EXR, HDR, TIFF; a texture cache; sRGB/linear handling; a
  colour management setting (linear workflow default, like the reference).
- [ ] **Material manager:** a grid of material previews; drag and drop onto objects;
  material preview renders.
- [ ] **Environment:** sky and HDRI environment objects (with K).

## Observation jobs

| Job | Form | Status |
|---|---|---|
| O28 | [uv-projection.md](../../parity/uv-projection.md): primitives' UVs and projections | todo |
| O29 | scene-defaults §4: Standard material channel defaults | todo |
| O42 | [shaders.md](../../parity/shaders.md): shaders sampled on a 32×32 UV grid as field-kind samples (noise types, gradient types, tiles, checker, brick) | todo |
| O43 | [material-renders.md](../../parity/material-renders.md): reference renders of material variations on a sphere (colour, reflectance roughness, bump with noise, transparency, luminance) | todo |
| G7 | GUI-SESSIONS §G7: material manager and node editor look and behaviour | gui-only |

## Implementation design

- **`crates/material` (`forge-material`):**
  - `Material` node type (`Category::Material`) with channel params;
  - `NodeMaterial` (graph);
  - `fn compile(&Material) -> ShadingGraph`: a small IR both renderers consume (CPU
    interprets it, GPU compiles it to WGSL);
  - texture tags as `Category::Tag` nodes with projection params.
- **`crates/shaders` (`forge-shaders`):**
  - each shader is `fn sample(ctx: &ShadeCtx) -> Color`, with `ShadeCtx` = UVW, position,
    normal, time;
  - noise comes from `forge-noise`;
  - parity: level B against O42 samples (the UV grid uses the field-kind format with
    points (u, v, 0)).
- **UVs:**
  - primitive UVs are added to `PolyMesh` as a corner attribute (`uvs: Vec<[f32; 2]>` per
    corner, plus a UV index);
  - the golden `vt`/`f v/vt` already carry the reference UVs: parity for every primitive
    case, level A;
  - LSCM via `nalgebra-sparse` least squares;
  - packing: a skyline bin packer.
- **Images:** the `image` crate (PNG/JPEG/TIFF/HDR) and `exr` (OpenEXR, pure Rust).

## Briefs

| # | Title | Depends on | Observations |
|---|---|---|---|
| 123 | Material model: Standard material + material tag + IR compile | – | O29 |
| 124 | Node material graph + node editor UI (shared canvas with 109) | 123, 109 | G7 |
| 125 | Shaders I: Noise, Gradient, Color, Fresnel, Falloff, Layer, Colorizer, Bitmap | 063, 123 | O42 |
| 126 | Shaders II: Tiles, Checkerboard, Brick, Fusion, Filter, Normalizer, Vertex Map, Field, AO | 125 | O42 |
| 127 | Texture tag projections + Generate UVW | 123, 128 | O28 |
| 128 | Corner UVs on PolyMesh; primitive UV parity | – | O28, all primitive goldens |
| 129 | UV editor (view, transform, relax LSCM, pack, seams) | 128, 150 | G7 |
| 130 | Image IO, texture cache, colour management | – | – |
| 131 | Material manager UI with preview renders | 123, 136 | G7 |
| 132 | Viewport material display (PBR preview, textures) | 123, 143 | – |
| 133 | Environment: sky, HDRI, background, floor | 130, 137 | O29 |
| 134 | Material exchange mapping (glTF PBR ⇄ node material) | 124, 164 | – |

## Done when
S8's material part passes. Every primitive's UVs match. Shader samples match at level B,
or statistically where noise differs.
