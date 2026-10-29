# Fields II — observation (O13)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Method: exactly as [fields.md](fields.md) (O7): menu command, one field layer,
value channel, the same 729-point grid, `tests/golden/fields/<case>/samples.json`,
`observation = "docs/parity/fields-2.md"`. Unblocks: P2 beyond Linear, Spherical and Box.

## Parameters (fill once per field type, as in fields.md)

### Cylinder field
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### Cone field
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### Torus field
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### Capsule field
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### Radial field
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### Random field
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

## Cases

| Case | Field | Changed from default | Notes |
|---|---|---|---|
| cylinder-defaults | Cylinder | – | |
| cylinder-size | Cylinder | radius 200, height 300 (or the closest fields) | |
| cone-defaults | Cone | – | |
| torus-defaults | Torus | – | |
| capsule-defaults | Capsule | – | |
| radial-defaults | Radial | – | |
| random-defaults | Random | – | record the seed and the noise type shown |
| random-seed | Random | seed 12345 | |
| two-layers-max | Spherical at (−100, 0, 0) and Spherical at (100, 0, 0) in one field list, the top layer's blend mode Max | – | Sample the list. Record the layer order and each blend mode |
| two-layers-add | the same two layers, the top layer's blend mode Add | – | |
| layer-opacity-50 | one Spherical layer, layer opacity (strength) 50 % | – | |

## Notes
TODO
