# REALTIME — the real-time GPU renderer (area R)

Sources: [area R](../roadmap/areas/R-realtime.md), decision D19 and the shared
material specs ([MATERIAL.md](MATERIAL.md), [SHADERS.md](SHADERS.md),
[RENDER.md](RENDER.md)). This is our own design, not reference parity.

## 1. Shading IR → WGSL (brief 197, part 1)

One specialised function per material, generated from the shading IR
(`forge_material::ir::ShadingGraph`). Part 1 covers the IR nodes and every material
channel; the shader families are brief 207 and use the documented fallback here.

### 1.1 Entry point and types

Every module declares:

```wgsl
struct ShadeCtx {
  uvw: vec3<f32>, position: vec3<f32>, normal: vec3<f32>, view: vec3<f32>, time: f32,
  vertex_weight: f32, ao: f32,
};
struct Surface { /* the channel fields below */ };
fn forge_surface(ctx: ShadeCtx) -> Surface;
```

`vertex_weight` and `ao` mirror the CPU `Option`s of `forge_material::ShadeCtx`: a host without
a vertex map passes `vertex_weight = 0.0`, a renderer without occlusion passes `ao = 1.0`
(the values the CPU shaders use for `None`). `CODEGEN_VERSION` is 2 since this change.

`forge_surface` evaluates every present channel and fills absent ones with the documented
defaults. Channels are packed into `vec4<f32>` rows so a consumer or a test can upload and
compare them without a layout table:

| Field | Contents |
|---|---|
| `diffuse` | colour rgb, `level` |
| `diffuse_params` | `falloff`, `roughness`, `model` (0 Lambertian, 1 Oren-Nayar), present |
| `specular` | colour rgb, `strength` |
| `specular_params` | `width`, `falloff`, `inner_width`, `metal` |
| `specular_flags` | present, `kind` (documented index), `colored`, 0 |
| `emission` | rgb, present |
| `transmission` | colour rgb, `ior` |
| `transmission_params` | absorption rgb, `absorption_distance` |
| `transmission_flags` | `fresnel_reflectivity`, total-internal-reflection, exit-reflections, additive |
| `transmission_present` | present in `.x` |
| `environment` | rgb, present |
| `fog` | rgb, `distance` |
| `fog_present` | present in `.x` |
| `alpha` | colour rgb, `soft` |
| `alpha_params` | delta rgb, `invert` |
| `alpha_flags` | present, `image_alpha`, `premultiplied`, 0 |
| `bump` | colour rgb, `strength` |
| `bump_present` | present in `.x` |
| `normal` | colour rgb, `strength` |
| `normal_params` | present, `method` index, flip bits, `swap_yz` |
| `displacement` | colour rgb, `strength` |
| `displacement_params` | present, `height`, `kind` index, 0 |
| `diffusion` | `brightness`, `affect_luminance`, `affect_specular`, `affect_reflection` |
| `diffusion_present` | present in `.x` |

All fields are `vec4<f32>` (present flags use `.x`) so a harness can upload and compare
fixed-size rows. Absent channels are zero, except the alpha colour (1,1,1) and alpha
`soft`/flags zero.

### 1.2 IR nodes

- `Color(c)` → `vec3(c)`.
- `Scale { input, factor }` → `input * factor`.
- `Mix { base, top, mode, strength }` → `mix(base, top, strength)` for Normal, and
  `base + (op(base, top) - base) * strength` for Add/Subtract/Multiply, exactly as
  `forge_material::ir::mix`.
- `Texture(name)` (a shader-node channel) compiles through a `TextureCodegen` (section 2).
  Without one (`compile`, `ShaderCache::module`) it is the shader default colour
  `(0.8, 0.8, 0.8)` and the name is reported in `WgslMaterial::fallbacks`.

### 1.3 Bindings and validation

Part 1 needs no buffers for the IR itself: all constants and node values are inlined.
The comparison harness appends its own `@group(0)` storage bindings for the input contexts
and the output surfaces. Every generated module must pass `naga` validation
(`ValidationFlags::all`, `Capabilities::all`) in a plain unit test, so CI checks codegen
without a GPU.

### 1.4 Cache

`ShaderCache` maps a key to an `Arc<WgslMaterial>`. The key is the stable FNV-1a hash of
`CODEGEN_VERSION` and the graph's canonical debug form (nodes and outputs); the same IR
returns the same `Arc` without recompiling, and an edit produces a new key. The cache also
exposes a compile counter for the tests. Compiling a 50-node material is measured in a test
and must stay under 100 ms.

### 1.5 Checks

`crates/material/tests/wgsl.rs` validates representative modules with naga, checks cache
reuse/recompile and the 50-node timing without a GPU.
`crates/material/tests/wgsl_gpu.rs` evaluates a grid of `ShadeCtx` through the generated
module on the GPU (skipped without an adapter unless `FORGE_REQUIRE_GPU=1`) and compares
each present channel with the CPU interpreter (`ShadingGraph::eval` and the scalar fields)
within 1e-4 in f32.

## 2. Shader nodes to WGSL (brief 207, part 2 of 197)

Source of truth: the CPU shaders ([SHADERS.md](SHADERS.md) S1-S8, `crates/shaders`). Every
`Shader` variant has a WGSL equivalent that computes the same value in `f32`. Nothing falls
back: the CPU `Shader` enum has no variant without a port (the S3 noise types without a public
definition never build, so they never reach the codegen). An unknown shader name is white, as in
`ShaderSet` (S1).

### 2.1 Hook and layout

`forge_material::wgsl` exposes `TextureCodegen { fingerprint(name), emit(name, &mut
ModuleBuilder) }`, `compile_with(graph, lib)` and `ShaderCache::module_with(graph, lib)`; the
cache key also folds in `fingerprint(name)` of every referenced shader (a hash of the whole
shader tree, bitmap pixels included). `forge_shaders::wgsl` implements it for a `ShaderSet`
(`compile(graph, &set)`, `ShaderSetCodegen`).

- **One library function per family**, emitted once per module whatever the number of nodes
  (`ModuleBuilder::once`): `forge_noise_eval`, `forge_blend`, `forge_ramp_seg`, `forge_hsl`
  helpers, `forge_tiles`, `forge_bitmap`. Each node instance becomes a small generated function
  `forge_fN(ctx) -> vec3<f32>` that calls the library function with its parameters inlined as
  constants; identical instances (same text) share one function. Child shaders (Layer, Fusion,
  Colorizer, Filter, Normalizer) are emitted first, and the parent calls them by name.
- Parameters are inlined constants computed on the host in `f64` and written as `f32`
  (`0.1 * global_scale`, `max(high - low, 1e-9)`, ...), so the GPU does the same operations as the
  CPU in the same order, with `f32` rounding only.
- `ShadeCtx` carries everything the CPU context has (section 1.1); `uvw`, `position`, `normal`,
  `view`, `time`, `vertex_weight`, `ao`.
- Gradient knots are unrolled into a per-ramp function: `t <= first` and `t >= last` clamp, else
  the last knot with position `<= t` selects the segment, `x = (t - p0) / (p1 - p0)` (1 when
  `p1 <= p0`), interpolation none (`x = 0`), linear, or cubic (`x * x * (3 - 2 x)`), exactly
  `Gradient::eval`.

### 2.2 Noise (S3, `forge-noise`)

All 17 `NoiseKind`s, always the 4D variants (the shader samples `sample4`): Perlin, Value,
Simplex, Box, Cell, Voronoi1/2/3, Fbm, Turbulence, RidgedMultifractal, WavyTurbulence,
Electric, Gaseous, BlisteredTurbulence, DisplacedTurbulence, DisplacedVoronoi.
`forge_noise_eval(kind, seed, octaves, p: vec4<f32>)` takes `kind` as the index in
`NoiseKind::ALL`; lacunarity 2, gain 0.5, warp 0.5 are the `Noise::default()` constants the
shaders use.

The CPU noise has **no permutation table**: it hashes lattice cells with SplitMix64 (`hash_cell`,
`octave_seed`, the Worley jitter and id hashes, the tile hash). WGSL has no 64-bit integers, so
the port keeps the identical hash on `vec2<u32>` (`x` = low word, `y` = high word): 64-bit xor,
shift, add, and a wrapping multiply built from 16-bit limbs. The seed rules are the same
(`seed * 0x9e3779b97f4a7c15 ^ 0x123456789abcdef0`, per-octave `octave_seed`, offsets 100 + k for the
warp, 200 for the blister mask), so lattice values, gradient choices, Worley jitter and ids are
bit-identical to the CPU; only the arithmetic on the values is `f32`. The unit value of a hash
is the top 24 bits of the CPU's 53-bit value plus the next 24 bits (`f32` rounding error 3e-8).
Gradient choice `(h >> 8) % 32` is the low word's bits 8-12. Simplex sorts axes by decreasing
offset with the same insertion sort, Worley visits the 3^4 neighbourhood in the same order
(`f1`, `f2`, id of the nearest) with `1e30` as the initial distance.
Output stage (shader level): `v = 0.5 + 0.5 n` or `|n|`, clip, brightness, contrast, clamp,
`mix(c1, c2, v)`, constants as in S3. Lattice coordinate `q = (p / relative_scale) / (0.1 *
global_scale)` in `f32`; time (`animation_speed * ctx.time`) is the fourth coordinate.

**Bound.** One lattice value cannot be exact: `q` carries an `f32` rounding of `|q| * 6e-8`
before the `fade` curve, amplified by the octave frequency, and the discontinuous kinds (Box,
Cell, Voronoi borders) would jump if `f32` and `f64` put a point on different sides of a cell
boundary. The harness points are fixed pseudo-random values rounded to `f32` first (no point sits
on a boundary), so CPU and GPU see identical inputs. Measured errors (section 2.9) are
far below 1e-4 for every kind, including 8 octaves, so noise keeps the default bound.

### 2.3 Gradient, Color, Checkerboard, Fresnel, Falloff (S2, S4, S6)

Per variant, the S2/S4/S6 formulas with the 2D `t` rules as constants: U `u`, V `1 - v`,
Diagonal `(u + 1 - v) / 2`, Circular `sqrt(2) * hypot(u - 0.5, v - 0.5)`, Radial
`atan2(v - 0.5, u - 0.5) / tau + 0.5`, Linear/Spherical/Cylindrical from `position`.
Turbulence: `t += turbulence * forge_noise_eval(Fbm, seed, octaves, vec4(q / (0.1 * scale), 0))`
with `q` = `uvw` for the 2D kinds and `position` for the 3D kinds. `cycle` wraps with
`t - floor(t)`. Fresnel and Falloff use `normalize(normal)`, `normalize(view)` and the
CPU's "no geometry" cases (zero-length normal or view: `smoothstep(v)` for Fresnel, `t = 0` for
Falloff). Checkerboard: `floor(u * 2 * fu)`, `floor(v * 2 * fv)`, parity of the sum.

### 2.4 Tiles (S4)

Per UV axis `x = uv / pitch`, `d = min(fract, 1 - fract) * pitch`, `p` as in S4, combined with
`min`. Colour by `(i + j)` parity, or with `randomize` by the CPU's `cell_hash`
(`(seed << 32) ^ i * C1 ^ j * C2`, xorshift-multiply, `% 3`), on the same 64-bit helpers as noise.

### 2.5 Layer, Fusion (S5)

`forge_blend(base, top, mode, opacity)` with the mode index of `BlendMode` declaration order
(normal 0 ... difference 8): `base + (blend(base, top) - base) * opacity` per component.
Layer folds the enabled layers over the base (black without a base); Fusion is one blend of its
two children.

### 2.6 Colorizer, Filter, Normalizer (S7, S8)

Colorizer: luminance `0.2126 r + 0.7152 g + 0.0722 b` or a channel, wrap/clamp, ramp.
Filter: the CPU's `rgb_to_hsl` / `hsl_to_rgb` (same branches), hue added, saturation scaled and
clamped, then lightness, contrast about 0.5, `pow(max(v, 0), 1 / max(gamma, 1e-9))`, invert.
Normalizer: `clamp((v - low) / max(high - low, 1e-9), 0, 1)` per channel. A missing child
shader is black, as in the CPU.

### 2.7 Vertex Map, AO (S8)

`mix(low, high, clamp(ctx.vertex_weight, 0, 1))` and `mix(low, high, clamp(ctx.ao, 0, 1))`. The
renderer fills `vertex_weight` / `ao` per fragment (defaults in section 1.1).

### 2.8 Bitmap (S8)

Bindings: every distinct bitmap of a module is `WgslMaterial::resources[k]` (an `Arc<Image>`,
linear `f32` RGBA, row 0 at the top) and is declared
`@group(1) @binding(k) var forge_tex_k: texture_2d<f32>;`. The host uploads it as
`Rgba32Float` (unfilterable-float binding, no sampler). **Filtering is done in the shader with
`textureLoad`, not by the sampler hardware**, so it equals the CPU rule exactly: `x = u * w`,
`y = (1 - v) * h`; nearest takes `floor(x), floor(y)`; bilinear takes the four texels around
`(x - 0.5, y - 0.5)` with fractional weights; `repeat` wraps the texel index with a Euclidean
modulo, otherwise it clamps to the edge. The size comes from `textureDimensions`. Alpha is
ignored (the CPU returns rgb).

### 2.9 Checks and measured bounds

`crates/shaders/tests/wgsl.rs` validates every family with naga and times the 50-node
compile; `crates/shaders/tests/wgsl_gpu.rs` runs every family on a grid of contexts through
the generated module on the GPU against `Shader::sample` (skipped without an adapter unless
`FORGE_REQUIRE_GPU=1`) and asserts the per-family bound below. Default bound 1e-4 (absolute,
per channel).

Grid: 192 fixed pseudo-random contexts (uvw in [-0.25, 1.25], position in [-2, 2], time in
[0, 6], every tenth without geometry, every seventh without vertex weight, every eleventh
without occlusion). Maximum absolute error per channel over the grid, measured on the software
Vulkan driver (lavapipe); the bound is what the test asserts.

| Family | Cases | Measured max | Bound |
|---|---|---|---|
| Noise | all 17 kinds (4 octaves, 4D with time), 8-octave Fbm/Turbulence/Ridged, absolute, clip/brightness/contrast, relative scale, seeds | 3.6e-6 (Simplex with clip and contrast; Ridged 8 octaves 3.6e-6) | 1e-4 (default) |
| Gradient | 8 kinds, each plain and with cycle + linear ramp + turbulence; step ramp, one knot, degenerate axis | 3.7e-6 (Radial) | 1e-4 (default) |
| Color, Checkerboard, Fresnel, Falloff | 6 cases (invert, ramps, no-geometry points) | 1.7e-6 | 1e-4 (default) |
| Tiles | bevel, no bevel, randomize (seeds 0 and 12345), non-square pitch | 2.9e-6 | 1e-4 (default) |
