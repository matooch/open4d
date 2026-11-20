# SHADERS — shaders sampled at a point (briefs 125, 126)

Sources: O42 [docs/parity/shaders.md](../parity/shaders.md) and `tests/golden/shaders/` (17 cases,
`samples*.json` on a 32 x 32 UV grid, red/green/blue, tolerance 1e-4); O43
[material-renders.md](../parity/material-renders.md) (Fresnel and Checkerboard are used in the
renders); published models: smoothstep, Schlick's Fresnel approximation, HSL, the blend formulas
(Porter-Duff / SVG compositing), Perlin / Worley noise through `forge-noise` (NOISE.md).
Nothing is read from the reference app.

Crate `forge-shaders` (`crates/shaders`) depends on `forge-material` (the `ShadeCtx` and
`TextureSampler` of the IR, MATERIAL.md M3), `forge-noise` and `forge-image`.

## S1 Contract

- `Shader::sample(&ShadeCtx) -> [f64; 3]` (linear RGB). `ShadeCtx` = `uvw`, `position`,
  `normal`, `view`, `time`, `vertex_weight`, `ao`. `ShaderSet` (named shaders) implements
  `TextureSampler`; an unknown name is white.
- `build(type_id, &Values, &Inputs) -> Result<Shader, ShaderError>` builds from a parameter map over
  the type's defaults. `Inputs` carries the child shaders (Layer, Colorizer, Filter, Fusion,
  Normalizer) and the decoded image (Bitmap). `ShadersPlugin` registers 17 `forge.shader.*`
  types (`Category::Shader`, not placeable in a scene yet, MATERIAL.md M1).
- **Colour parameters are sRGB-encoded display values and are decoded to linear** (IEC 61966-2-1)
  when built. Evidence `tiles`: colour 0.5 samples as 0.21404 in all three channels; 0 and 1 are
  fixed points, so the black-to-white defaults are unaffected.
- Percent parameters are fractions (`1.0` = 100 %).

## S2 Gradient

Knots (position, colour) with interpolation `none | linear | cubic`; cubic = smoothstep between
neighbouring knots (evidence `gradient-u`: value = `3t^2 - 2t^3`, error 5e-7). The shader
builds a two-knot ramp (start colour, end colour, default black to white).
Ramp parameter `t` (2D types, `uvw = (u, v, 0)`): **U** `t = u` (`gradient-u`); **V** `t = 1 - v`
(`gradient-v`: the ramp starts at the top); **Circular** `t = sqrt(2) * hypot(u - 0.5, v - 0.5)`,
so the unit square's corner is the end (`gradient-circular`, exact); Diagonal
`t = (u + 1 - v) / 2` and Radial (`atan2` around the centre) are our definitions,
**parity-pending**. 3D types: Linear (projection on `start -> end`), Spherical and Cylindrical
(distance from `start` over `radius`), **parity-pending**. `cycle` wraps `t` (`t - floor t`), else
clamps. Turbulence adds `turbulence * fbm(uvw)` to `t` (noise: S3).

## S3 Noise

`t = noise.sample(p)` with `forge-noise` (4D, time = `animation_speed * time`), `p` = `uvw`
(space Texture / UV) or `position`, divided by `relative_scale`; feature size `0.1 * global_scale`.
Output `v = 0.5 + 0.5 t` (`absolute`: `|t|`), then clip `(v - low) / (high - low)`, brightness
(added), contrast (`(v - 0.5)(1 + c) + 0.5`), clamp, colour `mix(colour 1, colour 2, v)`.
The type list is O42's 32 entries; mapped: Noise = Perlin, Box, Cell Noise, Blistered /
Displaced / Wavy Turbulence, Turbulence, Electric, FBM, Gaseous, Cell Voronoi and Voronoi 1 = Voronoi1, Voronoi 2, 3,
Displaced Voronoi. The others (Buya, Cranal, Dents, Fire, Hama, Luka, Mod Noise, Naki, Nutous, Ober,
Pezo, Poxo, Sema, Stupl, VL Noise, Sparse Convolution, Zada) have no public definition:
`Unsupported`.
**Known mismatch** (level B cannot be met): the reference noise basis and scale are not public.
`noise-defaults`, `-fbm`, `-turbulence`, `-voronoi1`, `-cell`, `-octaves`, `-scale`, `-time` and
`gradient-turbulence` differ value by value; the test pins that and checks the distribution
(values in 0..1, mean within 0.25 of the reference, non-degenerate spread; `noise-turbulence`
only the range, since our Turbulence is a sum of `|noise|` remapped to -1..1 and sits far below
the reference mean 0.5). `noise-perlin` was not observable (no Perlin entry in O42's list).

## S4 Colour, Checkerboard, Tiles

- **Color**: `colour * brightness`.
- **Checkerboard** (`checkerboard`, exact): cells are half a period wide, `i = floor(2 fu u)`,
  `j = floor(2 fv v)`, colour 1 when `i + j` is even, else colour 2 (frequency 10 gives 20 cells per
  UV unit; the grid shows period-3 samples).
- **Tiles**, Squares only (`tiles`, exact in all three channels): pitch `0.1 / (global * axis
  scale)` per UV axis (10 tiles at defaults); tile colour by `(iu + iv)` parity (colour 1 at the
  origin tile, colour 2 next; colour 3 only with `randomize`, seeded hash, **parity-pending**).
  Per axis, `d` = distance to the nearest tile edge, `h` = pitch / 2, grout `g = grout_width * h`,
  bevel band `b = bevel_width * (h - g)`, `p = smoothstep((d - g) / b)` (`p = 0` for `d < g`);
  the two axes combine as `min(p_u, p_v)` (the sample next to a bevel in u keeps the pure u value);
  colour = `mix(grout, tile, p)`. With Bevel off `p` is a step at `g`. Fit: three distances, max
  error 4e-7. Other patterns (Hexagons, Triangles, Bricks) and Rotate != 0 are `Unsupported`.

## S5 Layer and Fusion

Layer: first input is the base, each further input one layer bottom to top, all with the
shader's `blend_mode` and `opacity` (the reference's layer list is not scriptable, O42 notes).
`acc' = acc + (blend(acc, top) - acc) * opacity`; modes normal, add, subtract, multiply,
screen, overlay, darken, lighten, difference (the FIELDS.md F7 formulas). Fusion = a Layer of
exactly two inputs. Not sampled by O42 (`layer-multiply` not observed): **parity-pending**.

## S6 Fresnel and Falloff

Fresnel: `F0 = ((n - 1) / (n + 1))^2`, `F = F0 + (1 - F0)(1 - |N.V|)^5` through a black-to-white
ramp (`invert`: `1 - F`). Without a normal or view in the context it returns the observed fallback
`smoothstep(v)` (`fresnel`: exact; the grid gives `smoothstep(v)` per row). Falloff: the ramp over
`1 - |N.V|`; 0 without geometry. Both **parity-pending** away from the UV grid (O43 renders).

## S7 Colorizer

`t` = luminance (`0.2126 R + 0.7152 G + 0.0722 B`) or one channel of the child shader, wrapped
(`cycle`) or clamped, through the gradient. `colorizer` (over the default Noise) is a known
mismatch because of the noise.

## S8 Bitmap, Filter, Normalizer, Vertex Map, AO

- Bitmap: a decoded `forge_image::Image` (linear working space, IMAGE.md); `v` up (`row =
  (1 - v) * height`), bilinear with half-texel centres or nearest, `repeat` or clamp.
- Filter: HSL hue shift / saturation scale, then lightness added, contrast about 0.5, gamma, invert.
- Normalizer: `(v - low) / (high - low)` clamped per channel (a point sampler cannot find the
  image's min and max).
- Vertex Map: `mix(colour 0, colour 1, ctx.vertex_weight)`, 0 when the host gives none.
- AO: `mix(occluded, open, ctx.ao)`, open when the renderer gives none (render-time effect).

## S9 Unsupported

Brick (`brick`: the observed pattern is noise-perturbed, one case, not derived), the Field
shader (needs a field at the sample point), Layer lists with per-layer modes (data only through
the GUI), the noise types of S3, Tiles patterns of S4, MIP / blur (`blur_offset`, `blur_scale`
are accepted and ignored).

## S10 Parity summary

17 cases: 6 exact (gradient-u, -v, -circular, checkerboard, tiles, fresnel; each channel file),
10 known mismatch (noise basis, S3, plus `colorizer` over noise), 1 unsupported (brick).
Test: `crates/shaders/tests/shaders_parity.rs`.
