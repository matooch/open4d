# Deformer falloff through fields — observation (O24)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Method: exactly as [bend.md](bend.md): a cube with size_y 400 and segments 4/8/4, with
the deformer as its child. A field is added to the **deformer's** field (falloff) list.
- Export the cube as `tests/golden/deformer-falloff/<case>/mesh.obj`, with
  `feature = "deformer-falloff"` and `observation = "docs/parity/deformer-falloff.md"`.
- `[params]` holds the deformer's parameters and `deformer = "<name>"`.
- `[params.field]` holds every parameter of the field, including its position.
- `[params.input]` holds the cube's parameters.

Unblocks: PLAN phase 7 "with falloff", which makes the fields of P2 matter for deformers.

## Cases

| Case | Deformer | Field in its list | Notes |
|---|---|---|---|
| bend-spherical | Bend, strength 90° | Spherical, size 200, at (0, 200, 0) | |
| bend-spherical-inner | Bend, strength 90° | the same, inner offset 0 % | |
| bend-linear | Bend, strength 90° | Linear, direction Y+, length 200 | |
| bend-box | Bend, strength 90° | Box, size (400, 200, 400) | |
| twist-linear | Twist, angle 180° | Linear, direction Y+ | |
| taper-spherical | Taper, strength 50 % | Spherical, size 200, at (0, 200, 0) | |
| bulge-linear | Bulge, strength 100 % | Linear, direction X+ | |
| bend-field-strength-50 | Bend, strength 90° | Spherical as in bend-spherical, layer strength 50 % | |
| bend-empty-list | Bend, strength 90° | an empty field list, but with the falloff/field option switched on if one exists | Does an empty list mean "full effect"? |

## Notes
TODO
