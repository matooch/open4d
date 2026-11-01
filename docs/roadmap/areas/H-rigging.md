# H · Rigging, tags, constraints, node expressions

## Scope (v1)

- [ ] **Joint object:** a transform with a bone display (length to its first child joint).
  - Joint tools: align (orient to the child), mirror chain, create a joint chain by
    clicking.
- [ ] **Skin deformer and Weight tag:**
  - bind pose stored on the weight tag;
  - linear blend skinning, with dual quaternion as an option;
  - max influences;
  - normalisation.
- [ ] **Weights:**
  - auto weights at bind;
  - paint tool (add, subtract, smooth, set; brush falloff; symmetry);
  - weight manager panel (list per joint, copy/mirror/normalise/prune);
  - vertex weight display.
- [ ] **IK tag:**
  - 2-joint analytic solver;
  - chain solver (multi-joint);
  - goal object, pole vector (pole object or angle), twist;
  - IK/FK blend.
- [ ] **Constraint tag:** PSR (position, scale, rotation with weights and offsets), Aim
  (with up vector), Parent, Clamp (to surface or spline, distance).
- [ ] **Tags:** Target, Align to Spline, Vibrate, Look at Camera, Protection, Display
  (visibility, shading), Compositing (render visibility), Phong (done).
- [ ] **Pose Morph tag:**
  - point morphs and PSR morphs;
  - relative/absolute modes;
  - strengths animatable;
  - edit mode.
- [ ] **User data:** add typed parameters to any node, shown in the Attribute panel and
  animatable.
- [ ] **Node expressions:** a graph per object. Nodes:
  - object (get/set params), maths, compare, condition;
  - range mapper, spline sample, time, iteration, constant, result;
  - Rhai script node.

  Evaluated each frame before generators. Includes an editor UI.
- [ ] **v2:** character templates (walk cycles), muscle systems, cloth.

## Observation jobs

| Job | Form | Status |
|---|---|---|
| O34 | [rigging.md](../../parity/rigging.md): joint chain, cylinder bound with explicit weights set by script, posed at three rotations; skinned meshes; auto-weight bind if it runs without a dialog | todo |
| O35 | [ik.md](../../parity/ik.md): 2-joint IK with the goal at 5 positions; 3-joint chain; pole object; joint matrices as clone-kind samples | todo |
| O36 | [constraints.md](../../parity/constraints.md): PSR, Aim with up, Parent, Clamp; animated driver, sampled per frame | todo |
| O37 | [pose-morph.md](../../parity/pose-morph.md): point morph of a sphere at strengths 0, 0.5, 1, two morphs mixed; relative vs absolute | todo |
| O27 | animation-2: Target, Align to Spline, Vibrate | todo |
| O54 | [tags-2.md](../../parity/tags-2.md): Display, Protection, Look at Camera, selection tags, vertex map tag, restriction | todo |

## Implementation design

- **`crates/rig` (`forge-rig`):**
  - joint maths: chain world matrices, bind pose inverse;
  - skinning (LBS, DQS);
  - IK solvers: analytic 2-bone (law of cosines, with the pole plane), FABRIK for chains,
    then a rotation fit back into joint local rotations;
  - constraints;
  - morph blending.
- **Evaluation order** (per frame), documented in CORE.md:
  1. animation tracks (`scene_at`);
  2. expressions and tags in priority order: expressions, constraints, IK, other tags;
  3. generators and deformers (the skin is a deformer: it applies after its parent
     mesh's generator, like the existing deformers);
  4. dynamics (I).

  This matches how the reference app orders tag priorities (to be confirmed by O36: a
  constraint driven by an IK result).
- **Skinning parity:** level A when the weights are set explicitly (deterministic LBS).
  Auto weights are level C: our own heat-diffusion weights (bone-heat: solve the Laplacian
  with the heat source at the nearest visible bone), compared against the reference's
  auto weights for plausibility only.
- **Node expressions:** `crates/nodes` (`forge-nodes`). The graph model and evaluation
  are typed ports with topological evaluation and a cycle check. The editor UI uses
  `egui-snarl` (MIT) or our own node canvas.
- **IK and constraint samples:** joint and object world matrices are recorded as `clones`
  kind samples, one "clone" per joint or object per frame case. This reuses the
  harness's comparison.

## Briefs

| # | Title | Depends on | Observations |
|---|---|---|---|
| 100 | Joint object, bone display, joint tools (align, mirror, chain creation) | 011 | O34 |
| 101 | Skin deformer + Weight tag + bind pose (LBS, DQS option) | 100, 050 | O34 |
| 102 | Weight tools: paint brush, weight manager, normalise/mirror/smooth/prune | 101, 150 | G6 |
| 103 | Auto weights (bone heat) | 101 | O34 |
| 104 | IK tag: 2-bone, chain, pole, twist, IK/FK blend | 100 | O35 |
| 105 | Constraint tag: PSR, Aim, Parent, Clamp | 100 | O36 |
| 106 | Tags: Target, Align to Spline, Vibrate, Look at Camera, Protection, Display, Compositing | 088 | O27, O54 |
| 107 | Pose Morph tag | 088 | O37 |
| 108 | Node expression engine `forge-nodes` | 016 | – |
| 109 | Node expression editor UI | 108, 150 | G6 |
| 110 | User data on any node | 011, 150 | O54 |
| 111 | Evaluation order: tag priorities, expressions before generators (CORE.md) | 104, 105, 108 | O36 |
| 112 | Weight and joint display modes in the viewport | 101, 143 | – |
| 113 | Rig mirroring (joints and weights, left/right naming) | 101 | – |
| 114 | S6 acceptance test and rigging docs | 100–113 | – |

## Done when
S6 passes. Skinning with explicit weights matches O34 exactly. IK and constraints match
O35/O36 at level B.
