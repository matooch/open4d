# Rigging II: IK — observation (O35)

Status: observed · App version: Cinema 4D 2025.1.0 (25.010) · OS: macOS 26.7.2 (25G332, arm64) · Observer: Hermes · Date: 2026-11-20
Method: scripted. Unblocks: H/104. Level B.

## Setup

- **Joints:** a 3-joint chain J1 (0, 0, 0) → J2 (0, 100, 0) → J3 (0, 200, 0), made through
  the joint command.
- **IK:** add an **IK tag** on J1 with its tip set to J3, and a goal Null made through the
  tag's "add goal" function or the Null command.
- **Pole:** for the pole cases, a pole Null at the given position, linked as the pole.

Output per case:
- `tests/golden/ik/<case>/samples.json`, kind `clones`, one entry per joint (J1, J2, J3)
  holding each joint's **global** matrix (pos, x, y, z), with colour (1, 1, 1), weight 1
  and visible true;
- `mesh.obj`: a cube of size 10 at each joint, made with a cloner in Object mode on the
  joints, or the scene's joints exported. If joints don't export, add three small cubes as
  children of the joints and export those.

Use `feature = "ik"`.

## Parameters (fill once)

### IK tag
| Label | Our key | Default | Options / Min / Max |
|---|---|---|---|
| Icon File / ID | `icon_file_id` | (empty) | filename |
| Icon Color | `icon_color` | 0 | bool |
| Color | `color` | [1, 0.9, 0.4] | color |
| Enable | `enable` | 1 | bool |
| Use IK | `use_ik` | 1 | bool |
| IK Solver | `ik_solver` | 2D | 2D, 3D |
| End | `end` | J3 | object link |
| Goal | `goal` | Goal | object link |
| IK/FK | `ik_fk_percent` | 0% | float |
| Strength | `strength_percent` | 100% | float |
| Preferred Rot | `preferred_rot_percent` | 0% | float |
| Goal Offset | `goal_offset_percent` | 100% | float |
| Axis | `axis` | Auto | Auto, X, Y, Z, -X, -Y, -Z |
| Twist | `twist_degrees` | 0 deg | float |
| Squash | `squash_percent` | 0% | float |
| Type | `type` | Position | Position, Uniform Scale, Bone Scale, Volume Scale |
| Distance | `distance` | 0 | float |
| Clamp | `clamp` | 0 | bool |
| Stretch | `stretch_percent` | 0% | float |
| Type | `type_secondary` | Position | Position, Uniform Scale, Bone Scale, Volume Scale |
| Distance | `distance_secondary` | 0 | float |
| Clamp | `clamp_secondary` | 0 | bool |
| Show FK | `show_fk` | 0 | bool |
| Show IK | `show_ik` | 0 | bool |
| Handle Line | `handle_line` | 1 | bool |
| Pole Vector | `pole_vector` | Root | None, Root, Joint |
| FK | `fk` | [1, 0, 0] | color |
| IK | `ik` | [0, 0, 1] | color |
| Handle | `handle` | [0, 1, 0] | color |
| Pole Vector | `pole_vector_secondary` | [1, 0, 1] | color |
| Keep Goal | `keep_goal` | 0 | bool |
| Constrain Goal | `constrain_goal` | 0 | bool |
| Update On Goal | `update_on_goal` | 0 | bool |
| Align Goal | `align_goal` | 0 | bool |
| Update IK on FK | `update_ik_on_fk` | 1 | bool |
| Max Iterations | `max_iterations` | 10000 | int |
| Threshold | `threshold` | 0.01 | float |
| Enable | `enable_secondary` | 0 | bool |
| Strength | `strength_secondary_percent` | 30% | float |
| Drag | `drag_percent` | 2% | float |
| Pos Hold | `pos_hold_percent` | 100% | float |
| Rot Hold | `rot_hold_percent` | 30% | float |
| Collisions | `collisions` | 0 | bool |
| Radius | `radius` | 5 | float |
| Friction | `friction_percent` | 10% | float |
| Bounce | `bounce_percent` | 30% | float |
| Gravity | `gravity` | -9.81 | float |
| Mode | `mode` | Include | Include, Exclude |
| Continuous Update | `continuous_update` | 1 | bool |
| Steps | `steps` | 1 | int |
| Auto | `auto` | 1 | bool |

## Cases

| Case | Goal position | Pole | Notes |
|---|---|---|---|
| reach-straight | (0, 200, 0) | none | No change; tip stays at (0, 200, 0), J2 at (0.04, 100.00, 0). |
| reach-bend | (100, 100, 0) | none | Bends at J2 (100.00, 0.00, 0); tip (100, 100, 0). |
| reach-near | (0, 50, 0) | none | Chain folds instead of shortening: J2 (96.82, 25.00, 0); tip (0, 50, 0). |
| reach-far | (0, 400, 0) | none | Does not stretch; chain stays straight at full length, tip (0, 200, 0). |
| reach-side | (150, 50, 50) | none | Solves in 3D: J2 (92.68, -28.03, 25.00); tip (150, 50, 50). |
| pole-x | (100, 100, 0) | pole Null at (0, 100, 200) | Elbow goes to z -66.67, opposite the pole; tip (100, 100, 0). |
| pole-z | (100, 100, 0) | pole Null at (0, 100, −200) | Mirror of pole-x: elbow at z +66.67; tip (100, 100, 0). |
| four-joints | a 4-joint chain, spacing 100, goal (150, 150, 0) | none | 4-joint chain solves: J2 (99.87, -5.14, 0), J3 (180.04, 54.62, 0); tip (150, 150, 0). |
| twist | reach-bend, plus the IK tag's twist/rotation parameter at 45° (if present) | none | Twist 45 deg rotates the plane: J2 (85.36, 14.64, 50.00); tip unchanged (100, 100, 0). |

## Follow-up cases (planner, after brief 104)

| Case | Setup | Record |
|---|---|---|
| rest-orientation | the chain as built, no IK tag | every joint's local and global rotation (H, P, B) and its axes, written in the row and in `case.toml` |
| four-joints-2 | the `four-joints` chain, but J2 pre-rotated B 30° before the IK tag is added | joint global positions and axes, as in `four-joints` |

## Notes

- All nine cases solved with the tag's default 2D solver (Max Iterations 10000, Threshold 0.01);
  every reachable tip lands within 0.004 of its goal (worst: reach-side, 0.0032 in x;
  the others are below 1e-4).
- **Joints are not parented automatically.** The joint command leaves three separate roots
  (probe: `n_roots` 3, J2/J3 parent `None`). The chain here was parented through the object parent field;
  without that the tag sees three independent joints.
- **Out of reach does not stretch:** reach-far (goal 400 on a 300 chain) leaves the chain
  straight at full length with the tip at (0, 200, 0) - squash/stretch default to 0, so the
  chain only aims at the goal.
- **A pole Null bends the elbow away from itself:** a pole at (0, 100, +200) puts J2 at
  z = -66.67; the pole at -200 mirrors it to +66.67. Only the plane changes, never the tip.
- **Twist rotates the chain plane** about the chain axis: 45 deg moves J2 to
  (85.36, 14.64, 50.00) while the tip stays at (100, 100, 0).
- Recorded per case: the full IK tag dict, the joint dict and the cube dict
  (`[params.ik_tag]`, `[params.joint]`, `[params.cube]`).
- The mesh is a Cube of size 10 added as a child of each joint (joints alone do not export):
  24 vertices / 18 quads for 3 joints, 32 / 24 for 4.
- Goal and pole Nuls come from the Null command (5140); the IK tag is `MakeTag(1019561)`,
  tip `tag[2000]`, goal `tag[2001]`, pole `tag[2005]`.
