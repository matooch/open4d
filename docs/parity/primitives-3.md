# Primitives III — observation (O18)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Method: scripted, exactly as the primitive forms (for example [cube.md](cube.md)). Level A.
Unblocks: the solid primitives we don't have yet (PLAN phase 5).

Every case:
- The object is made through its menu command, at the origin.
- Only the row's changes are set.
- The object is exported as `tests/golden/<feature>/<case>/mesh.obj`, with the feature
  names given below.

`case.toml` lists **every** parameter under our keys, with
`observation = "docs/parity/primitives-3.md"`.

## Parameters (fill once per object, as in r25-defaults.md)

### Platonic (`platonic`)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### Oil Tank (`oil-tank`)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### Polygon (`polygon-prim`)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### Figure (`figure`)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### Landscape (`landscape`)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

## Cases

| Case | Object | Changed from default |
|---|---|---|
| platonic/defaults | Platonic | – |
| platonic/each-type | Platonic | one case per Type option, named `type-<option>` |
| platonic/segments | Platonic | segments 3 |
| oil-tank/defaults | Oil Tank | – |
| oil-tank/segments | Oil Tank | height segments 3, cap segments 6, rotation segments 12 |
| oil-tank/slice | Oil Tank | Slice on, To 270° |
| polygon-prim/defaults | Polygon | – |
| polygon-prim/triangle | Polygon | Triangle on |
| polygon-prim/segments | Polygon | segments 4 |
| figure/defaults | Figure | – |
| landscape/defaults | Landscape | – (record the seed shown) |
| landscape/seed | Landscape | seed 12345 |
| landscape/segments | Landscape | width and depth segments 20 |
| landscape/flat | Landscape | Fractal Noise off (or the closest), Rough/Fine furrows 50 % |

Landscape is random. Record every setting that influences it: seeds, noise options,
borders at sea level, and spherical. The seed alone must reproduce the mesh.

## Notes
TODO
