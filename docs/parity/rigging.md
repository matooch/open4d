# Rigging I: joints and skinning — observation (O34)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
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

### Skin deformer
| Label | Our key | Default | Options |
|---|---|---|---|

### Weight tag
| Label | Our key | Default | Options |
|---|---|---|---|

## Cases

| Case | Pose (set after binding) | Notes |
|---|---|---|
| bind-pose | none | The mesh must equal the unbound cylinder |
| j2-bend-45 | J2 rotation B 45° | |
| j2-bend-90 | J2 rotation B 90° | |
| j1-twist-90 | J1 rotation H 90° | Rotation about the chain axis |
| j2-bend-dq | J2 rotation B 90°, Skin type set to Dual Quaternion (or the option that is not linear) | |
| j2-scale | J2 scale (1, 1.5, 1) | |
| auto-weights | rebind with the app's automatic weighting (the bind command or the weight tag's auto-weight function). **Skip if it opens a dialog.** Pose J2 B 90° | Record the weight readout per point in `case.toml` as `weights` (level C) |

## Notes
TODO
