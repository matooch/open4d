# Rigging I: joints and skinning — observation (O34)

Status: observed · App version: 25.010 (c4d.GetC4DVersion() = 25010) · OS: macOS 26.7.2 (25G332) · Observer: Hermes (deepseek-v4.1-flash:1017) · Date: 2026-11-19
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

Option lists as read from the parameter table: Bone (ID 2008) = Null / Axis / From Parent / To Child;
Axis (2002) = X / Y / Z; Display object (2003) = None / Standard / Box / Line / Stick / Polygon;
Size object (2009) = Custom / Length; Display joint (2005) = None / Axis / Sphere (Wire) / Circle / Sphere;
Size joint (2010) = Custom / Bone; Rotation Order (928) = HPB (default) / XYZ / XZY / YXZ / YZX / ZXY / ZYX;
Visible (901, 902) = Default / On / Off; Display Color (907) = Material / Layer / Automatic / Custom.
A joint also carries a Fields tab (FieldList 440000245, Shape 1026402, Invert, Visible); it was not used.
The parameter table returned no DESC_MIN / DESC_MAX, and the form's "type a huge and a negative value" probe
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
Priority (2009), and the full Coord. tab.

### Weight tag
| Label | Our key | Default; Options |
|---|---|---|---|
| Weight | `weight_tag.weight` | 1.0; – |
| Expose on Object | `weight_tag.expose_on_object` | off; – |
| Joints | `weight_tag.joints` | the chain added to the tag; – |
| Vertex Map | `weight_tag.vertex_map` | none; – |

The tag's scripting surface (CAWeightTag) is GetWeight / SetWeight / GetWeightCount / GetIndexWeight /
GetWeightMap / SetWeightMap / GetWeightDirty / WeightDirty / TransferWeightMap — there is no auto-weight call.

## Cases

| Case | Pose (set after binding) | Notes |
|---|---|---|
| bind-pose | none | The mesh must equal the unbound cylinder |
| j2-bend-45 | J2 rotation B 45° | bbox (−43.587932, −150, −20) .. (120.208153, 120.207946, 20) |
| j2-bend-90 | J2 rotation B 90° | bbox (−47.500420, −150, −20) .. (150, 20, 20); the chain folds flat, so y never passes 20 |
| j1-twist-90 | J1 rotation H 90° | Rotation about the chain axis; bbox unchanged to 3e-4 (( −20.000305, −150, −20) .. (20.000305, 150, 20)) because the silhouette is a surface of revolution; the vertex normals do rotate |
| j2-bend-dq | J2 rotation B 90°, Skin type set to Dual Quaternion (or the option that is not linear) | Dual Quaternion does not exist in this build; the enum is Linear / Spherical / Blended, so the case was observed with Spherical (value 1). bbox (−67.175144, −150, −20) .. (150, 20.000002, 20) — Spherical reaches ~19.7 units further in −X than Linear |
| j2-scale | J2 scale (1, 1.5, 1) | bbox (−20, −150, −20) .. (20, 225, 20): the 1.5 Y scale propagates along the chain to the geometry above J2 (150 × 1.5 = 225) |
| auto-weights | rebind with the app’s automatic weighting (the bind command or the weight tag’s auto-weight function). **Skip if it opens a dialog.** Pose J2 B 90° | not observed: the weight tag scripting surface has no auto-weight function (only GetWeight / SetWeight / GetWeightMap / TransferWeightMap), and the interactive Auto Weight command opens a modal dialog, which the case says to skip. No per-point readout was produced |

## Notes
- Each case is a fresh document: cylinder through its menu command (radius 20, height 300, height
  segments 12, rotation segments 8, Orientation +Y) → Make Editable (106 points, 112 polygons), then the
  three-joint chain and the skin. The rig was rebuilt per pose instead of keyframing, so every
  `mesh.obj` is a single-pose export and `case.toml` names the pose.
- Joint chain as built: J1 at (0, −150, 0); J2 is a child of J1 at a relative position (0, 150, 0), so its
  absolute position is (0, 0, 0); J3 is a child of J2 at a relative position (0, 150, 0), absolute (0, 150, 0).
  `joint_positions` records the absolute positions the form asks for; `joint_chain` records the parent order.
- Weights were written through the weight tag (`SetWeight`) using the form’s rule. The 106 [J1, J2, J3]
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

