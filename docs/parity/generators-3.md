# Generators III — observation (O20)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Method: scripted ([agents/OBSERVER.md](../agents/OBSERVER.md)). Level A. Inputs are made
through their menu commands and placed as the generator's children in the order given.
- Export the generator to `tests/golden/<feature>/<case>/mesh.obj`.
- `case.toml` lists every generator parameter under our keys, with each child under
  `[params.input]`, `[params.input2]` and so on, in child order, as the existing
  generator cases do.
- `observation = "docs/parity/generators-3.md"`.

Unblocks: PLAN phase 6, plus deeper coverage of the generators we already have.

## Parameters (fill once per new generator)

### Metaball (`metaball`)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### Atom Array (`atom-array`)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### Spline Mask, measured through an Extrude as in [splines-2.md](splines-2.md) (`spline-mask`)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

## Cases

| Case (folder) | Generator | Children | Changed from default |
|---|---|---|---|
| metaball/two-spheres | Metaball | two default spheres at x −60 and x 60 | – |
| metaball/hull-50 | Metaball | the same | hull value 50 % (or the closest field) |
| metaball/editor-sub | Metaball | the same | editor subdivision 20 |
| atom-array/cube | Atom Array | a default cube | – |
| atom-array/radii | Atom Array | a default cube | cylinder radius 10, sphere radius 20, subdivision 6 |
| spline-mask/union | Spline Mask, inside an Extrude as in splines-2 | two default Circle splines, the second at x 150 | mode Union (record the default mode) |
| spline-mask/subtract | the same | the same | mode A subtract B (or the second option) |
| boole/r25-each-mode | Boole | two default cubes, the second at (50, 50, 50) | one case per Boolean Type, named `r25-<mode>`, with **R25 defaults** for everything else |
| subdivision/r25-types | Subdivision Surface | a default cube | one case per Type option, named `r25-type-<option>`, at the R25 default levels |
| symmetry/r25-planes | Symmetry | a default cube at x 100 | one case per mirror plane, named `r25-plane-<option>` |
| sweep/rail | Sweep | default Circle (profile), default Helix (path), a second Helix with end radius 400 (rail) | – |
| sweep/banking | Sweep | default Circle, default Helix | Banking off (record the default) |
| sweep/scale-end | Sweep | default Circle, default Helix | end scale 20 %, end rotation 180° |
| loft/three-circles | Loft | three default Circles at y 0, 200 and 400, the middle one with radius 100 | – |
| loft/linear | Loft | the same | Linear Interpolation on |
| lathe/partial | Lathe | default Rectangle at x 300 (so it doesn't cross the axis) | angle 180° |
| lathe/movement | Lathe | the same | movement 200 |

## Notes
TODO
