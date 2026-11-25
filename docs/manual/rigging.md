# Rigging

Rigging moves a mesh with a skeleton. The pieces are the **Joint** object (`forge.joint`), the
**Weight** tag (`forge.weight_tag`), the **Skin** deformer (`forge.skin`), the **IK** tag
(`forge.ik_tag`), the **Constraint** tag (`forge.constraint_tag`) and the **Pose Morph** tag
(`forge.pose_morph_tag`). The rules are in [RIG.md](../specs/RIG.md); the parameters are in the
reference below. Joints, weights, skin, IK and constraints follow recorded observations of the
reference application; Pose Morph is our own design and is marked parity-pending.

## Joints and chains

`joint.create_chain` makes a chain from a list of points: the first joint at the first point, each
next one a child at the difference of the points. `align` turns the chosen local axis of every joint
but the last towards its child. `joint.align` realigns existing joints and `joint.mirror` copies a
chain across a plane (`yz`, `xz` or `xy`), renaming left and right (`L`/`R`) or appending a suffix.

## Binding with automatic weights

`weights.auto_bind` takes a mesh object and the joints in slot order. It adds a Weight tag, fills it
with bone heat weights (each joint is a bone to its first child joint; a joint without a child is a
point), stores the current pose of the joints as the **bind pose**, and adds a Skin deformer. Run it
with the skeleton in its rest pose. Weights are plain data: `weights.paint`, `weights.smooth`,
`weights.normalise`, `weights.prune` and `weights.mirror` edit them afterwards, and the Skin
deformer offers Linear, Dual Quaternion and Blended skinning.

## IK with a pole

Put an IK tag on the **root** joint of the chain (`tag.attach` with type `forge.ik_tag`) and set
`end` (the tip joint), `goal` (any object) and optionally `pole`. The tag writes the joints'
rotations every frame, before any generator or deformer, so the skin follows. The solver never
stretches the chain: a goal that is too far gives the straight chain. With a pole object the elbow
bends in the plane of the chord and the pole, on the side **away from** the pole; without one it
bends against the root joint's Z axis. `twist` rotates the bend plane about the chord.

## Pose Morph

A Pose Morph tag stores a **base pose** and up to 16 **targets**, each a full set of point
positions with a name and a **strength**. Strengths are percent parameters like any other: key
them, link them, drive them from expressions. They are not limited to 0–100 %.

- `pose_morph.add_target` on a polygon object creates the tag when it is missing, takes the
  object's current points as the base (first target only) and as the new target. To make a real
  target, edit the points first, call `pose_morph.update_target`, then edit the object back, or pass
  `points` explicitly. `pose_morph.clear_target` empties a slot, `pose_morph.set_base` re-captures
  the base.
- **Relative** mode (the default) adds each target's offset from the base onto the object's
  points: `p = c + Σ s·(target − base)`. **Absolute** mode pulls the points towards the stored
  positions: `p = c + Σ s·(target − c)`. They agree while the object still equals the base.
- The morph runs on the object's own points, before every deformer, so a Skin deformer skins the
  morphed mesh.

Pose Morph is **parity-pending**: the reference application's strength could not be set from a
script, so there is no recorded data (observation O37); position/rotation/scale morphs are not
modelled.

## Walkthrough: an S6 scene from a script

The S6 acceptance scenario of the roadmap builds the whole thing in one Rhai script:
[`tests/acceptance/s6_rigging.rhai`](../../tests/acceptance/s6_rigging.rhai). A three-joint chain
is bound with automatic weights to an editable cylinder, an IK tag reaches for a keyed Goal with a
keyed Pole, and a Pose Morph widens the upper half of the cylinder with a keyed strength.

```rhai
let chain = run("joint.create_chain", #{ points: [[0, -100, 0], [0, 0, 0], [0, 100, 0]], prefix: "J", align: "y" });
run("weights.auto_bind", #{ object: body, joints: [chain.nodes[0], chain.nodes[1], chain.nodes[2]] });
let ik = run("tag.attach", #{ type: "forge.ik_tag", node: chain.nodes[0] }).node;
set_param(ik, "end", chain.nodes[2]);
set_param(ik, "goal", goal);
set_param(ik, "pole", pole);
run("key.set", #{ node: tag, track: "strength_0", frame: 30, value: 100.0 });
```

The acceptance test runs exactly that file through the CLI and checks, from `forge eval` output
and OBJ exports at the keyed frames, that the tip reaches the goal, that the elbow bends away
from the pole, that the skin's ends follow the root and the IK-driven tip, that the morph moves
only the points of its target in proportion to its keyed strength, that all 61 frames evaluate,
and that two runs give identical points.

```text
forge run tests/acceptance/s6_rigging.rhai --save s6.forge
forge eval s6.forge --frame 30 --node <body id> --export body30.obj
```
