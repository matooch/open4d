# Deformer falloff through fields — observation (O24)

Status: observed · App version: Cinema 4D R25 (version number 25010) · OS: macOS (Darwin 25.6.0) · Observer: observer session (MCP) · Date: 2026-10-29
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
All nine cases exported (`tests/golden/deformer-falloff/<case>/mesh.obj`, 162 vertices each).

- **How the field gets into the deformer's list:** the Create-menu command for a field adds
  it to the list of the deformer that is selected at that moment (list had 0 layers before,
  1 after). The probe used exactly that and added nothing by hand; one layer per case.
  The deformer's list is the parameter labelled "Field" on its Fields tab.
- **Empty list = full effect.** `bend-empty-list` is the fully bent cube (y range −200 to
  259.15, x range −100 to 359.15); no extra falloff/field switch exists on the deformer, so
  none was set. A deformer with no field behaves as if the whole space were weight 1.
- **Every field case differs from the empty-list mesh**, so the field list is applied by
  the deformer: bend-spherical changes 108 of 162 vertices, bend-spherical-inner 120,
  bend-linear 96, bend-box 41, bend-field-strength-50 121, taper-spherical 121,
  bulge-linear 121, twist-linear 121 (max displacement 81 to 459 units).
- bend-spherical vs bend-spherical-inner (inner offset 50 % vs 0 %) differ in 68 vertices;
  bend-spherical vs bend-field-strength-50 (layer strength 100 % vs 50 %) differ in 69.
  The field's own Strength stayed 100 % in every case; layer strength is the list row's
  value (`layer_strength_percent`), blend mode Normal.
- The Spherical field's size parameter is labelled "Size" in the field's attributes and is
  stored as `size` in `[params.field]`; Linear Length/Direction and Box Size as shown.
- Linear fields were left at their default length 100 where the row gives none
  (twist-linear, bulge-linear); the field sits at the origin unless the row gives a position.
- The deformer's Curvature and Fillet parameters (Bend) are hidden in the shown mode and are
  not in `[params]`; they stay at their defaults.
- Exporter: same OBJ settings as the other R25 cases (export-settings.md).
