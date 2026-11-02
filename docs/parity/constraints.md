# Constraints — observation (O36)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Method: scripted, 30 fps. Unblocks: H/105, 111. Level B.

## Setup

- **Driver:** a Null keyed with the record command:
  - frame 0: position (0, 0, 0), rotation (0, 0, 0);
  - frame 30: position (200, 100, 0), rotation H 90°.
- **Constrained object:** a cube (size 20) carrying a **Constraint tag** in the mode the
  row gives, with the driver as its target.

Output per case:
- one track file per sampled value of the cube's **global** transform, frames 0–30:
  `samples-pos_x.json`, `-pos_y`, `-pos_z`, `-rot_h`, `-rot_p`, `-rot_b` (kind `track`,
  rotation in degrees);
- `mesh.obj` of the cube at frame 15.

Use `tests/golden/constraints/<case>/` and `feature = "constraints"`.

## Parameters (fill once per constraint mode)

| Mode | Label | Our key | Default | Options / Min / Max |
|---|---|---|---|---|

## Cases

| Case | Mode | Settings | Notes |
|---|---|---|---|
| psr-position | PSR | position only | |
| psr-all | PSR | position, rotation, scale | |
| psr-offset | PSR | position, with "maintain offset" (or the closest option) on, the cube starting at (50, 0, 0) | |
| psr-weight-50 | PSR | position, strength/weight 50 % | |
| aim | Aim | target = driver, default axis | The cube at (0, 0, −200) |
| aim-up | Aim | as aim, plus an up vector: a second Null at (0, 500, 0) | |
| parent | Parent | target = driver | The cube at (100, 0, 0) |
| clamp-surface | Clamp | to a default sphere's surface (if supported), the cube at (300, 0, 0) | |
| ik-then-constraint | a 2-joint IK chain (as in ik.md) whose goal is PSR-constrained to the driver | – | Shows tag priority |

## Notes
TODO
