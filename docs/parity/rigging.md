# Rigging I: joints and skinning — observation (O34)

Status: observed (5 of 5 follow-up cases after brief 101, recorded 2026-11-21) · App version: 25.010 · OS: macOS 26.7.2 (25G332) · Observer: Hermes (deepseek-v4.1-flash:1017) · Date: 2026-11-19
Method: scripted. Unblocks: H/100, 101, 103. Skinning with **explicit** weights is level A.

## Setup

- **Mesh:** a cylinder made through its menu command: radius 20, height 300, height
  segments 12, rotation segments 8, at the origin, with Orientation +Y. Then **Make
  Editable**.
- **Joints:** a chain of 3 joints made through the joint command:
  - J1 at (0, −150, 0), J2 at (0, 0, 0), J3 at (0, 150, 0), each a child of the previous;
  - all rotations 0.
- **Binding:** add a Skin deformer under the cylinder and a Weight tag on the cylinder
  that lists J1, J2 and J3.

Folder: `tests/golden/rigging/<case>/mesh.obj`, with `feature = "rigging"`.

**Weights set by script, rule "explicit":** for each point with height y:
- J1 weight = clamp((−y)/150, 0, 1);
- J3 weight = clamp(y/150, 0, 1);
- J2 weight = 1 − J1 − J3.

Set these values through the weight tag's scripting interface. Record in `case.toml`
`[params]`: `weights_rule = "explicit-linear"` and `weights` as one
`[j1, j2, j3]` triple per point, in point order.

## Parameters (fill once)

### Joint
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Position | `joint.position` | (0, 0, 0) | not observed: bounds not probed | not observed: bounds not probed |
| Rotation | `joint.rotation` | (0, 0, 0) | not observed: bounds not probed | not observed: bounds not probed |
| Scale | `joint.scale` | (1, 1, 1) | not observed: bounds not probed | not observed: bounds not probed |
| Length | `joint.length` | 150 | not observed: bounds not probed | not observed: bounds not probed |
| Weight | `joint.kinematics_weight` | 1.0 | not observed: bounds not probed | not observed: bounds not probed |
| Stretch | `joint.stretch` | 1.0 | not observed: bounds not probed | not observed: bounds not probed |
| Rotation Order | `joint.rotation_order` | HPB | – | – |
| Priority | `joint.priority` | PriorityData | – | – |
| Bone | `joint.bone` | To Child | – | – |
| Axis | `joint.axis` | Z | – | – |
| Display (object) | `joint.display_object` | Standard | – | – |
| Size (object) | `joint.size_object` | Length | – | – |
| Display (joint) | `joint.display_joint` | Axis | – | – |
| Size (joint) | `joint.size_joint` | Bone | – | – |
| Enabled | `joint.enabled` | on | – | – |
| Visible in Editor | `joint.visible_editor` | Default | – | – |
| Visible in Renderer | `joint.visible_renderer` | Default | – | – |
| X-Ray | `joint.xray` | off | – | – |
| Display Color | `joint.display_color` | Automatic | – | – |
| Color | `joint.color` | (0.4235, 0.8980, 0.5098) when Custom | – | – |

Option lists as read from the parameter table: Bone = Null / Axis / From Parent / To Child;
Axis = X / Y / Z; Display object = None / Standard / Box / Line / Stick / Polygon;
Size object = Custom / Length; Display joint = None / Axis / Sphere (Wire) / Circle / Sphere;
Size joint = Custom / Bone; Rotation Order = HPB (default) / XYZ / XZY / YXZ / YZX / ZXY / ZYX;
Visible (in Editor and in Renderer) = Default / On / Off; Display Color = Material / Layer / Automatic / Custom.
A joint also carries a Fields tab (a field list, Shape, Invert, Visible); it was not used.
The parameter table returned no declared minimum or maximum, and the form's "type a huge and a negative value" probe
was not run here, so the numeric cells say not observed rather than a guess.

### Skin deformer
| Label | Our key | Default; Options |
|---|---|---|---|
| Coord | `skin.coord` | Joints; Object / Joints / Deformer |
| Type | `skin.type` | Linear; Linear / Spherical / Blended — no Dual Quaternion in this build |
| Transform | `skin.transform` | Object; Bone / Object |
| Length | `skin.length` | None; None / Uniform Scale / Bone Scale / Volume Scale |
| Blend | `skin.blend` | 0.5; – |
| Map | `skin.map` | none; – |
| Mode | `skin.mode` | Exclude; Include / Exclude |
| Force | `skin.force` | 0; – |
| List | `skin.list` | empty; – |

Standard object rows are also present: Enabled, Visible in Editor / Renderer, X-Ray, Display Color,
Priority, and the full Coord. tab.

### Weight tag
| Label | Our key | Default; Options |
|---|---|---|---|
| Weight | `weight_tag.weight` | 1.0; – |
| Expose on Object | `weight_tag.expose_on_object` | off; – |
| Joints | `weight_tag.joints` | the chain added to the tag; – |
| Vertex Map | `weight_tag.vertex_map` | none; – |

The tag's scripting surface reads and writes a point's weights, the whole weight map and its dirty state, and can
transfer a weight map — there is no auto-weight call.

## Cases

| Case | Pose (set after binding) | Notes |
|---|---|---|
| bind-pose | none | The mesh must equal the unbound cylinder |
| j2-bend-45 | J2 rotation B 45° | bbox (−43.587932, −150, −20) .. (120.208153, 120.207946, 20) |
| j2-bend-90 | J2 rotation B 90° | bbox (−47.500420, −150, −20) .. (150, 20, 20); the chain folds flat, so y never passes 20 |
| j1-twist-90 | J1 rotation H 90° | Rotation about the chain axis; bbox unchanged to 3e-4 (( −20.000305, −150, −20) .. (20.000305, 150, 20)) because the silhouette is a surface of revolution; the vertex normals do rotate |
| j2-bend-dq | J2 rotation B 90°, Skin type set to Dual Quaternion (or the option that is not linear) | Dual Quaternion does not exist in this build; the enum is Linear / Spherical / Blended, so the case was observed with Spherical (value 1). bbox (−67.175144, −150, −20) .. (150, 20.000002, 20) — Spherical reaches ~19.7 units further in −X than Linear |
| j2-scale | J2 scale (1, 1.5, 1) | bbox (−20, −150, −20) .. (20, 225, 20): the 1.5 Y scale propagates along the chain to the geometry above J2 (150 × 1.5 = 225) |
| auto-weights | rebind with the app’s automatic weighting (the bind command or the weight tag’s auto-weight function). **Skip if it opens a dialog.** Pose J2 B 90° | not observed: the weight tag scripting surface has no auto-weight function (only per-point weight reads and writes and weight-map read and transfer), and the interactive Auto Weight command opens a modal dialog, which the case says to skip. No per-point readout was produced |

## Follow-up cases (planner, after brief 101)

The Skin deformer matches `bind-pose` exactly, but the four posed cases differ from our linear blend by
4e-4 to 1.5e-3 with identical topology (RIG.md §8). These cases tell apart weight storage, joint-matrix
precision and export rounding. Same rig and weights as above unless a row says otherwise.

| Case | Setup | Record | Result |
|---|---|---|---|
| weights-readback | the rig as bound, no pose | Read back every point's three weights through the weight tag's per-point read, at full precision (no rounding). `samples.json` kind `track`, param `weight_j1` / `weight_j2` / `weight_j3`, frame = point index | observed: 106 points, three weights each at full precision, one file per parameter (`samples-weight_j1.json`, `samples-weight_j2.json`, `samples-weight_j3.json`, frame = point index); first point (1, 0, 0), point 52 = (0, 0.166674296177615, 0.833325703822385), last point (0, 0, 1), which rounds to the spot checks in the Notes |
| j2-bend-90-points | J2 rotation B 90° | Read the deformed point positions straight from the evaluated object (the deform cache), at full precision, not through OBJ. `samples.json` kind `track`, params `x` / `y` / `z`, frame = point index | observed: 106 positions read from the evaluated deform cache at full precision in `samples-x.json` / `samples-y.json` / `samples-z.json` (frame = point index); maximum deviation from the bind pose 169.99999996, posed bbox (−47.50042, −150, −20) … (150, 20, 20), identical to the bbox recorded for `j2-bend-90` |
| j2-bend-90-matrix | J2 rotation B 90° | Read J2's global matrix (offset and the three axes) at full precision, and its rotation as shown in the Coordinates manager. Write all twelve numbers in the row | observed: J2 global matrix at full precision = offset (0.0, 0.0, 0.0), axis 1 (6.123233995736766e-17, −1.0, 0.0), axis 2 (1.0, 6.123233995736766e-17, 0.0), axis 3 (0.0, 0.0, 1.0) — all twelve numbers also stored in `case.toml`; the object rotation reads back H 0, P 0, B 90 degrees as the Coordinates manager shows it |
| j2-bend-90-single | J2 rotation B 90°, but every point weighted 100 % to J2 (weights (0, 1, 0)) | `mesh.obj` and the same full-precision point readout as `j2-bend-90-points`. With one joint per point there is no blending, so any difference is matrix or export precision | observed: every point weighted (0, 1, 0) to J2, so the arm turns rigidly about the origin: posed bbox (−150, −20, −20) … (150, 20, 20), maximum deviation from the bind pose 169.99999996; positions at full precision in `samples-x/y/z.json`. `golden info`: 106 vertices, 112 faces (3-gons 16, 4-gons 96), size (300, 40, 40), centre (0, 0, 0), normals 27, uvs 121 |
| j2-bend-90-blended | J2 rotation B 90°, Skin Type = Blended (Blend left at its default) | `mesh.obj` | observed: the Skin Type row read back Blended before rendering (its cycle is Linear, Spherical, Blended); posed bbox (−57.337782, −150, −20) … (150, 20.000001, 20), which differs from the Linear pose, so Blended blends the two joints differently. `golden info`: 106 vertices, 112 faces (3-gons 16, 4-gons 96), size (207.337782, 170.000001, 40), centre (46.331109, −64.9999995, 0), normals 114, uvs 121 |

## Notes
- Each case is a fresh document: cylinder through its menu command (radius 20, height 300, height
  segments 12, rotation segments 8, Orientation +Y) → Make Editable (106 points, 112 polygons), then the
  three-joint chain and the skin. The rig was rebuilt per pose instead of keyframing, so every
  `mesh.obj` is a single-pose export and `case.toml` names the pose.
- Joint chain as built: J1 at (0, −150, 0); J2 is a child of J1 at a relative position (0, 150, 0), so its
  absolute position is (0, 0, 0); J3 is a child of J2 at a relative position (0, 150, 0), absolute (0, 150, 0).
  `joint_positions` records the absolute positions the form asks for; `joint_chain` records the parent order.
- Weights were written through the weight tag (its per-point weight write) using the form’s rule. The 106 [J1, J2, J3]
  triples are in `case.toml` (`weights`, point order) and their sha256 is `weights_sha256`, so the table can be
  checked without diffing 106 rows. Spot checks: point 0 = (1, 0, 0), point 52 = (0, 0.1667, 0.8333); every
  triple sums to 1.
- `bind_pose = true`: the skin was bound with the chain in its rest pose before any pose was applied, so
  `bind-pose` returns the undeformed cylinder (±20 / ±150 / ±20, 106 points, 112 polygons).
- Spherical is the only non-linear skin type this build offers; the `j2-bend-dq` row explains the substitution.
  Blended (value 2) exists as well but was not exported.
- `j1-twist-90` twists about the chain axis, which for a surface of revolution cannot move the bounding box; the
  case still matters because the normals rotate and the validator compares vertex and normal data, not just the box.
- Exports are the app’s own OBJ with the calibrated settings used by the other forms (no unit or axis
  conversion), written straight into the case folder.

### Follow-up read-back, 2026-11-21 (brief 101)

- Each three-value readout is written as one file per parameter (`samples-weight_j1.json` … and
  `samples-x.json` / `samples-y.json` / `samples-z.json`), the layout the dynamics cases already
  use, because the samples validator wants exactly one parameter per samples file (a combined
  `samples.json` with three parameter names is rejected: "param: expected a string").
- The five new case files carry the full `weights` array instead of the sibling files' weight
  hash; the hash's exact serialisation was not observable, so no hash was invented.
- Cross-checks against the committed cases: the read-back weights reproduce the spot checks in the
  Notes above (point 52 = (0, 0.1667, 0.8333) at four decimals), and the posed bounding box of
  `j2-bend-90-points` matches the `j2-bend-90` bbox exactly, so the rebuild is the same rig.
- The Skin deformer's type row reads Linear by default; Blended and Spherical are the other two
  entries in its cycle.
