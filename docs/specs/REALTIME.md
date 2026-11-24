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
};
struct Surface { /* the channel fields below */ };
fn forge_surface(ctx: ShadeCtx) -> Surface;
```

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
- `Texture(name)` (a shader-node channel) is **fallback in part 1**: it compiles to the
  shader default colour `(0.8, 0.8, 0.8)` and the name is reported in
  `WgslMaterial::fallbacks`. Brief 207 replaces it with the family's WGSL function.

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
