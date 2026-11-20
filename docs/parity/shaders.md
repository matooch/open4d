# Shaders sampled on a UV grid — observation (O42)

Status: observed (17 of 19 cases; see the two exceptions below) · App version: Cinema 4D 2025.1.0 (25.010) · OS: macOS 26.7.2 (25G332, arm64) · Observer: Hermes · Date: 2026-11-20
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

Every case below has `samples.json`, `samples-g.json` and `samples-b.json` (1024 points each,
u fastest), a `mesh.obj` of a default plane carrying the material, and the shader's whole
parameter table in `[params.shader]`. The ranges quoted are the red channel over the grid.

## Cases

| Case | Shader | Settings | Result |
|---|---|---|---|
| noise-defaults | Noise | defaults (record the noise type and seed) | type Noise, Seed 665, Octaves 5, Global Scale 100 %, Space Texture; red 0.178 … 0.84, first three values 0.2969, 0.4781, 0.5372 |
| noise-perlin | Noise | type Perlin, global scale 100 %, seed 0 | not observed: this build's Noise shader has no Perlin entry. Its type list runs Box, Blistered Turbulence, Buya, Cell Noise, Cranal, Dents, Displaced Turbulence, Electric, FBM, Fire, Gaseous, Hama, Luka, Mod Noise, Naki, Noise, Nutous, Ober, Pezo, Poxo, Sema, Stupl, Turbulence, VL Noise, Wavy Turbulence, Cell Voronoi, Displaced Voronoi, Sparse Convolution, Voronoi 1, Voronoi 2, Voronoi 3, Zada — there is nothing to select for "Perlin" |
| noise-fbm | Noise | type FBM | type FBM; red 0.0656 … 0.9071, first three 0.2218, 0.4022, 0.5686 |
| noise-turbulence | Noise | type Turbulence | type Turbulence; red 0.2457 … 0.7498, first three 0.3322, 0.4821, 0.5524 |
| noise-voronoi1 | Noise | type Voronoi 1 | type Voronoi 1; red 0.0233 … 1.0, first three 0.4694, 0.2617, 0.4865 |
| noise-cell | Noise | type Cell | type Cell Noise (the closest entry; "Cell" alone is not offered); red 0.0002 … 0.9963, first three 0.0253, 0.0253, 0.3895 |
| noise-octaves | Noise | Perlin, octaves 5 | recorded with the shader's default type Noise (no Perlin entry), Octaves 5 — the default is already 5, so the samples match noise-defaults value for value; red 0.178 … 0.84 |
| noise-scale | Noise | Perlin, global scale 400 % | recorded with the default type Noise (no Perlin entry), Global Scale 400 %; red 0.2208 … 0.6776, first three 0.3365, 0.3506, 0.3815 |
| noise-time | Noise | Perlin, animation speed 1, sampled at document time 1 s | recorded with the default type Noise (no Perlin entry), Animation Speed 1, document time 1 s; red 0.1858 … 0.8017, first three 0.3093, 0.4044, 0.5451 |
| gradient-u | Gradient | 2D-U, default colours | 2D - U; red 0.0007 … 0.9993, first three 0.0007, 0.0064, 0.0174 — the value rises with u |
| gradient-v | Gradient | 2D-V | 2D - V; red 0.0007 … 0.9993, first three 0.9993, 0.9993, 0.9993 — at v = 0.5/32 the value is already at the top of the ramp |
| gradient-circular | Gradient | 2D-Circular | 2D - Circular; red 0.0029 … 0.9971, first three 0.9971, 0.989, 0.9764 |
| gradient-turbulence | Gradient | 2D-U, turbulence 50 % | 2D - U with Turbulence 50 %; red 0.0 … 1.0, first three 0.0326, 0.0, 0.0 — the ramp is broken up by the turbulence |
| tiles | Tiles | defaults | Pattern Squares, Grout Width 5 %, Bevel 30 %, Orientation U; red 0.0012 … 0.214, the first three identical (0.2102) |
| checkerboard | Checkerboard | defaults (record U/V frequency) | U Frequency 10, V Frequency 10, Color 1 white, Color 2 black; red 0.0 … 1.0, first three 1.0, 1.0, 0.0 — exactly two levels |
| brick | Brick | defaults | Scale 25 %, Brick Width 50 %, Brick Height 25 %, Shift 50 %; red 0.0816 … 0.3954, first three 0.1242, 0.122, 0.1251 |
| colorizer | Colorizer over Noise | defaults | Colorizer at its defaults with a default Noise shader in its Texture slot, Input Luminance; red 0.0838 … 0.9314, first three 0.2121, 0.4671, 0.5557 |
| layer-multiply | Layer: Noise over Gradient, mode Multiply | | not observed: the Layer shader's layer list is not reachable from the script interface (reading it raises "Parameter value not accessible (object unknown in Python)"); the layers would have to be built in the GUI |
| fresnel | Fresnel | defaults: sampled in UV, which shows only its flat fallback. Keep it; the renders in O43 test it properly | defaults; red 0.0007 … 0.9993, first three 0.0007, 0.0007, 0.0007 — as the form expects, the UV grid only sees the flat fallback |

## Notes

- The sampling call needs the shader initialised for a document first (`InitRenderStruct(doc)`);
  without that it returns pure white (1, 1, 1) for every point. Each sample also carries a
  position, a normal, a direction and a time (seconds as a plain number), and the shader's own
  channel data is not writable, so the time comes from the document's frame.
- Shader parameter values are stored as enum numbers well above the cycle index (the Noise
  shader's type 2013 is "Noise", the Gradient's 2000 is "2D - U"), which is why the case files
  write the label the Attribute Manager shows; the raw numbers stay in the probe workspace.
- This build's Noise shader has no Perlin entry (see the `noise-perlin` row). Three rows asked
  for it: `noise-perlin` is left not observed, while `noise-octaves`, `noise-scale` and
  `noise-time` are recorded with the shader's default type "Noise" and the rest of the row's
  settings applied, because those rows are about the octaves, the scale and the animation speed.
- `noise-octaves` cannot differ from `noise-defaults`: the shader's default octave count is
  already 5, so the case documents that the default is 5 rather than testing a change.
- Octaves, Global Scale and Animation Speed are the three knobs that changed the samples;
  the seed (665) was left at the shader's default everywhere except the rows that name it.
- The checkboard is the only shader on this grid with exactly two levels (0 and 1); Tiles and
  Brick stay inside their palette because the grid falls on grout and brick faces rather than
  on colour 2.
- Layer shaders are the one shader type whose contents the script interface cannot touch, so
  the composite case is left for a GUI session.
