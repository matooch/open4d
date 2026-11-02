# Dynamics: rigid bodies — observation (O39)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Method: scripted, 30 fps, project 0–90.

Simulate by stepping the document frame by frame from 0, so the simulation runs in order.
At every frame 0–90, read each listed object's **global** position (cm) and rotation H/P/B
(degrees).

Output:
- `tests/golden/dynamics/<case>/samples-<object>-<param>.json` (kind `track`, param
  `pos_x` … `rot_b`, `tolerance` 1e-3);
- `mesh.obj`: the whole scene at frame 90.

Use `feature = "dynamics"`. Parity is level C (trajectory thresholds).
Unblocks: I/115, 116.

Before the first case, record: the dynamics scene settings (gravity, steps per frame,
scale, …) and the rigid body tag's parameters.

| Setting | Our key | Default | Min / Max / Options |
|---|---|---|---|

## Cases

| Case | Scene | Objects sampled | Notes |
|---|---|---|---|
| drop-cube | a cube (size 50) at (0, 300, 0), dynamic rigid body; a plane (default, 400×400) at y 0, collider | cube | Record the first-contact frame |
| drop-bounce | as drop-cube, with the cube's bounce at its maximum allowed value | cube | |
| drop-friction-slide | the cube starts at (0, 25, 0) with initial velocity (300, 0, 0) cm/s, friction default | cube | Sliding distance |
| drop-friction-high | as drop-friction-slide, with friction at its max | cube | |
| stack | 3 cubes (size 50) stacked at y 25, 75, 125 on the plane, all dynamic | cube1, cube2, cube3 | Should stay at rest |
| slope-sphere | a sphere (r 25) on a plane rotated B 20°, dynamic | sphere | Rolling |
| tumble | a cube at (0, 300, 0) rotated (30, 20, 10), dropped | cube | Rotations matter |
| clones-fall | a cloner (Grid 3×1×3, size 200) of cubes (size 30) at y 300, with a dynamic body tag on the cloner (per-clone bodies) | clone positions: the clones kind per frame is too large, so sample only clones 0 and 4 as objects `clone0`, `clone4`, using the clone matrices at each frame | |
| gravity-half | drop-cube, scene gravity at half the default | cube | |

## Notes
TODO
