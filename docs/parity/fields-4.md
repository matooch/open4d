# Fields IV: modifier layers and vertex maps — observation (O49)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Method: as [fields.md](fields.md) (O7): 729-point grid, value channel, samples in
`tests/golden/fields/<case>/samples.json`, with `observation = "docs/parity/fields-4.md"`.
The vertex-map cases export meshes instead (see below).

Unblocks: D/060, 062.

The base for the modifier cases is one Spherical field of size 200, with a modifier layer
**above** it. Record each modifier's parameter table.

## Cases

| Case | Layers (top to bottom) | Changes | Notes |
|---|---|---|---|
| mod-remap | Remap layer over Spherical | remap min 20 %, max 80 % | |
| mod-curve | Curve layer over Spherical | its default curve | |
| mod-step | Step layer over Spherical | 4 steps (if offered) | |
| mod-colorize | Colorize over Spherical | defaults: the value channel only | |
| mod-invert | Invert layer (if it exists) over Spherical | – | |
| mod-clamp | Clamp layer over Spherical | – | |
| freeze-frame10 | a Freeze layer over a Spherical field moving from (−200, 0, 0) at frame 0 to (200, 0, 0) at frame 30 | sampled at frame 10, after stepping from 0 | Time-dependent |
| decay-frame20 | Decay layer, same moving sphere | frame 20 | |
| delay-frame20 | Delay layer, same moving sphere | frame 20 | |
| vmap-from-field | a cube (segments 4/8/4, size_y 400) with a Vertex Map tag whose fields list holds a Linear field (direction Y+, length 200) | – | Export `mesh.obj`, and the vertex map weights per point as `samples.json` kind `track` with param `weight` and frame = point index (a reuse; note it) |
| restrict-bend | the same cube and vertex map, plus a Bend (strength 90) restricted by a Restriction tag naming the vertex map | – | `mesh.obj` |
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

## Notes
TODO
