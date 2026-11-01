# Deformers IV — observation (O45)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Method: as [deformers-2.md](deformers-2.md) (O9). Unless a row says otherwise:
- the input is a cube with size_y 400 and segments 4/8/4;
- the deformer is the cube's child;
- export to `tests/golden/<deformer>/<case>/mesh.obj`.

Linked helper objects (splines, cages, surfaces, cameras) are made through their menu
commands. Record each helper's parameters under `[params.helper]` in `case.toml`.

Unblocks: D/055, 056.

Record each deformer's parameter table once.

## Cases

| Case (folder) | Deformer | Setup / changes |
|---|---|---|
| spline-deformer/defaults | Spline | original spline: a default Arc; modifier spline: the same Arc rotated H 90°; both linked |
| spline-rail/defaults | Spline Rail | Rail splines: two default Helix splines, one at x −150 and one at x 150 |
| mesh-deformer/cage | Mesh (cage) | cage: a cube of size 300 × 500 × 300 with segments 1/1/1, made editable, linked and initialised (the "initialize" button or the scripting equivalent); then move cage point 0 by (−100, 0, 0) |
| surface/defaults | Surface | surface: a default plane at y 250, linked, initialised; then bend the plane with a Bend deformer (strength 90) |
| camera-deformer/grid | Camera | a camera at (0, 0, −800) linked, default grid; move grid point (1, 1) by (50, 50) in the deformer's own grid space |
| collision/sphere | Collision | colliders: a sphere (r 100) at (150, 0, 0) in its list, defaults |
| jiggle/frame-10 | Jiggle | the cube keyed in position X 0 → 300 over frames 0–10; frame-stepped from 0 to 10, exported at frame 10 |
| jiggle/frame-20 | Jiggle | the same, exported at frame 20 |
| delta-mush/defaults | Delta Mush | the cube made editable, point 0 moved by (80, 80, 80), then Delta Mush added with defaults |
| correction/defaults | Correction | under a Bend (strength 90) on the cube: Correction defaults, then move one corrected point through its point list |
| ffd-grid-5 | FFD | grid points 5/5/5, the centre grid point moved by (0, 100, 0) |
| explosion-fx/frame-10 | Explosion FX | defaults, frame 10 |
| box-gate/twist-narrow-x | Twist | follow-up from brief 050: O9 cube (segments 4/4/4), Twist angle 90°, mode Within Box, size (100, 250, 400) |
| box-gate/taper-narrow-x | Taper | same cube, Taper strength 50 %, Within Box, size (100, 250, 400) |
| box-gate/bulge-narrow-z | Bulge | same cube, Bulge strength 50 %, Within Box, size (400, 250, 100) |
| box-gate/bend-rotated | Bend | same cube, Bend strength 90°, Within Box, size (100, 250, 400), deformer rotated H 45° |
| box-gate/bend-angle | Bend | same cube, Bend strength 90°, angle 45°, Within Box, size (100, 250, 400) |
| box-gate/bend-equal | Bend | same cube (200 cm), Bend strength 90°, Within Box, size (200, 250, 200) exactly the extent |
| falloff/bend-offset | Bend | follow-up from brief 061: the O24 `bend-spherical` setup with the cube moved to (0, 100, 0) (the field stays at the origin) |
| falloff/bend-strength-200 | Bend | O24 `bend-spherical` with the field's Strength 200 % and Clamp Max off |
| falloff/shear-spherical | Shear | O9 cube, Shear defaults with strength 50, a default Spherical field in its falloff list |
| falloff/spherify-linear | Spherify | O9 cube, Spherify 100 %, a default Linear field in its falloff list |
| falloff/field-child | Bend | O24 `bend-spherical`, with the Spherical field made a child of the Bend deformer |

## Notes
TODO
