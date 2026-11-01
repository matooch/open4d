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

## Notes
TODO
