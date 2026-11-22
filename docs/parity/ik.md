# Rigging II: IK — observation (O35)

Status: observed (11 of 11 cases: 9 baseline plus 2 follow-ups after brief 104, 2026-11-21) · App version: Cinema 4D 2025.1.0 (25.010) · OS: macOS 26.7.2 (25G332, arm64) · Observer: Hermes · Date: 2026-11-20
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

| Case | Setup | Record | Result |
|---|---|---|---|
| rest-orientation | the chain as built, no IK tag | every joint's local and global rotation (H, P, B) and its axes, written in the row and in `case.toml` | observed: no IK tag and no goal; every joint read back from the app — J1 at (0, 0, 0), J2 at (0, 100, 0), J3 at (0, 200, 0); local rotation H 0 / P 0 / B 0 and global rotation (−0.0, 0.0, 0.0) (the negative zero is what the app reports) for all three, global axes (1, 0, 0), (0, 1, 0), (0, 0, 1) — the rest chain is exactly aligned with the world axes. The per-joint readbacks are in `case.toml` under `params.j1` … `params.j3`; `samples.json` (kind `clones`) and `mesh.obj` per the form output. `golden info`: 24 vertices, 18 faces (4-gons 18), size (10, 210, 10), centre (0, 100, 0), normals 6, uvs 4 |
| four-joints-2 | the `four-joints` chain, but J2 pre-rotated B 30° before the IK tag is added | joint global positions and axes, as in `four-joints` | observed: J2 pre-rotated B 30° before the tag was added; once the default solver runs, J2 local rotation reads back H −0.0 / P 0 / B 74.385659 — the solver replaces the seed. Joint global positions: J1 (0, 0, 0), J2 (−32.390436, 94.608983, −0.000001), J3 (50.008785, 151.269095, 0), J4 (150.000736, 150.000329, 0), so the tip lands within 0.001 of the goal (150, 150, 0) with the tag threshold 0.01. The seed picks the opposite elbow from `four-joints`, whose J2 is (99.867606, −5.14406, 0) and J3 (180.044056, 54.619949, 0). Global axes for all four joints are in `samples.json` (kind `clones`) and in `case.toml`. `golden info`: 32 vertices, 24 faces (4-gons 24), size (194.407175, 162.682102, 10.000001), centre (57.860185, 74.99108, −0.0000005), normals 20, uvs 4 |

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

### Follow-up read-back, 2026-11-21 (brief 104)

- Reading chosen for `rest-orientation`: "the chain as built" is the form's 3-joint chain with the
  size-10 cube children every case uses for export, and "no IK tag" also leaves out the goal Null,
  since the goal belongs to the IK setup the row excludes.
- Global rotations were read with the app's matrix-to-rotation helper, so no rotation was derived by
  hand; the axes come straight from the global matrices.
- The pre-rotation of `four-joints-2` only seeds the solver: J2 reads 74.385659° of bank after the
  solve, not the 30° that was set, and the chain folds the other way while the tip still reaches the
  goal within the default threshold. The 30° is recorded in `case.toml` as `j2_pre_rotation_bank_deg`.
