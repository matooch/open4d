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

Started, not finished (unattended window, 2026-11-20). What is now known from the probes:

- The two rigid-body commands exist and run from a script: one puts a "Dynamics Body" tag on the
  selected object (used for the moving body), the other puts a tag of the same type on the
  selected object for the static geometry — the probe applied the second to the plane and the
  first to the cube, and both objects then carry a tag whose attribute manager reads
  "Dynamics Body".
- Stepping the document frame by frame does run the simulation: with a 50 cm cube at y 300 over a
  default 400 x 400 plane and document 30 fps, the cube's global y read 300.000, 299.334,
  297.559, 294.677, 290.690, 285.600, 279.409, 272.118, 263.729, 254.245, 243.668, 231.998,
  219.238, 205.391, 190.456, 174.438 at frames 0 to 15. Every value is a global position read
  with the document set to that frame; no value here is interpolated.
- The rigid body tag's table, at its defaults, reads: Enabled on; Dynamic (Off / On / Ghost) On;
  Trigger (Immediately / At Velocity Peak / On Collision / By XPresso) Immediately with Trigger
  Velocity Threshold 10; Custom Initial Velocity off with Initial Linear Velocity (0, 0, 0) and
  Initial Angular Velocity (0, 0, 0); Set Initial State and Clear Initial State buttons; Show in
  Visualization on; plus the collision, mass, friction, bounce and force blocks further down
  the table.
- The scene-level dynamics settings (gravity, steps per frame, scale) were **not** located from
  the script: the document exposes no dynamics accessor, and no scene object of a dynamics type
  appeared in the object manager, so the Settings table at the top of this form is still empty.
  The fall in the probe above is therefore the new document's defaults, not a set of recorded
  values, and none of the nine cases is claimed.
- No case rows are filled and no golden data was written for this form: without the scene
  settings table, every case would record a simulation whose gravity and step count are unknown,
  which would be worse than leaving the rows empty.
