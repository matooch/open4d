# Fields IV: modifier layers and vertex maps — observation (O49)

Status: observed · App version: Cinema 4D R25 (version number 25010) · OS: macOS (Darwin 25.6.0) · Observer: observer session (MCP) · Date: 2026-11-19
Method: as [fields.md](fields.md) (O7): 729-point grid, value channel, samples in
`tests/golden/fields/<case>/samples.json`, with `observation = "docs/parity/fields-4.md"`.
The vertex-map cases export meshes instead (see below).

Unblocks: D/060, 062.

The base for the modifier cases is one Spherical field of size 200, with a modifier layer
**above** it. Record each modifier's parameter table.

## Cases

| Case | Layers (top to bottom) | Changes | Notes |
|---|---|---|---|
| mod-remap | Remap layer over Spherical | remap min 20 %, max 80 % | Export `samples.json` (the form's 729-point grid, value channel); the base Spherical (Size 200) keeps its own remapping at the defaults (Min 0 %, Max 100 %) so only the layer is measured. The Remap layer's table carries `Min` 1005004 = 20 % and `Max` 1005005 = 80 % and the samples run 0.2 at the field's outer edge to 0.8 at the centre, following 0.2 + 0.6 v: a Remap modifier layer rescales the value of the layer below it into [Min, Max] and holds it there outside that band. |
| mod-curve | Curve layer over Spherical | its default curve | |
| mod-step | Step layer over Spherical | 4 steps (if offered) | |
| mod-colorize | Colorize over Spherical | defaults: the value channel only | |
| mod-invert | Invert layer (if it exists) over Spherical | – | |
| mod-clamp | Clamp layer over Spherical | – | |
| freeze-frame10 | a Freeze layer over a Spherical field moving from (−200, 0, 0) at frame 0 to (200, 0, 0) at frame 30 | sampled at frame 10, after stepping from 0 | Time-dependent |
| decay-frame20 | Decay layer, same moving sphere | frame 20 | |
| delay-frame20 | Delay layer, same moving sphere | frame 20 | |
| vmap-from-field | a cube (segments 4/8/4, size_y 400) with a Vertex Map tag whose fields list holds a Linear field (direction Y+, length 200) | – | Export `mesh.obj`, and the vertex map weights per point as `samples.json` kind `track` with param `weight` and frame = point index (a reuse; note it) |
| restrict-bend | the same cube and vertex map, plus a Bend (strength 90) restricted by a Restriction tag naming the vertex map | – | Export `mesh.obj` (Bend strength 90, Angle 90, Restriction tag naming the vertex map at strength 1.0). Weights sampled from the tag's own Linear field through the app field engine (`FieldList.SampleListSimple`, 0.0-1.0 along Y+) and baked into the tag: a fields-driven Vertex Map tag keeps no data buffer in R25, and with Use Fields left on the Bend produced no deform cache at all (0 of 162 points moved).|
| remap-order-invert | Spherical (one layer) | follow-up from brief 057: Inner Offset 0 %, Invert on, Strength 50 % | settles the order of invert and strength |
| remap-order-min | Spherical | Inner Offset 0 %, Min 20 %, Strength 50 % | order of min/max and strength |
| remap-vs-clamp | Spherical | Inner Offset 0 %, Min 20 %, Max 80 % | remap (0.25 → 0.35) or clamp (stays 0.25) |
| clamp-200 | Spherical | Strength 200 %, Clamp Min and Clamp Max on; and `clamp-200-off` with both off | where the clamps act |
| step-inner-0 | Spherical | Inner Offset 0 %, contour Step, 4 steps | step formula (ceil or floor) |
| quantize-3 | Spherical | Inner Offset 0 %, contour Quantize, 4 steps, then `quantize-3` with 3 steps | does Quantize differ from Step |
| quadratic-curve | Spherical | Inner Offset 0 %, contour Quadratic, Curve 0 %, then `-50` and `-m50` with 50 % and −50 % | quadratic shape |
| clip-invert | Spherical | Invert on, Clip to Shape on | clipping before or after remapping |
| remap-off | Spherical | Enable Remapping off | value without remapping |
| mirror-graph | Linear | Mirror Graph on (if offered) | |
| cone-f/size | Cone | follow-up from brief 058: Height 200, Radius 50, on a 9×9×9 grid at spacing 25; then `cone-f/dir-yneg` with Direction Y− | exact cone extent and taper |
| clip-f/shapes | Cylinder, Cone, Torus, Capsule (one case each) | Clip to Shape on and Invert on | clip rule for the new shapes |
| capsule-f/unequal | Capsule | Top Radius 100, Bottom Radius 50; then `capsule-f/dir-yneg` | radius interpolation |
| torus-f/thick | Torus | Thickness 100, samples spaced 25 within 200 of the ring | torus distance metric |
| radial-f/combo | Radial | Start 45°, Start Transition 45°, End 270°, Offset 30°, Iterations 2; then `radial-f/axis-x` and `radial-f/axis-z` with the same settings | transitions, offset sign, iterations, axes |
| random-f/perlin-line | Random (Noise mode, Perlin) | 201 samples along x from −100 to 100 at 1-unit steps, seed 0; then seed 1 | feature size, octaves, seed hashing |
| random-f/space-none | Random | Space None | |
| random-f/sorted | Random | mode Sorted | sorting rule |
| random-f/speed-f30 | Random (Noise) | Animation Speed 100 %, sampled at frame 30 | time scale |
| random-f/types | Random (Noise) | one 201-sample line per noise type the brief leaves unsupported (record the type name per case) | defines those noise types |

## Notes
TODO
