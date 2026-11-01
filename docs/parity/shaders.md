# Shaders sampled on a UV grid — observation (O42)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Method: scripted.

1. Make a material through its menu command and put the shader in its Color channel. Use
   the material only as a host, not for rendering.
2. Sample the shader through the documented shader sampling call at the **32 × 32 UV grid**
   u, v ∈ {0.5/32, 1.5/32, …, 31.5/32}, with w = 0, at document time 0.

Output: `tests/golden/shaders/<case>/samples.json`, kind `field`:
- points (u, v, 0) in u-fastest order;
- value = the **red** channel; colour shaders also get `samples-g.json` and
  `samples-b.json`;
- `tolerance` 1e-4.

Add `mesh.obj`: a default plane with the material applied, exported as usual (its UVs are
the truth for "where the shader lands").

Use `feature = "shaders"`. Level B. Unblocks: D/063, J/125, 126.

Record each shader's parameter table (label, our key, default, options).

## Cases

| Case | Shader | Settings |
|---|---|---|
| noise-defaults | Noise | defaults (record the noise type and seed) |
| noise-perlin | Noise | type Perlin, global scale 100 %, seed 0 |
| noise-fbm | Noise | type FBM |
| noise-turbulence | Noise | type Turbulence |
| noise-voronoi1 | Noise | type Voronoi 1 |
| noise-cell | Noise | type Cell |
| noise-octaves | Noise | Perlin, octaves 5 |
| noise-scale | Noise | Perlin, global scale 400 % |
| noise-time | Noise | Perlin, animation speed 1, sampled at document time 1 s |
| gradient-u | Gradient | 2D-U, default colours |
| gradient-v | Gradient | 2D-V |
| gradient-circular | Gradient | 2D-Circular |
| gradient-turbulence | Gradient | 2D-U, turbulence 50 % |
| tiles | Tiles | defaults |
| checkerboard | Checkerboard | defaults (record U/V frequency) |
| brick | Brick | defaults |
| colorizer | Colorizer over Noise | defaults |
| layer-multiply | Layer: Noise over Gradient, mode Multiply | |
| fresnel | Fresnel | defaults: sampled in UV, which shows only its flat fallback. Keep it; the renders in O43 test it properly |

## Notes
TODO
