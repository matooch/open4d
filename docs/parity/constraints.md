# Constraints — observation (O36)

Status: observed (7 of 9 cases; Clamp inert and the IK-chain case not run — see Notes) · App version: Cinema 4D 2025.1.0 (25.010) · OS: macOS 26.7.2 (25G332, arm64) · Observer: Hermes · Date: 2026-11-20
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
| PSR | mode tag[1000] | target tag[10001], weight tag[10002], P tag[10005], S tag[10006], R tag[10007], Maintain Original tag[2100] | target none, weight 1.0, P on, S/R off | three P/S/R toggles; weight takes 0.5 (see psr-weight-50) |
| Aim | mode tag[1001] | target tag[20001], axis tag[2005], H/P/B tags [2001]–[2003] | axis Z+ | axis cycle X+/X−/Y+/Y−/Z+/Z−; setting tag[2005] to 0 or 5 changed nothing (see Notes) |
| Up | mode tag[1003] | up-vector target tag[40001] | none | only meaningful together with Aim |
| Parent | mode tag[1002] | target tag[30001], P tag[30004], S tag[30005], R tag[30006] | not captured; set explicitly per case | three P/S/R toggles |
| Clamp | mode tag[1004] | target tag[50001], weight tag[50002], strength tag[2400], mode tag[2401], to tag[2402], distance tag[2408], align tag[2403], as tag[2404], axis tag[2405], plane tag[2406], lock position tag[2407], use normals tag[2411] | – | every row accepted a value but none changed the result (see Notes) |

## Cases

| Case | Mode | Settings | Notes |
|---|---|---|---|
| psr-position | PSR | position only | Cube follows the driver exactly: X 0 → 200, Y 0 → 100, Z stays 0; rotation untouched (H stays 0) |
| psr-all | PSR | position, rotation, scale | As psr-position plus rotation: H 0 → 90 at frame 30 |
| psr-offset | PSR | position, with "maintain offset" (or the closest option) on, the cube starting at (50, 0, 0) | Offset preserved: X 50 → 250, i.e. driver position plus the cube's own 50 |
| psr-weight-50 | PSR | position, strength/weight 50 % | Weight 0.5 still gives a full follow (X 0 → 200): with a single target the weight normalises to 1.0, so this case records that behaviour rather than a blend |
| aim | Aim | target = driver, default axis | The cube at (0, 0, −200) turns to face the driver: H 0 → 315° (−45°), P 19.47°, B 8.13° at frame 30 |
| aim-up | Aim | as aim, plus an up vector: a second Null at (0, 500, 0) | Heading and pitch identical to aim, but B 8.13° → 341.57°: the Up mode (tag[1003]) and its target tag[40001] are live |
| parent | Parent | target = driver | The cube at (100, 0, 0) is attached rigidly: its own offset is kept and rotated with the driver, so frame 30 is (200, 100, 100) with H 90° |
| clamp-surface | Clamp | to a default sphere's surface (if supported), the cube at (300, 0, 0) | Not included: no clamp setting produced any observable effect |
| ik-then-constraint | a 2-joint IK chain (as in ik.md) whose goal is PSR-constrained to the driver | – | Not run this pass: the chain needs its own scene setup |

## Notes

- **Clamp is inert as scripted.** With the cube at (300, 0, 0) and a sphere of radius 100 at
  the origin, neither `distance` 150 with mode Min (already satisfied, so no change expected)
  nor `distance` 50 with mode Max (the cube starts outside and should be pulled in) moved the
  cube at any sampled frame. Varying `to` (Object/Surface), weight, strength and the axis rows
  made no difference either. Either the tag needs a different activation route in R25 or the
  rows are stubs; left out rather than recorded as a golden that asserts "nothing happens".
- **The Aim axis row did not respond.** Setting tag[2005] to 0 and to 5 while aiming at the
  same driver produced sample files identical to the default case (H 315°, B 8.1301° at frame
  30), so either that id is not the axis row or R25 ignores it. The axis value is recorded in
  each `case.toml` but no case asserts a per-axis orientation.
- **PSR weight needs two targets to be meaningful.** Only one target row (tag[10001]) is
  exposed by the tag description; a second target would need the repeated hidden ids, which
  were not identified. The 50 % case therefore documents normalisation to a single target.
- Parameter ids above were confirmed by writing them and reading the result back either from
  the tag (`mode`, `target`) or from the cube's transform (`maintain original`, the Up target).
- `aim`, `aim-up` and `parent` keep the cube at rest in X/Y/Z when the mode does not drive
  position — the tracks still record the full 31 frames.
