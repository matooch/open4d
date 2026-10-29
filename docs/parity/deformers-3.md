# Deformers III — observation (O19)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Method: exactly as [deformers-2.md](deformers-2.md) (O9). The input is a cube made with
its menu command, size_y 400, segments 4/8/4. The deformer is made through its menu
command and moved under the cube as its child.
- Export the cube as `tests/golden/<deformer>/<case>/mesh.obj`.
- `case.toml` lists every deformer parameter under our keys, and the cube under
  `[params.input]`.
- `observation = "docs/parity/deformers-3.md"`.

Unblocks: P1, the remaining deformers.

## Parameters (fill once per deformer)

For each deformer, list its Object tab. If the deformer has a Falloff or Field tab, note
that it is empty by default.

### Explosion (`explosion`)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### Shatter (`shatter`)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### Smoothing deformer (`smoothing-deformer`)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### Displacer (`displacer`)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### Polygon Reduction (`polygon-reduction`)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### Bevel deformer (`bevel-deformer`)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### Spline Wrap (`spline-wrap`)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### Wind (`wind`)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

## Cases

| Case | Deformer | Changed from default | Notes |
|---|---|---|---|
| explosion/defaults | Explosion | – | |
| explosion/strength-50 | Explosion | strength 50 % | |
| shatter/defaults | Shatter | – | |
| shatter/strength-50 | Shatter | strength 50 % | |
| smoothing-deformer/defaults | Smoothing deformer | – | Use a **cube with segments 4/4/4 and Fillet off**, made editable first, with point 0 moved by (50, 50, 50) through the script, so smoothing has something to do. Record the moved point |
| smoothing-deformer/iterations-10 | Smoothing deformer | iterations 10 (or the closest field) | the same input as defaults |
| displacer/defaults | Displacer | – | With no shader, note whether it moves anything |
| displacer/height-noise | Displacer | Shading: a Noise shader with default settings, Height 20 | the noise is seeded by its own defaults; record its seed |
| polygon-reduction/defaults | Polygon Reduction | – | input: a default sphere (16 segments) instead of the cube |
| polygon-reduction/50 | Polygon Reduction | strength 50 % | same input |
| bevel-deformer/defaults | Bevel deformer | – | input: a cube with segments 1/1/1 |
| bevel-deformer/offset-20 | Bevel deformer | offset 20, subdivision 2 | same input |
| spline-wrap/defaults | Spline Wrap | the spline link set to a default Helix (made through its menu command, at the origin) | Record the Helix's params under `[params.spline]` |
| spline-wrap/arc | Spline Wrap | the link set to a default Arc spline | |
| wind/frame-10 | Wind | – | the document at frame 10, 30 fps |

## Notes
TODO
