# Cloner and basic effectors — observation (O8)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Method: scripted ([agents/OBSERVER.md](../agents/OBSERVER.md)). Level B (numeric samples).
Unblocks: P3 (cloner, clone data, effectors).

Every case starts from a new document with a cloner made through its menu command and,
as its only child, a default cube scaled down to size 20 × 20 × 20. Read the cloner's
per-clone data **after evaluation**, through the app's documented clone-data call. For
each clone, record:
- its matrix: position, then the X, Y and Z axis vectors, in cloner space;
- its colour (RGB 0…1);
- its weight;
- whether it is visible.

Save `tests/golden/cloner/<case>/samples.json`:

```json
{ "kind": "clones", "tolerance": 1e-4,
  "clones": [ { "pos": [x,y,z], "x": [..], "y": [..], "z": [..],
                "color": [r,g,b], "weight": w, "visible": true }, ... ] }
```

Also export the cloner as OBJ to `mesh.obj` in the same folder, with the usual settings,
and add `mesh = "mesh.obj"` to `case.toml`.

`case.toml`: `feature = "cloner"`. `[params]` holds `mode` and every cloner parameter
shown for that mode, plus each effector's parameters under `[params.effector]`, using
our key names.

## Parameters (fill once per mode)

For each mode, list the Object tab and the Transform tab as shown: label, our key,
default, min and max.

### Linear mode
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### Radial mode
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### Grid Array mode
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### Honeycomb mode
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### Object mode
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

## Cases

| Case | Mode | Changed from default | Effector | Notes |
|---|---|---|---|---|
| linear-defaults | Linear | – | – | |
| linear-count-step | Linear | count 5, position offset (0, 50, 0), rotation step H 10° | – | |
| radial-defaults | Radial | – | – | |
| radial-plane-start-end | Radial | radius 150, plane XZ (or the second option), start 0°, end 180° | – | |
| grid-defaults | Grid Array | – | – | |
| grid-3x2x4 | Grid Array | count (3, 2, 4), size (300, 200, 400) | – | |
| honeycomb-defaults | Honeycomb | – | – | |
| object-vertex | Object, on a default sphere with 8 segments | distribution Vertex | – | |
| linear-plain | Linear | – | Plain, position (0, 100, 0), defaults otherwise | |
| linear-plain-field | Linear, count 10 | – | Plain, position (0, 100, 0), a Linear field in its falloff list | |
| grid-random | Grid Array | – | Random, defaults (record its seed) | |
| grid-random-seed | Grid Array | – | Random, seed 12345, position (50, 50, 50) | |

## Notes
TODO
