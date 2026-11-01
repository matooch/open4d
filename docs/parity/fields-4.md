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

## Notes
TODO
