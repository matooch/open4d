# Follow-ups to O5–O9 — observation (O11)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Method: scripted ([agents/OBSERVER.md](../agents/OBSERVER.md)). Each section adds cases next
to the job it follows up, in that job's folder and with that job's `case.toml` rules.
Don't edit or delete the existing cases. Add the new rows to **this** file, not to the
original form.

## 1. Fields (follows O7, `tests/golden/fields/`, `observation = "docs/parity/followups-1.md"`)

The O7 cases `spherical-inner`, `box-inner` and `spherical-interp-linear` repeated their
defaults. Replace their purpose with:

| Case | Field | Changed from default |
|---|---|---|
| spherical-inner-0 | Spherical | Inner Offset 0 % |
| spherical-inner-80 | Spherical | Inner Offset 80 % |
| box-inner-0 | Box | Inner Offset 0 % |
| linear-inner-50 | Linear | Inner Offset 50 % |
| spherical-steps-4 | Spherical | Contour Mode Step, Steps 4 |
| spherical-quantize-4 | Spherical | Contour Mode Quantize, Steps 4 |
| spherical-strength-50 | Spherical | Remapping Strength 50 % |
| spherical-scaled | Spherical | object scale (2, 1, 1) |

## 2. Cloner (follows O8, `tests/golden/cloner/`)

| Case | Mode | Changed | Effector |
|---|---|---|---|
| grid-random-seed-7 | Grid Array | – | Random, seed 7 (other values default) |
| grid-random-rot | Grid Array | – | Random, position off, rotation on (45°, 45°, 45°) |
| radial-xy | Radial | plane XY | – |
| linear-mode-endpoint | Linear | count 5, mode End Point (or the second Mode option), position (0, 200, 0) | – |

## 3. Deformers II (follows O9)

| Case (folder) | Deformer | Changed |
|---|---|---|
| wrap/spherical | Wrap | type Spherical (or the option that is not the default) |
| wrap/width-90 | Wrap | width angle 90° |
| squash-stretch/expand-0 | Squash & Stretch | factor 50 %, Expand 0 % |
| squash-stretch/expand-200 | Squash & Stretch | factor 50 %, Expand 200 % |

## 4. Modeling tools (follows O6, `tests/golden/modeling-tools/`)

| Case | Cube | Selection | Tool + settings |
|---|---|---|---|
| optimize-real | 1/1/1 editable | top polygon, then the Disconnect command (it duplicates its 4 points) | then the Optimize command on all points; record the polygon and point counts before and after |
| extrude-bevel | 1/1/1 | top polygon | Extrude, offset 50, Bevel 10 |
| extrude-subdiv | 1/1/1 | top polygon | Extrude, offset 50, Subdivision 2 |
| inset-subdiv | 1/1/1 | top polygon | Extrude Inner, offset 20, Subdivision 1 |
| bevel-poly-extrusion0 | 1/1/1 | top polygon | Bevel (polygon mode), offset 20, Extrusion 0 (the hidden field from O6) |

## 5. Sweep cap type (follows O5)

Sweep with a default Circle spline as the profile (first child) and a default Rectangle
as the path (second child). Export it to `tests/golden/rechecks/sweep-defaults/mesh.obj`,
then record the cap faces (count and corners) and the Caps Type read.

## Notes
TODO
