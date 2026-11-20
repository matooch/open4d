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

### 1.3 Joint tools (library functions on `Tx`; command wrappers are not registered, see §7)
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
See §4 below once implemented.

## 5. Mirroring (brief 113)
See below once implemented.

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
