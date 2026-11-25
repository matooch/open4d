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
| `type` | Type | enum `linear` (linear, dual_quaternion, blended) | O34: Linear default; the reference's Spherical is `dual_quaternion`, its Blended is `blended` |
| `blend` | Blend | float 0..=1, default 0.5 (used by `blended`) | O34 (`skin.blend` 0.5) |
| `max_influences` | Max influences | int 0..=32, 0 = unlimited | H design |
| `normalise` | Normalise | bool true | H design |
| `enabled` | Enabled | bool true | O34 |

Linear blend skinning, per point `p`, in the object's local space (the deformer contract):

`p' = Σ_j w_j · K_j · p`, `K_j = inverse(O) · J_j · B_j⁻¹`

with `O` the object's world matrix now, `J_j` the joint's world matrix now and `B_j` its stored
bind pose. Weights are first reduced to the `max_influences` largest (ties: lower slot) and, if
`normalise`, scaled to sum 1; a point whose weights sum to 0 stays where it is.
**Without `normalise`** the linear skin is `p' = p + Σ_j w_j · (K_j · p − p)` (the weights are used as
stored: a sum above or below 1 scales the displacement and the rest of the weight keeps the point at
its bind position). This is what the reference does (`weights-readback`: the stored weights of the
rings at y = ±75 sum to 1.0000153, and `j2-bend-90-points` shows the posed points carrying that factor,
7e-4 off the normalised result); the parity tests set `normalise` off. The default stays on (own design).
Rule evidence: O34 `bind-pose` (K_j = identity), `j2-bend-45`, `j2-bend-90`, `j1-twist-90`,
`j2-scale`; exchange `skinned-arm` frame 0 (bind) and its posed bounding boxes (§8).

Weights in the golden data: the `weights` array of the O34 cases is rounded to 4 decimals (0.8333);
`weights-readback` holds them at full precision (tolerance 1e-6) and fixes the rule. With
`s = (y + 150) / 150` (the position along the chain in bone lengths): J1 = clamp(1 − s, 0, 1),
J3 = clamp(s − 1, 0, 1), J2 = 1 − J1 − J3, **each weight stored on its own as
`floor(w · 65535 + 0.5) / 65535`** (k/65535, round half up; the half cases follow the
floating-point error of `s`, so 5/6 · 65535 = 54612.5 rounds up at y = −125 and down at y = +125,
and at y = ±75 both J1/J3 and J2 round 32767.5 up: the stored sum is 65536/65535). All 106
points agree to 1e-12 with this rule. The test builds it from the bind-pose y of each point;
the 4-decimal copies are checked within 5e-5.

Dual quaternion type (the reference's **Spherical**, O34 `j2-bend-dq`, exact): each `K_j` becomes a unit
dual quaternion from its normalised rotation columns and translation (scale and shear are dropped),
blended with the sign chosen against the first influence, normalised, applied to the point. The
posed mesh of `j2-bend-dq` (J2 turned B 90, bbox x min -67.175144) equals this rule at 1e-4 on every point.

Blended type (O34 `j2-bend-90-blended`, exact): `p' = (1 - blend) · p_linear + blend · p_dual`, the
two skins above evaluated on the same weights, `blend` 0.5 by default. Evidence: the posed bbox x min
-57.337782 is the mean of Linear's -47.50042 and Spherical's -67.175144, and the whole mesh agrees at
1e-4. **Parity-pending**: `blend` values other than 0.5 (only the default was observed).

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
positions after `export_weld`, same face lists; the readouts at their own 1e-6). Eight of the O34
mesh cases are exact, two readouts are exact, one is form only.

| Case | Result |
|---|---|
| `rigging/bind-pose` | exact (max error 4e-7) |
| `rigging/j2-bend-45`, `j2-bend-90`, `j1-twist-90`, `j2-scale` | **exact** (were `KNOWN_MISMATCH` with 4.3e-4 to 1.5e-3; unpinned) |
| `rigging/j2-bend-90-single` | exact: every point weighted (0, 1, 0), the arm turns rigidly |
| `rigging/weights-readback` | exact at 1e-6: 106 x 3 weights |
| `rigging/j2-bend-90-points` | exact at 1e-6: the 106 posed points read from the deform cache |
| `rigging/j2-bend-90-matrix` | form only (J2 global matrix and rotation H 0, P 0, B 90): no mesh, no samples; pinned in `repo_golden` |
| `rigging/j2-bend-dq` | **exact**: Spherical = dual quaternion skinning (§3) |
| `rigging/j2-bend-90-blended` | **exact**: Blended = 50 % mean of the linear and the dual quaternion result (§3) |

What settled the four old pins (they were attributed to the reference's numeric noise before the
follow-up readouts): (1) the stored weights are k/65535 with the half cases rounding by floating-point
error, not by a plain `round` of the exact rule (only the y = ±75 rings differ: the previous quantised
attempt gave 0.499992 for J2 there where the reference stores 0.500008); (2) the skin does not
normalise those rows; (3) with both, the posed points of `j2-bend-90-points` agree to 1e-6 and every
mesh case to 1e-4. The twist case had no such rows; it matches because the exact rule now agrees at
the stored precision too.

The `weights` triples in `case.toml` are 4-decimal copies; the test builds the exact rule from the
point heights and requires it to agree with the recorded values within 5e-5.

Exchange `skinned-arm` (frame 15's bbox stays 2.07e-4 off, pinned in the test as the animated angle's noise: the static 45 degree mesh matches exactly) (`crates/rig/tests/rigging_parity.rs`, second test): the rig of O34 with
J2 keyed B 0 at frame 0 and 90 at frame 30 (linear, `forge_anim::scene_at`).
- frame 0 mesh equals `mesh.obj` (the bind pose): exact; bbox sample exact.
- frame 30 bbox sample: error 4.2e-4; frame 15: 4.0e-4. Pinned as `KNOWN_MISMATCH_FRAMES`
  (reference noise as above). The whole mesh at frames 15 and 30 stays within the noise
  envelope of the `j2-bend-45` and `j2-bend-90` meshes, which confirms the keyed angle is
  45 and 90 degrees (linear interpolation) and that J2's channel is B.
Other exchange formats (FBX, glTF, Alembic skins) are not read here.

Not modelled (observed in O34's parameter table, not used by any case): Skin Coord, Transform,
Length, Map, Mode, Force, List; the weight tag's vertex map.

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
   Without a pole: `h` = **minus** the root joint's current world Z axis minus its part along `c`,
   normalised (`reach-bend`, `reach-side`). The O35 joints rest aligned with the world axes
   (`rest-orientation`: H 0 / P 0 / B 0, axes identity) and the recorded solved frames have `z = -Z`,
   `x = -X` for a chain along Y, hence the minus; earlier text assumed a rest H = 180. A degenerate input (pole on the chord, Z along the
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
Rest orientation of the O35 joints is the identity (`rest-orientation`, exact at 1e-4); the solved
frames are the rest frames turned 180° about Y (x = -X, z = -Z), which rule 1 and 4 reproduce.
Parity-pending: bone axes other than Y, the IK/FK blend, strength, pole with a twisted root.

**Known mismatch `four-joints`**: our FABRIK gives `J2 (-0.9, 100, 0)`, `J3 (55.5, 182.6, 0)`;
the reference `J2 (99.87, -5.14, 0)`, `J3 (180.04, 54.62, 0)` (bone directions at -2.95°, 36.67°,
107.5°). CCD (tip to root and root to tip), a Jacobian pseudo-inverse and FABRIK from the straight
chain were tried; none reproduces it. The tip reaches the goal in both; only the redundancy
resolution differs. More data would settle it (the same chain from a different start pose).

**Known mismatch `four-joints-2`** (the same chain with J2 pre-rotated B 30° before the tag; after the solve
J2 reads B 74.385659): the reference gives J2 (−32.390, 94.609), J3 (50.009, 151.269), tip (150.0007, 150.0003)
(bone angles 108.9°, 34.5°, −0.7°); our FABRIK from that seed picks another elbow (error 33). Hypotheses
(budget three, after the pinned `four-joints`): (1) CCD tip to root, (2) CCD root to tip: both land far
away (J1 at 83° and 114°); (3) a **Jacobian-transpose flow** (`dθ = g · Jᵀ e`, g small, stop at the 0.01 /
1e-3 residual) reproduces `four-joints-2` to 0.01 (J2 (−32.383, 94.611), J3 (50.009, 151.28); the discrete step
and the stop test of the reference would be needed for 1e-4) but, from the straight `four-joints` seed, it
ends on the other side (J1 at 100.7°) where the reference goes to −2.9°. Not adopted: it explains one of two
cases. Both stay pinned; the tip reaches the goal in both.

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

Every expression, IK, constraint and motion tag has `priority_group` (`initial`, `animation`,
`expression`, `dynamics`, `generators`, in this order) and `priority` (integer, default 0).
Expression tags have the implicit `expression`, 0. Tags run sorted by (group, number, document
order); document order is an object, its tags, then its children. Each tag sees the writes of
the tags before it and of the tracks. All of this happens before any generator or deformer
is evaluated, so the `dynamics` and `generators` groups only order tags among themselves
until a dynamics stage exists. Layers with expressions off skip these tag evaluations.

## 13. Motion and display tags (brief 106)

Sources: [O27](../parity/animation-2.md) and [O54](../parity/tags-2.md). The app registers
these tags and runs them after animation in the ordered tag stage (§12). Tag evaluation
uses exact document time for tracks, including fractional frames.

- **Target**: `target` links an object. Aim the host's +Z at its evaluated world position,
  with bank zero; only rotation changes (`target-tag`). This case uses the automatic
  eased position keys, not linear keys.
- **Align to Spline**: `spline` links the path, `position` is a fraction (0–1, default 0),
  and `tangential` (default off) also aims +Z along the analytical tangent. Measure the
  path with 50 equal-parameter chords per cubic, invert accumulated length by linear
  interpolation, then evaluate the cubic. Cap the fraction at `0.9999`. For an almost
  vertical aim, use zero heading and signed 90° pitch. The two recorded Circle cases
  support these rules; other path types are parity-pending. Their position keys have
  collapsed handles in both time and value. Invert time with 19 bisections; at the exact
  midpoint take the lower interval, as in the recorded three-second track.
- **Vibrate**: the seed, amplitude and frequency drive deterministic bounded noise.
  O27 is a statistical check, not a match of the reference noise sequence.
  Own design, parity-pending: relative offsets add to evaluated position and rotation
  and multiply evaluated scale; Regular Pulse substitutes a sine at the set frequency.
- **Look at Camera**: the first scene camera supplies the aim direction. O54's manifest
  and OBJ pin heading 161.565051°, pitch and bank zero; multiple cameras are parity-pending.
- **Protection**: enabled position locks stop direct `transform.set` edits on the locked
  axes, preserving the existing values (O54).
- **Display / Compositing**: descriptor-backed settings do not change exported geometry,
  including when `seen_by_camera` is off (O54). The tags are stored but their viewport
  and render effects are not implemented by this brief.

For a parented host, convert world positions and aim vectors through the inverse parent
matrix before writing local coordinates (TRANSFORM.md; parity-pending for O27/O54).

## 14. IK, constraint and ordering parity (briefs 104, 105, 111)

IK (`crates/rig/tests/ik_parity.rs`, 11 cases, level B, every joint's position and three axes
within the recorded 1e-4):

- exact: `reach-bend`, `reach-far`, `pole-x`, `pole-z`, `twist` (worst 5.9e-5, the reference's own residual) and `rest-orientation` (no tag: the chain as built, axes identity);
- `KNOWN_MISMATCH`, noise of the reference's 0.01 threshold (the rule matches, the recorded
  numbers carry its residual; our result is exact): `reach-straight` (4.3e-2), `reach-side`
  (3.6e-3), `reach-near` (5.4e-4); the test requires the error to stay below 0.06 and the tip to
  reach the goal;
- `KNOWN_MISMATCH`, rule not derived: `four-joints`, `four-joints-2` (section 10), the tip reaches the goal.

Constraints (`crates/rig/tests/constraints_parity.rs`, 7 cases, six tracks each, 31 frames):
all 7 exact within 1e-4 when the driver is the curve the reference recorded (`psr-all`'s
position and H tracks; error below 1e-8). With an ideal smoothstep driver the worst error is
2.9e-4 (the reference's driver curve itself deviates up to 1.3e-4 degrees), checked against an
envelope of 1e-3. The recorded meshes (a cube at frame 15) are not compared. `UNSUPPORTED`
and `KNOWN_MISMATCH` are empty. Not in O36: `clamp-surface`, `ik-then-constraint`.

Ordering (`crates/rig/tests/tag_order.rs`, `crates/app/tests/rig_tags.rs`): own unit tests of
section 12; there is no reference case, so the order of IK and constraints against each other
is parity-pending (O36 ordering case not recorded).

## 15. Pose Morph tag (brief 107)

Status: **own design (D20), parity-pending (O37)**. No golden data exists: the reference app does
not let a script set the morph strength, so `docs/parity/pose-morph.md` has 0 of 7 cases. A
GUI-assisted capture (GUI-SESSIONS.md G9) will check these rules later.
Sources: the public manual's description of the feature (a tag that stores a base pose and named
morph poses of the points, each pose with a strength in percent, keyable in an animation mode, and a
relative and an absolute way of combining them; the manual's exact wording of the absolute mode was
not available to this implementation, so the absolute rule below is our reading) and the standard
blend-shape maths (a weighted sum of target offsets from a base pose). Code:
`crates/rig/src/pose_morph.rs`; the hook is `forge_generators::evaluate`; commands in
`crates/app/src/pose_morph_commands.rs`.

### 15.1 Parameters

Tag type `forge.pose_morph_tag`, Category `Tag`, on an object with points (the commands accept
polygon objects). The tag has `MAX_TARGETS = 16` target slots; the param list is generated from
that constant, so the attribute UI, the undo history, the keyframe tracks and the native file all
come from the descriptors.

| Param id | Label | Kind, default | Notes |
|---|---|---|---|
| `enabled` | Enabled | bool true | off: the tag does nothing |
| `mode` | Mode | enum `relative` (relative, absolute) | §15.2 |
| `base` | (hidden) | `Vec3List` | the base pose; empty until the first target is made |
| `name_i` | Name | string, default `Morph i+1` | `i` in 0..16 |
| `target_i` | (hidden) | `Vec3List` | absolute point positions of target `i`; empty = slot unused |
| `strength_i` | Strength | float, default 0, unit percent, animatable, **no hard range** | stored as a fraction: 1.0 is 100 % |

Strengths below 0 and above 100 % are allowed (the evaluation extrapolates); the manual states no
limit. A strength track is an ordinary track on `strength_i` (brief 088), so keys, interpolation
and loops work as for any float.

### 15.2 Evaluation

Let `c` be the object's own points (its geometry after generators and Make Editable, before any
deformer), `b` the `base` list, `t_i` the target lists and `s_i` the strengths. A slot is *active*
when `t_i` has as many points as `c` and `s_i != 0`. The tag does nothing when it is disabled, when
`b` has another point count than `c` (the topology changed), or when no slot is active. For every
point `k`:

- **Relative** (default): `p_k = c_k + Σ_i s_i · (t_i,k − b_k)`. A target is the offset from the base
  pose; the offsets are added onto whatever the object's points are now, so a morph follows a base
  that was edited or animated afterwards. At `s = 1` and `c = b` the point is exactly `t`.
- **Absolute**: `p_k = c_k + Σ_i s_i · (t_i,k − c_k)`. A target is a goal position: a single target at
  100 % puts the point exactly on `t_i,k` whatever `c` is, 50 % goes half way from the current point.
  Several targets add their pulls (`s_1 + s_2 > 1` overshoots).

With `c = b` the two modes agree for every strength; they differ only once `c` has moved away from
the base. Points where every active target equals the base are never moved (relative) so a morph
that only touches the upper half leaves the lower half bit-identical.

### 15.3 Order with the skin

The tag runs on the bind mesh: after the object's geometry and before its deformer children, so the
skin deformer (§3) skins the morphed points. The skin's weights and bind pose stay those of the
unmorphed mesh (the point count does not change). Own design, the usual order for blend shapes.

### 15.4 Making and editing targets (undoable, one `Tx` each)

- `capture_base(tx, tag, points)`: stores the base pose.
- `add_target(tx, tag, name, points) -> slot`: the first free slot. Fails without a base pose, with
  16 targets in use, or with a point count that differs from the base.
- `set_target(tx, tag, slot, points)` replaces a target's points, `clear_target(tx, tag, slot)` empties
  the slot and resets its name and strength; keys on the strength track stay (the slot is reused).
- Commands: `pose_morph.add_target` (`object`, optional `name`; creates the tag if the object has
  none, takes the object's current points, and captures them as the base first when the tag has no
  base yet, so a target made from the unchanged object equals the base), `pose_morph.update_target` (`tag`, `slot`; current points,
  or `points`), `pose_morph.clear_target`, `pose_morph.set_base`.

### 15.5 Not modelled

Position, rotation and scale (PSR) morphs of the object itself, vertex-map and material morphs, the
manual's mix/fade tools, and tag-to-tag link morphs are left out: the manual's description of them
was not available and O37's `psr-morph` case was not observed. Parity-pending in full (O37).
