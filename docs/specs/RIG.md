# Rigging I: joints, weights, skin (forge-rig)

Status: briefs 100, 101, 103 (this file is owned by the rigging track).
Sources: observation `docs/parity/rigging.md` (O34; golden `tests/golden/rigging/*`),
`docs/parity/exchange.md` (skinned-arm, O44; golden `tests/golden/exchange/skinned-arm`),
design `docs/roadmap/areas/H-rigging.md`, and published literature (linear blend skinning,
Kavan et al. dual quaternion skinning 2007, Baran and Popovic "Automatic rigging and
animation of 3D characters" 2007 for bone heat).

STATUS rows: §1 joint (brief 100), §2 weight tag and §3 skin (101), §4 auto weights (103),
§5 mirroring (113), §6 display (112).

## 0. Conventions

- Rotations follow `docs/specs/TRANSFORM.md`: `(h, p, b)` turn about Y, X, Z by the negated
  angle. J2 "B 45" in O34 is `Rz(-45°)`. World matrices are `scene.world_matrix(id)`, so a
  parent's scale propagates to its children (j2-scale: J2 scale (1, 1.5, 1) puts J3 at
  y = 225).
- The crate registers three node types through `Rig` (a `Plugin`): `forge.joint` (object),
  `forge.weight_tag` (tag), `forge.skin` (deformer). Param ids are lower-case, derived from the
  O34 "Our key" column.

## 1. Joint object (brief 100)

Category `Object`. Besides the three transform params every hierarchy node has:

| Param id | Label | Kind, default | Source |
|---|---|---|---|
| `length` | Length | float 150, distance | O34 table |
| `bone` | Bone | enum `to_child` (null, axis, from_parent, to_child) | O34 |
| `axis` | Axis | enum `z` (x, y, z) | O34 |
| `display_object` | Display (object) | enum `standard` (none, standard, box, line, stick, polygon) | O34 |
| `size_object` | Size (object) | enum `length` (custom, length) | O34 |
| `display_joint` | Display (joint) | enum `axis` (none, axis, sphere_wire, circle, sphere) | O34 |
| `size_joint` | Size (joint) | enum `bone` (custom, bone) | O34 |
| `kinematics_weight` | Weight | float 1 | O34 (used by IK, brief 104) |
| `stretch` | Stretch | float 1 | O34 (used by IK) |
| `enabled` | Enabled | bool true | O34 |

Not modelled: Rotation Order (the transform has HPB only; TRANSFORM.md), priority, colour
rows, Fields tab (none were used in O34). No bounds were observed, so no hard ranges.

### 1.1 Bone length
`bone_vector(scene, joint)`: with `bone = to_child` and a first child joint, the child's local
position (the bone in this joint's local space, before this joint's scale); otherwise
`axis_unit(axis) · length`. `joint_length` is its norm. Observed chain: J2 is a child of J1 at
(0, 150, 0), so the default length 150 and the chain spacing agree (O34 Notes).

### 1.2 Chain world matrices and bind pose
`chain_world_matrices(scene, root)` lists `(joint, world)` for the joint and its joint
descendants, depth first. The bind pose of a joint is its world matrix at bind time, stored
relative to the skinned object (§2).

### 1.3 Joint tools (library functions on `Tx`; command wrappers in §7)
- `create_chain(tx, parent, points, prefix, align)`: joints named `prefix1`, `prefix2`, …
  (O34 names J1, J2, J3); the first at `points[0]` (parent space), each next a child at the
  difference of the points. With `align` each joint but the last is aligned to its child.
- `align_joint(tx, joint, axis)`: the local `axis` (x, y, z) points at the first child joint by
  the shortest rotation from the current orientation (twist is kept); every child keeps its
  world matrix (its local transform is recomputed). No child joint: unchanged.
- `mirror_chain(tx, root, plane, rename)`: copies the joint subtree next to `root` with world
  matrices `S · M · S` (`S` the reflection through the plane `yz`, `xz` or `xy`, so no negative
  scale), names through `mirror_name` (§5.1).
Own design; not observed.

### 1.4 Bone display
`bone_display(scene, joint)` gives the geometry the viewport draws: the joint's world origin and
its first child joint's world origin, and an octahedron around the bone
(`octahedron(a, b, size)`: apex at both ends, four waist points at 0.1 of the length from `a`
with radius 0.1 of the length). Size comes from the length when `size_object = length`.
Own design (O34 gives the option lists only).

## 2. Weight tag (brief 101)

Category `Tag`, on the mesh object (O34: "a Weight tag on the cylinder that lists J1, J2 and J3").

| Param id | Label | Kind, default | Source |
|---|---|---|---|
| `weight` | Weight | float 1 | O34 |
| `expose_on_object` | Expose on Object | bool false | O34 |
| `joints` | Joints | link list of objects | O34 |
| `weights` | (hidden) | `Vec3List`, sparse `(slot, point, weight)` triples, sorted by point then slot | own |
| `bind_pose` | (hidden) | `Vec3List`, 4 entries per joint: the x, y, z axes and the translation of `inverse(object world at bind) · joint world at bind` | own |

`WeightMap` is the typed view: `get(point) -> &[(slot, w)]`, `set`, `normalise`, `prune`.
Zero weights are not stored. `bind(tx, tag)` writes `bind_pose` from the current scene.
O34 vertex-map input and the weight-tag scripting API are not modelled.

## 3. Skin deformer (brief 101)

Category `Deformer`, child of the mesh object; it reads the first weight tag of that object.

| Param id | Label | Kind, default | Source |
|---|---|---|---|
| `type` | Type | enum `linear` (linear, dual_quaternion) | O34 (Linear default); dual quaternion is ours |
| `max_influences` | Max influences | int 0..=32, 0 = unlimited | H design |
| `normalise` | Normalise | bool true | H design |
| `enabled` | Enabled | bool true | O34 |

Linear blend skinning, per point `p`, in the object's local space (the deformer contract):

`p' = Σ_j w_j · K_j · p`, `K_j = inverse(O) · J_j · B_j⁻¹`

with `O` the object's world matrix now, `J_j` the joint's world matrix now and `B_j` its stored
bind pose. Weights are first reduced to the `max_influences` largest (ties: lower slot) and, if
`normalise`, scaled to sum 1; a point whose weights sum to 0 stays where it is.
Rule evidence: O34 `bind-pose` (K_j = identity), `j2-bend-45`, `j2-bend-90`, `j1-twist-90`,
`j2-scale`; exchange `skinned-arm` frame 0 (bind) and its posed bounding boxes (§8).

Weights in the golden data: the `weights` array is rounded to 4 decimals (0.8333); the
reference used the exact rule (O34: J1 = clamp(−y/150, 0, 1), J3 = clamp(y/150, 0, 1),
J2 = 1 − J1 − J3). The test builds the exact rule from the bind-pose y of each point and checks
it against the recorded triples within 5e-5.

Dual quaternion option (own, not parity-tested): each `K_j` becomes a unit dual quaternion
from its normalised rotation columns and translation (scale and shear are dropped), blended with
the sign chosen against the first influence, normalised, applied to the point. O34 had no
dual quaternion type; its Spherical type is §8.

## 4. Auto weights (brief 103)

Bone heat (Baran and Popovic 2007), `auto_weights(mesh, bones, options)`; one bone per joint
(`bones_for_joints`: the joint's world position to its first child joint's, a point for a joint
without a child joint, in the object's space). For bone `b` solve

`(L + D H) w_b = D H p_b`

with `L` the cotangent stiffness matrix of the triangulated mesh (negative cotangent weights
clamped to 0 so the matrix stays positive definite), `D` the barycentric vertex areas, `H` the
diagonal heat `c / d²` of every vertex that has a visible bone (`d` = distance to the nearest
visible bone, floored at 1e-4 of the bounding box diagonal; `c = heat`, default 1), and `p_b`
the indicator of the vertices whose nearest visible bone is `b`. A bone is visible from a vertex
when the segment from the vertex to the bone's closest point crosses no triangle that does not
use the vertex (`visibility`, default on). `L` is dimensionless and `D H` too, so the result does
not depend on the model's scale. The system is solved by Jacobi-preconditioned conjugate
gradients (own code; the design note's nalgebra-sparse is not used: not in the workspace).
Weights are clamped at 0, optionally pruned (`prune`, relative), normalised to sum 1; a
vertex no heat reaches (a detached component without a visible bone) takes weight 1 on its
nearest bone. Deterministic: bone order, vertex order and tie-breaking are fixed.
`auto_bind(tx, tag, mesh, options)` stores the weights on the tag and binds the current pose.

Parity: level C. O34's auto-weight case was **not observed** (the weight tag has no auto-weight
call and the app's command opens a dialog; docs/parity/rigging.md), so there is no reference
data. The test (`crates/rig/tests/auto_weights.rs`) uses the nearest bone as partner, which
heat diffusion approaches on a tube: on the O34 arm (106 points, bones J1-J2, J2-J3, J3 point)
the dominant joint equals the nearest bone's for 89 of 89 points (ties skipped), threshold 90 %.
It also checks ring symmetry, a monotone J1 weight along the arm, that a detached triangle takes
its nearest bone, determinism, and that a 0.8 rad bend with these weights stretches no edge by
more than 60 %. The "auto-weights" O34 case is therefore UNSUPPORTED (no data), not matched.

## 5. Mirroring (brief 113)

Own design (no observation). Joint chains are mirrored by `mirror_chain` (§1.3); this section
adds names and weights.

### 5.1 Names
`mirror_name(name, RenameRule::LeftRight)`, first match wins:
1. a `Left` / `Right` word (also `left`/`right`, `LEFT`/`RIGHT`) is swapped, case kept;
2. a lone `L` / `R` (or `l` / `r`) token between `_`, `.`, `-` or spaces is swapped
   (`Arm_L` -> `Arm_R`, `hand.R.001` -> `hand.L.001`, `L_arm` -> `R_arm`);
3. otherwise `_mirror` is appended (`Spine` -> `Spine_mirror`).
`RenameRule::Suffix(s)` appends `s`. The brief's rules `L_`/`R_`, `.L`/`.R`, `_left`/`_right`
are all covered by 1 and 2.

### 5.2 Weights
`joint_mirror_map(scene, joints, rename)` gives for every slot the slot of the joint named by
`mirror_name` (itself for centre joints and joints without a twin).
`mirror_weights(points, weights, slot_map, plane, from, centre_tolerance, max_distance)`:
every point on the side opposite to `from` takes the weights of the nearest source-side point
to its reflection, joint slots renamed by `slot_map`; points within `centre_tolerance` of the
plane keep their weights; a reflection with no source point within `max_distance` is counted as
`unmatched` and left alone. `mirror_tag_weights` does this for a weight tag (centre tolerance
1e-6 and distance limit 1e-3 of the bounding box diagonal) and stores the result;
`add_joints_to_tag` appends the mirrored joints to the tag's list. Nearest search is brute force
(`O(n²)`).

Test (`crates/rig/tests/mirroring.rs`): a cube body with a centre joint, a left arm chain
named `arm_L_1`, `arm_L_2`, weights authored on x > 0 only, the arm mirrored with `mirror_chain`,
weights mirrored; posing the left arm B +0.6 and the right arm B -0.6 gives a mesh where every
point has a mirror image (1e-9); the mirrored rows equal the reflected copies with swapped
slots; twin lookup by name; unmatched and side handling.

## 7. Editor integration (brief 192)

Own design; no observation. The rig crate cannot depend on `forge-command` (command ->
generators -> modifiers -> rig), so the three command wrappers live in `forge-app`
(`crates/app/src/rig_commands.rs`, registered by `default_command_registry`), like `param.edit`.
Every command is one transaction (one undo step) and fails without changing anything.

| Command | Args | Does |
|---|---|---|
| `joint.create_chain` | `points` (at least one `[x,y,z]`, in `parent` space), `parent` (node id, optional), `prefix` (default `Joint`), `align` (`x`, `y`, `z`, optional) | `create_chain`; selects the root joint; returns `{nodes}` root first |
| `joint.align` | `nodes` (default: the selection), `axis` (default `z`) | `align_joint` for every joint in the list (parents before children); non-joints are an error |
| `joint.mirror` | `nodes` (default: the selection; each must be a joint), `plane` (`yz` default, `xz`, `xy`), `suffix` (optional) | `mirror_chain` for each root; `suffix` selects `RenameRule::Suffix`, otherwise `LeftRight`; selects the new roots; returns `{nodes}` |

Display (VIEWPORT.md §13.4): the editor draws every joint's bone and joint marker and, for a
selected joint, the weight squares of the objects whose weight tag lists it.

## 8. Parity cases and what is pinned

Test: `crates/rig/tests/rigging_parity.rs` (tolerance from the case manifest: 1e-4 on exported
positions after `export_weld`, same face lists). 1 of 5 O34 mesh cases is exact; the other
four are pinned, plus one unsupported.

| Case | Result |
|---|---|
| `rigging/bind-pose` | exact (max error 4e-7): the skin at the bind pose returns the unbound cylinder |
| `rigging/j2-bend-45` | KNOWN_MISMATCH, max error 9.0e-4; topology and face lists identical |
| `rigging/j2-bend-90` | KNOWN_MISMATCH, max error 1.45e-3 |
| `rigging/j1-twist-90` | KNOWN_MISMATCH, max error 4.3e-4 |
| `rigging/j2-scale` | KNOWN_MISMATCH, max error 5.7e-4 |
| `rigging/j2-bend-dq` | UNSUPPORTED: the case was observed with the Spherical skin type; its rule is not derived (no formula was fitted) |

Why the four are pinned and not wrong: the rule (§3) puts every point where the expected
mesh has it up to an error of at most 1.5e-3 on coordinates of size 150, with the identical
topology; a wrong rule (rotation sign, bone offset, scale propagation) would be off by
units. The residual is the reference's own numeric noise, three measurements:
1. The twist case moves every vertex by an exact quarter turn about the chain axis, so the
   only error is the reference's: the expected mesh itself has `-0.000305 .. 20.000305` where
   the exact value is `0 .. 20`, i.e. an angle error of about 1.5e-5 rad (4.3e-4 at radius 20 to
   150).
2. Solving the bend-90 mesh for the effective weight of each point (`p' = p + w (Rp - p)`) gives
   the exact rule plus multiples of 2^-17 = 7.6e-6 (y = -125: 0.166659 vs 0.166667; y = 75:
   1.000015), the size of 1/65535 rounding. Quantising the weights to k/65535 and applying them
   as `p + Σ w_j (K_j p - p)` halves the error (bend-45 3.6e-4, bend-90 6.9e-4, scale 1.9e-4)
   but the twist stays at 4.3e-4, so no scheme reached 1e-4. Using the recorded 4-decimal
   weights instead is worse (3.9e-3).
3. `bind-pose` is exact, so the bind matrices are right.
Pins are guarded by `NOISE_ENVELOPE = 2e-3` (a regression of the rule leaves it) and by the
assertion that the case still fails the strict comparison. They are not a tolerance.

The `weights` triples in `case.toml` are 4-decimal copies; the test builds the exact rule from the
point heights and requires it to agree with the recorded values within 5e-5.

Exchange `skinned-arm` (`crates/rig/tests/rigging_parity.rs`, second test): the rig of O34 with
J2 keyed B 0 at frame 0 and 90 at frame 30 (linear, `forge_anim::scene_at`).
- frame 0 mesh equals `mesh.obj` (the bind pose): exact; bbox sample exact.
- frame 30 bbox sample: error 4.2e-4; frame 15: 4.0e-4. Pinned as `KNOWN_MISMATCH_FRAMES`
  (reference noise as above). The whole mesh at frames 15 and 30 stays within the noise
  envelope of the `j2-bend-45` and `j2-bend-90` meshes, which confirms the keyed angle is
  45 and 90 degrees (linear interpolation) and that J2's channel is B.
Other exchange formats (FBX, glTF, Alembic skins) are not read here.

Not modelled (observed in O34's parameter table, not used by any case): Skin Coord, Transform,
Length, Blend, Map, Mode, Force, List; Spherical and Blended types; the weight tag's vertex map.

## 9. Weight tools (brief 102)

Own design; GUI session G6 (brush modes, falloff, weight manager layout) is unanswered, so every
rule here is **parity-pending**. Source: standard skinning practice (public literature); no
observation.

### 9.1 Brush
`Brush { mode, radius, strength, falloff, target }` (`forge_rig::paint`). `radius > 0` in object
units, `strength` in `[0, 1]`, `target` (Set only) in `[0, 1]`, default 1. Falloff by the
distance `d` of a point from the stroke centre (`t = d / radius`, zero at `t >= 1`):
`constant` 1, `linear` `1 - t`, `smooth` `1 - (3 t^2 - 2 t^3)`. The factor of a point is
`f = strength * falloff(d)`.

`stroke(weights, points, adjacency, slot, centre, brush)` changes the weight `w` of joint `slot`
of every point inside the radius, then keeps the row normalised:
| mode | new `w` |
|---|---|
| `add` | `min(w + f, 1)` |
| `subtract` | `max(w - f, 0)` |
| `set` | `w + (target - w) f` |
| `smooth` | `w + (avg - w) f`, `avg` the mean `slot` weight of the point's edge neighbours (0 for a missing weight; a point without neighbours is unchanged) |

Normalise: the other slots of the row are scaled by `(1 - w) / (sum of the others)` so the row
sums to 1; when the others sum to 0 the row keeps `w` as it is (a point that has only this
joint and `w < 1` stays below 1 until normalised by hand). Rows that held no weights and end
up with `w = 0` stay empty. Points are processed from the old weights (a smooth stroke does not
see its own changes). `adjacency` comes from the mesh polygons (`point_adjacency`): the points
that share an edge, sorted, without duplicates.

### 9.2 Tools
`smooth_weights(weights, adjacency, iterations, factor)`: `iterations` times, every point's row
becomes `(1 - factor) row + factor mean(neighbour rows)` (all slots at once, so sums stay);
points without neighbours keep their row. `normalise` and `prune` are the `WeightMap` methods
(§2); `mirror_tag_weights` is §5.2.

### 9.3 Commands (forge-app, `crates/app/src/weight_commands.rs`)
Each is one transaction and fails without changing anything. `object` is the skinned object
(default: the selected object) and must carry a weight tag; the mesh is its unskinned
evaluation, brush centres are in object space.
| Command | Args | Does |
|---|---|---|
| `weights.paint` | `object`, `joint` (node id of a joint in the tag's list), `center` `[x,y,z]`, `radius`, `strength` (default 1), `mode` (`add` default, `subtract`, `set`, `smooth`), `falloff` (`smooth` default, `linear`, `constant`), `target` | one `stroke`; returns `{points}` the number of points inside the radius |
| `weights.normalise` | `object` | `WeightMap::normalise` |
| `weights.prune` | `object`, `threshold` (default 0.01) | `WeightMap::prune` |
| `weights.smooth` | `object`, `iterations` (default 1), `factor` (default 0.5) | `smooth_weights` |
| `weights.mirror` | `object`, `plane` (default `yz`), `from` (`positive` default, `negative`) | `mirror_tag_weights`; returns `{copied, unmatched}` |

### 9.4 Weight manager (UI)
A panel (Window menu, Weight Manager; docks at the right bottom) for the selected object's weight tag: the tag's joints as a
list (name, point count with a weight), the active joint for painting, the brush settings (mode,
radius, strength, falloff) and buttons Normalise, Prune, Smooth, Mirror that run the commands.
Painting in the viewport sends `weights.paint` with the picked surface point; a stroke dragged
over several frames is merged into one undo step by the UI (one `weights.paint` per drag
sample, grouped). Test: egui_kittest drives the panel and one paint sample.

Tests: `crates/rig/tests/weight_paint.rs` (brush modes, falloff, normalise rule, smooth,
prune, adjacency), `crates/app/tests/weight_commands.rs` (commands, undo), `crates/ui/tests/weight_manager.rs`.

## 10. IK tag (brief 104)

Source: parity observation O35 ([ik.md](../parity/ik.md), `tests/golden/ik`, level B). Code:
`crates/rig/src/ik.rs`. Node type `forge.ik_tag` (a tag on the root joint of the chain).

| Param | Kind | Default | Notes |
|---|---|---|---|
| `enabled` | bool | on | off or an unresolvable setup writes nothing |
| `end` | link (object) | none | the tip joint; the chain is `end` and its ancestors up to the host |
| `goal` | link (object) | none | goal: its world position |
| `pole` | link (object) | none | optional pole object |
| `twist` | float, angle | 0 | radians; rotates the bend plane about the chord |
| `strength` | float, percent | 1 | 0..1, weight of the IK result |
| `ik_fk` | float, percent | 0 | 0 = IK, 1 = FK; the joint rotations are slerped between the two |
| `priority_group`, `priority` | enum, int | expression, 0 | section 12 |

`threshold` (0.01) and `max_iterations` (10000) of the reference are the stop rule of the
chain solver (below); the secondary solver, squash/stretch, goal offset and the display options
of the reference tag are not derived (`Unsupported`: ignored).

**Rules** (chain `J1..Jn`, root `P`, goal `G`, chord `c = (G-P)/|G-P|`, bone lengths `L_i` taken
from the FK world positions):

1. **Bend plane (hinge `h`)**. With a pole object at `Q`: `q = (Q-P)` minus its part along `c`,
   `h = normalize(q × c)`; the elbow goes to the side opposite the pole (`pole-x`, `pole-z`).
   Without a pole: `h` = the root joint's current world Z axis minus its part along `c`,
   normalised (`reach-bend`, `reach-side`). A degenerate input (pole on the chord, Z along the
   chord) falls back to the root Z, then to any axis perpendicular to `c`. **Twist** rotates `h`
   about `c` by `-twist` (`twist`: 45 deg).
2. **Positions, two bones**. `d = |G-P|` clamped to `[|L1-L2|, L1+L2]` (the chain never
   stretches: `reach-far` is straight, tip at full length; `reach-near` folds). The root bone
   points along `c` turned about `h` by `+α`, `cos α = (L1² + d² - L2²) / (2 L1 d)`; the tip lies
   on the chord at distance `d`. One bone: aim along `c`.
3. **Positions, three or more bones**: FABRIK in the plane through `P` with normal `h` (FK
   positions and goal projected onto it), until the tip is within `threshold` of the goal or
   `max_iterations` passes; an unreachable goal gives the straight chain along `c`.
4. **Frames**. A joint's bone axis `a` is the unit direction of its first child joint's local
   position; its up axis `u` is the cyclic partner of the dominant axis of `a` (`Y→Z`, `Z→X`,
   `X→Y`), made perpendicular to `a`. The world rotation maps `(a, u, a×u)` to `(d, h, d×h)`,
   with `d` the direction to the next joint. For the O35 chain (bone along local Y, up local Z)
   this is `y = d`, `z = h`, `x = y × z`, exactly the recorded matrices (all joints of a case
   share `z`). The tip keeps its FK local rotation (`J3 = J2` in every case).
5. **Output**. Each joint's local rotation `W_parent⁻¹ · W` as `transform.rot` (h, p, b), mixed
   with the FK local rotation by `strength · (1 - ik_fk)` (quaternion slerp; the weights are
   not observed: parity-pending). Positions are not written.

Observation caveats. The reference stops at its 0.01 threshold, so its numbers carry residuals
up to 0.04 (`reach-straight`, J2 x = 0.043; `reach-side` tip off by 0.003; `reach-near` 5e-4).
They are outside the recorded tolerance 1e-4, so those cases are pinned (section 13).
Rest orientation of the O35 joints is not recorded; the harness gives every joint `H = 180°`
(local x = -X, z = -Z), which reproduces the recorded frames of the straight chain.
Parity-pending: bone axes other than Y, the IK/FK blend, strength, pole with a twisted root.

**Known mismatch `four-joints`**: our FABRIK gives `J2 (-0.9, 100, 0)`, `J3 (55.5, 182.6, 0)`;
the reference `J2 (99.87, -5.14, 0)`, `J3 (180.04, 54.62, 0)` (bone directions at -2.95°, 36.67°,
107.5°). CCD (tip to root and root to tip), a Jacobian pseudo-inverse and FABRIK from the straight
chain were tried; none reproduces it. The tip reaches the goal in both; only the redundancy
resolution differs. More data would settle it (the same chain from a different start pose).

## 11. Constraint tag (brief 105)

Source: O36 ([constraints.md](../parity/constraints.md), `tests/golden/constraints`, level B).
Code: `crates/rig/src/constraint.rs`. Node type `forge.constraint_tag`.

| Param | Kind | Default | Notes |
|---|---|---|---|
| `enabled` | bool | on | |
| `mode` | enum | `psr` | `psr`, `aim`, `parent`, `clamp` |
| `targets` | link rows | empty | rows are targets; the row strength is the target weight |
| `position`, `rotation`, `scale` | bool | on, off, off | PSR and Parent toggles |
| `maintain_offset` | bool | off | PSR: keep the offset captured at activation |
| `offset_pos`, `offset_rot`, `offset_scale` | vec3 (hidden) | 0, 0, 1 | the captured offset |
| `aim_axis` | enum | `z` | `x`, `-x`, `y`, `-y`, `z`, `-z`: the local axis that points at the target |
| `up_target` | link | none | aim: the object the up axis points at |
| `up_axis` | enum | `y` | local axis turned toward the up target |
| `clamp_min`, `clamp_max` | float, distance | 0, 0 | clamp: distance limits to the target (0 = off) |
| `strength` | float, percent | 1 | 0..1, mix with the unconstrained transform |
| `priority_group`, `priority` | enum, int | expression, 0 | section 12 |

**Target.** The weighted mean of the enabled rows: position and scale by weighted mean,
rotation by the sign-aligned weighted quaternion sum, normalised. Weights are normalised by
their sum, so one target always follows fully (`psr-weight-50`).

**PSR.** With `P`/`R`/`S` on: position = target position (`psr-position`), rotation = target
rotation (`psr-all`), scale = target scale. With `maintain_offset` the result is target +
`offset_pos` (a world delta, `psr-offset`), target rotation · `offset_rot`, target scale ·
`offset_scale` (the rotation and scale offsets are parity-pending: O36 only shows the position
offset). Components that are off keep the object's own value.

**Parent** (`parent`). The object keeps the transform it had relative to the target at
activation: `world = T · offset` where `offset` (pos/rot/scale) is the object's matrix in the
target's space; the `P`/`R`/`S` toggles pick which components of the result replace the object's own.

**Aim** (`aim`, `aim-up`). `d` = direction from the object to the target. Without an up target
the object turns by the minimal rotation (swing) taking its current axis `aim_axis` to `d`
(`aim`: H 315, P 19.47, B 8.13 at frame 30). With an up target the rotation maps the aim axis to
`d` and the up axis to the up direction made perpendicular to `d` (`aim-up`: B 341.57).
O36 saw no response to the axis row, so only `z` is verified; the other axes are parity-pending.

**Clamp.** O36 saw no effect from any clamp setting, so there is no golden case. We implement
the distance clamp (`clamp_min`/`clamp_max` from the object to the target, along the line) as our
own behaviour, tested by unit tests only; spline and surface clamps are `Unsupported` (ignored).

**Activation offset.** `capture_offset(scene, tag)` returns the param writes (`offset_*`) for the
scene's current state; the caller applies them when the tag is activated (frame 0 in the tests).

**Write-back.** The new world matrix becomes a local transform through the parent's inverse
(`pos`, scale from column lengths, rotation as h, p, b); only enabled tags with a valid target write.

## 12. Tag priority and evaluation order (brief 111)

Source: own design after the public manual's tag priority (group plus number); the order of IK
and constraints is checked by unit tests, O36 recorded no combined case (the
`ik-then-constraint` row was not run, see [constraints.md](../parity/constraints.md)).
Code: `crates/rig/src/tags.rs`; hook: `forge_app::schedule::animate` (CORE.md section 5a).

Every expression, IK and constraint tag has `priority_group` (`initial`, `animation`,
`expression`, `dynamics`, `generators`, in this order) and `priority` (integer, default 0).
Expression tags have the implicit `expression`, 0. Tags run sorted by (group, number, document
order); document order is an object, its tags, then its children. Each tag sees the writes of
the tags before it and of the tracks. All of this happens before any generator or deformer
is evaluated, so the `dynamics` and `generators` groups only order tags among themselves
until a dynamics stage exists. Layers with expressions off skip all three tag kinds.

## 13. Parity cases and what is pinned (briefs 104, 105, 111)

IK (`crates/rig/tests/ik_parity.rs`, 9 cases, level B, every joint's position and three axes
within the recorded 1e-4):

- exact: `reach-bend`, `reach-far`, `pole-x`, `pole-z`, `twist` (worst 5.9e-5, the reference's own residual);
- `KNOWN_MISMATCH`, noise of the reference's 0.01 threshold (the rule matches, the recorded
  numbers carry its residual; our result is exact): `reach-straight` (4.3e-2), `reach-side`
  (3.6e-3), `reach-near` (5.4e-4); the test requires the error to stay below 0.06 and the tip to
  reach the goal;
- `KNOWN_MISMATCH`, rule not derived: `four-joints` (section 10), the tip reaches the goal.

Constraints (`crates/rig/tests/constraints_parity.rs`, 7 cases, six tracks each, 31 frames):
all 7 exact within 1e-4 when the driver is the curve the reference recorded (`psr-all`'s
position and H tracks; error below 1e-8). With an ideal smoothstep driver the worst error is
2.9e-4 (the reference's driver curve itself deviates up to 1.3e-4 degrees), checked against an
envelope of 1e-3. The recorded meshes (a cube at frame 15) are not compared. `UNSUPPORTED`
and `KNOWN_MISMATCH` are empty. Not in O36: `clamp-surface`, `ik-then-constraint`.

Ordering (`crates/rig/tests/tag_order.rs`, `crates/app/tests/rig_tags.rs`): own unit tests of
section 12; there is no reference case, so the order of IK and constraints against each other
is parity-pending (O36 ordering case not recorded).
